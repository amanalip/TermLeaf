//! Core logical types: documents, sections, blocks, and validated positions.
//!
//! A [`Document`] stores one canonical logical text plus the block ranges that
//! partition it. Reading positions address that text, so they never change
//! when terminal width, theme, or reading mode changes. Layout, navigation,
//! search, and persistence all consume this module without any UI dependency.

use std::ops::Range;

use super::error::PositionError;

/// Stable opaque identity for one source book.
///
/// The inner representation is deliberately private so identity policy can
/// evolve (path plus size today, fingerprints later) without breaking
/// persisted state comparisons.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DocumentId(String);

impl DocumentId {
    /// Builds an identifier from caller-supplied stable components.
    #[must_use]
    pub fn new(representation: String) -> Self {
        Self(representation)
    }

    /// The stable string form used by persistence layers.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One logical block of content and its span in the canonical text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    kind: BlockKind,
    range: Range<usize>,
    cells: Vec<Range<usize>>,
    resource: Option<ImageResource>,
    source_range: Option<Range<usize>>,
    syntax_range: Option<Range<usize>>,
    code_language: Option<String>,
}

impl Block {
    /// Creates a block spanning `range` bytes of the canonical text.
    ///
    /// Callers within the crate construct blocks while parsing; the range is
    /// trusted to lie inside the document because [`Document::from_blocks`]
    /// verifies total coverage.
    #[must_use]
    pub const fn new(kind: BlockKind, range: Range<usize>) -> Self {
        Self {
            kind,
            range,
            cells: Vec::new(),
            resource: None,
            source_range: None,
            syntax_range: None,
            code_language: None,
        }
    }

    /// Creates a table block carrying its row-major cell ranges.
    ///
    /// Cells are byte ranges into the canonical text; their union must stay
    /// inside `range`. Layout aligns columns from these ranges when the
    /// terminal is wide enough and falls back to the delimited linear form
    /// otherwise.
    #[must_use]
    pub const fn table(range: Range<usize>, cells: Vec<Range<usize>>) -> Self {
        Self {
            kind: BlockKind::Table,
            range,
            cells,
            resource: None,
            source_range: None,
            syntax_range: None,
            code_language: None,
        }
    }

    /// Creates an image placeholder block over its caption line.
    ///
    /// The canonical text covered by `range` is exactly the caption; the
    /// lazy [`ImageResource`] records how a later decode pass reaches the
    /// pixels without ever holding them here.
    #[must_use]
    pub const fn image(range: Range<usize>, resource: ImageResource) -> Self {
        Self {
            kind: BlockKind::Image,
            range,
            cells: Vec::new(),
            resource: Some(resource),
            source_range: None,
            syntax_range: None,
            code_language: None,
        }
    }

    /// The semantic role of this block.
    #[must_use]
    pub const fn kind(&self) -> BlockKind {
        self.kind
    }

    /// Byte range of this block inside the canonical text.
    #[must_use]
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// Row-major cell ranges; non-empty only for tables.
    #[must_use]
    pub const fn cells(&self) -> &Vec<Range<usize>> {
        &self.cells
    }

    /// The lazy backing resource; present only for image blocks.
    #[must_use]
    pub const fn resource(&self) -> Option<&ImageResource> {
        self.resource.as_ref()
    }

    /// Byte range occupied by this block in the parser's original source.
    #[must_use]
    pub const fn source_range(&self) -> Option<&Range<usize>> {
        self.source_range.as_ref()
    }

    /// Full parser construct, including delimiters and metadata when present.
    #[must_use]
    pub const fn syntax_range(&self) -> Option<&Range<usize>> {
        self.syntax_range.as_ref()
    }

    /// Fenced code language as declared by the source, when present.
    #[must_use]
    pub fn code_language(&self) -> Option<&str> {
        self.code_language.as_deref()
    }

    pub(crate) fn set_source(&mut self, range: Option<Range<usize>>) {
        self.source_range = range;
    }

    pub(crate) fn set_syntax(&mut self, range: Option<Range<usize>>) {
        self.syntax_range = range;
    }

    pub(crate) fn set_code_language(&mut self, language: Option<String>) {
        self.code_language = language;
    }

    /// Extends the range end during assembly.
    ///
    /// Tight list groups share their separator newline with the previous
    /// item, so assembly extends that item's coverage instead of inserting
    /// a blank block. Callers must pass a byte at or after the current end.
    pub(crate) fn extend_to(&mut self, end: usize) {
        self.range.end = end.max(self.range.end);
    }
}

/// Semantic kinds the format adapters produce.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockKind {
    /// One reflowed run of non-blank source lines.
    Paragraph,
    /// One deliberate empty line preserved from the source.
    BlankLine,
    /// A section heading; `level` is 1 for the most important heading.
    Heading {
        /// Nesting depth starting at one.
        level: u8,
    },
    /// One entry of an enclosing list at `depth` nesting levels.
    ListItem {
        /// Zero-based nesting depth of the owning list.
        depth: u8,
        /// Whether the nearest enclosing list is ordered.
        ordered: bool,
    },
    /// One quoted passage; layout prefixes each visual row.
    Quote,
    /// Preformatted source text preserved verbatim, one row per line.
    CodeBlock,
    /// A horizontal rule rendered from its literal marker text.
    Separator,
    /// A table whose canonical rows join cells with a pipe delimiter.
    Table,
    /// An embedded image; canonical text holds only its caption line and the
    /// pixels stay lazy behind the block's resource reference.
    Image,
}

/// Inline semantic role applied to a canonical byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineKind {
    /// Emphasized text; rendered italic where the terminal allows.
    Emphasis,
    /// Strongly emphasized text; rendered bold.
    Strong,
    /// Inline code; rendered distinctly without reflow assumptions.
    Code,
    /// Link text whose destination stays inert during reading.
    Link,
}

/// One decorated byte range inside the canonical text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InlineSpan {
    range: Range<usize>,
    kind: InlineKind,
    destination: Option<String>,
    source_range: Option<Range<usize>>,
    syntax_range: Option<Range<usize>>,
    target: Option<Position>,
}

impl InlineSpan {
    /// Creates a decoration over `range`.
    #[must_use]
    pub const fn new(kind: InlineKind, range: Range<usize>) -> Self {
        Self {
            range,
            kind,
            destination: None,
            source_range: None,
            syntax_range: None,
            target: None,
        }
    }

    /// Creates a source-aware decoration, retaining an inert link destination.
    #[must_use]
    pub fn with_metadata(
        kind: InlineKind,
        range: Range<usize>,
        destination: Option<String>,
        source_range: Option<Range<usize>>,
    ) -> Self {
        Self {
            range,
            kind,
            destination,
            source_range,
            syntax_range: None,
            target: None,
        }
    }

    /// Byte range of this decoration.
    #[must_use]
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// The semantic role carried by this decoration.
    #[must_use]
    pub const fn kind(&self) -> InlineKind {
        self.kind
    }

    /// Destination exactly as declared by the source; it is never fetched.
    #[must_use]
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    /// Byte range occupied by this span in the parser's original source.
    #[must_use]
    pub const fn source_range(&self) -> Option<&Range<usize>> {
        self.source_range.as_ref()
    }

    /// Full parser construct, including link delimiters and destination.
    #[must_use]
    pub const fn syntax_range(&self) -> Option<&Range<usize>> {
        self.syntax_range.as_ref()
    }

    /// Resolved in-document destination, when the format adapter found one.
    #[must_use]
    pub const fn target(&self) -> Option<Position> {
        self.target
    }

    pub(crate) const fn set_target(&mut self, target: Option<Position>) {
        self.target = target;
    }

    pub(crate) fn set_syntax_range(&mut self, range: Option<Range<usize>>) {
        self.syntax_range = range;
    }
}

/// One exact correspondence between canonical visible bytes and source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMapping {
    canonical: Range<usize>,
    source: Range<usize>,
}

impl SourceMapping {
    #[must_use]
    pub const fn new(canonical: Range<usize>, source: Range<usize>) -> Self {
        Self { canonical, source }
    }

    #[must_use]
    pub const fn canonical_range(&self) -> &Range<usize> {
        &self.canonical
    }

    #[must_use]
    pub const fn source_range(&self) -> &Range<usize> {
        &self.source
    }
}

/// One ordered table-of-contents destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationPoint {
    title: String,
    position: Position,
}

impl NavigationPoint {
    #[must_use]
    pub fn new(title: impl Into<String>, position: Position) -> Self {
        Self {
            title: title.into(),
            position,
        }
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

/// Parser-level description of one embedded image placeholder.
///
/// Parsers record the source reference and alternative text exactly as the
/// book declares them; resolution against a container happens later, so
/// hostile or external targets never influence parsing itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageRef {
    src: String,
    alt: Option<String>,
}

impl ImageRef {
    /// Creates a placeholder description from declared attributes.
    #[must_use]
    pub fn new(src: impl Into<String>, alt: Option<String>) -> Self {
        Self {
            src: src.into(),
            alt,
        }
    }

    /// The source reference exactly as declared.
    #[must_use]
    pub fn src(&self) -> &str {
        &self.src
    }

    /// The alternative text when the book supplies one.
    #[must_use]
    pub fn alt(&self) -> Option<&str> {
        self.alt.as_deref()
    }

    /// The canonical caption line for this placeholder.
    ///
    /// The line is the block's whole canonical text, so captions tile like
    /// any other block content and stay addressable by reading positions.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.alt() {
            Some(alt) if !alt.trim().is_empty() => format!("[image: {}]", alt.trim()),
            _ => "[image]".to_owned(),
        }
    }
}

/// Lazy backing reference for one resolved image block.
///
/// The document never holds decoded pixels: `reference` names the container
/// member a later decode pass reads under [`crate::document::ImageLimits`],
/// and it stays `None` for external or unresolvable targets that must never
/// be fetched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageResource {
    reference: Option<String>,
    byte_len: Option<u64>,
}

impl ImageResource {
    /// Creates a resource pointing at one resolvable container member.
    ///
    /// The byte length is known for archive members and unknown for loose
    /// file references.
    #[must_use]
    pub fn member(reference: impl Into<String>, byte_len: Option<u64>) -> Self {
        Self {
            reference: Some(reference.into()),
            byte_len,
        }
    }

    /// Creates an unresolved resource that must never be fetched.
    #[must_use]
    pub const fn blocked() -> Self {
        Self {
            reference: None,
            byte_len: None,
        }
    }

    /// The canonical container member key, when the image resolves inside
    /// the book.
    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }

    /// The container-reported byte length, when known.
    #[must_use]
    pub const fn byte_len(&self) -> Option<u64> {
        self.byte_len
    }

    /// Whether a decode pass may read this resource at all.
    #[must_use]
    pub const fn is_fetchable(&self) -> bool {
        self.reference.is_some()
    }
}

/// One ordered group of blocks; TXT books use a single unnamed section.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Section {
    title: Option<String>,
    blocks: Vec<Block>,
}

impl Section {
    /// Creates a section with an optional title.
    #[must_use]
    pub fn new(title: Option<String>, blocks: Vec<Block>) -> Self {
        Self { title, blocks }
    }

    /// The section title, when the format supplies one.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Blocks in reading order.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
}

/// A complete book in canonical logical form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    id: DocumentId,
    title: Option<String>,
    canonical: String,
    sections: Vec<Section>,
    inline: Vec<InlineSpan>,
    navigation: Vec<NavigationPoint>,
    source_mappings: Vec<SourceMapping>,
}

impl Document {
    /// Assembles a multi-section document from blocks that cover `canonical`.
    ///
    /// Section block ranges must exactly tile the canonical text in order
    /// across the whole document, on character boundaries. Parsers construct
    /// that tiling from their own output, so a mismatch is a programming
    /// defect rather than reader input; it is reported as an error string.
    ///
    /// # Errors
    ///
    /// Returns a descriptive message when ranges are not contiguous across
    /// sections, exceed the canonical text, split a character, or leave
    /// bytes uncovered.
    pub fn from_sections(
        id: DocumentId,
        title: Option<String>,
        canonical: String,
        sections: Vec<Section>,
    ) -> Result<Self, String> {
        let mut expected = 0usize;
        for section in &sections {
            for block in section.blocks() {
                if block.range.start != expected || block.range.end < block.range.start {
                    return Err(format!(
                        "block range {}..{} does not continue coverage at byte {expected}",
                        block.range.start, block.range.end
                    ));
                }
                if block.range.end > canonical.len() {
                    return Err(format!(
                        "block range ends at {} beyond canonical length {}",
                        block.range.end,
                        canonical.len()
                    ));
                }
                if !canonical.is_char_boundary(block.range.start)
                    || !canonical.is_char_boundary(block.range.end)
                {
                    return Err(format!(
                        "block range {}..{} splits a character",
                        block.range.start, block.range.end
                    ));
                }
                expected = block.range.end;
            }
        }
        if expected != canonical.len() {
            return Err(format!(
                "blocks cover {expected} of {} canonical bytes",
                canonical.len()
            ));
        }

        let navigation = fallback_navigation(&sections);
        Ok(Self {
            id,
            title,
            canonical,
            sections,
            inline: Vec::new(),
            navigation,
            source_mappings: Vec::new(),
        })
    }

    /// Assembles a one-section document from blocks that cover `canonical`.
    ///
    /// Block ranges must exactly tile the canonical text on character
    /// boundaries. Parsers construct that tiling from their own output, so a
    /// mismatch is a programming defect rather than reader input; it is
    /// reported as an error string instead of an assertion so no path can
    /// build an unusable document silently.
    ///
    /// # Errors
    ///
    /// Returns a descriptive message when block ranges are not contiguous,
    /// exceed the canonical text, split a character, or leave bytes uncovered.
    pub fn from_single_section(
        id: DocumentId,
        title: Option<String>,
        canonical: String,
        blocks: Vec<Block>,
    ) -> Result<Self, String> {
        let mut expected = 0usize;
        for block in &blocks {
            if block.range.start != expected || block.range.end < block.range.start {
                return Err(format!(
                    "block range {}..{} does not continue coverage at byte {expected}",
                    block.range.start, block.range.end
                ));
            }
            if block.range.end > canonical.len() {
                return Err(format!(
                    "block range ends at {} beyond canonical length {}",
                    block.range.end,
                    canonical.len()
                ));
            }
            if !canonical.is_char_boundary(block.range.start)
                || !canonical.is_char_boundary(block.range.end)
            {
                return Err(format!(
                    "block range {}..{} splits a character",
                    block.range.start, block.range.end
                ));
            }
            expected = block.range.end;
        }
        if expected != canonical.len() {
            return Err(format!(
                "blocks cover {expected} of {} canonical bytes",
                canonical.len()
            ));
        }

        let sections = vec![Section::new(None, blocks)];
        let navigation = fallback_navigation(&sections);
        Ok(Self {
            id,
            title,
            canonical,
            sections,
            inline: Vec::new(),
            navigation,
            source_mappings: Vec::new(),
        })
    }

    /// Attaches validated inline decorations to the document.
    ///
    /// Spans must be sorted by start, lie inside the canonical text on
    /// character boundaries, and never overlap; parsers emit them in order,
    /// so a violation is a programming defect reported as an error string.
    ///
    /// # Errors
    ///
    /// Returns a descriptive message when spans are unordered, out of
    /// bounds, split a character, or overlap.
    pub fn with_inline(mut self, inline: Vec<InlineSpan>) -> Result<Self, String> {
        let mut previous_end = 0usize;
        for span in &inline {
            if span.range.start < previous_end {
                return Err(format!(
                    "inline span {}..{} overlaps or is unordered after byte {previous_end}",
                    span.range.start, span.range.end
                ));
            }
            if span.range.end > self.canonical.len() {
                return Err(format!(
                    "inline span ends at {} beyond canonical length {}",
                    span.range.end,
                    self.canonical.len()
                ));
            }
            if !self.canonical.is_char_boundary(span.range.start)
                || !self.canonical.is_char_boundary(span.range.end)
            {
                return Err(format!(
                    "inline span {}..{} splits a character",
                    span.range.start, span.range.end
                ));
            }
            previous_end = span.range.end;
        }
        self.inline = inline;
        Ok(self)
    }

    /// Attaches ordered, already validated in-document navigation points.
    #[must_use]
    pub fn with_navigation(mut self, navigation: Vec<NavigationPoint>) -> Self {
        self.navigation = navigation;
        self
    }

    /// Attaches exact correspondences to the parser's original bytes.
    #[must_use]
    pub fn with_source_mappings(mut self, mappings: Vec<SourceMapping>) -> Self {
        self.source_mappings = mappings;
        self
    }

    /// The stable document identity.
    #[must_use]
    pub fn id(&self) -> &DocumentId {
        &self.id
    }

    /// The display title, falling back to the identifier stem.
    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("Untitled")
    }

    /// The full canonical logical text.
    #[must_use]
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Total canonical byte length; the percentage denominator.
    #[must_use]
    pub fn len(&self) -> usize {
        self.canonical.len()
    }

    /// Whether the document carries no content at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.canonical.is_empty()
    }

    /// All sections in order.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// All inline decorations, sorted and non-overlapping.
    #[must_use]
    pub fn inline_spans(&self) -> &[InlineSpan] {
        &self.inline
    }

    /// Ordered format-supplied or section-based fallback destinations.
    #[must_use]
    pub fn navigation_points(&self) -> &[NavigationPoint] {
        &self.navigation
    }

    /// Exact canonical-to-source segments in canonical reading order.
    #[must_use]
    pub fn source_mappings(&self) -> &[SourceMapping] {
        &self.source_mappings
    }

    fn section_start_byte(&self, section: usize) -> usize {
        self.sections
            .get(section)
            .and_then(|section| section.blocks.first())
            .map_or_else(
                || {
                    self.sections
                        .get(..section)
                        .and_then(|sections| {
                            sections
                                .iter()
                                .rev()
                                .find_map(|section| section.blocks.last())
                        })
                        .map_or(0, |block| block.range.end)
                },
                |block| block.range.start,
            )
    }

    /// The strongest decoration covering `byte`, if any.
    ///
    /// Producers never overlap decorations, so at most one applies; the
    /// linear scan stays cheap because layout queries in ascending order
    /// through [`Document::inline_spans`].
    #[must_use]
    pub fn inline_kind_at(&self, byte: usize) -> Option<InlineKind> {
        self.inline
            .iter()
            .find(|span| span.range.contains(&byte))
            .map(|span| span.kind)
    }

    /// The text of one block, borrowed from the canonical text.
    #[must_use]
    pub fn block_text(&self, section: usize, block: usize) -> Option<&str> {
        let section = self.sections.get(section)?;
        let block = section.blocks.get(block)?;
        self.canonical.get(block.range.clone())
    }

    /// Validates and constructs a reading position.
    ///
    /// `offset` counts bytes from the start of the referenced block and may
    /// equal the block length so positions can sit on every valid boundary.
    ///
    /// # Errors
    ///
    /// Rejects unknown sections, unknown blocks, offsets outside the block,
    /// and offsets that split a character, each as a typed variant.
    pub fn position(
        &self,
        section: usize,
        block: usize,
        offset: usize,
    ) -> Result<Position, PositionError> {
        let blocks = self
            .sections
            .get(section)
            .map(|s| s.blocks.as_slice())
            .ok_or(PositionError::Section { section })?;
        let range = blocks
            .get(block)
            .map(|b| b.range.clone())
            .ok_or(PositionError::Block { section, block })?;

        let absolute = range.start.saturating_add(offset);
        if offset > range.len() {
            return Err(PositionError::OffsetOutsideBlock { block, offset });
        }
        if !self.canonical.is_char_boundary(absolute) {
            return Err(PositionError::NotCharBoundary { offset });
        }
        Ok(Position {
            section,
            block,
            offset,
        })
    }

    /// Convenience anchor at the very start of the document.
    ///
    /// # Errors
    ///
    /// Fails only for a zero-block document, which callers must treat as an
    /// already-at-start condition rather than unwrap.
    pub fn first_position(&self) -> Result<Position, PositionError> {
        self.position(0, 0, 0)
    }

    /// One-based logical line number containing `absolute_byte`.
    ///
    /// Lines are counted over the canonical bytes, so the number is identical
    /// at every width and after every theme or mode change. Offsets that fall
    /// inside a multi-byte character still count the lines before them
    /// instead of panicking.
    #[must_use]
    pub fn logical_line_number(&self, absolute_byte: usize) -> usize {
        let clamped = absolute_byte.min(self.canonical.len());
        let mut lines = 1usize;
        for byte in &self.canonical.as_bytes()[..clamped] {
            if *byte == b'\n' {
                lines += 1;
            }
        }
        lines
    }
}

/// A logical reading position.
///
/// Ordinary construction goes through [`Document::position`]. Section
/// navigation may additionally address the start of an empty section, which
/// resolves to that section's canonical boundary without inventing content.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Position {
    section: usize,
    block: usize,
    offset: usize,
}

impl Position {
    /// Origin position used when no valid boundary exists (empty books).
    ///
    /// Resolution against a document clamps safely instead of panicking.
    pub const ORIGIN: Self = Self {
        section: 0,
        block: 0,
        offset: 0,
    };

    /// Referenced section ordinal.
    #[must_use]
    pub const fn section(&self) -> usize {
        self.section
    }

    /// Referenced block ordinal within its section.
    #[must_use]
    pub const fn block(&self) -> usize {
        self.block
    }

    /// Byte offset from the start of the referenced block.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Resolves this position to an absolute byte in the canonical text.
    ///
    /// Empty-section anchors resolve to that section's canonical boundary.
    /// Other missing blocks and mismatched documents resolve clamped instead
    /// of panicking.
    #[must_use]
    pub fn absolute_byte(&self, document: &Document) -> usize {
        document
            .sections()
            .get(self.section)
            .and_then(|section| section.blocks().get(self.block))
            .map_or_else(
                || {
                    document
                        .section_start_byte(self.section)
                        .saturating_add(self.offset)
                        .min(document.len())
                },
                |block| block.range.start + self.offset,
            )
    }
}

fn fallback_navigation(sections: &[Section]) -> Vec<NavigationPoint> {
    sections
        .iter()
        .enumerate()
        .map(|(section, content)| {
            let title = content
                .title()
                .filter(|title| !title.trim().is_empty())
                .map_or_else(|| format!("Section {}", section + 1), str::to_owned);
            NavigationPoint::new(
                title,
                Position {
                    section,
                    block: 0,
                    offset: 0,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{DocumentId, PositionError};
    use super::*;

    fn sample() -> Document {
        // Canonical: a b \n | \n | d é \n   ('é' occupies bytes 5..7)
        Document::from_single_section(
            DocumentId::new("model-sample".to_owned()),
            Some("Sample".to_owned()),
            "ab\ncdé\n".to_owned(),
            vec![
                Block::new(BlockKind::Paragraph, 0..3),
                Block::new(BlockKind::BlankLine, 3..4),
                Block::new(BlockKind::Paragraph, 4..8),
            ],
        )
        .expect("sample blocks tile the canonical text")
    }

    #[test]
    fn model_003_positions_resolve_at_every_valid_block_boundary() {
        let document = sample();
        let blocks = document.sections()[0].blocks();

        for (index, block) in blocks.iter().enumerate() {
            let start = document
                .position(0, index, 0)
                .expect("block start is valid");
            let end = document
                .position(0, index, block.range().len())
                .expect("block end is valid");

            assert_eq!(start.absolute_byte(&document), block.range().start);
            assert_eq!(end.absolute_byte(&document), block.range().end);
        }

        let mid = document.position(0, 2, 1).expect("mid-paragraph offset");
        assert_eq!(
            document.block_text(0, 2).expect("text").chars().next(),
            Some('d')
        );
        assert_eq!(mid.absolute_byte(&document), 5);
    }

    #[test]
    fn model_004_invalid_sections_blocks_and_offsets_are_rejected() {
        let document = sample();

        assert_eq!(
            document.position(9, 0, 0),
            Err(PositionError::Section { section: 9 })
        );
        assert_eq!(
            document.position(0, 7, 0),
            Err(PositionError::Block {
                section: 0,
                block: 7
            })
        );
        assert_eq!(
            document.position(0, 0, 4),
            Err(PositionError::OffsetOutsideBlock {
                block: 0,
                offset: 4
            })
        );
        assert_eq!(
            document.position(0, 2, 2),
            Err(PositionError::NotCharBoundary { offset: 2 })
        );

        assert!(
            std::panic::catch_unwind(|| {
                let _ = document.first_position();
            })
            .is_ok()
        );
    }

    #[test]
    fn logical_line_numbers_count_canonical_newlines_only() {
        let document = sample();

        assert_eq!(document.logical_line_number(0), 1);
        assert_eq!(document.logical_line_number(2), 1);
        assert_eq!(document.logical_line_number(4), 2);
        assert_eq!(document.logical_line_number(6), 2);
        assert_eq!(
            document.logical_line_number(usize::MAX),
            3,
            "offsets past the end clamp instead of panicking"
        );
    }

    #[test]
    fn from_single_section_rejects_non_tiling_ranges_without_panic() {
        let result = Document::from_single_section(
            DocumentId::new("broken".to_owned()),
            None,
            "abc".to_owned(),
            vec![
                Block::new(BlockKind::Paragraph, 0..1),
                Block::new(BlockKind::Paragraph, 2..4),
            ],
        );

        let message = result.expect_err("gap and overflow are rejected");
        assert!(message.contains("does not continue coverage"));
    }

    #[test]
    fn model_001_multi_section_documents_tile_across_sections() {
        // Canonical spans two chapters; each section owns a contiguous half.
        let canonical = "Title One\nbody one\nTitle Two\nbody two\n".to_owned();
        let sections = vec![
            Section::new(
                Some("One".to_owned()),
                vec![
                    Block::new(BlockKind::Heading { level: 1 }, 0..10),
                    Block::new(BlockKind::Paragraph, 10..19),
                ],
            ),
            Section::new(
                Some("Two".to_owned()),
                vec![
                    Block::new(BlockKind::Heading { level: 1 }, 19..29),
                    Block::new(BlockKind::Paragraph, 29..38),
                ],
            ),
        ];
        let document = Document::from_sections(
            DocumentId::new("multi".to_owned()),
            None,
            canonical.clone(),
            sections,
        )
        .expect("tiling sections assemble");

        assert_eq!(document.sections().len(), 2);
        assert_eq!(document.sections()[0].title(), Some("One"));
        assert_eq!(document.block_text(1, 0), Some("Title Two\n"));
        let position = document.position(1, 0, 0).expect("section two starts");
        assert_eq!(position.absolute_byte(&document), 19);

        // Coverage gaps across the section boundary are programming defects.
        let broken = vec![
            Section::new(None, vec![Block::new(BlockKind::Paragraph, 0..4)]),
            Section::new(None, vec![Block::new(BlockKind::Paragraph, 6..8)]),
        ];
        let error =
            Document::from_sections(DocumentId::new("x".to_owned()), None, canonical, broken)
                .expect_err("gaps reject");
        assert!(error.contains("does not continue coverage"));
    }

    #[test]
    fn section_navigation_fallbacks_have_stable_titles_and_exact_starts() {
        let document = Document::from_sections(
            DocumentId::new("fallbacks".to_owned()),
            None,
            "one".to_owned(),
            vec![
                Section::new(
                    Some("Named".to_owned()),
                    vec![Block::new(BlockKind::Paragraph, 0..3)],
                ),
                Section::new(Some("  ".to_owned()), Vec::new()),
                Section::new(None, Vec::new()),
            ],
        )
        .expect("empty sections preserve tiling");

        let points = document.navigation_points();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].title(), "Named");
        assert_eq!(points[1].title(), "Section 2");
        assert_eq!(points[2].title(), "Section 3");
        for (index, point) in points.iter().enumerate() {
            assert_eq!(point.position().section(), index);
            assert_eq!(point.position().absolute_byte(&document), [0, 3, 3][index]);
        }
    }
}
