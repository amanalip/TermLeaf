use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};

use super::Action;

#[derive(Clone, Debug)]
pub struct OpenBook {
    path: PathBuf,
    _file: Arc<File>,
}

impl OpenBook {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl PartialEq for OpenBook {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for OpenBook {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum View {
    RecentBooks,
    OpenPath,
    Reader { book: OpenBook },
    LinkFocus,
    TextSelection,
    SearchEntry,
    SearchHistory,
    SearchResults,
    TableOfContents,
    AnnotationList,
    BookmarkDialog,
    HighlightDialog,
    NoteEditor,
    ThemeSelection,
    LinkConfirmation,
    Help { return_to: Box<View> },
    RecoverableError,
    TooSmall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    RecentBooks,
    PathField,
    ReadingAnchor,
    Link,
    SelectionEndpoint,
    SearchField,
    SearchHistoryItem,
    SearchResult,
    TableOfContentsItem,
    AnnotationItem,
    BookmarkNameField,
    HighlightColor,
    NoteField,
    ThemeOption,
    ConfirmationAction,
    Help,
    RecoveryAction,
    SuspendedView,
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
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    if metadata.permissions().mode() & 0o444 == 0 {
                        bail!(
                            "could not open book '{}'; the file is not readable",
                            path.display()
                        );
                    }
                }
                let file = File::open(&path).with_context(|| {
                    format!(
                        "could not open book '{}'; check that the file is readable",
                        path.display()
                    )
                })?;
                View::Reader {
                    book: OpenBook {
                        path,
                        _file: Arc::new(file),
                    },
                }
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
            View::OpenPath => Focus::PathField,
            View::Reader { .. } => Focus::ReadingAnchor,
            View::LinkFocus => Focus::Link,
            View::TextSelection => Focus::SelectionEndpoint,
            View::SearchEntry => Focus::SearchField,
            View::SearchHistory => Focus::SearchHistoryItem,
            View::SearchResults => Focus::SearchResult,
            View::TableOfContents => Focus::TableOfContentsItem,
            View::AnnotationList => Focus::AnnotationItem,
            View::BookmarkDialog => Focus::BookmarkNameField,
            View::HighlightDialog => Focus::HighlightColor,
            View::NoteEditor => Focus::NoteField,
            View::ThemeSelection => Focus::ThemeOption,
            View::LinkConfirmation => Focus::ConfirmationAction,
            View::Help { .. } => Focus::Help,
            View::RecoverableError => Focus::RecoveryAction,
            View::TooSmall => Focus::SuspendedView,
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
    fn app_002_help_returns_to_its_invoking_view_and_focus() -> Result<()> {
        let mut app = App::new(None)?;

        app.update(Action::ShowHelp);
        assert_eq!(app.focus(), Focus::Help);
        app.update(Action::Back);

        assert_eq!(app.view(), &View::RecentBooks);
        assert_eq!(app.focus(), Focus::RecentBooks);
        Ok(())
    }

    #[test]
    fn app_002_quit_stops_the_state_loop() -> Result<()> {
        let mut app = App::new(None)?;

        app.update(Action::Quit);

        assert!(!app.is_running());
        Ok(())
    }

    #[test]
    fn app_001_each_view_owns_exactly_one_focus_kind() {
        let views = [
            View::RecentBooks,
            View::OpenPath,
            View::LinkFocus,
            View::TextSelection,
            View::SearchEntry,
            View::SearchHistory,
            View::SearchResults,
            View::TableOfContents,
            View::AnnotationList,
            View::BookmarkDialog,
            View::HighlightDialog,
            View::NoteEditor,
            View::ThemeSelection,
            View::LinkConfirmation,
            View::RecoverableError,
            View::TooSmall,
        ];
        let expected = [
            Focus::RecentBooks,
            Focus::PathField,
            Focus::Link,
            Focus::SelectionEndpoint,
            Focus::SearchField,
            Focus::SearchHistoryItem,
            Focus::SearchResult,
            Focus::TableOfContentsItem,
            Focus::AnnotationItem,
            Focus::BookmarkNameField,
            Focus::HighlightColor,
            Focus::NoteField,
            Focus::ThemeOption,
            Focus::ConfirmationAction,
            Focus::RecoveryAction,
            Focus::SuspendedView,
        ];

        for (view, expected_focus) in views.into_iter().zip(expected) {
            let app = App {
                view,
                running: true,
            };
            assert_eq!(app.focus(), expected_focus);
        }

        let file = tempfile::NamedTempFile::new().expect("create reader focus fixture");
        let mut reader = App::new(Some(file.path().to_path_buf())).expect("open reader fixture");
        assert_eq!(reader.focus(), Focus::ReadingAnchor);
        reader.update(Action::ShowHelp);
        assert_eq!(reader.focus(), Focus::Help);
    }
}
