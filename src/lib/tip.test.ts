// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { placeTip, splitKeys, tipOf } from "./tip";

const view = { width: 600, height: 400 };
const tip = { width: 120, height: 26 };

describe("tooltip", () => {
  it("sits centred under the control", () => {
    expect(placeTip({ left: 240, top: 20, width: 40, height: 30 }, tip, view)).toEqual({ x: 200, y: 58, below: true });
  });
  it("stays inside the window at the edges", () => {
    expect(placeTip({ left: 570, top: 20, width: 30, height: 30 }, tip, view).x).toBe(600 - 8 - 120);
    expect(placeTip({ left: 0, top: 20, width: 30, height: 30 }, tip, view).x).toBe(8);
  });
  it("goes above when there is no room below", () => {
    const p = placeTip({ left: 240, top: 360, width: 40, height: 30 }, tip, view);
    expect(p.below).toBe(false);
    expect(p.y).toBe(360 - 8 - 26);
  });
  it("a tooltip wider than the window starts at the margin", () => {
    expect(placeTip({ left: 100, top: 20, width: 30, height: 30 }, { width: 700, height: 26 }, view).x).toBe(8);
  });
  it("can sit on a control's left, centred on it, inside the window", () => {
    expect(placeTip({ left: 300, top: 20, width: 40, height: 40 }, tip, view, 8, 8, "left")).toEqual({ x: 172, y: 27, below: true });
    expect(placeTip({ left: 60, top: 0, width: 40, height: 10 }, tip, view, 8, 8, "left")).toEqual({ x: 8, y: 8, below: true });
  });
  it("a line's whole words only while it is cut short (data-tip-cut)", () => {
    const line = (scrollWidth: number, clientWidth: number) => {
      const attrs: Record<string, string> = { "data-tip": "Changed from Paper · Dark · your picture", "data-tip-cut": "" };
      return { scrollWidth, clientWidth, textContent: "Changed from…", getAttribute: (k: string) => attrs[k] ?? null, hasAttribute: (k: string) => k in attrs } as unknown as Element;
    };
    expect(tipOf(line(300, 220))?.label).toBe("Changed from Paper · Dark · your picture");
    expect(tipOf(line(220, 220))).toBeNull();
  });
  it("splits shortcuts into key chips, with alternatives", () => {
    expect(splitKeys("Ctrl+Shift+S")).toEqual([["Ctrl", "Shift", "S"]]);
    expect(splitKeys("Ctrl+F|/")).toEqual([["Ctrl", "F"], ["/"]]);
    expect(splitKeys("Ctrl++|Ctrl+,")).toEqual([["Ctrl", "+"], ["Ctrl", ","]]);
    expect(splitKeys("")).toEqual([]);
    expect(splitKeys(null)).toEqual([]);
  });
});
