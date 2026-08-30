//! Markdown ingestion through pulldown-cmark's bounded event stream.
//!
//! Source-aware parsing maps straight into the shared document model
//! without an HTML round trip: headings, paragraphs, lists, quotes, code,
//! rules, tables, and inline roles keep their reading-relevant structure
//! while raw HTML and active references stay completely inert. The byte
//! budget matches plain text (`DEC-TEST-012`): metadata first, then a
//! guarded read, then strict UTF-8 decoding.

use std::{fs::File, io::Read, ops::Range, path::Path, sync::Arc};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{
    error::{DocumentError, sanitize_path},
    model::{
        Block, BlockKind, Document, DocumentId, ImageRef, ImageResource, InlineKind, InlineSpan,
        SourceMapping,
    },
    text::{TextLimits, file_stem_title},
};

/// Reads and parses a Markdown book under the supplied limits.
///
/// # Errors
///
/// Returns [`DocumentError::TooLarge`] above the byte limit before any full
/// read, [`DocumentError::Read`] for operating-system failures, and
/// [`DocumentError::InvalidEncoding`] for undecodable or malformed input.
pub fn load_markdown_file(path: &Path, limits: &TextLimits) -> Result<Document, DocumentError> {
    let display = sanitize_path(&path.display().to_string());
    let mut file = File::open(path).map_err(|source| DocumentError::Read {
        path: display.clone(),
        source,
    })?;

    let declared = file.metadata().map_err(|source| DocumentError::Read {
        path: display.clone(),
        source,
    })?;
    if declared.len() > limits.max_bytes {
        return Err(DocumentError::TooLarge {
            path: display,
            size: declared.len(),
            limit: limits.max_bytes,
        });
    }

    let mut bytes = Vec::new();
    (&mut file)
        .take(limits.max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| DocumentError::Read {
            path: display.clone(),
            source,
        })?;
    if bytes.len() as u64 > limits.max_bytes {
        return Err(DocumentError::TooLarge {
            path: display,
            size: bytes.len() as u64,
            limit: limits.max_bytes,
        });
    }

    load_markdown_bytes(&display, &bytes, limits)
}

/// Parses in-memory bytes with the same pipeline as [`load_markdown_file`].
///
/// # Errors
///
/// Same variants as [`load_markdown_file`] minus [`DocumentError::Read`].
pub fn load_markdown_bytes(
    path: &str,
    bytes: &[u8],
    limits: &TextLimits,
) -> Result<Document, DocumentError> {
    if bytes.len() as u64 > limits.max_bytes {
        return Err(DocumentError::TooLarge {
            path: path.to_owned(),
            size: bytes.len() as u64,
            limit: limits.max_bytes,
        });
    }
    let source = std::str::from_utf8(bytes).map_err(|error| DocumentError::InvalidEncoding {
        path: path.to_owned(),
        offset: error.valid_up_to(),
        cause: "invalid UTF-8 sequence".to_owned(),
    })?;
    let id = DocumentId::new(format!("{path}:markdown"));
    parse_markdown(id, Some(file_stem_title(path)), source).map_err(|detail| {
        DocumentError::InvalidStructure {
            path: path.to_owned(),
            detail,
        }
    })
}

/// Inclusive event-to-model conversion limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkdownWorkLimits {
    pub max_events: usize,
    pub max_nesting: usize,
    pub max_output_bytes: usize,
    pub max_blocks: usize,
}

impl Default for MarkdownWorkLimits {
    fn default() -> Self {
        Self {
            max_events: 2_000_000,
            max_nesting: 256,
            max_output_bytes: 64 * 1024 * 1024,
            max_blocks: 1_000_000,
        }
    }
}

/// One block under construction from the event stream.
struct PendingBlock {
    kind: BlockKind,
    text: String,
    /// Per-byte inline role parallel to `text`.
    marks: Vec<Option<InlineMark>>,
    sources: Vec<Option<Range<usize>>>,
    syntax_range: Option<Range<usize>>,
    code_language: Option<String>,
    /// Whitespace collapsing state.
    pending_space: bool,
    pending_mark: Option<InlineMark>,
    pending_source: Option<Range<usize>>,
    /// A paragraph boundary landed inside this block; the next content
    /// inserts the joining newline lazily so trailing separators never
    /// accumulate.
    needs_join: bool,
    /// Code blocks append verbatim without collapsing.
    literal: bool,
}

impl PendingBlock {
    fn new(kind: BlockKind, literal: bool) -> Self {
        Self {
            kind,
            text: String::new(),
            marks: Vec::new(),
            sources: Vec::new(),
            syntax_range: None,
            code_language: None,
            pending_space: false,
            pending_mark: None,
            pending_source: None,
            needs_join: false,
            literal,
        }
    }

    fn has_content(&self) -> bool {
        !self.text.is_empty()
    }

    /// Appends decoded text verbatim, one role entry per byte.
    fn push_str_raw(&mut self, raw: &str, mark: Option<InlineMark>, source: Option<Range<usize>>) {
        self.text.push_str(raw);
        self.marks.extend(std::iter::repeat_n(mark, raw.len()));
        self.sources.extend(std::iter::repeat_n(source, raw.len()));
    }

    /// Appends one character under the collapsing rules.
    fn push_char(
        &mut self,
        character: char,
        mark: Option<InlineMark>,
        source: Option<Range<usize>>,
    ) {
        if self.literal {
            let mut buffer = [0u8; 4];
            let encoded = character.encode_utf8(&mut buffer);
            self.push_str_raw(encoded, mark, source);
            return;
        }
        if character.is_whitespace() {
            self.pending_space = true;
            self.pending_mark = mark;
            self.pending_source = source;
            return;
        }
        if self.needs_join && self.has_content() {
            self.push_str_raw("\n", None, None);
        } else if self.pending_space && self.has_content() {
            self.push_str_raw(" ", self.pending_mark.clone(), self.pending_source.clone());
        }
        self.needs_join = false;
        self.pending_space = false;
        self.pending_mark = None;
        self.pending_source = None;
        let mut buffer = [0u8; 4];
        let encoded = character.encode_utf8(&mut buffer);
        self.push_str_raw(encoded, mark, source);
    }

    /// Appends decoded text with the supplied effective role.
    fn push_text(
        &mut self,
        raw: &str,
        mark: Option<&InlineMark>,
        source_range: Range<usize>,
        source_text: &str,
    ) {
        for (character, source) in raw.chars().zip(character_source_ranges(
            raw,
            source_text,
            source_range.start,
        )) {
            self.push_char(character, mark.cloned(), source);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InlineMark {
    kind: InlineKind,
    destination: Option<Arc<str>>,
    syntax_range: Option<Range<usize>>,
}

fn character_source_ranges(
    rendered: &str,
    source: &str,
    source_start: usize,
) -> Vec<Option<Range<usize>>> {
    let mut cursor = 0usize;
    rendered
        .chars()
        .map(|character| {
            let encoded = character.to_string();
            let relative = source.get(cursor..)?.find(&encoded)?;
            let start = cursor + relative;
            let end = start + encoded.len();
            cursor = end;
            Some(source_start + start..source_start + end)
        })
        .collect()
}

fn mapping_runs(sources: &[Option<Range<usize>>], window: Range<usize>) -> Vec<SourceMapping> {
    let mut mappings: Vec<SourceMapping> = Vec::new();
    let mut index = window.start;
    while index < window.end {
        let source = sources.get(index).cloned().flatten();
        let mut end = index + 1;
        while end < window.end && sources.get(end) == sources.get(index) {
            end += 1;
        }
        if let Some(source) = source {
            if let Some(previous) = mappings.last_mut()
                && previous.canonical_range().end == index
                && previous.source_range().end == source.start
            {
                let canonical_start = previous.canonical_range().start;
                let source_start = previous.source_range().start;
                *previous = SourceMapping::new(canonical_start..end, source_start..source.end);
            } else {
                mappings.push(SourceMapping::new(index..end, source));
            }
        }
        index = end;
    }
    mappings
}

fn contiguous_source_range(
    mappings: &[SourceMapping],
    canonical: &Range<usize>,
) -> Option<Range<usize>> {
    let relevant: Vec<&SourceMapping> = mappings
        .iter()
        .filter(|mapping| {
            mapping.canonical_range().start < canonical.end
                && canonical.start < mapping.canonical_range().end
        })
        .collect();
    let first = *relevant.first()?;
    let last = *relevant.last()?;
    if first.canonical_range().start > canonical.start
        || last.canonical_range().end < canonical.end
        || relevant
            .iter()
            .any(|mapping| mapping.canonical_range().len() != mapping.source_range().len())
    {
        return None;
    }
    let start = first.source_range().start + canonical.start - first.canonical_range().start;
    let end = last.source_range().end - (last.canonical_range().end - canonical.end);
    relevant
        .windows(2)
        .all(|pair| pair[0].source_range().end == pair[1].source_range().start)
        .then_some(start..end)
}

/// Accumulated document under construction.
#[derive(Default)]
struct Assembled {
    canonical: String,
    blocks: Vec<Block>,
    inline: Vec<InlineSpan>,
    mappings: Vec<SourceMapping>,
}

impl Assembled {
    /// Inserts the inter-block newline, tight for grouped list entries.
    fn separate(&mut self, kind: BlockKind) {
        if self.canonical.is_empty() {
            return;
        }
        let tight = matches!(
            self.blocks.last().map(Block::kind),
            Some(BlockKind::ListItem { .. })
        ) && matches!(kind, BlockKind::ListItem { .. });
        let at = self.canonical.len();
        self.canonical.push('\n');
        if tight {
            if let Some(last) = self.blocks.last_mut() {
                last.extend_to(at + 1);
            }
        } else {
            self.blocks
                .push(Block::new(BlockKind::BlankLine, at..at + 1));
        }
    }

    /// Commits one finished block, shifting decorations to global offsets.
    fn commit(&mut self, block: &PendingBlock) {
        if !block.has_content() {
            return;
        }
        self.separate(block.kind);
        let start = self.canonical.len();
        self.canonical.push_str(&block.text);
        let local_mappings = mapping_runs(&block.sources, 0..block.text.len());
        for (range, mark) in runs_from_marks(&block.marks) {
            let mut span = InlineSpan::with_metadata(
                mark.kind,
                start + range.start..start + range.end,
                mark.destination.map(|destination| destination.to_string()),
                contiguous_source_range(&local_mappings, &range),
            );
            span.set_syntax_range(mark.syntax_range);
            self.inline.push(span);
        }
        let mut committed = Block::new(block.kind, start..self.canonical.len());
        committed.set_source(contiguous_source_range(
            &local_mappings,
            &(0..block.text.len()),
        ));
        committed.set_syntax(block.syntax_range.clone());
        committed.set_code_language(block.code_language.clone());
        self.blocks.push(committed);
        self.mappings
            .extend(local_mappings.into_iter().map(|mapping| {
                SourceMapping::new(
                    start + mapping.canonical_range().start..start + mapping.canonical_range().end,
                    mapping.source_range().clone(),
                )
            }));
    }
}

/// Row-major table accumulation before assembly.
#[derive(Default)]
struct TableBuffer {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    /// Whether a `Start` opened a cell or row still awaiting its end event;
    /// prevents double-closing from creating phantom empty cells.
    cell_open: bool,
    row_open: bool,
}

impl TableBuffer {
    fn open_row(&mut self) {
        self.current_row.clear();
        self.row_open = true;
    }

    fn open_cell(&mut self) {
        self.current_cell.clear();
        self.cell_open = true;
    }

    fn close_cell(&mut self) {
        if !self.cell_open {
            return;
        }
        self.cell_open = false;
        let cell = self.current_cell.trim().to_owned();
        self.current_cell.clear();
        self.current_row.push(cell);
    }

    fn close_row(&mut self) {
        if !self.row_open {
            return;
        }
        self.close_cell();
        self.row_open = false;
        if !self.current_row.is_empty() {
            let row = std::mem::take(&mut self.current_row);
            self.rows.push(row);
        }
    }

    /// Joins rows with newlines and cells with pipe delimiters.
    fn finish(&mut self) -> (String, Vec<Range<usize>>) {
        self.close_row();
        let mut text = String::new();
        let mut ranges = Vec::new();
        for row in &self.rows {
            if !text.is_empty() {
                text.push('\n');
            }
            for (index, cell) in row.iter().enumerate() {
                if index > 0 {
                    text.push_str(" | ");
                }
                let start = text.len();
                text.push_str(cell);
                ranges.push(start..text.len());
            }
        }
        (text, ranges)
    }
}

/// Event-stream state driving one Markdown parse.
struct ParserState {
    out: Assembled,
    pending: Option<PendingBlock>,
    /// Roles stack as `None` for plain contexts such as image alt text.
    inline_stack: Vec<Option<InlineMark>>,
    list_stack: Vec<bool>,
    table: Option<TableBuffer>,
    /// Declared image destinations plus the kind of block each placeholder
    /// split, awaiting their collected alt text.
    image_stack: Vec<(String, Option<BlockKind>)>,
    /// When set, text events feed the alt-text capture instead of the open
    /// block; tables keep the legacy plain-text behavior instead.
    alt_capture: Option<String>,
    structure: Vec<TagEnd>,
}

impl ParserState {
    fn flush(&mut self) {
        if let Some(block) = self.pending.take() {
            self.out.commit(&block);
        }
    }

    fn current_mark(&self) -> Option<InlineMark> {
        self.inline_stack.last().cloned().flatten()
    }

    fn start(&mut self, tag: &Tag<'_>, source_range: Range<usize>) {
        match tag {
            Tag::Paragraph => {
                // Transparent inside list items and quotes, whose own
                // pending block absorbs the paragraph's content.
                if self.pending.is_none() {
                    self.pending = Some(PendingBlock::new(BlockKind::Paragraph, false));
                }
            }
            Tag::Heading { level, .. } => {
                self.flush();
                self.pending = Some(PendingBlock::new(
                    BlockKind::Heading {
                        level: heading_level(*level),
                    },
                    false,
                ));
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.pending = Some(PendingBlock::new(BlockKind::Quote, false));
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let mut block = PendingBlock::new(BlockKind::CodeBlock, true);
                block.syntax_range = Some(source_range);
                if let CodeBlockKind::Fenced(language) = kind {
                    let language = language.trim();
                    if !language.is_empty() {
                        block.code_language = Some(language.to_owned());
                    }
                }
                self.pending = Some(block);
            }
            Tag::List(ordered) => {
                // Nested lists split the enclosing item: its own text
                // completes before deeper entries begin.
                self.flush();
                self.list_stack.push(ordered.is_some());
            }
            Tag::Item => {
                self.flush();
                self.pending = Some(PendingBlock::new(
                    BlockKind::ListItem {
                        depth: u8::try_from(self.list_stack.len().saturating_sub(1))
                            .unwrap_or(u8::MAX),
                        ordered: self.list_stack.last().copied().unwrap_or(false),
                    },
                    false,
                ));
            }
            Tag::Table(_) => {
                self.flush();
                self.table = Some(TableBuffer::default());
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some(buffer) = self.table.as_mut() {
                    buffer.open_row();
                }
            }
            Tag::TableCell => {
                if let Some(buffer) = self.table.as_mut() {
                    buffer.open_cell();
                }
            }
            Tag::Emphasis => self.inline_stack.push(Some(InlineMark {
                kind: InlineKind::Emphasis,
                destination: None,
                syntax_range: Some(source_range),
            })),
            Tag::Strong => self.inline_stack.push(Some(InlineMark {
                kind: InlineKind::Strong,
                destination: None,
                syntax_range: Some(source_range),
            })),
            Tag::Link { dest_url, .. } => self.inline_stack.push(Some(InlineMark {
                kind: InlineKind::Link,
                destination: Some(Arc::from(dest_url.as_ref())),
                syntax_range: Some(source_range),
            })),
            // Images split the open flow and capture their alt text; inside
            // table cells the alt text stays part of the cell instead.
            Tag::Image { dest_url, .. } => {
                if self.table.is_some() {
                    self.inline_stack.push(None);
                } else {
                    let reopened = self.pending.as_ref().map(|block| block.kind);
                    self.flush();
                    self.image_stack
                        .push((dest_url.as_ref().to_owned(), reopened));
                    self.alt_capture = Some(String::new());
                }
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => match self.pending.as_ref().map(|block| block.kind) {
                Some(BlockKind::Paragraph) => self.flush(),
                Some(_) => {
                    if let Some(block) = self.pending.as_mut() {
                        block.needs_join = true;
                    }
                }
                None => {}
            },
            TagEnd::Item | TagEnd::Heading(_) | TagEnd::BlockQuote(_) | TagEnd::CodeBlock => {
                self.flush();
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            TagEnd::TableCell => {
                if let Some(buffer) = self.table.as_mut() {
                    buffer.close_cell();
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(buffer) = self.table.as_mut() {
                    buffer.close_row();
                }
            }
            TagEnd::Table => {
                self.flush();
                self.commit_table();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
                self.inline_stack.pop();
            }
            TagEnd::Image => {
                if self.table.is_some() {
                    self.inline_stack.pop();
                } else if let Some((dest, reopened)) = self.image_stack.pop() {
                    let alt = self.alt_capture.take().map(|text| {
                        let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
                        if collapsed.is_empty() {
                            None
                        } else {
                            Some(collapsed)
                        }
                    });
                    self.emit_image(&dest, alt.flatten());
                    // Flow resumes in the kind of block the placeholder
                    // split, so trailing words are never dropped.
                    if let Some(kind) = reopened {
                        self.pending = Some(PendingBlock::new(kind, false));
                    }
                }
            }
            _ => {}
        }
    }

    /// Commits one image placeholder block with its caption line.
    fn emit_image(&mut self, dest: &str, alt: Option<String>) {
        let info = ImageRef::new(dest, alt);
        self.out.separate(BlockKind::Image);
        let start = self.out.canonical.len();
        self.out.canonical.push_str(&info.caption());
        self.out.blocks.push(Block::image(
            start..self.out.canonical.len(),
            markdown_image_resource(dest),
        ));
    }

    fn commit_table(&mut self) {
        let Some(mut buffer) = self.table.take() else {
            return;
        };
        let (text, cells) = buffer.finish();
        if text.is_empty() {
            return;
        }
        self.out.separate(BlockKind::Table);
        let start = self.out.canonical.len();
        self.out.canonical.push_str(&text);
        let shifted = cells
            .into_iter()
            .map(|range| start + range.start..start + range.end)
            .collect();
        self.out
            .blocks
            .push(Block::table(start..self.out.canonical.len(), shifted));
    }

    fn rule(&mut self) {
        self.flush();
        self.out.separate(BlockKind::Separator);
        let start = self.out.canonical.len();
        self.out.canonical.push_str("* * *");
        self.out.blocks.push(Block::new(
            BlockKind::Separator,
            start..self.out.canonical.len(),
        ));
    }
}

/// Parses one Markdown source into the shared logical document.
fn parse_markdown(id: DocumentId, title: Option<String>, source: &str) -> Result<Document, String> {
    let (parsed, source_base) = source
        .strip_prefix('\u{feff}')
        .map_or((source, 0), |parsed| (parsed, '\u{feff}'.len_utf8()));
    let parser = Parser::new_ext(parsed, Options::ENABLE_TABLES)
        .into_offset_iter()
        .map(|(event, range)| (event, source_base + range.start..source_base + range.end));
    convert_events(id, title, source, parser, MarkdownWorkLimits::default())
}

#[allow(clippy::too_many_lines)]
fn convert_events<'a>(
    id: DocumentId,
    title: Option<String>,
    source: &'a str,
    events: impl IntoIterator<Item = (Event<'a>, Range<usize>)>,
    limits: MarkdownWorkLimits,
) -> Result<Document, String> {
    let mut state = ParserState {
        out: Assembled::default(),
        pending: None,
        inline_stack: Vec::new(),
        list_stack: Vec::new(),
        table: None,
        image_stack: Vec::new(),
        alt_capture: None,
        structure: Vec::new(),
    };

    let mut event_count = 0usize;
    let mut output_work = 0usize;
    for (event, source_range) in events {
        event_count = event_count
            .checked_add(1)
            .ok_or_else(|| "Markdown event count overflowed".to_owned())?;
        if event_count > limits.max_events {
            return Err(format!(
                "Markdown event count {event_count} exceeds the {} event limit",
                limits.max_events
            ));
        }
        if source_range.start > source_range.end
            || source_range.end > source.len()
            || !source.is_char_boundary(source_range.start)
            || !source.is_char_boundary(source_range.end)
        {
            return Err("Markdown parser produced an invalid source range".to_owned());
        }
        let cost = match &event {
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text) => text.len(),
            Event::SoftBreak | Event::HardBreak => 1,
            Event::Rule => 5,
            _ => 0,
        };
        output_work = output_work
            .checked_add(cost)
            .ok_or_else(|| "Markdown output work overflowed".to_owned())?;
        if output_work > limits.max_output_bytes {
            return Err(format!(
                "Markdown output work {output_work} exceeds the {} byte limit",
                limits.max_output_bytes
            ));
        }
        match event {
            Event::Start(tag) => {
                if state.structure.len() >= limits.max_nesting {
                    return Err(format!(
                        "Markdown nesting exceeds the {} level limit",
                        limits.max_nesting
                    ));
                }
                state.structure.push(tag.to_end());
                state.start(&tag, source_range);
            }
            Event::End(tag) => {
                if state.structure.pop() != Some(tag) {
                    return Err("Markdown event stream has mismatched structure".to_owned());
                }
                state.end(tag);
            }
            Event::Text(text) => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push_str(&text);
                } else {
                    let mark = state.current_mark();
                    if let Some(buffer) = state.table.as_mut() {
                        buffer.current_cell.push_str(&text);
                    } else if let Some(block) = state.pending.as_mut() {
                        let source_text = source.get(source_range.clone()).unwrap_or_default();
                        block.push_text(&text, mark.as_ref(), source_range, source_text);
                    }
                }
            }
            Event::Code(code) => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push_str(&code);
                } else if let Some(block) = state.pending.as_mut() {
                    let source_text = source.get(source_range.clone()).unwrap_or_default();
                    block.push_text(
                        &code,
                        Some(&InlineMark {
                            kind: InlineKind::Code,
                            destination: None,
                            syntax_range: Some(source_range.clone()),
                        }),
                        source_range,
                        source_text,
                    );
                }
            }
            Event::SoftBreak => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push(' ');
                } else {
                    let mark = state.current_mark();
                    if let Some(block) = state.pending.as_mut() {
                        block.push_char(' ', mark, Some(source_range));
                    }
                }
            }
            Event::HardBreak => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push(' ');
                } else if let Some(block) = state.pending.as_mut() {
                    block.push_str_raw("\n", None, Some(source_range));
                }
            }
            Event::Rule => state.rule(),
            // Raw HTML, task markers, and footnote references stay inert.
            _ => {}
        }
    }
    if !state.structure.is_empty()
        || !state.inline_stack.is_empty()
        || !state.list_stack.is_empty()
        || state.table.is_some()
        || !state.image_stack.is_empty()
    {
        return Err("Markdown event stream ended with unfinished structure".to_owned());
    }
    state.flush();

    if state.out.blocks.len() > limits.max_blocks {
        return Err(format!(
            "Markdown produced {} blocks beyond the {} block limit",
            state.out.blocks.len(),
            limits.max_blocks
        ));
    }
    if state.out.canonical.len() > limits.max_output_bytes {
        return Err(format!(
            "Markdown produced {} bytes beyond the {} byte limit",
            state.out.canonical.len(),
            limits.max_output_bytes
        ));
    }

    Document::from_single_section(id, title, state.out.canonical, state.out.blocks)?
        .with_inline(state.out.inline)
        .map(|document| document.with_source_mappings(state.out.mappings))
}

/// Groups equal adjacent byte roles into ordered decoration ranges.
fn runs_from_marks(marks: &[Option<InlineMark>]) -> Vec<(Range<usize>, InlineMark)> {
    let mut runs = Vec::new();
    let mut index = 0usize;
    while index < marks.len() {
        let mark = marks[index].clone();
        let mut end = index + 1;
        while end < marks.len() && marks[end] == mark {
            end += 1;
        }
        if let Some(decoration) = mark {
            runs.push((index..end, decoration));
        }
        index = end;
    }
    runs
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Classifies one Markdown image destination under the lazy-resource policy.
///
/// Plain relative paths stay fetchable references into the book's own
/// neighborhood (their byte length is unknown until a decode pass opens
/// them); absolute, parent-escaping, and scheme-prefixed targets become
/// blocked resources that must never be fetched.
fn markdown_image_resource(dest: &str) -> ImageResource {
    let path = dest.split(['#', '?']).next().unwrap_or_default();
    let fetchable = !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path.split(['/', '\\']).all(|segment| segment != "..");
    if fetchable {
        ImageResource::member(path, None)
    } else {
        ImageResource::blocked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::CowStr;

    fn kinds_of(document: &Document) -> Vec<(String, InlineKind)> {
        let canonical = document.canonical();
        document
            .inline_spans()
            .iter()
            .map(|span| (canonical[span.range().clone()].to_owned(), span.kind()))
            .collect()
    }

    #[test]
    fn md_001_full_semantic_fixture_maps_into_the_shared_model() {
        let source = "# Title Line\n\n\
                      Intro *soft* and **bold** plus `code x`.\n\n\
                      - first\n- second\n  1. inner one\n  2. inner two\n\n\
                      > quoted words\n\n\
                      ```rust\nfn main() {}\n```\n\n\
                      ---\n\n\
                      | A | B |\n|---|---|\n| one | two |\n\n\
                      [outside](https://example.net) link\n";
        let document = parse_markdown(DocumentId::new("md001".to_owned()), None, source)
            .expect("fixture parses");

        assert_eq!(document.sections().len(), 1);
        assert_eq!(document.navigation_points()[0].title(), "Section 1");
        assert_eq!(
            document.navigation_points()[0].position(),
            document.position(0, 0, 0).expect("section start")
        );
        let blocks: Vec<BlockKind> = document.sections()[0]
            .blocks()
            .iter()
            .filter(|block| block.kind() != BlockKind::BlankLine)
            .map(Block::kind)
            .collect();
        assert_eq!(
            blocks,
            [
                BlockKind::Heading { level: 1 },
                BlockKind::Paragraph,
                BlockKind::ListItem {
                    depth: 0,
                    ordered: false
                },
                BlockKind::ListItem {
                    depth: 0,
                    ordered: false
                },
                BlockKind::ListItem {
                    depth: 1,
                    ordered: true
                },
                BlockKind::ListItem {
                    depth: 1,
                    ordered: true
                },
                BlockKind::Quote,
                BlockKind::CodeBlock,
                BlockKind::Separator,
                BlockKind::Table,
                BlockKind::Paragraph,
            ]
        );

        assert!(document.canonical().contains("quoted words"));
        assert!(document.canonical().contains("A | B\none | two"));
        assert_eq!(
            kinds_of(&document),
            [
                ("soft".to_owned(), InlineKind::Emphasis),
                ("bold".to_owned(), InlineKind::Strong),
                ("code x".to_owned(), InlineKind::Code),
                ("outside".to_owned(), InlineKind::Link),
            ]
        );
    }

    #[test]
    fn md_002_nested_lists_keep_depth_and_ordering_distinct() {
        let source = "- leaf\n- outer\n  1. numbered\n     - deepest\n";
        let document =
            parse_markdown(DocumentId::new("md002".to_owned()), None, source).expect("parses");
        let entries: Vec<(u8, bool)> = document.sections()[0]
            .blocks()
            .iter()
            .filter_map(|block| match block.kind() {
                BlockKind::ListItem { depth, ordered } => Some((depth, ordered)),
                _ => None,
            })
            .collect();
        assert_eq!(
            entries,
            [(0, false), (0, false), (1, true), (2, false)],
            "depths follow nesting; ordering flags stay per list"
        );
    }

    #[test]
    fn md_005_code_content_maps_without_fences_language_or_indentation() {
        let source =
            "```rust\nlet λ = call!(\"x, y!\");\n```\n\n    indented(λ, !);\n    second line\n";
        let document =
            parse_markdown(DocumentId::new("md005".to_owned()), None, source).expect("parses");
        let code: Vec<&Block> = document.sections()[0]
            .blocks()
            .iter()
            .filter(|block| block.kind() == BlockKind::CodeBlock)
            .collect();
        assert_eq!(code.len(), 2);

        assert_eq!(code[0].code_language(), Some("rust"));
        assert_eq!(
            &document.canonical()[code[0].range().clone()],
            "let λ = call!(\"x, y!\");\n"
        );
        assert_eq!(
            &source[code[0].source_range().expect("contiguous content").clone()],
            "let λ = call!(\"x, y!\");\n"
        );
        assert_eq!(
            &source[code[0].syntax_range().expect("fenced syntax").clone()],
            "```rust\nlet λ = call!(\"x, y!\");\n```"
        );

        assert_eq!(code[1].code_language(), None);
        assert_eq!(
            &document.canonical()[code[1].range().clone()],
            "indented(λ, !);\nsecond line\n"
        );
        assert_eq!(
            code[1].source_range(),
            None,
            "indent gaps are non-contiguous"
        );
        let mapped_source: Vec<&str> = document
            .source_mappings()
            .iter()
            .filter(|mapping| {
                mapping.canonical_range().start >= code[1].range().start
                    && mapping.canonical_range().end <= code[1].range().end
            })
            .map(|mapping| &source[mapping.source_range().clone()])
            .collect();
        assert_eq!(mapped_source, ["indented(λ, !);\n", "second line\n"]);
    }

    #[test]
    fn md_010_link_targets_and_source_offsets_map_to_original_markdown() {
        let source = concat!(
            "[same!](one.md) and [same!](two.md), ",
            "then [a\\[b\\] λ?!](three.md#δ).\n"
        );
        let document =
            parse_markdown(DocumentId::new("md010".to_owned()), None, source).expect("parses");
        let links: Vec<&InlineSpan> = document
            .inline_spans()
            .iter()
            .filter(|span| span.kind() == InlineKind::Link)
            .collect();
        assert_eq!(links.len(), 3);
        assert_eq!(&document.canonical()[links[0].range().clone()], "same!");
        assert_eq!(&document.canonical()[links[1].range().clone()], "same!");
        assert_eq!(links[0].destination(), Some("one.md"));
        assert_eq!(links[1].destination(), Some("two.md"));
        assert_eq!(&source[links[0].source_range().unwrap().clone()], "same!");
        assert_eq!(&source[links[1].source_range().unwrap().clone()], "same!");
        assert_eq!(
            &source[links[0].syntax_range().unwrap().clone()],
            "[same!](one.md)"
        );
        assert_eq!(
            &source[links[1].syntax_range().unwrap().clone()],
            "[same!](two.md)"
        );

        assert_eq!(&document.canonical()[links[2].range().clone()], "a[b] λ?!");
        assert_eq!(links[2].source_range(), None, "escaped label is segmented");
        let mapped: Vec<&str> = document
            .source_mappings()
            .iter()
            .filter(|mapping| {
                mapping.canonical_range().start >= links[2].range().start
                    && mapping.canonical_range().end <= links[2].range().end
            })
            .map(|mapping| &source[mapping.source_range().clone()])
            .collect();
        assert_eq!(mapped.concat(), "a[b] λ?!");
        assert!(links.iter().all(|link| link.target().is_none()));
    }

    #[test]
    fn md_006_inline_code_punctuation_stays_literal_between_delimiters() {
        let source = "call `f(λ, y!) != g(x)?` now\n";
        let document =
            parse_markdown(DocumentId::new("md006".to_owned()), None, source).expect("parses");
        assert_eq!(
            kinds_of(&document),
            [("f(λ, y!) != g(x)?".to_owned(), InlineKind::Code)]
        );
        let code = &document.inline_spans()[0];
        assert_eq!(
            &source[code.source_range().expect("content range").clone()],
            "f(λ, y!) != g(x)?"
        );
        assert_eq!(
            &source[code.syntax_range().expect("delimiter range").clone()],
            "`f(λ, y!) != g(x)?`"
        );
    }

    #[test]
    fn md_008_raw_html_and_remote_references_stay_inert() {
        let source = "<script>alert(1)</script>\n\n\
                      plain <em onmouseover=\"x\">safe</em> \
                      <img src=\"http://host/x.png\" alt=\"remote\"> tail\n";
        let document =
            parse_markdown(DocumentId::new("md008".to_owned()), None, source).expect("parses");
        let canonical = document.canonical();
        assert!(!canonical.contains("alert"), "script bodies never surface");
        assert!(
            !canonical.contains("http"),
            "remote references never surface"
        );
        assert!(canonical.contains("safe"));
        assert!(canonical.contains("tail"));
    }

    #[test]
    fn md_009_malformed_constructs_parse_deterministically_without_panicking() {
        let source = "**unclosed bold and *mixed markers\n\ndangling ` tick\n\
                      | broken | table |\n| --- |\n| missing cell\n";
        let first =
            parse_markdown(DocumentId::new("md009a".to_owned()), None, source).expect("parses");
        let second =
            parse_markdown(DocumentId::new("md009b".to_owned()), None, source).expect("parses");
        assert_eq!(
            first.canonical(),
            second.canonical(),
            "identical sources produce identical logical text"
        );
    }

    #[test]
    fn md_009_malformed_event_streams_return_typed_errors() {
        let id = || DocumentId::new("malformed-events".to_owned());
        let unmatched = [(Event::End(TagEnd::Strong), 0..0)];
        assert!(
            convert_events(id(), None, "", unmatched, MarkdownWorkLimits::default())
                .unwrap_err()
                .contains("mismatched")
        );

        let unfinished = [(Event::Start(Tag::Strong), 0..0)];
        assert!(
            convert_events(id(), None, "", unfinished, MarkdownWorkLimits::default())
                .unwrap_err()
                .contains("unfinished")
        );

        let mismatched = [
            (Event::Start(Tag::Emphasis), 0..0),
            (Event::End(TagEnd::Strong), 0..0),
        ];
        assert!(
            convert_events(id(), None, "", mismatched, MarkdownWorkLimits::default())
                .unwrap_err()
                .contains("mismatched")
        );
    }

    #[test]
    fn md_010_invalid_event_offsets_and_bom_offsets_are_exact() {
        let reversed = Range { start: 2, end: 1 };
        for range in [reversed, 0..3, 1..2] {
            let events = [(Event::Text(CowStr::Borrowed("λ")), range)];
            assert!(
                convert_events(
                    DocumentId::new("bad-offset".to_owned()),
                    None,
                    "λ",
                    events,
                    MarkdownWorkLimits::default()
                )
                .unwrap_err()
                .contains("invalid source range")
            );
        }

        let source = "\u{feff}[λ](target.md)";
        let document = parse_markdown(DocumentId::new("bom-offset".to_owned()), None, source)
            .expect("BOM source parses");
        let link = &document.inline_spans()[0];
        assert_eq!(&source[link.source_range().unwrap().clone()], "λ");
        assert_eq!(
            &source[link.syntax_range().unwrap().clone()],
            "[λ](target.md)"
        );
        assert!(link.source_range().unwrap().start >= '\u{feff}'.len_utf8());
    }

    #[test]
    fn md_012_event_nesting_output_and_block_work_limits_are_exact() {
        let source = "x";
        let text = [(Event::Text(CowStr::Borrowed("x")), 0..1)];
        convert_events(
            DocumentId::new("work-at".to_owned()),
            None,
            source,
            text.clone(),
            MarkdownWorkLimits {
                max_events: 1,
                max_nesting: 0,
                max_output_bytes: 1,
                max_blocks: 0,
            },
        )
        .expect("an uncontained text event creates no block and meets exact limits");
        assert!(
            convert_events(
                DocumentId::new("work-over".to_owned()),
                None,
                source,
                text,
                MarkdownWorkLimits {
                    max_events: 0,
                    ..MarkdownWorkLimits::default()
                },
            )
            .unwrap_err()
            .contains("event count")
        );

        let nested = [(Event::Start(Tag::Paragraph), 0..1)];
        assert!(
            convert_events(
                DocumentId::new("nest-over".to_owned()),
                None,
                source,
                nested,
                MarkdownWorkLimits {
                    max_nesting: 0,
                    ..MarkdownWorkLimits::default()
                },
            )
            .unwrap_err()
            .contains("nesting")
        );
    }

    #[test]
    fn md_008_registered_hostile_fixture_is_inert_and_deterministic() {
        let source = include_str!("../../tests/fixtures/markdown/hostile.md");
        let first = parse_markdown(DocumentId::new("hostile".to_owned()), None, source)
            .expect("hostile Markdown stays a bounded model");
        let second = parse_markdown(DocumentId::new("hostile".to_owned()), None, source)
            .expect("repeat parse succeeds");
        assert_eq!(first.canonical(), second.canonical());
        assert!(!first.canonical().contains("fetch("));
        assert!(!first.canonical().contains("/etc/passwd"));
        assert!(image_blocks(&first).is_empty());
    }

    #[test]
    fn md_012_byte_boundaries_reject_exactly_before_any_parsing() {
        let limits = TextLimits { max_bytes: 10 };
        let at_limit = b"0123456789";
        load_markdown_bytes("edge.md", at_limit, &limits).expect("exactly the limit parses");

        let over = b"01234567890";
        let error =
            load_markdown_bytes("over.md", over, &limits).expect_err("above the limit rejects");
        assert!(matches!(
            error,
            DocumentError::TooLarge {
                size: 11,
                limit: 10,
                ..
            }
        ));
    }

    fn image_blocks(document: &Document) -> Vec<&Block> {
        document.sections()[0]
            .blocks()
            .iter()
            .filter(|block| block.kind() == BlockKind::Image)
            .collect()
    }

    #[test]
    fn md_004_standalone_images_become_caption_blocks_with_local_references() {
        let source = "intro line\n\n![a red square](images/red.png)\n\nafter\n";
        let document =
            parse_markdown(DocumentId::new("md004".to_owned()), None, source).expect("parses");

        assert_eq!(
            document.canonical(),
            "intro line\n[image: a red square]\nafter"
        );
        let images = image_blocks(&document);
        assert_eq!(images.len(), 1);
        let resource = images[0].resource().expect("image resource");
        assert_eq!(resource.reference(), Some("images/red.png"));
        assert_eq!(resource.byte_len(), None);
        assert!(resource.is_fetchable());
    }

    #[test]
    fn md_004_mid_paragraph_images_split_flow_like_xhtml() {
        let source = "before ![first](one.png) middle ![](two.png) tail\n";
        let document =
            parse_markdown(DocumentId::new("md004b".to_owned()), None, source).expect("parses");

        let rendered: Vec<(BlockKind, String)> = document.sections()[0]
            .blocks()
            .iter()
            .enumerate()
            .map(|(index, block)| {
                (
                    block.kind(),
                    document.block_text(0, index).unwrap_or_default().to_owned(),
                )
            })
            .collect();

        assert_eq!(
            rendered,
            vec![
                (BlockKind::Paragraph, "before".to_owned()),
                (BlockKind::BlankLine, "\n".to_owned()),
                (BlockKind::Image, "[image: first]".to_owned()),
                (BlockKind::BlankLine, "\n".to_owned()),
                (BlockKind::Paragraph, "middle".to_owned()),
                (BlockKind::BlankLine, "\n".to_owned()),
                (BlockKind::Image, "[image]".to_owned()),
                (BlockKind::BlankLine, "\n".to_owned()),
                (BlockKind::Paragraph, "tail".to_owned()),
            ]
        );
    }

    #[test]
    fn md_004_remote_escaping_and_absolute_targets_never_stay_fetchable() {
        for dest in [
            "http://host/x.png",
            "https://host/x.png",
            "data:image/png;base64,AAAA",
            "/etc/passwd",
            "../outside.png",
            "a/../../escape.png",
        ] {
            let source = format!("![alt text]({dest})\n");
            let document = parse_markdown(DocumentId::new(format!("md004-{dest}")), None, &source)
                .expect("parses");
            let images = image_blocks(&document);
            assert_eq!(images.len(), 1, "{dest}");
            let resource = images[0].resource().expect("resource present");
            assert!(!resource.is_fetchable(), "{dest} must stay blocked");
            assert_eq!(resource.reference(), None, "{dest}");
        }

        // The caption keeps the alt text either way.
        let source = "![alt words](http://host/x.png)\n";
        let document =
            parse_markdown(DocumentId::new("md004-alt".to_owned()), None, source).expect("parses");
        assert_eq!(document.block_text(0, 0), Some("[image: alt words]"));
    }

    #[test]
    fn md_008_raw_html_images_stay_completely_inert() {
        let source = "<img src=\"http://host/x.png\" alt=\"remote\">\n";
        let document =
            parse_markdown(DocumentId::new("md008b".to_owned()), None, source).expect("parses");
        assert!(
            image_blocks(&document).is_empty(),
            "raw HTML never becomes a resource reference"
        );
        assert!(!document.canonical().contains("http"));
    }
}
