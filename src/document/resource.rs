//! Lazy, bounded providers for image bytes associated with an open document.

use std::{
    io::Read,
    path::{Component, Path},
    sync::Arc,
};

use cap_std::{ambient_authority, fs::Dir};

use super::{ImageResource, PreflightedArchive, sanitize_path};

/// Clonable access to resources owned by one inspected book.
#[derive(Clone, Debug, Default)]
pub enum ResourceProvider {
    /// Formats without local resources.
    #[default]
    None,
    /// An EPUB's immutable, preflighted source bytes.
    Epub(Arc<PreflightedArchive>),
    /// Loose files constrained to an already-open Markdown book directory.
    Markdown { root: Arc<Dir> },
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
    /// Creates a provider holding an open capability for a Markdown file's parent.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the parent directory cannot be opened.
    pub fn markdown(book: &Path) -> std::io::Result<Self> {
        let parent = book
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        Ok(Self::Markdown {
            root: Arc::new(Dir::open_ambient_dir(parent, ambient_authority())?),
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

fn read_local(root: &Dir, reference: &str, limit: u64) -> Result<Vec<u8>, ResourceReadError> {
    let relative = Path::new(reference);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(ResourceReadError::UnsafePath);
    }
    // Symlinks are unnecessary for book resources. Rejecting the observed
    // entry gives stable diagnostics while the capability remains the actual
    // security boundary if the entry changes immediately after this check.
    if root
        .symlink_metadata(relative)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(ResourceReadError::EscapesBook);
    }
    let mut file = root
        .open(relative)
        .map_err(|_| ResourceReadError::Unreadable)?;
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
        assert_eq!(
            provider.read_bounded(&ImageResource::member("/outside", None), 10),
            Err(ResourceReadError::UnsafePath)
        );
    }

    #[cfg(windows)]
    #[test]
    fn img_012_markdown_provider_rejects_windows_device_and_drive_paths() {
        let directory = tempfile::tempdir().expect("book directory");
        let book = directory.path().join("book.md");
        std::fs::write(&book, "book").expect("book");
        let provider = ResourceProvider::markdown(&book).expect("provider");

        for reference in [r"\\.\NUL", r"C:\outside", r"C:outside"] {
            assert_eq!(
                provider.read_bounded(&ImageResource::member(reference, None), 10),
                Err(ResourceReadError::UnsafePath)
            );
        }
        assert_eq!(
            provider.read_bounded(&ImageResource::member("NUL", None), 10),
            Err(ResourceReadError::Unreadable),
            "cap-std rejects reserved device basenames"
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

    #[cfg(unix)]
    #[test]
    fn img_012_markdown_provider_never_follows_concurrently_swapped_escape() {
        use std::sync::{
            Arc as StdArc, Barrier,
            atomic::{AtomicBool, Ordering},
        };

        let directory = tempfile::tempdir().expect("book directory");
        let outside = tempfile::tempdir().expect("outside directory");
        let book = directory.path().join("book.md");
        let live = directory.path().join("images");
        let parked = directory.path().join("images-safe");
        std::fs::write(&book, "![plate](images/plate.bin)").expect("book");
        std::fs::create_dir(&live).expect("image directory");
        std::fs::write(live.join("plate.bin"), b"inside").expect("inside resource");
        std::fs::write(outside.path().join("plate.bin"), b"outside").expect("outside resource");
        let provider = ResourceProvider::markdown(&book).expect("provider");
        let start = StdArc::new(Barrier::new(2));
        let escape_installed = StdArc::new(Barrier::new(2));
        let escape_observed = StdArc::new(Barrier::new(2));
        let finished = StdArc::new(AtomicBool::new(false));

        let swap_start = StdArc::clone(&start);
        let swap_installed = StdArc::clone(&escape_installed);
        let swap_observed = StdArc::clone(&escape_observed);
        let swap_finished = StdArc::clone(&finished);
        let outside_path = outside.path().to_path_buf();
        let swapper = std::thread::spawn(move || {
            swap_start.wait();
            for iteration in 0..2_000 {
                std::fs::rename(&live, &parked).expect("park safe directory");
                std::os::unix::fs::symlink(&outside_path, &live).expect("install escape");
                if iteration == 0 {
                    swap_installed.wait();
                    swap_observed.wait();
                }
                std::thread::yield_now();
                std::fs::remove_file(&live).expect("remove escape");
                std::fs::rename(&parked, &live).expect("restore safe directory");
            }
            swap_finished.store(true, Ordering::Release);
        });

        start.wait();
        let resource = ImageResource::member("images/plate.bin", None);
        escape_installed.wait();
        assert!(
            provider.read_bounded(&resource, 16).is_err(),
            "an installed escape is rejected"
        );
        escape_observed.wait();
        while !finished.load(Ordering::Acquire) {
            if let Ok(bytes) = provider.read_bounded(&resource, 16) {
                assert_eq!(bytes, b"inside", "outside bytes crossed the capability");
            }
        }
        swapper.join().expect("swapper thread");
    }
}
