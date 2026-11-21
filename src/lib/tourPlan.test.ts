// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { DIP_OUT, STEPS, clip, cornerPlan, cornerWay, covers, fadePlan, fadeWay, laterWords, placeBubble, straightPlan, straightWay, union, type Pose } from "./tourPlan";
import type { Box4 } from "./motion";
import { parseCapture } from "./capture";

describe("the quick tour's plan", () => {
  it("tells one story, under a minute: dump, Inbox, tidy, a time, the hours, a step, Style, then the ?", () => {
    expect(STEPS).toEqual(["dump", "inbox", "tidy", "time", "ruler", "step", "style", "more"]);
  });

  it("puts the bubble under what it points at, else above, else beside, never off the panel", () => {
    // Under a field at the top of a 720 × 836 panel.
    expect(placeBubble({ x: 18, y: 100, w: 684, h: 60 }, 340, 150, 720, 836, "below")).toEqual({ x: 190, y: 172 });
    // Over the "?" at the bottom right corner: above, kept 16 px from the right edge.
    const p = placeBubble({ x: 660, y: 780, w: 40, h: 40 }, 340, 240, 720, 836, "above");
    expect(p).toEqual({ x: 364, y: 528 });
    // No room under: above.
    expect(placeBubble({ x: 100, y: 700, w: 100, h: 40 }, 200, 150, 720, 836, "below").y).toBe(538);
    // A tall thing filling the height in a wide panel: beside it.
    expect(placeBubble({ x: 20, y: 20, w: 300, h: 800 }, 300, 200, 720, 836, "below")).toEqual({ x: 332, y: 20 });
    // Nothing to point at: in the middle.
    expect(placeBubble(null, 300, 200, 720, 836, "center").x).toBe(210);
    // A narrow panel: the bubble keeps at least 16 px on each side.
    const n = placeBubble({ x: 300, y: 10, w: 40, h: 40 }, 340, 150, 380, 760, "below");
    expect(n.x).toBe(24);
    // While its words change, the side is chosen for the taller of its two heights: shrinking
    // from 200 to 150 px, it stays above a field low in the panel (no swapping sides on the way),
    // its bottom kept 12 px over it.
    const t = { x: 18, y: 590, w: 684, h: 60 };
    expect(placeBubble(t, 340, 150, 720, 836, "below").y).toBe(662);
    expect(placeBubble(t, 340, 150, 720, 836, "below", 200).y).toBe(590 - 12 - 150);
  });

  it("clips and joins boxes", () => {
    expect(clip({ x: 0, y: 0, w: 100, h: 100 }, { x: 50, y: 50, w: 100, h: 100 })).toEqual({ x: 50, y: 50, w: 50, h: 50 });
    expect(clip({ x: 0, y: 0, w: 10, h: 10 }, { x: 50, y: 50, w: 10, h: 10 })).toBeNull();
    expect(union({ x: 0, y: 0, w: 10, h: 10 }, { x: 50, y: 50, w: 10, h: 10 })).toEqual({ x: 0, y: 0, w: 60, h: 60 });
  });

  it("writes a time a little later the way people type it", () => {
    expect(laterWords(new Date(2026, 10, 20, 12, 15), "en")).toBe("2pm");
    expect(laterWords(new Date(2026, 10, 20, 12, 15), "fr")).toBe("14h");
    expect(laterWords(new Date(2026, 10, 20, 8, 0), "en")).toBe("10am");
    expect(laterWords(new Date(2026, 10, 20, 22, 30), "fr")).toBe("demain 9h");
  });

  it("the entry line's demo reads as a deadline today, in both languages (no name that is a day)", () => {
    const now = new Date(2026, 10, 20, 12, 42);
    for (const [line, lang] of [["Call Lea {when}!", "en"], ["Appeler Léa {when}!", "fr"]] as const) {
      const p = parseCapture(line.replace("{when}", laterWords(now, lang)), "task", now);
      expect(p.deadline).toBe(true);
      expect(p.due?.getDate()).toBe(20);
      expect(p.due?.getHours()).toBe(14);
    }
  });
});

describe("the way from the Style button down to the ?", () => {
  // As the panel lays them out (docked, 836 px high): the Style button (its words, its brush, or
  // the ⋯ it is tucked into at 380 px) and the "?" at the board's corner, each lit 4 px around.
  const H = 836;
  const lit = (x: number, y: number, w: number, h: number): Box4 => ({ x: x - 4, y: y - 4, w: w + 8, h: h + 8 });
  const panels: [number, Box4][] = [
    [356, lit(231, 12, 34, 40)],
    [380, lit(255, 12, 34, 40)],
    [420, lit(147, 15, 34, 34)],
    [480, lit(207, 15, 34, 34)],
    [560, lit(215, 15, 34, 34)],
    [720, lit(375, 15, 34, 34)],
    [760, lit(399, 13, 40, 34)],
    [880, lit(440, 13, 77, 34)],
  ];
  const poses = (W: number, style: Box4, hs: number, hm: number) => {
    const bw = Math.min(340, W - 32);
    const help = lit(W - 54, 788, 28, 28);
    const at = (b: Box4, bh: number, side: "below" | "above") => placeBubble(b, bw, bh, W, H, side);
    const pose = (b: Box4, bh: number, side: "below" | "above"): Pose => ({ a: b, b, r: 12, ring: 1, two: 0, bx: at(b, bh, side).x, by: at(b, bh, side).y, show: 1 });
    return { bw, s: pose(style, hs, "below"), m: pose(help, hm, "above") };
  };

  it("the bubble never has the window under it, there and back, in English and French", () => {
    for (const [W, style] of panels)
      for (const [hs, hm] of [[150, 200], [190, 250]]) {
        const { bw, s, m } = poses(W, style, hs, hm);
        const way = cornerPlan(s, m, bw, Math.max(hs, hm));
        for (let k = 0; k <= 400; k++) {
          const p = cornerWay(s, m, k / 400, bw, way);
          expect(covers(p, bw, Math.max(hs, hm)), `${W} px, ${way}, at ${k / 400}`).toBe(false);
        }
        // Beside the window's way where there is room; a narrow panel fades it instead.
        expect(way, `${W} px`).toBe(W >= 480 ? "aside" : "dip");
      }
  });

  it("starts and ends where the two steps show things, and goes along the top, then down the side", () => {
    for (const [W, style] of panels) {
      const { bw, s, m } = poses(W, style, 170, 230);
      for (const way of ["aside", "dip"] as const) {
        expect(cornerWay(s, m, 0, bw, way)).toEqual(s);
        expect(cornerWay(s, m, 1, bw, way)).toEqual(m);
      }
      const c = (b: Box4) => [b.x + b.w / 2, b.y + b.h / 2];
      const [sx, sy] = c(s.a);
      const [mx, my] = c(m.a);
      let last = [sx, sy];
      for (let k = 1; k <= 100; k++) {
        const [x, y] = c(cornerWay(s, m, k / 100, bw, "aside").a);
        // Always on towards the corner, never back; past half way down, it is by the side.
        expect(x).toBeGreaterThanOrEqual(last[0] - 1e-6);
        expect(y).toBeGreaterThanOrEqual(last[1] - 1e-6);
        if (y > (sy + my) / 2) expect(x).toBeGreaterThanOrEqual(mx - 0.1 * (mx - sx) - 1e-6);
        last = [x, y];
      }
    }
  });
});

describe("a straight move between two steps", () => {
  const pose = (a: Box4, bx: number, by: number): Pose => ({ a, b: a, r: 12, ring: 1, two: 0, bx, by, show: 1 });
  it("glides, unless the window would pass under the bubble: then the bubble fades out and back in", () => {
    // A task low in a narrow panel, its bubble over it → the ⋯ at the top, the bubble under it.
    const f = pose({ x: 28, y: 533, w: 290, h: 124 }, 16, 374);
    const t = pose({ x: 227, y: 8, w: 42, h: 48 }, 16, 68);
    expect(straightPlan(f, t, 324, 150)).toBe("dip");
    for (let k = 0; k <= 200; k++) expect(covers(straightWay(f, t, k / 200, "dip"), 324, 150), `at ${k / 200}`).toBe(false);
    expect(straightWay(f, t, 0, "dip")).toEqual(f);
    expect(straightWay(f, t, 1, "dip")).toEqual(t);
    // The Inbox's head, its bubble under it → Tidy at the top, its bubble under it: a glide.
    const g = pose({ x: 20, y: 300, w: 330, h: 120 }, 16, 432);
    const h = pose({ x: 300, y: 10, w: 48, h: 48 }, 16, 70);
    expect(straightPlan(g, h, 340, 150)).toBe("glide");
    // A window opening from the bubble's middle (the first step has nothing to point at) glides.
    const dot = { ...pose({ x: 190, y: 380, w: 0, h: 0 }, 20, 300), ring: 0 };
    expect(straightPlan(dot, g, 340, 150)).toBe("glide");
  });

  it("a bubble over its window that grows as it fades out keeps its bottom where it was", () => {
    const f = pose({ x: 19, y: 447, w: 342, h: 127 }, 20, 288);
    const t = pose({ x: 18, y: 102, w: 344, h: 96 }, 20, 210);
    for (let k = 0; k <= 20; k++) {
      const p = (k / 20) * DIP_OUT * 0.99;
      const grown = 21 * (k / 20);
      const at = straightWay(f, t, p, "dip", grown);
      // (Its height then: 147 + grown.)
      if (at.show > 0.02) expect(at.by + 147 + grown).toBeLessThanOrEqual(f.by + 147 + 1e-6);
    }
  });

  it("turned into a dip on the way, the window doesn't stop", () => {
    const f = pose({ x: 115, y: 304, w: 182, h: 84 }, 16, 400);
    const t = pose({ x: 28, y: 533, w: 290, h: 124 }, 16, 374);
    let last = f.a.y;
    for (let k = 1; k <= 33; k++) {
      const y = straightWay(f, t, k / 40, "dip", 0, true).a.y;
      expect(y).toBeGreaterThan(last);
      last = y;
    }
  });
});

describe("with less motion", () => {
  const pose = (a: Box4, bx: number, by: number): Pose => ({ a, b: a, r: 12, ring: 1, two: 0, bx, by, show: 1 });
  const bw = 340;
  const bh = 160;
  /** How much the veil shows over a point, from how lit the two windows are there. */
  const veil = (inOld: boolean, inNew: boolean, old: number, lit: number) => (inOld ? 1 - old : 1) * (inNew ? 1 - lit : 1);
  const inside = (b: Box4, x: number, y: number) => x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h;
  const moves: [string, Pose, Pose][] = [
    // The time step (the entry line, its bubble under it) → the hours (lower, its bubble over them).
    ["entry line → hours", pose({ x: 18, y: 102, w: 344, h: 96 }, 20, 210), pose({ x: 19, y: 447, w: 342, h: 127 }, 20, 275)],
    // The Inbox's head, its bubble under it → the bigger task right under that bubble.
    ["Inbox → a task under its bubble", pose({ x: 20, y: 120, w: 330, h: 120 }, 16, 252), pose({ x: 20, y: 300, w: 330, h: 90 }, 16, 120)],
    // Two windows on the same place, the bubble changing sides.
    ["the same place", pose({ x: 20, y: 300, w: 330, h: 90 }, 16, 402), pose({ x: 20, y: 300, w: 330, h: 90 }, 16, 128)],
    // Overlapping windows, each under the other's bubble.
    ["each under the other's bubble", pose({ x: 20, y: 300, w: 330, h: 200 }, 16, 120), pose({ x: 20, y: 120, w: 330, h: 200 }, 16, 332)],
  ];
  it("no bubble shows over a lit window, and where both windows light the same place it never dims", () => {
    for (const [name, f, t] of moves) {
      const plan = fadePlan(f, t, bw, bh);
      for (let k = 0; k <= 200; k++) {
        const { pose: at, lit, old } = fadeWay(f, t, k / 200, plan);
        // The bubble (where it shows now) over the new window while it is lit, or over the old one.
        if (at.show > 0.02 && lit > 0.02) expect(covers({ ...at, a: t.a, b: t.b }, bw, bh, 0), `${name} new at ${k / 200}`).toBe(false);
        if (at.show > 0.02 && old > 0.02) expect(covers({ ...at, a: f.a, b: f.b }, bw, bh, 0), `${name} old at ${k / 200}`).toBe(false);
        // A point both windows light (when they meet): never under the veil but at the very middle
        // of a move whose windows each sit under the other's bubble.
        const x = Math.max(f.a.x, t.a.x) + 1;
        const y = Math.max(f.a.y, t.a.y) + 1;
        if (inside(f.a, x, y) && inside(t.a, x, y) && name !== "each under the other's bubble") expect(veil(true, true, old, lit), `${name} dims at ${k / 200}`).toBeLessThan(0.02);
      }
      const end = fadeWay(f, t, 1, plan);
      expect(end.pose).toEqual(t);
      expect([end.lit, end.old]).toEqual([1, 0]);
    }
  });
});

describe("the tour's sample", () => {
  it("has ids no saved note or line has, which the store refuses to save", async () => {
    const { SAMPLE_ITEM, SAMPLE_NOTE, SAMPLE_PARENT, SAMPLE_STEPS, inMemory } = await import("./sample");
    const all = [SAMPLE_ITEM, SAMPLE_NOTE, SAMPLE_PARENT, ...SAMPLE_STEPS];
    expect(all.every((id) => inMemory(id))).toBe(true);
    // (All different, so taking one away never takes another.)
    expect(new Set(all).size).toBe(all.length);
    expect([1, 42, null, undefined].some((id) => inMemory(id))).toBe(false);
  });
});
