//! Cell-width measurement and safe visible representations.
//!
//! Layout and rendering share this module so measured widths always match
//! what the UI draws. Tabs expand to the next multiple-of-eight stop, C0
//! control bytes render as two-cell caret notation instead of reaching the
//! terminal, and paragraph-internal newline join markers render as spaces.

use unicode_width::UnicodeWidthChar;

/// Cells consumed by one expanded tab starting at `column`.
#[must_use]
pub const fn tab_advance(column: u16) -> u16 {
    8 - (column % 8)
}

/// The caret-notation pair for a control byte, when one is required.
#[must_use]
pub const fn caret_notation(byte: u8) -> Option<[char; 2]> {
    let letter = match byte {
        0x7F => '?',
        0x00 => '@',
        0x01..=0x1A => (b'A' + (byte - 1)) as char,
        0x1B => '[',
        0x1C => '\\',
        0x1D => ']',
        0x1E => '^',
        0x1F => '_',
        _ => return None,
    };
    Some(['^', letter])
}

fn is_control_char(ch: char) -> bool {
    matches!(u32::from(ch), 0x00..=0x1F | 0x7F)
}

/// Width in terminal cells of raw text rendered starting at `column`.
#[must_use]
pub fn display_width(raw: &str, column: u16) -> u16 {
    let mut width = 0u16;
    for ch in raw.chars() {
        match ch {
            '\t' => width += tab_advance(column + width),
            '\n' => width += 1,
            c if is_control_char(c) => width += 2,
            c => width += u16::try_from(c.width().unwrap_or(0)).unwrap_or(u16::MAX),
        }
    }
    width
}

/// The exact string a raw span renders as when it starts at `column`.
///
/// Newline join markers become single spaces, tabs expand to their stops,
/// and control bytes become caret pairs, matching [`display_width`] exactly.
#[must_use]
pub fn visible_text(raw: &str, column: u16) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut col = column;
    for ch in raw.chars() {
        match ch {
            '\t' => {
                let advance = tab_advance(col);
                out.extend(std::iter::repeat_n(' ', usize::from(advance)));
                col += advance;
            }
            '\n' => {
                out.push(' ');
                col += 1;
            }
            c if is_control_char(c) => {
                let byte = u32::from(c).try_into().unwrap_or(0x7F);
                if let Some([head, tail]) = caret_notation(byte) {
                    out.push(head);
                    out.push(tail);
                }
                col += 2;
            }
            c => {
                out.push(c);
                col += u16::try_from(c.width().unwrap_or(0)).unwrap_or(u16::MAX);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_expand_to_eight_cell_stops() {
        assert_eq!(tab_advance(0), 8);
        assert_eq!(tab_advance(7), 1);
        assert_eq!(tab_advance(8), 8);
    }

    #[test]
    fn wide_and_narrow_characters_measure_in_cells() {
        assert_eq!(display_width("abc", 0), 3);
        assert_eq!(display_width("漢字", 0), 4);
        assert_eq!(display_width("a\tb", 0), 9);
    }

    #[test]
    fn measurement_matches_rendered_visible_text() {
        for (raw, column) in [
            ("plain words", 0),
            ("a\ttabbed\trow", 3),
            ("jo\nined", 0),
            ("\u{1}[ok\u{7F}", 2),
            ("漢字テスト", 5),
        ] {
            let rendered = visible_text(raw, column);
            assert_eq!(display_width(raw, column), display_width(&rendered, column));
            assert!(!rendered.contains('\n'));
            assert!(
                !rendered
                    .chars()
                    .any(|c| matches!(u32::from(c), 0x00..=0x1F | 0x7F))
            );
        }
    }

    #[test]
    fn control_bytes_count_caret_cells_without_leaking() {
        assert_eq!(visible_text("\u{1}[", 0), "^A[");
        assert_eq!(visible_text("ok\u{7F}", 0), "ok^?");
        assert!(caret_notation(0x41).is_none());
        assert_eq!(caret_notation(0x01), Some(['^', 'A']));
        assert_eq!(caret_notation(0x07), Some(['^', 'G']));
        assert_eq!(caret_notation(0x7F), Some(['^', '?']));
    }

    #[test]
    fn lay_011_ambiguous_width_characters_measure_narrow_deterministically() {
        // East Asian Ambiguous code points (plus-minus, white circle,
        // not sign) measure as one cell: the pinned narrow policy. The
        // measurement is pure, so repeated calls are always identical.
        for character in ['\u{B1}', '\u{25CB}', '\u{A4}', '\u{2190}'] {
            let first = display_width(&character.to_string(), 0);
            assert_eq!(first, 1, "{character:?} measures narrow");
            assert_eq!(
                first,
                display_width(&character.to_string(), 37),
                "{character:?} measurement never depends on the column"
            );
        }

        // A layout built from ambiguous characters is deterministic across
        // rebuilds and independent of theme or mode state.
        let text = "\u{B1}\u{25CB} value \u{2190} previous\n".repeat(4);
        let document = crate::document::text::document_from_text(
            crate::document::DocumentId::new("ambiguous".to_owned()),
            None,
            &text,
        )
        .expect("ambiguous fixture parses");
        let rows = super::super::layout_document(&document, 21);
        let again = super::super::layout_document(&document, 21);
        assert_eq!(rows.rows(), again.rows());
    }
}
