// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A new note from the board's right-click menu, taken away again at once (Esc): the undo list,
// the board's saved columns and the notes' order as they were. Blank lines by id. A press in the
// note keeps its waiting title.
import { beforeAll, describe, expect, it } from "vitest";
import { History } from "./history";
import { blankIds, nextBlank, pressedIn, withoutBlank } from "./newNote";
import { testStore } from "./testStore";

describe("forgetting a change at once", () => {
  const step = (label: string) => ({ label, undo: () => {}, redo: () => {} });
  it("gives back what could be redone before it", async () => {
    const h = new History();
    h.record(step("Ticked"));
    expect(await h.undo()).toBe("Ticked");
    const made = step("Note added");
    h.record(made);
    expect(h.canRedo).toBe(false);
    expect(h.forget(made)).toBe(true);
    expect(h.canRedo).toBe(true);
    expect(await h.redo()).toBe("Ticked");
  });
  it("gives back the oldest step it pushed out", async () => {
    const h = new History(2);
    h.record(step("one"));
    h.record(step("two"));
    const made = step("Note added");
    h.record(made);
    h.forget(made);
    expect(await h.undo()).toBe("two");
    expect(await h.undo()).toBe("one");
  });
  it("not once another change came after it", async () => {
    const h = new History();
    h.record(step("Ticked"));
    await h.undo();
    const made = step("Note added");
    h.record(made);
    h.record(step("Renamed"));
    h.forget(made);
    expect(h.canRedo).toBe(false);
    expect(await h.undo()).toBe("Renamed");
    expect(h.canUndo).toBe(false);
  });
});

describe("blank lines, each by its own id", () => {
  it("filling in or taking away one takes that one, whichever it is", () => {
    const ids = blankIds({ task: 3, idea: 2 });
    expect(new Set([...ids.task, ...ids.idea]).size).toBe(5);
    const all = { 7: ids };
    const after = withoutBlank(all, 7, "task", ids.task[1]);
    expect(after[7].task).toEqual([ids.task[0], ids.task[2]]);
    expect(after[7].idea).toEqual(ids.idea);
    // (Not there: the same list.)
    expect(withoutBlank(after, 7, "task", ids.task[1])).toBe(after);
    // The last one gone: the note leaves the list.
    let none = withoutBlank({ 7: { task: [1], idea: [] } }, 7, "task", 1);
    expect(none).toEqual({});
    none = withoutBlank(none, 8, "idea", 2);
    expect(none).toEqual({});
  });
  it("the keyboard goes to the next blank after Enter, the one before after Backspace", () => {
    expect(nextBlank([1, 2, 3], 1, "enter")).toBe(2);
    expect(nextBlank([1, 2, 3], 3, "enter")).toBe(1);
    expect(nextBlank([1], 1, "enter")).toBe(null);
    expect(nextBlank([1, 2, 3], 3, "backspace")).toBe(2);
    expect(nextBlank([1, 2, 3], 1, "backspace")).toBe(2);
    expect(nextBlank([1], 1, "backspace")).toBe(null);
  });
});

describe("a press in the new note", () => {
  it("keeps its waiting title: a blur right after a press in it", () => {
    expect(pressedIn(1000, 1010)).toBe(true);
    expect(pressedIn(1000, 1600)).toBe(false);
    expect(pressedIn(null, 1000)).toBe(false);
  });
});

describe("a new note taken away again at once", () => {
  let s: Awaited<ReturnType<typeof testStore>>;
  beforeAll(async () => {
    s = await testStore();
  });

  it("leaves the board's saved columns, the notes' order and the undo list as they were", async () => {
    const { store } = s;
    const order = () => [...store.notes].sort((a, b) => a.sort - b.sort).map((n) => n.id);
    const ids = order();
    // An arrangement saved at three columns, the board showing two.
    const three = ids.map((id, i) => ({ id, c: i % 3 }));
    await store.setSetting("board_steps", { "all:3": three });
    // A step to redo.
    const hub = store.notes.find((n) => n.title === "Hub")!;
    await store.rename(hub, "Hub 2");
    await store.history.undo();
    expect(store.history.canRedo).toBe(true);
    const before = order();
    // The board puts the new note after the first one, at two columns.
    store.board = {
      point: (p) => p,
      around: () => ({ above: null, below: null }),
      place: (id) => store.saveBoardSteps("all", 2, [before[0], id, ...before.slice(1)].map((x, i) => ({ id: x, c: i % 2 }))),
    };
    const n = await store.fromTemplate(null, "", { at: { x: 10, y: 10 }, fresh: true });
    expect(store.fresh).toBe(n.id);
    expect(order()).toContain(n.id);
    expect(await store.dropNew(n.id)).toBe(true);
    expect(store.notes.some((x) => x.id === n.id)).toBe(false);
    expect(store.settings.board_steps).toEqual({ "all:3": three });
    expect(order()).toEqual(before);
    expect(store.history.canRedo).toBe(true);
    await store.history.redo();
    expect(store.notes.find((x) => x.id === hub.id)?.title).toBe("Hub 2");
    store.board = null;
  });

  it("kept, Undo takes it from the saved columns too, and Redo puts it back there", async () => {
    const { store } = s;
    const ids = [...store.notes].sort((a, b) => a.sort - b.sort).map((n) => n.id);
    store.board = {
      point: (p) => p,
      around: () => ({ above: null, below: null }),
      place: (id) => store.saveBoardSteps("all", 2, [ids[0], id, ...ids.slice(1)].map((x, i) => ({ id: x, c: i % 2 }))),
    };
    const n = await store.fromTemplate(null, "", { at: { x: 10, y: 10 }, fresh: true });
    await store.nameNew(n, "Kept");
    const steps = () => (store.settings.board_steps as Record<string, { id: number }[]>)["all:2"].map((x) => x.id);
    expect(steps()).toContain(n.id);
    await store.history.undo();
    expect(steps()).not.toContain(n.id);
    await store.history.redo();
    expect(steps()[1]).toBe(n.id);
    store.board = null;
  });

  it("from a template with blank lines: filling in one takes that blank", async () => {
    const { store } = s;
    const n = await store.fromTemplate({ id: -1, name: "Today", palette: "soft", color: "Butter", paper: "plain", lines: [{ kind: "task", text: "" }, { kind: "task", text: "" }, { kind: "task", text: "" }] }, "");
    const ids = store.blanks[n.id].task;
    expect(ids.length).toBe(3);
    const made = await store.fillBlank(n.id, "task", ids[1], "Milk");
    expect(made?.text).toBe("Milk");
    expect(store.blanks[n.id].task).toEqual([ids[0], ids[2]]);
    store.dropBlank(n.id, "task", ids[2]);
    store.dropBlank(n.id, "task", ids[0]);
    expect(store.blanks[n.id]).toBeUndefined();
  });
});
