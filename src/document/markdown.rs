//! Markdown ingestion through pulldown-cmark's bounded event stream.
//!
//! Source-aware parsing maps straight into the shared document model
//! without an HTML round trip: headings, paragraphs, lists, quotes, code,
//! rules, tables, and inline roles keep their reading-relevant structure
//! while raw HTML and active references stay completely inert. The byte
//! budget matches plain text (`DEC-TEST-012`): metadata first, then a
//! guarded read, then strict UTF-8 decoding.

use std::{fs::File, io::Read, ops::Range, path::Path};

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{
    error::{DocumentError, sanitize_path},
    model::{
        Block, BlockKind, Document, DocumentId, ImageRef, ImageResource, InlineKind, InlineSpan,
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
        .take(limits.max_bytes + 1)
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
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);

    let id = DocumentId::new(format!("{path}:markdown"));
    parse_markdown(id, Some(file_stem_title(path)), source).map_err(|detail| {
        DocumentError::InvalidEncoding {
            path: path.to_owned(),
            offset: 0,
            cause: detail,
        }
    })
}

/// One block under construction from the event stream.
struct PendingBlock {
    kind: BlockKind,
    text: String,
    /// Per-byte inline role parallel to `text`.
    kinds: Vec<Option<InlineKind>>,
    /// Whitespace collapsing state.
    pending_space: bool,
    pending_kind: Option<InlineKind>,
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
            kinds: Vec::new(),
            pending_space: false,
            pending_kind: None,
            needs_join: false,
            literal,
        }
    }

    fn has_content(&self) -> bool {
        !self.text.is_empty()
    }

    /// Appends decoded text verbatim, one role entry per byte.
    fn push_str_raw(&mut self, raw: &str, kind: Option<InlineKind>) {
        self.text.push_str(raw);
        self.kinds.extend(std::iter::repeat_n(kind, raw.len()));
    }

    /// Appends one character under the collapsing rules.
    fn push_char(&mut self, character: char, kind: Option<InlineKind>) {
        if self.literal {
            let mut buffer = [0u8; 4];
            let encoded = character.encode_utf8(&mut buffer);
            self.push_str_raw(encoded, kind);
            return;
        }
        if character.is_whitespace() {
            self.pending_space = true;
            self.pending_kind = kind;
            return;
        }
        if self.needs_join && self.has_content() {
            self.push_str_raw("\n", None);
        } else if self.pending_space && self.has_content() {
            self.push_str_raw(" ", self.pending_kind);
        }
        self.needs_join = false;
        self.pending_space = false;
        let mut buffer = [0u8; 4];
        let encoded = character.encode_utf8(&mut buffer);
        self.push_str_raw(encoded, kind);
    }

    /// Appends decoded text with the supplied effective role.
    fn push_text(&mut self, raw: &str, kind: Option<InlineKind>) {
        for character in raw.chars() {
            self.push_char(character, kind);
        }
    }
}

/// Accumulated document under construction.
#[derive(Default)]
struct Assembled {
    canonical: String,
    blocks: Vec<Block>,
    inline: Vec<InlineSpan>,
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
        for (range, kind) in runs_from_kinds(&block.kinds) {
            self.inline.push(InlineSpan::new(
                kind,
                start + range.start..start + range.end,
            ));
        }
        self.blocks
            .push(Block::new(block.kind, start..self.canonical.len()));
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
    inline_stack: Vec<Option<InlineKind>>,
    list_stack: Vec<bool>,
    table: Option<TableBuffer>,
    /// Declared image destinations plus the kind of block each placeholder
    /// split, awaiting their collected alt text.
    image_stack: Vec<(String, Option<BlockKind>)>,
    /// When set, text events feed the alt-text capture instead of the open
    /// block; tables keep the legacy plain-text behavior instead.
    alt_capture: Option<String>,
}

impl ParserState {
    fn flush(&mut self) {
        if let Some(block) = self.pending.take() {
            self.out.commit(&block);
        }
    }

    fn current_kind(&self) -> Option<InlineKind> {
        self.inline_stack.last().copied().flatten()
    }

    fn start(&mut self, tag: &Tag<'_>) {
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
            Tag::CodeBlock(_) => {
                self.flush();
                self.pending = Some(PendingBlock::new(BlockKind::CodeBlock, true));
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
            Tag::Emphasis => self.inline_stack.push(Some(InlineKind::Emphasis)),
            Tag::Strong => self.inline_stack.push(Some(InlineKind::Strong)),
            Tag::Link { .. } => self.inline_stack.push(Some(InlineKind::Link)),
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
    let parser = Parser::new_ext(source, Options::ENABLE_TABLES);
    let mut state = ParserState {
        out: Assembled::default(),
        pending: None,
        inline_stack: Vec::new(),
        list_stack: Vec::new(),
        table: None,
        image_stack: Vec::new(),
        alt_capture: None,
    };

    for event in parser {
        match event {
            Event::Start(tag) => state.start(&tag),
            Event::End(tag) => state.end(tag),
            Event::Text(text) => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push_str(&text);
                } else {
                    let kind = state.current_kind();
                    if let Some(buffer) = state.table.as_mut() {
                        buffer.current_cell.push_str(&text);
                    } else if let Some(block) = state.pending.as_mut() {
                        block.push_text(&text, kind);
                    }
                }
            }
            Event::Code(code) => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push_str(&code);
                } else if let Some(block) = state.pending.as_mut() {
                    block.push_text(&code, Some(InlineKind::Code));
                }
            }
            Event::SoftBreak => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push(' ');
                } else {
                    let kind = state.current_kind();
                    if let Some(block) = state.pending.as_mut() {
                        block.push_char(' ', kind);
                    }
                }
            }
            Event::HardBreak => {
                if let Some(capture) = state.alt_capture.as_mut() {
                    capture.push(' ');
                } else if let Some(block) = state.pending.as_mut() {
                    block.push_str_raw("\n", None);
                }
            }
            Event::Rule => state.rule(),
            // Raw HTML, task markers, and footnote references stay inert.
            _ => {}
        }
    }
    state.flush();

    Document::from_single_section(id, title, state.out.canonical, state.out.blocks)?
        .with_inline(state.out.inline)
}

/// Groups equal adjacent byte roles into ordered decoration ranges.
fn runs_from_kinds(kinds: &[Option<InlineKind>]) -> Vec<(Range<usize>, InlineKind)> {
    let mut runs = Vec::new();
    let mut index = 0usize;
    while index < kinds.len() {
        let kind = kinds[index];
        let mut end = index + 1;
        while end < kinds.len() && kinds[end] == kind {
            end += 1;
        }
        if let Some(decoration) = kind {
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
    fn fenced_code_preserves_bytes_verbatim() {
        let source = "```text\nkeep    spacing\n\tand tabs\n\nblank above\n```\n";
        let document =
            parse_markdown(DocumentId::new("md005".to_owned()), None, source).expect("parses");
        let code = document.sections()[0]
            .blocks()
            .iter()
            .find(|block| block.kind() == BlockKind::CodeBlock)
            .expect("code block present");
        let range = code.range();
        let raw = &document.canonical()[range.clone()];
        assert!(
            raw.contains("keep    spacing"),
            "internal spacing survives: {raw:?}"
        );
        assert!(raw.contains('\n'), "line breaks survive");
    }

    #[test]
    fn md_006_inline_code_punctuation_stays_literal_between_delimiters() {
        let source = "call `f(x, y) != g(x)` now\n";
        let document =
            parse_markdown(DocumentId::new("md006".to_owned()), None, source).expect("parses");
        assert_eq!(
            kinds_of(&document),
            [("f(x, y) != g(x)".to_owned(), InlineKind::Code)]
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
