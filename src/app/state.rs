use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use super::Action;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum View {
    RecentBooks,
    Reader { path: PathBuf },
    Help { return_to: Box<View> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    RecentBooks,
    ReadingAnchor,
    Help,
}

#[derive(Debug)]
pub struct App {
    view: View,
    running: bool,
}

impl App {
    /// Creates the initial application state for a home or local-book launch.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied book path cannot be inspected or is
    /// not a regular file. Validation happens before terminal initialization.
    pub fn new(book: Option<PathBuf>) -> Result<Self> {
        let view = match book {
            Some(path) => {
                let metadata = path.metadata().with_context(|| {
                    format!(
                        "could not open book '{}'; check that the path exists and is readable",
                        path.display()
                    )
                })?;
                if !metadata.is_file() {
                    bail!(
                        "could not open book '{}'; the path is not a file",
                        path.display()
                    );
                }
                View::Reader { path }
            }
            None => View::RecentBooks,
        };

        Ok(Self {
            view,
            running: true,
        })
    }

    #[must_use]
    pub const fn view(&self) -> &View {
        &self.view
    }

    #[must_use]
    pub const fn focus(&self) -> Focus {
        match self.view {
            View::RecentBooks => Focus::RecentBooks,
            View::Reader { .. } => Focus::ReadingAnchor,
            View::Help { .. } => Focus::Help,
        }
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::ShowHelp if !matches!(self.view, View::Help { .. }) => {
                self.view = View::Help {
                    return_to: Box::new(self.view.clone()),
                };
            }
            Action::Back => {
                let current = std::mem::replace(&mut self.view, View::RecentBooks);
                self.view = match current {
                    View::Help { return_to } => *return_to,
                    other => other,
                };
            }
            Action::ShowHelp => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_returns_to_its_invoking_view_and_focus() -> Result<()> {
        let mut app = App::new(None)?;

        app.update(Action::ShowHelp);
        assert_eq!(app.focus(), Focus::Help);
        app.update(Action::Back);

        assert_eq!(app.view(), &View::RecentBooks);
        assert_eq!(app.focus(), Focus::RecentBooks);
        Ok(())
    }

    #[test]
    fn quit_stops_the_state_loop() -> Result<()> {
        let mut app = App::new(None)?;

        app.update(Action::Quit);

        assert!(!app.is_running());
        Ok(())
    }
}
