// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { dayMonth, t, weekdayShort } from "./i18n.svelte";

const pad = (n: number) => String(n).padStart(2, "0");
export const hhmm = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

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
 * (French: "jeu. 09:00", "24 nov. 09:00", "Hier 18:00"). */
export function formatDue(ms: number, now = new Date()): string {
  const d = new Date(ms);
  const days = daysFrom(d, now);
  if (days === 0) return hhmm(d);
  if (days === -1) return t("Yesterday {time}", { time: hhmm(d) });
  if (days > 0 && days < 7) return `${weekdayShort(d)} ${hhmm(d)}`;
  return `${dayMonth(d)} ${hhmm(d)}`;
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
