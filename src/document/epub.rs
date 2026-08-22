//! EPUB ingestion: bounded archive preflight followed by `rbook` semantics.
//!
//! The pipeline follows the delivery plan exactly: open the immutable source
//! once, enforce every archive boundary in [`super::archive`], then let
//! `rbook` resolve the package, metadata, spine, and navigation over those
//! same inspected bytes. Chapters convert through the tolerant XHTML layer
//! into the shared document model; images and fonts stay lazy and unread.

use std::path::Path;

use rbook::Epub;

use super::archive::open_book_archive;
use super::model::{Block, BlockKind, Section};
use super::text::file_stem_title;
use super::xhtml::{self, SemanticBlock};
use super::{
    ArchiveLimits, Document, DocumentError, DocumentId, PreflightedArchive, canonical_key,
    sanitize_path,
};

/// Reads a local `.epub` file once and builds its logical document.
///
/// # Errors
///
/// Returns every [`DocumentError`] variant the archive preflight or the
/// package parser can produce; nothing reaches the terminal when this fails.
pub fn load_epub_file(path: &Path, limits: &ArchiveLimits) -> Result<Document, DocumentError> {
    let display = sanitize_path(&path.display().to_string());
    let snapshot = open_book_archive(path, limits).map_err(DocumentError::from)?;
    build_document(&display, &snapshot)
}

fn build_document(display: &str, snapshot: &PreflightedArchive) -> Result<Document, DocumentError> {
    if snapshot.member("META-INF/encryption.xml").is_some() {
        return Err(DocumentError::from(super::ArchiveError::EncryptedMember {
            path: display.to_owned(),
            member: "META-INF/encryption.xml".to_owned(),
        }));
    }

    let epub = Epub::read(snapshot.shared_bytes().cursor()).map_err(|error| {
        DocumentError::InvalidPackage {
            path: display.to_owned(),
            detail: error.to_string(),
        }
    })?;

    if is_fixed_layout(&epub) {
        return Err(DocumentError::UnsupportedFixedLayout {
            path: display.to_owned(),
        });
    }

    let labels = toc_labels(&epub);
    let mut chapters: Vec<ChapterContent> = Vec::new();
    for entry in epub.spine() {
        if !entry.is_linear() {
            continue;
        }
        let Some(manifest_entry) = entry.manifest_entry() else {
            return Err(DocumentError::InvalidPackage {
                path: display.to_owned(),
                detail: "the spine references a missing manifest entry".to_owned(),
            });
        };
        let key = resource_key(manifest_entry.href().decode().as_ref()).map_err(|detail| {
            DocumentError::InvalidPackage {
                path: display.to_owned(),
                detail,
            }
        })?;
        if snapshot.member(&key).is_none() {
            return Err(DocumentError::InvalidPackage {
                path: display.to_owned(),
                detail: format!(
                    "the spine references missing resource '{}'",
                    sanitize_path(&key)
                ),
            });
        }
        let raw = snapshot
            .read_member(&key)
            .map_err(|error| DocumentError::InvalidPackage {
                path: display.to_owned(),
                detail: error.to_string(),
            })?;
        let source = std::str::from_utf8(&raw).map_err(|_| DocumentError::InvalidPackage {
            path: display.to_owned(),
            detail: format!("chapter '{}' is not valid UTF-8", sanitize_path(&key)),
        })?;
        let title = labels
            .iter()
            .find(|(label_key, _)| *label_key == key)
            .map(|(_, label)| label.clone());
        chapters.push(ChapterContent {
            title,
            blocks: xhtml::convert_xhtml(source),
        });
    }

    let title = epub
        .metadata()
        .title()
        .map(|title| title.value().trim().to_owned())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| file_stem_title(display));

    let id = DocumentId::new(format!("{display}:epub"));
    assemble(id, Some(title), &chapters).map_err(|detail| DocumentError::InvalidPackage {
        path: display.to_owned(),
        detail,
    })
}

fn is_fixed_layout(epub: &Epub) -> bool {
    let metadata = epub.metadata();
    for property in ["rendition:layout", "rendition#layout"] {
        if metadata
            .by_property(property)
            .any(|entry| entry.value() == "pre-paginated")
        {
            return true;
        }
    }
    epub.manifest().iter().any(|item| {
        item.properties()
            .as_str()
            .split_whitespace()
            .any(|property| property == "fixed-layout")
    })
}

fn toc_labels(epub: &Epub) -> Vec<(String, String)> {
    let mut labels = Vec::new();
    if let Some(root) = epub.toc().contents() {
        for entry in root.flatten() {
            let Some(manifest_entry) = entry.manifest_entry() else {
                continue;
            };
            let Ok(key) = resource_key(manifest_entry.href().decode().as_ref()) else {
                continue;
            };
            labels.push((key, entry.label().to_owned()));
        }
    }
    labels
}

/// Reduces one package href to its canonical archive key.
fn resource_key(href: &str) -> Result<String, String> {
    let without_fragment = href.split(['#', '?']).next().unwrap_or_default();
    let trimmed = without_fragment.trim_start_matches('/');
    canonical_key(trimmed)
        .map_err(|rejection| format!("resource href is not acceptable ({})", rejection.reason()))
}

struct ChapterContent {
    title: Option<String>,
    blocks: Vec<SemanticBlock>,
}

/// Joins converted chapters into one tiled multi-section document.
///
/// Consecutive blocks separate through one canonical blank line, including
/// across chapter boundaries, so visual spacing never depends on layout.
fn assemble(
    id: DocumentId,
    title: Option<String>,
    chapters: &[ChapterContent],
) -> Result<Document, String> {
    let mut canonical = String::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut pending_separator = false;

    for chapter in chapters {
        if chapter.blocks.is_empty() {
            continue;
        }
        let mut blocks = Vec::new();
        for block in &chapter.blocks {
            if pending_separator {
                let start = canonical.len();
                canonical.push('\n');
                blocks.push(Block::new(BlockKind::BlankLine, start..start + 1));
            } else {
                pending_separator = true;
            }
            let start = canonical.len();
            canonical.push_str(&block.text);
            let end = canonical.len();
            blocks.push(Block::new(block.kind, start..end));
        }
        sections.push(Section::new(chapter.title.clone(), blocks));
    }

    if sections.is_empty() {
        sections.push(Section::new(None, Vec::new()));
    }
    Document::from_sections(id, title, canonical, sections)
}
