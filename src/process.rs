use std::{
    io::{self, Write},
    panic::{self, AssertUnwindSafe},
    process::ExitCode,
    sync::Mutex,
};

use anyhow::Result;

use crate::{
    app::{App, StartupOptions},
    cli::Cli,
    interrupt,
    persistence::config::Settings,
    terminal,
    ui::theme::ThemeName,
};

static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

/// Builds and runs the application behind its process-level panic boundary.
#[must_use]
pub fn run(cli: Cli) -> ExitCode {
    interrupt::clear();
    run_and_report(
        || {
            interrupt::install()?;
            let settings = Settings::load();
            let app = App::open(StartupOptions {
                book: cli.book,
                theme: startup_theme(cli.theme.as_deref(), settings.theme.as_deref()),
            })?;
            terminal::run(app)
        },
        &mut io::stderr(),
    )
}

/// Applies configuration precedence: explicit option beats config.toml.
///
/// An unrecognized slug falls back to the default rather than blocking
/// startup; typed configuration errors arrive with the Phase 3 cases.
fn startup_theme(explicit: Option<&str>, configured: Option<&str>) -> ThemeName {
    [explicit, configured]
        .into_iter()
        .flatten()
        .find_map(ThemeName::parse)
        .unwrap_or_default()
}

pub(crate) fn run_and_report<F>(operation: F, diagnostics: &mut dyn Write) -> ExitCode
where
    F: FnOnce() -> Result<()>,
{
    let _hook_guard = match PANIC_HOOK_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    // The default hook writes before unwinding, while the alternate screen may
    // still be active. Worker threads will require a broader hook policy later.
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let outcome = panic::catch_unwind(AssertUnwindSafe(operation));
    panic::set_hook(previous_hook);

    match outcome {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            let _write_result = writeln!(
                diagnostics,
                "TermLeaf could not continue: {}",
                crate::document::sanitize_path(&error.to_string())
            );
            ExitCode::FAILURE
        }
        Err(_) => {
            let _write_result = writeln!(
                diagnostics,
                "TermLeaf stopped because of an internal error.\nTerminal restoration was attempted."
            );
            ExitCode::from(101)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;

    #[test]
    fn err_002_handled_error_is_reported_outside_the_operation() {
        let mut diagnostics = Vec::new();

        let status = run_and_report(
            || {
                bail!("controlled application error");
            },
            &mut diagnostics,
        );

        assert_eq!(status, ExitCode::FAILURE);
        assert_eq!(
            String::from_utf8_lossy(&diagnostics),
            "TermLeaf could not continue: controlled application error\n"
        );
    }

    #[test]
    fn term_004_panic_hook_is_silent_until_the_operation_unwinds() {
        let mut diagnostics = Vec::new();

        let status = run_and_report(
            || {
                panic!("controlled process panic");
            },
            &mut diagnostics,
        );

        assert_eq!(status, ExitCode::from(101));
        assert_eq!(
            String::from_utf8_lossy(&diagnostics),
            "TermLeaf stopped because of an internal error.\nTerminal restoration was attempted.\n"
        );
    }

    #[test]
    fn cfg_002_explicit_theme_overrides_config_which_overrides_the_default() {
        use crate::ui::theme::ThemeName;

        assert_eq!(
            startup_theme(Some("light"), Some("dark")),
            ThemeName::Light,
            "the command line wins"
        );
        assert_eq!(startup_theme(None, Some("dark")), ThemeName::Dark);
        assert_eq!(
            startup_theme(Some("monochrome"), None),
            ThemeName::Monochrome
        );
        assert_eq!(
            startup_theme(None, None),
            ThemeName::Paper,
            "no signal keeps the built-in default"
        );
        assert_eq!(
            startup_theme(None, Some("not-a-theme")),
            ThemeName::Paper,
            "an unrecognized configured slug falls back"
        );
        assert_eq!(startup_theme(Some("bogus"), Some("dark")), ThemeName::Dark);
    }
}
