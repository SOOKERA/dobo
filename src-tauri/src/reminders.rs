// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The reminder clock. Two kinds of reminder:
//! - Reminder (the standard): rings at the time, then again every 5 min until answered, at
//!   most 6 times.
//! - Deadline: a rhythm. Starts before the time, rings more often as it nears, "I'm on it"
//!   silences it, then "Is it really done?" checks in later.
//!
//! The clock wakes every few seconds and compares with the wall clock, so it survives sleep:
//! after a long gap it rings once for the latest missed moment (never a flood).

use crate::db::{self, Item};
use crate::notify::{self, Notice};
use crate::AppState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

const MIN: i64 = 60_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Rhythm {
    /// First ring, minutes before the time (0: none).
    pub start: i64,
    /// More rings, minutes before the time.
    pub pre: Vec<i64>,
    /// After the time, minutes between rings.
    pub every: i64,
    /// Stop after this many rings nobody answered.
    pub max: i64,
    /// After "I'm on it", minutes until "Is it really done?".
    pub checkin: i64,
    /// After "Not yet", minutes until asking again.
    pub notyet: i64,
}

impl Rhythm {
    pub fn preset(name: &str) -> Rhythm {
        match name {
            "gentle" => Rhythm { start: 30, pre: vec![15], every: 10, max: 3, checkin: 45, notyet: 20 },
            "insistent" => Rhythm { start: 120, pre: vec![60, 30, 15, 10, 5], every: 3, max: 10, checkin: 20, notyet: 10 },
            _ => Rhythm { start: 60, pre: vec![30, 15, 5], every: 5, max: 6, checkin: 30, notyet: 15 },
        }
    }
    /// The standard reminder.
    pub fn standard() -> Rhythm {
        Rhythm { start: 0, pre: vec![], every: 5, max: 6, checkin: 30, notyet: 15 }
    }
    fn sane(mut self) -> Rhythm {
        self.every = self.every.clamp(1, 24 * 60);
        self.max = self.max.clamp(1, 50);
        self.checkin = self.checkin.clamp(1, 24 * 60);
        self.notyet = self.notyet.clamp(1, 24 * 60);
        self.start = self.start.clamp(0, 7 * 24 * 60);
        self.pre.retain(|p| *p > 0 && *p <= 7 * 24 * 60);
        self
    }
}

/// The rhythm an item uses: its own, else the default chosen in Settings.
pub fn rhythm_of(conn: &Connection, it: &Item) -> Rhythm {
    if it.alarm != "deadline" {
        return Rhythm::standard();
    }
    let default_preset = db::get_setting(conn, "deadline_preset").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "normal".into());
    let custom_default = || db::get_setting(conn, "deadline_custom").and_then(|v| serde_json::from_value::<Rhythm>(v).ok());
    let r = match &it.rhythm {
        Some(v) => match v.get("preset").and_then(|p| p.as_str()) {
            Some("custom") => serde_json::from_value::<Rhythm>(v.clone()).ok().or_else(custom_default),
            Some(p) => Some(Rhythm::preset(p)),
            None => None,
        },
        None => None,
    };
    let r = r.unwrap_or_else(|| if default_preset == "custom" { custom_default().unwrap_or_else(|| Rhythm::preset("normal")) } else { Rhythm::preset(&default_preset) });
    r.sane()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ring {
    /// Before the time (deadlines only).
    Early,
    /// At the time, or again after it.
    Due,
    /// "Is it really done?"
    Check,
}

/// Every moment this item should ring, from its current state.
pub fn slots(it: &Item, r: &Rhythm) -> Vec<(i64, Ring)> {
    let Some(due) = it.due_at else { return vec![] };
    // ("passed": a step whose time came while its task was done; quiet until its time changes.)
    if it.done_at.is_some() || it.alarm == "off" || it.kind != "task" || it.ring_state == "passed" {
        return vec![];
    }
    if let Some(c) = it.check_at {
        return (0..3).map(|k| (c.saturating_add(k * r.notyet * MIN), Ring::Check)).collect();
    }
    if it.on_it_at.is_some() {
        return vec![];
    }
    let deadline = it.alarm == "deadline";
    let mut v = Vec::new();
    let after = it.snooze_until.unwrap_or(i64::MIN);
    if let Some(s) = it.snooze_until {
        v.push((s, if s < due { Ring::Early } else { Ring::Due }));
    }
    if deadline {
        let mut befores: Vec<i64> = std::iter::once(r.start).chain(r.pre.iter().copied()).filter(|m| *m > 0).collect();
        befores.sort_unstable();
        befores.dedup();
        for m in befores {
            let t = due.saturating_sub(m * MIN);
            if t > after {
                v.push((t, Ring::Early));
            }
        }
    }
    // The chain at and after the time: restarts after a snooze that ends after the time.
    let base = if after > due { after } else { due };
    for k in 0..r.max {
        let t = base.saturating_add(k * r.every * MIN);
        if t > after || (k == 0 && after <= due) {
            v.push((t, Ring::Due));
        }
    }
    v.sort_by_key(|(t, _)| *t);
    v.dedup_by_key(|(t, _)| *t);
    v
}

/// How far ahead of the clock a last ring may be before it means the clock was moved back (set
/// wrong, ahead, then put right). Small corrections (the clock synced by a few seconds) are not.
const CLOCK_BACK: i64 = 10 * MIN;

/// The clock was moved back since this item last rang or was answered (its last ring is "in the
/// future"): what it was waiting for keeps its length from then (a 10 min snooze is 10 min from
/// now), and the rest follows from now. Otherwise a reminder that rang at the wrong time, ahead,
/// would never ring at its real time (it counts as rung until the clock passes that moment
/// again). Returns whether anything changed.
pub fn after_clock_back(it: &mut Item, now: i64) -> bool {
    let Some(last) = it.last_ring_at.filter(|l| *l > now + CLOCK_BACK) else { return false };
    let shift = |t: Option<i64>| t.map(|t| now.saturating_add(t.saturating_sub(last).max(0)));
    it.snooze_until = shift(it.snooze_until);
    it.check_at = shift(it.check_at);
    if it.on_it_at.is_some_and(|t| t > now) {
        it.on_it_at = Some(now);
    }
    // (Now: what already passed has rung; nothing rings twice for it.)
    it.last_ring_at = Some(now);
    it.rings = 0;
    true
}

/// The ring to play now, if any: the latest moment that passed since the last ring.
pub fn due_ring(it: &Item, r: &Rhythm, now: i64) -> Option<(i64, Ring)> {
    let last = it.last_ring_at.unwrap_or(i64::MIN);
    slots(it, r).into_iter().rev().find(|(t, _)| *t > last && *t <= now)
}

/// Updates the item for a ring played now.
pub fn apply_ring(it: &mut Item, ring: Ring, now: i64) {
    it.last_ring_at = Some(now);
    match ring {
        Ring::Early => it.ring_state = "early".into(),
        Ring::Due => {
            it.ring_state = "ringing".into();
            it.rings += 1;
        }
        Ring::Check => it.ring_state = "check".into(),
    }
}

/// The longest "snooze:<minutes>" and the farthest "until:<ms>" (Later…) accepted.
const SNOOZE_MAX_MIN: i64 = 7 * 24 * 60;
const UNTIL_MAX_MS: i64 = 400 * 24 * 60 * MIN;

/// The answers a notification button, a `ztodo:` link or the Now card may give. Anything else
/// (a link made up elsewhere) is refused before it touches the item.
pub fn valid_action(action: &str) -> bool {
    match action {
        "done" | "onit" | "notyet" => true,
        a => {
            if let Some(m) = a.strip_prefix("snooze:") {
                m.parse::<i64>().is_ok()
            } else if let Some(t) = a.strip_prefix("until:") {
                t.parse::<i64>().is_ok()
            } else {
                false
            }
        }
    }
}

/// An answer from a notification button or the Now card.
pub fn apply_answer(it: &mut Item, r: &Rhythm, action: &str, now: i64) {
    match action {
        "done" => {
            it.done_at = Some(now);
            it.ring_state = String::new();
            it.check_at = None;
        }
        "onit" => {
            it.on_it_at = Some(now);
            it.check_at = Some(now + r.checkin * MIN);
            it.snooze_until = None;
            it.ring_state = "on_it".into();
            it.rings = 0;
        }
        "notyet" => {
            it.check_at = Some(now + r.notyet * MIN);
            it.ring_state = "on_it".into();
        }
        a if a.starts_with("snooze:") || a.starts_with("until:") => {
            // "snooze:10" (minutes from now) or "until:<ms>" (Later…: this evening, tomorrow…).
            let until = match a.strip_prefix("until:") {
                Some(ms) => ms.parse::<i64>().ok().filter(|t| *t > now).map(|t| t.min(now + UNTIL_MAX_MS)).unwrap_or(now + 10 * MIN),
                None => now + a[7..].parse::<i64>().unwrap_or(10).clamp(1, SNOOZE_MAX_MIN) * MIN,
            };
            it.snooze_until = Some(until);
            it.on_it_at = None;
            it.check_at = None;
            it.ring_state = "snoozed".into();
            it.rings = 0;
        }
        _ => return,
    }
    it.last_ring_at = Some(now);
}

fn hhmm(ms: i64) -> String {
    crate::clock::show_hhmm(ms)
}

fn notice_for(it: &Item, ring: Ring, note_title: &str, now: i64) -> Notice {
    use crate::i18n::{duration, ordinal};
    let note_title = crate::i18n::note_title(note_title);
    let due = it.due_at.unwrap_or(now);
    let deadline = it.alarm == "deadline";
    let ahead = due.saturating_sub(now);
    let mut buttons: Vec<(String, String)> = Vec::new();
    let (title, body) = match ring {
        Ring::Check => {
            buttons.push((tr!("Yes, done", "Oui, fini"), "done".into()));
            buttons.push((tr!("Not yet", "Pas encore"), "notyet".into()));
            (tr!("Is “{}” really done?", "« {} », vraiment fini ?", it.text), tr!("You said you were on it · {}", "Vous vous en occupiez · {}", note_title))
        }
        // (A time on another day says which: a reminder missed overnight rings "Was due
        // yesterday at 22:00", never as if it were tonight's.)
        Ring::Early => {
            let left = (ahead as f64 / MIN as f64).round().max(1.0) as i64;
            let body = match crate::clock::day_word(due, now) {
                Some(day) => tr!("Due {} at {} · in {} · {}", "Pour {} à {} · dans {} · {}", day, hhmm(due), duration(left), note_title),
                None => tr!("Due at {} · in {} · {}", "Pour {} · dans {} · {}", hhmm(due), duration(left), note_title),
            };
            (it.text.clone(), body)
        }
        Ring::Due => {
            let late = ahead < -MIN;
            let when = match crate::clock::day_word(due, now) {
                Some(day) => tr!("{} at {}", "{} à {}", day, hhmm(due)),
                None => tr!("{}", "à {}", hhmm(due)),
            };
            let body = if !late {
                tr!("Due now · {}", "C’est l’heure · {}", note_title)
            } else if it.rings > 1 {
                tr!("Was due {} · {} reminder · {}", "Prévu {} · {} rappel · {}", when, ordinal(it.rings), note_title)
            } else {
                tr!("Was due {} · {}", "Prévu {} · {}", when, note_title)
            };
            (it.text.clone(), body)
        }
    };
    if ring != Ring::Check {
        if deadline {
            buttons.push((tr!("I’m on it", "Je m’en occupe"), "onit".into()));
            buttons.push((tr!("Done", "Fait"), "done".into()));
            buttons.push(("10 min".into(), "snooze:10".into()));
        } else {
            buttons.push((tr!("Done", "Fait"), "done".into()));
            buttons.push(("10 min".into(), "snooze:10".into()));
            buttons.push((tr!("1 hour", "1 h"), "snooze:60".into()));
        }
    }
    let mut buttons: Vec<(String, String)> = buttons.into_iter().map(|(l, a)| (l, format!("ring?item={}&a={}", it.id, a))).collect();
    // "Later…" opens the panel on the Now card with its choices (this evening, tomorrow…).
    if ring != Ring::Check {
        buttons.push((tr!("Later…", "Plus tard…"), format!("open?item={}&later=1", it.id)));
    }
    Notice {
        tag: format!("i{}", it.id),
        title,
        body,
        buttons,
        open: format!("open?item={}", it.id),
        // Stays on screen until answered, except the early, gentler rings.
        persistent: ring != Ring::Early,
    }
}

/// The notification's tag when several reminders ring together.
pub const GROUP_TAG: &str = "rings";

/// A reminder the grouped notification names: its line, its words, and whether its own
/// notification would stay on screen until answered.
#[derive(Clone, Debug, PartialEq)]
struct Member {
    id: i64,
    text: String,
    persistent: bool,
}

/// The reminders the grouped notification names: once all of them are answered, ticked or gone,
/// it goes too.
static GROUPED: std::sync::Mutex<Vec<Member>> = std::sync::Mutex::new(Vec::new());

/// The reminders a new grouped notification names: those the one it replaces still names (not
/// answered yet: they would be hidden otherwise), then those ringing now (`rang`), each once.
fn group_members(waiting: &[Member], rang: &[(Notice, RingEvent)]) -> Vec<Member> {
    let now: Vec<Member> = rang.iter().map(|(n, e)| Member { id: e.item.id, text: e.item.text.clone(), persistent: n.persistent }).collect();
    let mut all: Vec<Member> = waiting.iter().filter(|w| !now.iter().any(|n| n.id == w.id)).cloned().collect();
    all.extend(now);
    all
}

/// Several reminders ringing at once (due together, or come due while the computer slept): one
/// notification, "3 reminders", with the first ones' words and a button that opens Dobo on them
/// (the Now card has each one's answers). One alone keeps its own notification, with its buttons.
/// (`members`: `group_members`.)
fn group_notice(members: &[Member]) -> Option<Notice> {
    if members.len() < 2 {
        return None;
    }
    const SHOWN: usize = 3;
    let short = |s: &str| if s.chars().count() > 40 { format!("{}…", s.chars().take(38).collect::<String>().trim_end()) } else { s.to_string() };
    let mut body = members.iter().take(SHOWN).map(|m| short(&m.text)).collect::<Vec<_>>().join(" · ");
    if members.len() > SHOWN {
        let more = members.len() - SHOWN;
        body.push_str(&trn!(more, " · and {} more", " · and {} more", " · et {} autre", " · et {} autres", more));
    }
    // (The first one's note opens; the Now card shows them all, the first one first.)
    let open = format!("open?item={}", members[0].id);
    Some(Notice {
        tag: GROUP_TAG.into(),
        title: tr!("{} reminders", "{} rappels", members.len()),
        body,
        buttons: vec![(tr!("Open {}", "Ouvrir {}", crate::APP_NAME), open.clone())],
        open,
        persistent: members.iter().any(|m| m.persistent),
    })
}

/// A reminder's notification goes (answered, ticked, deleted, put away): its own, and the grouped
/// one once nothing it names still waits.
pub fn clear_ring(app: &AppHandle, id: i64) {
    notify::clear(app, &format!("i{id}"));
    let mut g = GROUPED.lock().unwrap();
    if g.iter().any(|m| m.id == id) {
        g.retain(|m| m.id != id);
        if g.is_empty() {
            notify::clear(app, GROUP_TAG);
        }
    }
}

/// What a ring tells the page: the item changed and (maybe) which space to show.
#[derive(Serialize, Clone)]
pub struct RingEvent {
    pub item: Item,
    pub space: String,
    pub note_title: String,
}

fn note_of(conn: &Connection, it: &Item) -> (String, String) {
    db::get_note(conn, it.note_id).map(|n| (n.title, n.space)).unwrap_or_else(|_| ("Dobo".into(), "work".into()))
}

/// One look at the clock: rings what is due now.
pub fn tick(app: &AppHandle) {
    let now = db::now_ms();
    let mut rang: Vec<(Notice, RingEvent)> = Vec::new();
    let tidy: Option<Notice>;
    let look: Option<Notice>;
    {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        if let Ok(items) = db::timed_items(&conn) {
            for mut it in items {
                let r = rhythm_of(&conn, &it);
                if after_clock_back(&mut it, now) {
                    let _ = db::save_ring(&conn, &it);
                }
                if let Some((_, ring)) = due_ring(&it, &r, now) {
                    apply_ring(&mut it, ring, now);
                    if db::save_ring(&conn, &it).is_ok() {
                        let (title, space) = note_of(&conn, &it);
                        rang.push((notice_for(&it, ring, &title, now), RingEvent { item: it, space, note_title: title }));
                    }
                }
            }
        }
        tidy = tidy_tick(&conn, now);
        look = lookback_tick(&conn, now);
        // Shown while the notes are still held: an answer saved right after (it waits for them)
        // clears its notification after this, never before.
        // (A new group replaces the one on screen: it names those still waiting there too.)
        let members = (rang.len() >= 2).then(|| group_members(&GROUPED.lock().unwrap(), &rang));
        if let Some((members, g)) = members.and_then(|m| group_notice(&m).map(|g| (m, g))) {
            // (Each one's own notification from an earlier ring goes: the group names it now.)
            for (_, ev) in &rang {
                notify::clear(app, &format!("i{}", ev.item.id));
            }
            *GROUPED.lock().unwrap() = members;
            notify::show(app, &g);
        } else {
            for (n, ev) in &rang {
                // (Ringing on its own again: it isn't one of the group's any more.)
                GROUPED.lock().unwrap().retain(|m| m.id != ev.item.id);
                notify::show(app, n);
            }
        }
        for n in tidy.iter().chain(look.iter()) {
            notify::show(app, n);
        }
    }
    for (_, ev) in rang {
        let _ = app.emit("ring", ev);
    }
    if tidy.is_some() {
        let _ = app.emit("tidy-ring", ());
    }
    if look.is_some() {
        let _ = app.emit("lookback-ready", ());
    }
    crate::look::tick(app, now);
    crate::backup::maybe_daily(app, now);
    // (Also heard at once from Windows, navkeys.rs; this catches a change made while no window
    // was there to hear it.)
    if crate::clock::refresh_hour12() {
        let _ = app.emit("clock-format", crate::clock::hour12());
    }
}

// ---- The evening look back ----

/// Work hours as Settings saves them (the page's defaults where nothing was saved).
struct WorkHours {
    on: bool,
    /// 0 = Sunday … 6 = Saturday.
    days: Vec<u64>,
    from: String,
    until: String,
}

fn work_hours(conn: &Connection) -> WorkHours {
    let wh = db::get_setting(conn, "work_hours").unwrap_or(serde_json::Value::Null);
    let time = |k: &str, d: &str| wh.get(k).and_then(|v| v.as_str()).filter(|s| crate::look::valid_hhmm(s)).unwrap_or(d).to_string();
    WorkHours {
        on: wh.get("on").and_then(|v| v.as_bool()).unwrap_or(false),
        // (Without days saved: Monday to Friday, as the page counts them, so the notification and
        // the look back card come at the same time.)
        days: wh.get("days").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|d| d.as_u64()).collect()).unwrap_or_else(|| vec![1, 2, 3, 4, 5]),
        from: time("from", "09:00"),
        until: time("until", "19:00"),
    }
}

impl WorkHours {
    /// 22:00 to 06:00: they end the next morning.
    fn crosses_midnight(&self) -> bool {
        self.until < self.from
    }

    /// A work day: work hours on, and that day of the week ticked.
    fn works_on(&self, ms: i64) -> bool {
        self.on && crate::clock::weekday(ms).is_some_and(|d| self.days.contains(&(d as u64)))
    }
}

/// What the look back of the day starting at `midnight` counts, from..until, in step with the
/// page's `lookbackSpan` (plan.ts): its calendar day, but a work day whose hours cross midnight
/// runs on to their end the next morning, and the day after such a night starts where that night
/// ended (nothing counted twice, nothing from the night left out).
fn lookback_span(wh: &WorkHours, midnight: i64) -> Option<(i64, i64)> {
    let next_midnight = crate::clock::day_at(midnight, 1, "00:00")?;
    if !wh.crosses_midnight() {
        return Some((midnight, next_midnight));
    }
    // (The night before ends this morning; this day's night, tomorrow morning.)
    let start = if wh.works_on(crate::clock::day_at(midnight, -1, "12:00")?) { midnight.max(crate::clock::day_at(midnight, 0, &wh.until)?) } else { midnight };
    let end = if wh.works_on(crate::clock::day_at(midnight, 0, "12:00")?) { next_midnight.max(crate::clock::day_at(midnight, 1, &wh.until)?) } else { next_midnight };
    Some((start, end))
}

/// When the look back of a day comes (`days` after the day of `now`: 0 today's, -1 yesterday's):
/// the end of work hours on a work day (the next morning, for hours that cross midnight), else
/// the end of the day ("My day ends at", 22:00 by default).
pub fn lookback_at(conn: &Connection, now: i64, days: i64) -> Option<i64> {
    let wh = work_hours(conn);
    let weekday = crate::clock::weekday(crate::clock::day_at(now, days, "12:00")?)?;
    if wh.on && wh.days.contains(&(weekday as u64)) {
        return crate::clock::day_at(now, days + wh.crosses_midnight() as i64, &wh.until);
    }
    let end = db::get_setting(conn, "day").and_then(|d| d.get("end").and_then(|v| v.as_str()).map(String::from)).unwrap_or_else(|| "22:00".into());
    crate::clock::day_at(now, days, &end)
}

/// The look back that is due now, with the midnight of the day it looks back on: today's once its
/// time has come; before that, yesterday's when it comes today (after work hours that crossed
/// midnight).
fn lookback_due(conn: &Connection, now: i64) -> Option<(i64, i64)> {
    let midnight = crate::clock::today_at(now, "00:00")?;
    let today = lookback_at(conn, now, 0)?;
    if now >= today {
        return Some((today, midnight));
    }
    let yesterday = lookback_at(conn, now, -1)?;
    (yesterday >= midnight && now >= yesterday).then_some((yesterday, crate::clock::day_at(now, -1, "00:00")?))
}

fn lookback_tick(conn: &Connection, now: i64) -> Option<Notice> {
    let (at, midnight) = lookback_due(conn, now)?;
    // (One "in the future": it came while the clock was set wrong, ahead; today's is still to come.)
    let last = db::get_setting(conn, "lookback_last").and_then(|v| v.as_i64()).filter(|l| *l <= now + CLOCK_BACK).unwrap_or(0);
    if last >= at {
        return None;
    }
    let _ = db::set_setting(conn, "lookback_last", &serde_json::json!(now));
    // Nothing happened that day (the look back would show nothing): no need to look back.
    let busy = lookback_count(conn, midnight)?;
    if busy == 0 {
        return None;
    }
    Some(Notice {
        tag: "lookback".into(),
        title: tr!("That was {}.", "C’était {}.", crate::clock::weekday_name(midnight)),
        body: tr!("Here is what happened, and what moves on · 2 minutes", "Voici ce qui s’est passé, et ce qui reste pour la suite · 2 minutes"),
        buttons: vec![],
        open: format!("lookback?day={midnight}"),
        persistent: false,
    })
}

/// What the look back of the day starting at `midnight` shows, counted as LookBack.svelte lists
/// it: the tasks ticked in its span, the tasks still open that were due that day or the day
/// before (up to its span's end), and the ideas written in its span.
fn lookback_count(conn: &Connection, midnight: i64) -> Option<i64> {
    let (start, end) = lookback_span(&work_hours(conn), midnight)?;
    // (The day before: 23 or 25 hours long when the clocks change.)
    let day_before = crate::clock::day_at(midnight, -1, "00:00")?;
    conn.query_row(
        "SELECT COUNT(*) FROM items WHERE
           (kind = 'task' AND done_at >= ?1 AND done_at < ?2)
           OR (kind = 'task' AND done_at IS NULL AND due_at >= ?3 AND due_at < ?2)
           OR (kind = 'idea' AND created_at >= ?1 AND created_at < ?2)",
        rusqlite::params![start, end, day_before],
        |r| r.get(0),
    )
    .ok()
}

/// An answer (notification button, Now card). Returns the updated item.
pub fn answer(app: &AppHandle, item_id: i64, action: &str) -> Result<Item, String> {
    if !valid_action(action) {
        return Err(tr!("Unknown answer: {}", "Réponse inconnue : {}", action));
    }
    let now = db::now_ms();
    let (it, next, steps) = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let saved = save_answer(&conn, item_id, action, now)?;
        // (While the notes are held: after a ring saved just before, never before its notification.)
        clear_ring(app, item_id);
        // Done: its steps' reminders are paused with it.
        for s in &saved.2 {
            clear_ring(app, s.id);
        }
        saved
    };
    let _ = app.emit("item-updated", it.clone());
    for n in next.into_iter().chain(steps) {
        let _ = app.emit("item-updated", n);
    }
    Ok(it)
}

/// The answer saved, with a repeating task's next occurrence, all or nothing: never a task done
/// whose next one wasn't made (the series would stop there).
fn save_answer(conn: &Connection, item_id: i64, action: &str, now: i64) -> Result<db::Saved, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut it = db::get_item(conn, item_id)?;
    // An old notification's button (the task ticked meanwhile, on the board or from another
    // notification, or made an idea): nothing to answer. "Done" again would move when it was
    // done (into today's look back), a snooze would wake a finished task.
    if it.done_at.is_some() || it.kind != "task" {
        return Ok((it, None, vec![]));
    }
    let was_open = it.done_at.is_none();
    let r = rhythm_of(conn, &it);
    apply_answer(&mut it, &r, action, now);
    db::save_ring(conn, &it)?;
    // A repeating task: its next occurrence.
    let done = was_open && it.done_at.is_some();
    let next = if done { db::spawn_next(conn, &it)? } else { None };
    let steps = if done { db::after_done(conn, &it)? } else { vec![] };
    let steps = steps.into_iter().map(|id| db::get_item(conn, id)).collect::<Result<Vec<_>, _>>()?;
    let saved = db::get_item(conn, item_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((saved, next, steps))
}

// ---- The tidy-the-inbox reminder ----

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TidySettings {
    /// Daily at this time ("17:30"), when on.
    pub daily: bool,
    pub time: String,
    /// And/or when this many things wait.
    pub when_many: bool,
    pub many: i64,
}

impl Default for TidySettings {
    fn default() -> Self {
        TidySettings { daily: true, time: "17:30".into(), when_many: true, many: 10 }
    }
}

pub fn tidy_settings(conn: &Connection) -> TidySettings {
    db::get_setting(conn, "tidy").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
}

fn tidy_tick(conn: &Connection, now: i64) -> Option<Notice> {
    let s = tidy_settings(conn);
    let waiting = db::inbox_count(conn);
    if waiting == 0 {
        return None;
    }
    let mut last = db::get_setting(conn, "tidy_last").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut snooze = db::get_setting(conn, "tidy_snooze").and_then(|v| v.as_i64()).unwrap_or(0);
    // The clock was moved back since (it rang or was answered "in the future"): from now, a
    // snooze keeping its length; else it would stay quiet until the clock got there again.
    if last > now + CLOCK_BACK {
        snooze = if snooze > last { now + (snooze - last) } else { 0 };
        last = now;
        let _ = db::set_setting(conn, "tidy_last", &serde_json::json!(last));
        let _ = db::set_setting(conn, "tidy_snooze", &serde_json::json!(snooze));
    }
    if snooze > now {
        return None;
    }
    let mut ring = snooze > 0 && snooze > last;
    if !ring && s.daily
        && let Some(today_at) = crate::clock::today_at(now, &s.time) {
            ring = now >= today_at && last < today_at;
        }
    // "When many wait": every 3 hours at most, and only during the day ("My day", 08:00 to
    // 22:00 at first), never in the middle of the night.
    if !ring && s.when_many && waiting >= s.many && in_my_day(conn, now) {
        ring = now - last > 3 * 60 * MIN;
    }
    if !ring {
        return None;
    }
    let _ = db::set_setting(conn, "tidy_last", &serde_json::json!(now));
    let _ = db::set_setting(conn, "tidy_snooze", &serde_json::json!(0));
    let minutes = ((waiting as f64) * 0.4).ceil().max(1.0) as i64;
    Some(Notice {
        tag: "tidy".into(),
        title: tr!("Time to tidy your Inbox", "Un moment pour faire le tri"),
        body: format!(
            "{} · {}",
            trn!(waiting, "{} thing waiting", "{} things waiting", "{} chose en attente", "{} choses en attente", waiting),
            trn!(minutes, "about {} minute", "about {} minutes", "environ {} minute", "environ {} minutes", minutes)
        ),
        buttons: vec![
            (tr!("Tidy now", "Trier maintenant"), "tidy?a=now".into()),
            (tr!("In 1 hour", "Dans 1 h"), "tidy?a=snooze:60".into()),
            (tr!("Tomorrow", "Demain"), "tidy?a=tomorrow".into()),
        ],
        open: "tidy?a=now".into(),
        persistent: false,
    })
}

/// Now is within "My day" (Settings › My day: 08:00 to 22:00 at first).
fn in_my_day(conn: &Connection, now: i64) -> bool {
    let day = db::get_setting(conn, "day").unwrap_or(serde_json::Value::Null);
    let get = |k: &str, d: &str| day.get(k).and_then(|v| v.as_str()).filter(|s| crate::look::valid_hhmm(s)).unwrap_or(d).to_string();
    crate::look::in_hours(&crate::clock::local_hhmm(now), &get("start", "08:00"), &get("end", "22:00"))
}

pub fn tidy_answer(app: &AppHandle, action: &str) {
    let now = db::now_ms();
    {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let until = if let Some(m) = action.strip_prefix("snooze:") {
            now + m.parse::<i64>().unwrap_or(60).clamp(1, SNOOZE_MAX_MIN) * MIN
        } else if action == "tomorrow" {
            let s = tidy_settings(&conn);
            // (The next calendar day: now + 24 hours skips Sunday when it is Saturday 23:30 and
            // the clocks go forward that night.)
            crate::clock::day_at(now, 1, &s.time).unwrap_or(now + 24 * 60 * MIN)
        } else {
            0
        };
        let _ = db::set_setting(&conn, "tidy_snooze", &serde_json::json!(until));
        let _ = db::set_setting(&conn, "tidy_last", &serde_json::json!(now));
    }
    notify::clear(app, "tidy");
    if action == "now" {
        crate::window::show(app);
        crate::to_page(app, "open-tidy", serde_json::Value::Null);
    }
}

/// The clock: a thread that looks every 5 seconds.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            tick(&app);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_and_the_next_occurrence_are_saved_together_or_not_at_all() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = db::now_ms();
        let due = crate::clock::today_at(now, "09:00").unwrap();
        let it = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Weekly", due_at: Some(due), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        let it = db::update_item(&c, &Item { repeat: Some(serde_json::json!({"freq": "week", "interval": 1})), ..it }).unwrap();
        // The next occurrence can't be made (here: refused by the file): the answer isn't kept
        // either, so the task is still open with its rule, and answering again works.
        c.execute_batch("CREATE TRIGGER no_new BEFORE INSERT ON items BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert!(save_answer(&c, it.id, "done", now).is_err());
        let kept = db::get_item(&c, it.id).unwrap();
        assert!(kept.done_at.is_none() && kept.repeat.is_some());
        c.execute_batch("DROP TRIGGER no_new").unwrap();
        let (done, next, _) = save_answer(&c, it.id, "done", now).unwrap();
        assert!(done.done_at.is_some() && done.repeat.is_none());
        assert!(next.expect("the next one").repeat.is_some());
    }

    #[test]
    fn the_clock_set_ahead_then_put_right_still_rings_at_the_real_time() {
        let due = 10_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        // The clock a day ahead: tomorrow's reminder rang (wrongly) and was snoozed for 10 min.
        let wrong = due + 2 * MIN;
        let (_, ring) = due_ring(&it, &r, wrong).unwrap();
        apply_ring(&mut it, ring, wrong);
        apply_answer(&mut it, &r, "snooze:10", wrong);
        // Put right: a day earlier. The snooze still lasts 10 minutes from now…
        let now = wrong - 24 * 60 * MIN;
        assert!(after_clock_back(&mut it, now));
        assert_eq!(it.snooze_until, Some(now + 10 * MIN));
        assert!(due_ring(&it, &r, now + 9 * MIN).is_none());
        assert!(due_ring(&it, &r, now + 10 * MIN).is_some());
        // …and one not snoozed rings at its real time, not only once the clock passes the wrong one.
        let mut it = task(due, "reminder");
        apply_ring(&mut it, Ring::Due, wrong);
        let now = wrong - 24 * 60 * MIN;
        assert!(after_clock_back(&mut it, now));
        assert!(due_ring(&it, &r, due - MIN).is_none());
        assert_eq!(due_ring(&it, &r, due).map(|(t, _)| t), Some(due));
        // A clock synced back by a few seconds is not a clock moved back: nothing rings twice.
        let mut it = task(due, "reminder");
        apply_ring(&mut it, Ring::Due, due);
        assert!(!after_clock_back(&mut it, due - 3_000));
        assert!(due_ring(&it, &r, due + 1_000).is_none());
    }

    #[test]
    fn the_look_back_and_the_tidy_reminder_after_the_clock_was_moved_back() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let now = db::now_ms();
        let day_end = crate::clock::today_at(now, "22:00").unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        db::create_item(&c, db::NewItem { note_id: n.id, kind: "idea", text: "Pay", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        // The look back came "tomorrow" (the clock was a day ahead): today's still comes.
        db::set_setting(&c, "lookback_last", &serde_json::json!(day_end + 24 * 60 * MIN)).unwrap();
        // (Its link names the day, so a click on it the next day opens that day's.)
        let n = lookback_tick(&c, day_end + MIN).expect("today's look back");
        assert_eq!(n.open, format!("lookback?day={}", crate::clock::today_at(now, "00:00").unwrap()));
        assert!(lookback_tick(&c, day_end + 2 * MIN).is_none());
        // The tidy reminder answered "In 1 hour" a day ahead: in an hour from now, not tomorrow.
        let inbox = crate::db::inbox_of(&c, "work").unwrap();
        if inbox.is_none() {
            let i = db::create_note(&c, "Inbox", "work", "soft", "Sage").unwrap();
            db::create_item(&c, db::NewItem { note_id: i.id, kind: "task", text: "Sort", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        }
        let wrong = now + 24 * 60 * MIN;
        db::set_setting(&c, "tidy", &serde_json::json!({"daily": false, "time": "17:30", "when_many": false, "many": 10})).unwrap();
        db::set_setting(&c, "tidy_last", &serde_json::json!(wrong)).unwrap();
        db::set_setting(&c, "tidy_snooze", &serde_json::json!(wrong + 60 * MIN)).unwrap();
        assert!(tidy_tick(&c, now).is_none());
        assert!(tidy_tick(&c, now + 59 * MIN).is_none());
        assert!(tidy_tick(&c, now + 61 * MIN).is_some());
    }

    #[test]
    fn the_look_back_comes_when_the_page_shows_it() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let now = db::now_ms();
        // A work day by the page's defaults (Monday to Friday) when only "on" was saved.
        let weekday = crate::clock::weekday(now).unwrap();
        db::set_setting(&c, "work_hours", &serde_json::json!({"on": true, "until": "18:00"})).unwrap();
        let want = if (1..=5).contains(&weekday) { "18:00" } else { "22:00" };
        assert_eq!(lookback_at(&c, now, 0), crate::clock::today_at(now, want));
        db::set_setting(&c, "work_hours", &serde_json::json!({"on": true, "days": [], "until": "18:00"})).unwrap();
        assert_eq!(lookback_at(&c, now, 0), crate::clock::today_at(now, "22:00"));
    }

    #[test]
    fn work_hours_across_midnight_look_back_when_they_end() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        // Monday 16 Nov 2026: a night from 22:00 to Tuesday 06:00, every night of the week.
        let at = |d: u32, h: u32| crate::repeat::to_ms(jiff::civil::date(2026, 11, d as i8).at(h as i8, 0, 0, 0)).unwrap();
        db::set_setting(&c, "work_hours", &serde_json::json!({"on": true, "days": [0, 1, 2, 3, 4, 5, 6], "from": "22:00", "until": "06:00"})).unwrap();
        db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Rounds", due_at: Some(at(16, 23)), parent_id: None, alarm: "off", link: None, file: None }).unwrap();
        assert_eq!(lookback_at(&c, at(16, 12), 0), Some(at(17, 6)));
        // Monday's comes on Tuesday at 06:00, named Monday, opening Monday's.
        assert!(lookback_tick(&c, at(17, 5)).is_none());
        let n = lookback_tick(&c, at(17, 6)).expect("Monday's look back");
        assert_eq!((n.title.as_str(), n.open.clone()), ("That was Monday.", format!("lookback?day={}", at(16, 0))));
        assert!(lookback_tick(&c, at(17, 9)).is_none(), "once");
        // Day hours: as before.
        db::set_setting(&c, "work_hours", &serde_json::json!({"on": true, "days": [1], "from": "09:00", "until": "17:00"})).unwrap();
        assert_eq!(lookback_at(&c, at(16, 12), 0), Some(at(16, 17)));
    }

    #[test]
    fn the_look_back_counts_what_the_page_shows_on_the_same_span() {
        // As logic.test.ts ("the look back counts the whole night, once"): Monday 16 Nov 2026, a
        // night from 22:00 to 06:00 on Monday to Friday.
        let at = |d: u32, h: u32| crate::repeat::to_ms(jiff::civil::date(2026, 11, d as i8).at(h as i8, 0, 0, 0)).unwrap();
        let night = WorkHours { on: true, days: vec![1, 2, 3, 4, 5], from: "22:00".into(), until: "06:00".into() };
        // Monday (no night before it): from midnight to Tuesday 06:00, when its night ends.
        assert_eq!(lookback_span(&night, at(16, 0)), Some((at(16, 0), at(17, 6))));
        // Tuesday: from where Monday's night ended.
        assert_eq!(lookback_span(&night, at(17, 0)), Some((at(17, 6), at(18, 6))));
        // Saturday (not a work day): from Friday's night's end to its own midnight.
        assert_eq!(lookback_span(&night, at(21, 0)), Some((at(21, 6), at(22, 0))));
        // Day hours: the calendar day.
        let day = WorkHours { on: true, days: vec![1, 2, 3, 4, 5], from: "09:00".into(), until: "19:00".into() };
        assert_eq!(lookback_span(&day, at(18, 0)), Some((at(18, 0), at(19, 0))));

        // Ticked in Monday's night (Tuesday 03:00): Monday's, never Tuesday's too.
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        db::set_setting(&c, "work_hours", &serde_json::json!({"on": true, "days": [1, 2, 3, 4, 5], "from": "22:00", "until": "06:00"})).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let it = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Rounds", due_at: None, parent_id: None, alarm: "off", link: None, file: None }).unwrap();
        c.execute("UPDATE items SET created_at = ?1, done_at = ?2 WHERE id = ?3", rusqlite::params![at(10, 12), at(17, 3), it.id]).unwrap();
        assert_eq!(lookback_count(&c, at(16, 0)), Some(1));
        assert_eq!(lookback_count(&c, at(17, 0)), Some(0));
        // Ticked in Tuesday's night (Wednesday 03:00): Tuesday's.
        c.execute("UPDATE items SET done_at = ?1 WHERE id = ?2", rusqlite::params![at(18, 3), it.id]).unwrap();
        assert_eq!(lookback_count(&c, at(17, 0)), Some(1));
        // Still open, due in Tuesday's night: Tuesday's (and, due "the day before", Wednesday's).
        c.execute("UPDATE items SET done_at = NULL, due_at = ?1 WHERE id = ?2", rusqlite::params![at(18, 2), it.id]).unwrap();
        assert_eq!(lookback_count(&c, at(17, 0)), Some(1));
        assert_eq!(lookback_count(&c, at(16, 0)), Some(0));
        // A task written without a time, and not ticked, isn't in the look back.
        c.execute("UPDATE items SET due_at = NULL, created_at = ?1 WHERE id = ?2", rusqlite::params![at(17, 12), it.id]).unwrap();
        assert_eq!(lookback_count(&c, at(17, 0)), Some(0));
        // An idea written that day is.
        c.execute("UPDATE items SET kind = 'idea' WHERE id = ?1", rusqlite::params![it.id]).unwrap();
        assert_eq!(lookback_count(&c, at(17, 0)), Some(1));
    }

    #[test]
    fn many_waiting_in_the_inbox_never_rings_in_the_night() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let inbox = db::create_note(&c, "Inbox", "work", "soft", "Sage").unwrap();
        for k in 0..12 {
            db::create_item(&c, db::NewItem { note_id: inbox.id, kind: "task", text: &format!("Line {k}"), due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        }
        db::set_setting(&c, "tidy", &serde_json::json!({"daily": false, "time": "17:30", "when_many": true, "many": 10})).unwrap();
        let now = db::now_ms();
        let night = crate::clock::today_at(now, "03:00").unwrap();
        assert!(tidy_tick(&c, night).is_none());
        assert!(tidy_tick(&c, crate::clock::today_at(now, "07:59").unwrap()).is_none());
        assert!(tidy_tick(&c, crate::clock::today_at(now, "08:00").unwrap()).is_some());
        // My day set to end at 01:00 (after midnight): 00:30 is still the day.
        db::set_setting(&c, "day", &serde_json::json!({"start": "08:00", "end": "01:00"})).unwrap();
        db::set_setting(&c, "tidy_last", &serde_json::json!(0)).unwrap();
        assert!(tidy_tick(&c, crate::clock::today_at(now, "00:30").unwrap()).is_some());
    }

    #[test]
    fn done_from_a_notification_pauses_the_steps_too() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = db::now_ms();
        let task = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Move", due_at: Some(now), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        let step = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Pack", due_at: Some(now), parent_id: Some(task.id), alarm: "reminder", link: None, file: None }).unwrap();
        let mut s = step.clone();
        apply_ring(&mut s, Ring::Due, now);
        db::save_ring(&c, &s).unwrap();
        let (_, _, steps) = save_answer(&c, task.id, "done", now).unwrap();
        assert_eq!((steps.len(), steps[0].ring_state.as_str()), (1, ""));
        assert!(db::timed_items(&c).unwrap().is_empty());
    }

    #[test]
    fn reminders_ringing_together_share_one_notification() {
        let ev = |id: i64, text: &str| {
            let it = Item { id, text: text.into(), ..task(1_000 * MIN, "reminder") };
            (notice_for(&it, Ring::Due, "Hub", 1_000 * MIN), RingEvent { item: it, space: "work".into(), note_title: "Hub".into() })
        };
        let group = |rang: &[(Notice, RingEvent)]| group_notice(&group_members(&[], rang));
        // One alone: its own notification, as before.
        assert!(group(&[ev(1, "Pay rent")]).is_none());
        let two = [ev(1, "Pay rent"), ev(2, "Call Sam")];
        let g = group(&two).unwrap();
        assert_eq!((g.title.as_str(), g.body.as_str(), g.tag.as_str()), ("2 reminders", "Pay rent · Call Sam", GROUP_TAG));
        assert_eq!(g.open, "open?item=1");
        assert_eq!(g.buttons, vec![("Open Dobo".to_string(), "open?item=1".to_string())]);
        assert!(g.persistent);
        let five: Vec<_> = (1..=5).map(|k| ev(k, &format!("Task {k} with a name long enough to be cut short somewhere"))).collect();
        let g = group(&five).unwrap();
        assert_eq!(g.title, "5 reminders");
        assert!(g.body.ends_with(" · and 2 more"), "{}", g.body);
        assert_eq!(g.body.matches('…').count(), 3);
        crate::i18n::set("fr");
        let g = group(&five[..4]).unwrap();
        assert_eq!((g.title.as_str(), g.buttons[0].0.as_str()), ("4 rappels", "Ouvrir Dobo"));
        assert!(g.body.ends_with(" · et 1 autre"), "{}", g.body);
        assert!(group(&five).unwrap().body.ends_with(" · et 2 autres"));
        crate::i18n::set("en");
    }

    /// A new group replaces the one on screen: those it still names (not answered yet) stay
    /// named, first, each once; never hidden.
    #[test]
    fn a_new_group_keeps_the_reminders_still_waiting_in_the_last_one() {
        let ev = |id: i64, text: &str| {
            let it = Item { id, text: text.into(), ..task(1_000 * MIN, "reminder") };
            (notice_for(&it, Ring::Due, "Hub", 1_000 * MIN), RingEvent { item: it, space: "work".into(), note_title: "Hub".into() })
        };
        let first = group_members(&[], &[ev(1, "Pay rent"), ev(2, "Call Sam")]);
        // "Pay rent" answered since (clear_ring takes it out); "Call Sam" still waits.
        let waiting: Vec<Member> = first.into_iter().filter(|m| m.id != 1).collect();
        // Two more ring, one of them "Call Sam" again: named once, with its words now.
        let next = group_members(&waiting, &[ev(3, "Water plants"), ev(2, "Call Sam today")]);
        assert_eq!(next.iter().map(|m| m.id).collect::<Vec<_>>(), [3, 2]);
        let next = group_members(&waiting, &[ev(3, "Water plants"), ev(4, "Book flights")]);
        assert_eq!(next.iter().map(|m| m.id).collect::<Vec<_>>(), [2, 3, 4]);
        let g = group_notice(&next).unwrap();
        assert_eq!((g.title.as_str(), g.body.as_str()), ("3 reminders", "Call Sam · Water plants · Book flights"));
        assert_eq!(g.open, "open?item=2");
    }

    #[test]
    fn an_old_notification_on_a_finished_task_changes_nothing() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = db::now_ms();
        let it = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Pay", due_at: Some(now), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        let (done, _, _) = save_answer(&c, it.id, "done", now - 3 * 60 * MIN).unwrap();
        for a in ["done", "snooze:10", "onit", "notyet", "until:99999999999999"] {
            let (again, next, _) = save_answer(&c, it.id, a, now).unwrap();
            assert!(next.is_none());
            assert_eq!((again.done_at, again.snooze_until, again.on_it_at, again.check_at), (done.done_at, None, None, None), "{a}");
        }
        // An idea now: a "Done" from the notification it had as a task doesn't tick it.
        let idea = db::create_item(&c, db::NewItem { note_id: n.id, kind: "idea", text: "Maybe", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        assert!(save_answer(&c, idea.id, "done", now).unwrap().0.done_at.is_none());
    }

    #[test]
    fn a_line_brought_back_by_undo_does_not_ring_again_at_once() {
        let mut c = db::open(std::path::Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = db::now_ms();
        let due = now - 30 * MIN;
        let it = db::create_item(&c, db::NewItem { note_id: n.id, kind: "task", text: "Pay", due_at: Some(due), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        // It rang, then was snoozed for an hour.
        save_answer(&c, it.id, "snooze:60", now).unwrap();
        let before = db::get_item(&c, it.id).unwrap();
        assert_eq!(before.ring_state, "snoozed");
        // Deleted, then Undo: still snoozed, quiet until then.
        db::delete_item(&c, it.id).unwrap();
        db::restore(&mut c, &[], std::slice::from_ref(&before), &[]).unwrap();
        let back = db::get_item(&c, it.id).unwrap();
        assert_eq!((back.ring_state.as_str(), back.snooze_until, back.last_ring_at), ("snoozed", before.snooze_until, before.last_ring_at));
        let r = Rhythm::standard();
        assert!(due_ring(&back, &r, now + MIN).is_none());
        // A line still there keeps its own (newer) state over an older copy.
        let stale = Item { ring_state: String::new(), snooze_until: None, last_ring_at: None, ..before.clone() };
        db::restore(&mut c, &[], &[stale], &[]).unwrap();
        assert_eq!(db::get_item(&c, it.id).unwrap().snooze_until, before.snooze_until);
    }

    fn task(due: i64, alarm: &str) -> Item {
        Item { id: 1, note_id: 1, kind: "task".into(), text: "Send build".into(), due_at: Some(due), alarm: alarm.into(), ..Default::default() }
    }

    #[test]
    fn standard_rings_at_the_time_then_every_5_min_6_times() {
        let due = 1_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        assert!(due_ring(&it, &r, due - 1).is_none());
        let mut count = 0;
        for m in 0..60 {
            let now = due + m * MIN;
            if let Some((_, ring)) = due_ring(&it, &r, now) {
                assert_eq!(ring, Ring::Due);
                apply_ring(&mut it, ring, now);
                count += 1;
            }
        }
        assert_eq!(count, 6);
        assert_eq!(it.rings, 6);
    }

    #[test]
    fn after_sleep_it_rings_once_not_a_flood() {
        let due = 1_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        // Asleep from before the time until 2 hours after.
        let now = due + 120 * MIN;
        let (t, ring) = due_ring(&it, &r, now).unwrap();
        assert_eq!(t, due + 25 * MIN);
        apply_ring(&mut it, ring, now);
        assert!(due_ring(&it, &r, now + 60 * MIN).is_none());
    }

    #[test]
    fn deadline_rings_early_then_closer_together() {
        let due = 1_000 * MIN;
        let r = Rhythm::preset("normal");
        let it = task(due, "deadline");
        let times: Vec<i64> = slots(&it, &r).iter().map(|(t, _)| (t - due) / MIN).collect();
        assert_eq!(times, vec![-60, -30, -15, -5, 0, 5, 10, 15, 20, 25]);
    }

    #[test]
    fn on_it_then_really_done() {
        let due = 1_000 * MIN;
        let r = Rhythm::preset("normal");
        let mut it = task(due, "deadline");
        apply_answer(&mut it, &r, "onit", due);
        assert!(due_ring(&it, &r, due + 29 * MIN).is_none());
        let (_, ring) = due_ring(&it, &r, due + 30 * MIN).unwrap();
        assert_eq!(ring, Ring::Check);
        apply_ring(&mut it, ring, due + 30 * MIN);
        apply_answer(&mut it, &r, "notyet", due + 31 * MIN);
        assert!(due_ring(&it, &r, due + 45 * MIN).is_none());
        assert_eq!(due_ring(&it, &r, due + 46 * MIN).unwrap().1, Ring::Check);
        apply_answer(&mut it, &r, "done", due + 47 * MIN);
        assert!(slots(&it, &r).is_empty());
    }

    #[test]
    fn snooze_moves_the_next_ring_and_restarts_the_count() {
        let due = 1_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        apply_ring(&mut it, Ring::Due, due);
        apply_answer(&mut it, &r, "snooze:10", due + MIN);
        assert!(due_ring(&it, &r, due + 10 * MIN).is_none());
        assert_eq!(due_ring(&it, &r, due + 11 * MIN).unwrap().0, due + 11 * MIN);
        assert_eq!(slots(&it, &r).len(), 6);
    }

    #[test]
    fn later_snoozes_until_a_chosen_time() {
        let due = 1_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        apply_ring(&mut it, Ring::Due, due);
        // "This evening": 4 hours later.
        let evening = due + 240 * MIN;
        apply_answer(&mut it, &r, &format!("until:{evening}"), due + MIN);
        assert_eq!(it.snooze_until, Some(evening));
        assert!(due_ring(&it, &r, evening - MIN).is_none());
        assert_eq!(due_ring(&it, &r, evening).unwrap().0, evening);
        // A time already past falls back to 10 minutes.
        apply_answer(&mut it, &r, "until:5", due + 2 * MIN);
        assert_eq!(it.snooze_until, Some(due + 12 * MIN));
    }

    #[test]
    fn answers_from_outside_are_checked_and_bounded() {
        assert!(valid_action("done") && valid_action("snooze:10") && valid_action("until:5"));
        assert!(!valid_action("rm -rf") && !valid_action("snooze:abc") && !valid_action(""));
        let due = 1_000 * MIN;
        let r = Rhythm::standard();
        let mut it = task(due, "reminder");
        // Made-up huge values never overflow (a panic would stop the clock) and stay sane.
        apply_answer(&mut it, &r, "snooze:99999999999999", due);
        assert_eq!(it.snooze_until, Some(due + SNOOZE_MAX_MIN * MIN));
        apply_answer(&mut it, &r, "snooze:-30", due);
        assert_eq!(it.snooze_until, Some(due + MIN));
        apply_answer(&mut it, &r, &format!("until:{}", i64::MAX), due);
        assert_eq!(it.snooze_until, Some(due + UNTIL_MAX_MS));
        it.snooze_until = Some(i64::MAX - 1);
        let _ = slots(&it, &r);
        let mut far = task(i64::MIN + 1, "deadline");
        let _ = slots(&far, &Rhythm::preset("insistent"));
        far.check_at = Some(i64::MAX - 1);
        let _ = slots(&far, &r);
        let custom = Rhythm { start: 10, pre: vec![i64::MAX / 2], every: 5, max: 3, checkin: 5, notyet: 5 }.sane();
        assert!(custom.pre.is_empty());
    }

    #[test]
    fn a_ring_for_another_day_says_which() {
        let at = |d: u32, h: u32| crate::repeat::to_ms(jiff::civil::date(2026, 11, d as i8).at(h as i8, 0, 0, 0)).unwrap();
        // Missed overnight (the PC off): it rings the next morning, saying it was yesterday's.
        let it = task(at(17, 22), "reminder");
        assert_eq!(notice_for(&it, Ring::Due, "Hub", at(18, 8)).body, "Was due yesterday at 22:00 · Hub");
        assert_eq!(notice_for(&it, Ring::Due, "Hub", at(17, 23)).body, "Was due 22:00 · Hub");
        // A deadline whose early ring comes the day before.
        let d = task(at(19, 9), "deadline");
        assert_eq!(notice_for(&d, Ring::Early, "Hub", at(18, 20)).body, "Due tomorrow at 09:00 · in 13 h · Hub");
        crate::i18n::set("fr");
        assert_eq!(notice_for(&it, Ring::Due, "Hub", at(18, 8)).body, "Prévu hier à 22:00 · Hub");
        assert_eq!(notice_for(&it, Ring::Due, "Hub", at(17, 23)).body, "Prévu à 22:00 · Hub");
        // Windows on a 12-hour clock: the times too, in either language.
        crate::clock::set_hour12(true);
        assert_eq!(notice_for(&it, Ring::Due, "Hub", at(18, 8)).body, "Prévu hier à 10:00\u{a0}PM · Hub");
        crate::i18n::set("en");
        assert_eq!(notice_for(&d, Ring::Early, "Hub", at(18, 20)).body, "Due tomorrow at 9:00\u{a0}AM · in 13 h · Hub");
        crate::clock::set_hour12(false);
    }

    #[test]
    fn snoozing_an_early_ring_keeps_the_time() {
        let due = 1_000 * MIN;
        let r = Rhythm::preset("normal");
        let mut it = task(due, "deadline");
        apply_ring(&mut it, Ring::Early, due - 60 * MIN);
        apply_answer(&mut it, &r, "snooze:10", due - 59 * MIN);
        let times: Vec<i64> = slots(&it, &r).iter().map(|(t, _)| (t - due) / MIN).collect();
        assert_eq!(times, vec![-49, -30, -15, -5, 0, 5, 10, 15, 20, 25]);
    }
}
