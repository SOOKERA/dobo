// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { BOARD_PAD, DOCK, FOLD_AFTER, FOOT, GAP, OPEN_AFTER, OPEN_H, STRIP_H, dockAt, dockLift, dockPlace, dockWake, within, type Dock } from "./hoursDock";

/** The dock after the pointer is seen at each [time, inside] in turn. */
const run = (seen: [number, boolean][], from: Dock = DOCK) => seen.reduce((d, [now, inside]) => dockAt(d, inside, now), from);

describe("the hours under the Inbox", () => {
  it("open once the slip rests on the strip a moment", () => {
    const d = run([
      [0, false],
      [10, true],
    ]);
    expect(d.open).toBe(false);
    expect(dockWake(d)).toBe(10 + OPEN_AFTER);
    expect(dockAt(d, true, 10 + OPEN_AFTER - 1).open).toBe(false);
    expect(dockAt(d, true, 10 + OPEN_AFTER).open).toBe(true);
  });
  it("stay shut for a slip carried straight past the strip", () => {
    const d = run([
      [0, false],
      [16, true],
      [32, true],
      [48, false],
      [64, false],
    ]);
    expect(d).toEqual({ open: false, armed: true, since: null });
    expect(dockWake(d)).toBe(null);
  });
  it("wait for a line picked up under the strip to leave it once", () => {
    const d = run([
      [0, true],
      [500, true],
    ]);
    expect(d.open).toBe(false);
    expect(dockWake(d)).toBe(null);
    expect(run([[600, false], [700, true], [700 + OPEN_AFTER, true]], d).open).toBe(true);
  });
  it("fold a short moment after the slip moves away, never at the edge", () => {
    const open: Dock = { open: true, armed: true, since: null };
    // In and out at the edge: still open, the moment starts again each time it comes back.
    const d = run(
      [
        [0, false],
        [FOLD_AFTER - 10, true],
        [FOLD_AFTER, false],
        [FOLD_AFTER * 2 - 1, false],
      ],
      open,
    );
    expect(d.open).toBe(true);
    expect(dockWake(d)).toBe(FOLD_AFTER * 2);
    const folded = dockAt(d, false, FOLD_AFTER * 2);
    expect(folded).toEqual({ open: false, armed: true, since: null });
    // Back on the strip: they open again.
    expect(run([[1000, true], [1000 + OPEN_AFTER, true]], folded).open).toBe(true);
  });
  it("gives the same dock back when nothing changes", () => {
    const d: Dock = { open: false, armed: true, since: 20 };
    expect(dockAt(d, true, 40)).toBe(d);
    const o: Dock = { open: true, armed: true, since: null };
    expect(dockAt(o, true, 40)).toBe(o);
  });
});

describe("near the strip", () => {
  const b = { left: 10, top: 100, right: 300, bottom: 140 };
  it("counts a pad round it", () => {
    expect(within(b, 150, 120)).toBe(true);
    expect(within(b, 150, 150)).toBe(false);
    expect(within(b, 150, 150, 12)).toBe(true);
    expect(within(b, 150, 88, 12)).toBe(true);
    expect(within(b, 150, 87, 12)).toBe(false);
    expect(within(null, 150, 120)).toBe(false);
  });
});

describe("where the strip sits", () => {
  it("hangs just under the Inbox and sticks near the top of the board", () => {
    // The place comes after the Inbox's gap; the strip goes back up to GAP under the Inbox, and
    // stuck at the top it is GAP from the board's top.
    const p = dockPlace(1);
    expect(p.drop).toBe(GAP - 18);
    expect(p.stick + p.drop).toBe(GAP);
  });
  it("stays whole in sight at the bottom, under a long Inbox", () => {
    for (const z of [0.88, 1, 1.15]) {
      const p = dockPlace(z);
      // The strip's bottom edge, its place held `bottom` above the bottom padding of a board
      // whose bottom edge is at 600: clear of the note sizes there.
      const place = 600 - BOARD_PAD - p.bottom;
      expect(place + p.drop + STRIP_H).toBeCloseTo(600 - FOOT, 1);
      expect(p.stick + p.drop).toBeCloseTo(GAP, 1);
    }
  });
});

describe("the hours opening", () => {
  const area = { top: 100, bottom: 700 };
  it("open where the strip is when there is room under it", () => {
    expect(dockLift(200, area)).toBe(0);
    expect(dockLift(700 - FOOT - OPEN_H, area)).toBe(0);
  });
  it("rise just enough to show whole near the bottom of the board, clear of the note sizes", () => {
    expect(dockLift(700 - FOOT - OPEN_H + 30, area)).toBe(30);
    expect(dockLift(700 - FOOT - STRIP_H, area)).toBe(OPEN_H - STRIP_H);
  });
  it("never rise past the board's top (a very short board)", () => {
    expect(dockLift(150, { top: 100, bottom: 200 })).toBe(150 - 100 - GAP);
  });
});
