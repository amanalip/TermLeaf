//! Typed domain errors for document loading and position construction.
//!
//! Reader-facing messages state what failed, why, and one recovery action,
//! without debug chains or unrelated private data.

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
