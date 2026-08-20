use std::io::{self, stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    app::{App, action_for},
    ui,
};

trait TerminalControl {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn hide_cursor(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
    fn enable_bracketed_paste(&mut self) -> io::Result<()>;
    fn disable_bracketed_paste(&mut self) -> io::Result<()>;
}

#[derive(Debug, Default)]
pub struct CrosstermControl;

impl TerminalControl for CrosstermControl {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        execute!(stdout(), EnterAlternateScreen)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        execute!(stdout(), LeaveAlternateScreen)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(stdout(), Hide)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        execute!(stdout(), Show)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        execute!(stdout(), EnableBracketedPaste)
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        execute!(stdout(), DisableBracketedPaste)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetupStage {
    New,
    RawMode,
    AlternateScreen,
    CursorHidden,
    BracketedPaste,
    Restored,
}

#[derive(Debug)]
struct TerminalSession<C: TerminalControl> {
    control: C,
    stage: SetupStage,
}

impl<C: TerminalControl> TerminalSession<C> {
    pub fn start(control: C) -> io::Result<Self> {
        let mut session = Self {
            control,
            stage: SetupStage::New,
        };

        if let Err(error) = session.setup() {
            let _cleanup_result = session.restore();
            return Err(error);
        }
        Ok(session)
    }

    fn setup(&mut self) -> io::Result<()> {
        self.control.enable_raw_mode()?;
        self.stage = SetupStage::RawMode;
        self.control.enter_alternate_screen()?;
        self.stage = SetupStage::AlternateScreen;
        self.control.hide_cursor()?;
        self.stage = SetupStage::CursorHidden;
        self.control.enable_bracketed_paste()?;
        self.stage = SetupStage::BracketedPaste;
        Ok(())
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if self.stage == SetupStage::Restored {
            return Ok(());
        }

        let mut first_error = None;
        if self.stage == SetupStage::BracketedPaste {
            record_error(&mut first_error, self.control.disable_bracketed_paste());
        }
        if matches!(
            self.stage,
            SetupStage::BracketedPaste | SetupStage::CursorHidden
        ) {
            record_error(&mut first_error, self.control.show_cursor());
        }
        if matches!(
            self.stage,
            SetupStage::BracketedPaste | SetupStage::CursorHidden | SetupStage::AlternateScreen
        ) {
            record_error(&mut first_error, self.control.leave_alternate_screen());
        }
        if matches!(
            self.stage,
            SetupStage::BracketedPaste
                | SetupStage::CursorHidden
                | SetupStage::AlternateScreen
                | SetupStage::RawMode
        ) {
            record_error(&mut first_error, self.control.disable_raw_mode());
        }
        self.stage = SetupStage::Restored;

        first_error.map_or(Ok(()), Err)
    }
}

impl<C: TerminalControl> Drop for TerminalSession<C> {
    fn drop(&mut self) {
        let _cleanup_result = self.restore();
    }
}

fn record_error(first_error: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result
        && first_error.is_none()
    {
        *first_error = Some(error);
    }
}

/// Runs the full-screen terminal event loop until the application exits.
///
/// # Errors
///
/// Returns an error when terminal setup, rendering, event input, or restoration
/// fails. Any modes changed before a failure are still offered for restoration.
pub fn run(mut app: App) -> Result<()> {
    let mut session =
        TerminalSession::start(CrosstermControl).context("could not initialize the terminal")?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("could not create the terminal renderer")?;

    let run_result = run_loop(&mut terminal, &mut app);
    drop(terminal);
    let restore_result = session
        .restore()
        .context("could not fully restore the terminal");

    run_result.and(restore_result)
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.is_running() {
        terminal.draw(|frame| ui::render(frame, app))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(action) = action_for(key)
        {
            app.update(action);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[derive(Debug, Default)]
    struct MockState {
        calls: Vec<&'static str>,
        fail_at: Option<&'static str>,
    }

    #[derive(Clone, Debug)]
    struct MockControl(Rc<RefCell<MockState>>);

    impl MockControl {
        fn call(&self, name: &'static str) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.calls.push(name);
            if state.fail_at == Some(name) {
                return Err(io::Error::other("injected terminal failure"));
            }
            Ok(())
        }
    }

    impl TerminalControl for MockControl {
        fn enable_raw_mode(&mut self) -> io::Result<()> {
            self.call("enable_raw")
        }
        fn disable_raw_mode(&mut self) -> io::Result<()> {
            self.call("disable_raw")
        }
        fn enter_alternate_screen(&mut self) -> io::Result<()> {
            self.call("enter_screen")
        }
        fn leave_alternate_screen(&mut self) -> io::Result<()> {
            self.call("leave_screen")
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.call("hide_cursor")
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.call("show_cursor")
        }
        fn enable_bracketed_paste(&mut self) -> io::Result<()> {
            self.call("enable_paste")
        }
        fn disable_bracketed_paste(&mut self) -> io::Result<()> {
            self.call("disable_paste")
        }
    }

    fn mock(fail_at: Option<&'static str>) -> (MockControl, Rc<RefCell<MockState>>) {
        let state = Rc::new(RefCell::new(MockState {
            calls: Vec::new(),
            fail_at,
        }));
        (MockControl(Rc::clone(&state)), state)
    }

    #[test]
    fn term_001_normal_exit_restores_changed_modes_in_reverse_order() -> io::Result<()> {
        let (control, state) = mock(None);
        let mut session = TerminalSession::start(control)?;

        session.restore()?;

        assert_eq!(
            state.borrow().calls,
            [
                "enable_raw",
                "enter_screen",
                "hide_cursor",
                "enable_paste",
                "disable_paste",
                "show_cursor",
                "leave_screen",
                "disable_raw",
            ]
        );
        Ok(())
    }

    #[test]
    fn term_005_each_setup_failure_rolls_back_completed_steps() {
        let cases: &[(&str, &[&str])] = &[
            ("enable_raw", &["enable_raw"]),
            (
                "enter_screen",
                &["enable_raw", "enter_screen", "disable_raw"],
            ),
            (
                "hide_cursor",
                &[
                    "enable_raw",
                    "enter_screen",
                    "hide_cursor",
                    "leave_screen",
                    "disable_raw",
                ],
            ),
            (
                "enable_paste",
                &[
                    "enable_raw",
                    "enter_screen",
                    "hide_cursor",
                    "enable_paste",
                    "show_cursor",
                    "leave_screen",
                    "disable_raw",
                ],
            ),
        ];

        for (failure, expected_calls) in cases {
            let (control, state) = mock(Some(failure));

            assert!(TerminalSession::start(control).is_err());
            assert_eq!(&state.borrow().calls, expected_calls, "failed at {failure}");
        }
    }

    #[test]
    fn term_004_unwinding_drops_the_guard_and_restores_modes() {
        let (control, state) = mock(None);

        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = TerminalSession::start(control);
            panic!("controlled test panic");
        }));

        assert!(panic_result.is_err());
        assert!(state.borrow().calls.ends_with(&[
            "disable_paste",
            "show_cursor",
            "leave_screen",
            "disable_raw",
        ]));
    }

    #[test]
    fn cleanup_attempts_every_step_after_one_restore_failure() -> io::Result<()> {
        let (control, state) = mock(None);
        let mut session = TerminalSession::start(control)?;
        state.borrow_mut().fail_at = Some("show_cursor");

        assert!(session.restore().is_err());

        let calls = &state.borrow().calls;
        assert!(calls.ends_with(&[
            "disable_paste",
            "show_cursor",
            "leave_screen",
            "disable_raw",
        ]));
        Ok(())
    }
}
