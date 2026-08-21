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
