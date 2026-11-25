// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A repeating task's one line and its history: when it opens again, the calendar's marks, the
// summary in the rule's own units, the look back counting each time once, and the store's tick,
// untick, Undo and Redo over the preview's backend.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { doneBetween, doneDays, dueDays, isOccurrence, markWords, monthCells, monthGrid, nextOne, reopensAt, seriesStart, summary, tickRepeat, untickedRule, unitOf, viewOf, waitsUntil, weekCells, weekNames, weekYear, yearWeeks, type Cell } from "./doneLog";
import { dayKey } from "./plan";
import { dayName, i18n, t } from "./i18n.svelte";
import type { Done, Item, Repeat } from "./types";

const never = { kind: "never" } as const;
const at = (y: number, m: number, d: number, h = 12, mi = 0) => new Date(y, m - 1, d, h, mi).getTime();
const item = (over: Partial<Item>): Item => ({
  id: 1, note_id: 1, kind: "task", text: "Stretch", due_at: null, done_at: null, sort: 0, created_at: at(2026, 9, 1), parent_id: null, alarm: "reminder", rhythm: null, link: null, file: null, repeat: null,
  ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null, ...over,
});
const daily: Repeat = { freq: "day", interval: 1, ends: never, done: 0, at: "10:00" };
const day = { start: "08:00", end: "22:00" };
let nextId = 1;
const done = (due: number, doneAt = due + 5 * 60_000, itemId = 1): Done => ({ id: nextId++, item_id: itemId, due_at: due, done_at: doneAt });

afterEach(() => {
  i18n.choice = "en";
});

describe("a ticked repeating task opens again", () => {
  it("at the start of the day on its next date, never later than its own time", () => {
    expect(reopensAt(at(2026, 11, 19, 10), day)).toBe(at(2026, 11, 19, 8));
    expect(reopensAt(at(2026, 11, 19, 7), day)).toBe(at(2026, 11, 19, 7));
    expect(reopensAt(at(2026, 11, 19, 7), { start: "06:00", end: "22:00" })).toBe(at(2026, 11, 19, 6));
  });

  it("the evening before, when the day crosses midnight and its time is before the day's end", () => {
    const night = { start: "22:00", end: "06:00" };
    expect(reopensAt(at(2026, 11, 19, 2), night)).toBe(at(2026, 11, 18, 22));
    expect(reopensAt(at(2026, 11, 19, 23), night)).toBe(at(2026, 11, 19, 22));
    expect(reopensAt(at(2026, 11, 19, 12), night)).toBe(at(2026, 11, 19, 12));
    expect(reopensAt(at(2026, 11, 19, 6), night)).toBe(at(2026, 11, 19, 6));
    // (The first of a month: the last evening of the one before.)
    expect(reopensAt(at(2026, 12, 1, 2), night)).toBe(at(2026, 11, 30, 22));
    expect(reopensAt(at(2026, 11, 19, 2), day)).toBe(at(2026, 11, 19, 2));
  });

  it("only when it waits, ticked, for its next day", () => {
    const ticked = item({ due_at: at(2026, 11, 19, 10), done_at: at(2026, 11, 18, 10), repeat: { ...daily, ticked: { due: at(2026, 11, 18, 10), done: 0 } } });
    expect(waitsUntil(ticked, day)).toBe(at(2026, 11, 19, 8));
    // A task from before (no tick kept), an open one, an idea: never.
    expect(waitsUntil({ ...ticked, repeat: daily }, day)).toBeNull();
    expect(waitsUntil({ ...ticked, done_at: null }, day)).toBeNull();
    expect(waitsUntil({ ...ticked, kind: "idea" }, day)).toBeNull();
  });

  it("ticking moves it to the next occurrence and keeps what unticking gives back", () => {
    // Every Friday, this Friday's ticked early on Wednesday: next Friday.
    const fri: Repeat = { freq: "week", interval: 1, days: [5], ends: never, done: 3, at: "18:00" };
    const t1 = tickRepeat(fri, at(2026, 11, 20, 18), at(2026, 11, 18, 12))!;
    expect(t1.at).toBe(at(2026, 11, 27, 18));
    expect(t1.rule).toMatchObject({ done: 4, ticked: { due: at(2026, 11, 20, 18), done: 3 } });
    expect(untickedRule(t1.rule)).toEqual(fri);
    // The last of "after 4 times": over, nothing to move to.
    expect(tickRepeat({ ...fri, ends: { kind: "after", count: 4 } }, at(2026, 11, 20, 18), at(2026, 11, 20, 18))).toBeNull();
  });
});

describe("the calendar", () => {
  it("lays a month out in weeks from Monday", () => {
    // November 2026 starts on a Sunday: six days before it, then 30 days, five weeks and more.
    const g = monthGrid(2026, 10);
    expect(g[0].slice(0, 6)).toEqual([null, null, null, null, null, null]);
    expect(g[0][6]?.getDate()).toBe(1);
    expect(g.length).toBe(6);
    expect(g.flat().filter(Boolean).length).toBe(30);
    expect(g.every((w) => w.length === 7)).toBe(true);
  });

  it("marks the days a rule is due, from the day the task was made", () => {
    const keys = (s: Set<string>) => [...s].map((k) => Number(k.slice(-2)));
    const nov = (it: Item, start = new Date(2026, 8, 1)) => keys(dueDays(it, new Date(2026, 10, 1), new Date(2026, 10, 30), start));
    expect(nov(item({ due_at: at(2026, 11, 20, 18), repeat: { freq: "week", interval: 1, days: [5], ends: never } }))).toEqual([6, 13, 20, 27]);
    // Every 2 weeks on Monday, counted from its own Mondays.
    expect(nov(item({ due_at: at(2026, 11, 16, 9), repeat: { freq: "week", interval: 2, days: [1], ends: never } }))).toEqual([2, 16, 30]);
    // Weekdays, first due on the 18th (nothing done yet): from then.
    const wd = item({ due_at: at(2026, 11, 18, 9), repeat: { freq: "weekday", interval: 1, ends: never } });
    expect(seriesStart(wd, []).getDate()).toBe(18);
    expect(nov(wd, seriesStart(wd, []))).toEqual([18, 19, 20, 23, 24, 25, 26, 27, 30]);
    // Done before (its history starts on the 9th): from then.
    expect(seriesStart(wd, [done(at(2026, 11, 9, 9))]).getDate()).toBe(9);
    // Every month on the 31st: the 28th in February.
    const rent = item({ due_at: at(2027, 1, 31, 8), repeat: { freq: "month", interval: 1, monthday: 31, ends: never } });
    expect(keys(dueDays(rent, new Date(2027, 1, 1), new Date(2027, 1, 28), new Date(2027, 0, 1)))).toEqual([28]);
    // Every 3 days, from one of its own days.
    const three = item({ due_at: at(2026, 11, 3, 9), repeat: { freq: "day", interval: 3, ends: never } });
    expect(nov(three).slice(0, 4)).toEqual([3, 6, 9, 12]);
    expect(isOccurrence(three.repeat!, new Date(2026, 10, 4, 12), new Date(2026, 10, 3, 12))).toBe(false);
    // Not after its last day.
    expect(nov(item({ due_at: at(2026, 11, 2, 9), repeat: { ...daily, ends: { kind: "on", date: at(2026, 11, 4) } } }))).toEqual([1, 2, 3, 4]);
    // "After 5 times", 3 done, at the 19th now: the 19th and the 20th are its last two.
    const five = { ...daily, ends: { kind: "after", count: 5 }, done: 3 } as const;
    expect(nov(item({ due_at: at(2026, 11, 19, 10), repeat: five }), new Date(2026, 10, 17))).toEqual([17, 18, 19, 20]);
    // Ticked and waiting at the 20th (4 done): the 20th is the last.
    const waiting = item({ due_at: at(2026, 11, 20, 10), done_at: at(2026, 11, 19, 10), repeat: { ...five, done: 4, ticked: { due: at(2026, 11, 19, 10), done: 3 } } });
    expect(nov(waiting, new Date(2026, 10, 17))).toEqual([17, 18, 19, 20]);
    // Every Monday, "after 3", one done, at Monday the 9th: the 9th and the 16th.
    const mondays: Repeat = { freq: "week", interval: 1, days: [1], ends: { kind: "after", count: 3 }, done: 1 };
    expect(nov(item({ due_at: at(2026, 11, 9, 9), repeat: mondays }), new Date(2026, 10, 2))).toEqual([2, 9, 16]);
    // The same, its time moved by hand to Tuesday the 10th: that one is its second, the Monday
    // after (the 16th) its last, never the 23rd.
    expect(nov(item({ due_at: at(2026, 11, 10, 9), repeat: mondays }), new Date(2026, 10, 2))).toEqual([2, 9, 16]);
  });

  it("marks each time done on the day it was for (a Friday task ticked on Wednesday is Friday's)", () => {
    const log = [done(at(2026, 11, 20, 18), at(2026, 11, 18, 12)), done(at(2026, 11, 13, 18)), { ...done(at(2026, 11, 6, 18)), item_id: 2 }];
    expect([...doneDays(log, 1)].sort()).toEqual([dayKey(at(2026, 11, 13)), dayKey(at(2026, 11, 20))]);
  });
});

describe("a year of weeks or months", () => {
  const from = (cells: Cell[], y: number, m: number, d: number) => cells.find((c) => dayKey(c.from) === dayKey(at(y, m, d)))!;
  const states = (cells: Cell[]) => cells.map((c) => c.state);
  const run = (cells: Cell[], c: Cell, n: number) => states(cells.slice(cells.indexOf(c), cells.indexOf(c) + n));

  it("is the view of weekly and monthly rules; daily and weekdays keep the month", () => {
    expect(viewOf({ freq: "day", interval: 1, ends: never })).toBe("days");
    expect(viewOf({ freq: "weekday", interval: 1, ends: never })).toBe("days");
    expect(viewOf({ freq: "week", interval: 2, days: [1, 4], ends: never })).toBe("weeks");
    expect(viewOf({ freq: "month", interval: 1, monthday: 31, ends: never })).toBe("months");
    expect(viewOf(null)).toBe("days");
  });

  it("counts a year's weeks from Monday, ISO style: 52, or 53", () => {
    // 2026 starts on a Thursday, 2020 (leap) on a Wednesday: 53 weeks. 2025 and 2027: 52.
    expect(yearWeeks(2026).length).toBe(53);
    expect(yearWeeks(2020).length).toBe(53);
    expect(yearWeeks(2025).length).toBe(52);
    expect(yearWeeks(2027).length).toBe(52);
    expect(yearWeeks(2026).every((d) => d.getDay() === 1)).toBe(true);
    // Week 1 can start in December: 2025's on 30 December 2024, 2026's on 29 December 2025.
    expect(dayKey(yearWeeks(2025)[0])).toBe("2024-12-30");
    expect(dayKey(yearWeeks(2026)[0])).toBe("2025-12-29");
    // …and the last one end in January: 2026's 53rd is 28 December to 3 January 2027.
    expect(dayKey(yearWeeks(2026)[52])).toBe("2026-12-28");
    expect(dayKey(yearWeeks(2027)[0])).toBe("2027-01-04");
    expect(weekYear(new Date(2024, 11, 30, 12))).toBe(2025);
    expect(weekYear(new Date(2027, 0, 3, 12))).toBe(2026);
    expect(weekYear(new Date(2027, 0, 4, 12))).toBe(2027);
  });

  it("puts a time done across New Year in the week it belongs to", () => {
    const fri: Repeat = { freq: "week", interval: 1, days: [5], ends: never };
    const it2 = item({ due_at: at(2027, 1, 8, 9), repeat: fri });
    const log = [done(at(2026, 12, 25, 9)), done(at(2027, 1, 1, 9))];
    const start = seriesStart(it2, log);
    const y26 = weekCells(it2, log, 2026, start, new Date(at(2027, 1, 6)));
    const y27 = weekCells(it2, log, 2027, start, new Date(at(2027, 1, 6)));
    expect(y26[52]).toMatchObject({ state: "done", part: 1 });
    expect(dayKey(y26[52].days[0].date)).toBe("2027-01-01");
    expect(dayKey(y26[52].to)).toBe("2027-01-03");
    expect(y26[51].state).toBe("done");
    // 2027's first week (from the 4th): its Friday the 8th still to come, and current.
    expect(y27[0]).toMatchObject({ state: "later", current: true });
    // Before its first time: nothing, never "missed".
    expect(states(y26.slice(0, 51)).every((s) => s === "none")).toBe(true);
  });

  it("shows the weeks off of an every-2-weeks rule, nothing before its first one", () => {
    const two = item({ due_at: at(2026, 11, 16, 9), repeat: { freq: "week", interval: 2, days: [1], ends: never } });
    const log = [done(at(2026, 10, 5, 9)), done(at(2026, 10, 19, 9))];
    const start = seriesStart(two, log);
    const cells = weekCells(two, log, 2026, start, new Date(at(2026, 11, 16, 8)));
    expect(from(cells, 2026, 9, 28).state).toBe("none");
    // Done, off, done, off, missed (2 Nov), off, still to come (today's), off…
    expect(run(cells, from(cells, 2026, 10, 5), 9)).toEqual(["done", "off", "done", "off", "missed", "off", "later", "off", "later"]);
    expect(from(cells, 2026, 11, 16).current).toBe(true);
    expect(from(cells, 2026, 12, 28).state).toBe("later");
  });

  it("fills a week on several days in part when only some were done", () => {
    const monThu = item({ due_at: at(2026, 11, 16, 9), repeat: { freq: "week", interval: 1, days: [1, 4], ends: never } });
    const log = [done(at(2026, 11, 2, 9)), done(at(2026, 11, 5, 9)), done(at(2026, 11, 9, 9))];
    const start = seriesStart(monThu, log);
    const cells = weekCells(monThu, log, 2026, start, new Date(at(2026, 11, 18)));
    expect(from(cells, 2026, 11, 2)).toMatchObject({ state: "done", part: 1 });
    const nine = from(cells, 2026, 11, 9);
    expect(nine).toMatchObject({ state: "partial", part: 0.5 });
    expect(nine.days.map((d) => [d.date.getDate(), d.mark])).toEqual([
      [9, "done"],
      [12, "missed"],
    ]);
    // This week: Monday missed, Thursday to come.
    const now = from(cells, 2026, 11, 16);
    expect(now.state).toBe("missed");
    expect(now.days.map((d) => d.mark)).toEqual(["missed", "later"]);
    // Its Monday done: some done, the rest to come.
    const more = weekCells(monThu, [...log, done(at(2026, 11, 16, 9))], 2026, start, new Date(at(2026, 11, 18)));
    expect(from(more, 2026, 11, 16)).toMatchObject({ state: "partial", part: 0.5 });
    expect(from(more, 2026, 11, 16).days[1].mark).toBe("later");
    // Thursday's done on Wednesday (its time moved by hand): the week is done.
    const moved = weekCells(monThu, [...log, done(at(2026, 11, 16, 9)), done(at(2026, 11, 18, 9))], 2026, start, new Date(at(2026, 11, 19)));
    expect(from(moved, 2026, 11, 16)).toMatchObject({ state: "done", part: 1 });
  });

  it("lets a time moved by hand stand for its closest day, the next one first", () => {
    const words = (c: Cell) => markWords(c, "week", (d) => dayName(d.getDay(), true));
    // Monday and Thursday: Monday missed, Thursday's moved to Wednesday and done.
    const monThu = item({ due_at: at(2026, 11, 23, 9), repeat: { freq: "week", interval: 1, days: [1, 4], ends: never } });
    const log = [done(at(2026, 11, 18, 9))];
    const week = from(weekCells(monThu, log, 2026, new Date(at(2026, 11, 16)), new Date(at(2026, 11, 20))), 2026, 11, 16);
    expect(week.days.map((d) => [d.date.getDate(), d.mark])).toEqual([
      [16, "missed"],
      [18, "done"],
    ]);
    expect(words(week)).toBe("Done Wednesday, missed Monday");
    i18n.choice = "fr";
    expect(words(week)).toBe("Faite mercredi, manquée lundi");
    i18n.choice = "en";
    // Monday and Friday, Wednesday done (as close to both): it stands for Friday.
    const monFri = item({ due_at: at(2026, 11, 23, 9), repeat: { freq: "week", interval: 1, days: [1, 5], ends: never } });
    const both = from(weekCells(monFri, log, 2026, new Date(at(2026, 11, 16)), new Date(at(2026, 11, 22))), 2026, 11, 16);
    expect(words(both)).toBe("Done Wednesday, missed Monday");
  });

  it("keeps the past's rhythm when its time is moved by hand", () => {
    // Every 2 weeks on Friday: done 10 Oct, 24 Oct, 7 Nov, missed 21 Nov, then put off a week by
    // hand to 28 Nov. Today: Tuesday 25 Nov 2025.
    const fri = item({ due_at: at(2025, 11, 28, 18), created_at: at(2025, 10, 1), repeat: { freq: "week", interval: 2, days: [5], ends: never } });
    const log = [done(at(2025, 10, 10, 18)), done(at(2025, 10, 24, 18)), done(at(2025, 11, 7, 18))];
    const now = new Date(at(2025, 11, 25, 9));
    const start = seriesStart(fri, log);
    const cells = weekCells(fri, log, 2025, start, now);
    // Done, off, done, off, done, off, missed (21 Nov), to come (28 Nov, this week), off, to come.
    expect(run(cells, from(cells, 2025, 10, 6), 10)).toEqual(["done", "off", "done", "off", "done", "off", "missed", "later", "off", "later"]);
    expect(from(cells, 2025, 11, 24).current).toBe(true);
    // The month of days, and the summary, the same way.
    const nov = [...dueDays(fri, new Date(2025, 10, 1), new Date(2025, 10, 30), start, log, now)].map((k) => Number(k.slice(-2)));
    expect(nov).toEqual([7, 21, 28]);
    expect(summary(fri, log, now.getTime())).toMatchObject({ done: 3, of: 4, streak: 0 });
    // Every 3 days, done the 3rd, 6th and 9th, the 12th missed, put off from the 15th to the
    // 16th by hand: the past from its history, from today on from its time now.
    const three = item({ due_at: at(2026, 11, 16, 9), repeat: { freq: "day", interval: 3, ends: never } });
    const done3 = [3, 6, 9].map((d) => done(at(2026, 11, d, 9)));
    const today = new Date(at(2026, 11, 14, 9));
    const days = [...dueDays(three, new Date(2026, 10, 1), new Date(2026, 10, 30), seriesStart(three, done3), done3, today)].map((k) => Number(k.slice(-2)));
    expect(days).toEqual([3, 6, 9, 12, 16, 19, 22, 25, 28]);
  });

  it("names each row's months, never from its last square", () => {
    const names = (y: number) => {
      const w = yearWeeks(y);
      return [0, 1, 2, 3].map((r) => weekNames(w.slice(r * 13, r === 3 ? w.length : (r + 1) * 13)).map((n) => `${n.col}:${n.month.getMonth() + 1}`));
    };
    // 2027: April and July would start on a row's last square; the next row's first says them.
    expect(names(2027)).toEqual([
      ["0:1", "4:2", "8:3"],
      ["0:4", "4:5", "8:6"],
      ["0:7", "4:8", "8:9"],
      ["0:10", "4:11", "8:12"],
    ]);
    // 2026 (53 weeks): each row's first square says its month, the 53rd ends the last row.
    expect(names(2026)).toEqual([
      ["0:1", "5:2", "9:3"],
      ["0:4", "5:5", "9:6"],
      ["0:7", "5:8", "9:9"],
      ["0:10", "5:11", "9:12"],
    ]);
    // 2032: the last row's first week is September's last, October starts on the next one: it
    // says October, never two names on top of each other.
    expect(names(2032)[3]).toEqual(["1:10", "5:11", "9:12"]);
  });

  it("stops at the end rule: after N times, or on a date", () => {
    // Every Monday, after 3, one done, at the 9th: the 9th and the 16th, then nothing.
    const after = item({ due_at: at(2026, 11, 9, 9), repeat: { freq: "week", interval: 1, days: [1], ends: { kind: "after", count: 3 }, done: 1 } });
    const log = [done(at(2026, 11, 2, 9))];
    const cells = weekCells(after, log, 2026, seriesStart(after, log), new Date(at(2026, 11, 4)));
    expect(run(cells, from(cells, 2026, 11, 2), 5)).toEqual(["done", "later", "later", "none", "none"]);
    // Every 2 weeks, until 20 November: no weeks off after it.
    const until = item({ due_at: at(2026, 11, 2, 9), repeat: { freq: "week", interval: 2, days: [1], ends: { kind: "on", date: at(2026, 11, 20) } } });
    const c2 = weekCells(until, [], 2026, seriesStart(until, []), new Date(at(2026, 11, 1)));
    expect(run(c2, from(c2, 2026, 11, 2), 5)).toEqual(["later", "off", "later", "none", "none"]);
  });

  it("lays a monthly task's year out by month, the 31st as each month's last day", () => {
    const rent = item({ due_at: at(2027, 6, 30, 8), repeat: { freq: "month", interval: 1, monthday: 31, ends: never } });
    // (March's and May's missed.)
    const log = [done(at(2027, 1, 31, 8)), done(at(2027, 2, 28, 8)), done(at(2027, 4, 30, 8))];
    const cells = monthCells(rent, log, 2027, seriesStart(rent, log), new Date(at(2027, 6, 10)));
    expect(cells.length).toBe(12);
    expect(states(cells)).toEqual(["done", "done", "missed", "done", "missed", "later", "later", "later", "later", "later", "later", "later"]);
    expect(cells[1].days.map((d) => d.date.getDate())).toEqual([28]);
    expect(cells[3].days.map((d) => d.date.getDate())).toEqual([30]);
    expect(cells[5].current).toBe(true);
    expect(cells.filter((c) => c.current).length).toBe(1);
  });

  it("shows the months off of an every-2-months rule, from its first month (mid-year)", () => {
    const two = item({ due_at: at(2026, 11, 15, 9), repeat: { freq: "month", interval: 2, monthday: 15, ends: never } });
    const log = [done(at(2026, 5, 15, 9)), done(at(2026, 7, 16, 9)), done(at(2026, 9, 15, 9))];
    const cells = monthCells(two, log, 2026, seriesStart(two, log), new Date(at(2026, 11, 24)));
    // Nothing before May; the 16th of July done (a day late); November's missed.
    expect(states(cells)).toEqual(["none", "none", "none", "none", "done", "off", "done", "off", "done", "off", "missed", "off"]);
    expect(cells[6].days.map((d) => d.date.getDate())).toEqual([16]);
  });

  it("starts a weekly task's year at its first week", () => {
    const wk = item({ due_at: at(2026, 11, 23, 9), repeat: { freq: "week", interval: 1, days: [1], ends: never } });
    const log = [done(at(2026, 6, 8, 9)), done(at(2026, 6, 22, 9))];
    const start = seriesStart(wk, log);
    const cells = weekCells(wk, log, 2026, start, new Date(at(2026, 11, 24)));
    const i = cells.indexOf(from(cells, 2026, 6, 8));
    expect(states(cells.slice(0, i)).every((s) => s === "none")).toBe(true);
    expect(states(cells.slice(i, i + 3))).toEqual(["done", "missed", "done"]);
    // The year before it: all nothing.
    expect(states(weekCells(wk, log, 2025, start, new Date(at(2026, 11, 24)))).every((s) => s === "none")).toBe(true);
  });
});

describe("the summary, in the rule's own units", () => {
  const now = at(2026, 11, 24, 21);

  it("counts a daily task in days, and its streak", () => {
    const it = item({ created_at: at(2026, 9, 1), due_at: at(2026, 11, 25, 10), repeat: daily });
    // The last 30 days (25 Oct to 23 Nov, today's not done yet), done on all but three; the last 5 in a row.
    const log: Done[] = [];
    for (let d = 1; d <= 30; d++) if (![6, 11, 23].includes(d)) log.push(done(at(2026, 11, 24 - d, 10)));
    const s = summary(it, log, now)!;
    expect(s).toMatchObject({ unit: "days", done: 27, of: 30, streak: 5 });
    // Today's done: it counts (the window moves on a day), and the streak goes on.
    expect(summary(it, [...log, done(at(2026, 11, 24, 10))], now)).toMatchObject({ done: 27, of: 30, streak: 6 });
  });

  it("weekdays, weeks, months and times", () => {
    expect(unitOf({ freq: "weekday", interval: 1, ends: never })).toEqual({ unit: "weekdays", window: 20 });
    expect(unitOf({ freq: "week", interval: 1, days: [5], ends: never })).toEqual({ unit: "weeks", window: 8 });
    expect(unitOf({ freq: "week", interval: 1, days: [1, 4], ends: never })).toEqual({ unit: "times", window: 12 });
    expect(unitOf({ freq: "week", interval: 2, days: [1], ends: never })).toEqual({ unit: "times", window: 10 });
    expect(unitOf({ freq: "month", interval: 1, ends: never })).toEqual({ unit: "months", window: 6 });
    // Every Friday: the last 8 Fridays (2 Oct to 20 Nov), all done; then one missed (23 Oct):
    // 7 of 8, the last 4 in a row.
    const fri = item({ due_at: at(2026, 11, 27, 18), repeat: { freq: "week", interval: 1, days: [5], ends: never } });
    const log = [2, 9, 16, 23, 30].map((d) => done(at(2026, 10, d, 18))).concat([6, 13, 20].map((d) => done(at(2026, 11, d, 18))));
    expect(summary(fri, log, now)).toMatchObject({ unit: "weeks", done: 8, of: 8, streak: 8 });
    const missed = log.filter((r) => dayKey(r.due_at!) !== dayKey(at(2026, 10, 23)));
    expect(summary(fri, missed, now)).toMatchObject({ done: 7, of: 8, streak: 4 });
    // Weekdays: the weekend never breaks a streak.
    const wd = item({ due_at: at(2026, 11, 25, 9), repeat: { freq: "weekday", interval: 1, ends: never } });
    const week = [17, 18, 19, 20, 23].map((d) => done(at(2026, 11, d, 9)));
    expect(summary(wd, week, now)).toMatchObject({ unit: "weekdays", streak: 5 });
    // Ticked early (this Friday's, on Wednesday): it counts already.
    const early = item({ due_at: at(2026, 12, 4, 18), done_at: at(2026, 11, 25, 12), repeat: { freq: "week", interval: 1, days: [5], ends: never, ticked: { due: at(2026, 11, 27, 18), done: 8 } } });
    expect(summary(early, [...log, done(at(2026, 11, 27, 18), at(2026, 11, 25, 12))], at(2026, 11, 25, 13))).toMatchObject({ done: 8, streak: 9 });
  });

  it("a task made today, nothing done yet: nothing to count", () => {
    const fresh = item({ created_at: now, due_at: at(2026, 11, 25, 10), repeat: daily });
    expect(summary(fresh, [], now)).toMatchObject({ of: 0, ever: 0 });
  });

  it("reads in English and in French", () => {
    expect(t("Done {n} of the last {of} days", { n: 18, of: 30 })).toBe("Done 18 of the last 30 days");
    expect(t("{n} weeks in a row", { n: 3 })).toBe("3 weeks in a row");
    expect(t("Done {n} times so far", { n: 1 })).toBe("Done once so far");
    i18n.choice = "fr";
    expect(t("Done {n} of the last {of} days", { n: 18, of: 30 })).toBe("Faite 18 jours sur les 30 derniers");
    expect(t("Done {n} of the last {of} days", { n: 1, of: 30 })).toBe("Faite 1 jour sur les 30 derniers");
    expect(t("Done {n} of the last {of} weeks", { n: 7, of: 8 })).toBe("Faite 7 semaines sur les 8 dernières");
    expect(t("{n} days in a row", { n: 5 })).toBe("5 jours d’affilée");
    expect(nextOne(at(2026, 11, 25, 7, 30), new Date(at(2026, 11, 24, 21)))).toBe("la suivante demain à 07:30");
    i18n.choice = "en";
    expect(nextOne(at(2026, 11, 24, 22), new Date(at(2026, 11, 24, 9)))).toBe("next one today at 22:00");
  });
});

describe("the look back", () => {
  it("counts a repeating task once on each day it was done", () => {
    const mon = at(2026, 11, 16, 0);
    const tue = at(2026, 11, 17, 0);
    const wed = at(2026, 11, 18, 0);
    const monDone = done(at(2026, 11, 16, 10), at(2026, 11, 16, 11));
    // Still ticked on Monday's evening: the line, once.
    const ticked = item({ due_at: at(2026, 11, 17, 10), done_at: monDone.done_at, repeat: { ...daily, ticked: { due: at(2026, 11, 16, 10), done: 0 } } });
    expect(doneBetween([ticked], [monDone], mon, tue).map((e) => e.key)).toEqual(["i1"]);
    // Open again on Tuesday: Monday still has it, from its history.
    const open = { ...ticked, done_at: null, repeat: daily };
    expect(doneBetween([open], [monDone], mon, tue).map((e) => e.text)).toEqual(["Stretch"]);
    expect(doneBetween([open], [monDone], tue, wed)).toEqual([]);
    // A line deleted for good: its history isn't shown.
    expect(doneBetween([], [monDone], mon, tue)).toEqual([]);
  });
});

describe("the store's tick of a repeating task (the preview's backend)", () => {
  // (Each on the preview's sample notes as they start.)
  beforeEach(() => {
    vi.resetModules();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("one line: ticked it waits for its next day, unticked or undone it is as it was, redone ticked again", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const lines = store.items.length;
    const had = store.logOf(weekly.id).length;
    await store.toggleDone(weekly);
    let now = store.items.find((i) => i.id === weekly.id)!;
    expect(store.items.length).toBe(lines);
    expect(now.done_at).not.toBeNull();
    expect(now.due_at).toBe(weekly.due_at! + 7 * 86400_000);
    expect(now.repeat?.ticked?.due).toBe(weekly.due_at);
    expect(store.logOf(weekly.id).length).toBe(had + 1);
    // Unticked with its box: back as it was.
    await store.toggleDone(now);
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at });
    expect(now.repeat?.ticked).toBeUndefined();
    expect(now.repeat?.done).toBe(weekly.repeat?.done);
    expect(store.logOf(weekly.id).length).toBe(had);
    // Ticked, then Undo ("Task back to do"), then Redo.
    await store.toggleDone(now);
    await store.history.undo();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at });
    expect(store.logOf(weekly.id).length).toBe(had);
    await store.history.redo();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now.done_at).not.toBeNull();
    expect(now.due_at).toBe(weekly.due_at! + 7 * 86400_000);
    // Its next day comes: open again at its next time, its history kept.
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(now.due_at! - 2 * 3600_000));
    await store.reload();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at! + 7 * 86400_000 });
    expect(now.repeat?.ticked).toBeUndefined();
    expect(store.logOf(weekly.id).length).toBe(had + 1);
    // Undo of that "Done" now: its time and its count back, its time out of the history.
    await store.history.undo();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at });
    expect(store.logOf(weekly.id).length).toBe(had);
  });

  it("one Undo never takes back two times: ticked on the board, open again, ticked from a notification", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const had = store.logOf(weekly.id).length;
    // T1 on the board; its next day comes; T2 from its notification.
    await store.toggleDone(weekly);
    const t1 = store.items.find((i) => i.id === weekly.id)!.done_at!;
    const second = weekly.due_at! + 7 * 86400_000;
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(second - 2 * 3600_000));
    await store.reload();
    let now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: second });
    vi.setSystemTime(new Date(second + 60_000));
    await store.answer(now, "done");
    now = store.items.find((i) => i.id === weekly.id)!;
    const t2 = now.done_at!;
    expect(t2).not.toBe(t1);
    expect(now.due_at).toBe(second + 7 * 86400_000);
    // Ctrl+Z (the board's "Done", T1): T1 leaves its history, T2 stays, and so does the line.
    await store.history.undo();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: t2, due_at: second + 7 * 86400_000 });
    expect(now.repeat?.ticked?.due).toBe(second);
    const times = store.logOf(weekly.id).map((r) => r.done_at);
    expect(times).toContain(t2);
    expect(times).not.toContain(t1);
    expect(times.length).toBe(had + 1);
  });

  it("its steps start fresh when it opens again, and Undo gives them back exactly", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const step = (await store.addStep(weekly, "Inbox zero"))!;
    const plain = (await store.addStep(weekly, "Notes"))!;
    const stepDue = weekly.due_at! - 3600_000;
    await store.setDue(store.items.find((i) => i.id === step.id)!, stepDue);
    await store.toggleDone(store.items.find((i) => i.id === step.id)!);
    await store.toggleDone(store.items.find((i) => i.id === plain.id)!);
    const stepDone = store.items.find((i) => i.id === step.id)!.done_at;
    expect(stepDone).not.toBeNull();
    // A third one, snoozed.
    const snoozed = (await store.addStep(store.items.find((i) => i.id === weekly.id)!, "Call"))!;
    await store.setDue(store.items.find((i) => i.id === snoozed.id)!, stepDue);
    await store.answer(store.items.find((i) => i.id === snoozed.id)!, "snooze:10");
    const snoozedUntil = store.items.find((i) => i.id === snoozed.id)!.snooze_until;
    expect(snoozedUntil).not.toBeNull();
    // The task ticked: its steps as they were while it waits.
    await store.toggleDone(store.items.find((i) => i.id === weekly.id)!);
    expect(store.items.find((i) => i.id === step.id)).toMatchObject({ done_at: stepDone, due_at: stepDue });
    // Its next day: open again, its steps unticked, the timed one a week on.
    const week = 7 * 86400_000;
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(weekly.due_at! + week - 2 * 3600_000));
    await store.reload();
    expect(store.items.find((i) => i.id === weekly.id)).toMatchObject({ done_at: null, due_at: weekly.due_at! + week });
    expect(store.items.find((i) => i.id === step.id)).toMatchObject({ done_at: null, due_at: stepDue + week, ring_state: "" });
    expect(store.items.find((i) => i.id === plain.id)).toMatchObject({ done_at: null, due_at: null });
    // The times they were done stay in the look back of that day (once each), in their own
    // history.
    expect(store.logOf(step.id).map((r) => [r.due_at, r.done_at])).toEqual([[stepDue, stepDone]]);
    const from = new Date(stepDone!).setHours(0, 0, 0, 0);
    const to = new Date(stepDone!).setHours(24, 0, 0, 0);
    expect(doneBetween(store.items, store.doneLog, from, to).filter((e) => e.text === "Inbox zero").length).toBe(1);
    // Undo of the task's "Done": the task and its steps exactly as before, their reminders too
    // (a snooze past since waits a minute, never ringing at once), and the steps' times out of
    // the history again.
    await store.history.undo();
    expect(store.items.find((i) => i.id === weekly.id)).toMatchObject({ done_at: null, due_at: weekly.due_at });
    expect(store.items.find((i) => i.id === step.id)).toMatchObject({ done_at: stepDone, due_at: stepDue });
    expect(store.items.find((i) => i.id === plain.id)!.done_at).not.toBeNull();
    expect(store.items.find((i) => i.id === snoozed.id)).toMatchObject({ due_at: stepDue, ring_state: "snoozed", snooze_until: Date.now() + 60_000 });
    expect(snoozedUntil! < Date.now()).toBe(true);
    expect(store.logOf(step.id)).toEqual([]);
    expect(store.logOf(plain.id)).toEqual([]);
    expect((await store.backend.loadAll()).done_log!.filter((r) => r.item_id === step.id || r.item_id === plain.id)).toEqual([]);
  });

  it("Undo leaves a step moved to another task since as it is", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const other = store.items.find((i) => i.kind === "task" && i.parent_id == null && !i.repeat && i.id !== weekly.id)!;
    const step = (await store.addStep(weekly, "Notes"))!;
    await store.toggleDone(store.items.find((i) => i.id === step.id)!);
    await store.toggleDone(store.items.find((i) => i.id === weekly.id)!);
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(weekly.due_at! + 7 * 86400_000 - 2 * 3600_000));
    await store.reload();
    // Made a step of another task, then the Undo: it stays there, unticked.
    const fresh = store.items.find((i) => i.id === step.id)!;
    expect(fresh.done_at).toBeNull();
    await store.backend.updateItem({ ...fresh, parent_id: other.id });
    await store.reload();
    await store.history.undo();
    expect(store.items.find((i) => i.id === weekly.id)).toMatchObject({ done_at: null, due_at: weekly.due_at });
    expect(store.items.find((i) => i.id === step.id)).toMatchObject({ parent_id: other.id, done_at: null });
  });

  it("an untick of a ticked copy kept from before it opened again leaves nothing to undo", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    await store.toggleDone(weekly);
    const stale = store.items.find((i) => i.id === weekly.id)!;
    const steps = (store.history as unknown as { past: unknown[] }).past.length;
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(weekly.due_at! + 7 * 86400_000 - 2 * 3600_000));
    // (Opened again in the backend; the board still has its ticked copy, and its box is clicked.)
    await store.backend.loadAll();
    await store.toggleDone(stale);
    const now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at! + 7 * 86400_000 });
    expect((store.history as unknown as { past: unknown[] }).past.length).toBe(steps);
  });

  it("one Undo of a tick moved on twice since only takes that time out of the history", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const had = store.logOf(weekly.id).length;
    const week = 7 * 86400_000;
    // T1 on the board; open again; T2 from its notification; open again.
    await store.toggleDone(weekly);
    const t1 = store.items.find((i) => i.id === weekly.id)!.done_at!;
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(weekly.due_at! + week - 2 * 3600_000));
    await store.reload();
    vi.setSystemTime(new Date(weekly.due_at! + week + 60_000));
    await store.answer(store.items.find((i) => i.id === weekly.id)!, "done");
    vi.setSystemTime(new Date(weekly.due_at! + 2 * week - 2 * 3600_000));
    await store.reload();
    let now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at! + 2 * week });
    // Undo of T1: the line stays at its third time, T1 alone leaves its history.
    await store.history.undo();
    now = store.items.find((i) => i.id === weekly.id)!;
    expect(now).toMatchObject({ done_at: null, due_at: weekly.due_at! + 2 * week });
    expect(now.repeat?.done).toBe(weekly.repeat!.done! + 2);
    const times = store.logOf(weekly.id).map((r) => r.done_at);
    expect(times).not.toContain(t1);
    expect(times.length).toBe(had + 1);
  });

  it("opened late, its steps whose time is past wait a minute", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const step = (await store.addStep(weekly, "Inbox zero"))!;
    await store.setDue(store.items.find((i) => i.id === step.id)!, weekly.due_at! - 3600_000);
    await store.toggleDone(store.items.find((i) => i.id === weekly.id)!);
    // The computer asleep until after both times on its next day.
    const late = weekly.due_at! + 7 * 86400_000 + 30 * 60_000;
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(late));
    await store.reload();
    expect(store.items.find((i) => i.id === step.id)).toMatchObject({ done_at: null, snooze_until: late + 60_000 });
    expect(store.items.find((i) => i.id === weekly.id)).toMatchObject({ done_at: null, snooze_until: late + 60_000 });
  });

  it("a stale ticked copy saved after it opened again never ticks it twice", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const had = store.logOf(weekly.id).length;
    await store.toggleDone(weekly);
    const stale = { ...store.items.find((i) => i.id === weekly.id)! };
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(weekly.due_at! + 7 * 86400_000 - 2 * 3600_000));
    // (Opened again in the backend; the board still has its ticked copy and renames it.)
    await store.backend.loadAll();
    const saved = await store.backend.updateItem({ ...stale, text: "Weekly, short" });
    expect(saved).toMatchObject({ text: "Weekly, short", done_at: null, due_at: weekly.due_at! + 7 * 86400_000 });
    expect(saved.repeat?.ticked).toBeUndefined();
    expect((await store.backend.loadAll()).done_log!.filter((r) => r.item_id === weekly.id).length).toBe(had + 1);
  });

  it("the preview's backend keeps a repeat's own time, as the app's", async () => {
    const { testStore } = await import("./testStore");
    const { store } = await testStore();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    const plain = { ...weekly.repeat! };
    delete plain.at;
    const saved = await store.backend.updateItem({ ...weekly, repeat: plain });
    const d = new Date(weekly.due_at!);
    expect(saved.repeat?.at).toBe(`${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`);
  });
});
