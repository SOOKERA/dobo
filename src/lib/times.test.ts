// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Times as people type them (English and French), and the times the board offers.
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { findDate, parseCapture } from "./capture";
import { dayBounds, laterChoices, lookbackTomorrow } from "./plan";
import { firstOccurrence, nextOccurrence } from "./repeat";
import { dayWord, formatDue, formatDueLong } from "./time";
import { clockTime, dateAndTime, hourMark, i18n, t } from "./i18n.svelte";

// Wednesday 18 Nov 2026, 14:30.
const ref = new Date(2026, 10, 18, 14, 30);
const parse = (s: string) => parseCapture(s, "task", ref);

describe("typed repeats", () => {
  it("every Mon and Thu, every Thu, Oxford commas", () => {
    const p = parse("gym every Mon and Thu 7pm");
    expect(p.text).toBe("gym");
    expect(p.repeat).toMatchObject({ freq: "week", days: [1, 4] });
    expect(p.due?.getDay()).toBe(4);
    expect(p.due?.getHours()).toBe(19);
    expect(parse("call every Thu").repeat?.days).toEqual([4]);
    expect(parse("review every Tue, Thurs").repeat?.days).toEqual([2, 4]);
    expect(parse("run every Mon, Wed, and Fri").repeat?.days).toEqual([1, 3, 5]);
    expect(parse("run every Mon, Wed, and Fri").text).toBe("run");
    expect(parse("sport tous les lundis, mercredis et vendredis").repeat?.days).toEqual([1, 3, 5]);
  });

  it("every other week / every 2nd week is every 2 weeks, never the 2nd of the month", () => {
    expect(parse("payroll every other week").repeat).toMatchObject({ freq: "week", interval: 2 });
    expect(parse("payroll every 2nd week").repeat).toMatchObject({ freq: "week", interval: 2 });
    expect(parse("water every other day").repeat).toMatchObject({ freq: "day", interval: 2 });
    expect(parse("rent every 2nd").repeat).toMatchObject({ freq: "month", monthday: 2 });
  });
});

describe("typed times", () => {
  const at = (s: string, now = ref) => {
    const d = parseCapture(s, "task", now).due;
    return d ? `${d.getDate()}/${d.getMonth() + 1} ${d.getHours()}:${String(d.getMinutes()).padStart(2, "0")}` : null;
  };

  it("a part of the day keeps its hour (never 09:00 this morning, already past)", () => {
    expect(at("Call mom tonight")).toBe("18/11 22:00");
    expect(at("ce soir")).toBe("18/11 18:00");
    expect(at("this evening")).toBe("18/11 20:00");
    expect(at("demain soir")).toBe("19/11 18:00");
    expect(at("tomorrow afternoon")).toBe("19/11 15:00");
    expect(at("Thursday night")).toBe("19/11 20:00");
    // Today's, even once its hour has passed: the next full hour, never tomorrow.
    expect(at("cet après-midi")).toBe("18/11 15:00");
    expect(at("ce soir", new Date(2026, 10, 18, 19, 10))).toBe("18/11 20:00");
    // A morning, or a day alone, is still 09:00.
    expect(at("demain matin")).toBe("19/11 9:00");
    expect(at("tomorrow")).toBe("19/11 9:00");
    // The Remind? offer while editing reads them the same way.
    expect(findDate("Call mom tonight", ref)?.due.getHours()).toBe(22);
  });

  it("a bare \"at 3\" is the afternoon; with minutes, \"h\" or am/pm it is as written", () => {
    expect(at("call at 3")).toBe("18/11 15:00");
    expect(at("Call Sam at 5")).toBe("18/11 17:00");
    expect(at("rdv à 4")).toBe("18/11 16:00");
    expect(at("tomorrow at 3")).toBe("19/11 15:00");
    expect(at("friday at 4")).toBe("20/11 16:00");
    // Past today's: tomorrow afternoon.
    expect(at("call at 3", new Date(2026, 10, 18, 16, 0))).toBe("19/11 15:00");
    expect(at("at 7")).toBe("19/11 7:00");
    expect(at("at 10")).toBe("19/11 10:00");
    expect(at("at 7:30")).toBe("19/11 7:30");
    expect(at("à 3h")).toBe("19/11 3:00");
    expect(at("at 3am")).toBe("19/11 3:00");
    expect(at("at 3pm")).toBe("18/11 15:00");
    expect(at("Call Chanel 15h")).toBe("18/11 15:00");
  });
});

describe("French words chrono doesn't know, and what is left of the line", () => {
  const at = (s: string) => {
    const d = parse(s).due;
    return d ? `${d.getDate()}/${d.getMonth() + 1} ${d.getHours()}:${String(d.getMinutes()).padStart(2, "0")}` : null;
  };

  it("après-demain is in two days, not tomorrow", () => {
    expect(at("Payer après-demain 15h")).toBe("20/11 15:00");
    expect(parse("Payer après-demain 15h").text).toBe("Payer");
    expect(at("apres demain")).toBe("20/11 9:00");
    expect(parse("Appeler après demain").text).toBe("Appeler");
  });

  it("midi and minuit are times; après-midi stays the afternoon", () => {
    expect(at("demain midi")).toBe("19/11 12:00");
    expect(parse("Déjeuner à midi").text).toBe("Déjeuner");
    expect(at("Déjeuner à midi")).toBe("19/11 12:00");
    expect(at("minuit")).toBe("19/11 0:00");
    expect(at("demain après-midi")).toBe("19/11 14:00");
  });

  it("a joining word left alone isn't the task's words", () => {
    expect(parse("le 3 décembre").text).toBe("");
    expect(parse("Rdv le 3 décembre").text).toBe("Rdv");
    expect(parse("at midnight").text).toBe("");
    expect(parse("Call Chanel à 15h").text).toBe("Call Chanel");
  });
});

describe("Later… choices", () => {
  const day = { start: "08:00", end: "22:00" };
  it("still up after midnight, Tomorrow morning is this morning, never a day later", () => {
    const night = new Date(2026, 10, 19, 1, 0).getTime();
    const c = laterChoices(night, day);
    const morning = c.find((x) => x.label === "Tomorrow morning")!;
    expect(new Date(morning.at)).toEqual(new Date(2026, 10, 19, 8, 0));
    // In the order they come: 02:00, 08:00, 19:00, Monday.
    expect(c.map((x) => x.label)).toEqual(["1 hour", "Tomorrow morning", "This evening", "Next Monday"]);
    // Up early (06:30), or in the day: the next day's.
    expect(new Date(laterChoices(new Date(2026, 10, 19, 6, 30).getTime(), day).find((x) => x.label === "Tomorrow morning")!.at).getDate()).toBe(20);
    expect(new Date(laterChoices(new Date(2026, 10, 19, 23, 30).getTime(), day).find((x) => x.label === "Tomorrow morning")!.at).getDate()).toBe(20);
  });
});

describe("the look back's day", () => {
  it("Tomorrow, opened after midnight, is today, not the day after", () => {
    const wed = new Date(2026, 10, 18);
    const due = new Date(2026, 10, 18, 9, 30).getTime();
    // In the evening: Thursday 09:30.
    expect(new Date(lookbackTomorrow(due, wed, new Date(2026, 10, 18, 22, 10).getTime()))).toEqual(new Date(2026, 10, 19, 9, 30));
    // At 01:00 on Thursday, still looking back on Wednesday: Thursday 09:30 too.
    expect(new Date(lookbackTomorrow(due, wed, new Date(2026, 10, 19, 1, 0).getTime()))).toEqual(new Date(2026, 10, 19, 9, 30));
    // A time already past by then: the next day's.
    const late = new Date(2026, 10, 18, 0, 30).getTime();
    expect(new Date(lookbackTomorrow(late, wed, new Date(2026, 10, 19, 1, 0).getTime()))).toEqual(new Date(2026, 10, 20, 0, 30));
  });
});

// (Node's own: changing TZ changes the zone the dates below are made in.)
declare const process: { env: Record<string, string | undefined> };

// The nights the clocks change (Europe: last Sunday of March / October; US: 2nd Sunday of March,
// 1st of November), whatever this computer's own zone.
describe("days the clocks change", () => {
  let tz: string | undefined;
  beforeAll(() => {
    tz = process.env.TZ;
  });
  afterAll(() => {
    if (tz == null) delete process.env.TZ;
    else process.env.TZ = tz;
  });
  const zones = [
    ["Europe/Paris", [2026, 2, 29], [2026, 9, 25]],
    ["America/New_York", [2026, 2, 8], [2026, 10, 1]],
  ] as const;

  it("a day is 23 or 25 hours, and the look back takes it whole", () => {
    for (const [zone, spring, autumn] of zones) {
      process.env.TZ = zone;
      const s = dayBounds(new Date(spring[0], spring[1], spring[2], 12));
      expect((s.end - s.start) / 3_600_000).toBe(23);
      const a = dayBounds(new Date(autumn[0], autumn[1], autumn[2], 12));
      expect((a.end - a.start) / 3_600_000).toBe(25);
      // The last hour of the long day is still that day.
      expect(new Date(a.end - 1).getDate()).toBe(autumn[2]);
    }
  });

  it("Tomorrow keeps the time of day across the change", () => {
    for (const [zone, spring, autumn] of zones) {
      process.env.TZ = zone;
      for (const d of [spring, autumn]) {
        const before = new Date(d[0], d[1], d[2] - 1, 9, 0);
        const t = new Date(lookbackTomorrow(before.getTime(), before, before.getTime()));
        expect([t.getDate(), t.getHours(), t.getMinutes()]).toEqual([d[2], 9, 0]);
      }
    }
  });

  it("Later… and the repeats keep the time of day across the change", () => {
    for (const [zone, spring] of zones) {
      process.env.TZ = zone;
      const sat = new Date(spring[0], spring[1], spring[2] - 1, 23, 30).getTime();
      const morning = new Date(laterChoices(sat, { start: "08:00", end: "22:00" }).find((x) => x.label === "Tomorrow morning")!.at);
      expect([morning.getDate(), morning.getHours()]).toEqual([spring[2], 8]);
      const daily = nextOccurrence({ freq: "day", interval: 1, ends: { kind: "never" } }, new Date(spring[0], spring[1], spring[2] - 1, 9, 0).getTime())!;
      expect([new Date(daily).getDate(), new Date(daily).getHours()]).toEqual([spring[2], 9]);
      // 02:30 doesn't exist that night: one hour later, as the Rust side does.
      const skipped = firstOccurrence({ freq: "day", interval: 1, ends: { kind: "never" } }, 2, 30, new Date(spring[0], spring[1], spring[2], 0, 10));
      expect([skipped.getDate(), skipped.getHours(), skipped.getMinutes()]).toEqual([spring[2], 3, 30]);
    }
  });
});

describe("a time on another day says which", () => {
  it("yesterday, tomorrow, a weekday, a date; nothing for today", () => {
    const now = new Date(2026, 10, 18, 9, 0);
    i18n.choice = "en";
    expect(dayWord(new Date(2026, 10, 18, 22, 0).getTime(), now)).toBeNull();
    expect(dayWord(new Date(2026, 10, 17, 22, 0).getTime(), now)).toBe("yesterday");
    expect(dayWord(new Date(2026, 10, 19, 9, 0).getTime(), now)).toBe("tomorrow");
    expect(dayWord(new Date(2026, 10, 21, 9, 0).getTime(), now)).toBe("Saturday");
    expect(dayWord(new Date(2026, 11, 3, 9, 0).getTime(), now)).toBe("on 3 Dec");
    expect(t("Was due {day} at {time} · {where}", { day: "yesterday", time: "22:00", where: "Hub" })).toBe("Was due yesterday at 22:00 · Hub");
    i18n.choice = "fr";
    expect(dayWord(new Date(2026, 10, 17, 22, 0).getTime(), now)).toBe("hier");
    expect(t("Was due {day} at {time} · {where}", { day: "hier", time: "22:00", where: "Hub" })).toBe("Prévu hier à 22:00 · Hub");
    // In a sentence: the weekday in full, « le » before a date.
    expect(dayWord(new Date(2026, 10, 16, 22, 0).getTime(), now)).toBe("lundi");
    expect(dayWord(new Date(2026, 11, 3, 9, 0).getTime(), now)).toBe("le 3 déc.");
    i18n.choice = "en";
  });
});

describe("a date far away says its year", () => {
  it("24 Nov next year is not this year's", () => {
    i18n.choice = "en";
    const now = new Date(2026, 10, 18, 9, 0);
    expect(formatDue(new Date(2026, 11, 3, 9, 0).getTime(), now)).toBe("3 Dec 09:00");
    expect(formatDue(new Date(2027, 10, 24, 9, 0).getTime(), now)).toBe("24 Nov 2027 09:00");
    expect(formatDue(new Date(2025, 10, 24, 9, 0).getTime(), now)).toBe("24 Nov 2025 09:00");
    expect(dayWord(new Date(2027, 10, 24, 9, 0).getTime(), now)).toBe("on 24 Nov 2027");
  });
});

describe("12 or 24 hours, as Windows shows the time", () => {
  afterAll(() => {
    i18n.hour12 = false;
    i18n.choice = "system";
  });
  // (Made in each test: the tests before move the time zone.)
  let now = new Date();
  let late = 0;
  let morning = 0;
  beforeEach(() => {
    now = new Date(2026, 10, 18, 14, 30);
    late = new Date(2026, 10, 18, 22, 0).getTime();
    morning = new Date(2026, 10, 19, 9, 5).getTime();
  });

  it("24-hour by default, in both languages", () => {
    i18n.hour12 = false;
    i18n.choice = "en";
    expect(formatDue(late, now)).toBe("22:00");
    expect(clockTime(9, 5)).toBe("09:05");
    // The ruler's hours: bare in English, « 14h » in French.
    expect(hourMark(14)).toBe("14");
    i18n.choice = "fr";
    expect(hourMark(14)).toBe("14h");
    expect(hourMark(0)).toBe("0h");
    i18n.choice = "en";
  });

  it("a 12-hour Windows: every time shown, the language staying the app's", () => {
    i18n.hour12 = true;
    i18n.choice = "en";
    expect(formatDue(late, now)).toBe("10:00 PM");
    expect(formatDueLong(morning, now)).toBe("Tomorrow 9:05 AM");
    expect(clockTime(0, 30)).toBe("12:30 AM");
    expect(clockTime(12, 0)).toBe("12:00 PM");
    expect(hourMark(14)).toBe("2 PM");
    expect(dateAndTime(new Date(morning))).toBe("Thu 19 Nov, 9:05 AM");
    i18n.choice = "fr";
    expect(formatDueLong(morning, now)).toBe("Demain 9:05 AM");
    expect(dateAndTime(new Date(morning), true)).toBe("jeudi 19 novembre à 9:05 AM");
  });

  it("typed times read the same whichever the clock", () => {
    for (const h12 of [false, true]) {
      i18n.hour12 = h12;
      expect(parse("call 15h")?.due?.getHours()).toBe(15);
      expect(parse("call 3pm")?.due?.getHours()).toBe(15);
      expect(parse("call tomorrow 9am")?.due?.getDate()).toBe(19);
      expect(parse("appel demain 9h")?.due?.getHours()).toBe(9);
    }
  });
});
