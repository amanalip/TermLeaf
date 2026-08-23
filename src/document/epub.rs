//! EPUB ingestion: bounded archive preflight followed by `rbook` semantics.
//!
//! The pipeline follows the delivery plan exactly: open the immutable source
//! once, enforce every archive boundary in [`super::archive`], then let
//! `rbook` resolve the package, metadata, spine, and navigation over those
//! same inspected bytes. Chapters convert through the tolerant XHTML layer
//! into the shared document model; images and fonts stay lazy and unread.

use std::{collections::HashMap, path::Path, sync::Arc};

use rbook::Epub;

use super::archive::open_book_archive;
use super::model::{Block, BlockKind, ImageResource, InlineSpan, NavigationPoint, Section};
use super::text::file_stem_title;
use super::xhtml::{self, SemanticBlock, XhtmlBoundsError};
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
    EpubSnapshot::open(path, limits)?.build()
}

/// One EPUB source read exactly once and fully inspected.
///
/// Opening runs every archive boundary check; [`EpubSnapshot::build`] then
/// resolves package semantics over those same immutable bytes. The source
/// file is closed before `open` returns, so no later step ever touches the
/// path again (`EPUB-010`/`EPUB-016` byte stability).
#[derive(Debug)]
pub struct EpubSnapshot {
    display: String,
    archive: Arc<PreflightedArchive>,
}

impl EpubSnapshot {
    /// Stage one: reads the whole source once and preflights the archive.
    ///
    /// # Errors
    ///
    /// Returns every archive policy rejection before any semantic parsing;
    /// nothing reaches the terminal when this fails.
    pub fn open(path: &Path, limits: &ArchiveLimits) -> Result<Self, DocumentError> {
        let display = sanitize_path(&path.display().to_string());
        let archive = open_book_archive(path, limits).map_err(DocumentError::from)?;
        Ok(Self {
            display,
            archive: Arc::new(archive),
        })
    }

    /// Stage two: builds the logical document from inspected bytes alone.
    ///
    /// # Errors
    ///
    /// Returns the typed package failures; by construction these never
    /// depend on the current on-disk state of the source file.
    pub fn build(&self) -> Result<Document, DocumentError> {
        build_document(&self.display, &self.archive)
    }

    /// Clonable lazy access to the exact inspected archive bytes.
    #[must_use]
    pub fn resource_provider(&self) -> super::ResourceProvider {
        super::ResourceProvider::Epub(Arc::clone(&self.archive))
    }
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

    let toc = toc_entries(&epub);
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
        let title = toc
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.label.clone());
        let blocks = xhtml::convert_xhtml(source).map_err(
            |XhtmlBoundsError::TooManyNodes { nodes, limit }| DocumentError::ChapterTooComplex {
                path: display.to_owned(),
                member: sanitize_path(&key),
                nodes,
                limit,
            },
        )?;
        chapters.push(ChapterContent { title, key, blocks });
    }

    let title = epub
        .metadata()
        .title()
        .map(|title| title.value().trim().to_owned())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| file_stem_title(display));

    let id = DocumentId::new(format!("{display}:epub"));
    assemble(id, Some(title), &chapters, &toc, snapshot).map_err(|detail| {
        DocumentError::InvalidPackage {
            path: display.to_owned(),
            detail,
        }
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

struct TocEntry {
    label: String,
    key: String,
    fragment: Option<String>,
}

fn toc_entries(epub: &Epub) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    if let Some(root) = epub.toc().contents() {
        for entry in root.flatten() {
            let Some(href) = entry.href() else {
                continue;
            };
            let decoded = href.decode();
            let Ok(key) = resource_key(decoded.as_ref()) else {
                continue;
            };
            let fragment = decoded.split_once('#').map(|(_, value)| value.to_owned());
            entries.push(TocEntry {
                key,
                fragment,
                label: entry.label().to_owned(),
            });
        }
    }
    entries
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
    /// Canonical archive key of this chapter; image sources resolve
    /// relative to its directory.
    key: String,
    blocks: Vec<SemanticBlock>,
}

type LogicalLocation = (usize, usize, usize);
type LocationMap = HashMap<String, LogicalLocation>;
type PendingLink = (usize, String, String);

/// Joins converted chapters into one tiled multi-section document.
///
/// Consecutive blocks separate through one canonical blank line, including
/// across chapter boundaries, so visual spacing never depends on layout;
/// consecutive list items stay tight, their shared newline extending the
/// previous item's range so lists read as grouped entries. Inline
/// decorations and table cells shift to document-global byte ranges and
/// attach to the finished document. Declared images resolve against their
/// chapter location over the same inspected archive; anything external,
/// escaping, or missing becomes an unfetchable resource whose caption still
/// renders.
fn assemble(
    id: DocumentId,
    title: Option<String>,
    chapters: &[ChapterContent],
    toc: &[TocEntry],
    snapshot: &PreflightedArchive,
) -> Result<Document, String> {
    let mut canonical = String::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut inline: Vec<InlineSpan> = Vec::new();
    let mut pending_separator = false;
    let mut locations = LocationMap::new();
    let mut pending_links: Vec<PendingLink> = Vec::new();

    for chapter in chapters {
        if chapter.blocks.is_empty() {
            continue;
        }
        let base_dir = chapter
            .key
            .rsplit_once('/')
            .map_or(String::new(), |(dir, _)| dir.to_owned());
        let mut blocks: Vec<Block> = Vec::new();
        let section_index = sections.len();
        for (semantic_index, semantic) in chapter.blocks.iter().enumerate() {
            let previous_kind = blocks.last().map(Block::kind);
            let tight = pending_separator
                && matches!(previous_kind, Some(BlockKind::ListItem { .. }))
                && matches!(semantic.kind, BlockKind::ListItem { .. });
            if tight {
                // The shared newline joins the previous item's range, so no
                // blank row separates grouped entries while tiling stays
                // exact.
                let separator = canonical.len();
                canonical.push('\n');
                if let Some(last) = blocks.last_mut() {
                    last.extend_to(separator + 1);
                }
            } else if pending_separator {
                let start = canonical.len();
                canonical.push('\n');
                blocks.push(Block::new(BlockKind::BlankLine, start..start + 1));
            }
            pending_separator = true;

            let start = canonical.len();
            canonical.push_str(&semantic.text);
            for run in &semantic.inline {
                let span_index = inline.len();
                inline.push(InlineSpan::with_metadata(
                    run.kind,
                    start + run.range.start..start + run.range.end,
                    run.destination.clone(),
                    None,
                ));
                if let Some(destination) = &run.destination {
                    pending_links.push((span_index, chapter.key.clone(), destination.clone()));
                }
            }
            let model_block = blocks.len();
            if semantic_index == 0 {
                locations.insert(chapter.key.clone(), (section_index, model_block, 0));
            }
            for (id, offset) in &semantic.anchors {
                locations.entry(format!("{}#{id}", chapter.key)).or_insert((
                    section_index,
                    model_block,
                    *offset,
                ));
            }
            let range = start..canonical.len();
            if let Some(declared) = &semantic.image {
                let resource = resolve_image(&base_dir, declared.src(), snapshot)
                    .map_or_else(ImageResource::blocked, |(reference, byte_len)| {
                        ImageResource::member(reference, Some(byte_len))
                    });
                blocks.push(Block::image(range, resource));
            } else if semantic.cells.is_empty() {
                blocks.push(Block::new(semantic.kind, range));
            } else {
                let cells = semantic
                    .cells
                    .iter()
                    .map(|cell| start + cell.start..start + cell.end)
                    .collect();
                blocks.push(Block::table(range, cells));
            }
        }
        sections.push(Section::new(chapter.title.clone(), blocks));
    }

    if sections.is_empty() {
        sections.push(Section::new(None, Vec::new()));
    }
    let document = Document::from_sections(id, title, canonical, sections)?;
    finish_document(document, inline, pending_links, &locations, toc)
}

fn finish_document(
    document: Document,
    mut inline: Vec<InlineSpan>,
    pending_links: Vec<PendingLink>,
    locations: &LocationMap,
    toc: &[TocEntry],
) -> Result<Document, String> {
    for (span, chapter, destination) in pending_links {
        let target = resolve_internal_href(&chapter, &destination)
            .and_then(|key| locations.get(&key))
            .and_then(|(section, block, offset)| document.position(*section, *block, *offset).ok());
        inline[span].set_target(target);
    }
    let navigation: Vec<NavigationPoint> = toc
        .iter()
        .filter_map(|entry| {
            let key = entry.fragment.as_ref().map_or_else(
                || entry.key.clone(),
                |fragment| format!("{}#{fragment}", entry.key),
            );
            let (section, block, offset) = *locations.get(&key)?;
            let position = document.position(section, block, offset).ok()?;
            Some(NavigationPoint::new(entry.label.clone(), position))
        })
        .collect();
    document.with_inline(inline).map(|document| {
        if navigation.is_empty() {
            document
        } else {
            document.with_navigation(navigation)
        }
    })
}

fn resolve_internal_href(chapter_key: &str, href: &str) -> Option<String> {
    let (path, fragment) = href
        .split_once('#')
        .map_or((href, None), |(path, id)| (path, Some(id)));
    if path.contains('?') || fragment.is_some_and(str::is_empty) {
        return None;
    }
    let base_dir = chapter_key.rsplit_once('/').map_or("", |(dir, _)| dir);
    let key = if path.is_empty() {
        chapter_key.to_owned()
    } else {
        resolve_image_key(base_dir, path)?
    };
    let fragment = match fragment {
        Some(fragment) => Some(percent_decode(fragment)?),
        None => None,
    };
    Some(fragment.map_or(key.clone(), |id| format!("{key}#{id}")))
}

/// Resolves one declared image source against its chapter directory.
///
/// Returns the canonical member key and its declared byte size when the
/// target stays inside the book; scheme-prefixed, absolute, escaping,
/// empty, or missing targets yield `None` and must never be fetched.
fn resolve_image(
    base_dir: &str,
    src: &str,
    snapshot: &PreflightedArchive,
) -> Option<(String, u64)> {
    let key = resolve_image_key(base_dir, src)?;
    let info = snapshot.member(&key)?;
    Some((info.key().to_owned(), info.declared_size()))
}

/// Reduces a relative chapter reference to a canonical archive key.
///
/// Fragments and queries strip first, percent escapes decode strictly, a
/// scheme-like prefix rejects before any path logic, and dot segments merge
/// against the chapter directory without ever escaping the package root.
fn resolve_image_key(base_dir: &str, src: &str) -> Option<String> {
    let without_fragment = src.split(['#', '?']).next()?;
    let decoded = percent_decode(without_fragment)?;
    if decoded.is_empty() {
        return None;
    }
    // A scheme-like prefix such as `http:` or `data:` never resolves.
    if let Some(colon) = decoded.find(':') {
        let first_slash = decoded.find('/');
        if first_slash.is_none_or(|slash| colon < slash) {
            return None;
        }
    }
    let joined = if decoded.starts_with('/') {
        decoded.trim_start_matches('/').to_owned()
    } else if base_dir.is_empty() {
        decoded
    } else {
        format!("{base_dir}/{decoded}")
    };

    let mut segments: Vec<&str> = Vec::new();
    for segment in joined.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." if segments.pop().is_some() => {}
            ".." => return None,
            other => segments.push(other),
        }
    }
    if segments.is_empty() {
        return None;
    }
    canonical_key(&segments.join("/")).ok()
}

/// Decodes percent escapes strictly; malformed sequences reject.
fn percent_decode(input: &str) -> Option<String> {
    if !input.contains('%') {
        return Some(input.to_owned());
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let value = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod image_resolution_tests {
    use super::*;

    #[test]
    fn relative_sources_merge_against_the_chapter_directory() {
        assert_eq!(
            resolve_image_key("text", "images/pic.png").as_deref(),
            Some("text/images/pic.png")
        );
        assert_eq!(resolve_image_key("", "pic.png").as_deref(), Some("pic.png"));
        // Root-absolute references stay inside the archive by definition.
        assert_eq!(
            resolve_image_key("text", "/absolute/in/archive.png").as_deref(),
            Some("absolute/in/archive.png")
        );
    }

    #[test]
    fn dot_segments_merge_without_escaping_the_package_root() {
        assert_eq!(
            resolve_image_key("text/part1", "../shared/pic.png").as_deref(),
            Some("text/shared/pic.png")
        );
        assert_eq!(
            resolve_image_key("text", "./pic.png").as_deref(),
            Some("text/pic.png")
        );
        assert_eq!(resolve_image_key("text", "../../escape.png"), None);
    }

    #[test]
    fn schemes_fragments_and_queries_strip_or_reject() {
        assert_eq!(resolve_image_key("text", "http://host/x.png"), None);
        assert_eq!(resolve_image_key("text", "https://host/x.png"), None);
        assert_eq!(resolve_image_key("text", "data:image/png;base64,AA"), None);
        assert_eq!(resolve_image_key("text", "javascript:alert(1)"), None);
        assert_eq!(
            resolve_image_key("text", "pic.png#anchor").as_deref(),
            Some("text/pic.png")
        );
        assert_eq!(
            resolve_image_key("text", "pic.png?v=2").as_deref(),
            Some("text/pic.png")
        );
        assert_eq!(resolve_image_key("text", ""), None);
        assert_eq!(resolve_image_key("text", "#frag"), None);
    }

    #[test]
    fn percent_escapes_decode_strictly_or_reject() {
        assert_eq!(
            resolve_image_key("text", "my%20pics/pic.png").as_deref(),
            Some("text/my pics/pic.png")
        );
        assert_eq!(resolve_image_key("text", "bad%ZZ.png"), None);
        assert_eq!(resolve_image_key("text", "short%2.png"), None);
        assert_eq!(percent_decode("plain.png").as_deref(), Some("plain.png"));
    }
}

#[cfg(test)]
mod link_navigation_tests {
    use super::*;
    use crate::document::Position;

    fn assert_section_fallback(chapters: &[ChapterContent], snapshot: &PreflightedArchive) {
        let fallback = assemble(
            DocumentId::new("epub-fallback".to_owned()),
            None,
            chapters,
            &[],
            snapshot,
        )
        .expect("assembles without a usable TOC");
        assert_eq!(
            fallback.navigation_points().len(),
            fallback.sections().len()
        );
        for (index, point) in fallback.navigation_points().iter().enumerate() {
            assert_eq!(point.title(), fallback.sections()[index].title().unwrap());
            assert_eq!(point.position().section(), index);
            assert_eq!(point.position().block(), 0);
            assert_eq!(point.position().offset(), 0);
            assert_eq!(
                point.position().absolute_byte(&fallback),
                fallback.sections()[index].blocks()[0].range().start
            );
        }
    }

    #[test]
    fn epub_008_chapter_fragment_links_and_toc_resolve_inside_the_document() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/epub/minimal-epub2.epub");
        let snapshot = EpubSnapshot::open(&fixture, &ArchiveLimits::default()).expect("fixture");
        let first = xhtml::convert_xhtml(
            r#"<body><p>
                <a href="two.xhtml#container">container</a>
                <a href="two.xhtml#empty">empty</a>
                <a href="two.xhtml#spaced">spaced</a>
                <a href="two.xhtml#plate">plate</a>
            </p></body>"#,
        )
        .expect("source chapter");
        let second = xhtml::convert_xhtml(
            r#"<body>
                <section id="container"><h2>Same heading</h2></section>
                <a id="empty"></a><h2>Same heading</h2>
                <p>before <span id="spaced">  after</span></p>
                <p><img id="plate" src="plate.png" alt="plate"/></p>
            </body>"#,
        )
        .expect("target chapter");
        let chapters = vec![
            ChapterContent {
                title: Some("One".to_owned()),
                key: "text/one.xhtml".to_owned(),
                blocks: first,
            },
            ChapterContent {
                title: Some("Two".to_owned()),
                key: "text/two.xhtml".to_owned(),
                blocks: second,
            },
        ];
        let toc: Vec<TocEntry> = ["container", "empty", "spaced", "plate"]
            .into_iter()
            .map(|fragment| TocEntry {
                label: fragment.to_owned(),
                key: "text/two.xhtml".to_owned(),
                fragment: Some(fragment.to_owned()),
            })
            .collect();

        let document = assemble(
            DocumentId::new("epub008".to_owned()),
            None,
            &chapters,
            &toc,
            &snapshot.archive,
        )
        .expect("assembles");
        let targets: Vec<Position> = document
            .inline_spans()
            .iter()
            .map(|link| link.target().expect("internal target resolves"))
            .collect();
        assert_eq!(targets.len(), 4);
        assert_eq!(targets[0].section(), 1);
        assert_eq!(
            document.block_text(1, targets[0].block()),
            Some("Same heading")
        );
        assert_eq!(
            document.block_text(1, targets[1].block()),
            Some("Same heading")
        );
        assert_ne!(
            targets[0].block(),
            targets[1].block(),
            "duplicate text stays distinct"
        );
        assert_eq!(
            targets[2].offset(),
            6,
            "collapsed whitespace anchor is exact"
        );
        assert_eq!(
            document.sections()[1].blocks()[targets[3].block()].kind(),
            BlockKind::Image
        );
        let toc_positions: Vec<Position> = document
            .navigation_points()
            .iter()
            .map(NavigationPoint::position)
            .collect();
        assert_eq!(toc_positions, targets);

        assert_section_fallback(&chapters, &snapshot.archive);
    }
}
