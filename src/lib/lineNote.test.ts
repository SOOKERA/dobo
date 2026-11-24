// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A carried line dropped on "+ New note" (or the board's empty space) becomes a new note.
import { describe, expect, it } from "vitest";
import { arrange, type ArrangeInput } from "./arrange";
import { changedKeys, withFields } from "./history";
import { TITLE_MAX, lineInNote, nextNoteSlot, spaceForLine, titleFromLine } from "./lineNote";
import type { Item } from "./types";

const blankItem = (over: Partial<Item>): Item => ({
  id: 0, note_id: 0, kind: "task", text: "", due_at: null, done_at: null, sort: 0, created_at: 1, parent_id: null, alarm: "reminder", rhythm: null, link: null, file: null, repeat: null,
  ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null, ...over,
});

describe("a new note from a carried line: its title", () => {
  it("is the line's words when they fit", () => {
    expect(titleFromLine("Call the bank about the loan")).toBe("Call the bank about the loan");
    expect(titleFromLine("  Plan   the\ntrip  ")).toBe("Plan the trip");
    expect(titleFromLine("x".repeat(TITLE_MAX))).toBe("x".repeat(TITLE_MAX));
  });

  it("is cut on a word at about 60 characters, with an ellipsis", () => {
    const line = "Ask Lea whether the new printer for the office can do double-sided colour prints";
    const title = titleFromLine(line);
    expect(title).toBe("Ask Lea whether the new printer for the office can do…");
    expect(title.length).toBeLessThanOrEqual(TITLE_MAX + 1);
    // A space right after the 60th character: the cut keeps every word up to there.
    expect(titleFromLine(`${"a".repeat(55)} bcde fgh`)).toBe(`${"a".repeat(55)} bcde…`);
  });

  it("leaves no comma, colon or dash hanging before the ellipsis", () => {
    expect(titleFromLine(`Groceries for the weekend, then the pharmacy: milk, eggs, — and bread for Sunday`)).toBe("Groceries for the weekend, then the pharmacy: milk, eggs…");
  });

  it("cuts inside one long word (a link) rather than keep a too short title", () => {
    const url = `Read https://example.com/${"a".repeat(80)}`;
    const title = titleFromLine(url);
    expect(title).toBe(`${url.slice(0, TITLE_MAX)}…`);
  });

  it("never cuts an emoji in half", () => {
    const title = titleFromLine(`${"🍎".repeat(70)}`);
    expect([...title].length).toBe(TITLE_MAX + 1);
    expect(title.endsWith("🍎…")).toBe(true);
  });
});

describe("a new note from a carried line: its space", () => {
  it("is the board's, or in All the line's own", () => {
    expect(spaceForLine("work", "personal", "work")).toBe("work");
    expect(spaceForLine("personal", "work", "work")).toBe("personal");
    expect(spaceForLine("all", "personal", "work")).toBe("personal");
    expect(spaceForLine("all", "work", "personal")).toBe("work");
    // (A line whose note isn't known: where new things go.)
    expect(spaceForLine("all", undefined, "personal")).toBe("personal");
  });
});

describe("a new note from a carried line: the line moves inside, and Undo puts it back", () => {
  const step = blankItem({ id: 7, note_id: 3, parent_id: 2, kind: "task", text: "Book the hotel", due_at: 1_800_000_000_000, sort: 4, link: { url: "https://example.com", title: "Hotel", site: "example.com", image: null } });

  it("is the new note's first line, with everything it had", () => {
    const inside = lineInNote(step, 12);
    expect(inside).toMatchObject({ id: 7, note_id: 12, parent_id: null, sort: 0, kind: "task", text: "Book the hotel", due_at: 1_800_000_000_000 });
    expect(inside.link).toEqual(step.link);
    // Only its place changes.
    expect(changedKeys(step, inside).sort()).toEqual(["note_id", "parent_id", "sort"]);
  });

  it("goes back to its old place on Undo, keeping what changed meanwhile without a step", () => {
    const inside = lineInNote(step, 12);
    const keys = changedKeys(step, inside);
    // (Its reminder rang meanwhile: that stays.)
    const now: Item = { ...inside, ring_state: "passed" };
    const back = withFields(now, step, keys);
    expect(back).toMatchObject({ note_id: 3, parent_id: 2, sort: 4, ring_state: "passed" });
    // And Redo puts it inside again.
    expect(withFields(back, inside, keys)).toMatchObject({ note_id: 12, parent_id: null, sort: 0 });
  });

  it("an idea stays an idea, an Inbox line keeps its time", () => {
    const idea = blankItem({ id: 9, note_id: 1, kind: "idea", text: "A tea corner" });
    expect(lineInNote(idea, 5).kind).toBe("idea");
    const dumped = blankItem({ id: 10, note_id: 1, kind: "task", text: "Dentist", due_at: 1_700_000_000_000 });
    expect(lineInNote(dumped, 5).due_at).toBe(1_700_000_000_000);
  });
});

describe("the dashed “+ New note” card", () => {
  const H: Record<number, number> = { 1: 100, 2: 50, 3: 80, 4: 120 };
  const input = (ids: number[], cols: number, wide: number[] = []): ArrangeInput => ({ ids, wide: (id) => wide.includes(id), height: (id) => H[id] ?? 60, cols, colW: 100, gap: 10 });

  it("waits exactly where the new note will go", () => {
    for (const cols of [1, 2, 3]) {
      const inp = input([1, 2, 3, 4], cols, cols > 1 ? [4] : []);
      const spot = nextNoteSlot(inp, 92);
      const after = arrange({ ...inp, ids: [...inp.ids, 99] }).pos.get(99);
      expect(spot, `${cols} columns`).toMatchObject({ x: after!.x, y: after!.y, w: after!.w, h: 92 });
    }
  });

  it("goes to the column free the highest, and on an empty board to the first", () => {
    expect(nextNoteSlot(input([1, 2], 2), 92)).toMatchObject({ x: 110, y: 60 });
    expect(nextNoteSlot(input([], 3), 92)).toMatchObject({ x: 0, y: 0, w: 100 });
  });

  it("follows the columns as arranged by hand", () => {
    const inp = { ...input([1, 2, 3], 2), saved: [{ id: 1, c: 0 }, { id: 2, c: 0 }, { id: 3, c: 0 }] };
    expect(nextNoteSlot(inp, 92)).toMatchObject({ x: 110, y: 0 });
  });

  it("moves no note to make room for itself", () => {
    const inp = input([1, 2, 3, 4], 3);
    const before = arrange(inp).pos;
    nextNoteSlot(inp, 92);
    expect(arrange(inp).pos).toEqual(before);
  });
});
