// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Repeating tasks: "every Monday 10h", "every weekday", "every 2 weeks on Mon, Thu",
//! "every month on the 1st". Ticking one makes the next occurrence, at the same time of day.
//! The page has the same rules (src/lib/repeat.ts) for its previews.

use chrono::{Datelike, Days, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Ends {
    #[default]
    Never,
    /// After this many occurrences in all.
    After { count: i64 },
    /// No occurrence after this day (milliseconds, any time that day).
    On { date: i64 },
}

fn one() -> i64 {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Repeat {
    /// "day", "weekday", "week" or "month".
    pub freq: String,
    /// Every N days / weeks / months.
    #[serde(default = "one")]
    pub interval: i64,
    /// For "week": the days (0 = Sunday … 6 = Saturday). Empty: the first occurrence's day.
    #[serde(default)]
    pub days: Vec<u32>,
    /// For "month": the day of the month (clamped to short months).
    #[serde(default)]
    pub monthday: Option<u32>,
    #[serde(default)]
    pub ends: Ends,
    /// Occurrences already done.
    #[serde(default)]
    pub done: i64,
}

fn local(ms: i64) -> Option<NaiveDateTime> {
    Local.timestamp_millis_opt(ms).single().map(|d| d.naive_local())
}

/// A local date and time in milliseconds. A time skipped by a clock change (spring forward:
/// 02:30 doesn't exist that night) moves one hour later instead of ending the repeat.
pub(crate) fn to_ms(dt: NaiveDateTime) -> Option<i64> {
    Local
        .from_local_datetime(&dt)
        .earliest()
        .or_else(|| Local.from_local_datetime(&(dt + chrono::TimeDelta::hours(1))).earliest())
        .map(|d| d.timestamp_millis())
}

/// The longest step accepted ("every 3650 days"); a made-up bigger one is cut down, so the
/// date maths can't overflow (that would be a panic in the clock).
const INTERVAL_MAX: i64 = 3650;

fn sunday_based(w: Weekday) -> u32 {
    w.num_days_from_sunday()
}

/// Monday of the date's week.
fn week_start(d: NaiveDate) -> NaiveDate {
    d - Days::new(d.weekday().num_days_from_monday() as u64)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next = first + Months::new(1);
    (next - first).num_days() as u32
}

impl Repeat {
    /// The occurrence after `prev` (same time of day), ignoring when the repeat ends.
    pub fn next(&self, prev: i64) -> Option<i64> {
        let p = local(prev)?;
        let (date, time) = (p.date(), p.time());
        let n = self.interval.clamp(1, INTERVAL_MAX) as u64;
        let next_date = match self.freq.as_str() {
            "day" => date.checked_add_days(Days::new(n))?,
            "weekday" => {
                let mut d = date.checked_add_days(Days::new(1))?;
                while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                    d = d.checked_add_days(Days::new(1))?;
                }
                d
            }
            "week" => {
                let days: Vec<u32> = if self.days.is_empty() { vec![sunday_based(date.weekday())] } else { self.days.clone() };
                let start = week_start(date);
                let mut found = None;
                for k in 1..=(7 * n as i64 + 7) {
                    let d = date.checked_add_days(Days::new(k as u64))?;
                    let weeks = (week_start(d) - start).num_days() / 7;
                    if weeks % n as i64 == 0 && days.contains(&sunday_based(d.weekday())) {
                        found = Some(d);
                        break;
                    }
                }
                found?
            }
            "month" => {
                let want = self.monthday.unwrap_or(date.day()).clamp(1, 31);
                let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)?.checked_add_months(Months::new(n as u32))?;
                let day = want.min(days_in_month(first.year(), first.month()));
                NaiveDate::from_ymd_opt(first.year(), first.month(), day)?
            }
            _ => return None,
        };
        to_ms(next_date.and_time(time))
    }

    /// The next occurrence once the one due at `due` is done, or None when the repeat is over.
    /// Ticked late (the Monday one on Wednesday), it jumps to the first occurrence still to
    /// come, same time of day, so nothing rings at once; the missed ones count as passed.
    /// Returns that time and the new "done" count.
    pub fn next_after_done(&self, due: i64, now: i64) -> Option<(i64, i64)> {
        let last_day = match self.ends {
            Ends::On { date } => Some(local(date)?.date()),
            _ => None,
        };
        let mut prev = due;
        // Bounded: even "every day" ticked years late stays a few thousand steps.
        for done in (self.done + 1..).take(100_000) {
            if let Ends::After { count } = self.ends
                && done >= count
            {
                return None;
            }
            let next = self.next(prev)?;
            if let Some(last) = last_day
                && local(next)?.date() > last
            {
                return None;
            }
            if next > now {
                return Some((next, done));
            }
            prev = next;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
        to_ms(NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()).unwrap()
    }

    fn rule(freq: &str) -> Repeat {
        Repeat { freq: freq.into(), interval: 1, days: vec![], monthday: None, ends: Ends::Never, done: 0 }
    }

    #[test]
    fn every_monday_10h_goes_to_next_monday() {
        // Monday 17 Nov 2025, 10:00.
        let mut r = rule("week");
        r.days = vec![1];
        assert_eq!(r.next(at(2025, 11, 17, 10, 0)), Some(at(2025, 11, 24, 10, 0)));
    }

    #[test]
    fn every_weekday_skips_the_weekend() {
        let r = rule("weekday");
        // Friday 21 Nov 2025 -> Monday 24 Nov.
        assert_eq!(r.next(at(2025, 11, 21, 9, 0)), Some(at(2025, 11, 24, 9, 0)));
        assert_eq!(r.next(at(2025, 11, 18, 9, 0)), Some(at(2025, 11, 19, 9, 0)));
    }

    #[test]
    fn every_two_weeks_on_mon_and_thu() {
        let mut r = rule("week");
        r.interval = 2;
        r.days = vec![1, 4];
        // Mon 17 Nov -> Thu 20 Nov (same week) -> Mon 1 Dec (two weeks later).
        let a = r.next(at(2025, 11, 17, 9, 0)).unwrap();
        assert_eq!(a, at(2025, 11, 20, 9, 0));
        assert_eq!(r.next(a), Some(at(2025, 12, 1, 9, 0)));
    }

    #[test]
    fn every_month_on_the_31st_uses_short_months() {
        let mut r = rule("month");
        r.monthday = Some(31);
        assert_eq!(r.next(at(2026, 1, 31, 8, 0)), Some(at(2026, 2, 28, 8, 0)));
        assert_eq!(r.next(at(2026, 2, 28, 8, 0)), Some(at(2026, 3, 31, 8, 0)));
    }

    #[test]
    fn every_day_and_every_three_days() {
        let mut r = rule("day");
        assert_eq!(r.next(at(2025, 12, 31, 7, 30)), Some(at(2026, 1, 1, 7, 30)));
        r.interval = 3;
        assert_eq!(r.next(at(2025, 12, 31, 7, 30)), Some(at(2026, 1, 3, 7, 30)));
    }

    #[test]
    fn ends_after_a_count_or_on_a_date() {
        let mut r = rule("day");
        r.ends = Ends::After { count: 3 };
        r.done = 1;
        let t = at(2025, 11, 18, 9, 0);
        assert!(r.next_after_done(t, t).is_some());
        r.done = 2;
        assert!(r.next_after_done(t, t).is_none());
        let mut r = rule("day");
        r.ends = Ends::On { date: at(2025, 11, 19, 0, 0) };
        assert_eq!(r.next_after_done(t, t), Some((at(2025, 11, 19, 9, 0), 1)));
        let t2 = at(2025, 11, 19, 9, 0);
        assert!(r.next_after_done(t2, t2).is_none());
    }

    #[test]
    fn ticked_late_jumps_to_the_next_future_occurrence() {
        // Every Monday 10:00, the 17 Nov one ticked on Wednesday 19 Nov: next is Mon 24 Nov.
        let mut r = rule("week");
        r.days = vec![1];
        let mon = at(2025, 11, 17, 10, 0);
        assert_eq!(r.next_after_done(mon, at(2025, 11, 19, 15, 0)), Some((at(2025, 11, 24, 10, 0), 1)));
        // Three weeks late: still the next Monday to come, the missed ones counted.
        assert_eq!(r.next_after_done(mon, at(2025, 12, 10, 12, 0)), Some((at(2025, 12, 15, 10, 0), 4)));
        // Same day after the time: next week, not today.
        assert_eq!(r.next_after_done(mon, at(2025, 11, 17, 11, 0)), Some((at(2025, 11, 24, 10, 0), 1)));
    }

    #[test]
    fn ticked_late_month_end_and_weekday() {
        // On the 31st, the 31 Jan one ticked on 3 Mar: 28 Feb has passed, next is 31 Mar.
        let mut r = rule("month");
        r.monthday = Some(31);
        assert_eq!(r.next_after_done(at(2026, 1, 31, 8, 0), at(2026, 3, 3, 9, 0)), Some((at(2026, 3, 31, 8, 0), 2)));
        // Every weekday 9:00, Friday's ticked on Monday 10:00: next is Tuesday 9:00.
        let r = rule("weekday");
        assert_eq!(r.next_after_done(at(2025, 11, 21, 9, 0), at(2025, 11, 24, 10, 0)), Some((at(2025, 11, 25, 9, 0), 2)));
        // A limit reached by the missed ones ends the repeat.
        let mut r = rule("day");
        r.ends = Ends::After { count: 3 };
        assert!(r.next_after_done(at(2025, 11, 1, 9, 0), at(2025, 11, 10, 9, 0)).is_none());
    }

    #[test]
    fn huge_or_odd_rules_never_panic() {
        let mut r = rule("day");
        r.interval = i64::MAX;
        assert!(r.next(at(2025, 11, 18, 9, 0)).is_some());
        r.freq = "month".into();
        r.monthday = Some(0);
        assert_eq!(r.next(at(2025, 11, 18, 9, 0)).map(|t| local(t).unwrap().day()), Some(1));
        r.freq = "week".into();
        r.days = vec![42];
        assert!(r.next(at(2025, 11, 18, 9, 0)).is_none());
    }

    #[test]
    fn json_shape_matches_the_page() {
        let v = serde_json::json!({"freq": "week", "interval": 1, "days": [1], "ends": {"kind": "after", "count": 4}, "done": 0});
        let r: Repeat = serde_json::from_value(v).unwrap();
        assert_eq!(r.ends, Ends::After { count: 4 });
        let v = serde_json::json!({"freq": "month", "monthday": 1});
        let r: Repeat = serde_json::from_value(v).unwrap();
        assert_eq!(r.interval, 1);
        assert_eq!(r.ends, Ends::Never);
    }
}
