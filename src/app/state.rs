use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    document::{Document, DocumentError},
    layout::PageLayout,
    reader::{self, Mode},
    ui::status::StatusMessage,
    ui::theme::{ColorMode, ThemeName},
};
use anyhow::{Context, Result, bail};

use super::Action;
use crate::document::model::Position;

/// One open book plus its logical reading state.
///
/// The layout cache is keyed by content width; navigation between draws
/// reuses it, and a resize replaces it wholesale. The cache owner is the
/// session itself and the invalidation rule is the width key.
#[derive(Debug)]
pub struct ReaderSession {
    document: Document,
    anchor: Position,
    mode: Mode,
    cached_layout: Option<(u16, PageLayout)>,
}

impl ReaderSession {
    /// Opens a parsed document anchored at its start in paged mode.
    #[must_use]
    pub fn new(document: Document) -> Self {
        let anchor = document.first_position().unwrap_or(Position::ORIGIN);
        Self {
            document,
            anchor,
            mode: Mode::Paged,
            cached_layout: None,
        }
    }

    /// The open document.
    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    /// The validated logical reading anchor.
    #[must_use]
    pub const fn anchor(&self) -> Position {
        self.anchor
    }

    /// The active reading mode.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Sets the reading mode; the anchor never moves on a mode switch.
    pub const fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// The layout for one content width, reusing the cache when possible.
    #[must_use]
    pub fn layout_for(&mut self, width: u16) -> &PageLayout {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, width);
            self.cached_layout = Some((width, layout));
        }
        &self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above")
            .1
    }

    /// The cached layout when it already matches `width`.
    #[must_use]
    pub const fn cached_layout(&self, width: u16) -> Option<&PageLayout> {
        match self.cached_layout.as_ref() {
            Some((cached, layout)) if *cached == width => Some(layout),
            _ => None,
        }
    }

    /// Visible row texts for one content viewport, warming the cache.
    ///
    /// Strings are plain spans in row order; the UI layer styles them.
    #[must_use]
    pub fn plan_rows(&mut self, width: u16, height: u16) -> Vec<Vec<String>> {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, width);
            self.cached_layout = Some((width, layout));
        }
        let cache = self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above");
        let anchor = self.anchor.absolute_byte(&self.document);
        crate::layout::viewport::viewport_row_texts(&self.document, &cache.1, anchor, height)
    }

    /// Applies one navigation step, keeping the previous anchor on failure.
    pub fn navigate<F>(&mut self, content_width: u16, step: F)
    where
        F: FnOnce(&Document, &PageLayout, Position) -> Option<Position>,
    {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != content_width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, content_width);
            self.cached_layout = Some((content_width, layout));
        }
        let cache = self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above");
        if let Some(next) = step(&self.document, &cache.1, self.anchor) {
            self.anchor = next;
        }
    }
}

impl PartialEq for OpenBook {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for OpenBook {}

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
    ThemeSelection { return_to: Box<View> },
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

/// Minimum usable terminal geometry below which the reader suspends.
pub const MINIMUM_WIDTH: u16 = 20;
pub const MINIMUM_HEIGHT: u16 = 4;

#[derive(Debug)]
pub struct App {
    view: View,
    running: bool,
    theme: ThemeName,
    no_color: bool,
    color_mode: ColorMode,
    theme_cursor: usize,
    message: Option<StatusMessage>,
    reader: Option<ReaderSession>,
    content_width: u16,
    content_height: u16,
}

/// Reader launch choices after configuration precedence is applied.
///
/// `book` is the command-line path when one was supplied; `theme` is already
/// resolved (explicit option, then config.toml, then the built-in default).
#[derive(Debug, Default)]
pub struct StartupOptions {
    /// Local book path supplied on the command line, when any.
    pub book: Option<PathBuf>,
    /// Resolved startup theme for the session.
    pub theme: ThemeName,
}

impl App {
    /// Creates the initial application state for a home or local-book launch.
    ///
    /// The book is decoded before terminal initialization so failures reach
    /// the reader as plain diagnostics on an untouched shell. The open file
    /// handle stays held for the session, keeping the source immutable from
    /// the reader's point of view. Color capability is detected from the
    /// launch environment here so every later draw uses one fixed decision.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied book path cannot be inspected, is
    /// not a regular file, exceeds the size limit, or does not decode.
    pub fn open(options: StartupOptions) -> Result<Self> {
        let (view, reader) = match options.book {
            Some(path) => {
                Self::validate_path(&path)?;
                let display = path.display().to_string();
                let limits = crate::document::TextLimits::default();
                let document = crate::document::text::load_text_file(&path, &limits).map_err(
                    |error| -> anyhow::Error {
                        match error {
                            DocumentError::Read { source, .. } => anyhow::Error::new(source)
                                .context(format!("could not read '{display}'")),
                            // Typed document errors already name the path,
                            // reason, and recovery; an extra layer would
                            // only bury them.
                            typed => anyhow::Error::new(typed),
                        }
                    },
                )?;
                let file = File::open(&path)
                    .with_context(|| format!("could not hold '{display}' open"))?;
                let book = OpenBook {
                    path,
                    _file: Arc::new(file),
                };
                (View::Reader { book }, Some(ReaderSession::new(document)))
            }
            None => (View::RecentBooks, None),
        };

        Ok(Self {
            view,
            running: true,
            theme: options.theme,
            no_color: std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
            color_mode: ColorMode::detect(
                env_value("COLORTERM").as_deref(),
                env_value("TERM").as_deref(),
            ),
            theme_cursor: options.theme as usize,
            message: None,
            reader,
            content_width: MINIMUM_WIDTH,
            content_height: MINIMUM_HEIGHT,
        })
    }

    fn validate_path(path: &Path) -> Result<()> {
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
        Ok(())
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
            View::ThemeSelection { .. } => Focus::ThemeOption,
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

    /// The active theme name; `NO_COLOR` sessions still report the choice.
    #[must_use]
    pub const fn theme(&self) -> ThemeName {
        self.theme
    }

    /// Cursor position inside the theme selection list.
    #[must_use]
    pub const fn theme_cursor(&self) -> usize {
        self.theme_cursor
    }

    /// Whether the session must render without colors (`NO_COLOR`).
    #[must_use]
    pub const fn no_color(&self) -> bool {
        self.no_color
    }

    /// The terminal color capability detected at launch.
    #[must_use]
    pub const fn color_mode(&self) -> ColorMode {
        self.color_mode
    }

    /// Overrides the detected color capability so tests can exercise every
    /// fallback rendering deterministically.
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.color_mode = mode;
    }

    /// The reader session, when a book is open.
    #[must_use]
    pub const fn reader(&self) -> Option<&ReaderSession> {
        self.reader.as_ref()
    }

    /// Mutable reader access for the render layer's viewport reporting.
    #[must_use]
    pub fn reader_mut(&mut self) -> Option<&mut ReaderSession> {
        self.reader.as_mut()
    }

    /// The last rendered content viewport, used by navigation between draws.
    #[must_use]
    pub const fn content_viewport(&self) -> (u16, u16) {
        (self.content_width, self.content_height)
    }

    /// Records the content viewport produced by the latest render.
    pub fn set_content_viewport(&mut self, width: u16, height: u16) {
        self.content_width = width;
        self.content_height = height;
    }

    /// Shows a temporary status message replacing lower-priority fields.
    pub fn set_message(&mut self, text: impl Into<String>) {
        self.message = Some(StatusMessage::new(text));
    }

    #[must_use]
    pub const fn message(&self) -> Option<&StatusMessage> {
        self.message.as_ref()
    }

    /// Applies one action to the application state.
    ///
    /// Temporary messages tick once per delivered action, giving them a
    /// deterministic input-driven lifetime.
    pub fn update(&mut self, action: Action) {
        if matches!(self.view, View::ThemeSelection { .. }) {
            self.update_theme_selection(action);
            self.tick_message();
            return;
        }

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
            Action::ShowThemes => {
                self.theme_cursor = self.theme as usize;
                self.view = View::ThemeSelection {
                    return_to: Box::new(self.view.clone()),
                };
            }
            Action::NextLine | Action::PreviousLine if matches!(self.view, View::Reader { .. }) => {
                let direction = match action {
                    Action::NextLine => reader::Direction::TowardEnd,
                    _ => reader::Direction::TowardStart,
                };
                self.step(|document, layout, anchor| {
                    reader::step_line(layout, document, anchor, direction)
                });
            }
            Action::NextPage | Action::PreviousPage if matches!(self.view, View::Reader { .. }) => {
                let direction = match action {
                    Action::NextPage => reader::Direction::TowardEnd,
                    _ => reader::Direction::TowardStart,
                };
                let rows = usize::from(self.content_height);
                self.step(|document, layout, anchor| {
                    reader::step_page(layout, document, anchor, rows, direction)
                });
            }
            Action::DocumentStart if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, layout, _| reader::jump_document_start(layout, document));
            }
            Action::DocumentEnd if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, _, _| reader::jump_document_end(document));
            }
            Action::SectionStart if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, layout, _| reader::jump_section_start(layout, document, 0));
            }
            Action::SectionEnd if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, layout, _| reader::jump_section_end(layout, document, 0));
            }
            Action::SetModePaged | Action::SetModeContinuous
                if matches!(self.view, View::Reader { .. }) =>
            {
                let mode = match action {
                    Action::SetModePaged => Mode::Paged,
                    _ => Mode::Continuous,
                };
                self.set_mode(mode);
            }
            // Reader actions outside the Reader view are intentionally
            // inert, and Confirm has no global meaning yet: overlays such as
            // help must never move the hidden reading anchor.
            _ => {}
        }

        self.tick_message();
    }

    fn update_theme_selection(&mut self, action: Action) {
        let return_to = match &self.view {
            View::ThemeSelection { return_to } => (**return_to).clone(),
            _ => View::RecentBooks,
        };
        match action {
            Action::NextLine => {
                self.theme_cursor = (self.theme_cursor + 1) % ThemeName::ALL.len();
            }
            Action::PreviousLine => {
                self.theme_cursor =
                    (self.theme_cursor + ThemeName::ALL.len() - 1) % ThemeName::ALL.len();
            }
            Action::Confirm => {
                self.theme = ThemeName::ALL[self.theme_cursor];
                self.view = return_to;
                let label = self.theme.label();
                self.set_message(format!("Theme: {label}"));
            }
            Action::Quit | Action::Back | Action::ShowThemes => {
                self.view = return_to;
            }
            _ => {}
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        let Some(session) = self.reader.as_mut() else {
            return;
        };
        if session.mode() != mode {
            session.set_mode(mode);
            self.set_message(match mode {
                Mode::Paged => "Paged mode",
                Mode::Continuous => "Continuous mode",
            });
        }
    }

    fn step<F>(&mut self, movement: F)
    where
        F: FnOnce(&Document, &PageLayout, Position) -> Option<Position>,
    {
        if self.reader.is_some() {
            let (width, _) = self.content_viewport();
            if let Some(session) = self.reader.as_mut() {
                session.navigate(width, movement);
            }
        }
    }

    fn tick_message(&mut self) {
        if let Some(message) = self.message.as_mut()
            && message.tick()
        {
            self.message = None;
        }
    }
}

fn env_value(key: &str) -> Option<String> {
    std::env::var_os(key).map(|value| value.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_view(view: View) -> App {
        App {
            view,
            running: true,
            theme: ThemeName::Paper,
            no_color: false,
            color_mode: ColorMode::TrueColor,
            theme_cursor: ThemeName::Paper as usize,
            message: None,
            reader: None,
            content_width: MINIMUM_WIDTH,
            content_height: MINIMUM_HEIGHT,
        }
    }

    #[test]
    fn app_002_help_returns_to_its_invoking_view_and_focus() -> Result<()> {
        let mut app = App::open(StartupOptions::default())?;

        app.update(Action::ShowHelp);
        assert_eq!(app.focus(), Focus::Help);
        app.update(Action::Back);

        assert_eq!(app.view(), &View::RecentBooks);
        assert_eq!(app.focus(), Focus::RecentBooks);
        Ok(())
    }

    #[test]
    fn app_002_quit_stops_the_state_loop() -> Result<()> {
        let mut app = App::open(StartupOptions::default())?;

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
            View::ThemeSelection {
                return_to: Box::new(View::RecentBooks),
            },
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
            let app = app_with_view(view);
            assert_eq!(app.focus(), expected_focus);
        }

        let file = tempfile::NamedTempFile::new().expect("create reader focus fixture");
        let mut reader = App::open(StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..StartupOptions::default()
        })
        .expect("open reader fixture");
        assert!(matches!(reader.view(), View::Reader { .. }));
        assert_eq!(reader.focus(), Focus::ReadingAnchor);
        reader.update(Action::ShowHelp);
        assert_eq!(reader.focus(), Focus::Help);
    }
}
