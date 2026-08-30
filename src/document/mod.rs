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
mod resource;
mod structured;
pub mod text;
pub mod vector;
pub mod xhtml;

pub use archive::{
    ArchiveError, ArchiveLimits, MemberClass, MemberInfo, NameRejection, PreflightedArchive,
    SharedBookBytes, SharedBookCursor, canonical_key, checked_expansion_total, classify_member,
};
pub use epub::EpubSnapshot;
pub use error::{DocumentError, Format, PositionError, detect_format, sanitize_path};
pub use image::{DecodedImage, ImageLimits, ImageResourceError};
pub use resource::{ResourceProvider, ResourceReadError};
pub use structured::{XmlLimits, XmlStructureError, validate_control_xml, validate_xml_structure};
pub use vector::{
    VectorFormat, VectorImageError, VectorLimits, VectorWork, decode_vector_bounded,
    decode_vector_bounded_with_limits, sniff_vector_format,
};

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

/// A parsed document plus lazy access to resources from the same source.
#[derive(Debug)]
pub struct LoadedBook {
    pub document: Document,
    pub resources: ResourceProvider,
}

/// Loads a document and preserves its lazy resource source for the reader.
///
/// # Errors
///
/// Returns the same typed document errors as [`load_book_file`].
pub fn load_book_with_resources(
    path: &std::path::Path,
    text_limits: &text::TextLimits,
    archive_limits: &ArchiveLimits,
) -> Result<LoadedBook, DocumentError> {
    let (document, resources) = match detect_format(path) {
        Some(Format::PlainText) => (
            text::load_text_file(path, text_limits)?,
            ResourceProvider::None,
        ),
        Some(Format::Epub) => {
            let snapshot = EpubSnapshot::open(path, archive_limits)?;
            let document = snapshot.build()?;
            (document, snapshot.resource_provider())
        }
        Some(Format::Markdown) => {
            let document = markdown::load_markdown_file(path, text_limits)?;
            let resources =
                ResourceProvider::markdown(path).map_err(|source| DocumentError::Read {
                    path: sanitize_path(&path.display().to_string()),
                    source,
                })?;
            (document, resources)
        }
        None => {
            return Err(DocumentError::UnsupportedFormat {
                path: sanitize_path(&path.display().to_string()),
            });
        }
    };
    Ok(LoadedBook {
        document,
        resources,
    })
}
pub use model::{
    Block, BlockKind, Document, DocumentId, ImageRef, ImageResource, InlineKind, InlineSpan,
    NavigationPoint, Position, Section, SourceMapping,
};
pub use text::TextLimits;
