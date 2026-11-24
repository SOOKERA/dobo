// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A new note from the board's right-click menu: where it goes, its space, the built-in
// templates and their blank lines, and its waiting title.
import { describe, expect, it } from "vitest";
import { arrange, type ArrangeInput } from "./arrange";
import { History } from "./history";
import { tIn, type Key } from "./i18n.svelte";
import { blanksOf, builtInTemplates, columnAt, freshEnd, isBlank, isBuiltIn, spaceForSpot, spotPlace } from "./newNote";

const NEW = 99;
const COL = 248;
const GAP = 18;
// Three columns: 1 (h 100) and 4 (h 120) in the first, 2 (h 200) in the second, 3 (h 150) in
// the third; nothing saved (packed in order).
const board = (over: Partial<ArrangeInput> = {}): ArrangeInput => ({
  ids: [1, 2, 3, 4],
  wide: () => false,
  height: (id) => ({ 1: 100, 2: 200, 3: 150, 4: 120 })[id] ?? 100,
  cols: 3,
  colW: COL,
  gap: GAP,
  ...over,
});
/** The board with the new note put at a spot, and where each note is then. */
function placeAt(inp: ArrangeInput, x: number, y: number) {
  const a = arrange(inp);
  const s = spotPlace(a.seq, a.pos, NEW, { x, y }, inp.cols, inp.colW, inp.gap);
  const b = arrange({ ...inp, ids: [...inp.ids, NEW], saved: s.seq, height: (id) => (id === NEW ? 60 : inp.height(id)) });
  return { s, before: a.pos, after: b.pos };
}

describe("a new note where the board was right-clicked", () => {
  it("takes the column under the pointer (a click in the gap: the nearer one)", () => {
    expect(columnAt(10, 3, COL, GAP)).toBe(0);
    expect(columnAt(COL + 2, 3, COL, GAP)).toBe(0);
    expect(columnAt(COL + GAP - 2, 3, COL, GAP)).toBe(1);
    expect(columnAt(2 * (COL + GAP) + 40, 3, COL, GAP)).toBe(2);
    // (Off the board's edge: its last column.)
    expect(columnAt(5000, 3, COL, GAP)).toBe(2);
    expect(columnAt(-30, 3, COL, GAP)).toBe(0);
  });

  it("between two notes: under the one above, the one below making room; other columns stay", () => {
    // In the gap between 1 (0–100) and 4 (118–238).
    const { s, before, after } = placeAt(board(), 30, 108);
    expect(s.c).toBe(0);
    expect(s.above).toBe(1);
    expect(s.below).toBe(4);
    expect(after.get(NEW)).toMatchObject({ x: 0, y: 118 });
    expect(after.get(4)!.y).toBe(118 + 60 + GAP);
    expect(after.get(1)).toEqual(before.get(1));
    expect(after.get(2)).toEqual(before.get(2));
    expect(after.get(3)).toEqual(before.get(3));
  });

  it("far under a column's last note: it glides up to sit right under it, no gap", () => {
    const { s, after } = placeAt(board(), 2 * (COL + GAP) + 30, 600);
    expect(s.c).toBe(2);
    expect(s.above).toBe(3);
    expect(s.below).toBeNull();
    expect(after.get(NEW)).toMatchObject({ x: 2 * (COL + GAP), y: 150 + GAP });
  });

  it("in an empty column: first in it, at the top", () => {
    const { s, after } = placeAt(board({ cols: 5 }), 4 * (COL + GAP) + 30, 400);
    expect(s.c).toBe(4);
    expect(s.clicked).toBe(4);
    expect(s.above).toBeNull();
    expect(s.below).toBeNull();
    expect(after.get(NEW)).toMatchObject({ x: 4 * (COL + GAP), y: 0 });
  });

  it("in an empty column with an empty one on its left: the first empty one from the left", () => {
    // (Columns 4 and 5 empty: clicked in 5, it goes to 4.)
    const { s, after } = placeAt(board({ cols: 6 }), 5 * (COL + GAP) + 30, 400);
    expect(s.clicked).toBe(5);
    expect(s.c).toBe(4);
    expect(after.get(NEW)).toMatchObject({ x: 4 * (COL + GAP), y: 0 });
    // An empty board, clicked in the right column: the first column.
    const none = placeAt(board({ ids: [], cols: 2 }), COL + GAP + 30, 200);
    expect(none.s).toMatchObject({ clicked: 1, c: 0, above: null, below: null });
    expect(none.after.get(NEW)).toMatchObject({ x: 0, y: 0 });
  });

  it("above a column's first note: first in that column", () => {
    const { s, after } = placeAt(board(), COL + GAP + 20, 0);
    expect(s.above).toBeNull();
    expect(s.below).toBe(2);
    expect(after.get(NEW)!.y).toBe(0);
    expect(after.get(2)!.y).toBe(60 + GAP);
  });

  it("the order follows the spot: the new note's step comes right after the note above", () => {
    const { s } = placeAt(board(), 30, 108);
    const ids = s.seq.map((x) => x.id);
    expect(ids.indexOf(NEW)).toBe(ids.indexOf(1) + 1);
    expect(s.seq.find((x) => x.id === NEW)!.c).toBe(0);
  });

  it("a wide note covering the column counts as the one above", () => {
    // 1 wide over columns 0–1 (0–100), then 2 in column 0 and 3 in column 1 (as arranged).
    const inp = board({ ids: [1, 2, 3], wide: (id) => id === 1, saved: [{ id: 1, c: 0 }, { id: 2, c: 0 }, { id: 3, c: 1 }] });
    const { s, after } = placeAt(inp, COL + GAP + 20, 104);
    expect(s.c).toBe(1);
    expect(s.above).toBe(1);
    expect(s.below).toBe(3);
    expect(after.get(NEW)!.y).toBe(100 + GAP);
  });
});

describe("its space", () => {
  it("is the board's when it shows one space", () => {
    expect(spaceForSpot("personal", "work", "work")).toBe("personal");
    expect(spaceForSpot("work", undefined, "personal")).toBe("work");
  });

  it("in All: the note above's, else the one below's, else Work", () => {
    expect(spaceForSpot("all", "personal", "work")).toBe("personal");
    expect(spaceForSpot("all", undefined, "personal")).toBe("personal");
    expect(spaceForSpot("all", undefined, undefined)).toBe("work");
  });
});

describe("the built-in templates", () => {
  const words = (lang: "en" | "fr") => (k: Key) => tIn(lang, k);

  it("are Today, Ideas and Plan, named in the language of the moment", () => {
    expect(builtInTemplates(words("en")).map((t) => t.name)).toEqual(["Today", "Ideas", "Plan"]);
    expect(builtInTemplates(words("fr")).map((t) => t.name)).toEqual(["Aujourd’hui", "Idées", "Plan"]);
    for (const t of builtInTemplates(words("fr"))) expect(t.about).not.toBe(builtInTemplates(words("en")).find((e) => e.id === t.id)!.about);
  });

  it("hold only blank lines: tasks for Today, ideas for Ideas, tasks then ideas for Plan", () => {
    const [today, ideas, plan] = builtInTemplates(words("en"));
    expect(today.lines.every(isBlank)).toBe(true);
    expect(blanksOf(today)).toEqual({ task: 3, idea: 0 });
    expect(blanksOf(ideas)).toEqual({ task: 0, idea: 3 });
    expect(blanksOf(plan)).toEqual({ task: 3, idea: 2 });
    expect(plan.lines.map((l) => l.kind)).toEqual(["task", "task", "task", "idea", "idea"]);
    expect(blanksOf(null)).toEqual({ task: 0, idea: 0 });
  });

  it("have their own soft colours and ids no saved template has", () => {
    const all = builtInTemplates(words("en"));
    expect(new Set(all.map((t) => t.color)).size).toBe(3);
    expect(all.every((t) => t.palette === "soft" && t.paper === "plain")).toBe(true);
    expect(all.every((t) => isBuiltIn(t.id))).toBe(true);
    expect(isBuiltIn(Date.now())).toBe(false);
    expect(isBuiltIn(null)).toBe(false);
  });

  it("a saved template's lines are never blanks", () => {
    expect(blanksOf({ id: 1, name: "Shop", palette: "soft", color: "Sage", paper: "plain", lines: [{ kind: "task", text: "Milk" }, { kind: "idea", text: "Cake?" }] })).toEqual({ task: 0, idea: 0 });
  });
});

describe("a new note's waiting title", () => {
  it("words typed: the note is named, however it is left", () => {
    expect(freshEnd("enter", "Groceries")).toBe("name");
    expect(freshEnd("esc", " Trip ")).toBe("name");
    expect(freshEnd("blur", "Trip", { windowFocused: true })).toBe("name");
  });

  it("nothing typed: Esc or a click away takes it away again; Enter, or a click in it, keeps it", () => {
    expect(freshEnd("esc", "")).toBe("drop");
    expect(freshEnd("esc", "   ")).toBe("drop");
    expect(freshEnd("blur", "", { windowFocused: true })).toBe("drop");
    expect(freshEnd("blur", "", { windowFocused: true, inside: true })).toBe("keep");
    expect(freshEnd("enter", "")).toBe("keep");
  });

  it("another window taking the keys changes nothing yet", () => {
    expect(freshEnd("blur", "", { windowFocused: false })).toBe("wait");
    expect(freshEnd("blur", "Half typed", { windowFocused: false })).toBe("wait");
  });

  it("taken away again, it leaves nothing to undo or redo; made and kept, one Undo takes it back", async () => {
    const h = new History();
    const board: string[] = [];
    const made = () => {
      board.push("new");
      const step = { label: "Note added", undo: () => void board.splice(board.indexOf("new"), 1), redo: () => void board.push("new") };
      h.record(step);
      return step;
    };
    h.record({ label: "Renamed", undo: () => {}, redo: () => {} });
    // Esc with nothing typed: it goes, and the step before is the next to undo.
    const step = made();
    expect(h.forget(step)).toBe(true);
    await step.undo();
    expect(board).toEqual([]);
    expect(h.forget(step)).toBe(false);
    expect(await h.undo()).toBe("Renamed");
    expect(h.canRedo).toBe(true);
    expect(await h.redo()).toBe("Renamed");
    // Kept: Undo takes it away, Redo brings it back.
    made();
    expect(await h.undo()).toBe("Note added");
    expect(board).toEqual([]);
    expect(await h.redo()).toBe("Note added");
    expect(board).toEqual(["new"]);
  });
});
