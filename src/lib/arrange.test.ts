// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board's columns: masonry from the saved order, kept columns after a drag, moving a note.
import { describe, expect, it } from "vitest";
import { arrange, dropFromSteps, dropNotesFromSteps, moveTo, putBackInSteps, type Step } from "./arrange";

const GAP = 10;
const COLW = 100;
const H: Record<number, number> = { 1: 100, 2: 50, 3: 80, 4: 120, 5: 60, 6: 90, 7: 40, 8: 70 };

function lay(ids: number[], cols: number, saved?: Step[] | null, wide: number[] = []) {
  return arrange({ ids, wide: (id) => wide.includes(id), height: (id) => H[id] ?? 50, cols, colW: COLW, gap: GAP, saved });
}

/** Ids of each column, top to bottom. */
function columns(a: ReturnType<typeof lay>, cols: number) {
  const out: number[][] = Array.from({ length: cols }, () => []);
  const list = [...a.pos.entries()].sort((x, y) => x[1].y - y[1].y);
  for (const [id, b] of list) {
    const c = Math.round(b.x / (COLW + GAP));
    out[c].push(id);
    if (b.w > COLW * 1.5) out[c + 1].push(id);
  }
  return out;
}

/** Drag `id` so its middle sits at (column c, height cy) and return the new layout. */
function drag(a: ReturnType<typeof lay>, ids: number[], cols: number, id: number, c: number, cy: number, wide: number[] = []) {
  const seq = moveTo(a.seq, a.pos, id, c, cy, COLW, GAP, wide.includes(id));
  return { seq, after: lay(ids, cols, seq, wide) };
}

describe("board columns", () => {
  const ids = [1, 2, 3, 4, 5, 6, 7, 8];

  it("places notes in the column free the highest without a saved arrangement", () => {
    const a = lay(ids, 3);
    expect(columns(a, 3)).toEqual([[1, 6], [2, 4, 8], [3, 5, 7]]);
  });

  it("a saved arrangement reproduces the same places", () => {
    const a = lay(ids, 3);
    const b = lay(ids, 3, a.seq);
    for (const id of ids) expect(b.pos.get(id)).toEqual(a.pos.get(id));
  });

  it("moving a note down one slot swaps it with its neighbour only", () => {
    const a = lay(ids, 3);
    // 2 is above 4 in column 1; carry 2 just below 4's middle.
    const four = a.pos.get(4)!;
    const { after } = drag(a, ids, 3, 2, 1, four.y + four.h / 2 + 1);
    const cols = columns(after, 3);
    expect(cols[1]).toEqual([4, 2, 8]);
    // The other columns don't move at all.
    for (const id of [1, 6, 3, 5, 7]) expect(after.pos.get(id)).toEqual(a.pos.get(id));
  });

  it("moving a note up one slot", () => {
    const a = lay(ids, 3);
    const four = a.pos.get(4)!;
    const { after } = drag(a, ids, 3, 8, 1, four.y + four.h / 2 - 1);
    expect(columns(after, 3)[1]).toEqual([2, 8, 4]);
    for (const id of [1, 6, 3, 5, 7]) expect(after.pos.get(id)).toEqual(a.pos.get(id));
  });

  it("staying in its slot changes nothing", () => {
    const a = lay(ids, 3);
    const four = a.pos.get(4)!;
    const seq = moveTo(a.seq, a.pos, 4, 1, four.y + four.h / 2, COLW, GAP, false);
    const after = lay(ids, 3, seq);
    for (const id of ids) expect(after.pos.get(id)).toEqual(a.pos.get(id));
  });

  it("moving into another column only changes that column and the one it left", () => {
    const a = lay(ids, 3);
    const one = a.pos.get(1)!;
    // Carry 4 (column 1) into column 0, just under note 1.
    const { after } = drag(a, ids, 3, 4, 0, one.y + one.h / 2 + 1);
    const cols = columns(after, 3);
    expect(cols[0]).toEqual([1, 4, 6]);
    expect(cols[1]).toEqual([2, 8]);
    for (const id of [3, 5, 7]) expect(after.pos.get(id)).toEqual(a.pos.get(id));
  });

  it("goes to the top of a column when above everything, and to the bottom when below", () => {
    const a = lay(ids, 3);
    expect(columns(drag(a, ids, 3, 7, 0, -20).after, 3)[0]).toEqual([7, 1, 6]);
    expect(columns(drag(a, ids, 3, 1, 2, 9999).after, 3)[2]).toEqual([3, 5, 7, 1]);
  });

  it("keeps the dragged columns when a note is added (it goes to the lowest column, at the bottom)", () => {
    const a = lay(ids, 3);
    const four = a.pos.get(4)!;
    const { seq } = drag(a, ids, 3, 2, 1, four.y + four.h / 2 + 1);
    const after = lay([...ids, 9], 3, seq);
    const cols = columns(after, 3);
    expect(cols[1]).toEqual([4, 2, 8]);
    expect(cols.flat()).toContain(9);
    const nine = after.pos.get(9)!;
    expect(nine.y).toBeGreaterThan(0);
  });

  it("skips notes that are gone and clamps columns when there are fewer", () => {
    const saved: Step[] = [
      { id: 1, c: 0 },
      { id: 2, c: 4 },
      { id: 99, c: 1 },
    ];
    const a = lay([1, 2], 2, saved);
    expect(columns(a, 2)).toEqual([[1], [2]]);
  });

  it("a wide note spans two columns and the notes below it in both columns stay below it", () => {
    const wide = [3];
    const a = lay(ids, 3, null, wide);
    const w = a.pos.get(3)!;
    expect(w.w).toBe(COLW * 2 + GAP);
    // Move a normal note from below the wide one to above it, in one of its columns.
    const c = Math.round(w.x / (COLW + GAP));
    const below = columns(a, 3)[c].filter((id) => a.pos.get(id)!.y > w.y);
    expect(below.length).toBeGreaterThan(0);
    const mover = below[0];
    const { after } = drag(a, ids, 3, mover, c, w.y - 1, wide);
    expect(after.pos.get(mover)!.y).toBeLessThan(after.pos.get(3)!.y);
    // Nothing overlaps.
    const boxes = [...after.pos.values()];
    for (let i = 0; i < boxes.length; i++)
      for (let j = i + 1; j < boxes.length; j++) {
        const p = boxes[i];
        const q = boxes[j];
        const overlap = p.x < q.x + q.w && q.x < p.x + p.w && p.y < q.y + q.h && q.y < p.y + p.h;
        expect(overlap).toBe(false);
      }
  });

  it("carrying a wide note moves it as a pair of columns", () => {
    const wide = [5];
    const a = lay(ids, 3, null, wide);
    const { after } = drag(a, ids, 3, 5, 1, -20, wide);
    const b = after.pos.get(5)!;
    expect(b.y).toBe(0);
    expect(Math.round(b.x / (COLW + GAP))).toBe(1);
  });
});

describe("saved columns when a note is deleted", () => {
  const before = { "all:3": [{ id: 1, c: 0 }, { id: 2, c: 1 }, { id: 3, c: 0 }], "work:2": [{ id: 3, c: 1 }, { id: 4, c: 0 }] };
  it("leave the deleted note out of every view and column count", () => {
    expect(dropFromSteps(before, 3)).toEqual({ "all:3": [{ id: 1, c: 0 }, { id: 2, c: 1 }], "work:2": [{ id: 4, c: 0 }] });
    expect(dropFromSteps(before, 9)).toBeNull();
    expect(dropFromSteps(undefined, 3)).toBeNull();
  });
  it("put it back where it was on undo, after the note that was before it", () => {
    const now = { ...dropFromSteps(before, 3)!, "all:3": [{ id: 2, c: 1 }, { id: 1, c: 0 }] };
    expect(putBackInSteps(now, before, 3)).toEqual({ "all:3": [{ id: 2, c: 1 }, { id: 3, c: 0 }, { id: 1, c: 0 }], "work:2": [{ id: 3, c: 1 }, { id: 4, c: 0 }] });
    expect(putBackInSteps(before, before, 3)).toBeNull();
  });
});

describe("the Inbox leaves the board (1.0: it is a strip, not a card)", () => {
  const saved = { "all:2": [{ id: 1, c: 0 }, { id: 2, c: 1 }, { id: 3, c: 0 }], "work:3": [{ id: 4, c: 2 }, { id: 1, c: 0 }], "personal:2": [{ id: 5, c: 0 }] };

  it("its places go from every saved arrangement, the others keep theirs in order", () => {
    expect(dropNotesFromSteps(saved, [1, 4])).toEqual({ "all:2": [{ id: 2, c: 1 }, { id: 3, c: 0 }], "work:3": [], "personal:2": [{ id: 5, c: 0 }] });
  });

  it("nothing to change, nothing saved again (no Inbox in them, or no saved columns)", () => {
    expect(dropNotesFromSteps(saved, [9])).toBeNull();
    expect(dropNotesFromSteps(saved, [])).toBeNull();
    expect(dropNotesFromSteps(undefined, [1])).toBeNull();
  });

  it("the board packs the other notes as before, the Inbox's room closed", () => {
    // Before: the Inbox (1) heads the first column; after, the note under it moves up.
    const after = lay([2, 3], 2, dropNotesFromSteps(saved, [1])!["all:2"]);
    expect(after.pos.get(3)).toMatchObject({ x: 0, y: 0 });
    expect(after.pos.get(2)).toMatchObject({ x: COLW + GAP, y: 0 });
  });
});
