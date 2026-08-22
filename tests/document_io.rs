//! Filesystem-boundary journeys: immutable sources, read-only books, and
//! right-to-left samples. These cases complement the process-level CLI tests
//! by exercising the library surface directly on real files.

use std::io::Write;

use anyhow::{Context, Result};
use termleaf::app::{Action, App, StartupOptions};
use termleaf::document::{TextLimits, sanitize_path};

fn temp_book(name: &str, contents: &str) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix(name)
        .suffix(".txt")
        .tempfile()
        .context("create temporary book")?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.flush())
        .context("write temporary book")?;
    Ok(file)
}

#[test]
fn cli_003_existing_supported_path_opens_one_immutable_source() -> Result<()> {
    let contents = "first paragraph\n\nsecond paragraph\n";
    let file = temp_book("cli003-book", contents)?;
    let before = std::fs::read(file.path()).context("snapshot source bytes")?;

    let app = App::open(StartupOptions {
        book: Some(file.path().to_path_buf()),
        ..StartupOptions::default()
    })
    .context("a supported local path opens")?;

    let session = app.reader().context("the reader session opens the book")?;
    assert_eq!(session.document().canonical(), contents);
    assert!(matches!(app.view(), termleaf::app::View::Reader { .. }));

    let after = std::fs::read(file.path()).context("reread source bytes")?;
    assert_eq!(before, after, "opening never rewrites the source");
    Ok(())
}

#[test]
fn txt_010_read_only_source_survives_navigation_and_exit() -> Result<()> {
    let contents = "read only line one\nread only line two\n".repeat(30);
    let file = temp_book("readonly-book", &contents)?;
    let mut permissions = std::fs::metadata(file.path())
        .context("source metadata")?
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(file.path(), permissions).context("mark source read-only")?;
    let before = std::fs::read(file.path()).context("snapshot source bytes")?;

    let mut app = App::open(StartupOptions {
        book: Some(file.path().to_path_buf()),
        ..StartupOptions::default()
    })?;
    app.set_content_viewport(60, 12);
    for _ in 0..10 {
        app.update(Action::NextPage);
        app.update(Action::NextLine);
    }
    app.update(Action::DocumentEnd);
    app.update(Action::Quit);
    assert!(!app.is_running());

    assert!(
        std::fs::metadata(file.path())?.permissions().readonly(),
        "permissions stay read-only"
    );
    assert_eq!(
        before,
        std::fs::read(file.path()).context("reread source bytes")?,
        "navigation and exit leave the source untouched"
    );
    Ok(())
}

#[test]
fn lay_014_right_to_left_samples_stay_bounded_and_logical() -> Result<()> {
    // TermLeaf lays out logical order only; visual bidi reordering stays an
    // explicitly documented limitation until its dedicated phase.
    let arabic = "\u{627}\u{644}\u{633}\u{644}\u{627}\u{645} \u{639}\u{644}\u{64A}\u{643}\u{645} \u{648}\u{631}\u{62D}\u{645}\u{629}\n";
    let hebrew =
        "\u{5E9}\u{5DC}\u{5D5}\u{5DD} \u{5E2}\u{5D5}\u{5DC}\u{5DD} \u{5E9}\u{5DC}\u{5D5}\u{5DD}\n";
    let mixed_direction = "start \u{627}\u{644}\u{639}\u{631}\u{628}\u{64A}\u{629} middle \u{5E2}\u{5D1}\u{5E8}\u{5D9}\u{5EA} end\n";
    let source = format!("{arabic}{hebrew}\n{mixed_direction}");

    let document = termleaf::document::text::load_text_bytes(
        "rtl-samples.txt",
        source.as_bytes(),
        &TextLimits::default(),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;

    assert_eq!(document.canonical(), &source, "logical order is preserved");

    for width in [4_u16, 9, 17, 40, 120] {
        let layout = termleaf::layout::layout_document(&document, width);
        let canonical_len = document.len();
        let mut last_end = 0_usize;
        for row in layout.rows() {
            for span in row.spans() {
                let range = span.range();
                assert!(range.start >= last_end && range.end <= canonical_len);
                last_end = range.end;
            }
        }
    }

    // Navigation across the whole document stays valid at every width.
    for width in [8_u16, 23] {
        let layout = termleaf::layout::layout_document(&document, width);
        let start = document.first_position().expect("start");
        let mut anchor = start;
        while let Some(next) = termleaf::reader::step_line(
            &layout,
            &document,
            anchor,
            termleaf::reader::Direction::TowardEnd,
        ) {
            assert!(next.absolute_byte(&document) >= anchor.absolute_byte(&document));
            anchor = next;
        }
    }
    Ok(())
}

#[test]
fn err_003_sanitized_paths_keep_diagnostics_terminal_safe() {
    let hostile = "bad\u{1b}[2J\u{7F}name.txt";
    let escaped = sanitize_path(hostile);

    assert!(!escaped.contains('\u{1b}'));
    assert!(!escaped.contains('\u{7F}'));
    assert!(escaped.contains("^["), "escape becomes caret notation");
    assert!(escaped.ends_with("name.txt"), "readable parts survive");
}

const EPUB2_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/epub/minimal-epub2.epub"
);
const EPUB3_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/epub/minimal-epub3.epub"
);

fn archive_limits() -> termleaf::document::ArchiveLimits {
    termleaf::document::ArchiveLimits::default()
}

fn load_book(path: &std::path::Path) -> anyhow::Result<termleaf::document::Document> {
    termleaf::document::load_book_file(path, &TextLimits::default(), &archive_limits())
        .map_err(|error| anyhow::anyhow!("{error}"))
}

/// Writes a minimal EPUB-shaped archive with full member control.
struct TempEpub(tempfile::NamedTempFile);

impl TempEpub {
    fn new(name: &str, members: &[(&str, &str)]) -> anyhow::Result<Self> {
        let owned: Vec<(&str, String)> = members
            .iter()
            .map(|(path, body)| (*path, (*body).to_owned()))
            .collect();
        Self::new_from_strings(name, &owned)
    }

    /// Same archive writer with owned member bodies for dynamic content.
    fn new_from_strings(name: &str, members: &[(&str, String)]) -> anyhow::Result<Self> {
        let owned: Vec<(&str, Vec<u8>)> = members
            .iter()
            .map(|(path, body)| (*path, body.clone().into_bytes()))
            .collect();
        Self::new_from_bytes(name, &owned)
    }

    /// Archive writer accepting raw bytes for binary members such as images.
    fn new_from_bytes(name: &str, members: &[(&str, Vec<u8>)]) -> anyhow::Result<Self> {
        let file = tempfile::Builder::new()
            .prefix(name)
            .suffix(".epub")
            .tempfile()
            .context("create temporary epub")?;
        let mut writer = zip::ZipWriter::new(&file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (path, body) in members {
            use std::io::Write as _;
            writer
                .start_file(*path, options)
                .with_context(|| format!("start {path}"))?;
            writer.write_all(body).context("member body")?;
        }
        writer.finish().context("finish epub")?;
        file.as_file().sync_data().ok();
        Ok(Self(file))
    }

    fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

const CONTAINER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

#[test]
fn epub_001_minimal_epub2_follows_spine_order_with_metadata_and_ncx() -> anyhow::Result<()> {
    let path = std::path::Path::new(EPUB2_FIXTURE);
    let document = load_book(path)?;

    assert_eq!(document.title(), "TermLeaf Fixture EPUB 2");
    assert_eq!(document.sections().len(), 2, "linear spine order");
    let titles: Vec<_> = document
        .sections()
        .iter()
        .filter_map(|s| s.title())
        .collect();
    assert_eq!(
        titles,
        ["Chapter One", "Chapter Two"],
        "NCX labels title chapters"
    );

    let canonical = document.canonical();
    let first = canonical.find("garden gate").expect("chapter one body");
    let second = canonical.find("orchard wall").expect("chapter two body");
    assert!(first < second, "canonical text follows spine order");

    let heading_kind = termleaf::document::model::BlockKind::Heading { level: 1 };
    for (index, expected_heading) in [(0_usize, "Chapter One"), (1, "Chapter Two")] {
        let section = &document.sections()[index];
        assert!(
            section
                .blocks()
                .iter()
                .any(|block| block.kind() == heading_kind),
            "{expected_heading} converts as a level-one heading"
        );
    }
    Ok(())
}

#[test]
fn epub_002_minimal_epub3_nav_document_drives_the_same_reading_order() -> anyhow::Result<()> {
    let path = std::path::Path::new(EPUB3_FIXTURE);
    let document = load_book(path)?;

    assert_eq!(document.title(), "TermLeaf Fixture EPUB 3");
    let titles: Vec<_> = document
        .sections()
        .iter()
        .filter_map(|s| s.title())
        .collect();
    assert_eq!(
        titles,
        ["Chapter One", "Chapter Two"],
        "the nav document labels chapters"
    );
    assert!(document.canonical().contains("orchard wall"));
    Ok(())
}

#[test]
fn epub_004_missing_or_untrustworthy_metadata_falls_back_to_the_filename() -> anyhow::Result<()> {
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title></dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:fallback</dc:identifier>
  </metadata>
  <manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#;
    let chapter = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
                   <p>fallback body text</p></body></html>";
    let book = TempEpub::new(
        "epub004-fallback",
        &[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c1.xhtml", chapter),
        ],
    )?;

    let document = load_book(book.path())?;
    let stem = book
        .path()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("temp stem")?;
    assert_eq!(document.title(), stem, "empty title falls back to the stem");
    assert_eq!(document.sections().len(), 1);
    Ok(())
}

#[test]
fn epub_006_encrypted_content_rejects_before_any_resource_decoding() -> anyhow::Result<()> {
    let encryption = r#"<?xml version="1.0"?>
<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <enc:EncryptedData xmlns:enc="http://www.w3.org/2001/04/xmlenc#">
    <enc:CipherData><enc:CipherReference URI="OEBPS/ch1.xhtml"/></enc:CipherData>
  </enc:EncryptedData>
</encryption>
"#;
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Encrypted Fixture</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:encrypted</dc:identifier>
  </metadata>
  <manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#;
    let chapter = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body><p>x</p></body></html>";
    let book = TempEpub::new(
        "epub006-encrypted",
        &[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("META-INF/encryption.xml", encryption),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c1.xhtml", chapter),
        ],
    )?;

    let error = load_book(book.path()).expect_err("encrypted books reject");
    let message = error.to_string();
    assert!(message.contains("encrypted"), "{message}");
    assert!(message.contains("encryption.xml"), "{message}");
    Ok(())
}

#[test]
fn epub_007_fixed_layout_books_receive_a_specific_unsupported_message() -> anyhow::Result<()> {
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         unique-identifier="bookid" version="3.0">
  <metadata>
    <dc:title>Fixed Layout Fixture</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:fixed</dc:identifier>
    <meta property="rendition:layout">pre-paginated</meta>
  </metadata>
  <manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#;
    let page = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
               <p>one fixed page</p></body></html>";
    let book = TempEpub::new(
        "epub007-fixed",
        &[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c1.xhtml", page),
        ],
    )?;

    let error = load_book(book.path()).expect_err("fixed layout rejects");
    let message = error.to_string();
    assert!(message.contains("fixed layout"), "{message}");
    assert!(message.contains("reflowable"), "{message}");
    Ok(())
}

#[test]
fn epub_003_nonlinear_spine_resources_stay_out_of_the_reading_order() -> anyhow::Result<()> {
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Nonlinear Fixture</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:nonlinear</dc:identifier>
  </metadata>
  <manifest>
    <item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
    <item id="notes" href="notes.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="notes" linear="no"/>
  </spine>
</package>
"#;
    let main_chapter = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
                        <p>linear prose only</p></body></html>";
    let notes = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
                 <p>supplementary notes stay outside the linear order</p></body></html>";
    let book = TempEpub::new(
        "epub003-nonlinear",
        &[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c1.xhtml", main_chapter),
            ("OEBPS/notes.xhtml", notes),
        ],
    )?;

    let document = load_book(book.path())?;
    assert_eq!(
        document.sections().len(),
        1,
        "nonlinear entries are skipped"
    );
    assert!(document.canonical().contains("linear prose only"));
    assert!(!document.canonical().contains("supplementary notes"));
    Ok(())
}

#[test]
fn epub_014_binary_resources_stay_lazy_during_text_opening() -> anyhow::Result<()> {
    // Garbage font and image bytes would fail any decoder; a successful open
    // proves text extraction never decodes lazy binary resources.
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         unique-identifier="bookid" version="2.0">
  <metadata>
    <dc:title>Lazy Resources Fixture</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:lazy</dc:identifier>
  </metadata>
  <manifest>
    <item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
    <item id="font" href="font.otf" media-type="font/otf"/>
    <item id="img" href="pic.png" media-type="image/png"/>
  </manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#;
    let chapter = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
                   <p>text extraction only</p></body></html>";
    let garbage_font = "this is not an OpenType font";
    let garbage_image = "neither is this a PNG";
    let book = TempEpub::new(
        "epub014-lazy",
        &[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c1.xhtml", chapter),
            ("OEBPS/font.otf", garbage_font),
            ("OEBPS/pic.png", garbage_image),
        ],
    )?;

    let document = load_book(book.path())?;
    assert_eq!(document.title(), "Lazy Resources Fixture");
    assert_eq!(document.sections().len(), 1);
    assert!(document.canonical().contains("text extraction only"));
    Ok(())
}

impl TempEpub {
    /// Replaces the whole file content after the archive was written.
    fn overwrite(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path())
            .context("reopen epub for rewrite")?;
        file.write_all(bytes).context("overwrite epub bytes")?;
        file.sync_all().context("sync overwritten epub")?;
        Ok(())
    }

    /// Cuts the file down to `len` bytes without touching its bytes before.
    fn truncate_to(&self, len: u64) -> anyhow::Result<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(self.path())
            .context("reopen epub for truncation")?;
        file.set_len(len).context("truncate epub")?;
        file.sync_all().context("sync truncated epub")?;
        Ok(())
    }

    /// Appends raw bytes beyond the original end of file.
    fn append(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(self.path())
            .context("reopen epub for append")?;
        file.write_all(bytes).context("append epub bytes")?;
        file.sync_all().context("sync appended epub")?;
        Ok(())
    }
}

const STABLE_CHAPTER: &str = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
                              <p>the original inspected passage</p></body></html>";

fn stable_edition_members(title: &str) -> Vec<(&'static str, String)> {
    let opf = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:stability</dc:identifier>
  </metadata>
  <manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#
    );
    vec![
        ("mimetype", "application/epub+zip".to_owned()),
        ("META-INF/container.xml", CONTAINER.to_owned()),
        ("OEBPS/content.opf", opf),
        ("OEBPS/c1.xhtml", STABLE_CHAPTER.to_owned()),
    ]
}

#[test]
fn epub_010_mutations_after_preflight_never_reach_semantic_parsing() -> anyhow::Result<()> {
    // A second, structurally valid book used as hostile replacement content.
    let tampered = TempEpub::new_from_strings(
        "epub010-tampered",
        &stable_edition_members("Tampered Edition"),
    )?;
    let tampered_bytes = std::fs::read(tampered.path()).context("read replacement bytes")?;

    let book =
        TempEpub::new_from_strings("epub010-stable", &stable_edition_members("Stable Edition"))?;
    let snapshot = termleaf::document::EpubSnapshot::open(book.path(), &archive_limits())
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    // Replace the source with a different complete book after inspection.
    book.overwrite(&tampered_bytes)?;
    let document = snapshot
        .build()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    assert_eq!(
        document.title(),
        "Stable Edition",
        "replacement bytes never reach semantic parsing"
    );
    assert!(
        document
            .canonical()
            .contains("the original inspected passage")
    );

    // Truncation and appended garbage are equally invisible to the builder.
    book.truncate_to(0)?;
    let document = snapshot
        .build()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    assert_eq!(document.title(), "Stable Edition");

    book.append(b"\x00\x01hostile tail")?;
    let document = snapshot
        .build()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    assert_eq!(document.sections().len(), 1);
    assert!(
        document
            .canonical()
            .contains("the original inspected passage")
    );
    Ok(())
}

#[test]
fn epub_016_source_disappears_or_swaps_after_inspection_and_parsing_still_succeeds()
-> anyhow::Result<()> {
    let decoy =
        TempEpub::new_from_strings("epub016-decoy", &stable_edition_members("Decoy Edition"))?;

    let book = TempEpub::new_from_strings(
        "epub016-vanishing",
        &stable_edition_members("Vanishing Edition"),
    )?;
    let original_path = book.path().to_path_buf();
    let snapshot = termleaf::document::EpubSnapshot::open(book.path(), &archive_limits())
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    // Rename the inspected source away, then delete it outright.
    let moved_aside = original_path.with_extension("moved.epub");
    std::fs::rename(&original_path, &moved_aside).context("rename inspected source")?;
    let document = snapshot
        .build()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    assert_eq!(document.title(), "Vanishing Edition");
    assert!(
        document
            .canonical()
            .contains("the original inspected passage")
    );

    std::fs::remove_file(&moved_aside).context("delete inspected source")?;
    let document = snapshot
        .build()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    assert_eq!(document.title(), "Vanishing Edition");

    // With the path recreated as a symlink to a different book, the builder
    // still reports only the originally inspected bytes (Unix: unprivileged
    // symlink creation is unavailable on Windows).
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(decoy.path(), &original_path)
            .context("swap source for a decoy symlink")?;
        let document = snapshot
            .build()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        assert_eq!(
            document.title(),
            "Vanishing Edition",
            "a swapped path cannot change what was already inspected"
        );
        assert!(!document.canonical().contains("decoy"));
    }
    Ok(())
}

#[test]
fn md_001_markdown_books_open_through_the_unified_loader() -> anyhow::Result<()> {
    let source = "# Chapter Head\n\n\
                  Body with *stress* and **force**.\n\n\
                  - one\n- two\n";
    let file = tempfile::Builder::new()
        .prefix("md001")
        .suffix(".md")
        .tempfile()
        .context("create temporary markdown book")?;
    std::fs::write(file.path(), source).context("write markdown book")?;

    let document = load_book(file.path())?;
    assert_eq!(document.sections().len(), 1);
    assert!(document.canonical().contains("Chapter Head"));
    assert!(
        document.canonical().contains("one\ntwo"),
        "tight list joins"
    );
    Ok(())
}

#[test]
fn cli_007_markdown_content_still_validates_after_the_extension_gate() -> anyhow::Result<()> {
    let file = tempfile::Builder::new()
        .prefix("not-really")
        .suffix(".md")
        .tempfile()
        .context("create misleading markdown book")?;
    std::fs::write(file.path(), [0xFF, 0xFE, 0x00, 0x01]).context("write invalid UTF-8")?;

    let error =
        termleaf::document::load_book_file(file.path(), &TextLimits::default(), &archive_limits())
            .expect_err("invalid content rejects after the gate");
    let message = error.to_string();
    assert!(message.contains("could not read"), "{message}");
    assert!(message.contains("UTF-8"), "{message}");
    Ok(())
}

/// Builds one small EPUB whose chapter body is supplied verbatim.
fn epub_with_chapter(
    name: &str,
    title: &str,
    chapter: &str,
    png: &[u8],
) -> anyhow::Result<TempEpub> {
    let opf = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:{title}</dc:identifier>
  </metadata>
  <manifest>
    <item id="c1" href="text/c1.xhtml" media-type="application/xhtml+xml"/>
    <item id="img1" href="images/red.png" media-type="image/png"/>
  </manifest>
  <spine><itemref idref="c1"/></spine>
</package>
"#
    );
    TempEpub::new_from_bytes(
        name,
        &[
            ("mimetype", b"application/epub+zip".to_vec()),
            ("META-INF/container.xml", CONTAINER.as_bytes().to_vec()),
            ("OEBPS/content.opf", opf.as_bytes().to_vec()),
            ("OEBPS/text/c1.xhtml", chapter.as_bytes().to_vec()),
            ("OEBPS/images/red.png", png.to_vec()),
        ],
    )
}

#[test]
fn epub_013_embedded_images_resolve_to_lazy_member_resources() -> anyhow::Result<()> {
    use termleaf::document::model::BlockKind;

    // One uniform red 2x2 PNG generated through the enabled decoder stack.
    let png = {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([255, 0, 0, 255]),
        ));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .context("encode fixture png")?;
        cursor.into_inner()
    };

    let chapter = concat!(
        r#"<html xmlns="http://www.w3.org/1999/xhtml"><body>"#,
        r#"<p>before the plate</p>"#,
        r#"<p><img src="../images/red.png" alt="a red square"/></p>"#,
        r#"<p>after the plate</p>"#,
        r#"</body></html>"#
    );
    let book = epub_with_chapter("epub013-illustrated", "Illustrated Fixture", chapter, &png)?;

    let document = load_book(book.path())?;
    assert_eq!(document.sections().len(), 1);

    let images: Vec<_> = document.sections()[0]
        .blocks()
        .iter()
        .filter(|block| block.kind() == BlockKind::Image)
        .collect();
    assert_eq!(images.len(), 1, "exactly one declared image converts");

    let block = images[0];
    let caption = document
        .block_text(0, 2)
        .context("image block carries its caption")?;
    assert_eq!(caption, "[image: a red square]");
    assert!(
        document.canonical().contains(caption),
        "captions tile into the canonical text"
    );

    let resource = block.resource().context("image resource present")?;
    assert_eq!(resource.reference(), Some("OEBPS/images/red.png"));
    assert_eq!(resource.byte_len(), Some(png.len() as u64));
    assert!(resource.is_fetchable());

    // Hostile and external targets stay inside the book but never resolve.
    for hostile_src in [
        "http://host/evil.png",
        "../../outside.png",
        "/abs.png",
        "missing/plate.png",
    ] {
        let src = format!(
            "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>\
             <img src=\"{hostile_src}\" alt=\"hostile\"/></body></html>"
        );
        let book = epub_with_chapter("epub013-hostile", "Hostile Fixture", &src, &png)?;
        let document = load_book(book.path())?;
        let images: Vec<_> = document.sections()[0]
            .blocks()
            .iter()
            .filter(|block| block.kind() == BlockKind::Image)
            .collect();
        assert_eq!(images.len(), 1, "{hostile_src}");
        let resource = images[0].resource().context("resource present")?;
        assert!(!resource.is_fetchable(), "{hostile_src} must not resolve");
        assert_eq!(resource.reference(), None);
        assert_eq!(
            document.block_text(0, 0),
            Some("[image: hostile]"),
            "{hostile_src}: the caption still renders"
        );
    }
    Ok(())
}
