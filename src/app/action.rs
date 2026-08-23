use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Every application action reachable from the keyboard in Phase 1.
///
/// The map mixes conventional terminal expectations with a small Vim-style
/// family and stays conflict-free: no essential action requires modifiers
/// beyond `Ctrl-C`/`Ctrl-B`/`Ctrl-F`, mouse input, `AltGr`, key-release events, or
/// modern keyboard extensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Quit,
    ShowHelp,
    Back,
    Confirm,
    NextLine,
    PreviousLine,
    NextPage,
    PreviousPage,
    DocumentStart,
    DocumentEnd,
    SectionStart,
    SectionEnd,
    SetModePaged,
    SetModeContinuous,
    ShowThemes,
    ShowToc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding {
    pub action: Action,
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub label: &'static str,
    pub description: &'static str,
}

const BINDINGS: &[Binding] = &[
    Binding {
        action: Action::Quit,
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        label: "q",
        description: "Quit",
    },
    Binding {
        action: Action::Quit,
        key: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        label: "Ctrl-C",
        description: "Quit",
    },
    Binding {
        action: Action::ShowHelp,
        key: KeyCode::F(1),
        modifiers: KeyModifiers::NONE,
        label: "F1",
        description: "Help",
    },
    Binding {
        action: Action::ShowHelp,
        key: KeyCode::Char('?'),
        modifiers: KeyModifiers::NONE,
        label: "?",
        description: "Help",
    },
    Binding {
        action: Action::Back,
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        label: "Esc",
        description: "Back",
    },
    Binding {
        action: Action::NextLine,
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        label: "Down",
        description: "Next line",
    },
    Binding {
        action: Action::NextLine,
        key: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        label: "j",
        description: "Next line",
    },
    Binding {
        action: Action::PreviousLine,
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        label: "Up",
        description: "Previous line",
    },
    Binding {
        action: Action::PreviousLine,
        key: KeyCode::Char('k'),
        modifiers: KeyModifiers::NONE,
        label: "k",
        description: "Previous line",
    },
    Binding {
        action: Action::NextPage,
        key: KeyCode::PageDown,
        modifiers: KeyModifiers::NONE,
        label: "PgDn",
        description: "Next page",
    },
    Binding {
        action: Action::NextPage,
        key: KeyCode::Char('f'),
        modifiers: KeyModifiers::CONTROL,
        label: "Ctrl-F",
        description: "Next page",
    },
    Binding {
        action: Action::PreviousPage,
        key: KeyCode::PageUp,
        modifiers: KeyModifiers::NONE,
        label: "PgUp",
        description: "Previous page",
    },
    Binding {
        action: Action::PreviousPage,
        key: KeyCode::Char('b'),
        modifiers: KeyModifiers::CONTROL,
        label: "Ctrl-B",
        description: "Previous page",
    },
    Binding {
        action: Action::DocumentStart,
        key: KeyCode::Home,
        modifiers: KeyModifiers::NONE,
        label: "Home",
        description: "Book start",
    },
    Binding {
        action: Action::DocumentStart,
        key: KeyCode::Char('g'),
        modifiers: KeyModifiers::NONE,
        label: "gg",
        description: "Book start",
    },
    Binding {
        action: Action::DocumentEnd,
        key: KeyCode::End,
        modifiers: KeyModifiers::NONE,
        label: "End",
        description: "Book end",
    },
    Binding {
        action: Action::DocumentEnd,
        key: KeyCode::Char('G'),
        modifiers: KeyModifiers::NONE,
        label: "G",
        description: "Book end",
    },
    Binding {
        action: Action::SectionStart,
        key: KeyCode::Char('{'),
        modifiers: KeyModifiers::NONE,
        label: "{",
        description: "Previous section",
    },
    Binding {
        action: Action::SectionEnd,
        key: KeyCode::Char('}'),
        modifiers: KeyModifiers::NONE,
        label: "}",
        description: "Next section",
    },
    Binding {
        action: Action::SetModePaged,
        key: KeyCode::Char('p'),
        modifiers: KeyModifiers::NONE,
        label: "p",
        description: "Paged mode",
    },
    Binding {
        action: Action::SetModeContinuous,
        key: KeyCode::Char('c'),
        modifiers: KeyModifiers::NONE,
        label: "c",
        description: "Continuous mode",
    },
    Binding {
        action: Action::ShowThemes,
        key: KeyCode::Char('t'),
        modifiers: KeyModifiers::NONE,
        label: "t",
        description: "Themes",
    },
    Binding {
        action: Action::ShowToc,
        key: KeyCode::Char('o'),
        modifiers: KeyModifiers::NONE,
        label: "o",
        description: "Contents",
    },
    Binding {
        action: Action::ShowToc,
        key: KeyCode::F(2),
        modifiers: KeyModifiers::NONE,
        label: "F2",
        description: "Contents",
    },
    Binding {
        action: Action::Confirm,
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        label: "Enter",
        description: "Apply",
    },
];

#[must_use]
pub const fn bindings() -> &'static [Binding] {
    BINDINGS
}

/// Turns key events into actions, owning the multikey prefix state.
///
/// Prefix policy: a lone `g` opens the countable prefix; a second `g`
/// completes book-start, and any other key cancels the prefix and is then
/// mapped normally, so unrelated input is never lost. The policy needs no
/// timer, which keeps behavior deterministic in tests.
#[derive(Debug, Default)]
pub struct KeyMapper {
    g_prefix_pending: bool,
}

impl KeyMapper {
    #[must_use]
    pub fn map(&mut self, event: KeyEvent) -> Option<Action> {
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return None;
        }
        let event = normalize_shift(event);

        if self.g_prefix_pending {
            self.g_prefix_pending = false;
            return match (event.code, event.modifiers) {
                (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Action::DocumentStart),
                _ => Self::map_single(event),
            };
        }

        if matches!(
            (event.code, event.modifiers),
            (KeyCode::Char('g'), KeyModifiers::NONE)
        ) {
            self.g_prefix_pending = true;
            return None;
        }
        Self::map_single(event)
    }

    fn map_single(event: KeyEvent) -> Option<Action> {
        BINDINGS
            .iter()
            .find(|binding| binding.key == event.code && binding.modifiers == event.modifiers)
            .map(|binding| binding.action)
    }
}

/// Drops the SHIFT modifier from character keys before matching.
///
/// Real terminals report capital letters as the uppercase character carrying
/// SHIFT; the registry addresses characters directly (`G`, `?`), so SHIFT
/// must not participate in the comparison. Ctrl- and Alt-based bindings keep
/// their modifiers.
fn normalize_shift(mut event: KeyEvent) -> KeyEvent {
    if matches!(event.code, KeyCode::Char(_)) {
        event.modifiers.remove(KeyModifiers::SHIFT);
    }
    event
}

#[must_use]
pub fn action_for(event: KeyEvent) -> Option<Action> {
    KeyMapper::default().map(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn key_004_ignores_key_release_events() {
        let event = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );

        assert_eq!(action_for(event), None);
    }

    #[test]
    fn app_002_help_and_input_use_the_same_binding_registry() {
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);

        assert_eq!(action_for(event), Some(Action::ShowHelp));
        assert!(bindings().iter().any(|binding| binding.label == "F1"));
    }

    #[test]
    fn term_003_ctrl_c_requests_a_clean_exit() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert_eq!(action_for(event), Some(Action::Quit));
    }

    #[test]
    fn nav_007_conventional_and_vim_families_reach_the_same_actions() {
        let cases = [
            (key(KeyCode::Down, KeyModifiers::NONE), Action::NextLine),
            (
                key(KeyCode::Char('j'), KeyModifiers::NONE),
                Action::NextLine,
            ),
            (key(KeyCode::Up, KeyModifiers::NONE), Action::PreviousLine),
            (
                key(KeyCode::Char('k'), KeyModifiers::NONE),
                Action::PreviousLine,
            ),
            (key(KeyCode::PageDown, KeyModifiers::NONE), Action::NextPage),
            (
                key(KeyCode::Char('f'), KeyModifiers::CONTROL),
                Action::NextPage,
            ),
            (
                key(KeyCode::PageUp, KeyModifiers::NONE),
                Action::PreviousPage,
            ),
            (
                key(KeyCode::Char('b'), KeyModifiers::CONTROL),
                Action::PreviousPage,
            ),
            (
                key(KeyCode::Home, KeyModifiers::NONE),
                Action::DocumentStart,
            ),
            (key(KeyCode::End, KeyModifiers::NONE), Action::DocumentEnd),
            (
                key(KeyCode::Char('G'), KeyModifiers::NONE),
                Action::DocumentEnd,
            ),
            (
                key(KeyCode::Char('G'), KeyModifiers::SHIFT),
                Action::DocumentEnd,
            ),
            (
                key(KeyCode::Char('?'), KeyModifiers::SHIFT),
                Action::ShowHelp,
            ),
            (
                key(KeyCode::Char('{'), KeyModifiers::NONE),
                Action::SectionStart,
            ),
            (
                key(KeyCode::Char('}'), KeyModifiers::NONE),
                Action::SectionEnd,
            ),
            (
                key(KeyCode::Char('p'), KeyModifiers::NONE),
                Action::SetModePaged,
            ),
            (
                key(KeyCode::Char('c'), KeyModifiers::NONE),
                Action::SetModeContinuous,
            ),
            (
                key(KeyCode::Char('t'), KeyModifiers::NONE),
                Action::ShowThemes,
            ),
        ];

        for (event, expected) in cases {
            assert_eq!(action_for(event), Some(expected), "for {event:?}");
        }
    }

    #[test]
    fn key_003_gg_prefix_is_deterministic_and_never_loses_input() {
        let mut mapper = KeyMapper::default();

        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            Some(Action::DocumentStart)
        );

        let mut mapper = KeyMapper::default();
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            mapper.map(key(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Action::NextLine),
            "unrelated key after lone g still maps"
        );
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            None,
            "prefix was consumed by the cancellation"
        );

        let mut mapper = KeyMapper::default();
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::CONTROL)),
            None,
            "control-modified g cancels the prefix without completing it"
        );

        let mut mapper = KeyMapper::default();
        assert_eq!(
            mapper.map(key(KeyCode::Char('g'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            mapper.map(key(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Some(Action::DocumentEnd),
            "a terminal's shifted G cancels the prefix and then maps normally"
        );
    }

    #[test]
    fn key_002_flow_control_keys_do_not_collide_with_text_keys() {
        assert_ne!(
            action_for(key(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            action_for(key(KeyCode::Char('b'), KeyModifiers::NONE))
        );
        assert_ne!(
            action_for(key(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            action_for(key(KeyCode::Char('f'), KeyModifiers::NONE))
        );
    }

    #[test]
    fn every_binding_has_a_unique_label_and_help_entry() {
        let mut labels: Vec<&str> = bindings().iter().map(|b| b.label).collect();
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), count, "labels are unique for help rendering");
    }
}
