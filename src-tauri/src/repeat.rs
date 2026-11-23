// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Repeating tasks: "every Monday 10h", "every weekday", "every 2 weeks on Mon, Thu",
//! "every month on the 1st". Ticking one makes the next occurrence, at the same time of day.
//! The page has the same rules (src/lib/repeat.ts) for its previews.

use crate::clock::{Local, Zone};
use jiff::Span;
use jiff::civil::{Date, DateTime, Time, Weekday};
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
    /// Its own time of day ("02:30"), kept as the clock shows it: each occurrence is at that
    /// time, also after a night the clocks change (an occurrence moved later because its time
    /// didn't exist that night goes back to it the next time). Older repeats without it use the
    /// time of the occurrence before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

fn local_in(tz: &impl Zone, ms: i64) -> Option<DateTime> {
    tz.civil(ms)
}

/// A local date and time in milliseconds. A time skipped by a clock change (spring forward:
/// 02:30 doesn't exist that night) is the first minute after it that does (03:00), never the
/// end of the repeat; a time that comes twice (the night the clocks go back) is the first one.
#[cfg(test)]
pub(crate) fn to_ms(dt: DateTime) -> Option<i64> {
    to_ms_in(&Local, dt)
}

/// `to_ms` in a given time zone (the tests' own zones with clock changes).
pub(crate) fn to_ms_in(tz: &impl Zone, dt: DateTime) -> Option<i64> {
    if let Some(ms) = tz.instant(dt) {
        return Some(ms);
    }
    // (Clock changes skip an hour, a few half an hour: three hours is more than any.)
    let minute = dt.with().second(0).subsec_nanosecond(0).build().ok()?;
    (1..=180).find_map(|k| tz.instant(minute.checked_add(Span::new().minutes(k)).ok()?))
}

/// "HH:MM" as a time of day: one or two digits each (white space may follow the colon), hours
/// 0 to 23, minutes 0 to 59; anything else is not a time.
pub fn parse_hhmm(s: &str) -> Option<Time> {
    fn number(s: &str) -> Option<(i8, &str)> {
        let n = s.bytes().take(2).take_while(u8::is_ascii_digit).count();
        Some((s[..n].parse().ok()?, &s[n..]))
    }
    let (h, rest) = number(s.trim())?;
    let (m, rest) = number(rest.strip_prefix(':')?.trim_start())?;
    if !rest.is_empty() {
        return None;
    }
    Time::new(h, m, 0, 0).ok()
}

/// The longest step accepted ("every 3650 days"); a made-up bigger one is cut down, so the
/// date maths can't overflow (that would be a panic in the clock).
const INTERVAL_MAX: i64 = 3650;

fn sunday_based(w: Weekday) -> u32 {
    w.to_sunday_zero_offset() as u32
}

/// Monday of the date's week.
fn week_start(d: Date) -> Option<Date> {
    d.checked_sub(Span::new().days(d.weekday().to_monday_zero_offset())).ok()
}

impl Repeat {
    /// The occurrence after `prev` (at its own time of day), ignoring when the repeat ends.
    #[cfg(test)]
    pub fn next(&self, prev: i64) -> Option<i64> {
        self.next_in(&Local, prev)
    }

    /// `next` in a given time zone.
    pub(crate) fn next_in(&self, tz: &impl Zone, prev: i64) -> Option<i64> {
        let p = local_in(tz, prev)?;
        let date = p.date();
        let time = self.at.as_deref().and_then(parse_hhmm).unwrap_or(p.time());
        let n = self.interval.clamp(1, INTERVAL_MAX);
        let next_date = match self.freq.as_str() {
            "day" => date.checked_add(Span::new().days(n)).ok()?,
            "weekday" => {
                let mut d = date.tomorrow().ok()?;
                while matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday) {
                    d = d.tomorrow().ok()?;
                }
                d
            }
            "week" => {
                let days: Vec<u32> = if self.days.is_empty() { vec![sunday_based(date.weekday())] } else { self.days.clone() };
                let start = week_start(date)?;
                let mut found = None;
                for k in 1..=(7 * n + 7) {
                    let d = date.checked_add(Span::new().days(k)).ok()?;
                    let weeks = crate::clock::days_between(start, week_start(d)?) / 7;
                    if weeks % n == 0 && days.contains(&sunday_based(d.weekday())) {
                        found = Some(d);
                        break;
                    }
                }
                found?
            }
            "month" => {
                let want = self.monthday.unwrap_or(date.day() as u32).clamp(1, 31);
                let first = date.first_of_month().checked_add(Span::new().months(n)).ok()?;
                let day = want.min(first.days_in_month() as u32);
                Date::new(first.year(), first.month(), day as i8).ok()?
            }
            _ => return None,
        };
        to_ms_in(tz, next_date.to_datetime(time))
    }

    /// The next occurrence once the one due at `due` is done, or None when the repeat is over.
    /// Ticked late (the Monday one on Wednesday), it jumps to the first occurrence still to
    /// come, same time of day, so nothing rings at once; the missed ones count as passed.
    /// Returns that time and the new "done" count.
    pub fn next_after_done(&self, due: i64, now: i64) -> Option<(i64, i64)> {
        self.next_after_done_in(&Local, due, now)
    }

    /// `next_after_done` in a given time zone.
    pub(crate) fn next_after_done_in(&self, tz: &impl Zone, due: i64, now: i64) -> Option<(i64, i64)> {
        let last_day = match self.ends {
            Ends::On { date } => Some(local_in(tz, date)?.date()),
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
            let next = self.next_in(tz, prev)?;
            if let Some(last) = last_day
                && local_in(tz, next)?.date() > last
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
        to_ms(jiff::civil::date(y as i16, m as i8, d as i8).at(h as i8, min as i8, 0, 0)).unwrap()
    }

    fn rule(freq: &str) -> Repeat {
        Repeat { freq: freq.into(), interval: 1, days: vec![], monthday: None, ends: Ends::Never, done: 0, at: None }
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
        assert_eq!(r.next(at(2025, 11, 18, 9, 0)).map(|t| local_in(&Local, t).unwrap().day()), Some(1));
        r.freq = "week".into();
        r.days = vec![42];
        assert!(r.next(at(2025, 11, 18, 9, 0)).is_none());
    }

    #[test]
    fn month_ends_and_leap_years() {
        let mut r = rule("month");
        r.monthday = Some(29);
        // 29 Jan 2027 → 28 Feb (not a leap year) → 29 Mar; 2028 is: 29 Feb.
        assert_eq!(r.next(at(2027, 1, 29, 8, 0)), Some(at(2027, 2, 28, 8, 0)));
        assert_eq!(r.next(at(2027, 2, 28, 8, 0)), Some(at(2027, 3, 29, 8, 0)));
        assert_eq!(r.next(at(2028, 1, 29, 8, 0)), Some(at(2028, 2, 29, 8, 0)));
        r.monthday = Some(31);
        assert_eq!(r.next(at(2028, 1, 31, 8, 0)), Some(at(2028, 2, 29, 8, 0)));
        assert_eq!(r.next(at(2028, 2, 29, 8, 0)), Some(at(2028, 3, 31, 8, 0)));
        // Every 12 months on the 29th, from a leap day: the 28th in the years between.
        r.monthday = Some(29);
        r.interval = 12;
        assert_eq!(r.next(at(2028, 2, 29, 8, 0)), Some(at(2029, 2, 28, 8, 0)));
        // Every day over the year's end and a leap day.
        let d = rule("day");
        assert_eq!(d.next(at(2028, 2, 28, 7, 0)), Some(at(2028, 2, 29, 7, 0)));
        assert_eq!(d.next(at(2026, 12, 31, 23, 30)), Some(at(2027, 1, 1, 23, 30)));
    }

    #[test]
    fn the_same_time_of_day_across_the_clock_changes() {
        use crate::clock::zones::{new_york, paris};
        for tz in [paris(), new_york()] {
            let at = |m: u32, d: u32, h: u32, min: u32| to_ms_in(&tz, jiff::civil::date(2026, m as i8, d as i8).at(h as i8, min as i8, 0, 0)).unwrap();
            let (spring, autumn) = if tz.to_offset(jiff::Timestamp::from_millisecond(crate::clock::pin::utc_ms(2026, 1, 1, 0, 0, 0)).unwrap()).seconds() == 3600 { (29, 25) } else { (8, 1) };
            let autumn_month = if autumn == 25 { 10 } else { 11 };
            // Every day at 09:00: still 09:00 the day after each change (23 / 25 hours later).
            let day = rule("day");
            assert_eq!(day.next_in(&tz, at(3, spring - 1, 9, 0)), Some(at(3, spring, 9, 0)));
            assert_eq!(day.next_in(&tz, at(3, spring - 1, 9, 0)).unwrap() - at(3, spring - 1, 9, 0), 23 * 3_600_000);
            let before_autumn = if autumn == 1 { at(10, 31, 9, 0) } else { at(10, autumn - 1, 9, 0) };
            assert_eq!(day.next_in(&tz, before_autumn), Some(at(autumn_month, autumn, 9, 0)));
            assert_eq!(day.next_in(&tz, before_autumn).unwrap() - before_autumn, 25 * 3_600_000);
            // Every week, ticked late across the change: the next one still to come, at 09:00.
            let mut week = rule("week");
            week.days = vec![0];
            let sunday_before = at(3, spring - 7, 9, 0);
            assert_eq!(week.next_after_done_in(&tz, sunday_before, at(3, spring, 8, 0)), Some((at(3, spring, 9, 0), 1)));
            // At 02:30, skipped on the night the clocks go forward: 03:00 that night, the first
            // minute after it there is.
            assert_eq!(day.next_in(&tz, at(3, spring - 1, 2, 30)), Some(at(3, spring, 3, 0)));
        }
    }

    /// A repeat keeps its own time of day across both clock changes, in Paris (the change at
    /// 02:00 → 03:00 and 03:00 → 02:00) and New York (02:00 → 03:00 and 02:00 → 01:00).
    #[test]
    fn a_repeat_keeps_its_own_time_across_the_clock_changes() {
        use crate::clock::zones::{Zone, new_york, paris};
        let hm = |tz: &Zone, ms: i64| tz.civil(ms).unwrap().strftime("%m-%d %H:%M").to_string();
        for (tz, spring, autumn, doubled) in [(paris(), (3, 29), (10, 25), "02:30"), (new_york(), (3, 8), (11, 1), "01:30")] {
            let at = |m: u32, d: u32, h: u32, min: u32| to_ms_in(&tz, jiff::civil::date(2026, m as i8, d as i8).at(h as i8, min as i8, 0, 0)).unwrap();
            let mut daily = rule("day");
            daily.at = Some("02:30".into());
            // Spring: 02:30 doesn't exist, so the first minute after it (03:00); the day after,
            // 02:30 again (never 03:00 or 03:30 from then on).
            let before = at(spring.0, spring.1 - 1, 2, 30);
            let skipped = daily.next_in(&tz, before).unwrap();
            assert_eq!(hm(&tz, skipped), format!("{:02}-{:02} 03:00", spring.0, spring.1));
            let after = daily.next_in(&tz, skipped).unwrap();
            assert_eq!(hm(&tz, after), format!("{:02}-{:02} 02:30", spring.0, spring.1 + 1));
            // Ticked late across the change, the same: the next 02:30 still to come.
            assert_eq!(daily.next_after_done_in(&tz, before, skipped + 60_000).map(|(t, _)| hm(&tz, t)), Some(format!("{:02}-{:02} 02:30", spring.0, spring.1 + 1)));
            // Autumn: the time that comes twice is the first of the two, then the same time again.
            daily.at = Some(doubled.into());
            let (h, m) = (doubled[..2].parse().unwrap(), doubled[3..].parse().unwrap());
            let eve = if autumn.1 == 1 { at(10, 31, h, m) } else { at(autumn.0, autumn.1 - 1, h, m) };
            let twice = daily.next_in(&tz, eve).unwrap();
            assert_eq!(hm(&tz, twice), format!("{:02}-{:02} {doubled}", autumn.0, autumn.1));
            assert_eq!(twice - eve, 24 * 3_600_000, "the first of the two: 24 hours after the day before's");
            let next = daily.next_in(&tz, twice).unwrap();
            assert_eq!(hm(&tz, next), format!("{:02}-{:02} {doubled}", autumn.0, autumn.1 + 1));
            // A weekly one at 09:00 stays at 09:00 on both sides.
            let mut weekly = rule("week");
            weekly.days = vec![0];
            weekly.at = Some("09:00".into());
            assert_eq!(hm(&tz, weekly.next_in(&tz, at(spring.0, spring.1 - 7, 9, 0)).unwrap()), format!("{:02}-{:02} 09:00", spring.0, spring.1));
        }
    }

    #[test]
    fn an_older_repeat_without_its_own_time_uses_the_one_before() {
        let r: Repeat = serde_json::from_value(serde_json::json!({"freq": "day"})).unwrap();
        assert_eq!(r.at, None);
        assert_eq!(r.next(at(2025, 11, 18, 7, 45)), Some(at(2025, 11, 19, 7, 45)));
        // Its own time wins over the one before's (an occurrence that was moved later).
        let r = Repeat { at: Some("07:15".into()), ..r };
        assert_eq!(r.next(at(2025, 11, 18, 8, 15)), Some(at(2025, 11, 19, 7, 15)));
        assert!(serde_json::to_value(&r).unwrap().get("at").is_some());
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

/// Repeats on the clock changes pinned down, written against chrono before Dobo moved to
/// another date library: the same answers after the move.
#[cfg(test)]
mod pinned_repeats {
    use super::*;
    use crate::clock::pin::{civil, utc_str};
    use crate::clock::zones::{Zone, new_york, paris};

    fn rule(freq: &str, at: Option<&str>) -> Repeat {
        Repeat { freq: freq.into(), interval: 1, days: vec![], monthday: None, ends: Ends::Never, done: 0, at: at.map(String::from) }
    }

    fn ms(tz: &Zone, y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
        to_ms_in(tz, civil(y, m, d, h, mi, 0)).unwrap()
    }

    fn next(r: &Repeat, tz: &Zone, prev: i64) -> String {
        utc_str(r.next_in(tz, prev).unwrap())
    }

    #[test]
    fn an_older_repeat_moved_by_the_skipped_hour_stays_at_its_new_time() {
        let p = paris();
        let r = rule("day", None);
        let moved = r.next_in(&p, ms(&p, 2026, 3, 28, 2, 30)).unwrap();
        assert_eq!(utc_str(moved), "2026-03-29 01:00:00");
        assert_eq!(next(&r, &p, moved), "2026-03-30 01:00:00");
    }

    #[test]
    fn from_the_second_run_of_the_doubled_hour() {
        let (p, ny) = (paris(), new_york());
        let second_p = crate::clock::pin::utc_ms(2026, 10, 25, 1, 30, 0);
        let second_ny = crate::clock::pin::utc_ms(2026, 11, 1, 6, 30, 0);
        for at in [None, Some("02:30")] {
            assert_eq!(next(&rule("day", at), &p, second_p), "2026-10-26 01:30:00", "{at:?}");
        }
        for at in [None, Some("01:30")] {
            assert_eq!(next(&rule("day", at), &ny, second_ny), "2026-11-02 06:30:00", "{at:?}");
        }
    }

    #[test]
    fn weekday_week_and_month_repeats_over_the_changes() {
        let (p, ny) = (paris(), new_york());
        // Every weekday at 02:30, Friday's to Monday's over the spring weekend.
        assert_eq!(next(&rule("weekday", Some("02:30")), &p, ms(&p, 2026, 3, 27, 2, 30)), "2026-03-30 00:30:00");
        assert_eq!(next(&rule("weekday", Some("02:30")), &ny, ms(&ny, 2026, 3, 6, 2, 30)), "2026-03-09 06:30:00");
        // Every Sunday at 02:30: 03:00 on the night it doesn't exist, then 02:30 again.
        let mut sunday = rule("week", Some("02:30"));
        sunday.days = vec![0];
        let skipped = sunday.next_in(&ny, ms(&ny, 2026, 3, 1, 2, 30)).unwrap();
        assert_eq!(utc_str(skipped), "2026-03-08 07:00:00");
        assert_eq!(next(&sunday, &ny, skipped), "2026-03-15 06:30:00");
        // Every month on the 29th at 02:30: from 28 February, 29 March (skipped: 03:00), 29 April.
        let mut monthly = rule("month", Some("02:30"));
        monthly.monthday = Some(29);
        let march = monthly.next_in(&p, ms(&p, 2026, 2, 28, 2, 30)).unwrap();
        assert_eq!(utc_str(march), "2026-03-29 01:00:00");
        assert_eq!(next(&monthly, &p, march), "2026-04-29 00:30:00");
    }

    #[test]
    fn the_last_day_and_ticked_late_over_the_changes() {
        let p = paris();
        // Ends on 29 March (any time that day): the moved 03:00 that day still counts, the 30th not.
        let mut r = rule("day", Some("02:30"));
        r.ends = Ends::On { date: ms(&p, 2026, 3, 29, 0, 30) };
        let due = ms(&p, 2026, 3, 28, 2, 30);
        let (t, done) = r.next_after_done_in(&p, due, due).unwrap();
        assert_eq!((utc_str(t).as_str(), done), ("2026-03-29 01:00:00", 1));
        assert_eq!(r.next_after_done_in(&p, t, t), None);
        // A last day given inside the doubled hour's second run: that day.
        r.ends = Ends::On { date: crate::clock::pin::utc_ms(2026, 10, 25, 1, 30, 0) };
        let eve = ms(&p, 2026, 10, 24, 2, 30);
        assert_eq!(r.next_after_done_in(&p, eve, eve).map(|(t, d)| (utc_str(t), d)), Some(("2026-10-25 00:30:00".into(), 1)));
        // Ticked at 02:45 summer time on 25 October: that day's 02:30 (the first) has passed, so
        // the next is the 26th (never the second 02:30 a quarter of an hour later).
        let r = rule("day", Some("02:30"));
        let now = crate::clock::pin::utc_ms(2026, 10, 25, 0, 45, 0);
        assert_eq!(r.next_after_done_in(&p, eve, now).map(|(t, d)| (utc_str(t), d)), Some(("2026-10-26 01:30:00".into(), 2)));
    }
}
