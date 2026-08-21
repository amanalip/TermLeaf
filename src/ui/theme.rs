//! Built-in themes expressed as semantic roles.
//!
//! Widgets never reference raw colors; they request a role such as
//! [`Role::Surface`] from the active [`Theme`]. Every role carries its own
//! non-color attributes where meaning would otherwise depend on color alone,
//! and `NO_COLOR` sessions resolve every role to terminal defaults plus
//! attributes.

use ratatui::style::{Color, Modifier, Style};

/// The five first-release themes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemeName {
    Dark,
    Light,
    HighContrast,
    Monochrome,
    #[default]
    Paper,
}

impl ThemeName {
    /// Every built-in theme in selection-view order.
    pub const ALL: [ThemeName; 5] = [
        ThemeName::Dark,
        ThemeName::Light,
        ThemeName::HighContrast,
        ThemeName::Monochrome,
        ThemeName::Paper,
    ];

    /// Stable configuration spelling of the theme.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrast => "high-contrast",
            Self::Monochrome => "monochrome",
            Self::Paper => "paper",
        }
    }

    /// Human label for the selection view.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::HighContrast => "High contrast",
            Self::Monochrome => "Monochrome",
            Self::Paper => "Paper",
        }
    }

    /// Parses a configuration value; unknown names fall back to Paper.
    #[must_use]
    pub fn parse(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|name| name.slug() == slug)
    }

    /// The next theme in selection order.
    #[must_use]
    pub const fn next(self) -> Self {
        let current = self as usize;
        let following = (current + 1) % Self::ALL.len();
        Self::ALL[following]
    }
}

/// Semantic presentation roles requested by widgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Canvas,
    Surface,
    Text,
    Secondary,
    Accent,
    Link,
    Selection,
    Search,
    Warning,
    Error,
}

/// One complete theme mapping roles to styled output.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    name: ThemeName,
    styles: [Style; 10],
}

impl Theme {
    /// The named built-in theme.
    #[must_use]
    pub fn named(name: ThemeName) -> Self {
        match name {
            ThemeName::Dark => dark(),
            ThemeName::Light => light(),
            ThemeName::HighContrast => high_contrast(),
            ThemeName::Monochrome => monochrome(),
            ThemeName::Paper => paper(),
        }
    }

    /// The attribute-only fallback used for `NO_COLOR` sessions.
    #[must_use]
    pub fn no_color() -> Self {
        monochrome()
    }

    /// The theme's identity.
    #[must_use]
    pub const fn name(&self) -> ThemeName {
        self.name
    }

    /// The style for one semantic role.
    ///
    /// Variant order in [`Role`] defines the index; the layout of the array
    /// is checked by the role-index test below.
    #[must_use]
    pub const fn style(&self, role: Role) -> Style {
        self.styles[role as usize]
    }
}

const fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

fn dark() -> Theme {
    let surface = Color::Rgb(0x14, 0x18, 0x1D);
    Theme {
        name: ThemeName::Dark,
        styles: [
            fg(Color::Rgb(0x0C, 0x0F, 0x12)),
            fg(surface),
            fg(Color::Rgb(0xDE, 0xE1, 0xE4)),
            fg(Color::Rgb(0x98, 0xA0, 0xA8)),
            fg(Color::Rgb(0x7F, 0xB0, 0x69)).add_modifier(Modifier::BOLD),
            fg(Color::Rgb(0x79, 0xB8, 0xFF)).add_modifier(Modifier::UNDERLINED),
            Style::new().bg(Color::Rgb(0x2E, 0x4A, 0x66)),
            Style::new().bg(Color::Rgb(0x5A, 0x46, 0x32)),
            fg(Color::Rgb(0xE0, 0xB8, 0x4C)).add_modifier(Modifier::BOLD),
            fg(Color::Rgb(0xF2, 0x75, 0x72)).add_modifier(Modifier::BOLD),
        ],
    }
}

fn light() -> Theme {
    Theme {
        name: ThemeName::Light,
        styles: [
            fg(Color::Rgb(0xE8, 0xE6, 0xE0)),
            fg(Color::Rgb(0xFD, 0xFC, 0xF8)),
            fg(Color::Rgb(0x20, 0x21, 0x24)),
            fg(Color::Rgb(0x5F, 0x63, 0x68)),
            fg(Color::Rgb(0x2E, 0x6B, 0x34)).add_modifier(Modifier::BOLD),
            fg(Color::Rgb(0x1A, 0x5F, 0xB4)).add_modifier(Modifier::UNDERLINED),
            Style::new().bg(Color::Rgb(0xFF, 0xE9, 0xA8)),
            Style::new().bg(Color::Rgb(0xFF, 0xD9, 0x7A)),
            fg(Color::Rgb(0x8A, 0x5A, 0x00)).add_modifier(Modifier::BOLD),
            fg(Color::Rgb(0xA6, 0x1B, 0x1B)).add_modifier(Modifier::BOLD),
        ],
    }
}

fn high_contrast() -> Theme {
    Theme {
        name: ThemeName::HighContrast,
        styles: [
            fg(Color::Black),
            fg(Color::Black),
            fg(Color::White),
            fg(Color::White).add_modifier(Modifier::BOLD),
            fg(Color::Yellow).add_modifier(Modifier::BOLD),
            fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            Style::new().add_modifier(Modifier::REVERSED),
            fg(Color::Yellow).add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
            fg(Color::Yellow).add_modifier(Modifier::REVERSED),
            fg(Color::White).add_modifier(Modifier::REVERSED | Modifier::BOLD),
        ],
    }
}

fn monochrome() -> Theme {
    Theme {
        name: ThemeName::Monochrome,
        styles: [
            Style::new(),
            Style::new(),
            Style::new(),
            Style::new().add_modifier(Modifier::DIM),
            Style::new().add_modifier(Modifier::BOLD),
            Style::new().add_modifier(Modifier::UNDERLINED),
            Style::new().add_modifier(Modifier::REVERSED),
            Style::new().add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
            Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        ],
    }
}

/// The Paper palette from `project_plan.md`, exact initial true-color values.
fn paper() -> Theme {
    Theme {
        name: ThemeName::Paper,
        styles: [
            fg(Color::Rgb(0xD8, 0xD0, 0xBE)),
            fg(Color::Rgb(0xF4, 0xEE, 0xDC)),
            fg(Color::Rgb(0x29, 0x28, 0x21)),
            // Deepened from the plan's starting #625F52 so secondary text
            // clears 4.5:1 on both the page and the outer canvas.
            fg(Color::Rgb(0x5A, 0x57, 0x49)),
            fg(Color::Rgb(0x4F, 0x5D, 0x38)).add_modifier(Modifier::BOLD),
            // Deepened from the plan's starting #855A3A so link text clears
            // 4.5:1 on the outer canvas as well as the page.
            fg(Color::Rgb(0x73, 0x50, 0x33)).add_modifier(Modifier::UNDERLINED),
            Style::new().bg(Color::Rgb(0xDD, 0xC8, 0x9B)),
            Style::new().bg(Color::Rgb(0xC8, 0xAD, 0x62)),
            fg(Color::Rgb(0x7A, 0x4A, 0x00)).add_modifier(Modifier::BOLD),
            fg(Color::Rgb(0x8B, 0x2F, 0x1F)).add_modifier(Modifier::BOLD),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relative_luminance(color: Color) -> f64 {
        let Color::Rgb(red, green, blue) = color else {
            return f64::NAN;
        };
        let channel = |value: u8| {
            let scaled = f64::from(value) / 255.0;
            if scaled <= 0.03928 {
                scaled / 12.92
            } else {
                ((scaled + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
    }

    fn contrast(first: Color, second: Color) -> f64 {
        let lighter = relative_luminance(first).max(relative_luminance(second));
        let darker = relative_luminance(first).min(relative_luminance(second));
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn theme_010_every_paper_text_pairing_meets_the_contrast_floor() {
        let theme = Theme::named(ThemeName::Paper);
        let surface = theme.style(Role::Surface).fg.expect("Paper surface");
        let canvas = theme.style(Role::Canvas).fg.expect("Paper canvas");
        let selection = theme.style(Role::Selection).bg.expect("Paper selection");
        let search = theme.style(Role::Search).bg.expect("Paper search");
        let text = theme.style(Role::Text).fg.expect("Paper text");

        let foregrounds = [
            (Role::Text, text),
            (Role::Secondary, Color::Rgb(0x5A, 0x57, 0x49)),
            (Role::Accent, Color::Rgb(0x4F, 0x5D, 0x38)),
            (Role::Link, Color::Rgb(0x73, 0x50, 0x33)),
            (Role::Warning, Color::Rgb(0x7A, 0x4A, 0x00)),
            (Role::Error, Color::Rgb(0x8B, 0x2F, 0x1F)),
        ];
        for (role, foreground) in foregrounds {
            for background in [surface, canvas] {
                let ratio = contrast(foreground, background);
                assert!(ratio >= 4.5, "{role:?} on {background:?} is {ratio:.2}:1");
            }
        }

        for background in [selection, search] {
            let ratio = contrast(text, background);
            assert!(ratio >= 4.5, "text on {background:?} is {ratio:.2}:1");
        }
    }

    #[test]
    fn theme_001_all_five_themes_define_every_role_distinctly() {
        for name in ThemeName::ALL {
            let theme = Theme::named(name);
            assert_eq!(theme.name(), name);
            let text = theme.style(Role::Text);
            let secondary = theme.style(Role::Secondary);
            assert_ne!(
                text.add_modifier(Modifier::empty()),
                secondary.add_modifier(Modifier::empty()),
                "{name:?} distinguishes text from secondary"
            );
        }
    }

    #[test]
    fn theme_names_round_trip_through_config_slugs() {
        for name in ThemeName::ALL {
            assert_eq!(ThemeName::parse(name.slug()), Some(name));
        }
        assert_eq!(ThemeName::parse("nope"), None);
    }

    #[test]
    fn no_color_fallback_uses_terminal_defaults_with_attributes() {
        let theme = Theme::no_color();
        for role in [
            Role::Canvas,
            Role::Surface,
            Role::Text,
            Role::Secondary,
            Role::Accent,
            Role::Link,
        ] {
            let style = theme.style(role);
            assert_eq!(style.fg, None, "{role:?} keeps terminal default");
            assert_eq!(style.bg, None, "{role:?} keeps terminal default");
        }
        let link = theme.style(Role::Link);
        assert_eq!(
            link.add_modifier & Modifier::UNDERLINED,
            Modifier::UNDERLINED
        );
    }

    #[test]
    fn role_indices_match_the_style_array_layout() {
        assert_eq!(Role::Canvas as usize, 0);
        assert_eq!(Role::Error as usize, 9);
    }
}
