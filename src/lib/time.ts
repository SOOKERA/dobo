// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { clockTime, dayMonth, dayMonthYear, t, weekdayLong, weekdayShort } from "./i18n.svelte";

/** A time as shown: "15:00", or "3:00 PM" when Windows uses a 12-hour clock. */
export const hhmm = (d: Date) => clockTime(d.getHours(), d.getMinutes());
/** A setting's "HH:MM" as shown ("19:00" or "7:00 PM"). */
export function shownHHMM(s: string): string {
  const [h, m] = s.split(":").map(Number);
  return Number.isFinite(h) && Number.isFinite(m) ? clockTime(h, m) : s;
}

function startOfDay(d: Date): number {
  const x = new Date(d);
  x.setHours(0, 0, 0, 0);
  return x.getTime();
}

/** Whole days from `now`'s day to `d`'s day (0 today, 1 tomorrow, -1 yesterday). */
export function daysFrom(d: Date, now = new Date()): number {
  return Math.round((startOfDay(d) - startOfDay(now)) / 86400000);
}

/** "15:00" today, "Thu 09:00" within the week, "24 Nov 09:00" after, "Yesterday 18:00" before
 * (French: "jeu. 09:00", "24 nov. 09:00", "Hier 18:00"); more than half a year away, with its
 * year ("24 Nov 2027 09:00": "24 Nov" alone would read as this year's). */
export function formatDue(ms: number, now = new Date()): string {
  const d = new Date(ms);
  const days = daysFrom(d, now);
  if (days === 0) return hhmm(d);
  if (days === -1) return t("Yesterday {time}", { time: hhmm(d) });
  if (days > 0 && days < 7) return `${weekdayShort(d)} ${hhmm(d)}`;
  return `${dayOf(d, days)} ${hhmm(d)}`;
}

/** "24 Nov", with the year when more than half a year away. */
function dayOf(d: Date, days: number): string {
  return Math.abs(days) > 183 ? dayMonthYear(d) : dayMonth(d);
}

/** The day of a time that isn't today, as it sits in a sentence ("Was due yesterday at 18:00",
 * "Due Thursday at 09:00", "Was due on 24 Nov at 09:00"; « Prévu lundi à 22:00 », « Prévu le
 * 24 nov. à 09:00 »): "yesterday", "tomorrow", the weekday in full within the week, else the
 * date. Null for today. */
export function dayWord(ms: number, now = new Date()): string | null {
  const d = new Date(ms);
  const days = daysFrom(d, now);
  if (days === 0) return null;
  if (days === -1) return t("yesterday");
  if (days === 1) return t("tomorrow");
  if (Math.abs(days) < 7) return weekdayLong(d);
  return t("on {date}", { date: dayOf(d, days) });
}

/** Shorter, for the capture preview chip: "Today 17:30", "Tomorrow 09:00". */
export function formatDueLong(ms: number, now = new Date()): string {
  const d = new Date(ms);
  const days = daysFrom(d, now);
  if (days === 0) return t("Today {time}", { time: hhmm(d) });
  if (days === 1) return t("Tomorrow {time}", { time: hhmm(d) });
  return formatDue(ms, now);
}

/** The clock under the entry line: "Thu 19 Nov · 19:02" (short: "Thu 19 · 19:02"); French
 * "jeu. 19 nov. · 19:02". */
export function clockDate(now = new Date(), short = false): string {
  return `${weekdayShort(now)} ${short ? now.getDate() : dayMonth(now)} · ${hhmm(now)}`;
}
