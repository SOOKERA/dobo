// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Tests for the page's own logic (the Rust side has its own with `cargo test`):
// repeats, the drag ruler's hours, the "Later…" times, the look back window, undo, smart dates.
import { describe, expect, it } from "vitest";
import { findRepeat, firstOccurrence, nextAfterDone, nextOccurrence, repeatLabel } from "./repeat";
import { crossesMidnight, daylight, endName, hourLabelStep, inWorkHours, laterChoices, lookbackDay, lookbackSpan, rulerHours, rulerRange, spanDay, timeAt } from "./plan";
import { History, changedKeys, withFields } from "./history";
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
    // Its next-day spot is then the day after tomorrow: by its name, never "Tomorrow" again.
    expect(new Date(late.next.start).getDate()).toBe(new Date(wed(12)).getDate() + 2);
    expect(late.next.label).toBe("Friday");
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

describe("work hours that cross midnight (22:00 to 06:00)", () => {
  // Monday to Friday nights; 16 Nov 2026 is a Monday.
  const night = { on: true, days: [1, 2, 3, 4, 5], from: "22:00", until: "06:00" };
  const at = (d: number, h: number, m = 0) => new Date(2026, 10, d, h, m).getTime();

  it("count for the day they start", () => {
    expect(crossesMidnight(night)).toBe(true);
    expect(crossesMidnight(work)).toBe(false);
    expect(inWorkHours(night, at(16, 21, 59))).toBe(false);
    expect(inWorkHours(night, at(16, 22))).toBe(true);
    expect(inWorkHours(night, at(17, 3))).toBe(true);
    expect(inWorkHours(night, at(17, 6))).toBe(false);
    expect(inWorkHours(night, at(17, 12))).toBe(false);
    // Saturday 02:00 is still Friday's night; Monday 02:00 was Sunday's (not a work day).
    expect(inWorkHours(night, at(21, 2))).toBe(true);
    expect(inWorkHours(night, at(21, 22))).toBe(false);
    expect(inWorkHours(night, at(23, 2))).toBe(false);
    // Day hours as before; the same start and end is never.
    expect(inWorkHours(work, wed(12))).toBe(true);
    expect(inWorkHours(work, wed(20))).toBe(false);
    expect(inWorkHours({ ...night, until: "22:00" }, at(16, 22))).toBe(false);
  });

  it("the ruler in Work runs to a little after they end, the next morning", () => {
    const r = rulerRange(at(16, 20), "work", day, night);
    expect(r.main.end).toBe(at(17, 7));
    // In the middle of the night, the night that is on.
    expect(rulerRange(at(17, 2), "work", day, night).main.end).toBe(at(17, 7));
    // After it ends, today runs to the end of tonight's; Tomorrow is the next night, from 22:00
    // to 06:00 the morning after.
    const r2 = rulerRange(at(17, 6, 50), "work", day, night);
    expect(r2.main.end).toBe(at(18, 7));
    expect([r2.next.start, r2.next.end]).toEqual([at(18, 22), at(19, 6)]);
  });

  it("the look back comes when they end, and shows until the day's own", () => {
    // Monday's night ends Tuesday 06:00: Monday's look back, all Tuesday until Tuesday's.
    expect(lookbackDay(at(17, 5, 59), day, night)).toBeNull();
    expect(lookbackDay(at(17, 6, 1), day, night)?.getDate()).toBe(16);
    expect(lookbackDay(at(17, 18), day, night)?.getDate()).toBe(16);
    // Saturday (not a work day) ends with My day: Friday's from Saturday 06:00, then Saturday's.
    expect(lookbackDay(at(21, 12), day, night)?.getDate()).toBe(20);
    expect(lookbackDay(at(21, 22, 30), day, night)?.getDate()).toBe(21);
  });

  it("the look back counts the whole night, once", () => {
    // Monday (no night before it): from midnight to Tuesday 06:00, when its night ends.
    expect(lookbackSpan(new Date(2026, 10, 16, 12), night)).toEqual({ start: at(16, 0), end: at(17, 6) });
    // Tuesday: from where Monday's night ended.
    expect(lookbackSpan(new Date(2026, 10, 17, 12), night)).toEqual({ start: at(17, 6), end: at(18, 6) });
    // Saturday (not a work day): from Friday's night's end to its own midnight.
    expect(lookbackSpan(new Date(2026, 10, 21, 12), night)).toEqual({ start: at(21, 6), end: at(22, 0) });
    // Day hours: the calendar day.
    expect(lookbackSpan(new Date(2026, 10, 18, 12), work)).toEqual({ start: new Date(2026, 10, 18).getTime(), end: new Date(2026, 10, 19).getTime() });
  });

  it("opened in the night, the look back is the night's own day", () => {
    // Tuesday 03:00, in Monday's night: Monday (what was ticked since 22:00 is in it).
    expect(spanDay(at(17, 3), night).getDate()).toBe(16);
    const span = lookbackSpan(spanDay(at(17, 3), night), night);
    expect(span.start <= at(16, 22) && at(17, 3) < span.end).toBe(true);
    // From 06:00, Tuesday's.
    expect(spanDay(at(17, 6), night).getDate()).toBe(17);
    // Saturday 03:00 (Friday's night): Friday. Sunday 03:00 (no night on Saturday): Sunday.
    expect(spanDay(at(21, 3), night).getDate()).toBe(20);
    expect(spanDay(at(22, 3), night).getDate()).toBe(22);
    // Day hours: always the calendar day.
    expect(spanDay(at(17, 3), work).getDate()).toBe(17);
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

  it("takes back only what a change made, never what changed meanwhile", () => {
    const before = { id: 1, text: "Call", done_at: null as number | null, link: null as { url: string } | null, sort: 2 };
    const edited = { ...before, text: "Call Léa" };
    const keys = changedKeys(before, edited);
    expect(keys).toEqual(["text"]);
    // Meanwhile, with no step of its own: done from its notification, a link's preview arrived.
    const now = { ...edited, done_at: 99, link: { url: "https://a.b" } };
    expect(withFields(now, before, keys)).toEqual({ ...now, text: "Call" });
    expect(withFields(now, edited, keys)).toEqual(now);
    // Values are compared by what they hold (a rule written again unchanged is no change).
    expect(changedKeys({ r: { every: 1 }, a: undefined }, { r: { every: 1 }, a: null })).toEqual([]);
    expect(changedKeys({ r: { every: 1 } }, { r: { every: 2 } })).toEqual(["r"]);
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
  const names = { hourName: (h: number) => String(h), clock: (h: number, m: number) => `${h}:${String(m).padStart(2, "0")}` };
  const at = (h: number, m = 0) => new Date(2026, 10, 24, h, m).getTime();
  it("a short evening on a wide track names its half hours, never over the two ends' names", () => {
    // 20:00 to 22:00 on 680 px, the ends' names 40 px wide.
    const marks = rulerHours(at(20), at(22), 680, { minGap: 32, startRoom: 40, endRoom: 40, ...names });
    expect(marks.filter((m) => m.label).map((m) => m.label)).toEqual(["20:30", "21:00", "21:30"]);
    // Every half hour has its tick, the ends included.
    expect(marks.map((m) => m.at)).toEqual([at(20), at(20, 30), at(21), at(21, 30), at(22)]);
  });
  it("a long day names whole hours, as many as fit", () => {
    const marks = rulerHours(at(9), at(22), 400, { minGap: 32, startRoom: 30, endRoom: 30, ...names });
    const named = marks.filter((m) => m.label);
    expect(named.every((m) => m.major && !m.label!.includes(":"))).toBe(true);
    for (let i = 1; i < named.length; i++) expect((named[i].x - named[i - 1].x) * 400).toBeGreaterThanOrEqual(32);
    expect(named.every((m) => m.x * 400 >= 40 && m.x * 400 <= 360)).toBe(true);
  });
  it("a narrow ruler still names the hours that fit, not only the clock's own steps", () => {
    // 08:30 to 22:00 on 174 px, the ends' names 45 and 65 px wide, names 44 px apart: no
    // even hour fits between them, 14:00 does.
    const marks = rulerHours(at(8, 30), at(22), 174, { minGap: 44, startRoom: 45, endRoom: 65, ...names, nameW: () => 28 });
    expect(marks.filter((m) => m.label).map((m) => m.label)).toEqual(["14"]);
    // Where the clock's own steps name as many, they are kept: 10, 12, 14… on a wider ruler.
    const wide = rulerHours(at(8, 30), at(22), 330, { minGap: 32, startRoom: 30, endRoom: 30, ...names, nameW: () => 14 });
    expect(wide.filter((m) => m.label).every((m) => new Date(m.at).getHours() % 2 === 0)).toBe(true);
    // Every name clear of the one before it, and of the ends, by 6 px.
    for (const [w, gap, s, e] of [[174, 44, 45, 65], [230, 32, 40, 40], [260, 44, 60, 50], [400, 32, 30, 30]]) {
      const named = rulerHours(at(8, 30), at(22), w, { minGap: gap, startRoom: s, endRoom: e, ...names, nameW: () => 20 }).filter((m) => m.label);
      for (let i = 1; i < named.length; i++) expect((named[i].x - named[i - 1].x) * w).toBeGreaterThanOrEqual(gap);
      expect(named.every((m) => m.x * w - 10 >= s + 6 && m.x * w + 10 <= w - e - 6)).toBe(true);
    }
  });
  it("the hours named stay evenly apart, on the smallest of the clock's steps that fits them all", () => {
    const h12 = { hourName: (h: number) => `${h % 12 || 12} ${h < 12 ? "AM" : "PM"}`, clock: (h: number, m: number) => `${h % 12 || 12}:${String(m).padStart(2, "0")} ${h < 12 ? "AM" : "PM"}` };
    const hoursOf = (marks: ReturnType<typeof rulerHours>) => marks.filter((m) => m.label).map((m) => new Date(m.at).getHours());
    const even = (hs: number[]) => hs.length > 1 && hs.every((h, i) => i === 0 || h - hs[i - 1] === hs[1] - hs[0]) && hs.every((h) => h % (hs[1] - hs[0]) === 0);
    // A 12-hour clock, 08:00 to 23:00 on 660 px: 44 px an hour, every hour just too close.
    const twelve = hoursOf(rulerHours(at(8), at(23), 660, { minGap: 44, startRoom: 30, endRoom: 34, ...h12, nameW: (n) => n.length * 6 }));
    expect(twelve).toEqual([10, 12, 14, 16, 18, 20]);
    // A 24-hour clock, 09:15 to 23:00 on 440 px: 32 px an hour.
    const day = hoursOf(rulerHours(at(9, 15), at(23), 440, { minGap: 32, startRoom: 32, endRoom: 14, ...names, nameW: (n) => n.length * 6 }));
    expect(day).toEqual([12, 14, 16, 18, 20, 22]);
    expect(even(twelve) && even(day)).toBe(true);
  });
  it("an end on the hour is named as the hours are; one between, with its minutes", () => {
    expect(endName(at(22), names)).toBe("22");
    expect(endName(at(21, 40), names)).toBe("21:40");
  });
  it("now's place keeps its own name clear", () => {
    // Now at 20:25 on a strip from 20:00: its name (40 px) starts at its line, about 142 px in,
    // and 20:30 (170 px) would touch it.
    const marks = rulerHours(at(20), at(22), 680, { minGap: 32, startRoom: (25 / 120) * 680 + 40, endRoom: 40, ...names });
    expect(marks.filter((m) => m.label).map((m) => m.label)).toEqual(["21:00", "21:30"]);
  });
  it("the sun by day, the moon in the evening and the night", () => {
    expect([5, 6, 12, 17, 18, 22].map((h) => daylight(at(h)))).toEqual([false, true, true, true, false, false]);
  });
});
