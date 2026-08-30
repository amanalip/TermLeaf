//! Grapheme-aware, cell-width-aware layout with source mapping.
//!
//! One layout pass turns the canonical logical text into visual rows for one
//! viewport width. Every row records the byte ranges it renders, ordered and
//! in bounds, so navigation can always map a visual row back to logical
//! content and resize can relocate the same passage. This module depends on
//! neither Ratatui nor Crossterm.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::document::{BlockKind, Document, InlineKind};

mod width;

pub use width::{caret_notation, display_width, tab_advance, visible_text};
pub mod viewport;

/// Rows reserved by an image block while its bounded preview is loading.
pub const IMAGE_PLACEHOLDER_ROWS: usize = 6;

/// One rendered piece of a row: the raw byte range it displays.
///
/// Rendering applies [`visible_text`] with the running column, so tabs,
/// control bytes, and newline join markers transform identically here and in
/// the UI without storing duplicated strings. An optional decoration carries
/// the inline semantic role that styling layers may apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    range: Range<usize>,
    decoration: Option<InlineKind>,
}

impl Span {
    /// The raw canonical bytes this span renders.
    #[must_use]
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// The inline semantic role carried by this span, if any.
    #[must_use]
    pub const fn decoration(&self) -> Option<InlineKind> {
        self.decoration
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
///
/// `prefix` holds synthesized leading text such as list markers or quote
/// bars; it is presentation only and never part of the canonical text or of
/// any position. `padding` spaces align table columns after each span.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VisualRow {
    section: usize,
    block: usize,
    prefix: String,
    spans: Vec<Span>,
    padding: Vec<u16>,
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

    /// Synthesized leading text, such as list markers or quote bars.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Padding cells inserted after each span for table alignment.
    #[must_use]
    pub const fn padding(&self) -> &Vec<u16> {
        &self.padding
    }

    /// Spans in left-to-right order.
    #[must_use]
    pub const fn spans(&self) -> &Vec<Span> {
        &self.spans
    }

    /// Measured cell count, identical to the sum of rendered widths.
    #[must_use]
    pub const fn cells(&self) -> u16 {
        self.cells
    }

    /// The complete visible text of this row, including its prefix.
    ///
    /// Padding spaces after each span reproduce aligned table columns; the
    /// result is exactly what the renderer paints.
    #[must_use]
    pub fn text(&self, document: &Document) -> String {
        let mut out = String::new();
        out.push_str(&self.prefix);
        let mut column = display_width(&self.prefix, 0);
        for (index, span) in self.spans.iter().enumerate() {
            let visible = span.visible(document, column);
            column += display_width(&visible, column);
            out.push_str(&visible);
            if let Some(pad) = self.padding.get(index) {
                let pad = " ".repeat(usize::from(*pad));
                column += u16::try_from(pad.len()).unwrap_or(u16::MAX);
                out.push_str(&pad);
            }
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
    // Ordered-list numbering restarts whenever a non-list block intervenes;
    // counters are per depth so nested lists number independently.
    let mut list_counters = [0u64; 8];
    for (section_index, section) in document.sections().iter().enumerate() {
        for (block_index, block) in section.blocks().iter().enumerate() {
            match block.kind() {
                BlockKind::BlankLine => rows.push(VisualRow {
                    section: section_index,
                    block: block_index,
                    ..VisualRow::default()
                }),
                BlockKind::Paragraph | BlockKind::Separator | BlockKind::Heading { .. } => {
                    list_counters = [0u64; 8];
                    let text = document
                        .block_text(section_index, block_index)
                        .unwrap_or_default();
                    wrap_paragraph(
                        document,
                        text,
                        block.range().start,
                        section_index,
                        block_index,
                        width,
                        &mut rows,
                    );
                }
                BlockKind::Image => {
                    list_counters = [0u64; 8];
                    append_image_rows(
                        document,
                        section_index,
                        block_index,
                        block.range().start,
                        width,
                        &mut rows,
                    );
                }
                BlockKind::ListItem { depth, ordered } => {
                    let depth_index = usize::from(depth).min(list_counters.len() - 1);
                    if ordered {
                        list_counters[depth_index] += 1;
                    }
                    let marker = if ordered {
                        format!("{}. ", list_counters[depth_index])
                    } else {
                        "\u{2022} ".to_owned()
                    };
                    let indent = " ".repeat(usize::from(display_width(&marker, 0)));
                    let prefix = format!("{}{}", "  ".repeat(usize::from(depth)), marker);
                    let continuation = format!("{}{}", "  ".repeat(usize::from(depth)), indent);
                    wrap_prefixed(
                        document,
                        RowOwner {
                            section: section_index,
                            block: block_index,
                        },
                        block.range(),
                        width,
                        (&prefix, &continuation),
                        &mut rows,
                    );
                }
                BlockKind::Quote => {
                    wrap_prefixed(
                        document,
                        RowOwner {
                            section: section_index,
                            block: block_index,
                        },
                        block.range(),
                        width,
                        ("> ", "> "),
                        &mut rows,
                    );
                }
                BlockKind::CodeBlock => {
                    code_block_rows(
                        document,
                        section_index,
                        block_index,
                        block.range(),
                        width,
                        &mut rows,
                    );
                }
                BlockKind::Table => {
                    table_rows(
                        document,
                        section_index,
                        block_index,
                        block.range(),
                        block.cells(),
                        width,
                        &mut rows,
                    );
                }
            }
        }
    }
    rows
}

fn append_image_rows(
    document: &Document,
    section: usize,
    block: usize,
    start: usize,
    width: u16,
    rows: &mut Vec<VisualRow>,
) {
    let first_row = rows.len();
    let text = document.block_text(section, block).unwrap_or_default();
    wrap_paragraph(document, text, start, section, block, width, rows);
    let caption_rows = rows.len().saturating_sub(first_row);
    while rows.len().saturating_sub(first_row + caption_rows) < IMAGE_PLACEHOLDER_ROWS {
        rows.push(VisualRow {
            section,
            block,
            ..VisualRow::default()
        });
    }
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
    document: &Document,
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

    let mut flow = FlowContext {
        owner: RowOwner { section, block },
        width,
        pending_join: &mut pending_join,
        current: &mut current,
        rows,
    };
    for atom in paragraph_atoms(text, base) {
        match atom {
            Atom::Join { newline } => {
                if !flow.current.spans.is_empty() {
                    *flow.pending_join = Some(newline);
                }
            }
            Atom::Content {
                text: piece,
                offset,
                ..
            } => {
                // Subdivide the atom at decoration boundaries so every
                // placed span carries exactly one inline role.
                let mut cursor = offset.start;
                while cursor < offset.end {
                    let run_end = match document.inline_kind_at(cursor) {
                        Some(_) => document
                            .inline_spans()
                            .iter()
                            .find(|span| span.range().contains(&cursor))
                            .map_or(offset.end, |span| span.range().end.min(offset.end)),
                        None => document
                            .inline_spans()
                            .iter()
                            .map(|span| span.range().start)
                            .find(|start| *start > cursor && *start < offset.end)
                            .unwrap_or(offset.end),
                    };
                    flow.place(
                        &piece[cursor - offset.start..run_end - offset.start],
                        cursor..run_end,
                        document.inline_kind_at(cursor),
                    );
                    cursor = run_end;
                }
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

/// Placement context shared by the wrapping helpers.
struct FlowContext<'a> {
    owner: RowOwner,
    width: u16,
    pending_join: &'a mut Option<usize>,
    current: &'a mut VisualRow,
    rows: &'a mut Vec<VisualRow>,
}

impl FlowContext<'_> {
    fn place(&mut self, piece: &str, offset: Range<usize>, decoration: Option<InlineKind>) {
        let mut start = 0usize;
        while start < piece.len() {
            let remaining = &piece[start..];
            let starts_new_row = self.current.spans.is_empty();
            let joins = usize::from(!starts_new_row && self.pending_join.is_some());
            let base = self.current.cells + u16::try_from(joins).unwrap_or(1);
            let need = display_width(remaining, base);

            if !starts_new_row && base + need > self.width {
                flush_row(
                    self.current,
                    self.rows,
                    self.owner.section,
                    self.owner.block,
                );
                *self.pending_join = None;
                continue;
            }
            if starts_new_row {
                *self.pending_join = None;
            }

            if joins == 1 {
                let newline = self.pending_join.take().unwrap_or_default();
                self.current.spans.push(Span {
                    range: newline..newline + 1,
                    decoration: None,
                });
                self.current.cells += 1;
            }

            let free = self.width - self.current.cells;
            let whole = display_width(remaining, self.current.cells);
            if whole <= free {
                self.current.spans.push(Span {
                    range: offset.start + start..offset.end,
                    decoration,
                });
                self.current.cells += whole;
                return;
            }

            let chunk = force_fit_chunk(remaining, free, self.current.cells);
            let end = start + chunk.len();
            let column = self.current.cells;
            self.current.spans.push(Span {
                range: offset.start + start..offset.start + end,
                decoration,
            });
            self.current.cells += display_width(chunk, column);
            flush_row(
                self.current,
                self.rows,
                self.owner.section,
                self.owner.block,
            );
            start = end;
        }
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

/// Wraps one block whose every visual row starts with a synthesized prefix.
///
/// List items and quotes keep their marker or bar on every row, so wrapped
/// content stays visually grouped. Prefix bytes are presentation only: they
/// never enter the canonical text, positions, or search. Placement runs
/// against the width remaining under the prefix.
fn wrap_prefixed(
    document: &Document,
    owner: RowOwner,
    range: &Range<usize>,
    width: u16,
    prefixes: (&str, &str),
    rows: &mut Vec<VisualRow>,
) {
    let head_cells = display_width(prefixes.0, 0);
    let tail_cells = display_width(prefixes.1, 0);
    let mut inner = Vec::new();
    wrap_paragraph(
        document,
        document
            .block_text(owner.section, owner.block)
            .unwrap_or_default(),
        range.start,
        owner.section,
        owner.block,
        width.saturating_sub(head_cells),
        &mut inner,
    );

    for (index, mut row) in inner.into_iter().enumerate() {
        let (prefix, cells) = if index == 0 {
            (prefixes.0, head_cells)
        } else {
            (prefixes.1, tail_cells)
        };
        row.prefix = String::from(prefix);
        row.cells = row.cells.saturating_add(cells);
        rows.push(row);
    }
}

/// Emits one verbatim row per source line of a code block.
///
/// Lines never reflow and keep their original indentation; overlong lines
/// hard-split across rows without join markers, and interior blank lines
/// stay blank so code shape survives visually.
fn code_block_rows(
    document: &Document,
    section: usize,
    block: usize,
    range: &Range<usize>,
    width: u16,
    rows: &mut Vec<VisualRow>,
) {
    let text = document.block_text(section, block).unwrap_or_default();
    let mut consumed = 0usize;
    for line in text.split_terminator('\n') {
        let line_base = range.start + consumed;
        consumed += line.len() + 1;
        emit_verbatim_line(section, block, line_base, line, width, rows);
    }
}

fn emit_verbatim_line(
    section: usize,
    block: usize,
    base: usize,
    line: &str,
    width: u16,
    rows: &mut Vec<VisualRow>,
) {
    if line.is_empty() {
        rows.push(VisualRow {
            section,
            block,
            ..VisualRow::default()
        });
        return;
    }
    let mut start = 0usize;
    while start < line.len() {
        let remaining = &line[start..];
        let mut row = VisualRow {
            section,
            block,
            ..VisualRow::default()
        };
        let whole = display_width(remaining, row.cells);
        let taken = if whole <= width {
            remaining.len()
        } else {
            force_fit_chunk(remaining, width, row.cells).len()
        };
        row.spans.push(Span {
            range: base + start..base + start + taken,
            decoration: None,
        });
        row.cells += display_width(&remaining[..taken], row.cells);
        rows.push(row);
        start += taken;
    }
}

/// Lays out one table either as aligned columns or as linearized lines.
///
/// Column widths come from the widest cell per column. When the aligned
/// form fits the available width, each source row becomes one visual row:
/// cell spans carry padding spaces that right-align every column while the
/// canonical ` | ` delimiters still render between them. When it does not
/// fit, the table falls back to ordinary wrapping over its delimited
/// lines, which preserves every cell's content and order.
fn table_rows(
    document: &Document,
    section: usize,
    block: usize,
    range: &std::ops::Range<usize>,
    cells: &[Range<usize>],
    width: u16,
    rows: &mut Vec<VisualRow>,
) {
    let text = document.block_text(section, block).unwrap_or_default();
    if cells.is_empty() {
        wrap_paragraph(document, text, range.start, section, block, width, rows);
        return;
    }

    // Group cells into source rows using the newline boundaries.
    let mut source_rows: Vec<Vec<&Range<usize>>> = vec![Vec::new()];
    let mut line_end = range.start + text.find('\n').unwrap_or(text.len());
    let mut cursor = range.start;
    for cell in cells {
        while cell.start >= line_end && cursor < range.start + text.len() {
            cursor = line_end + 1;
            let rest = &text[cursor - range.start..];
            line_end = cursor + rest.find('\n').unwrap_or(rest.len());
            source_rows.push(Vec::new());
        }
        source_rows.last_mut().expect("seeded").push(cell);
    }

    let column_count = source_rows.first().map_or(0, Vec::len);
    if column_count == 0 {
        wrap_paragraph(document, text, range.start, section, block, width, rows);
        return;
    }

    let mut column_widths = vec![0u16; column_count];
    for row in &source_rows {
        for (index, cell) in row.iter().enumerate() {
            if index < column_count {
                let measured = display_width(text.get(cell.start..cell.end).unwrap_or_default(), 0);
                column_widths[index] = column_widths[index].max(measured);
            }
        }
    }
    let delimiters = u16::try_from(column_count.saturating_sub(1))
        .unwrap_or(0)
        .saturating_mul(3);
    let natural = column_widths
        .iter()
        .copied()
        .fold(0u16, u16::saturating_add)
        .saturating_add(delimiters);

    if natural > width || source_rows.iter().any(|row| row.len() != column_count) {
        // Linearized fallback: the delimited lines wrap like prose.
        wrap_paragraph(document, text, range.start, section, block, width, rows);
        return;
    }

    let total_rows = source_rows.len();
    for (row_index, row) in source_rows.iter().enumerate() {
        let mut current = VisualRow {
            section,
            block,
            ..VisualRow::default()
        };
        for (index, cell) in row.iter().enumerate() {
            let cell_text = text.get(cell.start..cell.end).unwrap_or_default();
            let cell_cells = display_width(cell_text, 0);
            current.spans.push(Span {
                range: cell.start..cell.end,
                decoration: None,
            });
            current.cells += cell_cells;
            if index + 1 < row.len() {
                current
                    .padding
                    .push(column_widths[index].saturating_sub(cell_cells));
                let delimiter_start = cell.end;
                let delimiter_end = row[index + 1].start;
                current.spans.push(Span {
                    range: delimiter_start..delimiter_end,
                    decoration: None,
                });
                current.padding.push(0);
                current.cells += u16::try_from(delimiter_end - delimiter_start).unwrap_or(0);
            }
        }
        // The newline between source rows rides at the end of its row so
        // spans alone still reconstruct the canonical table.
        if row_index + 1 < total_rows
            && let Some(next_start) = source_rows[row_index + 1].first().map(|cell| cell.start)
            && next_start > 0
            && text.as_bytes()[next_start - 1] == b'\n'
        {
            current.spans.push(Span {
                range: next_start - 1..next_start,
                decoration: None,
            });
            current.cells += 1;
        }
        rows.push(current);
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

#[cfg(test)]
mod semantic_tests {
    use super::*;
    use crate::document::{markdown, text::TextLimits};

    fn md(source: &str) -> Document {
        markdown::load_markdown_bytes("semantic.md", source.as_bytes(), &TextLimits::default())
            .expect("fixture parses")
    }

    fn rows(document: &Document, width: u16) -> Vec<String> {
        layout_document(document, width)
            .rows()
            .iter()
            .map(|row| row.text(document))
            .collect()
    }

    #[test]
    fn lay_009_aligned_tables_keep_columns_and_reconstruct_exactly() {
        let document = md("| Tree | Age |\n|---|---|\n| Oak | 300 years |\n| Fig | 9 |\n");
        let rendered = rows(&document, 40);

        assert_eq!(
            rendered,
            ["Tree | Age ", "Oak  | 300 years ", "Fig  | 9"],
            "columns align through padding; inter-row newlines render as blanks"
        );

        // Reconstruction: span ranges cover every canonical table byte.
        let layout = layout_document(&document, 40);
        for row in layout.rows() {
            assert!(row.cells() <= 40);
        }
        let joined: String = layout
            .rows()
            .iter()
            .flat_map(VisualRow::spans)
            .map(|span| &document.canonical()[span.range().clone()])
            .collect();
        assert_eq!(
            joined, "Tree | Age\nOak | 300 years\nFig | 9",
            "span bytes alone reconstruct the canonical table"
        );
    }

    #[test]
    fn lay_009_narrow_tables_linearize_without_losing_cells() {
        let document = md("| Tree | Age |\n|---|---|\n| Oak | 300 years |\n");
        let rendered = rows(&document, 12);
        let flattened = rendered.join("\n");

        // Every cell's words survive, and reading order stays row-major.
        let mut positions = Vec::new();
        for word in ["Tree", "Age", "Oak", "300", "years"] {
            let found = flattened.find(word).unwrap_or_else(|| {
                panic!("cell word '{word}' survives linearization: {flattened:?}")
            });
            positions.push(found);
        }
        let ordered = positions.windows(2).all(|pair| pair[0] <= pair[1]);
        assert!(ordered, "cells keep their order: {positions:?}");
        for row in layout_document(&document, 12).rows() {
            assert!(row.cells() <= 12);
        }
    }

    #[test]
    fn lay_010_code_blocks_render_one_verbatim_row_per_line() {
        let source = "```text\nfn keep() {\n    return 1;\n}\n```\n";
        let document = md(source);
        let rendered = rows(&document, 40);

        assert_eq!(
            rendered,
            ["fn keep() {", "    return 1;", "}",],
            "indentation and line shape survive exactly"
        );
    }

    #[test]
    fn lay_010_overlong_code_lines_hard_split_without_join_markers() {
        let source = format!("```text\n{}\n```\n", "x".repeat(30));
        let document = md(&source);
        for width in [10u16, 13] {
            let layout = layout_document(&document, width);
            for row in layout.rows() {
                assert!(row.cells() <= width);
            }
        }
        let narrow = rows(&document, 10);
        assert!(narrow.iter().all(|row| row.len() <= 10));
    }

    #[test]
    fn md_002_list_items_carry_markers_with_hanging_indent() {
        let source = "- alpha item\n- beta\n";
        let document = md(source);
        let rendered = rows(&document, 20);

        assert_eq!(rendered, ["\u{2022} alpha item", "\u{2022} beta",]);

        let wrapped_source = "- a very long list item that must wrap somewhere here\n";
        let document = md(wrapped_source);
        let layout = layout_document(&document, 14);
        let wrapped: Vec<String> = layout
            .rows()
            .iter()
            .map(|row| row.text(&document))
            .collect();
        assert!(wrapped.len() > 1, "the item wraps: {wrapped:?}");
        assert!(wrapped[0].starts_with("\u{2022} "));
        assert!(
            wrapped[1..].iter().all(|row| row.starts_with("  ")),
            "continuation indents to the marker width: {wrapped:?}"
        );
        for row in layout.rows() {
            assert!(row.cells() <= 14);
        }
    }

    #[test]
    fn md_002_ordered_lists_number_sequentially_per_list() {
        let source = "1. first\n2. second\n\ntext\n\n1. restarts\n";
        let rendered = rows(&md(source), 24);
        assert_eq!(
            rendered,
            ["1. first", "2. second", "", "text", "", "1. restarts"],
            "numbering restarts after intervening content"
        );
    }

    #[test]
    fn md_001_quotes_prefix_every_visual_row() {
        let source = "> a longer quoted passage that wraps across rows\n";
        let document = md(source);
        let rendered = rows(&document, 18);

        assert_eq!(rendered.len(), 4, "{rendered:?}");
        assert!(
            rendered.iter().all(|row| row.starts_with("> ")),
            "every row keeps the quote bar: {rendered:?}"
        );
        assert!(rendered[0].contains("a longer"));
    }

    #[test]
    fn epub_011_xhtml_semantics_flow_through_layout() {
        let source = "<body><ul><li>one</li><li>two</li></ul>\
                      <pre>a  b\n   c</pre><hr/><table><tr><td>P</td><td>Q</td></tr></table></body>";
        let blocks = crate::document::xhtml::convert_xhtml(source)
            .expect("within budget")
            .into_iter()
            .map(|block| (block.kind, block.text))
            .collect::<Vec<_>>();

        assert_eq!(
            blocks,
            [
                (
                    BlockKind::ListItem {
                        depth: 0,
                        ordered: false
                    },
                    "one".to_owned()
                ),
                (
                    BlockKind::ListItem {
                        depth: 0,
                        ordered: false
                    },
                    "two".to_owned()
                ),
                (BlockKind::CodeBlock, "a  b\n   c".to_owned()),
                (BlockKind::Separator, "* * *".to_owned()),
                (BlockKind::Table, "P | Q".to_owned()),
            ]
        );
    }

    #[test]
    fn md_001_decorations_survive_wrapping_across_row_boundaries() {
        let source = "plain *emphasized words spanning the wrap boundary* tail\n";
        let document = md(source);
        for width in [12u16, 16, 21] {
            let layout = layout_document(&document, width);
            let emphasized: Vec<String> = layout
                .rows()
                .iter()
                .flat_map(VisualRow::spans)
                .filter(|span| span.decoration() == Some(InlineKind::Emphasis))
                .map(|span| span.visible(&document, 0))
                .collect();
            let covered: String = emphasized.concat();
            assert_eq!(
                covered.split_whitespace().collect::<Vec<_>>().join(" "),
                "emphasized words spanning the wrap boundary",
                "width {width}: decoration covers the full run"
            );
        }
    }

    #[test]
    fn epub_013_image_captions_wrap_within_every_width() {
        let source = concat!(
            "before\n\n![a long alternative text that must wrap ",
            "somewhere across rows](pic.png)\n\nafter\n"
        );
        let document = md(source);
        for width in [10u16, 14, 22] {
            let layout = layout_document(&document, width);
            for row in layout.rows() {
                assert!(
                    row.cells() <= width,
                    "width {width}: {:?}",
                    row.text(&document)
                );
            }
            let rendered = rows(&document, width);
            assert!(
                rendered.iter().any(|row| row.starts_with("[image:")),
                "width {width}: the caption renders"
            );
            assert_eq!(rendered[0], "before");
            assert_eq!(*rendered.last().expect("rows"), "after");
        }
    }
}
