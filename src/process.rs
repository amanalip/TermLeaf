use std::{
    io::{self, Write},
    panic::{self, AssertUnwindSafe},
    process::ExitCode,
    sync::Mutex,
};

use anyhow::Result;

use crate::{app::App, cli::Cli, terminal};

static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

/// Builds and runs the application behind its process-level panic boundary.
#[must_use]
pub fn run(cli: Cli) -> ExitCode {
    run_and_report(
        || {
            let app = App::new(cli.book)?;
            terminal::run(app)
        },
        &mut io::stderr(),
    )
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
            let _write_result = writeln!(diagnostics, "TermLeaf could not continue: {error}");
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
}
