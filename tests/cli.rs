use std::process::Command;

const TERMLEAF: &str = env!("CARGO_BIN_EXE_termleaf");

#[test]
fn cli_001_help_exits_before_terminal_initialization() {
    let output = Command::new(TERMLEAF)
        .arg("--help")
        .output()
        .expect("test process should launch");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: termleaf [BOOK]"));
    assert!(stdout.contains("Local book to open"));
    assert!(!stdout.contains('\u{1b}'));
}

#[test]
fn cli_002_version_matches_package_version() {
    let output = Command::new(TERMLEAF)
        .arg("--version")
        .output()
        .expect("test process should launch");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("termleaf ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn cli_005_missing_path_fails_without_terminal_sequences() {
    let output = Command::new(TERMLEAF)
        .arg("this-book-does-not-exist.txt")
        .output()
        .expect("test process should launch");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("this-book-does-not-exist.txt"));
    assert!(stderr.contains("check that the path exists and is readable"));
    assert!(!stderr.contains('\u{1b}'));
}
