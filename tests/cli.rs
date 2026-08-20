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
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().context("collect TermLeaf output"),
            Ok(None) => {}
            Err(error) => {
                terminate_or_abort(&mut child);
                return Err(error).context("poll TermLeaf test process");
            }
        }
        if Instant::now() >= deadline {
            terminate_or_abort(&mut child);
            bail!("TermLeaf process exceeded the 10-second case timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_or_abort(child: &mut std::process::Child) {
    if child.kill().is_err() {
        std::process::abort();
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    std::process::abort();
}

#[test]
fn cli_001_help_exits_before_terminal_initialization() -> Result<()> {
    let output = run(|command, _root| {
        command.arg("--help");
    })?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("[BOOK]"));
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

#[cfg(unix)]
#[test]
fn cli_006_unreadable_path_fails_before_terminal_initialization() -> Result<()> {
    use std::{fs, os::unix::fs::PermissionsExt};

    let output = run(|command, root| {
        let path = root.join("unreadable-book.txt");
        fs::write(&path, "unreadable").expect("create unreadable test book");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
            .expect("remove test book permissions");
        command.arg(path);
    })?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unreadable-book.txt"));
    assert!(stderr.contains("file is not readable"));
    assert!(!stderr.contains('\u{1b}'));
    Ok(())
}
