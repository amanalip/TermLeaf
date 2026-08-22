//! Typed domain errors for document loading and position construction.
//!
//! Reader-facing messages state what failed, why, and one recovery action,
//! without debug chains or unrelated private data.

use super::archive::ArchiveError;

/// Failures that can occur while decoding or bounding a source document.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The file exceeds the configured plain-text size limit.
    #[error(
        "book is too large: '{path}' is {size} bytes and the limit is {limit} bytes; \
         split the book or choose a smaller file"
    )]
    TooLarge {
        /// Safe display path of the rejected file.
        path: String,
        /// Actual byte length observed before any full read.
        size: u64,
        /// Configured inclusive byte limit.
        limit: u64,
    },

    /// BOM-marked UTF-16 content that does not decode cleanly.
    #[error(
        "could not decode '{path}' as {encoding}: {detail}; the file may be damaged or in an \
         unsupported encoding"
    )]
    InvalidUtf16 {
        /// Safe display path of the rejected file.
        path: String,
        /// Encoding name derived from the byte-order mark.
        encoding: &'static str,
        /// Short decoder explanation without raw byte dumps.
        detail: String,
    },

    /// Unmarked content that is not valid UTF-8.
    #[error(
        "could not read '{path}' at byte offset {offset}: {cause}; re-save the book as UTF-8 or \
         add a UTF-16 byte-order mark"
    )]
    InvalidEncoding {
        /// Safe display path of the rejected file.
        path: String,
        /// Zero-based byte offset of the first invalid sequence.
        offset: usize,
        /// Short validation explanation such as "invalid UTF-8 sequence".
        cause: String,
    },

    /// An I/O failure occurred while reading bounded source bytes.
    #[error("could not read '{path}': {source}")]
    Read {
        /// Safe display path of the rejected file.
        path: String,
        /// Underlying operating system error.
        #[source]
        source: std::io::Error,
    },

    /// The file carries an extension no format adapter accepts yet.
    #[error(
        "unsupported book format: '{path}'; TermLeaf currently opens plain-text '.txt', \
         Markdown '.md', and EPUB '.epub' books"
    )]
    UnsupportedFormat {
        /// Safe display path of the rejected file.
        path: String,
    },

    /// An archive-level policy rejection occurred before semantic parsing.
    #[error(transparent)]
    Archive(
        /// The typed archive failure carrying its own path and recovery.
        #[from]
        ArchiveError,
    ),

    /// The archive opened but the EPUB package itself is unusable.
    #[error(
        "could not open '{path}' as an EPUB book: {detail}; the file may be damaged or \
         not an EPUB"
    )]
    InvalidPackage {
        /// Safe display path of the rejected file.
        path: String,
        /// Short package-level explanation without raw dumps.
        detail: String,
    },

    /// One chapter's markup structure exceeds the safety budget.
    ///
    /// The count happens on raw bytes before any HTML5 tree allocation, so
    /// hostile or corrupt chapters stop here instead of consuming memory.
    #[error(
        "'{path}' holds chapter '{member}' declaring about {nodes} markup nodes beyond \
         the {limit} node limit; the chapter may be corrupt or hostile"
    )]
    ChapterTooComplex {
        /// Safe display path of the rejected book.
        path: String,
        /// Canonical archive key of the rejected chapter.
        member: String,
        /// Counted markup openings in the chapter source.
        nodes: usize,
        /// Inclusive policy limit that was exceeded.
        limit: usize,
    },

    /// The book declares itself pre-paginated (fixed layout).
    #[error(
        "'{path}' uses fixed layout, which TermLeaf cannot reflow; choose a reflowable \
         EPUB edition"
    )]
    UnsupportedFixedLayout {
        /// Safe display path of the rejected file.
        path: String,
    },
}

/// Resolves one path to its supported book format.
///
/// `DEC-TEST-001` resolution (DD-024): detection is extension-first and
/// case-insensitive. Phase 1 shipped `.txt`; Phase 2 extends the table with
/// `.epub` and Markdown `.md`. Content validity is still checked after the
/// extension gate, so a `.txt` file holding binary data fails decoding with
/// a typed reason.
#[must_use]
pub fn detect_format(path: &std::path::Path) -> Option<Format> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("txt") => Some(Format::PlainText),
        Some("epub") => Some(Format::Epub),
        Some("md" | "markdown") => Some(Format::Markdown),
        _ => None,
    }
}

/// The supported book formats, one per ingestion adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    /// Local plain text.
    PlainText,
    /// An EPUB 2 or EPUB 3 package behind the bounded archive layer.
    Epub,
    /// A source-aware Markdown document.
    Markdown,
}

/// Escapes control characters so hostile names cannot inject terminal
/// sequences into diagnostics.
///
/// Every C0 byte and `DEL` becomes its two-cell caret notation, matching
/// the reader's own visible-control rendering; ordinary text passes
/// through unchanged.
#[must_use]
pub fn sanitize_path(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        match u32::from(character) {
            0x7F => escaped.push_str("^?"),
            0x00..=0x1F => {
                let letter = match u32::from(character) {
                    0x1B => '[',
                    0x1C => '\\',
                    0x1D => ']',
                    0x1E => '^',
                    0x1F => '_',
                    other => char::from_u32(0x40 | other).unwrap_or('?'),
                };
                escaped.push('^');
                escaped.push(letter);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Rejection reasons for an out-of-range or misaligned [`Position`](super::Position).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PositionError {
    /// The referenced section does not exist in the document.
    #[error("section index {section} is outside the document")]
    Section {
        /// Rejected section ordinal.
        section: usize,
    },
    /// The referenced block does not exist in the section.
    #[error("block index {block} is outside section {section}")]
    Block {
        /// Referenced section ordinal.
        section: usize,
        /// Rejected block ordinal.
        block: usize,
    },
    /// The byte offset is not inside the referenced block range.
    #[error("byte offset {offset} is outside block {block}")]
    OffsetOutsideBlock {
        /// Referenced block ordinal.
        block: usize,
        /// Rejected byte offset.
        offset: usize,
    },
    /// The byte offset falls inside a multi-byte character.
    ///
    /// Offset zero of a validated block is always a boundary, so reports are
    /// always at least one byte into the block.
    #[error("byte offset {offset} splits a character")]
    NotCharBoundary {
        /// Rejected byte offset.
        offset: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_007_txt_extension_is_accepted_case_insensitively() {
        for name in [
            "book.txt",
            "BOOK.TXT",
            "pride-and-prejudice-1342.txt",
            "b.TxT",
        ] {
            assert_eq!(
                detect_format(std::path::Path::new(name)),
                Some(Format::PlainText),
                "{name} opens as plain text"
            );
        }
    }

    #[test]
    fn cli_007_unsupported_or_missing_extensions_are_one_typed_rejection() {
        for name in ["book.dat", "book", "book.text", ".txt"] {
            let result = detect_format(std::path::Path::new(name));
            assert_eq!(result, None, "{name} has no adapter");
            let error = DocumentError::UnsupportedFormat {
                path: name.to_owned(),
            };
            let message = error.to_string();
            assert!(message.contains("unsupported book format"), "{message}");
            assert!(message.contains(name), "{message}");
        }
        // The rejection names every supported format once.
        let message = DocumentError::UnsupportedFormat {
            path: "other.dat".to_owned(),
        }
        .to_string();
        assert!(message.contains("plain-text '.txt'"), "{message}");
        assert!(message.contains("'.md'"), "{message}");
        assert!(message.contains("'.epub'"), "{message}");
    }

    #[test]
    fn cli_007_markdown_extensions_are_accepted_case_insensitively() {
        for name in ["notes.md", "NOTES.MD", "readme.markdown", "b.Md"] {
            assert_eq!(
                detect_format(std::path::Path::new(name)),
                Some(Format::Markdown),
                "{name} opens as Markdown"
            );
        }
    }

    #[test]
    fn epub_extension_is_accepted_case_insensitively() {
        for name in ["book.epub", "BOOK.EPUB", "b.Epub"] {
            assert_eq!(
                detect_format(std::path::Path::new(name)),
                Some(Format::Epub),
                "{name} opens as EPUB"
            );
        }
    }

    #[test]
    fn err_001_each_document_error_names_path_reason_and_recovery() {
        let errors = [
            DocumentError::TooLarge {
                path: "big.txt".to_owned(),
                size: 40,
                limit: 32,
            }
            .to_string(),
            DocumentError::InvalidUtf16 {
                path: "old.txt".to_owned(),
                encoding: "UTF-16LE",
                detail: "bad unit".to_owned(),
            }
            .to_string(),
            DocumentError::InvalidEncoding {
                path: "broken.txt".to_owned(),
                offset: 3,
                cause: "invalid UTF-8 sequence".to_owned(),
            }
            .to_string(),
            DocumentError::Read {
                path: "gone.txt".to_owned(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            }
            .to_string(),
            DocumentError::UnsupportedFormat {
                path: "other.dat".to_owned(),
            }
            .to_string(),
        ];

        for message in &errors {
            assert!(message.contains('\''), "names its path: {message}");
            assert!(
                message.contains("could not")
                    || message.contains("unsupported")
                    || message.contains("too large"),
                "states what failed: {message}"
            );
            assert!(
                message.contains(';') || message.contains(':'),
                "gives a reason or recovery: {message}"
            );
        }
        assert!(errors[0].contains("limit is 32 bytes"));
        assert!(errors[1].contains("UTF-16LE"));
        assert!(errors[2].contains("byte offset 3"));
        assert!(errors[4].contains("plain-text '.txt'"));
    }

    #[test]
    fn err_003_control_bytes_in_failing_paths_cannot_reach_the_terminal() {
        // A hostile path is only ever echoed through Display after escaping;
        // the diagnostic writer in process.rs additionally refuses raw C0.
        let hostile = "tricky\u{1b}[31mname.txt";
        let error = DocumentError::InvalidEncoding {
            path: sanitize_path(hostile),
            offset: 0,
            cause: "invalid UTF-8 sequence".to_owned(),
        };
        let message = error.to_string();

        assert!(
            !message.contains('\u{1b}'),
            "no escape byte survives: {message}"
        );
        assert!(message.contains("^["));
    }
}
