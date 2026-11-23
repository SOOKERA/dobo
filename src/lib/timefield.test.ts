// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { keyTime, typeDigit, type HM, type Seg } from "./timefield";

/** Digits typed one after another into a part, as the field does (a done part moves on). */
function type(seg: "h" | "m", digits: string, start: HM, twelve = false) {
  let time = start;
  let buf = "";
  let done = false;
  for (const c of digits) {
    const r = typeDigit(seg, buf, Number(c), time, twelve);
    time = r.time;
    buf = r.done ? "" : r.buf;
    done = r.done;
  }
  return { time, done };
}

describe("the time field", () => {
  it("arrows, pages, Home and End", () => {
    const k = (seg: Seg, key: string, t: HM, twelve = false) => keyTime(seg, key, t, twelve);
    expect(k("h", "ArrowUp", [23, 30])).toEqual([0, 30]);
    expect(k("h", "ArrowDown", [0, 30])).toEqual([23, 30]);
    expect(k("m", "ArrowUp", [9, 59])).toEqual([9, 0]);
    expect(k("m", "PageUp", [9, 55])).toEqual([9, 5]);
    expect(k("h", "PageDown", [1, 0])).toEqual([22, 0]);
    expect(k("h", "Home", [15, 10])).toEqual([0, 10]);
    expect(k("h", "End", [3, 10])).toEqual([23, 10]);
    expect(k("m", "End", [3, 10])).toEqual([3, 59]);
    // 12-hour: hours run on through noon; Home/End stay in the half of the day.
    expect(k("h", "ArrowUp", [11, 0], true)).toEqual([12, 0]);
    expect(k("h", "Home", [15, 0], true)).toEqual([12, 0]);
    expect(k("h", "End", [3, 0], true)).toEqual([11, 0]);
    expect(k("p", "ArrowUp", [9, 0], true)).toEqual([21, 0]);
    expect(k("p", "a", [21, 0], true)).toEqual([9, 0]);
    expect(k("p", "P", [9, 0], true)).toEqual([21, 0]);
    expect(k("p", " ", [21, 0], true)).toEqual([9, 0]);
    expect(k("m", "Backspace", [9, 45])).toEqual([9, 0]);
    expect(k("h", "x", [9, 45])).toBeNull();
    expect(k("p", "a", [9, 45])).toEqual([9, 45]);
  });

  it("digits, 24-hour", () => {
    expect(type("h", "9", [0, 0])).toEqual({ time: [9, 0], done: true });
    expect(type("h", "1", [0, 0])).toEqual({ time: [1, 0], done: false });
    expect(type("h", "15", [0, 0])).toEqual({ time: [15, 0], done: true });
    expect(type("h", "07", [0, 0])).toEqual({ time: [7, 0], done: true });
    expect(type("h", "29", [0, 0]).time).toEqual([9, 0]);
    expect(type("m", "3", [9, 0])).toEqual({ time: [9, 3], done: false });
    expect(type("m", "30", [9, 0])).toEqual({ time: [9, 30], done: true });
    expect(type("m", "7", [9, 0])).toEqual({ time: [9, 7], done: true });
  });

  it("digits, 12-hour, keeping AM or PM", () => {
    expect(type("h", "3", [14, 0], true)).toEqual({ time: [15, 0], done: true });
    expect(type("h", "12", [14, 0], true)).toEqual({ time: [12, 0], done: true });
    expect(type("h", "12", [2, 0], true)).toEqual({ time: [0, 0], done: true });
    expect(type("h", "1", [2, 0], true)).toEqual({ time: [1, 0], done: false });
    expect(type("h", "10", [2, 0], true)).toEqual({ time: [10, 0], done: true });
    expect(type("h", "0", [2, 0], true)).toEqual({ time: [2, 0], done: false });
    expect(type("h", "09", [20, 0], true)).toEqual({ time: [21, 0], done: true });
  });
});
