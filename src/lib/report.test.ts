// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { afterEach, describe, expect, it } from "vitest";
import { i18n } from "./i18n.svelte";
import { details, hasWords, mailPlan, plainTime, reportText, sendFailText, sentProblems, subject, zoneOf, type ReportInfo } from "./report";

const info = (n = 2): ReportInfo => {
  const problems = [
    { time: "2026-11-22T10:00:00+03:00", area: "crash" as const, message: "index out of range (src\\db.rs:120)", version: "1.1.1" },
    { time: "2026-11-14T08:05:00+03:00", area: "startup" as const, message: "database is locked", version: "1.1.0" },
  ].slice(0, n);
  return { version: "1.1.1", installed: true, windows: "Windows 11 Pro 25H2 (build 26200)", problems, total: problems.length };
};

afterEach(() => {
  i18n.choice = "en";
});

describe("the report's words", () => {
  it("say everything Send report sends, the time zone only with problems", () => {
    const text = details({ info: info(), dark: true, include: true });
    expect(text.split("\r\n")).toEqual([
      "Dobo 1.1.1 (Setup)",
      "Windows 11 Pro 25H2 (build 26200)",
      "Language: English",
      "Look: Dark",
      "Time zone: UTC+03:00",
      "",
      "Recent problems (newest first, 2 of 2):",
      "2026-11-22 10:00  Stopped: index out of range (src\\db.rs:120)",
      "2026-11-14 08:05  Couldn’t start: database is locked (Dobo 1.1.0)",
    ]);
    const off = details({ info: info(), dark: false, include: false });
    expect(off).toContain("Recent problems: not included.");
    expect(off).not.toContain("Time zone");
    expect(details({ info: info(0), dark: false, include: true })).toContain("Recent problems: none noted.");
    expect(sentProblems({ info: info(), dark: false, include: false })).toEqual([]);
  });

  it("are in Dobo's language, with French typography", () => {
    i18n.choice = "fr";
    const text = reportText("  Le tableau est vide.  ", { info: info(1), dark: false, include: true });
    expect(text).toContain("Ce qui s’est passé :\r\nLe tableau est vide.\r\n");
    expect(text).toContain("Dobo 1.1.1 (installée)");
    expect(text).toContain("Langue : Français");
    expect(text).toContain("Apparence : Claire");
    expect(text).toContain("Arrêt : index out of range");
    expect(subject("1.1.1")).toBe("Dobo 1.1.1 : signalement d’un problème");
  });

  it("give a time in another offset its own zone", () => {
    const i = info();
    i.problems[1] = { ...i.problems[1], time: "2026-07-14T08:05:00+02:00" };
    const text = details({ info: i, dark: false, include: true });
    expect(text).toContain("Time zone: UTC+03:00");
    expect(text).toContain("2026-07-14 08:05 (UTC+02:00)  Couldn’t start");
    expect(text).toContain("2026-11-22 10:00  Stopped");
  });

  it("count only words that show", () => {
    expect(hasWords("  ​﻿⁠ ")).toBe(false);
    expect(hasWords(" x ")).toBe(true);
  });

  it("read times and zones as the PC had them", () => {
    expect(plainTime("2026-11-22T10:00:00+03:00")).toBe("2026-11-22 10:00");
    expect(zoneOf("2026-11-22T10:00:00-05:30")).toBe("UTC-05:30");
    expect(zoneOf("2026-11-22T10:00:00Z")).toBe("UTC");
    expect(zoneOf("2026-11-22T10:00:00")).toBe("");
  });
});

describe("Send by email", () => {
  it("fills in the whole report when it fits, to support only", () => {
    const plan = mailPlan("It froze.", { info: info(), dark: false, include: true });
    expect(plan.clipboard).toBeNull();
    expect(plan.url.startsWith("mailto:support@sookera.com?subject=Dobo%201.1.1%3A%20problem%20report&body=")).toBe(true);
    expect(decodeURIComponent(plan.url.split("&body=")[1])).toBe(reportText("It froze.", { info: info(), dark: false, include: true }));
  });

  it("puts what doesn't fit on the clipboard, and says where to paste it", () => {
    const parts = { info: info(), dark: false, include: true };
    const mid = mailPlan("x".repeat(1500), parts);
    expect(mid.clipboard).toBe(details(parts));
    expect(decodeURIComponent(mid.url)).toContain("The details are on your clipboard: paste them here.");
    const long = mailPlan("é".repeat(3000), parts);
    expect(long.clipboard).toBe(reportText("é".repeat(3000), parts));
    expect(long.url.length).toBeLessThanOrEqual(2000);
  });
});

describe("when Send report doesn't go", () => {
  it("says why and what to do instead", () => {
    expect(sendFailText({ kind: "offline" })).toMatch(/Check your connection/);
    expect(sendFailText({ kind: "busy", retryAfter: 1500 })).toMatch(/in 25 minutes/);
    expect(sendFailText({ kind: "busy", retryAfter: 30 })).toMatch(/in 1 minute,/);
    expect(sendFailText({ kind: "refused", message: "Too long." })).toBe("Too long.");
    expect(sendFailText({ kind: "server" })).toMatch(/isn’t answering/);
    expect(sendFailText("boom")).toMatch(/Send by email instead/);
  });
});
