//! Viewport slicing: which laid-out rows appear on screen.
//!
//! Converts a layout plus an anchor into plain per-row span strings for one
//! viewport height. Staying string-based keeps this module free of any UI
//! dependency while giving the renderer exactly what it must paint.

use crate::document::Document;

use super::{PageLayout, display_width};

/// Builds the visible row texts for one viewport of `height` rows.
///
/// The anchor's row becomes the first visible row; when fewer rows remain,
/// the window clamps so the final page still fills from the top.
#[must_use]
pub fn viewport_row_texts(
    document: &Document,
    layout: &PageLayout,
    anchor_byte: usize,
    height: u16,
) -> Vec<Vec<String>> {
    let rows = layout.rows();
    if rows.is_empty() || height == 0 {
        return Vec::new();
    }
    let top = layout
        .row_after(anchor_byte)
        .min(rows.len().saturating_sub(1));
    let end = (top + usize::from(height)).min(rows.len());

    let mut viewport = Vec::with_capacity(end - top);
    for row in &rows[top..end] {
        let mut spans = Vec::with_capacity(row.spans().len());
        let mut column = 0u16;
        for span in row.spans() {
            let visible = span.visible(document, column);
            column += display_width(&visible, column);
            spans.push(visible);
        }
        viewport.push(spans);
    }
    viewport
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, text::document_from_text};
    use crate::layout::layout_document;

    #[test]
    fn nav_011_default_open_pages_from_the_first_content_row() {
        let document =
            document_from_text(DocumentId::new("v".into()), None, "one\n\ntwo\n\nthree\n")
                .expect("fixture");
        let layout = layout_document(&document, 40);
        let anchor = document.first_position().expect("start");

        let rows = viewport_row_texts(&document, &layout, anchor.absolute_byte(&document), 2);
        assert_eq!(rows, [vec!["one".to_owned()], Vec::new()]);
    }

    #[test]
    fn viewport_clamps_at_the_end_without_panicking() {
        let document =
            document_from_text(DocumentId::new("v".into()), None, "only line\n").expect("fixture");
        let layout = layout_document(&document, 40);

        let rows = viewport_row_texts(&document, &layout, 0, 50);
        assert_eq!(rows.len(), 1);

        let empty = viewport_row_texts(&document, &layout, 0, 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn zero_width_documents_yield_no_rows() {
        let document = document_from_text(DocumentId::new("v".into()), None, "").expect("e");
        let layout = layout_document(&document, 40);
        assert!(viewport_row_texts(&document, &layout, 0, 10).is_empty());
    }
}
