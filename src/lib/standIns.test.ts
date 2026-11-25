// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A repeating task's history: a time done on a day of its own (its time moved by hand) stands for
// its closest occurrence, the same way for the squares, their words and the summary, also across
// a week's or a month's edge; the look back keeps a time from the history once the line is an
// idea; a step moves on by calendar days.
import { describe, expect, it } from "vitest";
import { daysApart, doneBetween, monthCells, seriesStart, shiftDays, summary, weekCells } from "./doneLog";
import type { Cell } from "./doneLog";
import type { Done, Item, Repeat } from "./types";

const never = { kind: "never" } as const;
const at = (y: number, m: number, d: number, h = 12, mi = 0) => new Date(y, m - 1, d, h, mi).getTime();
const item = (over: Partial<Item>): Item => ({
  id: 1, note_id: 1, kind: "task", text: "Stretch", due_at: null, done_at: null, sort: 0, created_at: at(2026, 1, 1), parent_id: null, alarm: "reminder", rhythm: null, link: null, file: null, repeat: null,
  ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null, ...over,
});
let nextId = 1;
const done = (due: number, doneAt = due + 60_000): Done => ({ id: nextId++, item_id: 1, due_at: due, done_at: doneAt });
const from = (cells: Cell[], y: number, m: number, d: number) => cells.find((c) => c.from.getTime() === new Date(y, m - 1, d, 12).getTime())!;
const marks = (c: Cell) => c.days.map((x) => [x.date.getDate(), x.mark]);

describe("a time done on a day of its own", () => {
  it("counts in the summary as in the squares (weekly, one week moved to its Wednesday)", () => {
    const rule: Repeat = { freq: "week", interval: 1, days: [1], ends: never, done: 8, at: "10:00" };
    const mondays = [at(2026, 10, 5, 10), at(2026, 10, 12, 10), at(2026, 10, 19, 10), at(2026, 10, 26, 10), at(2026, 11, 2, 10), at(2026, 11, 9, 10), at(2026, 11, 18, 10), at(2026, 11, 23, 10)];
    const log = mondays.map((m) => done(m));
    const it = item({ due_at: at(2026, 11, 30, 10), done_at: at(2026, 11, 23, 10, 5), repeat: { ...rule, ticked: { due: at(2026, 11, 23, 10), done: 7 } } });
    const now = new Date(2026, 10, 25, 12);
    const cells = weekCells(it, log, 2026, seriesStart(it, log), now);
    expect(cells.filter((c) => c.state === "done").length).toBe(8);
    expect(summary(it, log, now.getTime())).toMatchObject({ unit: "weeks", done: 8, of: 8, streak: 8 });
  });

  it("counts in the summary as in the squares (monthly on the 15th, one month done on the 18th)", () => {
    const rule: Repeat = { freq: "month", interval: 1, days: [], monthday: 15, ends: never, done: 5, at: "10:00" };
    const log = [done(at(2026, 6, 15, 10)), done(at(2026, 7, 15, 10)), done(at(2026, 8, 18, 10)), done(at(2026, 9, 15, 10)), done(at(2026, 10, 15, 10)), done(at(2026, 11, 15, 10))];
    const it = item({ due_at: at(2026, 12, 15, 10), repeat: rule });
    const now = new Date(2026, 10, 25, 12);
    const cells = monthCells(it, log, 2026, seriesStart(it, log), now);
    expect(cells.filter((c) => c.state === "done").length).toBe(6);
    expect(summary(it, log, now.getTime())).toMatchObject({ unit: "months", done: 6, of: 6, streak: 6 });
  });

  it("stands for its occurrence across a week's edge: a Monday done the Sunday before", () => {
    const rule: Repeat = { freq: "week", interval: 1, days: [1], ends: never, done: 4, at: "10:00" };
    // Mondays 2, 9, 23 November done; the 16th's moved by hand to Sunday the 15th, done then.
    const log = [done(at(2026, 11, 2, 10)), done(at(2026, 11, 9, 10)), done(at(2026, 11, 15, 10)), done(at(2026, 11, 23, 10))];
    const it = item({ due_at: at(2026, 11, 30, 10), repeat: rule });
    const now = new Date(2026, 10, 25, 12);
    const cells = weekCells(it, log, 2026, seriesStart(it, log), now);
    // The week of the 9th: its own Monday alone; the week of the 16th: done, by Sunday's.
    expect(from(cells, 2026, 11, 9)).toMatchObject({ state: "done", part: 1 });
    expect(marks(from(cells, 2026, 11, 9))).toEqual([[9, "done"]]);
    expect(from(cells, 2026, 11, 16)).toMatchObject({ state: "done", part: 1 });
    expect(marks(from(cells, 2026, 11, 16))).toEqual([[15, "done"]]);
    expect(summary(it, log, now.getTime())).toMatchObject({ done: 4, of: 4, streak: 4 });
  });

  it("stands for nothing further than half the rule's period", () => {
    // Every Monday: the 2nd and the 9th done, a time on Thursday the 12th (4 days before the
    // 16th: more than half a week), the 16th missed.
    const rule: Repeat = { freq: "week", interval: 1, days: [1], ends: never, done: 3, at: "10:00" };
    const it = item({ due_at: at(2026, 11, 23, 10), repeat: rule });
    const log = [done(at(2026, 11, 2, 10)), done(at(2026, 11, 9, 10)), done(at(2026, 11, 12, 10))];
    const now = new Date(2026, 10, 18, 12);
    const cells = weekCells(it, log, 2026, seriesStart(it, log), now);
    expect(marks(from(cells, 2026, 11, 9))).toEqual([
      [9, "done"],
      [12, "done"],
    ]);
    expect(from(cells, 2026, 11, 16)).toMatchObject({ state: "missed" });
    expect(summary(it, log, now.getTime())).toMatchObject({ done: 2, of: 3, streak: 0 });
  });
});

describe("the look back", () => {
  it("keeps a time from the history once its line is an idea (reminders.rs lookback_count)", () => {
    const t = at(2026, 11, 19, 11);
    const idea = item({ kind: "idea", done_at: t, created_at: at(2026, 10, 1) });
    const log: Done[] = [{ id: 9, item_id: 1, due_at: at(2026, 11, 19, 10), done_at: t }];
    expect(doneBetween([idea], log, at(2026, 11, 19, 0), at(2026, 11, 20, 0)).map((e) => e.key)).toEqual(["d9"]);
    // A task still ticked at that time: shown once, as the line.
    const task = item({ done_at: t, repeat: { freq: "day", interval: 1, ends: never } });
    expect(doneBetween([task], log, at(2026, 11, 19, 0), at(2026, 11, 20, 0)).map((e) => e.key)).toEqual(["i1"]);
  });
});

describe("a step of a repeating task open again", () => {
  it("moves on by whole calendar days at its own time of day (repeat.rs shift_days)", () => {
    // (Each a day the clocks change in Europe or the US: still 09:00, whatever the hours between.)
    for (const [y, m, d] of [
      [2026, 3, 28],
      [2026, 10, 24],
      [2026, 3, 7],
      [2026, 10, 31],
    ]) {
      expect(shiftDays(at(y, m, d, 9), 1)).toBe(at(y, m, d + 1, 9));
      expect(shiftDays(at(y, m, d, 9), 7)).toBe(at(y, m, d + 7, 9));
      expect(daysApart(at(y, m, d, 22), at(y, m, d + 1, 7))).toBe(1);
    }
    expect(shiftDays(at(2026, 11, 18, 9, 15), -2)).toBe(at(2026, 11, 16, 9, 15));
    expect(daysApart(at(2026, 11, 18, 10), at(2026, 11, 18, 23))).toBe(0);
  });
});
