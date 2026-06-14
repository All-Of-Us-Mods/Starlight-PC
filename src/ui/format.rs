//! Shared presentation-layer formatters for profile timestamps and durations.

use chrono::{DateTime, Local};

/// UNIX-millis → local "Mon D, YYYY · HH:MM".
fn datetime_ms(timestamp_ms: i64) -> String {
    DateTime::from_timestamp_millis(timestamp_ms)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%b %-d, %Y · %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

/// UNIX-millis → local "Mon D, YYYY".
pub fn date_ms(timestamp_ms: i64) -> String {
    DateTime::from_timestamp_millis(timestamp_ms)
        .map(|dt| dt.with_timezone(&Local).format("%b %-d, %Y").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Stat-card value for `last_launched_at`. `None` → `"Never"`.
pub fn last_launched(timestamp_ms: Option<i64>) -> String {
    match timestamp_ms {
        Some(ts) => datetime_ms(ts),
        None => "Never".to_string(),
    }
}

/// Stat-card value for `total_play_time`. `None`/`0` → `"Never played"`,
/// sub-minute → `"< 1 min"`, otherwise `"Xh Ym"` or `"Y min"`.
pub fn play_time(ms: Option<i64>) -> String {
    let total_ms = ms.unwrap_or(0).max(0);
    if total_ms == 0 {
        return "Never played".to_string();
    }
    let total_minutes = total_ms / 60_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes} min")
    } else {
        "< 1 min".to_string()
    }
}
