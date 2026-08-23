//! Plain-text ingestion: bounded reads, safe decoding, and paragraph structure.
//!
//! Pipeline: bound the byte count, detect a byte-order mark, require strict
//! UTF-8 unless a UTF-16 BOM selects `encoding_rs`, normalize CR and CRLF to
//! logical LF, then partition the text into paragraphs and deliberate blank
//! lines. Canonical text stays byte-faithful to the source apart from those
//! documented normalizations, which keeps logical positions meaningful.

use std::{fs::File, io::Read, path::Path};

use super::{Document, DocumentError, DocumentId, model::Block, model::BlockKind, sanitize_path};

/// Initial plain-text size policy pending the final `DEC-TEST-012` numbers.
///
/// The value bounds allocation before parsing; boundary behavior itself is
/// covered against the injected limit so tests stay small and fast.
#[derive(Clone, Copy, Debug)]
pub struct TextLimits {
    /// Inclusive maximum source size in bytes.
    pub max_bytes: u64,
}

impl Default for TextLimits {
    fn default() -> Self {
        Self {
            max_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Bom {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl Bom {
    fn detect(bytes: &[u8]) -> (Self, usize) {
        match bytes {
            [0xEF, 0xBB, 0xBF, rest @ ..] => (Self::Utf8, bytes.len() - rest.len()),
            [0xFF, 0xFE, rest @ ..] => (Self::Utf16Le, bytes.len() - rest.len()),
            [0xFE, 0xFF, rest @ ..] => (Self::Utf16Be, bytes.len() - rest.len()),
            _ => (Self::None, 0),
        }
    }

    /// The UTF-16 decoder selected by an explicit byte-order mark.
    const fn utf16_decoder(self) -> Option<&'static encoding_rs::Encoding> {
        match self {
            Self::Utf16Le => Some(encoding_rs::UTF_16LE),
            Self::Utf16Be => Some(encoding_rs::UTF_16BE),
            Self::None | Self::Utf8 => None,
        }
    }

    /// The human name of a marked UTF-16 encoding, for error messages.
    const fn utf16_name(self) -> Option<&'static str> {
        match self {
            Self::Utf16Le => Some("UTF-16LE"),
            Self::Utf16Be => Some("UTF-16BE"),
            Self::None | Self::Utf8 => None,
        }
    }
}

/// Decodes raw source bytes into logical text.
///
/// UTF-8 (with or without BOM) must validate strictly; malformed sequences
/// are rejected with their offset instead of being replaced. Only an explicit
/// UTF-16 byte-order mark routes bytes through `encoding_rs`.
///
/// # Errors
///
/// Returns [`DocumentError::InvalidUtf16`] for damaged marked UTF-16 and
/// [`DocumentError::InvalidEncoding`] for invalid or misleadingly unencoded
/// content, naming the first offending byte offset.
pub fn decode_text<'a>(
    path: &str,
    bytes: &'a [u8],
) -> Result<std::borrow::Cow<'a, str>, DocumentError> {
    let (bom, bom_len) = Bom::detect(bytes);
    let rest = &bytes[bom_len..];

    if let Some(decoder) = bom.utf16_decoder() {
        let (text, had_errors) = decoder.decode_without_bom_handling(rest);
        if had_errors {
            return Err(DocumentError::InvalidUtf16 {
                path: path.to_owned(),
                encoding: bom.utf16_name().unwrap_or("UTF-16"),
                detail: "the stream contains bytes that do not form valid units for the \
                         advertised encoding"
                    .to_owned(),
            });
        }
        return Ok(text);
    }

    match std::str::from_utf8(rest) {
        Ok(text) => {
            if let Some(offset) = unmarked_utf16_signature(rest) {
                return Err(DocumentError::InvalidEncoding {
                    path: path.to_owned(),
                    offset,
                    cause: "the content appears to be UTF-16 without a byte-order mark".to_owned(),
                });
            }
            Ok(std::borrow::Cow::Borrowed(text))
        }
        Err(error) => Err(DocumentError::InvalidEncoding {
            path: path.to_owned(),
            offset: bom_len + error.valid_up_to(),
            cause: invalid_utf8_cause(rest, error.valid_up_to()).to_owned(),
        }),
    }
}

/// Finds the start of a UTF-16-like alternating-NUL pattern.
///
/// ASCII text encoded as UTF-16 without a BOM still validates as UTF-8
/// because every second byte is NUL. A dense single-parity NUL run in the
/// opening window therefore marks content that would otherwise display
/// unreadable gaps.
fn unmarked_utf16_signature(bytes: &[u8]) -> Option<usize> {
    let window = &bytes[..bytes.len().min(512)];
    if window.len() < 4 {
        return None;
    }
    let zeros_even = window.iter().step_by(2).filter(|byte| **byte == 0).count();
    let zeros_odd = window
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|byte| **byte == 0)
        .count();
    let half = window.len() / 2;
    let dense = zeros_even.max(zeros_odd) * 4 >= half;
    dense.then(|| {
        window
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or_default()
    })
}

fn invalid_utf8_cause(bytes: &[u8], valid_up_to: usize) -> &'static str {
    if unmarked_utf16_signature(&bytes[valid_up_to.min(bytes.len())..]).is_some() {
        "the content appears to be UTF-16 without a byte-order mark"
    } else if bytes[valid_up_to..].starts_with(&[0xEF, 0xBB, 0xBF]) {
        "a stray byte-order mark appears inside the text"
    } else {
        "invalid UTF-8 sequence"
    }
}

/// Builds a document from already-decoded logical text.
///
/// Lines are classified blank when every character is a space or tab; runs of
/// non-blank lines become one [`BlockKind::Paragraph`] reflow unit, and every
/// blank line becomes its own preserved [`BlockKind::BlankLine`].
///
/// # Errors
///
/// Propagates the tiling invariant failure from
/// [`Document::from_single_section`](super::model::Document::from_single_section),
/// which indicates a parser defect rather than reader input.
pub fn document_from_text(
    id: DocumentId,
    title: Option<String>,
    text: &str,
) -> Result<Document, String> {
    let canonical = normalize_newlines(text);
    let blocks = build_blocks(&canonical);
    Document::from_single_section(id, title, canonical, blocks)
}

/// Replaces CRLF and lone CR with logical LF newlines.
fn normalize_newlines(text: &str) -> String {
    if !text.bytes().any(|byte| byte == b'\r') {
        return text.to_owned();
    }
    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(current) = chars.next() {
        if current == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(current);
        }
    }
    normalized
}

fn build_blocks(canonical: &str) -> Vec<Block> {
    let bytes = canonical.as_bytes();
    let mut blocks = Vec::new();
    // Open paragraph as (start, last_line_end); extended line by line.
    let mut paragraph: Option<(usize, usize)> = None;
    let mut cursor = 0usize;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let line_blank = canonical[cursor..index]
            .chars()
            .all(|c| c == ' ' || c == '\t');
        if line_blank {
            if let Some((start, _)) = paragraph.take() {
                blocks.push(Block::new(BlockKind::Paragraph, start..cursor));
            }
            blocks.push(Block::new(BlockKind::BlankLine, cursor..index + 1));
        } else {
            paragraph.get_or_insert((cursor, index)).1 = index;
        }
        cursor = index + 1;
    }

    let tail_blank = canonical[cursor..].chars().all(|c| c == ' ' || c == '\t');
    if tail_blank {
        if let Some((start, _)) = paragraph.take() {
            blocks.push(Block::new(BlockKind::Paragraph, start..cursor));
        }
        if cursor < bytes.len() {
            blocks.push(Block::new(BlockKind::BlankLine, cursor..bytes.len()));
        }
    } else if let Some((start, _)) = paragraph.take() {
        blocks.push(Block::new(BlockKind::Paragraph, start..bytes.len()));
    } else {
        blocks.push(Block::new(BlockKind::Paragraph, cursor..bytes.len()));
    }

    blocks
}

/// Reads and parses a plain-text book under the supplied limits.
///
/// The size check happens on metadata first and again on the guarded read, so
/// neither a lying header nor a racing writer can force an unbounded
/// allocation.
///
/// # Errors
///
/// Returns [`DocumentError::TooLarge`] above the byte limit before decoding,
/// [`DocumentError::Read`] for operating-system failures, and the decode
/// errors from [`decode_text`].
pub fn load_text_file(path: &Path, limits: &TextLimits) -> Result<Document, DocumentError> {
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

    load_text_bytes(&display, &bytes, limits)
}

/// Parses in-memory bytes with the same pipeline as [`load_text_file`].
///
/// # Errors
///
/// Same variants as [`load_text_file`] minus [`DocumentError::Read`].
pub fn load_text_bytes(
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
    let text = decode_text(path, bytes)?;
    let title = title_for(path);
    document_from_text(
        DocumentId::new(format!("{path}:{}", text.len())),
        Some(title),
        &text,
    )
    .map_err(|detail| DocumentError::InvalidEncoding {
        path: path.to_owned(),
        offset: 0,
        cause: detail,
    })
}

/// The display title derived from a path's file stem.
#[must_use]
pub fn file_stem_title(path: &str) -> String {
    title_for(path)
}

fn title_for(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map_or_else(|| "Untitled".to_owned(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "sample.txt";

    fn kinds(document: &Document) -> Vec<BlockKind> {
        document.sections()[0]
            .blocks()
            .iter()
            .map(Block::kind)
            .collect()
    }

    fn paragraph_texts(document: &Document) -> Vec<String> {
        document.sections()[0]
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind() == BlockKind::Paragraph)
            .map(|(i, _)| {
                document
                    .block_text(0, i)
                    .expect("paragraph text")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn txt_001_valid_utf8_without_bom_decodes_exactly() {
        let document = load_text_bytes(PATH, "héllo\nwörld\n".as_bytes(), &TextLimits::default())
            .expect("valid UTF-8 loads");

        assert_eq!(document.canonical(), "héllo\nwörld\n");
        assert_eq!(paragraph_texts(&document), ["héllo\nwörld\n"]);
        assert_eq!(document.title(), "sample");
    }

    #[test]
    fn txt_002_utf8_bom_is_removed_once_and_content_is_unchanged() {
        let plain = load_text_bytes(PATH, "content\n".as_bytes(), &TextLimits::default())
            .expect("plain loads");
        let marked = load_text_bytes(
            PATH,
            [0xEF, 0xBB, 0xBF]
                .iter()
                .copied()
                .chain(b"content\n".iter().copied())
                .collect::<Vec<u8>>()
                .as_slice(),
            &TextLimits::default(),
        )
        .expect("BOM-marked loads");

        assert_eq!(marked.canonical(), plain.canonical());
        assert_eq!(kinds(&marked), kinds(&plain));
        assert!(!marked.canonical().starts_with('\u{feff}'));
    }

    fn utf16_bytes(text: &str, big_endian: bool) -> Vec<u8> {
        let mut units = vec![0xFEFF_u16];
        units.extend(text.encode_utf16());
        units
            .iter()
            .flat_map(|unit| {
                if big_endian {
                    unit.to_be_bytes()
                } else {
                    unit.to_le_bytes()
                }
            })
            .collect()
    }

    #[test]
    fn txt_003_marked_utf16_le_and_be_decode_without_replacement() {
        for big_endian in [false, true] {
            let bytes = utf16_bytes("Zoë\ncafé\n", big_endian);
            let document = load_text_bytes(PATH, &bytes, &TextLimits::default())
                .expect("marked UTF-16 decodes");

            assert_eq!(document.canonical(), "Zoë\ncafé\n");
            assert!(!document.canonical().contains('\u{FFFD}'));
        }
    }

    #[test]
    fn txt_004_invalid_utf8_and_unmarked_utf16_are_rejected_with_reasons() {
        let invalid = load_text_bytes(PATH, &[0x68, 0xFF, 0x6F], &TextLimits::default())
            .expect_err("invalid UTF-8 is rejected");
        let DocumentError::InvalidEncoding {
            path,
            offset,
            cause,
        } = invalid
        else {
            panic!("expected InvalidEncoding, got {invalid:?}");
        };
        assert_eq!(path, PATH);
        assert_eq!(offset, 1);
        assert_eq!(cause, "invalid UTF-8 sequence");

        let mut unmarked = String::from("hi").encode_utf16().collect::<Vec<u16>>();
        unmarked.insert(0, 0x68); // stray high byte breaks naive pairing
        let bytes = unmarked
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect::<Vec<u8>>();
        let rejected = load_text_bytes(PATH, &bytes, &TextLimits::default())
            .expect_err("unmarked UTF-16 is rejected");
        let DocumentError::InvalidEncoding { offset, cause, .. } = rejected else {
            panic!("expected InvalidEncoding, got {rejected:?}");
        };
        assert_eq!(offset, 1);
        assert_eq!(
            cause,
            "the content appears to be UTF-16 without a byte-order mark"
        );
    }

    #[test]
    fn txt_005_lf_crlf_and_cr_variants_produce_equivalent_documents() {
        let base = load_text_bytes(PATH, b"a\nb\n", &TextLimits::default()).expect("LF loads");
        for variant in [&b"a\r\nb\r\n"[..], &b"a\rb\r"[..]] {
            let other =
                load_text_bytes(PATH, variant, &TextLimits::default()).expect("variant loads");
            assert_eq!(other.canonical(), base.canonical());
            assert_eq!(kinds(&other), kinds(&base));
        }
    }

    #[test]
    fn txt_006_paragraphs_and_repeated_blank_lines_survive_conversion() {
        let document = load_text_bytes(
            PATH,
            b"first para line one\nline two\n\n\nsecond para\n",
            &TextLimits::default(),
        )
        .expect("parses");

        assert_eq!(
            kinds(&document),
            [
                BlockKind::Paragraph,
                BlockKind::BlankLine,
                BlockKind::BlankLine,
                BlockKind::Paragraph
            ]
        );
        assert_eq!(
            paragraph_texts(&document),
            ["first para line one\nline two\n", "second para\n"]
        );
    }

    #[test]
    fn txt_007_empty_and_whitespace_only_files_are_stable() {
        let empty = load_text_bytes(PATH, b"", &TextLimits::default()).expect("empty file parses");
        assert!(empty.is_empty());
        assert!(empty.sections()[0].blocks().is_empty());
        assert!(empty.first_position().is_err());
        assert_eq!(empty.navigation_points()[0].title(), "Section 1");
        assert_eq!(
            empty.navigation_points()[0]
                .position()
                .absolute_byte(&empty),
            0
        );

        let blanks = load_text_bytes(PATH, b"  \n\t\n", &TextLimits::default())
            .expect("whitespace-only parses");
        assert_eq!(kinds(&blanks), [BlockKind::BlankLine, BlockKind::BlankLine]);
        let start = blanks.first_position().expect("blank lines are navigable");
        assert_eq!(start.absolute_byte(&blanks), 0);
    }

    #[test]
    fn txt_008_byte_limit_holds_at_below_and_above_the_boundary() {
        let limits = TextLimits { max_bytes: 10 };

        let below = load_text_bytes(PATH, b"012345678", &limits);
        assert!(below.is_ok());

        let exact = load_text_bytes(PATH, b"0123456789", &limits);
        assert!(exact.is_ok());

        let above = load_text_bytes(PATH, b"0123456789A", &limits)
            .expect_err("over-limit input is rejected");
        assert!(matches!(above, DocumentError::TooLarge { .. }));
        assert!(above.to_string().contains("limit is 10 bytes"));
    }

    #[test]
    fn txt_009_extremely_long_line_parses_with_progress_and_no_panic() {
        let long_line = "x".repeat(300_000);
        let source = format!("{long_line}\n");
        let document = document_from_text(DocumentId::new("long".to_owned()), None, &source)
            .expect("bounded long line parses");

        let blocks = document.sections()[0].blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].range().len(), source.len());

        let end = document
            .position(0, 0, source.len())
            .expect("end boundary is valid");
        assert_eq!(end.absolute_byte(&document), source.len());
    }

    #[test]
    fn model_002_traversal_is_deterministic_complete_and_unduplicated() {
        let source = "one\n\n\ntwo three\nfour\nfive\n\nsix\n";
        let document = document_from_text(DocumentId::new("traverse".to_owned()), None, source)
            .expect("fixture parses");

        // Reading order over the single section visits every block exactly
        // once and reconstructs the logical content in order.
        let mut visited_kinds = Vec::new();
        let mut reconstructed = String::new();
        for section in document.sections() {
            for (index, block) in section.blocks().iter().enumerate() {
                visited_kinds.push((index, block.kind()));
                reconstructed.push_str(document.block_text(0, index).expect("block text"));
            }
        }

        assert_eq!(
            visited_kinds
                .iter()
                .map(|(_, kind)| *kind)
                .collect::<Vec<_>>(),
            [
                BlockKind::Paragraph,
                BlockKind::BlankLine,
                BlockKind::BlankLine,
                BlockKind::Paragraph,
                BlockKind::BlankLine,
                BlockKind::Paragraph,
            ]
        );
        assert_eq!(reconstructed, source);

        // A second traversal is identical: no hidden state.
        let again: Vec<_> = document.sections()[0]
            .blocks()
            .iter()
            .map(|block| (block.kind(), block.range().clone()))
            .collect();
        assert_eq!(
            again,
            document.sections()[0]
                .blocks()
                .iter()
                .map(|block| (block.kind(), block.range().clone()))
                .collect::<Vec<_>>()
        );
    }
}
