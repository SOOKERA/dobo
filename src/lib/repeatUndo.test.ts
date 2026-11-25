// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A ticked repeating task waiting for its next day, changed, and its Undo: the store over the
// preview's backend (which keeps the app's rules, db.rs). Each Undo puts the line back exactly,
// never a second time in its history, never ringing at once; a copy the board kept from another
// moment never unticks it nor stops its series.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { untickedRule } from "./doneLog";
import type { Item } from "./types";

/** Wednesday 18 November 2026 at noon: after the day's start (08:00), well before its end. */
const NOON = new Date(2026, 10, 18, 12, 0);

async function start() {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOON);
  const { testStore } = await import("./testStore");
  return testStore();
}

describe("a ticked repeating task waiting for its next day", () => {
  beforeEach(() => {
    vi.resetModules();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  const stretch = (items: Item[]) => items.find((i) => i.text.startsWith("Stretch"))!;

  it("Repeat removed, then Undo: back ticked at its next time", async () => {
    const { store } = await start();
    const s0 = stretch(store.items);
    expect(s0.repeat?.ticked).toBeTruthy();
    await store.setRepeat(s0, null);
    let s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat]).toEqual([s0.done_at, s0.repeat!.ticked!.due, null]);
    await store.history.undo();
    await store.reload();
    s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat?.ticked, s.repeat?.done]).toEqual([s0.done_at, s0.due_at, s0.repeat!.ticked, s0.repeat!.done]);
    // Not open at once (its next day hasn't come): one time in its history, as before.
    const had = store.logOf(s0.id).length;
    await store.reload();
    expect(store.items.find((i) => i.id === s0.id)!.done_at).toBe(s0.done_at);
    expect(store.logOf(s0.id).length).toBe(had);
  });

  it("Repeat set to another shape, then Undo", async () => {
    const { store } = await start();
    const s0 = stretch(store.items);
    await store.setRepeat(s0, { ...untickedRule(s0.repeat!), freq: "week", days: [1] });
    await store.history.undo();
    await store.reload();
    const s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at != null, s.due_at, s.repeat?.ticked?.due, s.repeat?.done]).toEqual([true, s0.due_at, s0.repeat?.ticked?.due, s0.repeat?.done]);
  });

  it("its time set to an hour from now opens it at once; Undo puts it back ticked, its steps too", async () => {
    const { store } = await start();
    let s0 = stretch(store.items);
    const step = (await store.addStep(s0, "Mat"))!;
    await store.toggleDone(store.items.find((i) => i.id === step.id)!);
    const stepDone = store.items.find((i) => i.id === step.id)!.done_at;
    s0 = store.items.find((i) => i.id === s0.id)!;
    const had = store.logOf(s0.id).length;
    await store.setDue(s0, Date.now() + 3600_000);
    await store.reload();
    let s = store.items.find((i) => i.id === s0.id)!;
    expect(s.done_at).toBeNull();
    expect(store.items.find((i) => i.id === step.id)!.done_at).toBeNull();
    await store.history.undo();
    s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat?.ticked, s.repeat?.done]).toEqual([s0.done_at, s0.due_at, s0.repeat!.ticked, s0.repeat!.done]);
    expect(store.items.find((i) => i.id === step.id)!.done_at).toBe(stepDone);
    expect(store.logOf(s0.id).length).toBe(had);
    expect(store.logOf(step.id)).toEqual([]);
    // As saved: still waiting for its next day.
    await store.reload();
    s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at]).toEqual([s0.done_at, s0.due_at]);
    expect(store.logOf(s0.id).length).toBe(had);
  });

  it("an open copy from before its tick, saved or ticked, leaves it ticked and its history as it is", async () => {
    const { store, api } = await start();
    const s0 = stretch(store.items);
    const had = store.logOf(s0.id).length;
    const before: Item = { ...s0, done_at: null, due_at: s0.repeat!.ticked!.due, repeat: untickedRule(s0.repeat!) };
    // Saved (its words): the words are kept, the tick too.
    const saved = await api.updateItem({ ...before, text: "Stretch, 15 min" });
    expect([saved.text, saved.done_at, saved.due_at, saved.repeat?.ticked]).toEqual(["Stretch, 15 min", s0.done_at, s0.due_at, s0.repeat!.ticked]);
    // The board still shows that copy, and its box is clicked: nothing changes, no time is added
    // to its history (not even for a moment), nothing to undo.
    store.upsertItem({ ...before, text: "Stretch, 15 min" });
    const steps = (store.history as unknown as { past: unknown[] }).past.length;
    await store.toggleDone(store.items.find((i) => i.id === s0.id)!);
    const s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat?.ticked]).toEqual([s0.done_at, s0.due_at, s0.repeat!.ticked]);
    expect(store.logOf(s0.id).length).toBe(had);
    expect((store.history as unknown as { past: unknown[] }).past.length).toBe(steps);
  });

  it("its time changed while it waits, undone once it is open again: the time and rule back, still open", async () => {
    const { store } = await start();
    const s0 = stretch(store.items);
    const thu = s0.due_at!;
    await store.setDue(s0, thu + 90 * 60_000);
    let s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at]).toEqual([s0.done_at, thu + 90 * 60_000]);
    // Thursday 08:30: open again, at 09:00.
    vi.setSystemTime(new Date(thu + 60 * 60_000));
    await store.reload();
    expect(store.items.find((i) => i.id === s0.id)!.done_at).toBeNull();
    const had = store.logOf(s0.id).length;
    await store.history.undo();
    const back = () => {
      const x = store.items.find((i) => i.id === s0.id)!;
      return [x.done_at, x.due_at, x.repeat?.ticked, x.repeat?.at, x.repeat?.done];
    };
    expect(back()).toEqual([null, thu, undefined, "07:30", s0.repeat!.done]);
    await store.reload();
    expect(back()).toEqual([null, thu, undefined, "07:30", s0.repeat!.done]);
    expect(store.logOf(s0.id).length).toBe(had);
    // Redo: the time change again, still open.
    await store.history.redo();
    s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat?.ticked]).toEqual([null, thu + 90 * 60_000, undefined]);
  });

  it("a time that opened it at once, undone after it would have opened anyway: back at its time, open, never ticked again", async () => {
    const { store } = await start();
    const s0 = stretch(store.items);
    const had = store.logOf(s0.id).length;
    await store.setDue(s0, Date.now() + 3600_000);
    await store.reload();
    expect(store.items.find((i) => i.id === s0.id)!.done_at).toBeNull();
    // Thursday 08:00, past 07:30 when it was to open.
    vi.setSystemTime(new Date(s0.due_at! + 30 * 60_000));
    await store.reload();
    await store.history.undo();
    await store.reload();
    const s = store.items.find((i) => i.id === s0.id)!;
    expect([s.done_at, s.due_at, s.repeat?.ticked, s.repeat?.at, s.repeat?.done]).toEqual([null, s0.due_at, undefined, "07:30", s0.repeat!.done]);
    expect(store.logOf(s0.id).length).toBe(had);
  });

  it("a change saved from an open copy from before its tick: its Undo takes back that change alone", async () => {
    const { store } = await start();
    const s0 = stretch(store.items);
    const before: Item = { ...s0, done_at: null, due_at: s0.repeat!.ticked!.due, repeat: untickedRule(s0.repeat!) };
    store.upsertItem(before);
    await store.editItem(store.items.find((i) => i.id === s0.id)!, "Stretch, 15 min");
    let s = store.items.find((i) => i.id === s0.id)!;
    expect([s.text, s.done_at, s.due_at, s.repeat?.ticked]).toEqual(["Stretch, 15 min", s0.done_at, s0.due_at, s0.repeat!.ticked]);
    // (Its "done", time and rule as saved were never that change's: the Undo leaves them, also
    // once it is open again, on Thursday.)
    vi.setSystemTime(new Date(s0.due_at! + 30 * 60_000));
    await store.reload();
    await store.history.undo();
    await store.reload();
    s = store.items.find((i) => i.id === s0.id)!;
    expect([s.text, s.done_at, s.due_at, s.repeat?.ticked, s.repeat?.done]).toEqual([s0.text, null, s0.due_at, undefined, s0.repeat!.done]);
  });

  it("a quick second click on its box before the tick came back: still ticked, one Undo", async () => {
    const { store } = await start();
    const w0 = store.items.find((i) => i.text === "Weekly")!;
    const past = () => (store.history as unknown as { past: unknown[] }).past.length;
    const steps = past();
    const had = store.logOf(w0.id).length;
    const first = store.toggleDone(w0);
    const second = store.toggleDone(store.items.find((i) => i.id === w0.id)!);
    await Promise.all([first, second]);
    await new Promise((r) => setTimeout(r));
    const w = store.items.find((i) => i.id === w0.id)!;
    expect(w.done_at).not.toBeNull();
    expect(w.repeat?.ticked?.due).toBe(w0.due_at);
    expect(past()).toBe(steps + 1);
    expect(store.logOf(w0.id).length).toBe(had + 1);
    // That one Undo unticks it.
    await store.history.undo();
    expect(store.items.find((i) => i.id === w0.id)!).toMatchObject({ done_at: null, due_at: w0.due_at });
    expect(store.logOf(w0.id).length).toBe(had);
  });

  it("put back ticked by an Undo, its steps' rings stop showing, as at a tick", async () => {
    const { store, api } = await start();
    const w0 = store.items.find((i) => i.text === "Weekly")!;
    const step = await api.createItem({ noteId: w0.note_id, kind: "task", text: "Agenda", parentId: w0.id });
    await api.deleteItem(step.id);
    await api.restore([], [{ ...step, ring_state: "ringing", rings: 1 }], []);
    const r = await api.undoTick({ ...w0, done_at: Date.now() }, null, []);
    expect(r.item.done_at).not.toBeNull();
    expect(r.steps.find((x) => x.id === step.id)?.ring_state).toBe("");
  });

  it("Undo of Done once open again never rings at once", async () => {
    const { store } = await start();
    const weekly = store.items.find((i) => i.text === "Weekly")!;
    await store.toggleDone(weekly);
    // Its next Monday, after its time: open again, then the Undo.
    const late = weekly.due_at! + 7 * 86400_000 + 30 * 60_000;
    vi.setSystemTime(new Date(late));
    await store.reload();
    await store.history.undo();
    const w = store.items.find((i) => i.id === weekly.id)!;
    expect(w).toMatchObject({ done_at: null, due_at: weekly.due_at, snooze_until: late + 60_000 });
  });

  it("Redo after an Undo a day later, then Undo again", async () => {
    const { store } = await start();
    const w0 = store.items.find((i) => i.text === "Weekly")!;
    const had = store.logOf(w0.id).length;
    await store.toggleDone(w0);
    let w = store.items.find((i) => i.id === w0.id)!;
    const second = w.due_at!;
    vi.setSystemTime(new Date(second - 2 * 3600_000));
    await store.reload();
    await store.history.undo();
    w = store.items.find((i) => i.id === w0.id)!;
    expect([w.done_at, w.due_at]).toEqual([null, w0.due_at]);
    vi.setSystemTime(new Date(second + 8 * 86400_000));
    await store.history.redo();
    await store.reload();
    await store.history.undo();
    await store.reload();
    w = store.items.find((i) => i.id === w0.id)!;
    expect(w.due_at).toBe(w0.due_at);
    expect(w.repeat?.done).toBe(w0.repeat?.done);
    expect(store.logOf(w0.id).length).toBe(had);
  });
});
