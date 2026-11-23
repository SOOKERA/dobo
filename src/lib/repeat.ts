// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Repeating tasks (the same rules as src-tauri/src/repeat.rs, which makes the next occurrence
// when one is ticked). Here: understanding typed repeats in French and English, the first
// occurrence, and the words shown on the board.
import type { Repeat } from "./types";
import { dayMonth, dayName, i18n, list, ordinal, t } from "./i18n.svelte";

// Day words → 0 (Sunday) … 6.
const DAY_WORDS: [RegExp, number][] = [
  [/^(sun(day)?s?|dimanches?)$/i, 0],
  [/^(mon(day)?s?|lundis?)$/i, 1],
  [/^(tue(s(day)?)?s?|mardis?)$/i, 2],
  [/^(wed(nesday)?s?|mercredis?)$/i, 3],
  [/^(thu(rs(day)?)?s?|jeudis?)$/i, 4],
  [/^(fri(day)?s?|vendredis?)$/i, 5],
  [/^(sat(urday)?s?|samedis?)$/i, 6],
];
// (Each day's longest form first, so "Thursday" is taken whole; "Thu" is Thursday too.)
const DAY_ALT = "sundays?|sun|mondays?|mon|tuesdays?|tues?|wednesdays?|wed|thursdays?|thurs?|thu|fridays?|fri|saturdays?|sat|dimanches?|lundis?|mardis?|mercredis?|jeudis?|vendredis?|samedis?";
// Between days: ",", "and", "&", "et", and ", and" / ", et".
const DAY_SEP = String.raw`\s*(?:,\s*(?:and|et|&)?|and|&|et)\s*`;

function dayOf(word: string): number | null {
  for (const [re, n] of DAY_WORDS) if (re.test(word)) return n;
  return null;
}

export interface FoundRepeat {
  rule: Repeat;
  /** Where the words were in the line, to take them out. */
  index: number;
  length: number;
}

const never = { kind: "never" } as const;

/** Finds "every Monday", "tous les lundis", "every weekday", "chaque 1er du mois", "every 2
 * weeks"… in a typed line. ("Weekly" alone stays a word: it is often a task's name.) */
export function findRepeat(text: string): FoundRepeat | null {
  const tries: [RegExp, (m: RegExpMatchArray) => Repeat | null][] = [
    // every 2 weeks · toutes les 3 semaines · every 10 days · tous les 2 mois · every other week
    [
      /\b(?:every|each)\s+(\d{1,2}|other)(?:st|nd|rd|th)?\s+(days?|weeks?|months?)\b|\btou(?:te)?s\s+les\s+(\d{1,2})\s+(jours|semaines|mois)\b/i,
      (m) => {
        // ("Every other week" and "every 2nd week" are every 2 weeks, not the 2nd of the month.)
        const n = m[1]?.toLowerCase() === "other" ? 2 : Number(m[1] ?? m[3]);
        const unit = (m[2] ?? m[4]).toLowerCase();
        const freq = unit.startsWith("d") || unit.startsWith("j") ? "day" : unit.startsWith("w") || unit.startsWith("s") ? "week" : "month";
        return { freq, interval: Math.max(1, n), days: [], monthday: null, ends: never };
      },
    ],
    // every weekday · tous les jours ouvrés · en semaine
    [
      /\b(?:every|each)\s+(?:weekday|week\s*day|working\s+day|work\s*day)s?\b|\bweekdays\b|\b(?:tous\s+les|chaque)\s+jours?\s+(?:ouvr[ée]s?|ouvrables?|de\s+la\s+semaine)\b|\ben\s+semaine\b/i,
      () => ({ freq: "weekday", interval: 1, days: [], monthday: null, ends: never }),
    ],
    // every 1st (of the month) · every 15th · chaque 1er du mois · le 15 de chaque mois
    [
      /\b(?:every|each|on\s+the)\s+(\d{1,2})(?:st|nd|rd|th)(?:\s+(?:of\s+)?(?:the|each|every)\s+month)?\b|\b(?:chaque|tous\s+les|le)\s+(1er|\d{1,2})\s+(?:du\s+mois|de\s+chaque\s+mois)\b/i,
      (m) => {
        const d = Number(String(m[1] ?? m[2]).replace(/er$/i, ""));
        if (d < 1 || d > 31) return null;
        return { freq: "month", interval: 1, days: [], monthday: d, ends: never };
      },
    ],
    // every Monday and Thursday · tous les lundis et jeudis · chaque lundi
    [
      new RegExp(`\\b(?:every|each|tous\\s+les|chaque)\\s+((?:${DAY_ALT})(?:${DAY_SEP}(?:${DAY_ALT}))*)\\b`, "i"),
      (m) => {
        const days = m[1]
          .split(new RegExp(DAY_SEP, "i"))
          .map((w) => dayOf(w.trim()))
          .filter((d): d is number => d != null);
        return days.length ? { freq: "week", interval: 1, days: [...new Set(days)].sort(), monthday: null, ends: never } : null;
      },
    ],
    // every day · tous les jours · chaque jour · daily
    [/\b(?:every|each)\s+day\b|\bdaily\b|\b(?:tous\s+les\s+jours|chaque\s+jour)\b|\bquotidiennement\b/i, () => ({ freq: "day", interval: 1, days: [], monthday: null, ends: never })],
    // every week · toutes les semaines · chaque semaine
    [/\b(?:every|each)\s+week\b|\b(?:toutes\s+les\s+semaines|chaque\s+semaine)\b/i, () => ({ freq: "week", interval: 1, days: [], monthday: null, ends: never })],
    // every month · tous les mois · chaque mois · monthly
    [/\b(?:every|each)\s+month\b|\bmonthly\b|\b(?:tous\s+les\s+mois|chaque\s+mois)\b|\bmensuellement\b/i, () => ({ freq: "month", interval: 1, days: [], monthday: null, ends: never })],
  ];
  for (const [re, make] of tries) {
    const m = text.match(re);
    if (m && m.index != null) {
      const rule = make(m);
      if (rule) return { rule, index: m.index, length: m[0].length };
    }
  }
  return null;
}

function daysInMonth(y: number, m: number): number {
  return new Date(y, m + 1, 0).getDate();
}

function weekStart(d: Date): number {
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7));
  return x.getTime();
}

function matches(rule: Repeat, d: Date, anchor: Date): boolean {
  switch (rule.freq) {
    case "day":
      return true;
    case "weekday":
      return d.getDay() >= 1 && d.getDay() <= 5;
    case "week": {
      const days = rule.days?.length ? rule.days : [anchor.getDay()];
      return days.includes(d.getDay());
    }
    case "month": {
      const want = rule.monthday ?? anchor.getDate();
      return d.getDate() === Math.min(want, daysInMonth(d.getFullYear(), d.getMonth()));
    }
  }
}

/** The first occurrence from now, at h:m (today when it still can, else the next match). */
export function firstOccurrence(rule: Repeat, h: number, m: number, now = new Date()): Date {
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m, 0, 0);
  for (let i = 0; i < 400; i++) {
    if (d.getTime() > now.getTime() && matches(rule, d, now)) return d;
    d.setDate(d.getDate() + 1);
  }
  return d;
}

/** A day's time as the clock shows it, in milliseconds. A time the clocks skip that night
 * (02:30 when they go forward) is the first minute after it that exists (03:00), as the Rust
 * side counts it; one that comes twice is the first. */
export function wallClock(y: number, mo: number, d: number, h: number, m: number): number {
  for (let k = 0; k <= 180; k++) {
    const x = new Date(y, mo, d, h, m + k);
    const want = new Date(Date.UTC(y, mo, d, h, m + k));
    if (x.getHours() === want.getUTCHours() && x.getMinutes() === want.getUTCMinutes()) return x.getTime();
  }
  return new Date(y, mo, d, h, m).getTime();
}

/** The occurrence after `prev`, at the repeat's own time of day (else the time of `prev`), as
 * the Rust side computes it. */
export function nextOccurrence(rule: Repeat, prev: number): number | null {
  const p = new Date(prev);
  const n = Math.max(1, rule.interval || 1);
  const own = /^(\d{1,2}):(\d{2})$/.exec(rule.at ?? "");
  const [h, m] = own ? [Number(own[1]), Number(own[2])] : [p.getHours(), p.getMinutes()];
  // (The day is counted at noon: no clock change moves it to another date.)
  const d = new Date(p.getFullYear(), p.getMonth(), p.getDate(), 12);
  const at = (x: Date) => wallClock(x.getFullYear(), x.getMonth(), x.getDate(), h, m);
  switch (rule.freq) {
    case "day":
      d.setDate(d.getDate() + n);
      return at(d);
    case "weekday":
      do d.setDate(d.getDate() + 1);
      while (d.getDay() === 0 || d.getDay() === 6);
      return at(d);
    case "week": {
      const days = rule.days?.length ? rule.days : [p.getDay()];
      const start = weekStart(p);
      for (let k = 1; k <= 7 * n + 7; k++) {
        const c = new Date(d);
        c.setDate(c.getDate() + k);
        const weeks = Math.round((weekStart(c) - start) / (7 * 86400000));
        if (weeks % n === 0 && days.includes(c.getDay())) return at(c);
      }
      return null;
    }
    case "month": {
      const want = rule.monthday ?? p.getDate();
      const c = new Date(p.getFullYear(), p.getMonth() + n, 1, 12);
      c.setDate(Math.min(want, daysInMonth(c.getFullYear(), c.getMonth())));
      return at(c);
    }
  }
}

/** Next occurrence once the one due at `due` is done, with the new "done" count; null when
 * the repeat is over. Ticked late, it jumps to the first occurrence still to come (as the
 * Rust side does), so nothing rings at once; the missed ones count as passed. */
export function nextAfterDone(rule: Repeat, due: number, now = Date.now()): { at: number; done: number } | null {
  let last: number | null = null;
  if (rule.ends.kind === "on") {
    const d = new Date(rule.ends.date);
    d.setHours(23, 59, 59, 999);
    last = d.getTime();
  }
  let done = (rule.done ?? 0) + 1;
  let prev = due;
  for (let k = 0; k < 100_000; k++) {
    if (rule.ends.kind === "after" && done >= rule.ends.count) return null;
    const next = nextOccurrence(rule, prev);
    if (next == null) return null;
    if (last != null && next > last) return null;
    if (next > now) return { at: next, done };
    done += 1;
    prev = next;
  }
  return null;
}

/** The day of a month: "1st", "15th" / "1er", "15". */
function monthDay(n: number): string {
  return i18n.lang === "fr" ? (n === 1 ? "1er" : String(n)) : ordinal(n);
}
/** A weekday as it sits in a sentence ("Monday" / "lundi"). */
const longDay = (d: number) => dayName(d, true);

/** "Every Monday", "Every weekday", "Every 2 weeks on Mon and Thu", "Every month on the 1st". */
export function repeatLabel(rule: Repeat): string {
  const n = Math.max(1, rule.interval || 1);
  let s: string;
  switch (rule.freq) {
    case "day":
      s = n === 1 ? t("Every day") : t("Every {n} days", { n });
      break;
    case "weekday":
      s = t("Every weekday");
      break;
    case "week": {
      const days = rule.days ?? [];
      // "Mon and Thu", "Mon, Wed and Fri" / "lun. et jeu.", "lun., mer. et ven."
      const short = list(days.map((d) => dayName(d)));
      if (n === 1) s = days.length === 1 ? t("Every {day}", { day: longDay(days[0]) }) : days.length ? t("Every {days}", { days: short }) : t("Every week");
      else s = days.length ? t("Every {n} weeks on {days}", { n, days: short }) : t("Every {n} weeks", { n });
      break;
    }
    case "month":
      s = n === 1 ? t("Every month") : t("Every {n} months", { n });
      if (rule.monthday) s = t("{s} on the {nth}", { s, nth: monthDay(rule.monthday) });
      break;
  }
  if (rule.ends.kind === "after") s = t("{s} · {n} times", { s, n: rule.ends.count });
  if (rule.ends.kind === "on") s = t("{s} · until {date}", { s, date: dayMonth(new Date(rule.ends.date)) });
  return s;
}
