//! Viewport slicing: which laid-out rows appear on screen.
//!
//! Converts a layout plus an anchor into per-row cells carrying visible
//! text and its inline role. Staying string-based keeps this module free of
//! any UI dependency while giving the renderer exactly what it must paint.

use crate::document::{Document, InlineKind};

use super::{PageLayout, display_width};

/// One paintable piece of a row: text plus its semantic role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowCell {
    /// Visible text already transformed (tabs expanded, controls escaped).
    pub text: String,
    /// Inline role for styling; plain when absent.
    pub decoration: Option<InlineKind>,
}

/// Builds the visible row cells for one viewport of `height` rows.
///
/// The anchor's row becomes the first visible row; when fewer rows remain,
/// the window clamps so the final page still fills from the top.
#[must_use]
pub fn viewport_row_texts(
    document: &Document,
    layout: &PageLayout,
    anchor_byte: usize,
    height: u16,
) -> Vec<Vec<RowCell>> {
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
        let mut cells: Vec<RowCell> = Vec::with_capacity(row.spans().len());
        if !row.prefix().is_empty() {
            cells.push(RowCell {
                text: row.prefix().to_owned(),
                decoration: None,
            });
        }
        let mut column = 0u16;
        for (index, span) in row.spans().iter().enumerate() {
            let visible = span.visible(document, column);
            column += display_width(&visible, column);
            if !visible.is_empty() {
                cells.push(RowCell {
                    text: visible,
                    decoration: span.decoration(),
                });
            }
            if let Some(pad) = row.padding().get(index)
                && *pad > 0
            {
                cells.push(RowCell {
                    text: " ".repeat(usize::from(*pad)),
                    decoration: None,
                });
            }
        }
        viewport.push(cells);
    }
    viewport
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::text::TextLimits;
    use crate::document::{DocumentId, InlineKind, markdown, text::document_from_text};
    use crate::layout::layout_document;

    #[test]
    fn nav_011_default_open_pages_from_the_first_content_row() {
        let document =
            document_from_text(DocumentId::new("v".into()), None, "one\n\ntwo\n\nthree\n")
                .expect("fixture");
        let layout = layout_document(&document, 40);
        let anchor = document.first_position().expect("start");

        let rows = viewport_row_texts(&document, &layout, anchor.absolute_byte(&document), 2);
        assert_eq!(
            rows,
            [
                vec![RowCell {
                    text: "one".to_owned(),
                    decoration: None,
                }],
                Vec::new()
            ]
        );
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

    #[test]
    fn epub_011_decorated_cells_carry_inline_roles_to_the_renderer() {
        let document = markdown::load_markdown_bytes(
            "decorated.md",
            b"plain *tilted* **heavy** `code` end\n",
            &TextLimits::default(),
        )
        .expect("parses");
        let layout = layout_document(&document, 60);
        let rows = viewport_row_texts(&document, &layout, 0, 5);

        let flat: Vec<(String, Option<InlineKind>)> = rows
            .iter()
            .flatten()
            .map(|cell| (cell.text.clone(), cell.decoration))
            .collect();
        assert!(
            flat.iter()
                .any(|(text, decoration)| decoration.is_none() && text.starts_with("plain")),
            "plain text stays undecorated: {flat:?}"
        );
        assert!(
            flat.contains(&("tilted".to_owned(), Some(InlineKind::Emphasis))),
            "emphasis reaches the renderer: {flat:?}"
        );
        assert!(flat.contains(&("heavy".to_owned(), Some(InlineKind::Strong))));
        assert!(flat.contains(&("code".to_owned(), Some(InlineKind::Code))));
    }
}
