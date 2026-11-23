// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The time field's keys (components/TimeField.svelte): hours, minutes and AM/PM are spin
// buttons; this is what each key does to the time.

/** Hours, minutes, AM/PM. */
export type Seg = "h" | "m" | "p";
/** A time of day, 24-hour: [hours 0–23, minutes 0–59]. */
export type HM = [number, number];

export const wrapTime = (h: number, m: number): HM => [((h % 24) + 24) % 24, ((m % 60) + 60) % 60];

/** A key other than a digit, on one part: the new time, or null when the key isn't the field's.
 * Up/Down: one more or less (hours run on through AM and PM); Page Up/Down: 3 hours or 10
 * minutes; Home/End: the part's first and last; A/P (or Space, Up, Down) on AM/PM; Backspace
 * and Delete: the part back to zero. */
export function keyTime(seg: Seg, key: string, [h, m]: HM, twelve: boolean): HM | null {
  const pm = h >= 12;
  const step = (by: number): HM => (seg === "h" ? wrapTime(h + by, m) : seg === "m" ? wrapTime(h, m + by) : wrapTime(h + 12, m));
  switch (key) {
    case "ArrowUp":
      return step(1);
    case "ArrowDown":
      return step(-1);
    case "PageUp":
      return step(seg === "m" ? 10 : seg === "h" ? 3 : 1);
    case "PageDown":
      return step(seg === "m" ? -10 : seg === "h" ? -3 : 1);
    case "Home":
      return seg === "h" ? [twelve && pm ? 12 : 0, m] : seg === "m" ? [h, 0] : [h % 12, m];
    case "End":
      return seg === "h" ? [twelve && !pm ? 11 : 23, m] : seg === "m" ? [h, 59] : [(h % 12) + 12, m];
    case "Backspace":
    case "Delete":
      return seg === "h" ? [twelve && pm ? 12 : 0, m] : seg === "m" ? [h, 0] : null;
  }
  if (seg === "p" && /^[ap]$/i.test(key)) return [(h % 12) + (key.toLowerCase() === "p" ? 12 : 0), m];
  if (seg === "p" && key === " ") return step(1);
  return null;
}

/** A digit typed into the hours or the minutes, after `buf` (the digits typed into it just
 * before): the new time, the digits so far, and whether the part is complete (the next part
 * comes). "9" is 09 at once; "1" waits for "12" or "15"; "15" in 12-hour is 5. */
export function typeDigit(seg: "h" | "m", buf: string, d: number, [h, m]: HM, twelve: boolean): { time: HM; buf: string; done: boolean } {
  const typed = buf + d;
  const n = Number(typed);
  if (seg === "m") {
    const time: HM = [h, typed.length === 2 ? n : d];
    return { time, buf: typed, done: typed.length === 2 || d > 5 };
  }
  if (twelve) {
    const h12 = typed.length === 2 ? (n >= 1 && n <= 12 ? n : d) : d;
    const time: HM = h12 >= 1 ? [(h12 % 12) + (h >= 12 ? 12 : 0), m] : [h, m];
    return { time, buf: typed, done: typed.length === 2 || d > 1 };
  }
  return { time: [typed.length === 2 && n <= 23 ? n : d, m], buf: typed, done: typed.length === 2 || d > 2 };
}
