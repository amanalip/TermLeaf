use std::path::Path;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::{App, View, bindings};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let [body, status] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .areas(area);

    match app.view() {
        View::RecentBooks => render_recent_books(frame, body),
        View::Reader { path } => render_reader(frame, body, path),
        View::Help { .. } => render_help(frame, body),
    }

    frame.render_widget(
        Paragraph::new(" F1/? Help  q Quit")
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        status,
    );
}

fn render_recent_books(frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
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

fn render_reader(frame: &mut Frame<'_>, area: ratatui::layout::Rect, path: &Path) {
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Book");
    let text = Text::from(vec![
        Line::from(title).style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("The plain-text reading loop arrives in Phase 1."),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::default().title(" Reader ").borders(Borders::ALL)),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
    let lines = std::iter::once(Line::from("Foundation commands"))
        .chain(
            bindings()
                .iter()
                .map(|binding| Line::from(format!("{:<4} {}", binding.label, binding.description))),
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

    use super::*;

    #[test]
    fn base_shell_renders_deterministically() -> Result<()> {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend)?;
        let app = App::new(None)?;

        terminal.draw(|frame| render(frame, &app))?;

        let buffer = terminal.backend().buffer();
        let rendered = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(rendered.contains("Recent books"));
        assert!(rendered.contains("No recent books yet."));
        assert!(rendered.contains("F1/? Help  q Quit"));
        Ok(())
    }
}
