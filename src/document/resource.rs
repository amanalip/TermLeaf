//! Lazy, bounded providers for image bytes associated with an open document.

use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use super::{ImageResource, PreflightedArchive, sanitize_path};

/// Clonable access to resources owned by one inspected book.
#[derive(Clone, Debug, Default)]
pub enum ResourceProvider {
    /// Formats without local resources.
    #[default]
    None,
    /// An EPUB's immutable, preflighted source bytes.
    Epub(Arc<PreflightedArchive>),
    /// Loose files constrained to the Markdown book's canonical directory.
    Markdown { root: Arc<PathBuf> },
}

/// Typed lazy-resource resolution and bounded-read failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResourceReadError {
    #[error("resource is blocked")]
    Blocked,
    #[error("resource path is not a safe relative path")]
    UnsafePath,
    #[error("resource is outside the book directory")]
    EscapesBook,
    #[error("resource is missing or unreadable")]
    Unreadable,
    #[error("resource is not a regular file")]
    NotAFile,
    #[error("resource is too large ({size} bytes; limit {limit})")]
    TooLarge { size: u64, limit: u64 },
    #[error("archive resource could not be read: {detail}")]
    Archive { detail: String },
}

impl ResourceProvider {
    /// Creates a provider rooted at the canonical parent of a Markdown file.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the parent cannot be canonicalized.
    pub fn markdown(book: &Path) -> std::io::Result<Self> {
        let parent = book.parent().unwrap_or_else(|| Path::new("."));
        Ok(Self::Markdown {
            root: Arc::new(parent.canonicalize()?),
        })
    }

    /// Reads at most `limit` bytes plus a one-byte boundary probe.
    ///
    /// # Errors
    ///
    /// Rejects blocked, escaping, non-file, unreadable, and oversized resources.
    pub fn read_bounded(
        &self,
        resource: &ImageResource,
        limit: u64,
    ) -> Result<Vec<u8>, ResourceReadError> {
        let reference = resource.reference().ok_or(ResourceReadError::Blocked)?;
        match self {
            Self::None => Err(ResourceReadError::Blocked),
            Self::Epub(archive) => archive
                .read_member_bounded(reference, limit)
                .map_err(|error| ResourceReadError::Archive {
                    detail: sanitize_path(&error.to_string()),
                }),
            Self::Markdown { root } => read_local(root, reference, limit),
        }
    }
}

fn read_local(root: &Path, reference: &str, limit: u64) -> Result<Vec<u8>, ResourceReadError> {
    let relative = Path::new(reference);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(ResourceReadError::UnsafePath);
    }
    let target = root
        .join(relative)
        .canonicalize()
        .map_err(|_| ResourceReadError::Unreadable)?;
    if !target.starts_with(root) {
        return Err(ResourceReadError::EscapesBook);
    }
    let mut file = File::open(&target).map_err(|_| ResourceReadError::Unreadable)?;
    let metadata = file.metadata().map_err(|_| ResourceReadError::Unreadable)?;
    if !metadata.is_file() {
        return Err(ResourceReadError::NotAFile);
    }
    if metadata.len() > limit {
        return Err(ResourceReadError::TooLarge {
            size: metadata.len(),
            limit,
        });
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(metadata.len().min(limit)).unwrap_or(usize::MAX));
    file.by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ResourceReadError::Unreadable)?;
    if bytes.len() as u64 > limit {
        return Err(ResourceReadError::TooLarge {
            size: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn img_012_markdown_provider_reads_only_bounded_non_escaping_files() {
        let directory = tempfile::tempdir().expect("temp directory");
        let book = directory.path().join("book.md");
        std::fs::write(&book, "![plate](images/plate.bin)").expect("book");
        std::fs::create_dir(directory.path().join("images")).expect("image directory");
        std::fs::write(directory.path().join("images/plate.bin"), b"12345").expect("resource");
        let provider = ResourceProvider::markdown(&book).expect("canonical provider");

        let resource = ImageResource::member("images/plate.bin", None);
        assert_eq!(provider.read_bounded(&resource, 5), Ok(b"12345".to_vec()));
        assert_eq!(
            provider.read_bounded(&resource, 4),
            Err(ResourceReadError::TooLarge { size: 5, limit: 4 })
        );
        assert_eq!(
            provider.read_bounded(&ImageResource::member("../outside", None), 10),
            Err(ResourceReadError::UnsafePath)
        );
    }

    #[cfg(unix)]
    #[test]
    fn img_012_markdown_provider_rejects_symlink_escape() {
        let directory = tempfile::tempdir().expect("book directory");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        let book = directory.path().join("book.md");
        std::fs::write(&book, "![outside](escape)").expect("book");
        std::os::unix::fs::symlink(outside.path(), directory.path().join("escape"))
            .expect("escape symlink");
        let provider = ResourceProvider::markdown(&book).expect("provider");

        assert_eq!(
            provider.read_bounded(&ImageResource::member("escape", None), 10),
            Err(ResourceReadError::EscapesBook)
        );
    }
}
