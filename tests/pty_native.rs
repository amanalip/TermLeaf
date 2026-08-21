use std::{
    fmt::Write as _,
    io::{Read, Write},
    path::Path,
    sync::mpsc::{self, Receiver},
    sync::{Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tempfile::TempDir;

const TERMLEAF: &str = env!("CARGO_BIN_EXE_termleaf");
const CASE_TIMEOUT: Duration = Duration::from_secs(10);
static PTY_CASE_LOCK: Mutex<()> = Mutex::new(());

struct PtyCase {
    _serial: MutexGuard<'static, ()>,
    _root: TempDir,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Box<dyn Child + Send + Sync>,
    writer: Option<Box<dyn Write + Send>>,
    chunks: Receiver<Vec<u8>>,
    reader: Option<JoinHandle<()>>,
    reader_done: Option<Receiver<Result<()>>>,
    parser: vt100::Parser,
    output: Vec<u8>,
    deadline: Instant,
    initial_terminal_state: TerminalState,
    #[cfg(windows)]
    cursor_position_reported: bool,
    finished: bool,
}

fn isolated_command(root: &Path, arguments: &[String]) -> CommandBuilder {
    let mut command = CommandBuilder::new(TERMLEAF);
    command.env_clear();
    #[cfg(windows)]
    for key in [
        "ComSpec",
        "OS",
        "PATH",
        "PATHEXT",
        "SystemDrive",
        "SystemRoot",
        "WINDIR",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.env("TERM", "xterm-256color");
    command.env("LANG", "C.UTF-8");
    command.env("LC_ALL", "C.UTF-8");
    command.env("HOME", root);
    command.env("USERPROFILE", root);
    command.env("APPDATA", root.join("appdata"));
    command.env("LOCALAPPDATA", root.join("local-appdata"));
    command.env("TEMP", root.join("temp"));
    command.env("TMP", root.join("temp"));
    command.env("XDG_CONFIG_HOME", root.join("config"));
    command.env("XDG_DATA_HOME", root.join("data"));
    command.env("XDG_STATE_HOME", root.join("state"));
    command.env("XDG_CACHE_HOME", root.join("cache"));
    for argument in arguments {
        command.arg(argument);
    }
    command
}

impl PtyCase {
    fn spawn(arguments: &[&str]) -> Result<Self> {
        Self::spawn_with_env(
            |_| {
                arguments
                    .iter()
                    .map(|argument| (*argument).to_owned())
                    .collect()
            },
            &[],
        )
    }

    /// Spawns a session whose fixture files are created inside the isolated
    /// root; the setup closure returns absolute argument paths so the child
    /// never depends on its inherited working directory.
    fn spawn_with(setup: impl FnOnce(&Path) -> Vec<String>) -> Result<Self> {
        Self::spawn_with_env(setup, &[])
    }

    /// Like [`PtyCase::spawn_with`] but records extra child environment
    /// variables the case deliberately varies (locale rows, for example).
    /// Every extra variable is part of the harness contract's allowlist note.
    fn spawn_with_env(
        setup: impl FnOnce(&Path) -> Vec<String>,
        extra_environment: &[(&str, &str)],
    ) -> Result<Self> {
        let serial = match PTY_CASE_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let root = tempfile::tempdir().context("create isolated PTY root")?;
        for directory in [
            "appdata",
            "local-appdata",
            "temp",
            "config",
            "data",
            "state",
            "cache",
        ] {
            std::fs::create_dir_all(root.path().join(directory))
                .context("create isolated PTY directory")?;
        }
        let arguments = setup(root.path());
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open native PTY")?;
        let mut command = isolated_command(root.path(), &arguments);
        for (key, value) in extra_environment {
            command.env(key, value);
        }

        let initial_terminal_state = terminal_state(pair.master.as_ref());
        let child = pair
            .slave
            .spawn_command(command)
            .context("spawn TermLeaf in PTY")?;
        drop(pair.slave);
        let mut source = pair.master.try_clone_reader().context("clone PTY reader")?;
        let writer = pair.master.take_writer().context("take PTY writer")?;
        let (sender, chunks) = mpsc::channel();
        let (done_sender, reader_done) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            let result = loop {
                match source.read(&mut buffer) {
                    Ok(0) => break Ok(()),
                    Ok(count) => {
                        if sender.send(buffer[..count].to_vec()).is_err() {
                            break Ok(());
                        }
                    }
                    Err(error) if is_pty_eof(&error) => break Ok(()),
                    Err(error) => break Err(error).context("read PTY output"),
                }
            };
            let _send_result = done_sender.send(result);
        });

        Ok(Self {
            _serial: serial,
            _root: root,
            master: Some(pair.master),
            child,
            writer: Some(writer),
            chunks,
            reader: Some(reader),
            reader_done: Some(reader_done),
            parser: vt100::Parser::new(24, 80, 0),
            output: Vec::new(),
            deadline: Instant::now() + CASE_TIMEOUT,
            initial_terminal_state,
            #[cfg(windows)]
            cursor_position_reported: false,
            finished: false,
        })
    }

    fn wait_for_text(&mut self, expected: &str) -> Result<()> {
        while Instant::now() < self.deadline {
            self.receive_chunk(Duration::from_millis(50))?;
            if self.parser.screen().contents().contains(expected) {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait().context("poll PTY child")? {
                bail!(
                    "child exited before rendering {expected:?}: {status:?}; output={:?}",
                    String::from_utf8_lossy(&self.output)
                );
            }
        }
        self.kill_and_reap()?;
        bail!(
            "timed out waiting for {expected:?}; screen={:?} output={:?}",
            self.parser.screen().contents(),
            String::from_utf8_lossy(&self.output)
        )
    }

    fn send(&mut self, input: &[u8]) -> Result<()> {
        let writer = self.writer.as_mut().context("PTY writer is present")?;
        writer.write_all(input).context("write PTY input")?;
        writer.flush().context("flush PTY input")
    }

    /// Resizes the PTY and restarts the terminal model at the same geometry.
    ///
    /// The model starts empty; later draws refill it, which keeps every
    /// subsequent assertion about newly rendered content honest.
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let master = self.master.as_ref().context("PTY master is present")?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("resize PTY")?;
        self.parser = vt100::Parser::new(rows.max(1), cols.max(1), 0);
        Ok(())
    }

    /// The logical line number currently shown in the status line.
    fn status_location(screen: &vt100::Screen) -> Option<u32> {
        let contents = screen.contents();
        let mut search_from = 0usize;
        while let Some(found) = contents[search_from..].find("Loc ") {
            let digits = search_from + found + "Loc ".len();
            let end = contents[digits..]
                .chars()
                .take_while(char::is_ascii_digit)
                .map(char::len_utf8)
                .sum::<usize>()
                + digits;
            if end > digits {
                return contents[digits..end].parse::<u32>().ok();
            }
            search_from = digits;
        }
        None
    }

    /// Waits until the status location satisfies `matches`.
    fn wait_for_location(&mut self, describe: &str, matches: impl Fn(u32) -> bool) -> Result<()> {
        while Instant::now() < self.deadline {
            self.receive_chunk(Duration::from_millis(50))?;
            if let Some(location) = Self::status_location(self.parser.screen())
                && matches(location)
            {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait().context("poll PTY child")? {
                bail!(
                    "child exited before location {describe}: {status:?}; output={:?}",
                    String::from_utf8_lossy(&self.output)
                );
            }
        }
        self.kill_and_reap()?;
        bail!(
            "timed out waiting for location {describe}; screen={:?}",
            self.parser.screen().contents()
        )
    }

    #[cfg(unix)]
    fn raw_mode_changed_terminal_state(&self) -> Result<bool> {
        let master = self.master.as_ref().context("PTY master is present")?;
        Ok(terminal_state(master.as_ref()) != self.initial_terminal_state)
    }

    fn finish(mut self) -> Result<(portable_pty::ExitStatus, vt100::Screen, Vec<u8>)> {
        let status = loop {
            self.receive_chunk(Duration::from_millis(20))?;
            if let Some(status) = self.child.try_wait().context("poll PTY child")? {
                break status;
            }
            if Instant::now() >= self.deadline {
                self.kill_and_reap()?;
                bail!(
                    "PTY child exceeded the case timeout; output={:?}",
                    String::from_utf8_lossy(&self.output)
                )
            }
        };

        let final_terminal_state = terminal_state(
            self.master
                .as_ref()
                .context("PTY master is present")?
                .as_ref(),
        );
        if final_terminal_state != self.initial_terminal_state {
            bail!("terminal attributes were not restored after child exit")
        }

        drop(self.writer.take());
        drop(self.master.take());
        self.finish_reader()?;
        while let Ok(chunk) = self.chunks.try_recv() {
            self.process(&chunk);
        }
        self.finished = true;
        Ok((status, self.parser.screen().clone(), self.output.clone()))
    }

    #[cfg_attr(not(windows), allow(clippy::unnecessary_wraps))]
    fn receive_chunk(&mut self, timeout: Duration) -> Result<()> {
        match self.chunks.recv_timeout(timeout) {
            Ok(chunk) => self.process(&chunk),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
        #[cfg(windows)]
        {
            if !self.cursor_position_reported
                && self.output.windows(4).any(|window| window == b"\x1b[6n")
            {
                self.send(b"\x1b[1;1R")?;
                self.cursor_position_reported = true;
            }
            for sequence in [b"\x1b[6n".as_slice(), b"\x1b[1;1R".as_slice()] {
                while let Some(start) = self
                    .output
                    .windows(sequence.len())
                    .position(|window| window == sequence)
                {
                    self.output.drain(start..start + sequence.len());
                }
            }
            self.parser = vt100::Parser::new(24, 80, 0);
            self.parser.process(&self.output);
        }
        Ok(())
    }

    fn process(&mut self, chunk: &[u8]) {
        self.parser.process(chunk);
        self.output.extend_from_slice(chunk);
    }

    fn kill_and_reap(&mut self) -> Result<()> {
        if self.child.kill().is_err() {
            std::process::abort();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self
                .child
                .try_wait()
                .context("reap timed-out PTY child")?
                .is_some()
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        std::process::abort();
    }

    fn finish_reader(&mut self) -> Result<()> {
        let result = self
            .reader_done
            .take()
            .context("PTY reader completion receiver is present")?
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_else(|_| std::process::abort());
        result?;
        drop(self.reader.take());
        Ok(())
    }
}

#[cfg(unix)]
type TerminalState = String;

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalState;

#[cfg(unix)]
fn terminal_state(master: &dyn MasterPty) -> TerminalState {
    format!("{:?}", master.get_termios())
}

#[cfg(not(unix))]
fn terminal_state(_master: &dyn MasterPty) -> TerminalState {
    TerminalState
}

fn is_pty_eof(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof
    ) || cfg!(unix) && error.raw_os_error() == Some(5)
}

impl Drop for PtyCase {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            if self.child.kill().is_err() {
                std::process::abort();
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut reaped = false;
            while Instant::now() < deadline {
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
        drop(self.writer.take());
        drop(self.master.take());
        if let Some(reader_done) = self.reader_done.take()
            && reader_done.recv_timeout(Duration::from_secs(2)).is_err()
        {
            std::process::abort();
        }
        drop(self.reader.take());
    }
}

fn assert_restored(screen: &vt100::Screen, output: &[u8]) {
    assert!(!screen.alternate_screen());
    assert!(!screen.hide_cursor());
    assert!(!screen.bracketed_paste());
    assert!(output.windows(8).any(|bytes| bytes == b"\x1b[?1049l"));
}

#[test]
fn cli_004_launch_without_path_opens_recent_books() -> Result<()> {
    let mut case = PtyCase::spawn(&[])?;

    case.wait_for_text("Recent books")?;
    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn key_001_reader_keys_navigate_help_and_quit_inside_a_pty() -> Result<()> {
    let mut book = String::new();
    for index in 1..=60 {
        let _ = writeln!(book, "journey paragraph {index:02} carries readable words");
    }
    let mut case = PtyCase::spawn_with(|root| {
        let path = root.join("journey-book.txt");
        std::fs::write(&path, book).expect("write journey book");
        vec![path.display().to_string()]
    })?;

    case.wait_for_text("journey paragraph 01")?;
    assert!(case.parser.screen().alternate_screen());
    case.wait_for_location("= 1 at open", |location| location == 1)?;

    case.send(b"\x1b[B")?; // Down
    case.wait_for_location("Down moves to line 2", |location| location == 2)?;
    case.send(b"\x1b[A")?; // Up
    case.wait_for_location("Up returns to line 1", |location| location == 1)?;

    case.send(b"\x1b[6~")?; // PageDown
    case.wait_for_location("PageDown leaves the first page", |location| location > 1)?;
    case.send(b"\x1b[5~")?; // PageUp
    case.wait_for_location("PageUp returns toward the start", |location| location == 1)?;

    case.send(b"G")?; // Jump to the end of the book.
    case.wait_for_text("journey paragraph 60")?;

    case.send(b"?")?; // Help overlays the current passage.
    case.wait_for_text("Reader commands")?;
    case.send(b"\x1b")?; // Back returns to the same logical passage...
    std::thread::sleep(Duration::from_millis(100));
    case.send(b"\x1b[H")?; // ...conventional keys still reach the reader.
    case.wait_for_location("Home anchors the book start", |location| location == 1)?;

    case.send(b"\x1b[F")?; // End jumps to the end like G.
    case.wait_for_location("End anchors the book end", |location| location >= 60)?;
    case.wait_for_text("journey paragraph 60")?;

    case.send(b"\x1bOP")?; // F1 opens help through its function key.
    case.wait_for_text("Reader commands")?;
    case.send(b"\x1b")?; // Escape closes it again.
    std::thread::sleep(Duration::from_millis(100));

    case.send(b"G")?;
    case.wait_for_text("journey paragraph 60")?;
    std::thread::sleep(Duration::from_millis(100));
    case.send(b"gg")?; // The multikey prefix completes across PTY reads.
    case.wait_for_location("gg anchors the book start", |location| location == 1)?;

    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn key_002_flow_control_keys_page_without_colliding_in_a_pty() -> Result<()> {
    let mut book = String::new();
    for index in 1..=60 {
        let _ = writeln!(book, "journey paragraph {index:02} carries readable words");
    }
    let mut case = PtyCase::spawn_with(|root| {
        let path = root.join("flow-book.txt");
        std::fs::write(&path, book).expect("write flow book");
        vec![path.display().to_string()]
    })?;

    case.wait_for_text("journey paragraph 01")?;
    // Raw mode disables terminal flow control, so Ctrl-F/Ctrl-B reach the
    // reader as page keys exactly as the binding registry promises.
    case.send(b"\x06")?; // Ctrl-F: next page.
    case.wait_for_location("Ctrl-F pages forward", |location| location > 1)?;
    let paged = PtyCase::status_location(case.parser.screen()).context("status after Ctrl-F")?;

    case.send(b"\x02")?; // Ctrl-B: previous page.
    case.wait_for_location("Ctrl-B pages back to the start", |location| location == 1)?;

    case.send(b"\x06")?; // Forward again lands on the same anchor.
    case.wait_for_location("Ctrl-F reproduces its prior anchor", |location| {
        location == paged
    })?;

    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn key_006_escape_alt_ambiguity_and_ctrl_c_stay_safe_in_a_pty() -> Result<()> {
    let mut book = String::new();
    for index in 1..=60 {
        let _ = writeln!(book, "journey paragraph {index:02} carries readable words");
    }
    let mut case = PtyCase::spawn_with(|root| {
        let path = root.join("escape-book.txt");
        std::fs::write(&path, book).expect("write escape book");
        vec![path.display().to_string()]
    })?;

    case.wait_for_text("journey paragraph 01")?;
    case.send(b"?")?; // Help gives Back an observable effect.
    case.wait_for_text("Reader commands")?;

    // ESC followed by a letter in one write is an Alt chord, not Back plus
    // that letter: help must stay open and nothing else may fire.
    case.send(b"\x1bx")?;
    std::thread::sleep(Duration::from_millis(200));
    case.receive_chunk(Duration::from_millis(200))?;
    assert!(
        case.parser.screen().contents().contains("Reader commands"),
        "an Alt chord must not act as Back: {:?}",
        case.parser.screen().contents()
    );

    // A lone ESC in its own write is Back and closes help.
    case.send(b"\x1b")?;
    case.wait_for_text("journey paragraph 01")?;
    case.wait_for_location("help return keeps the passage", |location| location == 1)?;

    // Ctrl-C during ordinary reading quits cleanly; text-entry modes do not
    // exist yet, so this pins the Phase 1 scope of KEY-006.
    case.send(b"\x03")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

// ConPTY's input pipeline parses the bracketed-paste markers themselves and
// forwards the inner bytes as ordinary keystrokes (observed in CI run
// 32535423048: 't' opened Themes and 'c' switched modes), so this transport-
// level journey cannot run there. Paste-event inertness is proved on every
// platform by term_007's event-filter unit cases; real Windows terminals
// deliver markers correctly and stay covered by the release matrix.
#[cfg(not(windows))]
#[test]
fn key_007_paste_events_are_inert_in_every_phase_one_mode() -> Result<()> {
    let pasted = {
        let mut payload = b"\x1b[200~plain paste\r\nmultiline\x03control \x1b[6~".to_vec();
        payload.extend(std::iter::repeat_n(b'a', 65_536));
        payload.extend_from_slice(b"\x1b[201~");
        payload
    };

    // Reader mode: paste content never becomes keys or navigation.
    let mut book = String::new();
    for index in 1..=60 {
        let _ = writeln!(book, "journey paragraph {index:02} carries readable words");
    }
    let mut case = PtyCase::spawn_with(|root| {
        let path = root.join("paste-book.txt");
        std::fs::write(&path, book).expect("write paste book");
        vec![path.display().to_string()]
    })?;
    case.wait_for_text("journey paragraph 01")?;

    case.send(&pasted)?;
    // The embedded Ctrl-C and PageDown bytes arrive inside the bracketed
    // paste, so the application must stay alive, unmoved, and free of the
    // pasted content.
    std::thread::sleep(Duration::from_millis(300));
    case.receive_chunk(Duration::from_millis(300))?;
    let contents = case.parser.screen().contents();
    assert!(
        !contents.contains("plain paste"),
        "paste leaked to the screen"
    );
    assert!(
        contents.contains("journey paragraph 01"),
        "paste moved away from the anchor: {contents}"
    );
    case.wait_for_location("paste keeps the anchor", |location| location == 1)?;

    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;
    assert!(status.success(), "the app survives oversized pastes");
    assert_restored(&screen, &output);

    // Recent-books mode receives the same treatment.
    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("Recent books")?;
    case.send(&pasted)?;
    std::thread::sleep(Duration::from_millis(300));
    case.receive_chunk(Duration::from_millis(300))?;
    let contents = case.parser.screen().contents();
    assert!(
        !contents.contains("plain paste"),
        "paste leaked to the home screen"
    );
    assert!(
        contents.contains("Recent books"),
        "paste disturbed the home screen: {contents}"
    );

    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;
    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn term_006_resize_transients_recover_to_the_same_anchor() -> Result<()> {
    let mut book = String::new();
    for index in 1..=60 {
        let _ = writeln!(book, "journey paragraph {index:02} carries readable words");
    }
    let mut case = PtyCase::spawn_with(|root| {
        let path = root.join("resize-book.txt");
        std::fs::write(&path, book).expect("write resize book");
        vec![path.display().to_string()]
    })?;

    case.wait_for_text("journey paragraph 01")?;
    case.send(b"G")?;
    case.wait_for_location("the jump reaches the end", |location| location >= 60)?;

    // Rewrapping may split any phrase across visual rows, so anchor checks
    // after resize use the width-independent logical location.
    case.resize(40, 10)?;
    case.wait_for_location("narrow keeps the end anchor", |location| location >= 60)?;

    case.resize(8, 2)?; // Tiny transient below the usable minimum.
    // At this geometry even the suspension notice truncates to its first
    // word; the full message is asserted by lay_012 at a readable size.
    case.wait_for_text("Terminal")?;

    case.resize(80, 24)?; // Recovery restores the same logical passage.
    case.wait_for_location("recovered keeps the end anchor", |location| location >= 60)?;

    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn theme_002_configured_theme_loads_and_session_switch_reports_over_a_pty() -> Result<()> {
    let mut case = PtyCase::spawn_with(|root| {
        let path = termleaf::persistence::config::path_under(&root.join("config"));
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("config dir");
        std::fs::write(path, "theme = \"dark\"\n").expect("write startup config");
        vec![]
    })?;

    case.wait_for_text("No recent books yet.")?;
    case.send(b"t")?;
    case.wait_for_text("Themes")?;
    let contents = case.parser.screen().contents();
    assert!(
        contents.contains("> Dark  (applied)"),
        "the configured theme starts selected and applied: {contents}"
    );

    case.send(b"\r")?; // Enter applies the selection and reports it.
    case.wait_for_text("Theme: Dark")?;
    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn term_001_q_restores_terminal_modes() -> Result<()> {
    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("No recent books yet.")?;

    assert!(case.parser.screen().alternate_screen());
    assert!(case.parser.screen().hide_cursor());
    assert!(case.parser.screen().bracketed_paste());
    #[cfg(unix)]
    assert!(case.raw_mode_changed_terminal_state()?);
    case.send(b"q")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn term_003_ctrl_c_key_restores_terminal_modes() -> Result<()> {
    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("No recent books yet.")?;

    case.send(b"\x03")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[cfg(unix)]
#[test]
fn term_011_external_sigint_restores_terminal_modes() -> Result<()> {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };

    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("No recent books yet.")?;
    let process_id = case.child.process_id().context("PTY child process ID")?;
    let process_id = i32::try_from(process_id).context("PTY process ID fits i32")?;

    kill(Pid::from_raw(process_id), Signal::SIGINT).context("send SIGINT")?;
    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn term_008_vt100_models_balanced_lifecycle_output() -> Result<()> {
    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("Recent books")?;
    case.send(b"q")?;

    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert!(output.windows(8).any(|bytes| bytes == b"\x1b[?1049h"));
    assert!(output.windows(8).any(|bytes| bytes == b"\x1b[?1049l"));
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn term_012_supported_launch_baseline_is_restored() -> Result<()> {
    let mut case = PtyCase::spawn(&[])?;
    case.wait_for_text("Recent books")?;
    case.send(b"q")?;

    let (status, screen, output) = case.finish()?;

    assert!(status.success());
    assert_restored(&screen, &output);
    Ok(())
}

#[test]
fn cli_010_pre_terminal_error_emits_no_control_sequences() -> Result<()> {
    let missing = Path::new("definitely-missing-phase-zero-book.txt");
    let case = PtyCase::spawn(&[missing.to_str().context("missing path is UTF-8")?])?;

    let (status, screen, output) = case.finish()?;

    assert!(!status.success());
    #[cfg(unix)]
    assert!(!output.contains(&0x1b), "output={output:?}");
    #[cfg(windows)]
    for sequence in [b"\x1b[?1049h".as_slice(), b"\x1b[?25l", b"\x1b[?2004h"] {
        assert!(
            !output
                .windows(sequence.len())
                .any(|bytes| bytes == sequence),
            "application setup sequence {sequence:?} appeared in output={output:?}"
        );
    }
    assert!(!screen.alternate_screen());
    let diagnostic = String::from_utf8_lossy(&output);
    assert!(diagnostic.contains("definitely-missing-phase-zero-book.txt"));
    assert!(diagnostic.contains("check that the path exists and is readable"));
    Ok(())
}

#[test]
fn lay_015_locale_variants_render_identical_unicode() -> Result<()> {
    let mut book = String::new();
    for index in 1..=20 {
        let _ = writeln!(book, "locale paragraph {index:02} 漢字 readable words");
    }
    let spawn = |environment: &[(&str, &str)]| {
        PtyCase::spawn_with_env(
            |root| {
                let path = root.join("locale-book.txt");
                std::fs::write(&path, &book).expect("write locale book");
                vec![path.display().to_string()]
            },
            environment,
        )
    };

    // The harness default is C.UTF-8; a bare C and a UTF-8 English locale
    // must render the same Unicode without byte corruption. TermLeaf never
    // consults the locale: decoding is explicit and layout is pure Rust.
    for label in ["C", "en_US.UTF-8"] {
        let mut case = spawn(&[("LC_ALL", label), ("LANG", label)])?;
        case.wait_for_text("locale paragraph 01")?;
        case.wait_for_text("漢字")?;
        case.send(b"G")?;
        case.wait_for_location("the end of the locale book", |location| location >= 20)?;
        case.send(b"q")?;
        let (status, screen, output) = case.finish()?;
        assert!(status.success(), "{label} journey must exit cleanly");
        assert_restored(&screen, &output);
    }
    Ok(())
}
