//! Render-layer integration cases on Ratatui's `TestBackend`.
//!
//! Every reviewed render assertion here accompanies direct cell, bounds,
//! anchor, and non-color checks. This suite is the executable body of the
//! `pr-render` profile; snapshot review notes live in `testreport.md`.

use std::{io::Write, time::Duration};

use anyhow::{Context, Result};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};
use termleaf::app::{Action, App, StartupOptions, View};
use termleaf::terminal_image::{CellPixelSize, ImageBackend, NativeFramePlan};
use termleaf::ui::theme::ColorMode;
use termleaf::ui::{status::MESSAGE_LIFETIME, theme::ThemeName};

fn book(contents: &str) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("render-book")
        .suffix(".txt")
        .tempfile()?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.flush())
        .context("write render fixture")?;
    Ok(file)
}

fn reader_app(contents: &str) -> Result<App> {
    App::open(StartupOptions {
        book: Some(book(contents)?.path().to_path_buf()),
        ..StartupOptions::default()
    })
}

fn draw(app: &mut App, width: u16, height: u16) -> Result<Buffer> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| termleaf::ui::render(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}

fn draw_with_native(app: &mut App, width: u16, height: u16) -> Result<(Buffer, NativeFramePlan)> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    let mut native = NativeFramePlan::default();
    terminal.draw(|frame| termleaf::ui::render_with_native(frame, app, &mut native))?;
    Ok((terminal.backend().buffer().clone(), native))
}

fn markdown_image_app(bytes: &[u8], alt: &str) -> Result<(tempfile::TempDir, App)> {
    let directory = tempfile::tempdir()?;
    let book = directory.path().join("illustrated.md");
    std::fs::write(
        &book,
        format!("before image\n\n![{alt}](plate.png)\n\nafter image\n"),
    )?;
    std::fs::write(directory.path().join("plate.png"), bytes)?;
    let app = App::open(StartupOptions {
        book: Some(book),
        ..StartupOptions::default()
    })?;
    Ok((directory, app))
}

fn markdown_app(source: &str) -> Result<(tempfile::TempDir, App)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("structured.md");
    std::fs::write(&path, source)?;
    let app = App::open(StartupOptions {
        book: Some(path),
        ..StartupOptions::default()
    })?;
    Ok((directory, app))
}

fn red_png() -> Result<Vec<u8>> {
    let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([220, 10, 20, 255]));
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source).write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

fn epub_app(chapter: &str, png: &[u8]) -> Result<(tempfile::NamedTempFile, App)> {
    const CONTAINER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
    const OPF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Semantic Render Fixture</dc:title><dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:termleaf:semantic-render</dc:identifier>
  </metadata>
  <manifest>
    <item id="c1" href="text/c1.xhtml" media-type="application/xhtml+xml"/>
    <item id="img1" href="images/red.png" media-type="image/png"/>
  </manifest>
  <spine><itemref idref="c1"/></spine>
</package>"#;

    let file = tempfile::Builder::new()
        .prefix("semantic-render")
        .suffix(".epub")
        .tempfile()?;
    let mut writer = zip::ZipWriter::new(&file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (path, body) in [
        ("mimetype", b"application/epub+zip".as_slice()),
        ("META-INF/container.xml", CONTAINER.as_bytes()),
        ("OEBPS/content.opf", OPF.as_bytes()),
        ("OEBPS/text/c1.xhtml", chapter.as_bytes()),
        ("OEBPS/images/red.png", png),
    ] {
        writer.start_file(path, options)?;
        writer.write_all(body)?;
    }
    writer.finish()?;
    file.as_file().sync_data().ok();
    let app = App::open(StartupOptions {
        book: Some(file.path().to_path_buf()),
        ..StartupOptions::default()
    })?;
    Ok((file, app))
}

fn draw_until(
    app: &mut App,
    width: u16,
    height: u16,
    predicate: impl Fn(&Buffer) -> bool,
) -> Result<Buffer> {
    let mut last = String::new();
    for _ in 0..100 {
        let rendered = draw(app, width, height)?;
        if predicate(&rendered) {
            return Ok(rendered);
        }
        last = (0..height)
            .map(|y| row_text(&rendered, width, y).trim_end().to_owned())
            .filter(|row| !row.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
        std::thread::sleep(Duration::from_millis(5));
    }
    anyhow::bail!("image worker did not reach the expected render state: {last}")
}

fn contents(buffer: &Buffer) -> String {
    buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn row_text(buffer: &Buffer, width: u16, y: u16) -> String {
    (0..width).map(|x| buffer[(x, y)].symbol()).collect()
}

fn row_containing(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|y| row_text(buffer, buffer.area.width, *y).contains(needle))
}

fn is_clock_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 5
        && bytes[2] == b':'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
}

#[test]
fn theme_003_switching_themes_mid_passage_keeps_the_anchor() -> Result<()> {
    let passage = "wrapped words drift across the boundary of this line\n".repeat(20);
    let mut app = reader_app(&passage)?;
    app.set_content_viewport(70, 18);
    app.update(Action::NextPage);
    app.update(Action::NextLine);

    let anchor_before = app.reader().context("reader")?.anchor();
    for _ in 0..ThemeName::ALL.len() {
        app.update(Action::ShowThemes);
        assert!(matches!(app.view(), View::ThemeSelection { .. }));
        app.update(Action::NextLine); // Move to the next theme in the list.
        app.update(Action::Confirm);

        assert_eq!(
            app.reader().context("reader persists")?.anchor(),
            anchor_before,
            "the logical anchor never moves with a theme switch"
        );
        let rendered = draw(&mut app, 80, 24)?;
        assert!(contents(&rendered).contains("wrapped words"));
        assert_eq!(
            app.theme(),
            ThemeName::ALL[app.theme_cursor()],
            "the applied theme tracks the confirmed selection"
        );
    }
    Ok(())
}

#[test]
fn theme_004_paper_true_color_roles_render_exactly() -> Result<()> {
    let mut app = reader_app("true color passage\n")?;
    app.set_color_mode(ColorMode::TrueColor);
    let rendered = draw(&mut app, 80, 24)?;

    // Body text carries the exact charcoal ink; Paper styles are
    // foreground-only, so the page surface stays terminal-default.
    let body = &rendered[(4, 4)];
    assert_eq!(body.symbol(), "t", "passage starts at the padded column");
    assert_eq!(body.fg, Color::Rgb(0x29, 0x28, 0x21), "Text role");
    assert_eq!(body.bg, Color::Reset, "Paper keeps terminal backgrounds");

    // The rounded page boundary carries the olive accent.
    let border = &rendered[(2, 6)];
    assert_eq!(border.symbol(), "│", "the page boundary sits two cells in");
    assert_eq!(border.fg, Color::Rgb(0x4F, 0x5D, 0x38), "Accent role");
    Ok(())
}

#[test]
fn theme_006_paper_collapses_canvas_padding_then_boundary() -> Result<()> {
    let mut app = reader_app("collapse order passage\n")?;
    app.set_color_mode(ColorMode::TrueColor);

    // Wide keeps a four-cell outer canvas before the page boundary.
    let wide = draw(&mut app, 120, 30)?;
    assert_eq!(wide[(4, 10)].symbol(), "│");
    assert_eq!(wide[(3, 10)].symbol(), " ", "canvas occupies four cells");

    // Standard shrinks the canvas first.
    let standard = draw(&mut app, 80, 24)?;
    assert_eq!(standard[(2, 10)].symbol(), "│");
    assert_eq!(standard[(1, 10)].symbol(), " ");

    // Compact shrinks it again while the boundary remains.
    let compact = draw(&mut app, 64, 20)?;
    assert_eq!(compact[(1, 10)].symbol(), "│");

    // Narrow removes the boundary last and keeps content usable.
    let narrow = draw(&mut app, 48, 16)?;
    let narrow_contents = contents(&narrow);
    assert!(!narrow_contents.contains('╭'), "boundary removed last");
    assert!(!narrow_contents.contains('│'));
    assert!(narrow_contents.contains("collapse order passage"));

    // Below minimum suspends; recovery returns the same passage.
    let suspended = draw(&mut app, 12, 3)?;
    assert!(contents(&suspended).contains("Terminal"));
    let recovered = draw(&mut app, 80, 24)?;
    assert!(contents(&recovered).contains("collapse order passage"));
    Ok(())
}

#[test]
fn theme_009_paper_matrix_over_phase_one_states_stays_readable() -> Result<()> {
    let modes = [
        ColorMode::TrueColor,
        ColorMode::Ansi256,
        ColorMode::TerminalDefault,
    ];
    let sizes = [(120_u16, 40_u16), (80, 24), (40, 10), (20, 4), (10, 2)];

    for mode in modes {
        for (width, height) in sizes {
            let mut app = reader_app("matrix passage for every capability\n")?;
            app.set_color_mode(mode);

            let reader_view = draw(&mut app, width, height)?;
            if width < 20 || height < 4 {
                assert!(
                    contents(&reader_view).contains("Terminal"),
                    "{mode:?} {width}x{height}: below minimum must suspend"
                );
                continue;
            }
            assert!(
                contents(&reader_view).contains("matrix passage"),
                "{mode:?} {width}x{height}: anchor passage must stay visible"
            );

            // Non-color cue: the status band is reverse video.
            let status_has_cue = (0..width).any(|x| {
                reader_view[(x, height - 1)]
                    .style()
                    .add_modifier
                    .contains(Modifier::REVERSED)
            });
            assert!(
                status_has_cue,
                "{mode:?} {width}x{height}: status needs a non-color cue"
            );

            // Help renders under the same capability decision.
            app.update(Action::ShowHelp);
            let help_view = draw(&mut app, width, height)?;
            if width >= 40 && height >= 10 {
                assert!(contents(&help_view).contains("Reader commands"));
            }
        }
    }
    Ok(())
}

#[test]
fn render_002_empty_short_long_and_error_states_render_safely() -> Result<()> {
    let mut empty = reader_app("")?;
    let rendered = draw(&mut empty, 80, 24)?;
    assert!(contents(&rendered).contains("PAGED"));

    let mut short = reader_app("tiny\n")?;
    let rendered = draw(&mut short, 80, 24)?;
    assert!(contents(&rendered).contains("tiny"));

    let long_book = "long book paragraph with several words\n".repeat(400);
    let mut long = reader_app(&long_book)?;
    let rendered = draw(&mut long, 80, 24)?;
    assert!(contents(&rendered).contains("long book paragraph"));

    let mut suspended = reader_app("suspended\n")?;
    let rendered = draw(&mut suspended, 15, 2)?;
    assert!(contents(&rendered).contains("Terminal"));
    Ok(())
}

#[test]
fn img_009_011_014_production_half_blocks_render_and_preserve_source() -> Result<()> {
    let png = {
        let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([220, 10, 20, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source).write_to(&mut cursor, image::ImageFormat::Png)?;
        cursor.into_inner()
    };
    let (directory, mut app) = markdown_image_app(&png, "red plate")?;
    app.set_color_mode(ColorMode::TrueColor);
    let anchor = app.reader().context("reader")?.anchor();

    let rendered = draw_until(&mut app, 80, 24, |buffer| contents(buffer).contains('▀'))?;

    assert!(contents(&rendered).contains("before image"));
    assert!(contents(&rendered).contains("after image"));
    assert_eq!(app.reader().context("reader")?.anchor(), anchor);
    assert!(
        rendered
            .content
            .iter()
            .any(|cell| { cell.symbol() == "▀" && matches!(cell.fg, Color::Rgb(220, 10, 20)) })
    );
    assert_eq!(std::fs::read(directory.path().join("plate.png"))?, png);
    Ok(())
}

#[test]
fn img_008_native_backends_collect_one_protocol_without_escape_cells() -> Result<()> {
    let png = red_png()?;
    for backend in [
        ImageBackend::Kitty,
        ImageBackend::Sixel,
        ImageBackend::Iterm2,
    ] {
        let (_directory, mut app) = markdown_image_app(&png, "native plate")?;
        app.set_image_backend(Some(backend));
        app.set_cell_pixel_size(CellPixelSize::new(8, 16));
        let mut result = None;
        for _ in 0..100 {
            let rendered = draw_with_native(&mut app, 80, 30)?;
            if !rendered.1.placements().is_empty() {
                result = Some(rendered);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let (buffer, plan) = result.context("native worker completes")?;
        assert_eq!(plan.placements().len(), 1);
        let placement = &plan.placements()[0];
        assert_eq!(placement.image.backend(), backend);
        assert!(placement.image.wire_bytes() > 0);
        let stable_id = placement.image.id();
        let caption = row_containing(&buffer, "[image: native plate]").context("caption")?;
        assert!(caption < placement.row);
        assert!(row_containing(&buffer, "after image").context("following text")? > placement.row);
        assert!(
            buffer
                .content
                .iter()
                .all(|cell| !cell.symbol().chars().any(char::is_control)),
            "{backend:?}: native bytes never enter Ratatui cells"
        );
        if backend == ImageBackend::Kitty {
            let mut resized = None;
            for _ in 0..100 {
                let candidate = draw_with_native(&mut app, 30, 30)?;
                if !candidate.1.placements().is_empty() {
                    resized = Some(candidate.1);
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(
                resized.context("resized native worker")?.placements()[0]
                    .image
                    .id(),
                stable_id,
                "resize replaces the payload under the same logical ID"
            );
            app.update(Action::ShowHelp);
            assert!(
                draw_with_native(&mut app, 30, 30)?
                    .1
                    .placements()
                    .is_empty()
            );
            app.update(Action::Back);
            assert_eq!(
                draw_with_native(&mut app, 30, 30)?.1.placements()[0]
                    .image
                    .id(),
                stable_id,
                "navigation away and back retains the image identity"
            );
        }
    }
    Ok(())
}

#[test]
fn native_sixel_missing_geometry_and_partial_images_render_explicit_fallbacks() -> Result<()> {
    let png = red_png()?;
    let (_directory, mut sixel) = markdown_image_app(&png, "sixel plate")?;
    sixel.set_image_backend(Some(ImageBackend::Sixel));
    let failed = draw_until(&mut sixel, 80, 30, |buffer| {
        contents(buffer).contains("requires measured termi")
    })?;
    assert!(contents(&failed).contains("sixel plate"));
    assert!(!contents(&failed).contains('▀'));

    let tall = {
        let source = image::RgbaImage::from_pixel(2, 20, image::Rgba([220, 10, 20, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source).write_to(&mut cursor, image::ImageFormat::Png)?;
        cursor.into_inner()
    };
    let (_directory, mut app) = markdown_image_app(&tall, "tall plate")?;
    app.set_image_backend(Some(ImageBackend::Kitty));
    let mut partial = None;
    for _ in 0..100 {
        let candidate = draw_with_native(&mut app, 30, 8)?;
        if contents(&candidate.0).contains("partially outside viewport") {
            partial = Some(candidate);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let (buffer, plan) = partial.context("partial native image fallback")?;
    assert!(
        plan.placements().is_empty(),
        "partial native output cannot scroll"
    );
    assert!(contents(&buffer).contains("partially outside viewport"));

    app.update(Action::NextLine);
    let mut visible = None;
    for _ in 0..100 {
        let candidate = draw_with_native(&mut app, 30, 8)?;
        if !candidate.1.placements().is_empty() {
            visible = Some(candidate.1);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        visible
            .context("fully visible after scroll")?
            .placements()
            .len(),
        1
    );
    Ok(())
}

#[test]
fn img_012_decoder_failure_keeps_text_and_reports_alt_dimensions_reason() -> Result<()> {
    let too_wide = {
        let source = image::RgbaImage::from_pixel(16_385, 1, image::Rgba([1, 2, 3, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source).write_to(&mut cursor, image::ImageFormat::Png)?;
        cursor.into_inner()
    };
    let (_directory, mut app) = markdown_image_app(&too_wide, "oversized plate")?;
    app.set_color_mode(ColorMode::TrueColor);

    let rendered = draw_until(&mut app, 100, 24, |buffer| {
        let text = contents(buffer);
        text.contains("16385x1") && text.contains("dimension limit")
    })?;
    let text = contents(&rendered);
    assert!(text.contains("oversized plate"), "{text}");
    assert!(text.contains("before image"), "{text}");
    assert!(!text.contains('▀'));
    app.update(Action::NextPage);
    assert!(
        contents(&draw(&mut app, 100, 24)?).contains("after image"),
        "text following the bounded placeholder remains reachable"
    );
    Ok(())
}

#[test]
fn md_003_tables_render_aligned_wide_and_linearized_narrow() -> Result<()> {
    let source = concat!(
        "before table\n\n",
        "| Specimen name | Recorded lifespan |\n",
        "| --- | --- |\n",
        "| Oak | Three centuries |\n\n",
        "after table\n",
    );
    let (_directory, mut app) = markdown_app(source)?;

    let wide = draw(&mut app, 100, 24)?;
    let wide_header = row_containing(&wide, "Specimen name").context("wide table header")?;
    assert!(
        row_text(&wide, 100, wide_header).contains("Specimen name | Recorded lifespan"),
        "wide columns share one readable row"
    );
    assert!(row_text(&wide, 100, wide_header + 1).contains("Oak"));
    assert!(row_text(&wide, 100, wide_header + 1).contains("Three centuries"));

    let narrow = draw(&mut app, 20, 40)?;
    let specimen = row_containing(&narrow, "Specimen name").context("narrow first header")?;
    let lifespan = row_containing(&narrow, "Recorded").context("narrow second header")?;
    let oak = row_containing(&narrow, "Oak").context("narrow first value")?;
    let centuries = row_containing(&narrow, "Three").context("narrow second value")?;
    assert!(specimen < lifespan && lifespan < oak && oak <= centuries);
    assert_ne!(specimen, lifespan, "narrow table linearizes the wide row");
    assert!(row_containing(&narrow, "before table").context("before")? < specimen);
    assert!(centuries < row_containing(&narrow, "after table").context("after")?);
    Ok(())
}

#[test]
fn md_007_code_render_preserves_tabs_blank_lines_and_logical_text() -> Result<()> {
    let code = concat!(
        "$ printf\tone\n",
        "\n",
        "  indented continuation\n",
        "abcdefghijklmnopqrstuvwxyz0123456789\n",
    );
    let source = format!("before code\n\n```console\n{code}```\n\nafter code\n");
    let (_directory, mut app) = markdown_app(&source)?;

    let wide = draw(&mut app, 100, 24)?;
    let prompt = row_containing(&wide, "$ printf").context("prompt")?;
    let prompt_text = row_text(&wide, 100, prompt);
    assert!(prompt_text.contains("one"));
    assert!(!prompt_text.contains('\t'), "tabs are expanded for display");
    let indented = row_containing(&wide, "  indented continuation").context("indent")?;
    assert_eq!(
        indented,
        prompt + 2,
        "the interior blank code line survives"
    );

    let narrow = draw(&mut app, 30, 24)?;
    let long_start = row_containing(&narrow, "abcdefghijklmnop").context("long line")?;
    assert!(
        row_text(&narrow, 30, long_start + 1).contains("456789"),
        "overlong code hard-splits without changing its logical text"
    );
    let document = app.reader().context("reader")?.document();
    let logical = document.sections()[0]
        .blocks()
        .iter()
        .enumerate()
        .find(|(_, block)| block.kind() == termleaf::document::BlockKind::CodeBlock)
        .and_then(|(index, _)| document.block_text(0, index))
        .context("logical code block")?;
    assert_eq!(logical, code);
    Ok(())
}

#[test]
fn md_011_image_success_and_fallback_preserve_caption_and_order() -> Result<()> {
    let png = red_png()?;
    for (width, height) in [(100_u16, 40_u16), (30, 40)] {
        let (directory, mut app) = markdown_image_app(&png, "red plate")?;
        app.set_color_mode(ColorMode::TrueColor);
        let rendered = draw_until(&mut app, width, height, |buffer| {
            contents(buffer).contains('▀')
        })
        .with_context(|| format!("MD-011 image at {width}x{height}"))?;
        let before = row_containing(&rendered, "before image").context("before image")?;
        let caption = row_containing(&rendered, "[image: red plate]").context("caption")?;
        let image = row_containing(&rendered, "▀").context("image cells")?;
        let after = row_containing(&rendered, "after image").context("after image")?;
        assert!(before < caption && caption < image && image < after);
        assert!(
            row_text(&rendered, width, caption).contains("[image: red plate]"),
            "ready pixels never overwrite the caption"
        );
        assert_eq!(std::fs::read(directory.path().join("plate.png"))?, png);
    }

    let (_missing_directory, mut missing) =
        markdown_app("before missing\n\n![missing plate](missing.png)\n\nafter missing\n")?;
    missing.set_color_mode(ColorMode::TrueColor);
    let failed = draw_until(&mut missing, 80, 40, |buffer| {
        contents(buffer).contains("missing plate") && contents(buffer).contains("read:")
    })?;
    let before = row_containing(&failed, "before missing").context("before fallback")?;
    let caption = row_containing(&failed, "[image: missing plate]").context("fallback caption")?;
    let reason = row_containing(&failed, "read:").context("fallback reason")?;
    let after = row_containing(&failed, "after missing").context("after fallback")?;
    assert!(before < caption && caption < reason && reason < after);
    Ok(())
}

#[test]
fn epub_012_semantic_fixture_renders_wide_and_narrow_in_source_order() -> Result<()> {
    let chapter = concat!(
        r#"<html xmlns="http://www.w3.org/1999/xhtml"><head>"#,
        r#"<style>@font-face{font-family:Ignored} table{display:grid}</style></head><body>"#,
        r#"<p>before semantics</p>"#,
        r#"<table><tr><th>Specimen name</th><th>Recorded lifespan</th></tr>"#,
        r#"<tr><td>Oak</td><td>Three centuries</td></tr></table>"#,
        "<pre>$ inspect\tone\n\n  indented result</pre>",
        r#"<p>after semantics</p></body></html>"#,
    );
    let (_book, mut app) = epub_app(chapter, &red_png()?)?;

    let wide = draw(&mut app, 100, 30)?;
    let header = row_containing(&wide, "Specimen name").context("wide EPUB table")?;
    assert!(row_text(&wide, 100, header).contains("Specimen name | Recorded lifespan"));
    let prompt = row_containing(&wide, "$ inspect").context("EPUB code")?;
    let indented = row_containing(&wide, "  indented result").context("EPUB indentation")?;
    assert_eq!(indented, prompt + 2, "EPUB code keeps its blank line");
    assert!(
        !contents(&wide).contains("Ignored"),
        "CSS and custom fonts stay inert"
    );

    let narrow = draw(&mut app, 20, 40)?;
    let before = row_containing(&narrow, "before semantics").context("before")?;
    let specimen = row_containing(&narrow, "Specimen name").context("first cell")?;
    let lifespan = row_containing(&narrow, "Recorded").context("second cell")?;
    let oak = row_containing(&narrow, "Oak").context("third cell")?;
    let centuries = row_containing(&narrow, "Three").context("fourth cell")?;
    let code = row_containing(&narrow, "$ inspect").context("code after table")?;
    let after = row_containing(&narrow, "after semantics").context("after")?;
    assert!(
        before < specimen
            && specimen < lifespan
            && lifespan < oak
            && oak <= centuries
            && centuries < code
            && code < after
    );
    assert_ne!(specimen, lifespan, "narrow EPUB table linearizes");
    Ok(())
}

#[test]
fn epub_013_embedded_image_renders_without_extraction_and_keeps_order() -> Result<()> {
    let png = red_png()?;
    let chapter = concat!(
        r#"<html xmlns="http://www.w3.org/1999/xhtml"><body>"#,
        r#"<p>before archive image</p>"#,
        r#"<p><img src="../images/red.png" alt="archive red plate"/></p>"#,
        r#"<p>after archive image</p></body></html>"#,
    );
    let (book, mut app) = epub_app(chapter, &png)?;
    let source_before = std::fs::read(book.path())?;
    app.set_color_mode(ColorMode::TrueColor);

    for (width, height) in [(100_u16, 40_u16), (30, 40)] {
        let rendered = draw_until(&mut app, width, height, |buffer| {
            contents(buffer).contains('▀')
        })?;
        let before = row_containing(&rendered, "before archive image").context("before")?;
        let caption = row_containing(&rendered, "[image: archive red plate]").context("caption")?;
        let image = row_containing(&rendered, "▀").context("image")?;
        let after = row_containing(&rendered, "after archive image").context("after")?;
        assert!(before < caption && caption < image && image < after);
        assert!(row_text(&rendered, width, caption).contains("archive red plate"));
    }
    assert_eq!(
        std::fs::read(book.path())?,
        source_before,
        "EPUB is never extracted or rewritten"
    );
    Ok(())
}

#[test]
fn render_003_unicode_cells_render_with_exact_widths_and_no_leaks() -> Result<()> {
    let source = "ascii words\ne\u{301}acute CJK 漢字テスト\nflag \u{1F1FA}\u{1F1F8} tone \u{1F44D}\u{1F3FD}\ntab\tstop ctrl \u{7}bell\n";
    let mut app = reader_app(source)?;
    let rendered = draw(&mut app, 80, 24)?;

    // The combining cluster stays one cell; CJK occupies whole cells.
    assert!(
        rendered
            .content
            .iter()
            .any(|cell| cell.symbol() == "e\u{301}"),
        "combining marks stay attached to their base"
    );
    assert!(
        rendered.content.iter().any(|cell| cell.symbol() == "漢"),
        "CJK glyphs occupy their own cells"
    );
    assert!(
        contents(&rendered).contains("\u{1F1FA}\u{1F1F8}"),
        "flag intact"
    );

    // Control bytes become caret pairs and never reach the grid raw.
    assert!(
        contents(&rendered).contains("^G"),
        "bell becomes caret notation"
    );
    assert!(
        !rendered
            .content
            .iter()
            .any(|cell| cell.symbol().chars().any(char::is_control)),
        "no raw control character reaches any cell"
    );
    Ok(())
}

#[test]
fn lay_013_unicode_placement_claims_match_support_limits() -> Result<()> {
    // Render-layer evidence for grapheme placement. Font-dependent GUI
    // verification stays owned by the release manual matrix; this pins the
    // application-side cell claims that do not depend on fonts.
    let source =
        "e\u{301} 漢字 \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} \u{1F1FA}\u{1F1F8}\n".repeat(6);
    let mut app = reader_app(&source)?;
    let rendered = draw(&mut app, 40, 12)?;

    let symbols: Vec<&str> = rendered
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    for claim in [
        "e\u{301}",
        "漢",
        "字",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        "\u{1F1FA}\u{1F1F8}",
    ] {
        assert!(
            symbols.contains(&claim),
            "{claim:?} must appear as an unsplit cell"
        );
    }
    Ok(())
}

#[test]
fn render_004_redraw_is_stable_and_single_field_changes_are_local() -> Result<()> {
    let mut app = App::open(StartupOptions::default())?;

    let first = draw(&mut app, 60, 14)?;
    let second = draw(&mut app, 60, 14)?;
    assert_eq!(first, second, "unchanged state must not alter output");

    // One field changes exactly one row: the home status line message.
    app.set_message("Theme: Dark");
    let changed = draw(&mut app, 60, 14)?;
    for y in 0..13 {
        for x in 0..60 {
            assert_eq!(
                first[(x, y)].symbol(),
                changed[(x, y)].symbol(),
                "row {y} changed unexpectedly"
            );
        }
    }
    assert!(row_text(&changed, 60, 13).contains("Theme: Dark"));

    // After the deterministic tick lifetime the baseline row returns.
    for _ in 0..MESSAGE_LIFETIME {
        app.update(Action::NextLine);
    }
    let restored = draw(&mut app, 60, 14)?;
    assert_eq!(first, restored, "message expiry restores the baseline");
    Ok(())
}

#[test]
fn status_001_full_status_fields_render_at_wide_width() -> Result<()> {
    let mut app = reader_app("status fields\n")?;
    let rendered = draw(&mut app, 120, 40)?;
    let status_row = row_text(&rendered, 120, 39);

    assert!(status_row.contains("render-book"), "title: {status_row}");
    assert!(status_row.contains("Loc "), "location: {status_row}");
    assert!(status_row.contains("Page "), "page: {status_row}");
    assert!(status_row.contains('%'), "percent: {status_row}");
    assert!(status_row.contains("PAGED"), "mode: {status_row}");
    assert!(status_row.contains("[?]"), "hint: {status_row}");
    assert!(
        row_text(&rendered, 120, 39)
            .split_whitespace()
            .any(is_clock_token),
        "clock: {status_row}"
    );
    Ok(())
}

#[test]
fn status_006_fields_collapse_in_priority_order_and_messages_restore() -> Result<()> {
    let mut app = reader_app("collapse stepping\n")?;

    // Record the first width where each token disappears while narrowing.
    let tokens = ["clock-token", "Page ", "render-book", "[?]", "Loc "];
    let mut present_at = |width: u16| -> Vec<bool> {
        let rendered = draw(&mut app, width, 30).expect("draw");
        let status_row = row_text(&rendered, width, 29);
        vec![
            status_row.split_whitespace().any(is_clock_token),
            status_row.contains("Page "),
            status_row.contains("render-book"),
            status_row.contains("[?]"),
            status_row.contains("Loc "),
        ]
    };

    let mut drop_widths: Vec<Option<u16>> = vec![None; tokens.len()];
    let mut previous = present_at(120);
    for width in (20..120).rev() {
        let current = present_at(width);
        for (index, was_present) in previous.iter().enumerate() {
            if *was_present && !current[index] && drop_widths[index].is_none() {
                drop_widths[index] = Some(width + 1);
            }
        }
        previous = current;
    }

    // Priority order: clock first, then page, then title; the hint follows.
    let clock_drop = drop_widths[0].expect("clock collapses");
    let page_drop = drop_widths[1].expect("page collapses");
    let title_drop = drop_widths[2].expect("title collapses");
    let hint_drop = drop_widths[3].expect("hint collapses");
    assert!(
        clock_drop >= page_drop && page_drop >= title_drop,
        "clock then page then title must drop first: {clock_drop} {page_drop} {title_drop}"
    );
    assert!(
        title_drop > hint_drop,
        "the hint outlives the title in the collapse order: {title_drop} {hint_drop}"
    );

    let final_state = present_at(20);
    assert!(
        !final_state[0] && !final_state[1],
        "high-priority fields stay gone"
    );
    let survivors = draw(&mut app, 20, 6)?;
    let survivor_row = row_text(&survivors, 20, 5);
    assert!(survivor_row.contains('%'), "{survivor_row}");
    assert!(survivor_row.contains("PAGED"), "{survivor_row}");

    // Temporary messages replace lower-priority fields and restore.
    let mut home = App::open(StartupOptions::default())?;
    let baseline = draw(&mut home, 60, 10)?;
    home.set_message("Theme: Dark");
    let showing = draw(&mut home, 60, 10)?;
    assert!(row_text(&showing, 60, 9).contains("Theme: Dark"));
    for _ in 0..MESSAGE_LIFETIME {
        home.update(Action::NextLine);
    }
    let restored = draw(&mut home, 60, 10)?;
    assert_eq!(
        row_text(&baseline, 60, 9),
        row_text(&restored, 60, 9),
        "expired messages restore the prior status"
    );
    Ok(())
}

#[test]
fn help_001_help_opens_from_every_phase_one_mode_and_returns() -> Result<()> {
    // From Recent books.
    let mut app = App::open(StartupOptions::default())?;
    app.update(Action::ShowHelp);
    assert!(matches!(app.view(), View::Help { .. }));
    app.update(Action::Back);
    assert_eq!(app.view(), &View::RecentBooks);

    // From Reader, returning to the exact same view identity.
    let mut app = reader_app("help from reading\n")?;
    app.update(Action::ShowHelp);
    app.update(Action::Back);
    assert!(matches!(app.view(), View::Reader { .. }));

    // From Themes, including unwinding back through the overlay stack.
    app.update(Action::ShowThemes);
    assert!(matches!(app.view(), View::ThemeSelection { .. }));
    app.update(Action::ShowHelp);
    assert!(matches!(app.view(), View::Help { .. }));
    app.update(Action::Back);
    assert!(matches!(app.view(), View::ThemeSelection { .. }));
    app.update(Action::Back);
    assert!(matches!(app.view(), View::Reader { .. }));

    // Help over help stays a single overlay.
    let mut app = reader_app("idempotent help\n")?;
    app.update(Action::ShowHelp);
    app.update(Action::ShowHelp);
    assert!(matches!(app.view(), View::Help { .. }));

    // Every registered binding appears once help is open.
    let rendered = draw(&mut app, 80, 40)?;
    let text = contents(&rendered);
    for label in [
        "q", "Ctrl-C", "F1", "?", "Esc", "Down", "j", "Up", "k", "PgDn", "Ctrl-F", "PgUp",
        "Ctrl-B", "Home", "gg", "End", "G", "{", "}", "p", "c", "t", "Enter",
    ] {
        assert!(text.contains(label), "help shows {label}");
    }
    Ok(())
}

#[test]
fn nav_009_help_round_trip_preserves_the_passage_anchor() -> Result<()> {
    let book_text = "round trip passage with several readable words\n".repeat(40);
    for (width, height) in [(80_u16, 24_u16), (40, 10)] {
        let mut app = reader_app(&book_text)?;
        app.set_content_viewport(width.saturating_sub(8), height.saturating_sub(2));
        app.update(Action::DocumentEnd);
        let anchor_before = app.reader().context("reader")?.anchor();

        app.update(Action::ShowHelp);
        let overlay = draw(&mut app, width, height)?;
        assert!(contents(&overlay).contains("Reader commands"));

        app.update(Action::Back);
        assert_eq!(
            app.reader().context("reader after return")?.anchor(),
            anchor_before,
            "{width}x{height}: help return restores the anchor"
        );
        let restored = draw(&mut app, width, height)?;
        assert!(matches!(app.view(), View::Reader { .. }));
        assert!(contents(&restored).contains("PAGED"));
        assert!(!contents(&restored).contains("Reader commands"));
    }
    Ok(())
}

#[test]
fn lay_007_theme_mode_changes_keep_the_anchor_and_reuse_the_layout_cache() -> Result<()> {
    let long_passage = "cache behaviour passage\n".repeat(10);
    let mut app = reader_app(&long_passage)?;
    let _ = draw(&mut app, 80, 24)?; // Warm the cache for the reported width.

    let (content_width, _) = app.content_viewport();
    let anchor_before = app.reader().context("reader")?.anchor();
    assert!(
        app.reader()
            .context("reader")?
            .cached_layout(content_width)
            .is_some(),
        "drawing warms the width-keyed cache"
    );

    // Theme switches and mode switches keep both anchor and cache.
    app.update(Action::SetModeContinuous);
    assert_eq!(
        app.reader().context("reader")?.mode(),
        termleaf::reader::Mode::Continuous
    );
    assert!(
        app.reader()
            .context("reader")?
            .cached_layout(content_width)
            .is_some()
    );
    app.update(Action::ShowThemes);
    app.update(Action::Confirm);
    assert_eq!(app.reader().context("reader")?.anchor(), anchor_before);
    assert!(
        app.reader()
            .context("reader")?
            .cached_layout(content_width)
            .is_some()
    );

    // A resize invalidates by the width key only.
    let _ = draw(&mut app, 48, 16)?; // Narrow class drops all Paper chrome.
    let (narrow_width, _) = app.content_viewport();
    assert_ne!(narrow_width, content_width, "the resize changes geometry");
    assert!(
        app.reader()
            .context("reader")?
            .cached_layout(content_width)
            .is_none()
    );
    assert!(
        app.reader()
            .context("reader")?
            .cached_layout(narrow_width)
            .is_some()
    );
    assert_eq!(app.reader().context("reader")?.anchor(), anchor_before);
    Ok(())
}

#[test]
fn theme_001_all_five_themes_render_readable_passages() -> Result<()> {
    let mut app = reader_app("theme sweep passage\n")?;
    app.set_color_mode(ColorMode::TrueColor);

    let mut seen = Vec::new();
    for _ in 0..ThemeName::ALL.len() {
        seen.push(app.theme());
        let rendered = draw(&mut app, 60, 16)?;
        assert!(contents(&rendered).contains("theme sweep passage"));
        assert!(contents(&rendered).contains("PAGED"));
        app.update(Action::ShowThemes);
        app.update(Action::NextLine);
        app.update(Action::Confirm);
    }
    assert_eq!(seen.len(), ThemeName::ALL.len());
    Ok(())
}
