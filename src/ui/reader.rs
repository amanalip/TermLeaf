//! Reader viewport rendering: Paper chrome, responsive classes, and rows.
//!
//! The renderer is a pure projection of reader state plus one layout: it
//! never mutates the anchor, and every visible row comes from the shared
//! layout engine so widths and source mapping stay consistent.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::{
    app::{App, MINIMUM_WIDTH},
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
pub fn render(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
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

    let Some(rows) = app
        .reader_mut()
        .map(|session| session.plan_rows(content_width, content_height))
    else {
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
                    .map(|text| Span::styled(text, body))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines).style(theme.style(Role::Surface));
    frame.render_widget(paragraph, content);
}

fn active_theme(app: &App) -> Theme {
    if app.no_color() {
        Theme::no_color()
    } else {
        Theme::named(app.theme())
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
