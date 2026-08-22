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
    assert!(stdout.contains("--theme"), "the theme option is documented");
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

#[test]
fn txt_008_file_above_the_byte_limit_fails_before_terminal_setup() -> Result<()> {
    let output = run(|command, root| {
        let path = root.join("oversized-book.txt");
        let file = std::fs::File::create(&path).expect("create oversized test book");
        file.set_len(termleaf::document::TextLimits::default().max_bytes + 1)
            .expect("grow test book past the limit");
        command.arg(path);
    })?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("book is too large"), "{stderr}");
    assert!(
        stderr.contains("limit is 33554432 bytes"),
        "the default limit names its byte count: {stderr}"
    );
    assert!(!stderr.contains('\u{1b}'));
    Ok(())
}
#[test]
fn theme_002_startup_config_is_read_without_being_rewritten() -> Result<()> {
    let config_root = tempfile::tempdir()?;
    let output = run(|command, root| {
        let path = termleaf::persistence::config::path_under(config_root.path());
        std::fs::create_dir_all(path.parent().expect("config parent"))
            .expect("create config directory");
        std::fs::write(&path, "theme = \"monochrome\"\n").expect("write startup config");
        // A missing book keeps the journey short; settings still load first.
        // Point the child's XDG_CONFIG_HOME at the longer-lived directory so
        // the fixture survives past the harness root teardown.
        command.env("XDG_CONFIG_HOME", config_root.path());
        command.arg(root.join("missing-book.txt"));
    })?;

    assert!(!output.status.success());
    let contents = std::fs::read_to_string(termleaf::persistence::config::path_under(
        config_root.path(),
    ))
    .expect("config survives the run");
    assert_eq!(
        contents, "theme = \"monochrome\"\n",
        "startup loads config without rewriting it"
    );
    Ok(())
}

#[test]
fn cli_007_unsupported_extensions_reject_with_one_typed_message() -> Result<()> {
    // Extension-first detection (DEC-TEST-001 / DD-024): `.txt` and `.epub`
    // open through their adapters; every other extension rejects before any
    // terminal setup, regardless of what the content looks like.
    for name in ["notes.md", "future-book.mobi", "extensionless-book"] {
        let output = run(|command, root| {
            let path = root.join(name);
            std::fs::write(&path, "perfectly valid text\n").expect("write misleading book");
            command.arg(path);
        })?;

        assert!(!output.status.success(), "{name} must not open");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unsupported book format"), "{stderr}");
        assert!(stderr.contains(".txt"), "{stderr}");
        assert!(stderr.contains(".epub"), "{stderr}");
        assert!(!stderr.contains('\u{1b}'));
    }
    Ok(())
}

#[test]
fn cli_007_epub_content_still_validates_after_the_extension_gate() -> Result<()> {
    let output = run(|command, root| {
        let path = root.join("not-really.epub");
        std::fs::write(&path, "plain text wearing an epub name\n").expect("write misleading book");
        command.arg(path);
    })?;

    // An .epub extension reaches the EPUB adapter, so the failure is the
    // typed archive error rather than a format rejection.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("EPUB archive"), "{stderr}");
    assert!(stderr.contains("damaged"), "{stderr}");
    assert!(!stderr.contains("unsupported book format"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'));
    Ok(())
}

#[test]
fn cli_007_txt_content_still_validates_after_the_extension_gate() -> Result<()> {
    let output = run(|command, root| {
        let path = root.join("binary-book.txt");
        std::fs::write(&path, [0x68, 0xFF, 0x6F]).expect("write invalid UTF-8 bytes");
        command.arg(path);
    })?;

    // A .txt extension gets the text decoder, so the failure is the typed
    // encoding error rather than a format rejection.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("could not read"), "{stderr}");
    assert!(stderr.contains("invalid UTF-8 sequence"), "{stderr}");
    assert!(!stderr.contains("unsupported book format"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'));
    Ok(())
}
