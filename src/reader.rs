//! Reading state: the logical anchor, reading modes, and navigation.
//!
//! Every navigation action moves one validated logical position; viewports
//! are derived later from that anchor plus a layout. Nothing here depends on
//! a terminal, so all movement rules stay unit- and property-testable.

use crate::document::{Document, Position};
use crate::layout::PageLayout;

/// How the viewport advances through the document.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    /// One content viewport per step; the default reading mode.
    #[default]
    Paged,
    /// Scrolling by visual rows through the same layout.
    Continuous,
}

impl Mode {
    /// Short label for the status line; never color-encoded.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paged => "PAGED",
            Self::Continuous => "CONT",
        }
    }

    /// The other mode, for single-key toggling.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Paged => Self::Continuous,
            Self::Continuous => Self::Paged,
        }
    }
}

/// Direction for line, page, and section steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    TowardStart,
    TowardEnd,
}

/// Moves the anchor to the start of the next or previous content row.
///
/// Blank spacer rows never become anchors, so a forward step followed by a
/// backward step returns to the exact original position. Returns `None` at
/// the corresponding document boundary.
#[must_use]
pub fn step_line(
    layout: &PageLayout,
    document: &Document,
    anchor: Position,
    direction: Direction,
) -> Option<Position> {
    let current = layout.row_after(anchor.absolute_byte(document));
    let target = match direction {
        Direction::TowardEnd => next_content_row(layout, current + 1)?,
        Direction::TowardStart if current == 0 => return None,
        Direction::TowardStart => previous_content_row(layout, current - 1)?,
    };
    row_start_position(layout, document, target)
}

/// Moves the anchor by one viewport of rows, clamping at the ends.
///
/// Paging counts visual rows from the anchor's own row, so repeated
/// next-page steps advance until the clamp. The backward step is defined as
/// the inverse of the unclamped forward step: it restores the prior anchor
/// exactly whenever that forward hop fit inside the document and no blank
/// spacer rows were skipped past. Blank joiners may otherwise shorten the
/// hop, and the step never moves toward the end or below the document start.
#[must_use]
pub fn step_page(
    layout: &PageLayout,
    document: &Document,
    anchor: Position,
    viewport_rows: usize,
    direction: Direction,
) -> Option<Position> {
    if layout.rows().is_empty() || viewport_rows == 0 {
        return None;
    }
    let current = layout.row_after(anchor.absolute_byte(document));
    let last_index = layout.rows().len() - 1;
    let target = match direction {
        Direction::TowardEnd => {
            let jumped = current.saturating_add(viewport_rows).min(last_index);
            next_content_row(layout, jumped)
        }
        Direction::TowardStart if current == 0 => return None,
        Direction::TowardStart => inverse_page_step(layout, current, viewport_rows).or_else(|| {
            let jumped = current - viewport_rows.min(current);
            previous_content_row(layout, jumped)
        }),
    }?;
    row_start_position(layout, document, target)
}

/// Finds the smallest content row whose forward page step lands exactly on
/// `current`, making the backward step a true inverse of the forward step.
///
/// The search stays inside one bounded window below the anchor; layouts with
/// more consecutive blank rows than the window fall back to the plain hop.
fn inverse_page_step(layout: &PageLayout, current: usize, viewport_rows: usize) -> Option<usize> {
    let window = usize::from(u16::try_from(viewport_rows).unwrap_or(u16::MAX))
        .saturating_mul(2)
        .saturating_add(8);
    let upper = current.saturating_sub(1);
    let lower = current.saturating_sub(window);
    let mut candidate = lower;
    while candidate <= upper {
        if !layout.rows()[candidate].spans().is_empty() {
            let jumped = candidate
                .saturating_add(viewport_rows)
                .min(layout.rows().len() - 1);
            if next_content_row(layout, jumped) == Some(current) {
                return Some(candidate);
            }
        }
        candidate += 1;
    }
    None
}

/// Anchors the first content row of the document.
#[must_use]
pub fn jump_document_start(layout: &PageLayout, document: &Document) -> Option<Position> {
    row_start_position(layout, document, 0)
}

/// Anchors the very end of the document's logical text.
///
/// The position sits on the final byte boundary, so progress reads one
/// hundred percent when the reader reaches the end of the book.
#[must_use]
pub fn jump_document_end(document: &Document) -> Option<Position> {
    let section_index = document.sections().len().checked_sub(1)?;
    let blocks = document.sections()[section_index].blocks();
    let block_index = blocks.len().checked_sub(1)?;
    end_of_block(document, section_index, block_index)
}

/// Anchors the start of section `section`.
///
/// Phase 1 TXT books carry exactly one section, so this is also the document
/// start; the action stays distinct so later formats reuse the same key map.
#[must_use]
pub fn jump_section_start(
    _layout: &PageLayout,
    document: &Document,
    section: usize,
) -> Option<Position> {
    first_block_start(document, section)
}

/// Anchors the end of section `section`.
#[must_use]
pub fn jump_section_end(
    _layout: &PageLayout,
    document: &Document,
    section: usize,
) -> Option<Position> {
    let blocks = document.sections().get(section)?.blocks();
    let last = blocks.len().checked_sub(1)?;
    end_of_block(document, section, last)
}

/// Selects the exact previous or next navigation point in declared order.
///
/// The selected index is part of the reading state because distinct entries
/// may have the same destination. With no current selection, movement enters
/// the list at the corresponding end.
#[must_use]
pub fn step_section(
    document: &Document,
    current: Option<usize>,
    direction: Direction,
) -> Option<(usize, Position)> {
    let points = document.navigation_points();
    let target = match (current.filter(|index| *index < points.len()), direction) {
        (Some(index), Direction::TowardStart) => index.checked_sub(1)?,
        (Some(index), Direction::TowardEnd) => index.checked_add(1)?,
        (None, Direction::TowardStart) => points.len().checked_sub(1)?,
        (None, Direction::TowardEnd) => 0,
    };
    points.get(target).map(|point| (target, point.position()))
}

fn first_block_start(document: &Document, section: usize) -> Option<Position> {
    let blocks = document.sections().get(section)?.blocks();
    for block in 0..blocks.len() {
        if let Ok(position) = document.position(section, block, 0) {
            return Some(position);
        }
    }
    None
}

fn next_content_row(layout: &PageLayout, from: usize) -> Option<usize> {
    (from..layout.rows().len()).find(|index| !layout.rows()[*index].spans().is_empty())
}

fn previous_content_row(layout: &PageLayout, from: usize) -> Option<usize> {
    (0..=from.min(layout.rows().len().saturating_sub(1)))
        .rev()
        .find(|index| !layout.rows()[*index].spans().is_empty())
}

fn row_start_position(layout: &PageLayout, document: &Document, index: usize) -> Option<Position> {
    let row = layout.rows().get(index)?;
    let span = row.spans().first()?;
    absolute_to_position(document, span.range().start)
}

fn end_of_block(document: &Document, section: usize, block: usize) -> Option<Position> {
    let range = document
        .sections()
        .get(section)?
        .blocks()
        .get(block)?
        .range();
    absolute_to_position(document, range.end)
}

fn absolute_to_position(document: &Document, byte: usize) -> Option<Position> {
    let sections = document.sections();
    for (section_index, section) in sections.iter().enumerate() {
        for (block_index, block) in section.blocks().iter().enumerate() {
            let range = block.range();
            if range.start > byte || byte > range.end {
                continue;
            }
            let mut offset = byte - range.start;
            while offset > 0 {
                if let Ok(position) = document.position(section_index, block_index, offset) {
                    return Some(position);
                }
                offset -= 1;
            }
            return document.position(section_index, block_index, 0).ok();
        }
    }
    None
}

/// Progress fraction numerator helpers for the status line.
///
/// The percentage is floored so forward movement is monotonic and resize or
/// theme changes never alter it.
pub mod progress {
    use super::{Document, Position};

    /// Whole-number percentage of the book read at `anchor`, floored.
    #[must_use]
    pub fn percent(document: &Document, anchor: Position) -> u8 {
        let total = document.len();
        if total == 0 {
            return 0;
        }
        let byte = anchor.absolute_byte(document);
        let clamped = byte.min(total);
        let whole = u64::try_from(clamped).unwrap_or(u64::MAX) * 100
            / u64::try_from(total).unwrap_or(u64::MAX);
        whole.min(100) as u8
    }

    /// One-based logical line containing the anchor.
    #[must_use]
    pub fn location_line(document: &Document, anchor: Position) -> usize {
        document.logical_line_number(anchor.absolute_byte(document))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Direction, Document, Mode, PageLayout, Position, absolute_to_position, jump_document_end,
        jump_section_end, jump_section_start, progress, step_line, step_page,
    };
    use crate::document::{DocumentId, PositionError, text::document_from_text};
    use crate::layout::layout_document;

    const TEXT: &str = "alpha beta gamma\ndelta epsilon zeta\n\n\
                        eta theta iota\nkappa lambda mu\nnu xi omicron\n";

    fn fixture(width: u16) -> (Document, PageLayout, Position) {
        let document = document_from_text(DocumentId::new("nav".to_owned()), None, TEXT)
            .expect("fixture parses");
        let layout = layout_document(&document, width);
        let start = document.first_position().expect("start");
        (document, layout, start)
    }

    fn anchor_at(document: &Document, layout: &PageLayout, byte: usize) -> Position {
        let index = layout.row_after(byte);
        let row = &layout.rows()[index];
        let span = row.spans().first().expect("content row");
        absolute_to_position(document, span.range().start).expect("valid")
    }

    #[test]
    fn nav_001_next_then_previous_line_restores_the_anchor() {
        let (document, layout, start) = fixture(20);

        let forward = step_line(&layout, &document, start, Direction::TowardEnd)
            .expect("moves away from start");
        assert_ne!(forward, start);

        let restored =
            step_line(&layout, &document, forward, Direction::TowardStart).expect("returns");
        assert_eq!(restored, start);
    }

    #[test]
    fn nav_001_line_steps_land_on_content_not_blank_rows() {
        let (document, layout, start) = fixture(20);
        let mut anchor = start;

        for _ in 0..6 {
            let Some(next) = step_line(&layout, &document, anchor, Direction::TowardEnd) else {
                break;
            };
            let row = layout.row_after(next.absolute_byte(&document));
            assert!(!layout.rows()[row].spans().is_empty());
            anchor = next;
        }
    }

    #[test]
    fn nav_002_repeated_next_page_advances_until_the_clamp() {
        let (document, layout, start) = fixture(18);
        let mut anchor = start;
        let mut movements = 0;

        while let Some(next) = step_page(&layout, &document, anchor, 3, Direction::TowardEnd) {
            assert!(next.absolute_byte(&document) >= anchor.absolute_byte(&document));
            if next == anchor {
                break;
            }
            anchor = next;
            movements += 1;
            assert!(movements < 50, "no loop");
        }
        assert!(movements > 0);

        let end = anchor_at(&document, &layout, document.len().saturating_sub(1));
        assert_eq!(anchor, end, "clamps at the final content");
    }

    #[test]
    fn nav_003_previous_page_after_next_page_returns_without_resize() {
        let (document, layout, start) = fixture(18);

        let paged_down =
            step_page(&layout, &document, start, 4, Direction::TowardEnd).expect("page down");
        let back =
            step_page(&layout, &document, paged_down, 4, Direction::TowardStart).expect("page up");

        assert_eq!(back, start);
    }

    #[test]
    fn nav_004_boundary_navigation_clamps_safely() {
        let (document, layout, start) = fixture(20);

        assert_eq!(
            step_line(&layout, &document, start, Direction::TowardStart),
            None
        );
        assert_eq!(
            step_page(&layout, &document, start, 5, Direction::TowardStart),
            None
        );

        let end = jump_document_end(&document).expect("end exists");
        assert_eq!(
            step_line(&layout, &document, end, Direction::TowardEnd),
            None,
            "the last row has no successor"
        );
    }

    #[test]
    fn nav_012_continuous_mode_moves_exactly_one_visual_row() {
        let (document, layout, start) = fixture(24);
        let first =
            step_line(&layout, &document, start, Direction::TowardEnd).expect("one row forward");

        let rows_moved = i64::try_from(layout.row_after(first.absolute_byte(&document)))
            .unwrap_or(i64::MAX)
            - i64::try_from(layout.row_after(start.absolute_byte(&document))).unwrap_or(i64::MAX);
        assert_eq!(rows_moved, 1);

        let back =
            step_line(&layout, &document, first, Direction::TowardStart).expect("one row backward");
        assert_eq!(back, start);
    }

    #[test]
    fn mode_toggle_preserves_labels_and_round_trip() {
        assert_eq!(Mode::default(), Mode::Paged);
        assert_eq!(Mode::Paged.label(), "PAGED");
        assert_eq!(Mode::Continuous.label(), "CONT");
        assert_eq!(Mode::Paged.toggled().toggled(), Mode::Paged);
    }

    #[test]
    fn progress_percent_is_floored_and_monotonic() {
        let (document, _, _) = fixture(80);
        let layout = layout_document(&document, 80);
        let start = document.first_position().expect("start");
        let mid = anchor_at(&document, &layout, document.len() / 2);
        let end = jump_document_end(&document).expect("end");

        let low = progress::percent(&document, start);
        let middle = progress::percent(&document, mid);
        let high = progress::percent(&document, end);
        assert!(low <= middle && middle <= high);
        assert_eq!(low, 0);
        assert_eq!(high, 100);
    }

    #[test]
    fn positions_survive_width_changes_for_the_same_passage() {
        let (document, _, start) = fixture(60);
        let byte = start.absolute_byte(&document);

        let narrow_layout = layout_document(&document, 14);
        let relocated = anchor_at(&document, &narrow_layout, byte);
        let rebuilt = document
            .position(0, relocated.block(), relocated.offset())
            .expect("relocated stays valid");
        assert_eq!(rebuilt.absolute_byte(&document), byte);
    }

    #[test]
    fn nav_014_section_and_document_jumps_are_four_distinct_actions() {
        let (document, layout, _) = fixture(40);

        let section_start = jump_section_start(&layout, &document, 0).expect("section start");
        let document_start = document.first_position().expect("document start");
        assert_eq!(section_start, document_start);

        let section_end = jump_section_end(&layout, &document, 0).expect("section end");
        let document_end = jump_document_end(&document).expect("document end");
        assert_eq!(section_end, document_end);

        assert_ne!(section_start, section_end);
    }

    #[test]
    fn nav_005_section_steps_follow_declared_order_with_duplicate_positions() {
        use crate::document::{Block, BlockKind, NavigationPoint, Section};

        let base = Document::from_sections(
            DocumentId::new("nav005".to_owned()),
            None,
            "one\ntwo\nthree".to_owned(),
            vec![
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 0..4)]),
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 4..8)]),
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 8..13)]),
            ],
        )
        .expect("tiles");
        let first = base.position(0, 0, 0).expect("first");
        let exact_middle = base.position(1, 0, 1).expect("fragment offset");
        let last = base.position(2, 0, 0).expect("last");
        let document = base.with_navigation(vec![
            NavigationPoint::new("Three first", last),
            NavigationPoint::new("One", first),
            NavigationPoint::new("One duplicate", first),
            NavigationPoint::new("Two detail", exact_middle),
        ]);

        let expected = [
            ("Three first", last),
            ("One", first),
            ("One duplicate", first),
            ("Two detail", exact_middle),
        ];
        for (point, (title, position)) in document.navigation_points().iter().zip(expected) {
            assert_eq!(point.title(), title);
            assert_eq!(point.position(), position);
        }
        for index in 0..expected.len() - 1 {
            assert_eq!(
                super::step_section(&document, Some(index), Direction::TowardEnd),
                Some((index + 1, expected[index + 1].1))
            );
            assert_eq!(
                super::step_section(&document, Some(index + 1), Direction::TowardStart),
                Some((index, expected[index].1))
            );
        }
        assert_eq!(
            super::step_section(&document, None, Direction::TowardEnd),
            Some((0, last))
        );
        assert_eq!(
            super::step_section(&document, None, Direction::TowardStart),
            Some((expected.len() - 1, exact_middle))
        );
    }

    #[test]
    fn section_steps_cross_empty_sections_with_the_same_absolute_byte() {
        use crate::document::{Block, BlockKind, Section};

        let document = Document::from_sections(
            DocumentId::new("empty-sections".to_owned()),
            None,
            "one".to_owned(),
            vec![
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 0..3)]),
                Section::new(None, Vec::new()),
                Section::new(None, Vec::new()),
            ],
        )
        .expect("empty sections are valid");
        let points = document.navigation_points();
        assert_eq!(points[1].position().absolute_byte(&document), 3);
        assert_eq!(points[2].position().absolute_byte(&document), 3);

        let second =
            super::step_section(&document, Some(0), Direction::TowardEnd).expect("second point");
        let third = super::step_section(&document, Some(second.0), Direction::TowardEnd)
            .expect("third point");
        assert_eq!(second, (1, points[1].position()));
        assert_eq!(third, (2, points[2].position()));
        assert_eq!(
            super::step_section(&document, Some(third.0), Direction::TowardStart),
            Some(second)
        );
    }

    #[test]
    fn invalid_positions_surface_typed_errors_not_panics() {
        let (document, _, _) = fixture(40);
        assert_eq!(
            document.position(0, 999, 0),
            Err(PositionError::Block {
                section: 0,
                block: 999
            })
        );
    }
}
