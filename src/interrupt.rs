use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, bail};

static REQUESTED: AtomicBool = AtomicBool::new(false);
static HANDLER: OnceLock<std::result::Result<(), String>> = OnceLock::new();

/// Installs the process-wide external interrupt handler once.
///
/// # Errors
///
/// Returns an error when the operating system handler cannot be installed.
pub fn install() -> Result<()> {
    let result = HANDLER.get_or_init(|| {
        ctrlc::set_handler(|| REQUESTED.store(true, Ordering::SeqCst))
            .map_err(|error| error.to_string())
    });
    if let Err(error) = result {
        bail!("could not install the interrupt handler: {error}");
    }
    Ok(())
}

pub fn clear() {
    REQUESTED.store(false, Ordering::SeqCst);
}

#[must_use]
pub fn requested() -> bool {
    REQUESTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_011_interrupt_flag_is_process_safe() {
        clear();
        REQUESTED.store(true, Ordering::SeqCst);

        assert!(requested());
        clear();
        assert!(!requested());
    }
}
