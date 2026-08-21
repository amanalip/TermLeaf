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
}

impl Block {
    /// Creates a block spanning `range` bytes of the canonical text.
    ///
    /// Callers within the crate construct blocks while parsing; the range is
    /// trusted to lie inside the document because [`Document::from_blocks`]
    /// verifies total coverage.
    #[must_use]
    pub const fn new(kind: BlockKind, range: Range<usize>) -> Self {
        Self { kind, range }
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
}

/// Semantic kinds Phase 1 produces; later formats extend this enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockKind {
    /// One reflowed run of non-blank source lines.
    Paragraph,
    /// One deliberate empty line preserved from the source.
    BlankLine,
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
}

impl Document {
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

        Ok(Self {
            id,
            title,
            canonical,
            sections: vec![Section::new(None, blocks)],
        })
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

/// A validated logical reading position.
///
/// Construction goes through [`Document::position`]; instances therefore
/// always reference an existing block boundary and can never point outside
/// the document or into the middle of a character.
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
    /// The referenced block must come from the same document the position was
    /// validated against; mismatched documents are a programming defect, so a
    /// missing block resolves clamped instead of panicking.
    #[must_use]
    pub fn absolute_byte(&self, document: &Document) -> usize {
        document
            .sections()
            .get(self.section)
            .and_then(|section| section.blocks().get(self.block))
            .map_or(self.offset, |block| block.range.start + self.offset)
    }
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
}
