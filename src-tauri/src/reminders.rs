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
    if it.done_at.is_some() || it.alarm == "off" || it.kind != "task" {
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
    crate::clock::local_hhmm(ms)
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
        Ring::Early => {
            let left = (ahead as f64 / MIN as f64).round().max(1.0) as i64;
            (it.text.clone(), tr!("Due at {} · in {} · {}", "Pour {} · dans {} · {}", hhmm(due), duration(left), note_title))
        }
        Ring::Due => {
            let late = ahead < -MIN;
            let body = if !late {
                tr!("Due now · {}", "C’est l’heure · {}", note_title)
            } else if it.rings > 1 {
                tr!("Was due {} · {} reminder · {}", "Prévu à {} · {} rappel · {}", hhmm(due), ordinal(it.rings), note_title)
            } else {
                tr!("Was due {} · {}", "Prévu à {} · {}", hhmm(due), note_title)
            };
            (it.text.clone(), body)
        }
    };
    if ring != Ring::Check {
        if deadline {
            buttons.push((tr!("I'm on it", "Je m’en occupe"), "onit".into()));
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
    }
    for (notice, ev) in rang {
        notify::show(app, &notice);
        let _ = app.emit("ring", ev);
    }
    if let Some(n) = tidy {
        notify::show(app, &n);
        let _ = app.emit("tidy-ring", ());
    }
    if let Some(n) = look {
        notify::show(app, &n);
        let _ = app.emit("lookback-ready", ());
    }
    crate::look::tick(app, now);
    crate::backup::maybe_daily(app, now);
}

// ---- The evening look back ----

/// When the day's look back comes: the end of work hours on a work day, else the end of the
/// day ("My day ends at", 22:00 by default).
pub fn lookback_at(conn: &Connection, now: i64) -> Option<i64> {
    let wh = db::get_setting(conn, "work_hours").unwrap_or(serde_json::Value::Null);
    let on = wh.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
    let weekday = crate::clock::weekday(now)?;
    let work_day = wh.get("days").and_then(|v| v.as_array()).map(|a| a.iter().any(|d| d.as_u64() == Some(weekday as u64))).unwrap_or(false);
    if on && work_day {
        let until = wh.get("until").and_then(|v| v.as_str()).unwrap_or("19:00");
        return crate::clock::today_at(now, until);
    }
    let end = db::get_setting(conn, "day").and_then(|d| d.get("end").and_then(|v| v.as_str()).map(String::from)).unwrap_or_else(|| "22:00".into());
    crate::clock::today_at(now, &end)
}

fn lookback_tick(conn: &Connection, now: i64) -> Option<Notice> {
    let at = lookback_at(conn, now)?;
    let last = db::get_setting(conn, "lookback_last").and_then(|v| v.as_i64()).unwrap_or(0);
    if now < at || last >= at {
        return None;
    }
    let _ = db::set_setting(conn, "lookback_last", &serde_json::json!(now));
    // Nothing happened today: no need to look back.
    let midnight = crate::clock::today_at(now, "00:00")?;
    let busy: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM items WHERE done_at >= ?1 OR (due_at >= ?1 AND due_at < ?2) OR created_at >= ?1",
            rusqlite::params![midnight, midnight + 24 * 60 * MIN],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if busy == 0 {
        return None;
    }
    Some(Notice {
        tag: "lookback".into(),
        title: tr!("That was {}.", "C’était {}.", crate::clock::weekday_name(now)),
        body: tr!("Here is what happened, and what moves on · 2 minutes", "Voici ce qui s’est passé, et ce qui continue · 2 minutes"),
        buttons: vec![],
        open: "lookback".into(),
        persistent: false,
    })
}

/// An answer (notification button, Now card). Returns the updated item.
pub fn answer(app: &AppHandle, item_id: i64, action: &str) -> Result<Item, String> {
    if !valid_action(action) {
        return Err(tr!("Unknown answer: {}", "Réponse inconnue : {}", action));
    }
    let now = db::now_ms();
    let (it, next) = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let mut it = db::get_item(&conn, item_id)?;
        let was_open = it.done_at.is_none();
        let r = rhythm_of(&conn, &it);
        apply_answer(&mut it, &r, action, now);
        db::save_ring(&conn, &it)?;
        // A repeating task: its next occurrence.
        let next = if was_open && it.done_at.is_some() { db::spawn_next(&conn, &it)? } else { None };
        (db::get_item(&conn, item_id)?, next)
    };
    notify::clear(app, &format!("i{item_id}"));
    let _ = app.emit("item-updated", it.clone());
    if let Some(n) = next {
        let _ = app.emit("item-updated", n);
    }
    Ok(it)
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
    let last = db::get_setting(conn, "tidy_last").and_then(|v| v.as_i64()).unwrap_or(0);
    let snooze = db::get_setting(conn, "tidy_snooze").and_then(|v| v.as_i64()).unwrap_or(0);
    if snooze > now {
        return None;
    }
    let mut ring = snooze > 0 && snooze > last;
    if !ring && s.daily
        && let Some(today_at) = crate::clock::today_at(now, &s.time) {
            ring = now >= today_at && last < today_at;
        }
    if !ring && s.when_many && waiting >= s.many {
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
        title: tr!("Time to tidy your inbox", "Un moment pour faire le tri"),
        body: if crate::i18n::is_fr() {
            format!("{waiting} {} en attente · environ {minutes} {}", if waiting > 1 { "choses" } else { "chose" }, if minutes > 1 { "minutes" } else { "minute" })
        } else {
            format!("{waiting} {} waiting · about {minutes} {}", if waiting > 1 { "things" } else { "thing" }, if minutes > 1 { "minutes" } else { "minute" })
        },
        buttons: vec![
            (tr!("Tidy now", "Trier maintenant"), "tidy?a=now".into()),
            (tr!("In 1 hour", "Dans 1 h"), "tidy?a=snooze:60".into()),
            (tr!("Tomorrow", "Demain"), "tidy?a=tomorrow".into()),
        ],
        open: "tidy?a=now".into(),
        persistent: false,
    })
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
            crate::clock::today_at(now + 24 * 60 * MIN, &s.time).unwrap_or(now + 24 * 60 * MIN)
        } else {
            0
        };
        let _ = db::set_setting(&conn, "tidy_snooze", &serde_json::json!(until));
        let _ = db::set_setting(&conn, "tidy_last", &serde_json::json!(now));
    }
    notify::clear(app, "tidy");
    if action == "now" {
        crate::window::show(app);
        let _ = app.emit("open-tidy", ());
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
