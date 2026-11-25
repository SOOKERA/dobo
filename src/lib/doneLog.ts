// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A repeating task's history: one line that stays ticked until its next occurrence's day (the
// backend opens it again then: src-tauri/src/db.rs `reopen_due`), the times it was done (`Done`
// rows), and what the History card and the look back make of them: a month of days for a daily
// task, a year of weeks for a weekly one, a year of months for a monthly one.
import { nextAfterDone, nextOccurrence, wallClock } from "./repeat";
import { dayKey } from "./plan";
import { dayWord, hhmm } from "./time";
import { cap, list, t, type Key } from "./i18n.svelte";
import type { Done, Item, Repeat } from "./types";

const DAY = 86_400_000;

/** "My day" (Settings › My day): when it starts and ends, "08:00" and "22:00". */
export interface MyDay {
  start: string;
  end: string;
}

const minutesOf = (s: string) => {
  const m = /^(\d{1,2}):(\d{2})$/.exec(s);
  return m && Number(m[1]) < 24 && Number(m[2]) < 60 ? Number(m[1]) * 60 + Number(m[2]) : null;
};

/** When a ticked repeating task, already at its next occurrence `due`, is a to-do again: at the
 * start of the person's day that occurrence belongs to, never later than the occurrence's own
 * time (a 07:00 task opens by 07:00). A day that crosses midnight (22:00–06:00) started the
 * evening before for a time before its end (a 02:00 task opens at 22:00 the evening before). As
 * `reopens_at_in` in repeat.rs. */
export function reopensAt(due: number, day: MyDay): number {
  const d = new Date(due);
  const start = minutesOf(day.start);
  const end = minutesOf(day.end);
  if (start == null) return due;
  const night = end != null && start > end && d.getHours() * 60 + d.getMinutes() < end;
  const at = wallClock(d.getFullYear(), d.getMonth(), d.getDate() - (night ? 1 : 0), Math.floor(start / 60), start % 60);
  return Math.min(at, due);
}

/** A ticked repeating task waiting for its next day: when it opens again (null: not one). */
export function waitsUntil(it: Item, day: MyDay): number | null {
  if (it.kind !== "task" || it.done_at == null || it.due_at == null || !it.repeat?.ticked) return null;
  return reopensAt(it.due_at, day);
}

/** Ticking the occurrence due at `due`, at `now`, as the backend does it: the line's next time
 * and its rule then; null when the repeat is over (the line stays done). */
export function tickRepeat(rule: Repeat, due: number, now: number): { at: number; rule: Repeat } | null {
  const plain: Repeat = { ...rule };
  delete plain.ticked;
  const found = nextAfterDone(plain, due, now);
  if (!found) return null;
  return { at: found.at, rule: { ...plain, done: found.done, ticked: { due, done: plain.done ?? 0 } } };
}

/** The rule without what a tick keeps (the rule as unticking gives it back). */
export function untickedRule(rule: Repeat): Repeat {
  const r: Repeat = { ...rule, done: rule.ticked?.done ?? rule.done };
  delete r.ticked;
  return r;
}

// ---- The days of a rule ----

const noon = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate(), 12);
/** Calendar days from a to b (both at noon: no clock change moves them). */
const daysBetween = (a: Date, b: Date) => Math.round((noon(b).getTime() - noon(a).getTime()) / DAY);

/** Whole calendar days from the day of `from` to the day of `to` (repeat.rs `days_apart`). */
export function daysApart(from: number, to: number): number {
  return daysBetween(new Date(from), new Date(to));
}

/** `ms` moved on by `days` calendar days, at the same time of day on the clock: never a fixed
 * number of hours (the days the clocks change are 23 or 25 hours long). A time skipped that night
 * is the first minute after it that exists. As repeat.rs `shift_days`: how a repeating task's
 * steps move on when it opens again. */
export function shiftDays(ms: number, days: number): number {
  const d = new Date(ms);
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate() + days, d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds());
  if (x.getHours() === d.getHours() && x.getMinutes() === d.getMinutes()) return x.getTime();
  return wallClock(d.getFullYear(), d.getMonth(), d.getDate() + days, d.getHours(), d.getMinutes());
}
/** Monday of the date's week (the app's weeks start on Monday, in both languages). */
const monday = (d: Date) => {
  const x = noon(d);
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7));
  return x;
};
const daysIn = (y: number, m: number) => new Date(y, m + 1, 0).getDate();

/** Is `d` one of the rule's days, given one occurrence it had (`anchor`: its every-N steps
 * count from there)? */
export function isOccurrence(rule: Repeat, d: Date, anchor: Date): boolean {
  const n = Math.max(1, rule.interval || 1);
  switch (rule.freq) {
    case "day":
      return ((daysBetween(anchor, d) % n) + n) % n === 0;
    case "weekday":
      return d.getDay() >= 1 && d.getDay() <= 5;
    case "week": {
      const days = rule.days?.length ? rule.days : [anchor.getDay()];
      const weeks = Math.round(daysBetween(monday(anchor), monday(d)) / 7);
      return days.includes(d.getDay()) && ((weeks % n) + n) % n === 0;
    }
    case "month": {
      const months = (d.getFullYear() - anchor.getFullYear()) * 12 + d.getMonth() - anchor.getMonth();
      const want = rule.monthday ?? anchor.getDate();
      return ((months % n) + n) % n === 0 && d.getDate() === Math.min(want, daysIn(d.getFullYear(), d.getMonth()));
    }
  }
}

/** One occurrence the line surely had (its time now, or the one ticked): the every-N steps count
 * from it. */
function anchorOf(it: Item): Date {
  return new Date(it.repeat?.ticked?.due ?? it.due_at ?? it.created_at);
}

/** Where each day's every-N steps count from (`anchorFor(it, log, now)(day)`). From today on: its
 * time now (or the one ticked). Before: the latest time done on or before that day (else the
 * first one done), so a time moved by hand keeps the past's rhythm as it went: an every-2-weeks
 * task put off a week never turns its weeks done, or off between two, into missed ones. A rule
 * without steps (every day, every week…) counts from its time now. */
export function anchorFor(it: Item, log: readonly Done[], now?: Date): (d: Date) => Date {
  const here = anchorOf(it);
  const rule = it.repeat;
  if (!rule || !now || rule.freq === "weekday" || Math.max(1, rule.interval || 1) === 1) return () => here;
  const rows = log
    .filter((r) => r.item_id === it.id)
    .map((r) => noon(new Date(r.due_at ?? r.done_at)).getTime())
    .sort((a, b) => a - b);
  if (!rows.length) return () => here;
  const today = noon(now).getTime();
  return (d) => {
    const at = noon(d).getTime();
    if (at >= today) return here;
    // (The last one on or before that day: found by halves, a long history stays quick.)
    let lo = 0;
    for (let hi = rows.length - 1; lo < hi; ) {
      const mid = (lo + hi + 1) >> 1;
      if (rows[mid] <= at) lo = mid;
      else hi = mid - 1;
    }
    return new Date(rows[lo]);
  };
}

/** The first day its history knows: the first occurrence it has been at (the first time done,
 * or the one it waits at, never done yet). Earlier days aren't counted against it. */
export function seriesStart(it: Item, log: readonly Done[]): Date {
  let first = noon(anchorOf(it)).getTime();
  for (const r of log) if (r.item_id === it.id) first = Math.min(first, noon(new Date(r.due_at ?? r.done_at)).getTime());
  return new Date(first);
}

/** The days a repeating task is due in [from, to] (midnights; `to` included), from the first
 * day its history knows (`seriesStart`), and not after its last day; an every-N rule's past days
 * counted from its history (`anchorFor`, given its history and `now`). */
export function dueDays(it: Item, from: Date, to: Date, start: Date, log: readonly Done[] = [], now?: Date): Set<string> {
  const out = new Set<string>();
  const rule = it.repeat;
  if (!rule) return out;
  const anchor = anchorFor(it, log, now);
  const first = noon(start);
  const last = lastDay(it, rule, to);
  for (const d = noon(from); d <= last; d.setDate(d.getDate() + 1)) if (d >= first && isOccurrence(rule, d, anchor(d))) out.add(dayKey(d));
  return out;
}

/** Its last day in a span ending on `to` (noon): `to`, or the day the rule ends before it. */
function lastDay(it: Item, rule: Repeat, to: Date): Date {
  let last = noon(to);
  if (rule.ends.kind === "on" && noon(new Date(rule.ends.date)) < last) last = noon(new Date(rule.ends.date));
  // (Over: a ticked line whose rule has ended shows nothing after it.)
  if (it.done_at != null && !rule.ticked && it.due_at != null && noon(new Date(it.due_at)) < last) last = noon(new Date(it.due_at));
  // ("After N times": its time now is the first of those still to do, even moved by hand to a
  // day of its own, and the last one is as many of the series' occurrences on, each one after
  // the one before as ticking moves it; nothing after it.)
  else if (rule.ends.kind === "after" && it.due_at != null) {
    let at: number | null = it.due_at;
    for (let left = rule.ends.count - (rule.done ?? 0); left > 1 && at != null && noon(new Date(at)) <= last; left--) at = nextOccurrence(rule, at);
    if (at != null && noon(new Date(at)) < last) last = noon(new Date(at));
  }
  return last;
}

/** The days a line was done for: each time's occurrence (a Friday task ticked early on
 * Wednesday is Friday's), or the day it was done when it had no time. */
export function doneDays(log: readonly Done[], itemId: number): Set<string> {
  return new Set(log.filter((r) => r.item_id === itemId).map((r) => dayKey(r.due_at ?? r.done_at)));
}

/** A month as the calendar shows it: its weeks, Monday first, each day null outside it. */
export function monthGrid(year: number, month: number): (Date | null)[][] {
  const first = new Date(year, month, 1, 12);
  const lead = (first.getDay() + 6) % 7;
  const cells: (Date | null)[] = Array.from({ length: lead }, () => null);
  for (let d = 1; d <= daysIn(year, month); d++) cells.push(new Date(year, month, d, 12));
  while (cells.length % 7) cells.push(null);
  const weeks: (Date | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}

// ---- A year of weeks or months (weekly and monthly tasks) ----

/** What the History card shows: a month of days (daily, weekdays), a year of weeks (weekly, any
 * interval or days), a year of months (monthly). */
export type HistoryView = "days" | "weeks" | "months";
export function viewOf(rule: Repeat | null | undefined): HistoryView {
  return rule?.freq === "week" ? "weeks" : rule?.freq === "month" ? "months" : "days";
}

/** The year a week belongs to: its Thursday's (ISO weeks, from Monday). */
export function weekYear(d: Date): number {
  const th = monday(d);
  th.setDate(th.getDate() + 3);
  return th.getFullYear();
}

/** A year's weeks by their Mondays (noon): week 1 holds its first Thursday, so 52 of them, or 53
 * when the year starts on a Thursday (or a Wednesday, in a leap year). */
export function yearWeeks(year: number): Date[] {
  const out: Date[] = [];
  for (const d = monday(new Date(year, 0, 4, 12)); weekYear(d) === year; d.setDate(d.getDate() + 7)) out.push(new Date(d));
  return out;
}

/** A week's month: its Thursday's (by its Monday). */
export function monthOfWeek(mon: Date): Date {
  return new Date(mon.getFullYear(), mon.getMonth(), mon.getDate() + 3, 12);
}

/** A row of weeks' month names (by their Mondays): the column each goes in and its month. The
 * row's first square always says its month, unless the next one starts a new month (that one
 * says it: two names never overlap); each month after says it over its first week, but never
 * from the row's last square (the next row's first says it, with room to). */
export function weekNames(mondays: readonly Date[]): { col: number; month: Date }[] {
  const out: { col: number; month: Date }[] = [];
  const m = mondays.map((d) => monthOfWeek(d).getMonth());
  for (let j = 0; j < mondays.length; j++) {
    const opens = j === 0 ? m[1] === undefined || m[1] === m[0] : m[j] !== m[j - 1] && j < mondays.length - 1;
    if (opens) out.push({ col: j, month: monthOfWeek(mondays[j]) });
  }
  return out;
}

/** One occurrence: done, missed (its day is past), or still to come (today or later). */
export type Mark = "done" | "missed" | "later";
/** A week or a month: nothing (before the series' first day, after its last), off (an every-2
 * rule's week or month between two), all done, some done, missed, or still to come. */
export type CellState = "none" | "off" | "done" | "partial" | "missed" | "later";
export interface Cell {
  /** Its first and last days (noon). */
  from: Date;
  to: Date;
  /** Its occurrences, in order (a week on Monday and Thursday: two). */
  days: { date: Date; mark: Mark }[];
  state: CellState;
  /** The part of them done (0 to 1). */
  part: number;
  /** Today is in it. */
  current: boolean;
}

/** How far from its own day, in days, a time done on a day of its own (moved there by hand) may
 * stand for one of the rule's days: half its period (half a week for a weekly task, half a month
 * for a monthly one, a day for every 2 days; none for one every day or every weekday). */
function reach(rule: Repeat): number {
  const n = Math.max(1, rule.interval || 1);
  return (rule.freq === "day" ? n : rule.freq === "weekday" ? 1 : rule.freq === "week" ? 7 * n : 30 * n) / 2;
}

const fromKey = (k: string) => new Date(Number(k.slice(0, 4)), Number(k.slice(5, 7)) - 1, Number(k.slice(8, 10)), 12);
const plusDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, 12);

interface Occurrences {
  /** Its due days and the days it was done for (`dueDays`, `doneDays`). */
  due: Set<string>;
  done: Set<string>;
  /** Each time done on a day of its own that stands for one of its due days: that day, by its
   * own day. */
  home: Map<string, string>;
  /** The due days some time done stands for. */
  covered: Set<string>;
}

/** Its occurrences around [from, to] (and as far again on each side, so a time done just
 * outside stands for the day it is closest to): each time done on a day of its own stands for
 * the closest due day not done, within half the rule's period (`reach`), the next one first when
 * two are as close, also across a week's or a month's edge. One rule for the squares, their words
 * and the summary: a monthly task done on the 16th for the 15th is that month's, done; a Monday
 * done the Sunday before is that Monday's week. */
function occurrences(it: Item, rule: Repeat, log: readonly Done[], from: Date, to: Date, start: Date, now: Date): Occurrences {
  const r = reach(rule);
  const m = 2 * Math.ceil(r);
  const lo = dayKey(plusDays(from, -m));
  const hi = dayKey(plusDays(to, m));
  const due = dueDays(it, plusDays(from, -m), plusDays(to, m), start, log, now);
  const done = doneDays(log, it.id);
  const open = [...due].filter((k) => !done.has(k)).sort().map(fromKey);
  const home = new Map<string, string>();
  const covered = new Set<string>();
  for (const k of [...done].filter((k) => !due.has(k) && k >= lo && k <= hi).sort()) {
    const d = fromKey(k);
    let best = -1;
    let gap = Infinity;
    for (let j = 0; j < open.length; j++) {
      const g = Math.abs(daysBetween(d, open[j]));
      if (g <= r && (g < gap || (g === gap && open[j] > d))) {
        best = j;
        gap = g;
      }
    }
    if (best < 0) continue;
    const at = dayKey(open[best]);
    home.set(k, at);
    covered.add(at);
    open.splice(best, 1);
  }
  return { due, done, home, covered };
}

/** Spans of days as cells: each one's occurrences (its due days and the days it was done for)
 * and how they went, at `now`. */
function cellsOf(it: Item, log: readonly Done[], spans: [Date, Date][], start: Date, now: Date): Cell[] {
  const rule = it.repeat;
  const all = spans.length ? { from: spans[0][0], to: spans[spans.length - 1][1] } : null;
  const occ = rule && all ? occurrences(it, rule, log, all.from, all.to, start, now) : null;
  const end = rule && all ? lastDay(it, rule, all.to) : null;
  const first = noon(start);
  const every = Math.max(1, rule?.interval || 1) > 1;
  const today = dayKey(now);
  // (Each time done, by the occurrence it is for: its own day, or the one it stands for.)
  const dones = occ ? [...occ.done].map((k) => ({ date: fromKey(k), home: occ.home.get(k) ?? k })) : [];
  return spans.map(([a, b]) => {
    const ka = dayKey(a);
    const kb = dayKey(b);
    const days: Cell["days"] = dones.filter((x) => x.home >= ka && x.home <= kb).map((x) => ({ date: x.date, mark: "done" as Mark }));
    if (occ)
      for (const d = noon(a); d <= b; d.setDate(d.getDate() + 1)) {
        const k = dayKey(d);
        if (occ.due.has(k) && !occ.done.has(k) && !occ.covered.has(k)) days.push({ date: new Date(d), mark: k < today ? "missed" : "later" });
      }
    days.sort((x, y) => x.date.getTime() - y.date.getTime());
    const n = days.filter((x) => x.mark === "done").length;
    // (No occurrence: between two of an every-2 rule within the series, else nothing.)
    const state: CellState = !days.length
      ? every && end && noon(b) >= first && noon(a) <= end
        ? "off"
        : "none"
      : n === days.length
        ? "done"
        : n
          ? "partial"
          : days.some((x) => x.mark === "missed")
            ? "missed"
            : "later";
    return { from: noon(a), to: noon(b), days, state, part: days.length ? n / days.length : 0, current: today >= ka && today <= kb };
  });
}

// What a week or a month says of its days (its tooltip, and to a screen reader), done first,
// then missed, then still to come: "Done Monday, missed Thursday", "Done on the 15th".
const MARKS: Record<"week" | "month", Record<Mark, Key>> = {
  week: { done: "done {day} (week)", missed: "missed {day} (week)", later: "due {day} (week)" },
  month: { done: "done on the {day} (month)", missed: "missed the {day} (month)", later: "due on the {day} (month)" },
};
export function markWords(c: Cell, of: "week" | "month", day: (d: Date) => string): string {
  const parts: string[] = [];
  for (const mark of ["done", "missed", "later"] as const) {
    const days = c.days.filter((d) => d.mark === mark).map((d) => day(d.date));
    if (days.length) parts.push(t(MARKS[of][mark], { day: list(days), n: days.length }));
  }
  return cap(parts.join(", "));
}

/** A weekly task's year: one cell per week, Monday to Sunday. */
export function weekCells(it: Item, log: readonly Done[], year: number, start: Date, now: Date): Cell[] {
  return cellsOf(
    it,
    log,
    yearWeeks(year).map((m): [Date, Date] => [m, new Date(m.getFullYear(), m.getMonth(), m.getDate() + 6, 12)]),
    start,
    now,
  );
}

/** A monthly task's year: one cell per month. */
export function monthCells(it: Item, log: readonly Done[], year: number, start: Date, now: Date): Cell[] {
  return cellsOf(
    it,
    log,
    Array.from({ length: 12 }, (_, m): [Date, Date] => [new Date(year, m, 1, 12), new Date(year, m, daysIn(year, m), 12)]),
    start,
    now,
  );
}

// ---- The summary, in the rule's own units ----

/** "days", "weekdays", "weeks", "months", or "times" (every 3 days, a week on several days…). */
export type Unit = "days" | "weekdays" | "weeks" | "months" | "times";

/** The unit a rule is counted in, and how many of the last ones the summary looks at. */
export function unitOf(rule: Repeat): { unit: Unit; window: number } {
  const n = Math.max(1, rule.interval || 1);
  if (rule.freq === "weekday") return { unit: "weekdays", window: 20 };
  if (n > 1) return { unit: "times", window: 10 };
  if (rule.freq === "day") return { unit: "days", window: 30 };
  if (rule.freq === "month") return { unit: "months", window: 6 };
  return (rule.days?.length ?? 0) > 1 ? { unit: "times", window: 12 } : { unit: "weeks", window: 8 };
}

export interface Summary {
  unit: Unit;
  /** Occurrences done among the last `of`. */
  done: number;
  of: number;
  /** The latest ones done one after another (today's, still to come, doesn't break it). */
  streak: number;
  /** Ever done (the gentle line when 0). */
  ever: number;
}

/** How a repeating task went, at `now`: of its last occurrences (up to the unit's window, from
 * the first day its history knows), how many were done, and its streak. Today's occurrence counts once done;
 * not yet, it doesn't count against it. */
export function summary(it: Item, log: readonly Done[], now: number): Summary | null {
  const rule = it.repeat;
  if (!rule) return null;
  const { unit, window } = unitOf(rule);
  const today = noon(new Date(now));
  const anchor = anchorFor(it, log, new Date(now));
  const first = seriesStart(it, log);
  // The occurrences up to today, newest first (today's only once done; enough for the window and
  // a long streak).
  // (Ticked early, on Wednesday for Friday's: from Friday.)
  const ticked = rule.ticked ? noon(new Date(rule.ticked.due)) : null;
  const from = ticked && ticked > today ? ticked : today;
  // (Done: on its day, or by a time done on a day of its own standing for it, as the squares
  // count it.)
  const occ = occurrences(it, rule, log, first, from, first, new Date(now));
  const did = (k: string) => occ.done.has(k) || occ.covered.has(k);
  const past: string[] = [];
  for (const d = new Date(from); d >= first && past.length < 400; d.setDate(d.getDate() - 1)) {
    if (!isOccurrence(rule, d, anchor(d))) continue;
    const k = dayKey(d);
    if (d >= today && !did(k)) continue;
    past.push(k);
  }
  const last = past.slice(0, window);
  let streak = 0;
  while (streak < past.length && did(past[streak])) streak++;
  return { unit, done: last.filter(did).length, of: last.length, streak, ever: log.filter((r) => r.item_id === it.id).length };
}

// ---- The look back ----

export interface DoneEntry {
  key: string;
  text: string;
  at: number;
}

/** The tasks done in [start, end): lines ticked then, and a repeating task's times from its
 * history once it is open again (the time the line still carries is shown once, as the line; made
 * an idea since, from its history). */
export function doneBetween(items: readonly Item[], log: readonly Done[], start: number, end: number): DoneEntry[] {
  const byId = new Map(items.map((i) => [i.id, i]));
  const out: DoneEntry[] = [];
  for (const i of items) if (i.kind === "task" && i.done_at != null && i.done_at >= start && i.done_at < end) out.push({ key: `i${i.id}`, text: i.text, at: i.done_at });
  for (const r of log) {
    if (r.done_at < start || r.done_at >= end) continue;
    const it = byId.get(r.item_id);
    // (Made an idea since, its time from its history stays: reminders.rs `lookback_count`.)
    if (!it || (it.kind === "task" && it.done_at === r.done_at)) continue;
    out.push({ key: `d${r.id}`, text: it.text, at: r.done_at });
  }
  return out.sort((a, b) => a.at - b.at);
}

/** When a ticked repeating task's next one is, as it sits in a sentence: "next one tomorrow at
 * 07:30", « la suivante demain à 07:30 ». */
export function nextOne(due: number, now: Date): string {
  const day = dayWord(due, now);
  const time = hhmm(new Date(due));
  return day ? t("next one {day} at {time}", { day, time }) : t("next one today at {time}", { time });
}
