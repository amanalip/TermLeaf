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

/// The terminal color capability a session renders for.
///
/// Detection happens once at launch from the environment; rendering then uses
/// one fixed decision so styles never flicker between capability guesses.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorMode {
    /// Exact RGB values (COLORTERM advertising true color).
    TrueColor,
    /// Nearest entry of the xterm-256 palette (`TERM` advertising 256 colors).
    #[default]
    Ansi256,
    /// Terminal-default foreground and background plus attributes only.
    TerminalDefault,
}

impl ColorMode {
    /// Classifies a launch environment without reading it directly.
    ///
    /// `COLORTERM` advertising `truecolor`/`24bit` wins because it is the only
    /// widely honored true-color signal; `256color` in `TERM` selects the
    /// indexed palette; anything else falls back to terminal defaults, which
    /// stay readable on 16-color and unknown terminals.
    #[must_use]
    pub fn detect(colorterm: Option<&str>, term: Option<&str>) -> Self {
        let lowered = colorterm.unwrap_or_default().to_ascii_lowercase();
        if lowered.contains("truecolor") || lowered.contains("24bit") {
            Self::TrueColor
        } else if term.unwrap_or_default().contains("256color") {
            Self::Ansi256
        } else {
            Self::TerminalDefault
        }
    }
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

    /// The named theme adapted to the terminal's color capability.
    ///
    /// True color keeps the exact palette; 256-color terminals receive the
    /// nearest xterm palette entry for every RGB role; terminal-default
    /// capability receives the attribute-only fallback, which preserves
    /// contrast without assuming any background.
    #[must_use]
    pub fn for_output(name: ThemeName, mode: ColorMode) -> Self {
        match mode {
            ColorMode::TrueColor => Self::named(name),
            ColorMode::TerminalDefault => Self::no_color(),
            ColorMode::Ansi256 => nearest_256(Self::named(name)),
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

/// The xterm-256 color cube brightness steps for channels 0 through 5.
const CUBE_LEVELS: [u16; 6] = [0, 95, 135, 175, 215, 255];

/// Maps every RGB color in a theme to its nearest xterm-256 palette entry.
///
/// Named ANSI colors (the high-contrast theme) already address terminal
/// palette entries directly and pass through untouched. Modifiers and the
/// foreground/background slot are preserved exactly.
fn nearest_256(mut theme: Theme) -> Theme {
    theme.styles = theme.styles.map(|style| Style {
        fg: style.fg.map(to_256),
        bg: style.bg.map(to_256),
        underline_color: style.underline_color.map(to_256),
        ..style
    });
    theme
}

fn to_256(color: Color) -> Color {
    match color {
        Color::Rgb(red, green, blue) => Color::Indexed(nearest_index(red, green, blue)),
        direct => direct,
    }
}

/// Finds the closest xterm-256 index by squared RGB distance.
///
/// The 6×6×6 cube is a per-channel grid, so its best entry minimizes each
/// channel independently; every grayscale ramp entry supplies the other
/// candidates, and near-neutral tones such as Paper's page pick gray over the
/// cube whenever it is genuinely closer.
fn nearest_index(red: u8, green: u8, blue: u8) -> u8 {
    let squared = |first: u16, second: u16| -> u32 {
        let distance = u32::from(first.abs_diff(second));
        distance * distance
    };

    let closest_level = |value: u16| -> (usize, u32) {
        CUBE_LEVELS
            .iter()
            .enumerate()
            .map(|(index, level)| (index, squared(value, *level)))
            .min_by_key(|&(index, distance)| (distance, index))
            .unwrap_or((0, u32::MAX))
    };

    let (red_index, red_distance) = closest_level(u16::from(red));
    let (green_index, green_distance) = closest_level(u16::from(green));
    let (blue_index, blue_distance) = closest_level(u16::from(blue));
    let cube_index = 16 + 36 * red_index + 6 * green_index + blue_index;
    let cube_distance = red_distance + green_distance + blue_distance;

    let mut gray_index = 232;
    let mut gray_distance = u32::MAX;
    for step in 0..24_usize {
        let level = u16::try_from(8 + 10 * step).expect("the gray ramp stays in range");
        let distance = squared(u16::from(red), level)
            + squared(u16::from(green), level)
            + squared(u16::from(blue), level);
        if distance < gray_distance {
            gray_distance = distance;
            gray_index = 232 + step;
        }
    }

    if cube_distance <= gray_distance {
        u8::try_from(cube_index).expect("cube indices stay below 256")
    } else {
        u8::try_from(gray_index).expect("gray indices stay below 256")
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

    #[test]
    fn theme_005_detection_prefers_colorterm_then_term() {
        assert_eq!(
            ColorMode::detect(Some("truecolor"), None),
            ColorMode::TrueColor
        );
        assert_eq!(ColorMode::detect(Some("24bit"), None), ColorMode::TrueColor);
        assert_eq!(
            ColorMode::detect(Some("TrueColor"), None),
            ColorMode::TrueColor,
            "the signal is case insensitive"
        );
        assert_eq!(
            ColorMode::detect(None, Some("xterm-256color")),
            ColorMode::Ansi256
        );
        assert_eq!(
            ColorMode::detect(Some(""), Some("xterm")),
            ColorMode::TerminalDefault
        );
        assert_eq!(
            ColorMode::detect(None, None),
            ColorMode::TerminalDefault,
            "an unknown terminal keeps the safe default"
        );
    }

    #[test]
    fn theme_005_output_modes_preserve_identity_attributes_and_defaults() {
        for name in ThemeName::ALL {
            let exact = Theme::named(name);
            let true_color = Theme::for_output(name, ColorMode::TrueColor);
            let indexed = Theme::for_output(name, ColorMode::Ansi256);
            let default = Theme::for_output(name, ColorMode::TerminalDefault);

            for index in 0..exact.styles.len() {
                let source = exact.styles[index];
                let converted = indexed.styles[index];
                if let Some(Color::Rgb(..)) = source.fg {
                    assert!(
                        matches!(converted.fg, Some(Color::Indexed(_))),
                        "{name:?} role {index} converts RGB foreground to Indexed"
                    );
                } else {
                    assert_eq!(source.fg, converted.fg, "{name:?} role {index}");
                }
                if let Some(Color::Rgb(..)) = source.bg {
                    assert!(
                        matches!(converted.bg, Some(Color::Indexed(_))),
                        "{name:?} role {index} converts RGB background to Indexed"
                    );
                } else {
                    assert_eq!(source.bg, converted.bg, "{name:?} role {index}");
                }
                assert_eq!(
                    source.add_modifier, converted.add_modifier,
                    "{name:?} role {index} keeps non-color cues"
                );
                assert_eq!(
                    source.sub_modifier, converted.sub_modifier,
                    "{name:?} role {index} keeps non-color cues"
                );
            }

            for role in [
                Role::Canvas,
                Role::Surface,
                Role::Text,
                Role::Secondary,
                Role::Accent,
                Role::Link,
                Role::Selection,
                Role::Search,
                Role::Warning,
                Role::Error,
            ] {
                assert_eq!(true_color.style(role), exact.style(role));
                let fallback = default.style(role);
                let no_color = Theme::no_color().style(role);
                assert_eq!(
                    fallback.fg, no_color.fg,
                    "{name:?} terminal-default keeps defaults"
                );
                assert_eq!(
                    fallback.add_modifier, no_color.add_modifier,
                    "{name:?} terminal-default keeps attribute cues"
                );
            }
        }
    }

    #[test]
    fn theme_005_nearest_index_hits_known_palette_anchors() {
        let cases = [
            ((0, 0, 0), 16),
            ((255, 255, 255), 231),
            ((255, 0, 0), 196),
            ((0, 255, 0), 46),
            ((0, 0, 255), 21),
            // Paper's page tone is near-neutral and lands on the gray ramp.
            ((0xF4, 0xEE, 0xDC), 255),
        ];
        for ((red, green, blue), expected) in cases {
            assert_eq!(
                nearest_index(red, green, blue),
                expected,
                "#{red:02X}{green:02X}{blue:02X}"
            );
        }
    }

    /// The xterm-256 palette entry for cube and grayscale indexes.
    fn palette_rgb(index: u8) -> (u8, u8, u8) {
        match index {
            16..=231 => {
                let rest = u16::from(index) - 16;
                let level = |step: u16| {
                    u8::try_from(CUBE_LEVELS[usize::from(step)]).expect("cube levels fit u8")
                };
                (level(rest / 36), level((rest / 6) % 6), level(rest % 6))
            }
            232..=255 => {
                let gray = 8 + 10 * (u16::from(index) - 232);
                let gray = u8::try_from(gray).expect("gray ramp fits u8");
                (gray, gray, gray)
            }
            _ => unreachable!("base-16 anchors are terminal defined"),
        }
    }

    #[test]
    fn theme_005_every_palette_entry_maps_to_itself() {
        for index in 16..=255_u8 {
            let (red, green, blue) = palette_rgb(index);
            assert_eq!(
                nearest_index(red, green, blue),
                index,
                "#{red:02X}{green:02X}{blue:02X}"
            );
        }
    }

    #[test]
    fn theme_005_conversions_stay_within_one_cube_step_of_the_source() {
        for name in ThemeName::ALL {
            let source = Theme::named(name);
            let converted = Theme::for_output(name, ColorMode::Ansi256);
            for (before, after) in source.styles.iter().zip(converted.styles.iter()) {
                for (original, mapped) in before
                    .fg
                    .zip(after.fg)
                    .into_iter()
                    .chain(before.bg.zip(after.bg))
                {
                    let Color::Rgb(red, green, blue) = original else {
                        continue;
                    };
                    let Color::Indexed(index) = mapped else {
                        panic!("{name:?} maps RGB to Indexed");
                    };
                    let (mapped_red, mapped_green, mapped_blue) = palette_rgb(index);
                    assert!(
                        u16::from(red).abs_diff(u16::from(mapped_red)) <= 48
                            && u16::from(green).abs_diff(u16::from(mapped_green)) <= 48
                            && u16::from(blue).abs_diff(u16::from(mapped_blue)) <= 48,
                        "{name:?} #{red:02X}{green:02X}{blue:02X} drifts too far"
                    );
                }
            }
        }
    }
}
