// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Times the board offers: the hours of the drag ruler, the "Later…" choices, and when the
// evening look back comes. They follow "My day" (starts 8:00, ends 22:00) and the work hours.
// Labels stay English here (they are keys); spanName() and t() show them in the language.
import { cap, dayName, t, type Key } from "./i18n.svelte";

export interface DaySettings {
  start: string;
  end: string;
}
export const defaultDay: DaySettings = { start: "08:00", end: "22:00" };

export interface WorkHoursLike {
  on: boolean;
  /** 0 = Sunday … 6 = Saturday. */
  days: number[];
  from: string;
  until: string;
}

const MIN = 60_000;
const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/** That day's "HH:MM", in milliseconds. */
export function at(d: Date | number, hhmm: string): number {
  const x = new Date(d);
  const [h, m] = hhmm.split(":").map(Number);
  x.setHours(h || 0, m || 0, 0, 0);
  return x.getTime();
}

function addDays(d: Date | number, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

export function isWorkDay(w: WorkHoursLike, d: Date | number): boolean {
  return w.on && w.days.includes(new Date(d).getDay());
}

export interface Span {
  start: number;
  end: number;
  /** "Today", "Tomorrow", "Monday". */
  label: string;
}

export interface RulerRange {
  /** What the ruler shows first. */
  main: Span;
  /** Where "Tomorrow" leads (drop: its start; hold: the ruler shows it). */
  next: Span;
}

/** The hours of the drag ruler: from now to the end of the day (Work: a little after work hours
 * end; Personal: "My day ends at"). Late in the evening it shows tomorrow right away. */
export function rulerRange(now: number, space: "work" | "personal", day: DaySettings, work: WorkHoursLike): RulerRange {
  const useWork = space === "work" && work.on;
  // The next span after a given day.
  const nextSpan = (from: number): Span => {
    if (useWork) {
      for (let k = 1; k <= 7; k++) {
        const d = addDays(from, k);
        if (isWorkDay(work, d)) return { start: at(d, work.from), end: at(d, work.until), label: k === 1 ? "Tomorrow" : DAY_NAMES[d.getDay()] };
      }
    }
    const d = addDays(from, 1);
    return { start: at(d, day.start), end: at(d, day.end), label: "Tomorrow" };
  };
  const todayEnd = useWork && isWorkDay(work, now) ? at(now, work.until) + 60 * MIN : at(now, day.end);
  const start = Math.floor(now / (15 * MIN)) * 15 * MIN;
  if (todayEnd - now < 30 * MIN) {
    const main = nextSpan(now);
    return { main, next: nextSpan(main.start) };
  }
  return { main: { start, end: todayEnd, label: "Today" }, next: nextSpan(now) };
}

/** How many hours apart the ruler's hour labels are (1, 2, 3, 4, 6 or 12): the fewest that keep
 * them `minGap` px apart on a track `width` px wide spanning `hours` hours. */
export function hourLabelStep(hours: number, width: number, minGap = 32): number {
  const per = width / Math.max(hours, 1);
  for (const s of [1, 2, 3, 4, 6, 12]) if (per * s >= minGap) return s;
  return 24;
}

/** A pointer position on the ruler (0…1) as a time: snapped to the quarter hour unless exact. */
export function timeAt(span: Span, fraction: number, exact = false): number {
  const f = Math.min(1, Math.max(0, fraction));
  const t = span.start + (span.end - span.start) * f;
  const step = (exact ? 1 : 15) * MIN;
  const snapped = Math.round(t / step) * step;
  return Math.min(span.end, Math.max(span.start, snapped));
}

/** A span's label as shown: "Today" → "Aujourd’hui", "Monday" → "Lundi". */
export function spanName(label: string): string {
  const d = DAY_NAMES.indexOf(label);
  if (d >= 0) return cap(dayName(d, true));
  return label === "Today" || label === "Tomorrow" ? t(label) : label;
}

export interface LaterChoice {
  label: Key;
  at: number;
}

/** "Later…": 1 hour, This evening, Tomorrow morning, Next Monday (Pick a time is the page's). */
export function laterChoices(now: number, day: DaySettings): LaterChoice[] {
  const out: LaterChoice[] = [{ label: "1 hour", at: now + 60 * MIN }];
  const evening = at(now, day.end) - 3 * 60 * MIN;
  if (evening - now >= 30 * MIN) out.push({ label: "This evening", at: evening });
  out.push({ label: "Tomorrow morning", at: at(addDays(now, 1), day.start) });
  let d = addDays(now, 1);
  while (d.getDay() !== 1) d = addDays(d, 1);
  out.push({ label: "Next Monday", at: at(d, day.start) });
  return out;
}

/** When the day's look back comes: the end of work hours on a work day, else the end of day. */
export function lookbackAt(d: Date | number, day: DaySettings, work: WorkHoursLike): number {
  return isWorkDay(work, d) ? at(d, work.until) : at(d, day.end);
}

/** The day whose look back shows now (from its time until the next morning), or null. */
export function lookbackDay(now: number, day: DaySettings, work: WorkHoursLike): Date | null {
  if (now >= lookbackAt(now, day, work)) return new Date(now);
  const yesterday = addDays(now, -1);
  if (now < at(now, day.start) && now >= lookbackAt(yesterday, day, work)) return yesterday;
  return null;
}

export function dayKey(d: Date | number): string {
  const x = new Date(d);
  return `${x.getFullYear()}-${String(x.getMonth() + 1).padStart(2, "0")}-${String(x.getDate()).padStart(2, "0")}`;
}
