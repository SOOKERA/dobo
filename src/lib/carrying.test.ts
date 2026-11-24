// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Carrying a line: what the hours do while slips land, the board's edge scrolling near the
// strip, which side the slip rides on, and the store's steps for a carried line (put into a
// note, made a new note), undone and redone.
import { beforeAll, describe, expect, it } from "vitest";
import { edgeStep, hourLanding, slipOnLeft } from "./hoursDock";
import { testStore } from "./testStore";

describe("the hours while slips land", () => {
  it("show only for a slip landing on an hour, never for one landing on a new note", () => {
    expect(hourLanding([])).toBe(false);
    expect(hourLanding([{ kind: "new" }])).toBe(false);
    expect(hourLanding([{ kind: "new" }, { kind: "hour" }])).toBe(true);
  });
});

describe("the board's edge scrolling a carried line", () => {
  const area = { top: 100, bottom: 700 };
  const off = { overHour: false, atDock: false, dockTop: null };
  it("scrolls near the top and bottom edges, faster nearer them", () => {
    expect(edgeStep(400, area, 48, off)).toBe(0);
    expect(edgeStep(110, area, 48, off)).toBe(-10);
    expect(edgeStep(140, area, 48, off)).toBeLessThan(0);
    expect(edgeStep(690, area, 48, off)).toBe(10);
  });
  it("never over an hour", () => {
    expect(edgeStep(110, area, 48, { ...off, overHour: true })).toBe(0);
  });
  it("on or near the strip stuck under the board's top: only above its top", () => {
    const dock = { overHour: false, atDock: true, dockTop: 124 };
    // On the strip itself (or the open hours): they stay put under the slip.
    expect(edgeStep(130, area, 48, dock)).toBe(0);
    // Between the board's top and the strip: up it goes, to the Inbox.
    expect(edgeStep(110, area, 48, dock)).toBeLessThan(0);
    expect(edgeStep(110, area, 48, { ...dock, dockTop: null })).toBe(0);
  });
});

describe("the slip's side away from the hours", () => {
  it("rides on the pointer's right, unless its words would cross the window's edge", () => {
    expect(slipOnLeft(300, 200, 1000, false)).toBe(false);
    expect(slipOnLeft(850, 200, 1000, false)).toBe(true);
    // (Near the left edge too: the roomier side.)
    expect(slipOnLeft(100, 250, 300, false)).toBe(false);
  });
  it("comes back to the right only with room to spare", () => {
    // 200 px of words, 210 px of room on the right: still left until 24 px more.
    expect(slipOnLeft(766, 200, 1000, true)).toBe(true);
    expect(slipOnLeft(740, 200, 1000, true)).toBe(false);
  });
});

describe("the store's steps for a carried line", () => {
  let s: Awaited<ReturnType<typeof testStore>>;
  beforeAll(async () => {
    s = await testStore();
  });
  const pic = (id: number, noteId: number, itemId: number | null) => ({ id, note_id: noteId, item_id: itemId, path: `p${id}.png`, sort: 0, created_at: 0 });
  const picOf = (id: number) => s.store.pictures.find((p) => p.id === id);
  const saved = async (id: number) => (await s.api.loadAll()).pictures?.find((p) => p.id === id);

  it("put away from the Inbox: its pictures become the note's, and an undo gives them back", async () => {
    const { store, api } = s;
    const inbox = store.notes.find((n) => n.title === "Inbox")!;
    const hub = store.notes.find((n) => n.title === "Hub")!;
    const line = store.items.find((i) => i.note_id === inbox.id && i.text === "Book of Concord")!;
    await api.restore([], [], [pic(50, inbox.id, line.id)]);
    await store.reload();
    await store.moveItem(line, { noteId: hub.id, parentId: null });
    expect(picOf(50)).toMatchObject({ note_id: hub.id, item_id: null });
    expect(await saved(50)).toMatchObject({ note_id: hub.id, item_id: null });
    await store.history.undo();
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(inbox.id);
    expect(picOf(50)).toMatchObject({ note_id: inbox.id, item_id: line.id });
    expect(await saved(50)).toMatchObject({ note_id: inbox.id, item_id: line.id });
    await store.history.redo();
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(hub.id);
    expect(picOf(50)).toMatchObject({ note_id: hub.id, item_id: null });
    await store.history.undo();
    expect(picOf(50)).toMatchObject({ note_id: inbox.id, item_id: line.id });
  });

  it("into the Inbox: its pictures stay its own", async () => {
    const { store, api } = s;
    const hub = store.notes.find((n) => n.title === "Hub")!;
    const inbox = store.notes.find((n) => n.title === "Inbox")!;
    const line = store.items.find((i) => i.note_id === hub.id && i.text === "Weekly")!;
    await api.restore([], [], [pic(51, hub.id, line.id)]);
    await store.reload();
    await store.moveItem(line, { noteId: inbox.id, parentId: null });
    expect(picOf(51)).toMatchObject({ note_id: inbox.id, item_id: line.id });
    await store.history.undo();
    expect(picOf(51)).toMatchObject({ note_id: hub.id, item_id: line.id });
  });

  it("made a new note: one step; undo puts the line back whole, redo brings the same note", async () => {
    const { store, api } = s;
    const logi = store.notes.find((n) => n.title === "Logitech")!;
    const line = store.items.find((i) => i.note_id === logi.id && i.text.startsWith("Migrate"))!;
    const steps = store.items.filter((i) => i.parent_id === line.id).map((i) => i.id);
    expect(steps.length).toBe(2);
    await api.restore([], [], [pic(52, logi.id, line.id)]);
    await store.reload();
    const n = (await store.noteFromLine(line, "work"))!;
    expect(n.title).toBe("Migrate the Logitech stuff to GitHub");
    expect(store.items.find((i) => i.id === line.id)).toMatchObject({ note_id: n.id, parent_id: null });
    for (const id of steps) expect(store.items.find((i) => i.id === id)?.note_id).toBe(n.id);
    // (Its pictures stay its own, under it.)
    expect(picOf(52)).toMatchObject({ note_id: n.id, item_id: line.id });
    await store.history.undo();
    expect(store.notes.some((x) => x.id === n.id)).toBe(false);
    expect(store.items.find((i) => i.id === line.id)).toMatchObject({ note_id: logi.id, text: line.text, sort: line.sort });
    for (const id of steps) expect(store.items.find((i) => i.id === id)?.note_id).toBe(logi.id);
    expect(picOf(52)).toMatchObject({ note_id: logi.id, item_id: line.id });
    expect((await api.loadAll()).notes.some((x) => x.id === n.id)).toBe(false);
    await store.history.redo();
    expect(store.notes.find((x) => x.id === n.id)?.title).toBe(n.title);
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(n.id);
    expect(picOf(52)).toMatchObject({ note_id: n.id, item_id: line.id });
    await store.history.undo();
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(logi.id);
  });

  it("dropped on an empty spot: the note goes there (the board places it), and its undo takes it from the saved columns", async () => {
    const { store } = s;
    const hub = store.notes.find((n) => n.title === "Hub")!;
    const line = store.items.find((i) => i.note_id === hub.id && i.parent_id == null && i.kind === "idea")!;
    const placed: { id: number; at: { x: number; y: number } }[] = [];
    const ids = [...store.notes].sort((a, b) => a.sort - b.sort).map((n) => n.id);
    store.board = {
      point: (p) => p,
      around: () => ({ above: null, below: null }),
      place: (id, at) => {
        placed.push({ id, at });
        return store.saveBoardSteps("all", 2, [ids[0], id, ...ids.slice(1)].map((x, i) => ({ id: x, c: i % 2 })));
      },
    };
    const n = (await store.noteFromLine(line, "work", { x: 40, y: 300 }))!;
    expect(placed).toEqual([{ id: n.id, at: { x: 40, y: 300 } }]);
    await new Promise((r) => setTimeout(r, 0));
    const steps = () => (store.settings.board_steps as Record<string, { id: number }[]>)["all:2"].map((x) => x.id);
    expect(steps()[1]).toBe(n.id);
    await store.history.undo();
    expect(steps()).not.toContain(n.id);
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(hub.id);
    await store.history.redo();
    expect(steps()[1]).toBe(n.id);
    expect(store.items.find((i) => i.id === line.id)?.note_id).toBe(n.id);
    // On the dashed card (or from the line's menu): at the end, nothing placed.
    const other = store.items.find((i) => i.note_id === hub.id && i.parent_id == null && i.id !== line.id)!;
    await store.noteFromLine(other, "work");
    expect(placed.length).toBe(1);
    store.board = null;
  });
});
