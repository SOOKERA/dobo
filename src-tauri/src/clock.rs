// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Local time helpers (the reminders and work hours think in the person's own clock).

use chrono::{Datelike, Local, NaiveTime, TimeZone};

/// 0 = Sunday … 6 = Saturday, in the local day of `ms`.
pub fn weekday(ms: i64) -> Option<u32> {
    Local.timestamp_millis_opt(ms).single().map(|d| d.weekday().num_days_from_sunday())
}

/// "Thursday" / "jeudi" (in the app's language).
pub fn weekday_name(ms: i64) -> String {
    weekday(ms).map(|d| crate::i18n::weekday(d).to_string()).unwrap_or_else(|| tr!("today", "aujourd’hui"))
}

/// "2026-11-18_1730" for file names.
pub fn stamp(ms: i64) -> String {
    Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d_%H%M%S").to_string()).unwrap_or_default()
}

pub fn local_date(ms: i64) -> String {
    Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

pub fn local_hhmm(ms: i64) -> String {
    Local.timestamp_millis_opt(ms).single().map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
}

/// That day's "HH:MM" (the day of `ms`, local), in milliseconds. On the night the clocks go
/// forward, a time that doesn't exist that night counts one hour later.
pub fn today_at(ms: i64, hhmm: &str) -> Option<i64> {
    let day = Local.timestamp_millis_opt(ms).single()?.date_naive();
    let t = NaiveTime::parse_from_str(hhmm.trim(), "%H:%M").ok()?;
    crate::repeat::to_ms(day.and_time(t))
}
