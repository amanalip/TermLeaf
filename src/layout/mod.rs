//! Grapheme-aware, cell-width-aware layout with source mapping.
//!
//! One layout pass turns the canonical logical text into visual rows for one
//! viewport width. Every row records the byte ranges it renders, ordered and
//! in bounds, so navigation can always map a visual row back to logical
//! content and resize can relocate the same passage. This module depends on
//! neither Ratatui nor Crossterm.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::document::{BlockKind, Document};

mod width;

pub use width::{caret_notation, display_width, tab_advance, visible_text};
pub mod viewport;

/// One rendered piece of a row: the raw byte range it displays.
///
/// Rendering applies [`visible_text`] with the running column, so tabs,
/// control bytes, and newline join markers transform identically here and in
/// the UI without storing duplicated strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    range: Range<usize>,
}

impl Span {
    /// The raw canonical bytes this span renders.
    #[must_use]
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// The exact visible string for this span starting at `column`.
    #[must_use]
    pub fn visible(&self, document: &Document, column: u16) -> String {
        document
            .canonical()
            .get(self.range.clone())
            .map_or_else(String::new, |raw| visible_text(raw, column))
    }
}

/// One visual row of laid-out content.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VisualRow {
    section: usize,
    block: usize,
    spans: Vec<Span>,
    cells: u16,
}

impl VisualRow {
    /// Owning section ordinal; every format produces at least one section.
    #[must_use]
    pub const fn section(&self) -> usize {
        self.section
    }

    /// Owning block ordinal within its section; blank rows reference their
    /// blank-line block.
    #[must_use]
    pub const fn block(&self) -> usize {
        self.block
    }

    /// Spans in left-to-right order.
    #[must_use]
    pub const fn spans(&self) -> &Vec<Span> {
        &self.spans
    }

    /// Measured cell count, identical to the sum of rendered span widths.
    #[must_use]
    pub const fn cells(&self) -> u16 {
        self.cells
    }

    /// The complete visible text of this row.
    #[must_use]
    pub fn text(&self, document: &Document) -> String {
        let mut out = String::new();
        let mut column = 0u16;
        for span in &self.spans {
            let visible = span.visible(document, column);
            column += display_width(&visible, column);
            out.push_str(&visible);
        }
        out
    }
}

/// Layout output for one document at one width.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageLayout {
    width: u16,
    rows: Vec<VisualRow>,
}

impl PageLayout {
    /// The width this layout was produced for.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// All visual rows in reading order.
    #[must_use]
    pub const fn rows(&self) -> &Vec<VisualRow> {
        &self.rows
    }

    /// Index of the first visual row whose content ends after `byte`.
    ///
    /// Anchors map to this row, which keeps the same passage visible across
    /// any width change. Returns the final row when the byte sits at or past
    /// the end of the document.
    #[must_use]
    pub fn row_after(&self, byte: usize) -> usize {
        let mut covered = 0usize;
        for (index, row) in self.rows.iter().enumerate() {
            for span in &row.spans {
                covered = covered.max(span.range.end);
            }
            if covered > byte {
                return index;
            }
        }
        self.rows.len().saturating_sub(1)
    }
}

/// Wraps a document into visual rows for one content width.
///
/// A zero width yields an empty layout; callers show the terminal-too-small
/// state instead of laying out unreadable geometry.
#[must_use]
pub fn layout_document(document: &Document, width: u16) -> PageLayout {
    let rows = if width == 0 || document.is_empty() {
        Vec::new()
    } else {
        wrap_all_blocks(document, width)
    };
    PageLayout { width, rows }
}

fn wrap_all_blocks(document: &Document, width: u16) -> Vec<VisualRow> {
    let mut rows = Vec::new();
    for (section_index, section) in document.sections().iter().enumerate() {
        for (block_index, block) in section.blocks().iter().enumerate() {
            match block.kind() {
                BlockKind::BlankLine => rows.push(VisualRow {
                    section: section_index,
                    block: block_index,
                    spans: Vec::new(),
                    cells: 0,
                }),
                BlockKind::Paragraph | BlockKind::Heading { .. } => {
                    let text = document
                        .block_text(section_index, block_index)
                        .unwrap_or_default();
                    wrap_paragraph(
                        text,
                        block.range().start,
                        section_index,
                        block_index,
                        width,
                        &mut rows,
                    );
                }
            }
        }
    }
    rows
}

enum Atom<'t> {
    Content { text: &'t str, offset: Range<usize> },
    Join { newline: usize },
}

fn paragraph_atoms(text: &str, base: usize) -> Vec<Atom<'_>> {
    let mut atoms = Vec::new();
    let mut consumed = 0usize;
    for line in text.split_inclusive('\n') {
        let line_start = base + consumed;
        consumed += line.len();
        match line.strip_suffix('\n') {
            Some(content) => {
                push_content_atoms(&mut atoms, content, line_start);
                atoms.push(Atom::Join {
                    newline: line_start + content.len(),
                });
            }
            None => push_content_atoms(&mut atoms, line, line_start),
        }
    }
    atoms
}

fn push_content_atoms<'t>(atoms: &mut Vec<Atom<'t>>, content: &'t str, start: usize) {
    if content.is_empty() {
        return;
    }
    let mut emitted = 0usize;
    for (after, _opportunity) in unicode_linebreak::linebreaks(content) {
        if after >= content.len() {
            break;
        }
        if after > emitted {
            atoms.push(Atom::Content {
                text: &content[emitted..after],
                offset: start + emitted..start + after,
            });
            emitted = after;
        }
    }
    if content.len() > emitted {
        atoms.push(Atom::Content {
            text: &content[emitted..],
            offset: start + emitted..start + content.len(),
        });
    }
}

fn wrap_paragraph(
    text: &str,
    base: usize,
    section: usize,
    block: usize,
    width: u16,
    rows: &mut Vec<VisualRow>,
) {
    let mut current = VisualRow {
        section,
        block,
        ..VisualRow::default()
    };
    let mut pending_join: Option<usize> = None;

    for atom in paragraph_atoms(text, base) {
        match atom {
            Atom::Join { newline } => {
                if !current.spans.is_empty() {
                    pending_join = Some(newline);
                }
            }
            Atom::Content {
                text: piece,
                offset,
            } => {
                place_content(
                    piece,
                    offset,
                    RowOwner { section, block },
                    width,
                    &mut pending_join,
                    &mut current,
                    rows,
                );
            }
        }
    }

    flush_row(&mut current, rows, section, block);
}

/// Owning logical location of a row under construction.
#[derive(Clone, Copy)]
struct RowOwner {
    section: usize,
    block: usize,
}

fn place_content(
    piece: &str,
    offset: Range<usize>,
    owner: RowOwner,
    width: u16,
    pending_join: &mut Option<usize>,
    current: &mut VisualRow,
    rows: &mut Vec<VisualRow>,
) {
    let mut start = 0usize;
    while start < piece.len() {
        let remaining = &piece[start..];
        let starts_new_row = current.spans.is_empty();
        let joins = usize::from(!starts_new_row && pending_join.is_some());
        let need = display_width(remaining, current.cells + u16::try_from(joins).unwrap_or(1));

        if !starts_new_row && current.cells + u16::try_from(joins).unwrap_or(1) + need > width {
            flush_row(current, rows, owner.section, owner.block);
            *pending_join = None;
            continue;
        }
        if starts_new_row {
            *pending_join = None;
        }

        if joins == 1 {
            let newline = pending_join.take().unwrap_or_default();
            current.spans.push(Span {
                range: newline..newline + 1,
            });
            current.cells += 1;
        }

        let free = width - current.cells;
        let whole = display_width(remaining, current.cells);
        if whole <= free {
            current.spans.push(Span {
                range: offset.start + start..offset.end,
            });
            current.cells += whole;
            return;
        }

        let chunk = force_fit_chunk(remaining, free, current.cells);
        let end = start + chunk.len();
        let column = current.cells;
        current.spans.push(Span {
            range: offset.start + start..offset.start + end,
        });
        current.cells += display_width(chunk, column);
        flush_row(current, rows, owner.section, owner.block);
        start = end;
    }
}

/// Fills one row with the longest grapheme-safe prefix that fits.
///
/// Always consumes at least one grapheme so wrapping makes progress even
/// when a single cluster is wider than the row; such a cluster overflows by
/// its own width rather than being split.
fn force_fit_chunk(remaining: &str, free: u16, column: u16) -> &str {
    let mut end_bytes = 0usize;
    let mut used = 0u16;
    for (index, grapheme) in remaining.grapheme_indices(true) {
        let cells = display_width(grapheme, column + used);
        if used + cells > free && end_bytes > 0 {
            return &remaining[..end_bytes];
        }
        end_bytes = index + grapheme.len();
        used += cells;
        if used >= free {
            break;
        }
    }
    &remaining[..end_bytes]
}

fn flush_row(current: &mut VisualRow, rows: &mut Vec<VisualRow>, section: usize, block: usize) {
    if !current.spans.is_empty() {
        let mut row = std::mem::take(current);
        row.section = section;
        row.block = block;
        rows.push(row);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use unicode_segmentation::UnicodeSegmentation;

    use super::*;
    use crate::document::{DocumentId, text::document_from_text};

    fn doc(text: &str) -> Document {
        document_from_text(DocumentId::new("layout-test".to_owned()), None, text)
            .expect("test text parses")
    }

    fn rendered(layout: &PageLayout, document: &Document) -> Vec<String> {
        layout.rows().iter().map(|row| row.text(document)).collect()
    }

    #[test]
    fn lay_001_no_row_exceeds_available_cells_across_widths() {
        let document = doc("The quick brown fox jumps over the lazy dog.\n\n\
             漢字の文章は二倍の幅を使います。\n\n\
             supersupersupersupersupersupersupersuper long unbreakable run\n");
        for width in [2u16, 3, 5, 8, 13, 21, 40, 79, 80, 120, 200] {
            let layout = layout_document(&document, width);
            for row in layout.rows() {
                assert!(
                    row.cells() <= width,
                    "width {width}: row used {} cells",
                    row.cells()
                );
            }
        }
    }

    #[test]
    fn lay_002_grapheme_clusters_are_never_split_or_clipped() {
        let sample = "emoji \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466} family\n\
                      flag \u{1F1FA}\u{1F1F8} and e\u{301}acute\n";
        let document = doc(sample);
        let block_base = document.sections()[0].blocks()[0].range().start;
        let paragraph = document.block_text(0, 0).expect("paragraph");

        let starts: HashSet<usize> = paragraph
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain(std::iter::once(paragraph.len()))
            .collect();

        for width in [2u16, 4, 7, 11] {
            let layout = layout_document(&document, width);
            for row in &layout.rows()[..layout.rows().len()] {
                for span in row.spans() {
                    let relative_end = span.range().end - block_base;
                    assert!(
                        starts.contains(&relative_end),
                        "width {width}: span ends inside cluster at byte {relative_end}"
                    );
                }
            }
        }

        let narrow = rendered(&layout_document(&document, 4), &document).join("");
        assert!(narrow.contains('\u{1F468}'));
        assert!(narrow.contains('\u{1F1FA}'));
    }

    #[test]
    fn lay_003_cjk_lines_break_deterministically_at_opportunities() {
        let document = doc("漢字のテスト。\n");
        let rows = rendered(&layout_document(&document, 6), &document);

        assert_eq!(rows, ["漢字の", "テス", "ト。"]);
    }

    #[test]
    fn lay_004_tabs_and_control_bytes_render_as_safe_visible_text() {
        let document = doc("col\tumn\u{1B}end\n");
        let rows = rendered(&layout_document(&document, 40), &document);

        assert_eq!(rows.len(), 1);
        let text = &rows[0];
        assert!(text.contains("col     umn"), "tab expands: {text}");
        assert!(text.contains("^[end"), "escape becomes caret: {text}");
        assert!(
            !text
                .chars()
                .any(|c| matches!(u32::from(c), 0x00..=0x1F | 0x7F))
        );
    }

    #[test]
    fn lay_005_span_ranges_are_ordered_in_bounds_and_reconstruct_content() {
        let document = doc("first paragraph wraps here\n\nsecond one follows after blanks\n");
        let layout = layout_document(&document, 12);
        let canonical_len = document.len();

        let mut last_end = 0usize;
        for row in layout.rows() {
            for span in row.spans() {
                let range = span.range();
                assert!(range.start >= last_end, "ranges stay ordered");
                assert!(range.end <= canonical_len, "ranges stay in bounds");
                last_end = range.end;
            }
        }

        for (index, block) in document.sections()[0].blocks().iter().enumerate() {
            if block.kind() != BlockKind::Paragraph {
                continue;
            }
            let mut reconstructed = String::new();
            for row in layout.rows().iter().filter(|row| row.block() == index) {
                for span in row.spans() {
                    reconstructed.push_str(&span.visible(&document, 0));
                }
            }
            let mut expected = visible_text(document.block_text(0, index).expect("block text"), 0);
            if expected.ends_with(' ') {
                expected.pop();
            }
            assert_eq!(reconstructed, expected, "block {index} reconstructs");
        }
    }

    #[test]
    fn lay_006_resizing_keeps_the_anchor_passage_located() {
        let document = doc("alpha beta gamma delta epsilon zeta eta theta iota kappa\n");
        let anchor_byte = 24usize;

        for width in [40u16, 15, 60, 9] {
            let layout = layout_document(&document, width);
            let index = layout.row_after(anchor_byte);
            assert!(index < layout.rows().len());
            let covered: usize = layout.rows()[..=index]
                .iter()
                .flat_map(VisualRow::spans)
                .map(|span| span.range().end)
                .max()
                .unwrap_or_default();
            assert!(covered > anchor_byte, "width {width} covers the anchor");
        }
    }

    #[test]
    fn lay_008_degenerate_geometry_makes_progress_without_panicking() {
        let cases = [
            ("", 5_u16),
            ("unbreakableunbreakableunbreakable", 1),
            ("      deeply indented", 3),
            ("a", 1),
            ("word word word", 2),
        ];
        for (text, width) in cases {
            let document = doc(text);
            let layout = layout_document(&document, width);
            for row in layout.rows() {
                assert!(row.cells() <= width.max(1));
            }
        }

        let long_run = format!("{}\n", "x".repeat(500));
        let document = doc(&long_run);
        let layout = layout_document(&document, 16);
        assert_eq!(layout.rows().len(), 500 / 16 + 1);
    }

    #[test]
    fn hard_wrapped_lines_reflow_into_flowing_paragraphs() {
        let document = doc("one two\nthree four\n\nnext para\n");
        let rows = rendered(&layout_document(&document, 80), &document);

        assert_eq!(rows, ["one two three four", "", "next para"]);
    }

    #[test]
    fn zero_width_layout_is_empty_for_the_too_small_state() {
        let document = doc("content\n");
        assert!(layout_document(&document, 0).rows().is_empty());
    }
}
