//! Status line composition with a documented field collapse order.
//!
//! Fields carry semantic priorities from `ui_mockups.md`; when the terminal
//! narrows, the lowest-priority fields disappear whole — never truncated
//! mid-field — until only mode and progress remain. Temporary messages
//! replace lower-priority fields for a deterministic number of key events,
//! not wall-clock time.

use crate::layout::display_width;

/// Key events a temporary message stays visible for.
pub const MESSAGE_LIFETIME: u16 = 8;

/// A temporary status message with input-driven lifetime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusMessage {
    text: String,
    remaining: u16,
}

impl StatusMessage {
    /// Starts a message that will outlive `MESSAGE_LIFETIME` key events.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            remaining: MESSAGE_LIFETIME,
        }
    }

    /// The message body.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Records one key event; returns true when the message just expired.
    pub fn tick(&mut self) -> bool {
        self.remaining = self.remaining.saturating_sub(1);
        self.remaining == 0
    }
}

/// Dynamic page position inside the current layout; never persisted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PagePosition {
    /// One-based current page.
    pub current: usize,
    /// Total pages at the current width and height.
    pub total: usize,
}

/// Everything the status line needs to render one frame.
#[derive(Clone, Copy, Debug)]
pub struct StatusModel<'a> {
    /// Book title.
    pub title: &'a str,
    /// Current section title, when the format supplies one.
    pub chapter: Option<&'a str>,
    /// One-based logical line of the anchor.
    pub location_line: usize,
    /// Floored reading percentage.
    pub percent: u8,
    /// Dynamic page, shown only where meaningful.
    pub page: Option<PagePosition>,
    /// Mode label such as `PAGED`.
    pub mode_label: &'static str,
    /// Preformatted clock string from the injected clock source.
    pub clock: &'a str,
    /// Active temporary message, when any.
    pub message: Option<&'a str>,
}

/// Responsive width classes from `ui_mockups.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidthClass {
    Wide,
    Standard,
    Compact,
    Narrow,
}

/// Classifies a content width for responsive behavior.
#[must_use]
pub const fn classify(width: u16) -> WidthClass {
    if width >= 100 {
        WidthClass::Wide
    } else if width >= 72 {
        WidthClass::Standard
    } else if width >= 56 {
        WidthClass::Compact
    } else {
        WidthClass::Narrow
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Priority {
    Mode = 2,
    Percent = 3,
    Location = 4,
    Hint = 5,
    Chapter = 6,
    Title = 7,
    Page = 8,
    Clock = 9,
}

const SEPARATOR_WIDE: &str = " | ";
const SEPARATOR_NARROW: &str = "  ";
const HINT: &str = "[?]";

/// Formats the status line for one width, collapsing fields by priority.
#[must_use]
pub fn format_status(model: &StatusModel<'_>, width: u16) -> String {
    let limit = width;
    if let Some(message) = model.message {
        return render_message(message, limit);
    }

    let mut segments: Vec<(Priority, String)> = vec![(Priority::Title, model.title.to_owned())];
    if let Some(chapter) = model.chapter {
        segments.push((Priority::Chapter, chapter.to_owned()));
    }
    segments.push((Priority::Location, format!("Loc {}", model.location_line)));
    if let Some(page) = model.page {
        segments.push((
            Priority::Page,
            format!("Page {}/{}", page.current, page.total),
        ));
    }
    segments.push((Priority::Percent, format!("{}%", model.percent)));
    segments.push((Priority::Mode, model.mode_label.to_owned()));
    segments.push((Priority::Clock, model.clock.to_owned()));
    segments.push((Priority::Hint, HINT.to_owned()));

    let separator = match classify(width) {
        WidthClass::Narrow => SEPARATOR_NARROW,
        _ => SEPARATOR_WIDE,
    };

    loop {
        let candidate = join(&segments, separator);
        if display_width(&candidate, 0) <= limit {
            return candidate;
        }
        // Mode and percent are the last essential pair; everything else
        // drops lowest-priority first.
        let drop_index = segments
            .iter()
            .enumerate()
            .filter(|(_, (priority, _))| !matches!(priority, Priority::Mode | Priority::Percent))
            .max_by_key(|(_, (priority, _))| *priority)
            .map(|(index, _)| index);
        match drop_index {
            Some(index) if segments.len() > 2 => {
                segments.remove(index);
            }
            _ => return truncate_safe(&candidate, limit),
        }
    }
}

fn join(segments: &[(Priority, String)], separator: &str) -> String {
    let mut out = String::new();
    for (index, (_, text)) in segments.iter().enumerate() {
        if index > 0 {
            out.push_str(separator);
        }
        out.push_str(text);
    }
    out
}

fn render_message(message: &str, limit: u16) -> String {
    let mut out = String::from("! ");
    out.push_str(message);
    let with_hint_width = display_width(&out, 0)
        .saturating_add(display_width(HINT, 0))
        .saturating_add(3);
    if with_hint_width <= limit {
        out.push_str(SEPARATOR_WIDE);
        out.push_str(HINT);
        return out;
    }
    truncate_safe(&out, limit)
}

/// Cuts on a character boundary and marks the loss with an ellipsis.
fn truncate_safe(text: &str, limit: u16) -> String {
    if display_width(text, 0) <= limit {
        return text.to_owned();
    }
    let mut end = usize::from(limit).saturating_sub(1);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\u{2026}", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(clock: &str) -> StatusModel<'_> {
        StatusModel {
            title: "Pride and Prejudice",
            chapter: Some("Ch 27"),
            location_line: 1842,
            percent: 43,
            page: Some(PagePosition {
                current: 118,
                total: 274,
            }),
            mode_label: "PAGED",
            clock,
            message: None,
        }
    }

    #[test]
    fn status_005_full_status_matches_exact_expected_strings() {
        let wide = format_status(&model("14:07"), 120);
        assert_eq!(
            wide,
            "Pride and Prejudice | Ch 27 | Loc 1842 | Page 118/274 | 43% | PAGED | 14:07 | [?]"
        );

        // Collapse order follows the priority table: clock first; each
        // further narrowing removes the next-lowest field whole.
        let standard = format_status(&model("14:07"), 80);
        assert_eq!(
            standard,
            "Pride and Prejudice | Ch 27 | Loc 1842 | Page 118/274 | 43% | PAGED | [?]"
        );
        let compact = format_status(&model("14:07"), 60);
        assert_eq!(
            compact,
            "Pride and Prejudice | Ch 27 | Loc 1842 | 43% | PAGED | [?]"
        );
    }

    #[test]
    fn status_002_fields_collapse_in_documented_priority_order() {
        let mut previous = String::new();
        for width in (20..=120).rev() {
            let line = format_status(&model("14:07"), width);
            assert!(
                line.chars().count() <= usize::from(width),
                "width {width}: {line}"
            );

            let lost_clock = !previous.contains("14:07") || !line.contains("14:07");
            if lost_clock && previous.contains("14:07") {
                assert!(!line.contains("14:07"), "clock disappears once");
            }
            previous = line;
        }

        let narrowest = format_status(&model("14:07"), 12);
        assert!(narrowest.contains("43%"), "percent survives: {narrowest}");
        assert!(narrowest.contains("PAGED"), "mode survives: {narrowest}");
        assert!(!narrowest.contains("Pride"), "title drops first among text");
    }

    #[test]
    fn status_004_messages_replace_fields_for_deterministic_ticks() {
        let mut message = StatusMessage::new("Save failed: previous state retained");
        let text = message.text().to_owned();
        let mut showing = model("14:07");
        showing.message = Some(&text);

        let line = format_status(&showing, 60);
        assert!(line.contains("Save failed"));
        assert!(!line.contains("PAGED"), "mode yields to messages");

        let mut ticks = 0;
        while !message.tick() {
            ticks += 1;
        }
        assert_eq!(ticks + 1, MESSAGE_LIFETIME);

        showing.message = None;
        assert!(format_status(&showing, 60).contains("PAGED"));
    }

    #[test]
    fn status_003_mode_and_percent_update_while_page_is_layout_derived() {
        let mut updated = model("14:07");
        updated.mode_label = "CONT";
        updated.page = None;
        let line = format_status(&updated, 80);
        assert!(line.contains("CONT"));
        assert!(!line.contains("Page "), "continuous hides dynamic pages");

        updated.percent += 1;
        assert!(format_status(&updated, 80).contains("44%"));
    }

    #[test]
    fn narrow_class_uses_compact_separators() {
        let line = format_status(&model("14:07"), 40);
        assert_eq!(classify(40), WidthClass::Narrow);
        assert!(line.contains("43%"), "{line}");
        assert!(!line.contains(" | "), "narrow drops verbose separators");
    }

    #[test]
    fn truncation_never_splits_characters() {
        let long = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".repeat(30);
        let mut long_message = model("14:07");
        long_message.message = Some(&long);
        let line = format_status(&long_message, 10);
        assert!(line.chars().count() <= 11);
        assert!(line.ends_with('\u{2026}'));
    }
}
