// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A note whose tasks are all done says so: when it does.
import { describe, expect, it } from "vitest";
import { allDone } from "./allDone";
import type { Item, Repeat } from "./types";

const blankItem = (over: Partial<Item>): Item => ({
  id: 0, note_id: 1, kind: "task", text: "", due_at: null, done_at: null, sort: 0, created_at: 1, parent_id: null, alarm: "reminder", rhythm: null, link: null, file: null, repeat: null,
  ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null, ...over,
});
const NOW = new Date(2026, 10, 24, 12).getTime();
const task = (id: number, done: boolean, over: Partial<Item> = {}) => blankItem({ id, kind: "task", text: `Task ${id}`, done_at: done ? NOW - 1000 : null, ...over });
const idea = (id: number, over: Partial<Item> = {}) => blankItem({ id, kind: "idea", text: `Idea ${id}`, ...over });
const weekly: Repeat = { freq: "week", interval: 1, days: [1], monthday: null, ends: { kind: "never" }, done: 0 };

describe("a note says it's all done", () => {
  it("not with no tasks: empty, or ideas only", () => {
    expect(allDone([], false, NOW)).toBe(false);
    expect(allDone([idea(1), idea(2)], false, NOW)).toBe(false);
  });

  it("not while a task is still open", () => {
    expect(allDone([task(1, true), task(2, false)], false, NOW)).toBe(false);
    expect(allDone([task(1, false)], false, NOW)).toBe(false);
  });

  it("when every task is done, one or several, ideas beside them or not", () => {
    expect(allDone([task(1, true)], false, NOW)).toBe(true);
    expect(allDone([task(1, true), task(2, true)], false, NOW)).toBe(true);
    expect(allDone([task(1, true), idea(2), idea(3, { done_at: null })], false, NOW)).toBe(true);
  });

  it("never for the Inbox", () => {
    expect(allDone([task(1, true)], true, NOW)).toBe(false);
  });

  it("counts only the note's own lines, not steps (as its 'to do' count does)", () => {
    // A done task with a step left open: the task is done.
    expect(allDone([task(1, true), task(2, false, { parent_id: 1 })], false, NOW)).toBe(true);
    // An open task with every step ticked: still to do.
    expect(allDone([task(1, false), task(2, true, { parent_id: 1 })], false, NOW)).toBe(false);
    // Steps alone (an idea's steps) make no tasks.
    expect(allDone([idea(1), task(2, true, { parent_id: 1 })], false, NOW)).toBe(false);
  });

  it("not for a ticked repeating task about to come back (no flash before its next time arrives)", () => {
    const due = new Date(2026, 10, 23, 10).getTime();
    expect(allDone([task(1, true, { due_at: due, repeat: weekly })], false, NOW)).toBe(false);
    // Once its next time is made, the ticked one has no rule left and the new one is open.
    expect(allDone([task(1, true, { due_at: due }), task(2, false, { due_at: due + 7 * 864e5, repeat: { ...weekly, done: 1 } })], false, NOW)).toBe(false);
    // A rule that has ended: nothing comes back, so the note is done.
    expect(allDone([task(1, true, { due_at: due, repeat: { ...weekly, ends: { kind: "after", count: 1 } } })], false, NOW)).toBe(true);
  });
});
