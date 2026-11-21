// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Tests for the page's own logic (the Rust side has its own with `cargo test`):
// repeats, the drag ruler's hours, the "Later…" times, the look back window, undo, smart dates.
import { describe, expect, it } from "vitest";
import { findRepeat, firstOccurrence, nextAfterDone, nextOccurrence, repeatLabel } from "./repeat";
import { hourLabelStep, laterChoices, lookbackDay, rulerRange, timeAt } from "./plan";
import { History } from "./history";
import { findDate, parseCapture } from "./capture";

const day = { start: "08:00", end: "22:00" };
const work = { on: true, days: [1, 2, 3, 4, 5], from: "09:00", until: "19:00" };
// Wednesday 18 Nov 2026.
const wed = (h: number, m = 0) => new Date(2026, 10, 18, h, m).getTime();
const MIN = 60_000;

describe("repeats", () => {
  it("understands typed repeats in English and French", () => {
    expect(findRepeat("Weekly every Monday 10h")?.rule).toMatchObject({ freq: "week", days: [1] });
    expect(findRepeat("Weekly tous les lundis 10h")?.rule).toMatchObject({ freq: "week", days: [1] });
    expect(findRepeat("standup every weekday 9h")?.rule.freq).toBe("weekday");
    expect(findRepeat("loyer chaque 1er du mois")?.rule).toMatchObject({ freq: "month", monthday: 1 });
    expect(findRepeat("review every 2 weeks")?.rule).toMatchObject({ freq: "week", interval: 2 });
    expect(findRepeat("sport tous les lundis et jeudis")?.rule.days).toEqual([1, 4]);
    expect(findRepeat("Weekly")).toBeNull();
    expect(findRepeat("Spline v2")).toBeNull();
  });

  it("the typed line keeps its words and gets the first occurrence", () => {
    const p = parseCapture("Weekly every Monday 10h #hub", "task", new Date(wed(14)));
    expect(p.text).toBe("Weekly");
    expect(p.tag).toBe("hub");
    expect(p.repeat?.freq).toBe("week");
    expect(p.due?.getDay()).toBe(1);
    expect(p.due?.getHours()).toBe(10);
    expect(p.due!.getTime()).toBeGreaterThan(wed(14));
  });

  it("next occurrences match the Rust rules", () => {
    const mon = new Date(2026, 10, 16, 10).getTime();
    expect(new Date(nextOccurrence({ freq: "week", interval: 1, days: [1], ends: { kind: "never" } }, mon)!).getDate()).toBe(23);
    const fri = new Date(2026, 10, 20, 9).getTime();
    expect(new Date(nextOccurrence({ freq: "weekday", interval: 1, ends: { kind: "never" } }, fri)!).getDay()).toBe(1);
    const jan31 = new Date(2026, 0, 31, 8).getTime();
    expect(new Date(nextOccurrence({ freq: "month", interval: 1, monthday: 31, ends: { kind: "never" } }, jan31)!).getDate()).toBe(28);
    expect(nextAfterDone({ freq: "day", interval: 1, ends: { kind: "after", count: 2 }, done: 1 }, mon, mon)).toBeNull();
    // Ticked late (Wednesday): jumps to the next Monday still to come, missed ones counted.
    const weekly = { freq: "week" as const, interval: 1, days: [1], ends: { kind: "never" as const }, done: 0 };
    const late = nextAfterDone(weekly, mon, new Date(2026, 10, 18, 15).getTime())!;
    expect(new Date(late.at).getDate()).toBe(23);
    expect(late.done).toBe(1);
    const veryLate = nextAfterDone(weekly, mon, new Date(2026, 11, 9, 12).getTime())!;
    expect(new Date(veryLate.at).getDate()).toBe(14);
    expect(veryLate.done).toBe(4);
    expect(firstOccurrence({ freq: "day", interval: 1, ends: { kind: "never" } }, 9, 0, new Date(wed(14))).getDate()).toBe(19);
  });

  it("labels", () => {
    expect(repeatLabel({ freq: "week", interval: 1, days: [1], ends: { kind: "never" } })).toBe("Every Monday");
    expect(repeatLabel({ freq: "week", interval: 2, days: [1, 4], ends: { kind: "never" } })).toBe("Every 2 weeks on Mon and Thu");
    expect(repeatLabel({ freq: "month", interval: 1, monthday: 1, ends: { kind: "after", count: 4 } })).toBe("Every month on the 1st · 4 times");
  });
});

describe("the drag ruler's hours", () => {
  it("Work: from now to a little after work hours end; Tomorrow = next work day", () => {
    const r = rulerRange(wed(14, 42), "work", day, work);
    expect(r.main.start).toBe(wed(14, 30));
    expect(r.main.end).toBe(wed(20));
    expect(r.next.label).toBe("Tomorrow");
    expect(new Date(r.next.start).getHours()).toBe(9);
  });

  it("Personal: to the end of the day; late in the evening it shows tomorrow", () => {
    expect(rulerRange(wed(14, 42), "personal", day, work).main.end).toBe(wed(22));
    const late = rulerRange(wed(21, 50), "personal", day, work);
    expect(late.main.label).toBe("Tomorrow");
    expect(new Date(late.main.start).getHours()).toBe(8);
  });

  it("Friday evening in Work leads to Monday", () => {
    const fri = new Date(2026, 10, 20, 21).getTime();
    const r = rulerRange(fri, "work", day, work);
    expect(r.main.label).toBe("Monday");
  });

  it("snaps to the quarter hour unless exact", () => {
    const span = { start: wed(15), end: wed(19), label: "Today" };
    expect(timeAt(span, 0.3)).toBe(wed(16, 15));
    expect(timeAt(span, 0.3, true)).toBe(wed(16, 12));
  });
});

describe("Later… times", () => {
  it("this evening, tomorrow morning, next Monday", () => {
    const c = laterChoices(wed(14, 42), day);
    expect(c.map((x) => x.label)).toEqual(["1 hour", "This evening", "Tomorrow morning", "Next Monday"]);
    expect(c[1].at).toBe(wed(19));
    expect(new Date(c[2].at).getDate()).toBe(19);
    expect(new Date(c[3].at).getDay()).toBe(1);
    // After 18:30 there is no "this evening".
    expect(laterChoices(wed(19, 10), day).some((x) => x.label === "This evening")).toBe(false);
  });
});

describe("the look back", () => {
  it("shows from the end of work until the next morning", () => {
    expect(lookbackDay(wed(18, 59), day, work)).toBeNull();
    expect(lookbackDay(wed(19, 1), day, work)?.getDate()).toBe(18);
    const night = new Date(2026, 10, 19, 1).getTime();
    expect(lookbackDay(night, day, work)?.getDate()).toBe(18);
    const morning = new Date(2026, 10, 19, 8, 5).getTime();
    expect(lookbackDay(morning, day, work)).toBeNull();
  });
});

describe("undo", () => {
  it("undoes, redoes, and a new change drops the redo", async () => {
    let v = 0;
    const h = new History();
    const set = (to: number) => {
      const from = v;
      v = to;
      h.record({ label: "set", undo: () => void (v = from), redo: () => void (v = to) });
    };
    set(1);
    set(2);
    expect(await h.undo()).toBe("set");
    expect(v).toBe(1);
    expect(await h.redo()).toBe("set");
    expect(v).toBe(2);
    await h.undo();
    set(5);
    expect(h.canRedo).toBe(false);
    await h.undo();
    expect(v).toBe(1);
  });

  it("takes one step at a time, also when asked twice at once", async () => {
    const h = new History();
    const log: string[] = [];
    const slow = (name: string) => async () => {
      log.push(`${name} starts`);
      await new Promise((r) => setTimeout(r, 5));
      log.push(`${name} ends`);
    };
    h.record({ label: "a", undo: slow("undo a"), redo: slow("redo a") });
    h.record({ label: "b", undo: slow("undo b"), redo: slow("redo b") });
    expect(await Promise.all([h.undo(), h.undo(), h.redo()])).toEqual(["b", "a", "a"]);
    expect(log).toEqual(["undo b starts", "undo b ends", "undo a starts", "undo a ends", "redo a starts", "redo a ends"]);
    // A step that fails doesn't stop the next.
    h.record({ label: "c", undo: () => Promise.reject(new Error("no")), redo: () => {} });
    await expect(h.undo()).rejects.toThrow("no");
    expect(await h.undo()).toBe("a");
  });

  it("forgets everything after a restore", async () => {
    let v = 0;
    const h = new History();
    h.record({ label: "set", undo: () => void (v = -1), redo: () => void (v = 1) });
    h.clear();
    expect(h.canUndo).toBe(false);
    expect(await h.undo()).toBeNull();
    expect(v).toBe(0);
  });
});

describe("smart dates in notes", () => {
  it("finds times, and leaves look-alikes alone", () => {
    const ref = new Date(wed(14));
    expect(findDate("Call the client demain 9h", ref)?.certain).toBe(true);
    expect(findDate("Send build lundi 14h30", ref)?.certain).toBe(true);
    expect(findDate("Spline v2", ref)).toBeNull();
    expect(findDate("Get some product, 1/2 bag", ref)).toBeNull();
    expect(findDate("Walkthrough Hangar Y 15", ref)).toBeNull();
    const f = findDate("Fix glass tomorrow 3pm please", ref)!;
    expect("Fix glass tomorrow 3pm please".slice(f.index, f.index + f.length)).toBe("tomorrow 3pm");
    expect(f.due.getHours()).toBe(15);
  });
  void MIN;
});

describe("hour labels on the ruler", () => {
  it("labels every hour when there is room", () => {
    expect(hourLabelStep(8, 600)).toBe(1);
  });
  it("thins a long day on a narrow track so labels never touch", () => {
    // 21 hours on 300 px: 14 px an hour.
    const s = hourLabelStep(21, 300);
    expect(s).toBe(3);
    expect((300 / 21) * s).toBeGreaterThanOrEqual(32);
    expect(hourLabelStep(24, 230)).toBe(4);
  });
  it("keeps to clock-friendly steps", () => {
    for (let w = 40; w < 1200; w += 7) expect([1, 2, 3, 4, 6, 12, 24]).toContain(hourLabelStep(24, w));
  });
});
