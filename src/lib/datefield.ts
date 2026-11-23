// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The date field's keys (components/DateField.svelte): the day, the month and the year are spin
// buttons, as in the time field (./timefield.ts); this is what each key does to the date.

/** Day, month, year. */
export type DSeg = "d" | "mo" | "y";
/** A calendar date: [year, month 0–11, day 1–31]. */
export type YMD = [number, number, number];
/** The years the field goes between, both included. */
export type Years = [number, number];

export const daysIn = (y: number, mo: number) => new Date(y, mo + 1, 0).getDate();
/** The day kept inside its month (31 January, a month on: 28 or 29 February). */
export const fitDay = ([y, mo, d]: YMD): YMD => [y, mo, Math.max(1, Math.min(d, daysIn(y, mo)))];
const wrap = (n: number, len: number, from = 0) => ((((n - from) % len) + len) % len) + from;
const clamp = (n: number, [lo, hi]: Years) => Math.max(lo, Math.min(hi, n));

/** A key other than a digit, on one part: the new date, or null when the key isn't the field's.
 * Up/Down: one more or less (the day and the month go round, the year stops at its ends);
 * Page Up/Down: a week, a quarter or 5 years; Home/End: the part's first and last. */
export function keyDate(seg: DSeg, key: string, [y, mo, d]: YMD, years: Years): YMD | null {
  const n = daysIn(y, mo);
  const by = key === "ArrowUp" ? 1 : key === "ArrowDown" ? -1 : key === "PageUp" ? (seg === "d" ? 7 : seg === "mo" ? 3 : 5) : key === "PageDown" ? (seg === "d" ? -7 : seg === "mo" ? -3 : -5) : 0;
  if (by) {
    if (seg === "d") return [y, mo, wrap(Math.min(d, n) + by, n, 1)];
    if (seg === "mo") return fitDay([y, wrap(mo + by, 12), d]);
    return fitDay([clamp(y + by, years), mo, d]);
  }
  if (key === "Home") return seg === "d" ? [y, mo, 1] : seg === "mo" ? fitDay([y, 0, d]) : fitDay([years[0], mo, d]);
  if (key === "End") return seg === "d" ? [y, mo, n] : seg === "mo" ? fitDay([y, 11, d]) : fitDay([years[1], mo, d]);
  return null;
}

/** A digit typed into a part, after `buf` (the digits typed into it just before): the new date,
 * the digits so far, and whether the part is complete (the next part comes). The day: "4" is the
 * 4th at once, "1" waits for "12"; up to 31 whatever the month shown, since the month or the
 * year may be typed next (the date is fitted when they change, and when it is saved); the
 * month: "3" is March, "1" waits for "11"; the year takes its four digits ("2027"), and keeps the
 * date as it is meanwhile. */
export function typeDateDigit(seg: DSeg, buf: string, digit: number, date: YMD, years: Years): { date: YMD; buf: string; done: boolean } {
  const typed = buf + digit;
  const n = Number(typed);
  const [y, mo, d] = date;
  if (seg === "y") {
    if (typed.length < 4) return { date, buf: typed, done: false };
    return { date: fitDay([clamp(n, years), mo, d]), buf: typed, done: true };
  }
  if (seg === "d") {
    if (typed.length === 2) return { date: [y, mo, n >= 1 && n <= 31 ? n : digit >= 1 ? digit : d], buf: typed, done: true };
    return { date: digit >= 1 ? [y, mo, digit] : date, buf: typed, done: digit > 3 };
  }
  if (typed.length === 2) return { date: fitDay([y, n >= 1 && n <= 12 ? n - 1 : digit >= 1 ? digit - 1 : mo, d]), buf: typed, done: true };
  return { date: digit >= 1 ? fitDay([y, digit - 1, d]) : date, buf: typed, done: digit > 1 };
}

/** A year typed in part when the typing pauses: two digits are this century's ("27" is 2027),
 * anything shorter is dropped (null). */
export function pausedYear(buf: string, years: Years): number | null {
  return buf.length === 2 ? clamp(2000 + Number(buf), years) : null;
}
