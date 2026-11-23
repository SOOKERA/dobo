// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Local time helpers (the reminders and work hours think in the person's own clock).

use jiff::civil::{Date, DateTime};
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{Span, Timestamp};
use std::sync::atomic::{AtomicBool, Ordering};

/// A time zone the clock maths work in: the computer's own (`Local`), or the tests' own zones.
pub(crate) trait Zone {
    /// The date and time this zone's clock shows at `ms`. None out of the calendar.
    fn civil(&self, ms: i64) -> Option<DateTime>;
    /// The moment this zone's clock shows `dt`, in milliseconds: the first of the two when it
    /// comes twice (the night the clocks go back), None when it never does (skipped the night
    /// they go forward).
    fn instant(&self, dt: DateTime) -> Option<i64>;
}

impl Zone for TimeZone {
    fn civil(&self, ms: i64) -> Option<DateTime> {
        Timestamp::from_millisecond(ms).ok().map(|t| self.to_datetime(t))
    }

    fn instant(&self, dt: DateTime) -> Option<i64> {
        let both = self.to_ambiguous_timestamp(dt);
        if matches!(both.offset(), AmbiguousOffset::Gap { .. }) {
            return None;
        }
        both.earlier().ok().map(|t| t.as_millisecond())
    }
}

/// The computer's own time zone, as it is at each use (a zone changed in Windows' settings
/// counts at once).
///
/// On Windows: Windows' own rules for the year at hand (`GetTimeZoneInformationForYear`), the
/// ones its clock follows, as Dobo always had them; no time zone database is needed.
#[derive(Clone, Copy, Debug)]
pub struct Local;

impl Zone for Local {
    fn civil(&self, ms: i64) -> Option<DateTime> {
        // (Windows' rules for the year of that moment in UTC, as Dobo always had them.)
        let year = TimeZone::UTC.to_datetime(Timestamp::from_millisecond(ms).ok()?).year();
        local_zone(year)?.civil(ms)
    }

    fn instant(&self, dt: DateTime) -> Option<i64> {
        local_zone(dt.year())?.instant(dt)
    }
}

#[cfg(windows)]
fn local_zone(year: i16) -> Option<TimeZone> {
    Some(windows_zone::for_year_or_fixed(year))
}

#[cfg(not(windows))]
fn local_zone(_year: i16) -> Option<TimeZone> {
    Some(TimeZone::system())
}

/// Windows' time zone rules as a time zone the date maths understand.
#[cfg(windows)]
mod windows_zone {
    use jiff::tz::TimeZone;
    use std::sync::Mutex;
    use windows::Win32::Foundation::SYSTEMTIME;
    use windows::Win32::System::Time::{DYNAMIC_TIME_ZONE_INFORMATION, GetTimeZoneInformationForYear, TIME_ZONE_INFORMATION};

    /// The zones made lately, by their rules (a repeat ticked years late walks through a few years).
    static MADE: Mutex<Vec<(String, TimeZone)>> = Mutex::new(Vec::new());

    /// A zone with Windows' rules for `year`, read again each time: the computer's (None), or
    /// another Windows zone (the tests).
    pub fn for_year(which: Option<&DYNAMIC_TIME_ZONE_INFORMATION>, year: i16) -> Option<TimeZone> {
        // (Windows knows the years 1601 to 30827.)
        let year = (year as i32).clamp(1601, 30827) as u16;
        let mut info = TIME_ZONE_INFORMATION::default();
        unsafe { GetTimeZoneInformationForYear(year, which.map(|w| w as *const _), &mut info) }.ok()?;
        zone(&info)
    }

    /// The computer's zone for `year` (`for_year`); when Windows gives no rules for it, or rules
    /// that make no sense, its fixed offset from UTC (`Bias`, or UTC when there is none either),
    /// said once in the log: never no time at all (no due time, a file named "dobo--daily").
    pub fn for_year_or_fixed(year: i16) -> TimeZone {
        if let Some(tz) = for_year(None, year) {
            return tz;
        }
        let mut info = TIME_ZONE_INFORMATION::default();
        // (TIME_ZONE_ID_INVALID: no offset to be had.)
        let read = unsafe { windows::Win32::System::Time::GetTimeZoneInformation(&mut info) } != u32::MAX;
        fixed(read.then_some(&info))
    }

    /// A zone at the fixed offset `info` gives (`Bias`: minutes west of UTC), or UTC; said once.
    pub fn fixed(info: Option<&TIME_ZONE_INFORMATION>) -> TimeZone {
        static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let tz = info
            .filter(|i| i.Bias.abs() < 24 * 60)
            .and_then(|i| jiff::tz::Offset::from_seconds(-i.Bias * 60).ok())
            .map(TimeZone::fixed)
            .unwrap_or(TimeZone::UTC);
        if !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("Windows gave no time zone rules Dobo could read: the clock follows a fixed offset ({tz:?}) instead.");
        }
        tz
    }

    /// The zone for these rules (made once, then reused).
    pub fn zone(info: &TIME_ZONE_INFORMATION) -> Option<TimeZone> {
        let rules = posix(info)?;
        let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, tz)) = made.iter().find(|(r, _)| *r == rules) {
            return Some(tz.clone());
        }
        let tz = TimeZone::posix(&rules).ok()?;
        if made.len() >= 8 {
            made.remove(0);
        }
        made.push((rules, tz.clone()));
        Some(tz)
    }

    /// Windows' rules for a year written as a POSIX TZ rule ("<STD>-1<DST>-2,M3.5.0/2:00:00,
    /// M10.5.0/3:00:00"): the offsets, and when summer time starts (on the standard clock) and
    /// ends (on the summer clock), as Windows gives them. None for rules that make no sense.
    pub fn posix(info: &TIME_ZONE_INFORMATION) -> Option<String> {
        let standard = offset(info.Bias.checked_add(info.StandardBias)?)?;
        let summer = offset(info.Bias.checked_add(info.DaylightBias)?)?;
        Some(match (change(&info.DaylightDate)?, change(&info.StandardDate)?) {
            (Some(start), Some(end)) => format!("<STD>{standard}<DST>{summer},{start},{end}"),
            // No summer time (or only one change in the year, which Windows never gives).
            _ => format!("<STD>{standard}"),
        })
    }

    /// Minutes west of UTC (Windows' bias) as a POSIX offset ("-1:00" is an hour east).
    fn offset(west: i32) -> Option<String> {
        if west.abs() >= 24 * 60 {
            return None;
        }
        Some(format!("{}{}:{:02}", if west < 0 { "-" } else { "" }, west.abs() / 60, west.abs() % 60))
    }

    /// When a change comes: Some(None) for none; "M3.5.0/2:00:00" (the last Sunday of March at
    /// 02:00), or a fixed day of the year; None when Windows' values make no sense.
    fn change(st: &SYSTEMTIME) -> Option<Option<String>> {
        if st.wYear == 0 && st.wMonth == 0 {
            return Some(None);
        }
        if st.wHour > 23 || st.wMinute > 59 || st.wSecond > 59 || st.wMilliseconds > 999 {
            return None;
        }
        // (A change at 23:59:59.999, as Windows writes some, is midnight: the same for every
        // whole second.)
        let secs = st.wHour as u32 * 3600 + st.wMinute as u32 * 60 + st.wSecond as u32 + (st.wMilliseconds > 0) as u32;
        let time = format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
        if st.wYear != 0 {
            // A fixed date: its day of the year, counted from 0 (29 February counts).
            let date = jiff::civil::Date::new(st.wYear as i16, st.wMonth as i8, st.wDay as i8).ok()?;
            return Some(Some(format!("{}/{time}", date.day_of_year() - 1)));
        }
        // The nth weekday of the month (5: the last).
        if !(1..=12).contains(&st.wMonth) || !(1..=5).contains(&st.wDay) || st.wDayOfWeek > 6 {
            return None;
        }
        Some(Some(format!("M{}.{}.{}/{time}", st.wMonth, st.wDay, st.wDayOfWeek)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::clock::Zone;
        use crate::clock::pin::{civil, utc_ms};
        use crate::clock::zones::{new_york, paris};
        use windows::Win32::System::Time::{EnumDynamicTimeZoneInformation, SystemTimeToTzSpecificLocalTimeEx};

        /// A Windows zone by its name in the registry ("Romance Standard Time").
        fn windows_zone(key: &str) -> DYNAMIC_TIME_ZONE_INFORMATION {
            for i in 0.. {
                let mut z = DYNAMIC_TIME_ZONE_INFORMATION::default();
                assert_eq!(unsafe { EnumDynamicTimeZoneInformation(i, &mut z) }, 0, "{key} not found");
                let n = z.TimeZoneKeyName.iter().position(|c| *c == 0).unwrap_or(128);
                if String::from_utf16_lossy(&z.TimeZoneKeyName[..n]) == key {
                    return z;
                }
            }
            unreachable!()
        }

        /// The date and time on that zone's clock at `ms` as Dobo has it (Windows' rules for the
        /// year of `ms` in UTC).
        fn ours(which: Option<&DYNAMIC_TIME_ZONE_INFORMATION>, ms: i64) -> jiff::civil::DateTime {
            let year = TimeZone::UTC.to_datetime(jiff::Timestamp::from_millisecond(ms).unwrap()).year();
            for_year(which, year).unwrap().civil(ms).unwrap()
        }

        /// The same as Windows itself converts it.
        fn windows(which: Option<&DYNAMIC_TIME_ZONE_INFORMATION>, ms: i64) -> jiff::civil::DateTime {
            let u = TimeZone::UTC.to_datetime(jiff::Timestamp::from_millisecond(ms).unwrap());
            let utc = SYSTEMTIME { wYear: u.year() as u16, wMonth: u.month() as u16, wDay: u.day() as u16, wHour: u.hour() as u16, wMinute: u.minute() as u16, wSecond: u.second() as u16, ..Default::default() };
            let mut local = SYSTEMTIME::default();
            unsafe { SystemTimeToTzSpecificLocalTimeEx(which.map(|w| w as *const _), &utc, &mut local) }.unwrap();
            civil(local.wYear as i32, local.wMonth as u32, local.wDay as u32, local.wHour as u32, local.wMinute as u32, local.wSecond as u32)
        }

        #[test]
        fn windows_rules_for_paris_and_new_york_are_the_test_zones() {
            for (key, test_zone, rules) in [
                ("Romance Standard Time", paris(), "<STD>-1:00<DST>-2:00,M3.5.0/2:00:00,M10.5.0/3:00:00"),
                ("Eastern Standard Time", new_york(), "<STD>5:00<DST>4:00,M3.2.0/2:00:00,M11.1.0/2:00:00"),
            ] {
                let z = windows_zone(key);
                let mut info = TIME_ZONE_INFORMATION::default();
                unsafe { GetTimeZoneInformationForYear(2026, Some(&z), &mut info) }.unwrap();
                assert_eq!(posix(&info).as_deref(), Some(rules));
                let tz = for_year(Some(&z), 2026).unwrap();
                // Every quarter of an hour of 2026, both ways.
                let start = utc_ms(2026, 1, 1, 0, 0, 0);
                for k in 0..(365 * 96) {
                    let ms = start + k * 900_000;
                    assert_eq!(tz.civil(ms), test_zone.civil(ms), "{key} {ms}");
                    let dt = test_zone.civil(ms).unwrap();
                    assert_eq!(tz.instant(dt), test_zone.instant(dt), "{key} {dt}");
                }
            }
        }

        #[test]
        fn the_clock_as_windows_shows_it_in_many_zones() {
            // Both hemispheres, half hours, zones whose rules changed over the years, none.
            let keys = [
                "Romance Standard Time",
                "Eastern Standard Time",
                "AUS Eastern Standard Time",
                "Lord Howe Standard Time",
                "Iran Standard Time",
                "Morocco Standard Time",
                "Greenland Standard Time",
                "Pacific SA Standard Time",
                "Egypt Standard Time",
                "E. Africa Standard Time",
                "India Standard Time",
            ];
            let mut zones: Vec<Option<DYNAMIC_TIME_ZONE_INFORMATION>> = keys.iter().map(|k| Some(windows_zone(k))).collect();
            // And the computer's own.
            zones.push(None);
            let start = utc_ms(2015, 1, 1, 0, 30, 0);
            for z in &zones {
                for k in 0..(16 * 365 * 6) {
                    let ms = start + k * 4 * 3_600_000;
                    let (o, w) = (ours(z.as_ref(), ms), windows(z.as_ref(), ms));
                    // (Around the new year, when UTC is already in the next year or still in the
                    // last, Dobo has always used the UTC year's rules: Greenland, whose offset
                    // changed in 2023, was an hour off on that night, as before.)
                    if o.year() != w.year() || w.year() != TimeZone::UTC.to_datetime(jiff::Timestamp::from_millisecond(ms).unwrap()).year() {
                        continue;
                    }
                    assert_eq!(o, w, "{:?} {ms}", z.map(|z| String::from_utf16_lossy(&z.TimeZoneKeyName)));
                }
            }
        }

        #[test]
        fn odd_rules_from_windows() {
            let mut info = TIME_ZONE_INFORMATION { Bias: -180, ..Default::default() };
            assert_eq!(posix(&info).as_deref(), Some("<STD>-3:00"));
            info.Bias = 210;
            assert_eq!(posix(&info).as_deref(), Some("<STD>3:30"));
            // A change at 23:59:59.999 is midnight; a fixed date is its day of the year.
            info.DaylightBias = -60;
            info.DaylightDate = SYSTEMTIME { wMonth: 3, wDay: 5, wDayOfWeek: 6, wHour: 23, wMinute: 59, wSecond: 59, wMilliseconds: 999, ..Default::default() };
            info.StandardDate = SYSTEMTIME { wYear: 2028, wMonth: 3, wDay: 1, wHour: 2, ..Default::default() };
            assert_eq!(posix(&info).as_deref(), Some("<STD>3:30<DST>2:30,M3.5.6/24:00:00,60/2:00:00"));
            // Values that make no sense: no zone (as before, nothing rather than a wrong time).
            info.DaylightDate.wDay = 6;
            assert_eq!(posix(&info), None);
            info.DaylightDate.wDay = 5;
            info.Bias = 24 * 60;
            assert_eq!(posix(&info), None);
        }

        /// The odd rules above make zones that keep Windows' times: a fixed offset, and a summer
        /// time on the last Saturday of March at midnight.
        #[test]
        fn odd_rules_make_zones_that_tell_the_time() {
            use crate::clock::pin::utc_ms;
            let mut info = TIME_ZONE_INFORMATION { Bias: -180, ..Default::default() };
            let tz = zone(&info).unwrap();
            assert_eq!(tz.civil(utc_ms(2026, 6, 1, 12, 0, 0)).unwrap().hour(), 15);
            info.Bias = 210;
            info.DaylightBias = -60;
            info.DaylightDate = SYSTEMTIME { wMonth: 3, wDay: 5, wDayOfWeek: 6, wHour: 23, wMinute: 59, wSecond: 59, wMilliseconds: 999, ..Default::default() };
            info.StandardDate = SYSTEMTIME { wMonth: 10, wDay: 5, wDayOfWeek: 0, wHour: 2, ..Default::default() };
            let tz = zone(&info).unwrap();
            // Winter: 3:30 behind UTC; after the last Saturday of March (the 28th in 2026), 2:30.
            assert_eq!(tz.civil(utc_ms(2026, 1, 15, 12, 0, 0)).unwrap().to_string(), "2026-01-15T08:30:00");
            assert_eq!(tz.civil(utc_ms(2026, 4, 15, 12, 0, 0)).unwrap().to_string(), "2026-04-15T09:30:00");
        }

        /// No rules Windows gives make sense: a fixed offset from its Bias, else UTC; never none.
        #[test]
        fn no_rules_still_tell_the_time() {
            use crate::clock::pin::utc_ms;
            let east = TIME_ZONE_INFORMATION { Bias: -60, ..Default::default() };
            assert_eq!(fixed(Some(&east)).civil(utc_ms(2026, 1, 1, 12, 0, 0)).unwrap().hour(), 13);
            let odd = TIME_ZONE_INFORMATION { Bias: 99_999, ..Default::default() };
            assert_eq!(fixed(Some(&odd)).civil(utc_ms(2026, 1, 1, 12, 0, 0)).unwrap().hour(), 12);
            assert_eq!(fixed(None).civil(utc_ms(2026, 1, 1, 12, 0, 0)).unwrap().hour(), 12);
            // The computer's own: always a zone.
            for year in [1500, 2026, 32000] {
                let _ = for_year_or_fixed(year);
            }
        }
    }
}

/// The date and time on the computer's clock at `ms`.
fn local(ms: i64) -> Option<DateTime> {
    Local.civil(ms)
}

/// A year in file names: four digits ("0999"), signed outside 0 to 9999 ("-0001").
fn year4(y: i16) -> String {
    if (0..=9999).contains(&y) { format!("{y:04}") } else { format!("{y:+05}") }
}

/// 0 = Sunday … 6 = Saturday, in the local day of `ms`.
pub fn weekday(ms: i64) -> Option<u32> {
    local(ms).map(|d| d.weekday().to_sunday_zero_offset() as u32)
}

/// "Thursday" / "jeudi" (in the app's language).
pub fn weekday_name(ms: i64) -> String {
    weekday(ms).map(|d| crate::i18n::weekday(d).to_string()).unwrap_or_else(|| tr!("today", "aujourd’hui"))
}

/// The date and time for a file's name: the computer's clock, else UTC (a moment out of the
/// calendar: its nearest end). Never none: a file is never named "dobo--daily".
fn for_names(ms: i64) -> DateTime {
    local(ms).unwrap_or_else(|| TimeZone::UTC.to_datetime(Timestamp::from_millisecond(ms).unwrap_or(if ms < 0 { Timestamp::MIN } else { Timestamp::MAX })))
}

/// "2026-11-18_173000" for file names.
pub fn stamp(ms: i64) -> String {
    let d = for_names(ms);
    format!("{}-{:02}-{:02}_{:02}{:02}{:02}", year4(d.year()), d.month(), d.day(), d.hour(), d.minute(), d.second())
}

/// "2026-11-18" (the local date; for file names and to compare days).
pub fn local_date(ms: i64) -> String {
    let d = for_names(ms);
    format!("{}-{:02}-{:02}", year4(d.year()), d.month(), d.day())
}

/// "HH:MM" (24-hour) for comparing and storing. What people see goes through `show_hhmm`.
pub fn local_hhmm(ms: i64) -> String {
    local(ms).map(|d| format!("{:02}:{:02}", d.hour(), d.minute())).unwrap_or_default()
}

// ---- 12 or 24 hours, as Windows shows the time ----

static HOUR12: AtomicBool = AtomicBool::new(false);

// Tests run side by side: each test thread keeps its own (24-hour unless it says).
#[cfg(test)]
thread_local! {
    static TEST_HOUR12: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Does Windows show the time on a 12-hour clock ("3:00 PM")? Dobo shows its times the same way;
/// the words stay in the app's own language.
pub fn hour12() -> bool {
    #[cfg(test)]
    {
        TEST_HOUR12.with(|h| h.get())
    }
    #[cfg(not(test))]
    {
        HOUR12.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
pub fn set_hour12(on: bool) {
    TEST_HOUR12.with(|h| h.set(on));
}

/// The setting as Windows has it now, once read (`refresh_hour12`): what the app shows.
#[cfg(test)]
fn hour12_read() -> bool {
    HOUR12.load(Ordering::Relaxed)
}

/// A time format as Windows writes it ("HH:mm", "h:mm tt"): 12-hour when its hours are "h"
/// (H is the 24-hour clock).
pub fn is_12_hour_format(f: &str) -> bool {
    // (Quoted text, such as the 'h' of a French "HH' h 'mm", is words, not the hours.)
    let mut quoted = false;
    let mut twelve = false;
    for c in f.chars() {
        match c {
            '\'' => quoted = !quoted,
            'H' if !quoted => return false,
            'h' if !quoted => twelve = true,
            _ => {}
        }
    }
    twelve
}

/// Windows' short time format for this user (Settings › Time & language › Region).
fn windows_hour12() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_SSHORTTIME, LOCALE_STIMEFORMAT};
        let read = |what: u32| {
            let mut buf = [0u16; 80];
            // (No name: the user's own locale, with the changes they made to its formats.)
            let n = unsafe { GetLocaleInfoEx(windows::core::PCWSTR::null(), what, Some(&mut buf)) };
            (n > 1).then(|| String::from_utf16_lossy(&buf[..(n as usize - 1)]))
        };
        read(LOCALE_SSHORTTIME).or_else(|| read(LOCALE_STIMEFORMAT)).is_some_and(|f| is_12_hour_format(&f))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Reads Windows' setting again (at start, and when Windows says its settings changed). Returns
/// whether it changed.
pub fn refresh_hour12() -> bool {
    let now = windows_hour12();
    HOUR12.swap(now, Ordering::Relaxed) != now
}

/// A time as people see it: "15:00", or "3:00 PM" when Windows uses a 12-hour clock.
pub fn show_hhmm(ms: i64) -> String {
    let Some(d) = local(ms) else { return String::new() };
    format_hm(d.hour() as u32, d.minute() as u32, hour12())
}

/// "15:00" or "3:00 PM" (the space before AM/PM never breaks).
pub fn format_hm(h: u32, m: u32, twelve: bool) -> String {
    if !twelve {
        return format!("{h:02}:{m:02}");
    }
    let h12 = if h.is_multiple_of(12) { 12 } else { h % 12 };
    format!("{h12}:{m:02}\u{a0}{}", if h < 12 { "AM" } else { "PM" })
}

/// That day's "HH:MM" (the day of `ms`, local), in milliseconds. On the night the clocks go
/// forward, a time that doesn't exist that night is the first minute after it that does.
pub fn today_at(ms: i64, hhmm: &str) -> Option<i64> {
    day_at(ms, 0, hhmm)
}

/// "HH:MM" `days` calendar days after the day of `ms` (1: tomorrow's). Never `ms` + 24 hours:
/// the days the clocks change are 23 or 25 hours long.
pub fn day_at(ms: i64, days: i64, hhmm: &str) -> Option<i64> {
    day_at_in(&Local, ms, days, hhmm)
}

/// `day_at` in a given time zone (the tests' own zones with clock changes).
pub(crate) fn day_at_in(tz: &impl Zone, ms: i64, days: i64, hhmm: &str) -> Option<i64> {
    let day = tz.civil(ms)?.date().checked_add(Span::new().try_days(days).ok()?).ok()?;
    let t = crate::repeat::parse_hhmm(hhmm)?;
    crate::repeat::to_ms_in(tz, day.to_datetime(t))
}

/// The day of `ms` when it isn't `now`'s, as it sits in a sentence ("Was due yesterday at
/// 22:00", "Due Thursday at 09:00"): "yesterday" / "hier", "tomorrow" / "demain", a weekday
/// within the week, else the date ("3 Dec" / "3 déc."). None on the same day.
pub fn day_word(ms: i64, now: i64) -> Option<String> {
    let d = local(ms)?.date();
    let today = local(now)?.date();
    let days = days_between(today, d);
    Some(match days {
        0 => return None,
        -1 => tr!("yesterday", "hier"),
        1 => tr!("tomorrow", "demain"),
        n if n.abs() < 7 => crate::i18n::weekday(d.weekday().to_sunday_zero_offset() as u32).to_string(),
        _ => format!("{} {}", d.day(), crate::i18n::month_short(d.month() as u32)),
    })
}

/// Whole calendar days from `a` to `b` (negative when `b` comes first).
pub(crate) fn days_between(a: Date, b: Date) -> i64 {
    b.duration_since(a).as_hours() / 24
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32) -> i64 {
        crate::repeat::to_ms(jiff::civil::date(y as i16, m as i8, d as i8).at(h as i8, 0, 0, 0)).unwrap()
    }

    #[test]
    fn twelve_or_twenty_four_hours_as_windows_says() {
        assert!(!is_12_hour_format("HH:mm"));
        assert!(!is_12_hour_format("H:mm"));
        assert!(is_12_hour_format("h:mm tt"));
        assert!(is_12_hour_format("hh:mm tt"));
        assert!(!is_12_hour_format("HH' h 'mm"), "the quoted h is words");
        assert!(!is_12_hour_format(""));
        assert_eq!(format_hm(15, 0, false), "15:00");
        assert_eq!(format_hm(9, 5, false), "09:05");
        assert_eq!(format_hm(15, 0, true), "3:00\u{a0}PM");
        assert_eq!(format_hm(0, 30, true), "12:30\u{a0}AM");
        assert_eq!(format_hm(12, 0, true), "12:00\u{a0}PM");
        assert_eq!(format_hm(9, 5, true), "9:05\u{a0}AM");
        set_hour12(true);
        assert_eq!(show_hhmm(at(2026, 11, 18, 22)), "10:00\u{a0}PM");
        set_hour12(false);
        assert_eq!(show_hhmm(at(2026, 11, 18, 22)), "22:00");
    }

    /// Reading Windows' setting keeps it for the app, and says whether it changed.
    #[test]
    fn the_12_hour_clock_is_read_from_windows() {
        let windows = windows_hour12();
        refresh_hour12();
        assert_eq!(hour12_read(), windows);
        assert!(!refresh_hour12(), "unchanged: nothing to say");
        HOUR12.store(!windows, Ordering::Relaxed);
        assert!(refresh_hour12(), "changed");
        assert_eq!(hour12_read(), windows);
    }

    /// File names always have a date, even for a moment the clock can't show.
    #[test]
    fn file_name_stamps_are_never_empty() {
        for ms in [0, db_now(), i64::MAX, i64::MIN, -62_135_596_800_000] {
            let (s, d) = (stamp(ms), local_date(ms));
            assert_eq!((s.len() >= 17, d.len() >= 10), (true, true), "{ms}: {s} {d}");
        }
    }

    fn db_now() -> i64 {
        crate::db::now_ms()
    }

    #[test]
    fn another_day_says_which() {
        let now = at(2026, 11, 18, 9);
        assert_eq!(day_word(at(2026, 11, 18, 22), now), None);
        assert_eq!(day_word(at(2026, 11, 17, 22), now).as_deref(), Some("yesterday"));
        assert_eq!(day_word(at(2026, 11, 19, 0), now).as_deref(), Some("tomorrow"));
        assert_eq!(day_word(at(2026, 11, 21, 9), now).as_deref(), Some("Saturday"));
        assert_eq!(day_word(at(2026, 12, 3, 9), now).as_deref(), Some("3 Dec"));
        crate::i18n::set("fr");
        assert_eq!(day_word(at(2026, 11, 17, 22), now).as_deref(), Some("hier"));
        assert_eq!(day_word(at(2026, 12, 3, 9), now).as_deref(), Some("3 déc."));
        crate::i18n::set("en");
    }
}

/// Time zones with clock changes for the tests, whatever the computer's own zone: Paris and New
/// York (Europe: forward the last Sunday of March, back the last Sunday of October at 01:00 UTC,
/// in 2026 the 29 March and 25 October; US: forward the second Sunday of March, back the first
/// Sunday of November at 02:00 local, in 2026 the 8 March and 1 November).
#[cfg(test)]
pub(crate) mod zones {
    pub type Zone = jiff::tz::TimeZone;

    pub fn paris() -> Zone {
        Zone::posix("CET-1CEST,M3.5.0,M10.5.0/3").unwrap()
    }

    pub fn new_york() -> Zone {
        Zone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap()
    }
}

#[cfg(test)]
mod clock_change_tests {
    use super::zones::{new_york, paris};
    use super::*;

    fn hm(tz: &impl Zone, ms: i64) -> String {
        tz.civil(ms).unwrap().strftime("%d %H:%M").to_string()
    }

    fn at(tz: &impl Zone, m: u32, d: u32, h: u32, min: u32) -> i64 {
        crate::repeat::to_ms_in(tz, jiff::civil::date(2026, m as i8, d as i8).at(h as i8, min as i8, 0, 0)).unwrap()
    }

    #[test]
    fn tomorrow_is_the_next_calendar_day_across_the_clock_changes() {
        // Saturday 23:30 before each change: "tomorrow 17:30" is Sunday's (now + 24 h was Monday
        // in spring, the day being 23 hours long).
        for (tz, (m, d), (m2, d2)) in [(paris(), (3, 28), (10, 24)), (new_york(), (3, 7), (10, 31))] {
            assert_eq!(hm(&tz, day_at_in(&tz, at(&tz, m, d, 23, 30), 1, "17:30").unwrap()), format!("{:02} 17:30", d + 1));
            let next = if m2 == 10 && d2 == 31 { 1 } else { d2 + 1 };
            assert_eq!(hm(&tz, day_at_in(&tz, at(&tz, m2, d2, 23, 30), 1, "17:30").unwrap()), format!("{next:02} 17:30"));
        }
    }

    #[test]
    fn a_time_skipped_or_doubled_by_the_change() {
        let p = paris();
        // 02:30 doesn't exist on 29 March in Paris: the first minute after it that does.
        assert_eq!(hm(&p, day_at_in(&p, at(&p, 3, 29, 12, 0), 0, "02:30").unwrap()), "29 03:00");
        // 02:30 comes twice on 25 October: the first one (summer time, 00:30 UTC).
        let t = day_at_in(&p, at(&p, 10, 25, 12, 0), 0, "02:30").unwrap();
        assert_eq!(t / 1000, super::pin::utc_ms(2026, 10, 25, 0, 30, 0) / 1000);
        let ny = new_york();
        assert_eq!(hm(&ny, day_at_in(&ny, at(&ny, 3, 8, 12, 0), 0, "02:15").unwrap()), "08 03:00");
        // A day is 23 / 25 hours: midnight to midnight.
        let day = |tz: &super::zones::Zone, m, d| (day_at_in(tz, at(tz, m, d, 12, 0), 1, "00:00").unwrap() - day_at_in(tz, at(tz, m, d, 12, 0), 0, "00:00").unwrap()) / 3_600_000;
        assert_eq!((day(&p, 3, 29), day(&p, 10, 25), day(&ny, 3, 8), day(&ny, 11, 1)), (23, 25, 23, 25));
    }
}

/// What the pinning tests need from the date library: the only part of them that names it.
#[cfg(test)]
pub(crate) mod pin {
    use super::Zone;
    use jiff::civil::{DateTime, Time};
    use jiff::tz::TimeZone;

    pub type Civil = DateTime;

    /// A local (wall clock) date and time.
    pub fn civil(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> Civil {
        DateTime::new(y as i16, m as i8, d as i8, h as i8, mi as i8, s as i8, 0).unwrap()
    }

    /// Those fields read as UTC, in milliseconds.
    pub fn utc_ms(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
        TimeZone::UTC.to_timestamp(civil(y, m, d, h, mi, s)).unwrap().as_millisecond()
    }

    /// "2026-03-29 03:00:00" in UTC.
    pub fn utc_str(ms: i64) -> String {
        local_str(&TimeZone::UTC, ms)
    }

    /// "2026-03-29 03:00:00" as the zone's clock shows it.
    pub fn local_str(tz: &impl Zone, ms: i64) -> String {
        tz.civil(ms).unwrap().strftime("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// A time of day as (hours, minutes, seconds).
    pub fn hms(t: Option<Time>) -> Option<(u32, u32, u32)> {
        t.map(|t| (t.hour() as u32, t.minute() as u32, t.second() as u32))
    }
}

/// The local time behaviour pinned down on the clock changes (Paris and New York, spring and
/// autumn), written against chrono before Dobo moved to another date library: the same answers
/// after the move.
#[cfg(test)]
mod pinned_local_time {
    use super::pin::{civil, hms, local_str, utc_ms, utc_str};
    use super::zones::{Zone, new_york, paris};
    use super::*;

    const H: i64 = 3_600_000;

    /// A clock change in 2026: the offsets before and after (hours east of UTC), the hour skipped
    /// (spring) or doubled (autumn) on the local clock, and the moment in UTC.
    struct Change {
        m: u32,
        d: u32,
        before: i64,
        after: i64,
        hour: u32,
        utc_hour: u32,
    }

    fn changes() -> [(Zone, Change); 4] {
        [
            (paris(), Change { m: 3, d: 29, before: 1, after: 2, hour: 2, utc_hour: 1 }),
            (paris(), Change { m: 10, d: 25, before: 2, after: 1, hour: 2, utc_hour: 1 }),
            (new_york(), Change { m: 3, d: 8, before: -5, after: -4, hour: 2, utc_hour: 7 }),
            (new_york(), Change { m: 11, d: 1, before: -4, after: -5, hour: 1, utc_hour: 6 }),
        ]
    }

    /// The instant a wall-clock time stands for, as Dobo has it: before the change, the old
    /// offset; in a skipped hour, the first minute after it that exists (the seconds dropped); in
    /// a doubled hour, the first of the two (the old offset); after it, the new offset.
    fn expected(c: &Change, h: u32, mi: u32, s: u32) -> i64 {
        let as_utc = utc_ms(2026, c.m, c.d, h, mi, s);
        let skipped = c.after > c.before;
        if h < c.hour {
            as_utc - c.before * H
        } else if h == c.hour {
            if skipped { utc_ms(2026, c.m, c.d, c.hour + 1, 0, 0) - c.after * H } else { as_utc - c.before * H }
        } else {
            as_utc - c.after * H
        }
    }

    #[test]
    fn every_minute_around_each_change_to_an_instant() {
        for (tz, c) in changes() {
            for h in 0..=4 {
                for mi in 0..60 {
                    for s in [0, 37] {
                        let got = crate::repeat::to_ms_in(&tz, civil(2026, c.m, c.d, h, mi, s));
                        assert_eq!(got, Some(expected(&c, h, mi, s)), "{:02}-{:02} {h:02}:{mi:02}:{s:02}", c.m, c.d);
                    }
                }
            }
        }
    }

    #[test]
    fn every_minute_around_each_change_on_the_clock() {
        for (tz, c) in changes() {
            let change = utc_ms(2026, c.m, c.d, c.utc_hour, 0, 0);
            for k in -360..=360 {
                let ms = change + k * 60_000 + 30_500;
                let off = if ms < change { c.before } else { c.after };
                assert_eq!(local_str(&tz, ms), utc_str(ms + off * H), "{}", utc_str(ms));
            }
        }
    }

    #[test]
    fn a_time_on_a_given_day_on_each_change() {
        let (p, ny) = (paris(), new_york());
        let at = |tz: &Zone, m, d, hhmm: &str| day_at_in(tz, crate::repeat::to_ms_in(tz, civil(2026, m, d, 12, 0, 0)).unwrap(), 0, hhmm).map(utc_str);
        // Skipped (spring): the first minute after that exists.
        for hhmm in ["02:00", "02:01", "02:30", "02:59", "03:00"] {
            assert_eq!(at(&p, 3, 29, hhmm).as_deref(), Some("2026-03-29 01:00:00"), "Paris {hhmm}");
            assert_eq!(at(&ny, 3, 8, hhmm).as_deref(), Some("2026-03-08 07:00:00"), "New York {hhmm}");
        }
        assert_eq!(at(&p, 3, 29, "01:59").as_deref(), Some("2026-03-29 00:59:00"));
        assert_eq!(at(&p, 3, 29, "03:01").as_deref(), Some("2026-03-29 01:01:00"));
        assert_eq!(at(&ny, 3, 8, "01:59").as_deref(), Some("2026-03-08 06:59:00"));
        // Doubled (autumn): the first of the two.
        assert_eq!(at(&p, 10, 25, "01:59").as_deref(), Some("2026-10-24 23:59:00"));
        assert_eq!(at(&p, 10, 25, "02:00").as_deref(), Some("2026-10-25 00:00:00"));
        assert_eq!(at(&p, 10, 25, "02:30").as_deref(), Some("2026-10-25 00:30:00"));
        assert_eq!(at(&p, 10, 25, "02:59").as_deref(), Some("2026-10-25 00:59:00"));
        assert_eq!(at(&p, 10, 25, "03:00").as_deref(), Some("2026-10-25 02:00:00"));
        assert_eq!(at(&ny, 11, 1, "00:59").as_deref(), Some("2026-11-01 04:59:00"));
        assert_eq!(at(&ny, 11, 1, "01:00").as_deref(), Some("2026-11-01 05:00:00"));
        assert_eq!(at(&ny, 11, 1, "01:30").as_deref(), Some("2026-11-01 05:30:00"));
        assert_eq!(at(&ny, 11, 1, "02:00").as_deref(), Some("2026-11-01 07:00:00"));
        // From inside the doubled hour's second run (02:30 winter time in Paris, 01:30 in New
        // York): still that day, and the calendar days around it.
        let second_p = utc_ms(2026, 10, 25, 1, 30, 0);
        assert_eq!(local_str(&p, second_p), "2026-10-25 02:30:00");
        assert_eq!(day_at_in(&p, second_p, 0, "12:00").map(utc_str).as_deref(), Some("2026-10-25 11:00:00"));
        assert_eq!(day_at_in(&p, second_p, 0, "02:30").map(utc_str).as_deref(), Some("2026-10-25 00:30:00"));
        assert_eq!(day_at_in(&p, second_p, 1, "02:30").map(utc_str).as_deref(), Some("2026-10-26 01:30:00"));
        assert_eq!(day_at_in(&p, second_p, -1, "02:30").map(utc_str).as_deref(), Some("2026-10-24 00:30:00"));
        let second_ny = utc_ms(2026, 11, 1, 6, 30, 0);
        assert_eq!(local_str(&ny, second_ny), "2026-11-01 01:30:00");
        assert_eq!(day_at_in(&ny, second_ny, 0, "00:00").map(utc_str).as_deref(), Some("2026-11-01 04:00:00"));
        assert_eq!(day_at_in(&ny, second_ny, 1, "00:00").map(utc_str).as_deref(), Some("2026-11-02 05:00:00"));
        // Across the change by whole calendar days, never 24-hour steps.
        let sat = crate::repeat::to_ms_in(&p, civil(2026, 3, 28, 23, 30, 0)).unwrap();
        assert_eq!(day_at_in(&p, sat, 1, "00:00").map(utc_str).as_deref(), Some("2026-03-28 23:00:00"));
        assert_eq!(day_at_in(&p, sat, 2, "00:00").map(utc_str).as_deref(), Some("2026-03-29 22:00:00"));
        assert_eq!(day_at_in(&p, sat, 0, "bad"), None);
        assert_eq!(day_at_in(&p, sat, i64::MAX, "09:00"), None);
    }

    #[test]
    fn hours_and_minutes_read_as_before() {
        for (s, want) in [
            ("09:05", Some((9, 5, 0))),
            ("9:05", Some((9, 5, 0))),
            ("9:5", Some((9, 5, 0))),
            ("0:0", Some((0, 0, 0))),
            ("23:59", Some((23, 59, 0))),
            ("00:00", Some((0, 0, 0))),
            ("  09:05\n", Some((9, 5, 0))),
            ("9: 05", Some((9, 5, 0))),
            ("9:\u{a0}05", Some((9, 5, 0))),
            ("9 :05", None),
            ("24:00", None),
            ("23:60", None),
            ("09:05:00", None),
            ("009:05", None),
            ("09:005", None),
            ("0905", None),
            ("9h05", None),
            ("+9:05", None),
            ("9:-5", None),
            ("", None),
            (":", None),
            ("12:", None),
            (":30", None),
            ("\u{661}\u{662}:\u{663}\u{660}", None),
            ("09:05 PM", None),
        ] {
            assert_eq!(hms(crate::repeat::parse_hhmm(s)), want, "{s:?}");
            assert_eq!(today_at(utc_ms(2026, 11, 18, 12, 0, 0), s).is_some(), want.is_some(), "{s:?}");
        }
    }

    #[test]
    fn names_and_words_for_dates_as_before() {
        let t = |y, m, d, h, mi, s| crate::repeat::to_ms(civil(y, m, d, h, mi, s)).unwrap();
        // File names: the backups, broken-<date>.
        assert_eq!(stamp(t(2026, 1, 2, 3, 4, 5)), "2026-01-02_030405");
        assert_eq!(stamp(t(2026, 11, 22, 9, 0, 0) + 999), "2026-11-22_090000");
        assert_eq!(stamp(t(1970, 1, 1, 0, 0, 0) - 1), "1969-12-31_235959");
        assert_eq!(stamp(t(999, 12, 31, 23, 59, 59)), "0999-12-31_235959");
        assert_eq!(stamp(t(2099, 12, 31, 23, 59, 59)), "2099-12-31_235959");
        assert_eq!(local_date(t(2026, 3, 29, 23, 59, 59)), "2026-03-29");
        assert_eq!(local_date(t(2028, 2, 29, 0, 0, 0)), "2028-02-29");
        assert_eq!(local_hhmm(t(2026, 3, 29, 0, 7, 59)), "00:07");
        // The days of the week, Sunday first.
        assert_eq!(weekday(t(2026, 3, 29, 12, 0, 0)), Some(0));
        assert_eq!(weekday(t(2026, 11, 21, 23, 59, 59)), Some(6));
        assert_eq!(weekday_name(t(2026, 11, 19, 8, 0, 0)), "Thursday");
        // Out of any calendar: nothing (and never a panic); a file name still gets the calendar's
        // nearest end, never an empty date.
        for ms in [i64::MAX, i64::MIN] {
            assert_eq!((local_hhmm(ms), show_hhmm(ms)), (String::new(), String::new()));
            assert!(stamp(ms).len() >= 17 && local_date(ms).len() >= 10, "{} {}", stamp(ms), local_date(ms));
            assert_eq!((weekday(ms), weekday_name(ms)), (None, "today".to_string()));
            assert_eq!((today_at(ms, "09:00"), day_at(ms, 1, "09:00"), day_word(ms, t(2026, 1, 1, 0, 0, 0))), (None, None, None));
        }
        // The day words in notifications, at each end of the week.
        let now = t(2026, 3, 28, 23, 30, 0);
        assert_eq!(day_word(t(2026, 3, 29, 0, 0, 0), now).as_deref(), Some("tomorrow"));
        assert_eq!(day_word(t(2026, 3, 28, 0, 0, 0), now), None);
        assert_eq!(day_word(t(2026, 3, 22, 9, 0, 0), now).as_deref(), Some("Sunday"));
        assert_eq!(day_word(t(2026, 3, 21, 9, 0, 0), now).as_deref(), Some("21 Mar"));
        assert_eq!(day_word(t(2026, 4, 3, 9, 0, 0), now).as_deref(), Some("Friday"));
        assert_eq!(day_word(t(2026, 4, 4, 9, 0, 0), now).as_deref(), Some("4 Apr"));
        assert_eq!(day_word(t(2027, 3, 28, 9, 0, 0), now).as_deref(), Some("28 Mar"));
    }
}
