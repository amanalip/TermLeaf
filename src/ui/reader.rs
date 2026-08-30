//! Reader viewport rendering: Paper chrome, responsive classes, and rows.
//!
//! The renderer is a pure projection of reader state plus one layout: it
//! never mutates the anchor, and every visible row comes from the shared
//! layout engine so widths and source mapping stay consistent.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::{
    app::{App, ImageOverlay, ImageVisual, MINIMUM_WIDTH},
    document::InlineKind,
    layout::{viewport::RowCell, visible_text},
    terminal_image::{CellColor, NativeFramePlan, NativePlacement, Rgb},
    ui::status::{WidthClass, classify},
    ui::theme::{Role, Theme, ThemeName},
};

/// Chrome dimensions for one width class.
struct Chrome {
    margin: u16,
    padding: u16,
    border: bool,
}

const fn chrome(class: WidthClass, paper: bool) -> Chrome {
    if !paper {
        return Chrome {
            margin: 0,
            padding: 0,
            border: false,
        };
    }
    match class {
        // Collapse order: outer canvas first, then page padding, then the
        // page boundary last.
        WidthClass::Wide => Chrome {
            margin: 4,
            padding: 2,
            border: true,
        },
        WidthClass::Standard => Chrome {
            margin: 2,
            padding: 1,
            border: true,
        },
        WidthClass::Compact => Chrome {
            margin: 1,
            padding: 1,
            border: true,
        },
        WidthClass::Narrow => Chrome {
            margin: 0,
            padding: 0,
            border: false,
        },
    }
}

/// Renders the reading viewport into `area` and reports the content size.
pub fn render(frame: &mut Frame<'_>, area: Rect, app: &mut App, native: &mut NativeFramePlan) {
    let theme = active_theme(app);
    let paper = theme.name() == ThemeName::Paper;
    let chrome = chrome(classify(area.width), paper);

    let outer = shrink(area, chrome.margin);
    let (block, content) = if chrome.border {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.style(Role::Accent))
            .style(theme.style(Role::Surface))
            .padding(Padding::new(
                chrome.padding,
                chrome.padding,
                chrome.padding,
                chrome.padding,
            ));
        let inner = block.inner(outer);
        (Some(block), inner)
    } else {
        (None, outer)
    };

    let content_width = content.width.max(MINIMUM_WIDTH);
    let content_height = content.height;
    app.set_content_viewport(content_width, content_height);

    let backend = app.image_backend();
    let cell_pixels = app.cell_pixel_size();
    let background = image_background(&theme);
    let Some((rows, overlays)) = app.reader_mut().map(|session| {
        let rows = session.plan_rows(content_width, content_height);
        let overlays = session.prepare_visible_images(
            content_width,
            content_height,
            backend,
            background,
            cell_pixels,
        );
        (rows, overlays)
    }) else {
        return;
    };

    if let Some(block) = block {
        frame.render_widget(block, outer);
    }

    let body = body_style(&theme);
    let lines = rows
        .into_iter()
        .map(|row| {
            Line::from(
                row.into_iter()
                    .map(|cell: RowCell| {
                        Span::styled(cell.text, decorated(body, &theme, cell.decoration))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines).style(theme.style(Role::Surface));
    frame.render_widget(paragraph, content);
    for overlay in overlays {
        render_image_overlay(frame, content, overlay, &theme, native);
    }
}

fn render_image_overlay(
    frame: &mut Frame<'_>,
    content: Rect,
    overlay: ImageOverlay,
    theme: &Theme,
    native: &mut NativeFramePlan,
) {
    let y = content.y.saturating_add(overlay.row);
    if y >= content.bottom() {
        return;
    }
    match overlay.visual {
        ImageVisual::Loading(caption) => frame.render_widget(
            Paragraph::new(visible_text(&caption, 0)).style(theme.style(Role::Secondary)),
            Rect::new(content.x, y, content.width, 1),
        ),
        ImageVisual::Failed(caption) => frame.render_widget(
            Paragraph::new(visible_text(&caption, 0)).style(theme.style(Role::Warning)),
            Rect::new(content.x, y, content.width, 1),
        ),
        ImageVisual::ReadyCells(image) => {
            for (row, cells) in image.cells().chunks(usize::from(image.width())).enumerate() {
                let Ok(row) = u16::try_from(row) else {
                    break;
                };
                let cell_y = y.saturating_add(row);
                if cell_y >= content.bottom() {
                    break;
                }
                for (column, cell) in cells.iter().enumerate() {
                    let Ok(column) = u16::try_from(column) else {
                        break;
                    };
                    let cell_x = content.x.saturating_add(column);
                    if cell_x >= content.right() {
                        break;
                    }
                    frame.buffer_mut()[(cell_x, cell_y)]
                        .set_symbol("\u{2580}")
                        .set_fg(cell_color(cell.foreground))
                        .set_bg(cell_color(cell.background));
                }
            }
        }
        ImageVisual::Native { image, .. }
            if y.saturating_add(image.rows()) <= content.bottom()
                && content.x.saturating_add(image.columns()) <= content.right() =>
        {
            native.push(NativePlacement {
                column: content.x,
                row: y,
                image,
            });
        }
        ImageVisual::Native { caption, .. } => frame.render_widget(
            Paragraph::new(format!("(partially outside viewport) {caption}"))
                .style(theme.style(Role::Warning)),
            Rect::new(content.x, y, content.width, 1),
        ),
    }
}

const fn cell_color(color: CellColor) -> Color {
    match color {
        CellColor::Rgb(Rgb(red, green, blue)) => Color::Rgb(red, green, blue),
        CellColor::Indexed(index) => Color::Indexed(index),
    }
}

fn image_background(theme: &Theme) -> Rgb {
    match theme.style(Role::Surface).bg {
        Some(Color::Rgb(red, green, blue)) => Rgb(red, green, blue),
        Some(Color::Indexed(index)) => xterm_rgb(index),
        _ if matches!(theme.name(), ThemeName::Light | ThemeName::Paper) => Rgb(255, 255, 255),
        _ => Rgb(0, 0, 0),
    }
}

fn xterm_rgb(index: u8) -> Rgb {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        16..=231 => {
            let value = index - 16;
            Rgb(
                LEVELS[usize::from(value / 36)],
                LEVELS[usize::from(value % 36 / 6)],
                LEVELS[usize::from(value % 6)],
            )
        }
        232..=255 => {
            let level = 8 + (index - 232) * 10;
            Rgb(level, level, level)
        }
        _ => Rgb(0, 0, 0),
    }
}

/// Combines the body style with one inline role.
///
/// Attributes carry meaning even where color does not, so every decoration
/// keeps a distinct modifier and `NO_COLOR` sessions still differentiate
/// roles without any foreground or background values.
fn decorated(base: Style, theme: &Theme, decoration: Option<InlineKind>) -> Style {
    match decoration {
        None => base,
        Some(InlineKind::Emphasis) => base.italic(),
        Some(InlineKind::Strong) => base.bold(),
        Some(InlineKind::Code) => base
            .fg(theme.style(Role::Secondary).fg.unwrap_or_default())
            .bold(),
        Some(InlineKind::Link) => base
            .fg(theme.style(Role::Link).fg.unwrap_or_default())
            .underlined(),
    }
}

fn active_theme(app: &App) -> Theme {
    if app.no_color() {
        Theme::no_color()
    } else {
        Theme::for_output(app.theme(), app.color_mode())
    }
}

fn shrink(area: Rect, by: u16) -> Rect {
    let horizontal = by.saturating_mul(2);
    Rect {
        x: area.x + by.min(area.width / 2),
        y: area.y + by.min(area.height / 2),
        width: area.width.saturating_sub(horizontal),
        height: area.height.saturating_sub(horizontal),
    }
}

/// The text style for body rows given a theme.
#[must_use]
pub fn body_style(theme: &Theme) -> Style {
    let surface = theme.style(Role::Surface);
    let text = theme.style(Role::Text);
    match surface.bg {
        Some(background) => text.bg(background),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn img_012_caption_controls_are_sanitized_before_ratatui_buffer_write() {
        let backend = TestBackend::new(60, 2);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let mut native = NativeFramePlan::default();
                render_image_overlay(
                    frame,
                    Rect::new(0, 0, 60, 1),
                    ImageOverlay {
                        row: 0,
                        visual: ImageVisual::Failed(
                            "[image: bad \u{1b}[2J \u{1} plate; decode failed]".to_owned(),
                        ),
                    },
                    &Theme::named(ThemeName::Dark),
                    &mut native,
                );
            })
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("^["));
        assert!(text.contains("^A"));
        assert!(
            buffer
                .content
                .iter()
                .all(|cell| !cell.symbol().chars().any(char::is_control))
        );
    }
}
