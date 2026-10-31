//! "2m 05s" style durations.

use chrono::{DateTime, Utc};

/// Seconds as `45s`, `2m 05s` or `1h 02m`.
pub fn format_duration(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {:02}s", secs / 60, secs % 60),
        _ => format!("{}h {:02}m", secs / 3600, secs % 3600 / 60),
    }
}

/// The reported duration, or the time since `started` for something still running; `—` when unknown.
pub fn duration_text(duration: Option<u64>, started: Option<DateTime<Utc>>) -> String {
    let secs = duration.or_else(|| u64::try_from((Utc::now() - started?).num_seconds()).ok());
    secs.map_or("—".into(), format_duration)
}

#[cfg(test)]
#[path = "tests/duration.test.rs"]
mod tests;
