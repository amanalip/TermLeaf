use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Quit,
    ShowHelp,
    Back,
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
];

#[must_use]
pub const fn bindings() -> &'static [Binding] {
    BINDINGS
}

#[must_use]
pub fn action_for(event: KeyEvent) -> Option<Action> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }

    BINDINGS
        .iter()
        .find(|binding| binding.key == event.code && binding.modifiers == event.modifiers)
        .map(|binding| binding.action)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
