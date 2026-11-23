// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { fitDay, keyDate, pausedYear, typeDateDigit, type DSeg, type Years, type YMD } from "./datefield";

const YEARS: Years = [2026, 2076];

/** Digits typed one after another into a part, as the field does (a done part starts again). */
function type(seg: DSeg, digits: string, start: YMD) {
  let date = start;
  let buf = "";
  let done = false;
  for (const c of digits) {
    const r = typeDateDigit(seg, buf, Number(c), date, YEARS);
    date = r.date;
    buf = r.done ? "" : r.buf;
    done = r.done;
  }
  return { date, done, buf };
}

describe("the date field", () => {
  it("arrows, pages, Home and End", () => {
    const k = (seg: DSeg, key: string, d: YMD) => keyDate(seg, key, d, YEARS);
    // The day goes round its own month.
    expect(k("d", "ArrowUp", [2026, 10, 30])).toEqual([2026, 10, 1]);
    expect(k("d", "ArrowDown", [2026, 1, 1])).toEqual([2026, 1, 28]);
    expect(k("d", "PageUp", [2026, 10, 27])).toEqual([2026, 10, 4]);
    expect(k("d", "End", [2028, 1, 3])).toEqual([2028, 1, 29]);
    expect(k("d", "Home", [2026, 10, 19])).toEqual([2026, 10, 1]);
    // The month goes round the year; the day stays inside the month.
    expect(k("mo", "ArrowUp", [2026, 11, 5])).toEqual([2026, 0, 5]);
    expect(k("mo", "ArrowUp", [2026, 0, 31])).toEqual([2026, 1, 28]);
    expect(k("mo", "PageDown", [2026, 1, 10])).toEqual([2026, 10, 10]);
    expect(k("mo", "End", [2026, 3, 30])).toEqual([2026, 11, 30]);
    // The year stops at its ends.
    expect(k("y", "ArrowDown", [2026, 5, 1])).toEqual([2026, 5, 1]);
    expect(k("y", "ArrowUp", [2028, 1, 29])).toEqual([2029, 1, 28]);
    expect(k("y", "PageUp", [2074, 0, 1])).toEqual([2076, 0, 1]);
    expect(k("y", "End", [2026, 0, 1])).toEqual([2076, 0, 1]);
    expect(k("d", "x", [2026, 0, 1])).toBeNull();
    expect(k("mo", "Backspace", [2026, 0, 1])).toBeNull();
  });

  it("types the day, the month and the year", () => {
    const at: YMD = [2026, 10, 22];
    expect(type("d", "4", at)).toMatchObject({ date: [2026, 10, 4], done: true });
    expect(type("d", "1", at)).toMatchObject({ date: [2026, 10, 1], done: false });
    expect(type("d", "19", at)).toMatchObject({ date: [2026, 10, 19], done: true });
    expect(type("d", "07", at)).toMatchObject({ date: [2026, 10, 7], done: true });
    // A day the month hasn't: kept as typed (the month may come next), fitted when saved.
    expect(type("d", "31", at)).toMatchObject({ date: [2026, 10, 31], done: true });
    expect(fitDay(type("d", "31", at).date)).toEqual([2026, 10, 30]);
    expect(type("d", "30", [2026, 1, 10])).toMatchObject({ date: [2026, 1, 30], done: true });
    expect(type("d", "3", [2026, 1, 10])).toMatchObject({ date: [2026, 1, 3], done: false });
    expect(type("d", "39", at)).toMatchObject({ date: [2026, 10, 9], done: true });
    // "31 12 2027": the 31st of December, not the 1st.
    const dec = type("mo", "12", type("d", "31", at).date).date;
    expect(type("y", "2027", dec)).toMatchObject({ date: [2027, 11, 31], done: true });
    // The month or the year changed: the day kept inside it, never the 1st.
    expect(type("mo", "2", [2026, 10, 31])).toMatchObject({ date: [2026, 1, 28], done: true });
    expect(keyDate("d", "ArrowUp", [2026, 10, 31], YEARS)).toEqual([2026, 10, 1]);
    expect(keyDate("d", "ArrowDown", [2026, 10, 31], YEARS)).toEqual([2026, 10, 29]);
    expect(type("mo", "3", at)).toMatchObject({ date: [2026, 2, 22], done: true });
    expect(type("mo", "1", at)).toMatchObject({ date: [2026, 0, 22], done: false });
    expect(type("mo", "12", at)).toMatchObject({ date: [2026, 11, 22], done: true });
    expect(type("mo", "2", [2026, 0, 31])).toMatchObject({ date: [2026, 1, 28], done: true });
    // The year: its four digits, the date unchanged until then; outside the field's years, its end.
    expect(type("y", "202", at)).toMatchObject({ date: at, done: false, buf: "202" });
    expect(type("y", "2027", at)).toMatchObject({ date: [2027, 10, 22], done: true });
    expect(type("y", "1999", at)).toMatchObject({ date: [2026, 10, 22], done: true });
    expect(type("y", "2028", [2026, 1, 28])).toMatchObject({ date: [2028, 1, 28], done: true });
  });

  it("a year typed in part, when the typing pauses", () => {
    expect(pausedYear("27", YEARS)).toBe(2027);
    expect(pausedYear("2", YEARS)).toBeNull();
    expect(pausedYear("202", YEARS)).toBeNull();
    expect(pausedYear("99", YEARS)).toBe(2076);
  });

  it("keeps the day inside its month", () => {
    expect(fitDay([2026, 3, 31])).toEqual([2026, 3, 30]);
    expect(fitDay([2028, 1, 30])).toEqual([2028, 1, 29]);
  });
});
