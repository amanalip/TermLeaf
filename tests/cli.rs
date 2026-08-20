use std::{
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

const TERMLEAF: &str = env!("CARGO_BIN_EXE_termleaf");
const CASE_TIMEOUT: Duration = Duration::from_secs(10);

fn run(configure: impl FnOnce(&mut Command, &Path)) -> Result<Output> {
    let root = tempfile::tempdir().context("create isolated CLI root")?;
    let mut command = Command::new(TERMLEAF);
    command
        .env_clear()
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut command, root.path());
    let mut child = command.spawn().context("launch TermLeaf test process")?;
    let deadline = Instant::now() + CASE_TIMEOUT;
    loop {
        if child
            .try_wait()
            .context("poll TermLeaf test process")?
            .is_some()
        {
            return child.wait_with_output().context("collect TermLeaf output");
        }
        if Instant::now() >= deadline {
            child.kill().context("kill timed-out TermLeaf process")?;
            child.wait().context("reap timed-out TermLeaf process")?;
            bail!("TermLeaf process exceeded the 10-second case timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn cli_001_help_exits_before_terminal_initialization() -> Result<()> {
    let output = run(|command, _root| {
        command.arg("--help");
    })?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: termleaf [BOOK]"));
    assert!(stdout.contains("Local book to open"));
    assert!(!stdout.contains('\u{1b}'));
    Ok(())
}

#[test]
fn cli_002_version_matches_package_version() -> Result<()> {
    let output = run(|command, _root| {
        command.arg("--version");
    })?;

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("termleaf ", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn cli_005_missing_path_fails_without_terminal_sequences() -> Result<()> {
    let output = run(|command, root| {
        command.arg(root.join("this-book-does-not-exist.txt"));
    })?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("this-book-does-not-exist.txt"));
    assert!(stderr.contains("check that the path exists and is readable"));
    assert!(!stderr.contains('\u{1b}'));
    Ok(())
}
