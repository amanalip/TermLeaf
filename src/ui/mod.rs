//! Terminal rendering: a pure projection of application state.
//!
//! `render` dispatches by view, paints the reader viewport and status line,
//! and reports the content viewport back to the application so navigation
//! between draws uses the same geometry the reader displayed.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    app::{App, MINIMUM_HEIGHT, MINIMUM_WIDTH, View, bindings},
    clock, reader as reading,
    reader::Mode,
    terminal_image::NativeFramePlan,
};

pub mod reader;
pub mod status;
pub mod theme;

use status::{PagePosition, StatusModel, format_status};
use theme::{Role, Theme, ThemeName};

/// Renders one frame of the application.
pub fn render(frame: &mut Frame<'_>, app: &mut App) {
    render_with_native(frame, app, &mut NativeFramePlan::default());
}

/// Renders one frame and collects native image placements as a side channel.
pub fn render_with_native(frame: &mut Frame<'_>, app: &mut App, native: &mut NativeFramePlan) {
    let area = frame.area();

    if area.width < MINIMUM_WIDTH || area.height < MINIMUM_HEIGHT {
        render_too_small(frame, area);
        return;
    }

    let [body, status] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .areas(area);

    match app.view() {
        View::Reader { .. } => reader::render(frame, body, app, native),
        View::RecentBooks => render_recent_books(frame, body),
        View::ThemeSelection { .. } => render_theme_selection(frame, body, app),
        View::TableOfContents { .. } => render_toc(frame, body, app, native),
        View::Help { .. } => render_help(frame, body),
        view => render_future_view(frame, body, view),
    }

    render_status(frame, status, app);
}

fn render_toc(frame: &mut Frame<'_>, area: Rect, app: &mut App, native: &mut NativeFramePlan) {
    if area.width >= 120 {
        let [passage, panel] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(70), Constraint::Length(38)])
            .spacing(2)
            .areas(area);
        reader::render(frame, passage, app, native);
        render_toc_selection(frame, panel, app);
    } else {
        render_toc_selection(frame, area, app);
    }
}

fn active_theme(app: &App) -> Theme {
    if app.no_color() {
        Theme::no_color()
    } else {
        Theme::for_output(app.theme(), app.color_mode())
    }
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let line = if let Some(session) = app.reader() {
        let document = session.document();
        let anchor = session.anchor();
        let (content_width, content_height) = app.content_viewport();
        let page = match (session.mode(), content_height) {
            (Mode::Paged, height) if height > 0 => {
                session.cached_layout(content_width).map(|layout| {
                    let total = layout.rows().len().div_ceil(usize::from(height));
                    let current = layout
                        .row_after(anchor.absolute_byte(document))
                        .div_ceil(usize::from(height))
                        .clamp(1, total.max(1));
                    PagePosition { current, total }
                })
            }
            _ => None,
        };
        let clock_label = clock::now_label();
        let model = StatusModel {
            title: document.title(),
            chapter: None,
            location_line: reading::progress::location_line(document, anchor),
            percent: reading::progress::percent(document, anchor),
            page,
            mode_label: session.mode().label(),
            clock: &clock_label,
            message: None,
        };
        format_status(&model, area.width)
    } else {
        match app.message() {
            // Temporary confirmations stay visible on every screen, not
            // only while a book is open.
            Some(message) => format!(" {}", message.text()),
            None => " F1/? Help  q Quit".to_owned(),
        }
    };

    frame.render_widget(
        Paragraph::new(line).style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
    );
}

fn render_theme_selection(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let theme = active_theme(app);
    let cursor = app.theme_cursor();
    let lines: Vec<Line<'static>> = ThemeName::ALL
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let marker = if index == cursor { "> " } else { "  " };
            let applied = if *name == app.theme() {
                "  (applied)"
            } else {
                ""
            };
            let style = if index == cursor {
                theme.style(Role::Accent)
            } else {
                theme.style(Role::Text)
            };
            Line::from(format!("{marker}{}{applied}", name.label())).style(style)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center).block(
            Block::default()
                .title(" Themes ")
                .borders(Borders::ALL)
                .border_style(theme.style(Role::Accent)),
        ),
        area,
    );
}

/// Renders the table of contents overlay with a scrolling section list.
///
/// The selected entry leads the window so long books keep the cursor
/// visible; untitled sections fall back to a stable ordinal label.
fn render_toc_selection(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let theme = active_theme(app);
    let cursor = app.toc_cursor();
    let Some(session) = app.reader() else {
        return;
    };
    let document = session.document();
    let sections = document.navigation_points();
    let visible_height = area.height.saturating_sub(2).max(1) as usize;
    let top = (cursor + 1).saturating_sub(visible_height);

    let lines: Vec<Line<'static>> = sections
        .iter()
        .enumerate()
        .skip(top)
        .take(visible_height)
        .map(|(index, section)| {
            let marker = if index == cursor { "> " } else { "  " };
            let title = section.title().to_owned();
            let style = if index == cursor {
                theme.style(Role::Accent)
            } else {
                theme.style(Role::Text)
            };
            Line::from(format!("{marker}{title}")).style(style)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Table of contents ")
                .borders(Borders::ALL)
                .border_style(theme.style(Role::Accent)),
        ),
        area,
    );
}

fn render_too_small(frame: &mut Frame<'_>, area: Rect) {
    let _ = area;
    // Short, complete lines: clipping stays safe at any remaining size.
    let lines = vec![
        Line::from("Terminal too small").style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("Resize or press q."),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_future_view(frame: &mut Frame<'_>, area: Rect, view: &View) {
    let name = match view {
        View::OpenPath => "Open path",
        View::LinkFocus => "Link focus",
        View::TextSelection => "Text selection",
        View::SearchEntry => "Search",
        View::SearchHistory => "Search history",
        View::SearchResults => "Search results",
        View::TableOfContents { .. } => "Table of contents",
        View::AnnotationList => "Annotations",
        View::BookmarkDialog => "Bookmark",
        View::HighlightDialog => "Highlight",
        View::NoteEditor => "Note editor",
        View::LinkConfirmation => "Open link",
        View::RecoverableError => "Error",
        _ => return,
    };
    frame.render_widget(
        Paragraph::new("This view is implemented in its assigned delivery phase.")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(format!(" {name} "))
                    .borders(Borders::ALL),
            ),
        area,
    );
}

fn render_recent_books(frame: &mut Frame<'_>, area: Rect) {
    let text = Text::from(vec![
        Line::from("TermLeaf").style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("No recent books yet."),
        Line::from("Open a local book by passing its path on the command line."),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Recent books ")
                    .borders(Borders::ALL),
            ),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let lines = std::iter::once(Line::from("Reader commands"))
        .chain(
            bindings()
                .iter()
                .map(|binding| Line::from(format!("{:<6} {}", binding.label, binding.description))),
        )
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().title(" Help ").borders(Borders::ALL)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::app::Action;

    use super::*;

    fn book_app(contents: &str) -> Result<App> {
        use std::io::Write;

        let mut file = tempfile::Builder::new()
            .prefix("book")
            .suffix(".txt")
            .tempfile()?;
        writeln!(file, "{contents}")?;
        App::open(crate::app::StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..crate::app::StartupOptions::default()
        })
    }

    fn draw(app: &mut App, width: u16, height: u16) -> Result<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend)?;
        terminal.draw(|frame| render(frame, app))?;
        Ok(terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect())
    }

    #[test]
    fn app_003_base_shell_renders_deterministically() -> Result<()> {
        let mut terminal = Terminal::new(TestBackend::new(40, 10))?;
        let mut app = App::open(crate::app::StartupOptions::default())?;

        terminal.draw(|frame| render(frame, &mut app))?;

        let first_render = terminal.backend().buffer().clone();
        app.update(Action::ShowHelp);
        terminal.draw(|frame| render(frame, &mut app))?;
        app.update(Action::Back);
        terminal.draw(|frame| render(frame, &mut app))?;
        assert_eq!(&first_render, terminal.backend().buffer());
        let rendered = first_render
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(rendered.contains("Recent books"));
        assert!(rendered.contains("No recent books yet."));
        assert!(rendered.contains("F1/? Help  q Quit"));
        assert_eq!(first_render[(16, 1)].symbol(), "T");
        assert_eq!(first_render[(10, 3)].symbol(), "N");
        assert_eq!(first_render[(1, 9)].symbol(), "F");
        Ok(())
    }

    #[test]
    fn render_001_reader_shows_passage_status_and_mode_at_standard_size() -> Result<()> {
        let mut app = book_app("The quick brown fox jumps over the lazy dog.\n")?;
        assert!(matches!(app.view(), View::Reader { .. }));

        let rendered = draw(&mut app, 80, 24)?;

        assert!(rendered.contains("The quick brown fox"));
        assert!(rendered.contains("PAGED"));
        assert!(rendered.contains("0%"));
        assert!(rendered.contains("Loc 1"));
        Ok(())
    }

    #[test]
    fn lay_012_below_minimum_size_shows_the_safe_message_then_recovers() -> Result<()> {
        let mut app = book_app("passage\n")?;

        let small = draw(&mut app, 24, 3)?;
        assert!(small.contains("Terminal too small"), "{small}");

        let recovered = draw(&mut app, 80, 24)?;
        assert!(recovered.contains("passage"));
        assert!(!recovered.contains("Terminal too small"));
        Ok(())
    }

    #[test]
    fn nav_006_mode_switch_updates_the_status_label_in_place() -> Result<()> {
        let mut app = book_app("alpha\n\nbeta\n")?;
        draw(&mut app, 80, 24)?;

        app.update(Action::SetModeContinuous);
        let rendered = draw(&mut app, 80, 24)?;
        assert!(rendered.contains("CONT"));
        assert!(!rendered.contains("PAGED"));

        app.update(Action::SetModePaged);
        let rendered = draw(&mut app, 80, 24)?;
        assert!(rendered.contains("PAGED"));
        Ok(())
    }

    #[test]
    fn nav_009_wide_toc_keeps_the_passage_visible_beside_the_panel() -> Result<()> {
        let mut app = book_app("visible passage\n")?;
        app.update(Action::ShowToc);

        let wide = draw(&mut app, 120, 40)?;
        assert!(wide.contains("visible passage"));
        assert!(wide.contains("Table of contents"));

        let standard = draw(&mut app, 80, 24)?;
        assert!(!standard.contains("visible passage"));
        assert!(standard.contains("Table of contents"));
        Ok(())
    }

    #[test]
    fn lay_006_resize_preserves_logical_location_and_progress() -> Result<()> {
        fn status_row(app: &mut App, width: u16, height: u16) -> Result<(String, String)> {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend)?;
            terminal.draw(|frame| render(frame, app))?;
            let y = height - 1;
            let row: String = (0..width)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect();
            let percent = row
                .split_whitespace()
                .find(|token| token.ends_with('%'))
                .unwrap_or_default()
                .to_owned();
            let loc = row.split("Loc ").nth(1).map_or_else(String::new, |tail| {
                tail.chars().take_while(char::is_ascii_digit).collect()
            });
            Ok((percent, loc))
        }

        let contents = "first paragraph with several words to wrap around\n".repeat(12);
        let mut app = book_app(&contents)?;
        app.set_content_viewport(70, 20);
        app.update(Action::DocumentEnd);

        let (wide_percent, wide_loc) = status_row(&mut app, 90, 24)?;
        let (narrow_percent, narrow_loc) = status_row(&mut app, 40, 12)?;
        let (again_percent, again_loc) = status_row(&mut app, 90, 24)?;

        assert_eq!(wide_percent, "100%");
        assert_eq!(wide_percent, narrow_percent);
        assert_eq!(wide_percent, again_percent);
        assert_eq!(wide_loc, narrow_loc);
        assert_eq!(wide_loc, again_loc);
        Ok(())
    }

    #[test]
    fn theme_002_session_selection_applies_and_reports_the_theme() -> Result<()> {
        let mut app = book_app("content\n")?;
        draw(&mut app, 80, 24)?;

        app.update(Action::ShowThemes);
        assert!(matches!(app.view(), View::ThemeSelection { .. }));
        let rendered = draw(&mut app, 80, 24)?;
        assert!(rendered.contains("High contrast"));

        // Paper is the default; one step forward wraps to Dark.
        app.update(Action::NextLine);
        app.update(Action::Confirm);
        assert_eq!(app.theme(), ThemeName::Dark);
        assert!(matches!(app.view(), View::Reader { .. }));
        assert_eq!(
            app.message().map(crate::ui::status::StatusMessage::text),
            Some("Theme: Dark")
        );

        app.update(Action::ShowThemes);
        let rendered = draw(&mut app, 80, 24)?;
        assert!(rendered.contains("(applied)"));
        Ok(())
    }

    #[test]
    fn help_lists_every_registered_reader_binding() -> Result<()> {
        let mut app = book_app("content\n")?;
        app.update(Action::ShowHelp);
        let rendered = draw(&mut app, 80, 40)?;

        for binding in bindings() {
            assert!(
                rendered.contains(binding.label),
                "help shows {}",
                binding.label
            );
        }
        Ok(())
    }
}
