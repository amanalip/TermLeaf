//! Logical document model shared by every supported book format.
//!
//! The model is independent of terminal width and of any UI crate: a layout
//! pass turns [`Document`] content into visual rows, while reading positions
//! always address the canonical logical text recorded here.

mod archive;
mod epub;
mod error;
pub mod image;
pub mod markdown;
pub mod model;
pub mod text;
pub mod xhtml;

pub use archive::{
    ArchiveError, ArchiveLimits, MemberClass, MemberInfo, NameRejection, PreflightedArchive,
    SharedBookBytes, SharedBookCursor, canonical_key, classify_member,
};
pub use epub::EpubSnapshot;
pub use error::{DocumentError, Format, PositionError, detect_format, sanitize_path};
pub use image::{DecodedImage, ImageLimits, ImageResourceError};

/// Loads any supported local book through its format adapter.
///
/// # Errors
///
/// Returns the typed failure from the matching adapter; unsupported
/// extensions reject before any file content is read.
pub fn load_book_file(
    path: &std::path::Path,
    text_limits: &text::TextLimits,
    archive_limits: &ArchiveLimits,
) -> Result<Document, DocumentError> {
    match detect_format(path) {
        Some(Format::PlainText) => text::load_text_file(path, text_limits),
        Some(Format::Epub) => epub::load_epub_file(path, archive_limits),
        Some(Format::Markdown) => markdown::load_markdown_file(path, text_limits),
        None => Err(DocumentError::UnsupportedFormat {
            path: sanitize_path(&path.display().to_string()),
        }),
    }
}
pub use model::{
    Block, BlockKind, Document, DocumentId, InlineKind, InlineSpan, Position, Section,
};
pub use text::TextLimits;
