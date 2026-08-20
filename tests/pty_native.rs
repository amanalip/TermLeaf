use std::{
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

impl PtyCase {
    fn spawn(arguments: &[&str]) -> Result<Self> {
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
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open native PTY")?;
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
        command.env("HOME", root.path());
        command.env("USERPROFILE", root.path());
        command.env("APPDATA", root.path().join("appdata"));
        command.env("LOCALAPPDATA", root.path().join("local-appdata"));
        command.env("TEMP", root.path().join("temp"));
        command.env("TMP", root.path().join("temp"));
        command.env("XDG_CONFIG_HOME", root.path().join("config"));
        command.env("XDG_DATA_HOME", root.path().join("data"));
        command.env("XDG_STATE_HOME", root.path().join("state"));
        command.env("XDG_CACHE_HOME", root.path().join("cache"));
        for argument in arguments {
            command.arg(argument);
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
            "timed out waiting for {expected:?}; output={:?}",
            String::from_utf8_lossy(&self.output)
        )
    }

    fn send(&mut self, input: &[u8]) -> Result<()> {
        let writer = self.writer.as_mut().context("PTY writer is present")?;
        writer.write_all(input).context("write PTY input")?;
        writer.flush().context("flush PTY input")
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
