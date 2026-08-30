use std::time::Duration;
use std::{
    io::{self, stdout},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    app::{Action, App, KeyMapper},
    interrupt,
    terminal_image::{NativeFramePlan, NativeGraphicsSession},
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
    run_with_loop(&mut app, run_loop)
}

trait WorkerTeardown {
    fn request_shutdown(&mut self);
    fn join(&mut self);
}

impl WorkerTeardown for App {
    fn request_shutdown(&mut self) {
        self.request_worker_shutdown();
    }

    fn join(&mut self) {
        self.join_workers();
    }
}

fn ordered_teardown<W, T, C, F>(
    workers: &mut W,
    mut terminal: T,
    cleanup_graphics: C,
    restore: F,
) -> Result<()>
where
    W: WorkerTeardown,
    C: FnOnce(&mut T) -> Result<()>,
    F: FnOnce() -> Result<()>,
{
    workers.request_shutdown();
    let graphics_result = cleanup_graphics(&mut terminal);
    drop(terminal);
    let restore_result = restore();
    workers.join();
    graphics_result.and(restore_result)
}

fn run_with_loop<F>(app: &mut App, loop_operation: F) -> Result<()>
where
    F: FnOnce(
        &mut Terminal<CrosstermBackend<io::Stdout>>,
        &mut App,
        &mut NativeGraphicsSession,
    ) -> Result<()>,
{
    let mut session =
        TerminalSession::start(CrosstermControl).context("could not initialize the terminal")?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal =
        match Terminal::new(backend).context("could not create the terminal renderer") {
            Ok(terminal) => terminal,
            Err(error) => {
                let restore_result = ordered_teardown(
                    app,
                    (),
                    |()| Ok(()),
                    || {
                        session
                            .restore()
                            .context("could not fully restore the terminal")
                    },
                );
                return Err::<(), _>(error).and(restore_result);
            }
        };

    let mut graphics = NativeGraphicsSession::default();
    let run_result = catch_unwind(AssertUnwindSafe(|| {
        loop_operation(&mut terminal, app, &mut graphics)
    }));
    let cleanup_result = ordered_teardown(
        app,
        terminal,
        |_| {
            graphics
                .cleanup(&mut stdout())
                .context("could not clean up terminal images")
        },
        || {
            session
                .restore()
                .context("could not fully restore the terminal")
        },
    );
    match run_result {
        Ok(result) => result.and(cleanup_result),
        Err(payload) => resume_unwind(payload),
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    graphics: &mut NativeGraphicsSession,
) -> Result<()> {
    // One mapper owns multikey prefix state for the whole session; a fresh
    // mapper per event would make sequences such as `gg` impossible.
    let mut mapper = KeyMapper::default();
    while app.is_running() {
        app.drain_image_work();
        if interrupt::requested() {
            app.update(crate::app::Action::Quit);
            continue;
        }
        let mut native = NativeFramePlan::default();
        terminal.draw(|frame| ui::render_with_native(frame, app, &mut native))?;
        if graphics.requires_full_redraw(&native) {
            terminal.clear()?;
            native = NativeFramePlan::default();
            terminal.draw(|frame| ui::render_with_native(frame, app, &mut native))?;
        }
        graphics.synchronize(&mut stdout(), native)?;
        if event::poll(Duration::from_millis(100))?
            && let Some(action) = action_from_event(&mut mapper, &event::read()?)
        {
            app.update(action);
        }
    }
    Ok(())
}

/// Converts one terminal event into an application action.
///
/// Focus, mouse, resize, and bracketed-paste events are deliberately inert:
/// no Phase 1 mode consumes them, so forwarding them could only move state
/// by surprise. Keyboard events keep the session key mapper so multikey
/// prefixes survive between reads, including across inert events.
fn action_from_event(mapper: &mut KeyMapper, event: &Event) -> Option<Action> {
    match event {
        Event::Key(key) => mapper.map(*key),
        Event::FocusGained
        | Event::FocusLost
        | Event::Mouse(_)
        | Event::Resize(..)
        | Event::Paste(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    #[cfg(unix)]
    use anyhow::bail;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
    };

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
                return Err(io::Error::other(name));
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
    fn term_009_cancels_then_restores_terminal_before_joining_workers() -> Result<()> {
        struct Workers(Rc<RefCell<Vec<&'static str>>>);
        impl WorkerTeardown for Workers {
            fn request_shutdown(&mut self) {
                self.0.borrow_mut().push("cancel");
            }

            fn join(&mut self) {
                self.0.borrow_mut().push("join");
            }
        }

        struct TerminalDrop(Rc<RefCell<Vec<&'static str>>>);
        impl Drop for TerminalDrop {
            fn drop(&mut self) {
                self.0.borrow_mut().push("drop_terminal");
            }
        }

        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut workers = Workers(Rc::clone(&calls));
        let terminal = TerminalDrop(Rc::clone(&calls));
        ordered_teardown(
            &mut workers,
            terminal,
            |_| {
                calls.borrow_mut().push("cleanup_graphics");
                Ok(())
            },
            || {
                calls.borrow_mut().push("restore");
                Ok(())
            },
        )?;

        assert_eq!(
            *calls.borrow(),
            [
                "cancel",
                "cleanup_graphics",
                "drop_terminal",
                "restore",
                "join"
            ]
        );

        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut workers = Workers(Rc::clone(&calls));
        let terminal = TerminalDrop(Rc::clone(&calls));
        let result = ordered_teardown(
            &mut workers,
            terminal,
            |_| {
                calls.borrow_mut().push("cleanup_graphics");
                Err(anyhow::anyhow!("graphics cleanup failed"))
            },
            || {
                calls.borrow_mut().push("restore");
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(
            *calls.borrow(),
            [
                "cancel",
                "cleanup_graphics",
                "drop_terminal",
                "restore",
                "join"
            ],
            "a graphics failure cannot skip restoration or worker joins"
        );
        Ok(())
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
    fn app_004_cleanup_attempts_every_step_after_one_restore_failure() -> io::Result<()> {
        let (control, state) = mock(None);
        let mut session = TerminalSession::start(control)?;
        state.borrow_mut().fail_at = Some("show_cursor");

        let error = session
            .restore()
            .expect_err("show cursor restoration fails");

        let calls = &state.borrow().calls;
        assert!(calls.ends_with(&[
            "disable_paste",
            "show_cursor",
            "leave_screen",
            "disable_raw",
        ]));
        assert_eq!(error.to_string(), "show_cursor");
        Ok(())
    }

    #[test]
    fn term_007_focus_mouse_resize_and_paste_events_are_inert() {
        let mouse = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::NONE,
        });
        let events = [
            Event::FocusGained,
            Event::FocusLost,
            mouse,
            Event::Resize(80, 24),
            Event::Paste("q\n\x03whole paste".to_owned()),
        ];

        for event in events {
            let mut mapper = KeyMapper::default();
            assert!(
                action_from_event(&mut mapper, &event).is_none(),
                "{event:?} must not become an action"
            );
        }
    }

    #[test]
    fn term_007_unsupported_key_events_do_not_map_but_bindings_survive_inert_events() {
        let unsupported = [
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT),
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            ),
            KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE),
        ];
        for event in unsupported {
            let mut mapper = KeyMapper::default();
            assert!(action_from_event(&mut mapper, &Event::Key(event)).is_none());
        }

        // Inert traffic must not consume or corrupt the prefix state: a lone
        // `g`, then a paste, then `g` still completes book-start.
        let mut mapper = KeyMapper::default();
        let press = |mapper: &mut KeyMapper, code| {
            action_from_event(mapper, &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
        };
        assert_eq!(press(&mut mapper, KeyCode::Char('g')), None);
        assert_eq!(
            action_from_event(&mut mapper, &Event::Paste("ignored".to_owned())),
            None
        );
        assert_eq!(
            press(&mut mapper, KeyCode::Char('g')),
            Some(Action::DocumentStart)
        );
    }

    #[test]
    fn term_007_release_events_stay_inert_through_the_event_filter() {
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        let mut mapper = KeyMapper::default();

        assert!(action_from_event(&mut mapper, &release).is_none());
    }

    #[cfg(unix)]
    struct FaultPtyGuard {
        master: Option<Box<dyn portable_pty::MasterPty + Send>>,
        child: Box<dyn portable_pty::Child + Send + Sync>,
        reader: Option<std::thread::JoinHandle<()>>,
        output: Option<std::sync::mpsc::Receiver<io::Result<Vec<u8>>>>,
    }

    #[cfg(unix)]
    impl Drop for FaultPtyGuard {
        fn drop(&mut self) {
            if !matches!(self.child.try_wait(), Ok(Some(_))) {
                if self.child.kill().is_err() {
                    std::process::abort();
                }
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                let mut reaped = false;
                while std::time::Instant::now() < deadline {
                    if matches!(self.child.try_wait(), Ok(Some(_))) {
                        reaped = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                if !reaped {
                    std::process::abort();
                }
            }
            drop(self.master.take());
            if let Some(output) = self.output.take()
                && output.recv_timeout(Duration::from_secs(2)).is_err()
            {
                std::process::abort();
            }
            drop(self.reader.take());
        }
    }

    #[cfg(unix)]
    fn run_fault_in_pty(mode: &str) -> Result<(vt100::Screen, Vec<u8>)> {
        use std::{io::Read, time::Instant};

        use portable_pty::{CommandBuilder, PtySize, native_pty_system};

        let pair = native_pty_system().openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let initial_termios = format!("{:?}", pair.master.get_termios());
        let mut command = CommandBuilder::new(std::env::current_exe()?);
        command.env("TERM", "xterm-256color");
        command.env("TERMLEAF_TEST_TERMINAL_FAULT", mode);
        command.args([
            "--exact",
            "terminal::tests::terminal_fault_child",
            "--nocapture",
        ]);
        let child = pair.slave.spawn_command(command)?;
        drop(pair.slave);
        let mut source = pair.master.try_clone_reader()?;
        let (output_sender, output) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = match source.read_to_end(&mut output) {
                Ok(_) => Ok(output),
                Err(error) if error.raw_os_error() == Some(5) => Ok(output),
                Err(error) => Err(error),
            };
            let _send_result = output_sender.send(result);
        });
        let mut guard = FaultPtyGuard {
            master: Some(pair.master),
            child,
            reader: Some(reader),
            output: Some(output),
        };

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if guard.child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                bail!("controlled terminal fault child exceeded its timeout");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let final_termios = format!(
            "{:?}",
            guard
                .master
                .as_ref()
                .context("fault PTY master is present")?
                .get_termios()
        );
        assert_eq!(final_termios, initial_termios);
        drop(guard.master.take());
        let output = guard
            .output
            .take()
            .context("controlled terminal fault output receiver is present")?
            .recv_timeout(Duration::from_secs(2))
            .context("controlled terminal fault reader did not stop")??;
        drop(guard.reader.take());
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(&output);
        Ok((parser.screen().clone(), output))
    }

    #[cfg(unix)]
    fn position(output: &[u8], needle: &[u8]) -> Option<usize> {
        output
            .windows(needle.len())
            .position(|window| window == needle)
    }

    #[cfg(unix)]
    #[test]
    fn term_002_handled_active_error_restores_before_diagnostic() -> Result<()> {
        let (screen, output) = run_fault_in_pty("error")?;
        let leave = position(&output, b"\x1b[?1049l").context("leave alternate screen sequence")?;
        let diagnostic = position(&output, b"TermLeaf could not continue")
            .context("handled error diagnostic")?;

        assert!(diagnostic > leave);
        assert!(!screen.alternate_screen());
        assert!(!screen.hide_cursor());
        assert!(!screen.bracketed_paste());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn term_004_controlled_active_panic_restores_before_diagnostic() -> Result<()> {
        let (screen, output) = run_fault_in_pty("panic")?;
        let leave = position(&output, b"\x1b[?1049l").context("leave alternate screen sequence")?;
        let diagnostic = position(&output, b"TermLeaf stopped because of an internal error")
            .context("panic diagnostic")?;

        assert!(diagnostic > leave);
        assert!(!screen.alternate_screen());
        assert!(!screen.hide_cursor());
        assert!(!screen.bracketed_paste());
        assert!(!String::from_utf8_lossy(&output).contains("controlled active panic"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn terminal_fault_child() -> Result<()> {
        use std::process::ExitCode;

        let Ok(mode) = std::env::var("TERMLEAF_TEST_TERMINAL_FAULT") else {
            return Ok(());
        };
        let mut app = App::open(crate::app::StartupOptions::default())?;
        let status = crate::process::run_and_report(
            || {
                run_with_loop(&mut app, |terminal, app, _graphics| {
                    terminal.draw(|frame| ui::render(frame, app))?;
                    assert!(mode != "panic", "controlled active panic");
                    bail!("controlled active error")
                })
            },
            &mut io::stderr(),
        );
        assert_eq!(
            status,
            if mode == "panic" {
                ExitCode::from(101)
            } else {
                ExitCode::FAILURE
            }
        );
        Ok(())
    }
}
