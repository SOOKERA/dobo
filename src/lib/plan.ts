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

/** Work hours that cross midnight (22:00 to 06:00) end the next day. */
export function crossesMidnight(w: { from: string; until: string }): boolean {
  return w.until < w.from;
}

/** The work hours that start on the day of `d`: from its start to its end (the next morning's,
 * for hours that cross midnight). */
export function workSpan(w: WorkHoursLike, d: Date | number): { start: number; end: number } {
  return { start: at(d, w.from), end: at(crossesMidnight(w) ? addDays(d, 1) : d, w.until) };
}

/** The work hours `now` is in, or else today's (on a work day), or null. Hours that cross
 * midnight belong to the day they start: Saturday 02:00 is still Friday's night. */
export function currentShift(w: WorkHoursLike, now: Date | number): { start: number; end: number } | null {
  const t = new Date(now).getTime();
  const yesterday = addDays(now, -1);
  if (crossesMidnight(w) && isWorkDay(w, yesterday)) {
    const s = workSpan(w, yesterday);
    if (t < s.end) return s;
  }
  return isWorkDay(w, now) ? workSpan(w, now) : null;
}

/** Now within work hours (their days and hours, whether or not the switch is on). */
export function inWorkHours(w: WorkHoursLike, now: Date | number): boolean {
  if (w.from === w.until) return false;
  const s = currentShift({ ...w, on: true }, now);
  const t = new Date(now).getTime();
  return !!s && t >= s.start && t < s.end;
}

export interface Span {
  start: number;
  end: number;
  /** "Today", "Tomorrow", "Monday" (always as seen from now: the day after tomorrow by its name). */
  label: string;
}

export interface RulerRange {
  /** What the ruler shows first. */
  main: Span;
  /** Where its next-day spot leads (drop: its start; hold: the ruler shows it). */
  next: Span;
}

/** The hours of the drag ruler: from now to the end of the day (Work: a little after work hours
 * end; Personal: "My day ends at"). Late in the evening it shows tomorrow right away, and its
 * next-day spot is then the day after, by its name. */
export function rulerRange(now: number, space: "work" | "personal", day: DaySettings, work: WorkHoursLike): RulerRange {
  const useWork = space === "work" && work.on;
  // A day as seen from now: "Tomorrow", else its name. (Whole calendar days: a night the clocks
  // change is 23 or 25 hours.)
  const today = dayBounds(now).start;
  const nameOf = (d: Date): string => (Math.round((dayBounds(d).start - today) / 86_400_000) === 1 ? "Tomorrow" : DAY_NAMES[d.getDay()]);
  // The next span after a given day.
  const nextSpan = (from: number): Span => {
    if (useWork) {
      for (let k = 1; k <= 7; k++) {
        const d = addDays(from, k);
        if (isWorkDay(work, d)) return { ...workSpan(work, d), label: nameOf(d) };
      }
    }
    const d = addDays(from, 1);
    return { start: at(d, day.start), end: at(d, day.end), label: nameOf(d) };
  };
  const shift = useWork ? currentShift(work, now) : null;
  const todayEnd = shift ? shift.end + 60 * MIN : at(now, day.end);
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

export interface RulerHour {
  /** Where on the track (0…1). */
  x: number;
  at: number;
  /** On the hour (a taller tick). */
  major: boolean;
  /** Its name under the track, or null (a tick alone). */
  label: string | null;
}

/** The ruler's ticks (every half hour on a day of 9 hours or less, else every hour) and the names
 * under them: as many hours as fit without crowding (names `minGap` px apart, centre to centre),
 * and the half hours too when a short span leaves room for them. The two ends have names of their
 * own (`startRoom` and `endRoom` px wide): every name is clear of them, and of the one before
 * it, by 6 px (`nameW`: a name's width; about `minGap` less 6 without it). Whole hours on the
 * smallest of the clock's own steps (every 1, 2, 3… hours) that names all its hours between the
 * ends, so they stay evenly apart; else any hours that fit, so a narrow ruler still reads. */
export function rulerHours(start: number, end: number, width: number, o: { minGap: number; startRoom: number; endRoom: number; hourName: (h: number) => string; clock: (h: number, m: number) => string; nameW?: (name: string) => number }): RulerHour[] {
  const len = end - start;
  if (len <= 0) return [];
  const hours = len / 3_600_000;
  const w = width || 400;
  const step = hourLabelStep(hours, w, o.minGap);
  const halves = hours <= 9;
  // (The half hours named as well: room for two names an hour, each "20:30" wide.)
  const fine = halves && step === 1 && w / hours >= 2 * (o.minGap + 24);
  const first = new Date(start);
  first.setMinutes(first.getMinutes() > 0 ? 60 : 0, 0, 0);
  const out: RulerHour[] = [];
  for (let at = first.getTime() - (halves ? 1_800_000 : 0); at <= end; at += halves ? 1_800_000 : 3_600_000) {
    if (at < start) continue;
    const d = new Date(at);
    out.push({ x: (at - start) / len, at, major: d.getMinutes() === 0, label: null });
  }
  const nameOf = (h: RulerHour) => {
    const d = new Date(h.at);
    return fine ? o.clock(d.getHours(), d.getMinutes()) : o.hourName(d.getHours());
  };
  /** Half a name's width. */
  const halfOf = (name: string) => (o.nameW?.(name) ?? o.minGap - 6) / 2;
  /** A tick whose name is clear of the two ends' names. */
  const inside = (h: RulerHour) => {
    const px = h.x * w;
    const half = halfOf(nameOf(h));
    return px - half >= o.startRoom + 6 && px + half <= w - o.endRoom - 6;
  };
  /** Names the ticks `pick` allows, left to right, each clear of the ends and of the one before. */
  const place = (pick: (h: RulerHour) => boolean): Map<RulerHour, string> => {
    const named = new Map<RulerHour, string>();
    let lastX = -Infinity;
    let lastRight = -Infinity;
    for (const h of out) {
      if (!pick(h) || !inside(h)) continue;
      const name = nameOf(h);
      const px = h.x * w;
      const half = halfOf(name);
      if (px - lastX < o.minGap || px - half < lastRight + 6) continue;
      named.set(h, name);
      lastX = px;
      lastRight = px + half;
    }
    return named;
  };
  let named: Map<RulerHour, string> | null = null;
  if (fine) named = place(() => true);
  else {
    // (The smallest of the clock's own steps whose hours all fit between the ends, so the names
    // stay evenly apart; else any hours that fit.)
    for (const s of [1, 2, 3, 4, 6, 12]) {
      if (s < step) continue;
      const pick = (h: RulerHour) => h.major && new Date(h.at).getHours() % s === 0;
      const even = place(pick);
      if (even.size && even.size === out.filter((h) => pick(h) && inside(h)).length) {
        named = even;
        break;
      }
    }
    named ??= place((h) => h.major);
  }
  for (const [h, name] of named) h.label = name;
  return out;
}

/** An end of the ruler named as its hours are when it falls on the hour ("10 PM", « 22h »), else
 * with its minutes ("21:40"). */
export function endName(at: number, o: { hourName: (h: number) => string; clock: (h: number, m: number) => string }): string {
  const d = new Date(at);
  return d.getMinutes() === 0 ? o.hourName(d.getHours()) : o.clock(d.getHours(), d.getMinutes());
}

/** Daytime at this hour (the ruler's sun), else night (its moon). */
export function daylight(at: number): boolean {
  const h = new Date(at).getHours();
  return h >= 6 && h < 18;
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

/** "Later…": 1 hour, This evening, Tomorrow morning, Next Monday (Pick a time is the page's), in
 * the order they come. Still up in the night (after midnight, before 05:00), "Tomorrow morning"
 * is this morning's start, as the person means it, never a whole day later. */
export function laterChoices(now: number, day: DaySettings): LaterChoice[] {
  const out: LaterChoice[] = [{ label: "1 hour", at: now + 60 * MIN }];
  const evening = at(now, day.end) - 3 * 60 * MIN;
  if (evening - now >= 30 * MIN) out.push({ label: "This evening", at: evening });
  const startToday = at(now, day.start);
  const night = new Date(now).getHours() < 5 && now < startToday;
  out.push({ label: "Tomorrow morning", at: night ? startToday : at(addDays(now, 1), day.start) });
  let d = addDays(now, 1);
  while (d.getDay() !== 1) d = addDays(d, 1);
  out.push({ label: "Next Monday", at: at(d, day.start) });
  return out.sort((a, b) => a.at - b.at);
}

/** When the day's look back comes: the end of work hours on a work day (the next morning, for
 * hours that cross midnight), else the end of day. */
export function lookbackAt(d: Date | number, day: DaySettings, work: WorkHoursLike): number {
  return isWorkDay(work, d) ? workSpan(work, d).end : at(d, day.end);
}

/** The day whose look back shows now (from its time until the next morning; after work hours
 * that crossed midnight, until today's comes), or null. */
export function lookbackDay(now: number, day: DaySettings, work: WorkHoursLike): Date | null {
  if (now >= lookbackAt(now, day, work)) return new Date(now);
  const yesterday = addDays(now, -1);
  const y = lookbackAt(yesterday, day, work);
  if (now >= y && (now < at(now, day.start) || y >= at(now, "00:00"))) return yesterday;
  return null;
}

/** What a day's look back counts (done, due, added): its calendar day, but a work day whose
 * hours cross midnight runs on to their end the next morning, and the day after such a night
 * starts where that night ended (nothing counted twice, nothing from the night left out). Its
 * notification counts on the same span (reminders.rs `lookback_span`): keep them in step. */
export function lookbackSpan(d: Date | number, work: WorkHoursLike): { start: number; end: number } {
  const { start, end } = dayBounds(d);
  if (!crossesMidnight(work)) return { start, end };
  const before = addDays(start, -1);
  return {
    start: isWorkDay(work, before) ? Math.max(start, workSpan(work, before).end) : start,
    end: isWorkDay(work, d) ? Math.max(end, workSpan(work, d).end) : end,
  };
}

/** The day whose look back span holds `now`: today, but in the part of a night's work hours
 * after midnight it is still the day that night began (its span runs on to the night's end). */
export function spanDay(now: number, work: WorkHoursLike): Date {
  return now < lookbackSpan(now, work).start ? addDays(now, -1) : new Date(now);
}

/** A calendar day, from its midnight to the next one (23 or 25 hours on the nights the clocks
 * change, never a fixed 24). */
export function dayBounds(d: Date | number): { start: number; end: number } {
  const x = new Date(d);
  return { start: new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime(), end: new Date(x.getFullYear(), x.getMonth(), x.getDate() + 1).getTime() };
}

/** The look back's "Tomorrow" for a task (at its own time of day): the day after the one looked
 * back on, also when it's opened after midnight (then that's today), and never a time already
 * past. */
export function lookbackTomorrow(due: number, lookDay: Date | number, now: number): number {
  const time = new Date(due);
  const x = addDays(lookDay, 1);
  x.setHours(time.getHours(), time.getMinutes(), 0, 0);
  while (x.getTime() <= now) x.setDate(x.getDate() + 1);
  return x.getTime();
}

export function dayKey(d: Date | number): string {
  const x = new Date(d);
  return `${x.getFullYear()}-${String(x.getMonth() + 1).padStart(2, "0")}-${String(x.getDate()).padStart(2, "0")}`;
}
