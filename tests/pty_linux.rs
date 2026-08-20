#![cfg(unix)]

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
    reader: Option<JoinHandle<Result<()>>>,
    parser: vt100::Parser,
    output: Vec<u8>,
    deadline: Instant,
    initial_termios: String,
    finished: bool,
}

impl PtyCase {
    fn spawn(arguments: &[&str]) -> Result<Self> {
        let serial = match PTY_CASE_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let root = tempfile::tempdir().context("create isolated PTY root")?;
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
        command.env("TERM", "xterm-256color");
        command.env("LANG", "C.UTF-8");
        command.env("LC_ALL", "C.UTF-8");
        command.env("HOME", root.path());
        command.env("XDG_CONFIG_HOME", root.path().join("config"));
        command.env("XDG_DATA_HOME", root.path().join("data"));
        command.env("XDG_STATE_HOME", root.path().join("state"));
        command.env("XDG_CACHE_HOME", root.path().join("cache"));
        for argument in arguments {
            command.arg(argument);
        }

        let initial_termios = format!("{:?}", pair.master.get_termios());
        let child = pair
            .slave
            .spawn_command(command)
            .context("spawn TermLeaf in PTY")?;
        drop(pair.slave);
        let mut source = pair.master.try_clone_reader().context("clone PTY reader")?;
        let writer = pair.master.take_writer().context("take PTY writer")?;
        let (sender, chunks) = mpsc::channel();
        let reader = std::thread::spawn(move || -> Result<()> {
            let mut buffer = [0_u8; 4096];
            loop {
                match source.read(&mut buffer) {
                    Ok(0) => return Ok(()),
                    Ok(count) => {
                        if sender.send(buffer[..count].to_vec()).is_err() {
                            return Ok(());
                        }
                    }
                    Err(error) if error.raw_os_error() == Some(5) => return Ok(()),
                    Err(error) => return Err(error).context("read PTY output"),
                }
            }
        });

        Ok(Self {
            _serial: serial,
            _root: root,
            master: Some(pair.master),
            child,
            writer: Some(writer),
            chunks,
            reader: Some(reader),
            parser: vt100::Parser::new(24, 80, 0),
            output: Vec::new(),
            deadline: Instant::now() + CASE_TIMEOUT,
            initial_termios,
            finished: false,
        })
    }

    fn wait_for_text(&mut self, expected: &str) -> Result<()> {
        while Instant::now() < self.deadline {
            self.receive_chunk(Duration::from_millis(50));
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
        bail!("timed out waiting for {expected:?}")
    }

    fn send(&mut self, input: &[u8]) -> Result<()> {
        let writer = self.writer.as_mut().context("PTY writer is present")?;
        writer.write_all(input).context("write PTY input")?;
        writer.flush().context("flush PTY input")
    }

    fn raw_mode_changed_termios(&self) -> Result<bool> {
        let master = self.master.as_ref().context("PTY master is present")?;
        Ok(format!("{:?}", master.get_termios()) != self.initial_termios)
    }

    fn finish(mut self) -> Result<(portable_pty::ExitStatus, vt100::Screen, Vec<u8>)> {
        let status = loop {
            self.receive_chunk(Duration::from_millis(20));
            if let Some(status) = self.child.try_wait().context("poll PTY child")? {
                break status;
            }
            if Instant::now() >= self.deadline {
                self.kill_and_reap()?;
                bail!("PTY child exceeded the case timeout")
            }
        };

        let final_termios = format!(
            "{:?}",
            self.master
                .as_ref()
                .context("PTY master is present")?
                .get_termios()
        );
        if final_termios != self.initial_termios {
            bail!("terminal attributes were not restored after child exit")
        }

        drop(self.writer.take());
        drop(self.master.take());
        self.reader
            .take()
            .context("PTY reader thread is present")?
            .join()
            .map_err(|_| anyhow::anyhow!("PTY reader thread panicked"))??;
        while let Ok(chunk) = self.chunks.try_recv() {
            self.process(&chunk);
        }
        self.finished = true;
        Ok((status, self.parser.screen().clone(), self.output.clone()))
    }

    fn receive_chunk(&mut self, timeout: Duration) {
        match self.chunks.recv_timeout(timeout) {
            Ok(chunk) => self.process(&chunk),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }

    fn process(&mut self, chunk: &[u8]) {
        self.parser.process(chunk);
        self.output.extend_from_slice(chunk);
    }

    fn kill_and_reap(&mut self) -> Result<()> {
        self.child.kill().context("kill timed-out PTY child")?;
        self.child.wait().context("reap timed-out PTY child")?;
        Ok(())
    }
}

impl Drop for PtyCase {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _kill_result = self.child.kill();
            let _wait_result = self.child.wait();
        }
        drop(self.writer.take());
        drop(self.master.take());
        if let Some(reader) = self.reader.take() {
            let _join_result = reader.join();
        }
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
    assert!(case.raw_mode_changed_termios()?);
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
fn cli_010_pre_terminal_error_emits_no_control_sequences() -> Result<()> {
    let missing = Path::new("definitely-missing-phase-zero-book.txt");
    let case = PtyCase::spawn(&[missing.to_str().context("missing path is UTF-8")?])?;

    let (status, screen, output) = case.finish()?;

    assert!(!status.success());
    assert!(!output.contains(&0x1b));
    assert!(!screen.alternate_screen());
    let diagnostic = String::from_utf8_lossy(&output);
    assert!(diagnostic.contains("definitely-missing-phase-zero-book.txt"));
    assert!(diagnostic.contains("check that the path exists and is readable"));
    Ok(())
}
