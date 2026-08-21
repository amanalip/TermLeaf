//! Wall-clock formatting for the status line.
//!
//! Time is displayed in UTC with a fixed `HH:MM` shape so the status line is
//! deterministic for a given instant and never depends on host locale.

use std::time::{SystemTime, UNIX_EPOCH};

/// Formats the current instant as UTC `HH:MM`.
#[must_use]
pub fn now_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format_epoch(seconds)
}

/// Formats epoch seconds as UTC `HH:MM`.
#[must_use]
pub fn format_epoch(seconds: u64) -> String {
    let minutes_of_day = (seconds / 60) % (24 * 60);
    let hour = minutes_of_day / 60;
    let minute = minutes_of_day % 60;
    format!("{hour:02}:{minute:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instants_format_as_utc_hours_and_minutes() {
        assert_eq!(format_epoch(0), "00:00");
        assert_eq!(format_epoch(59 * 60 + 59), "00:59");
        assert_eq!(format_epoch(10 * 3600 + 42 * 60), "10:42");
        assert_eq!(format_epoch(23 * 3600 + 59 * 60), "23:59");
        assert_eq!(format_epoch(24 * 3600), "00:00");
    }
}
