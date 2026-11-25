// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A task just ticked in a note that puts its done tasks at the end: it keeps its place a moment,
// and the keys walk the lines in that order too, as they are on screen.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DoneHold, HOLD, shownOrder } from "./doneHold.svelte";

type Line = { id: number; done_at: number | null };
const ids = (l: readonly Line[]) => l.map((i) => i.id);

describe("a task just ticked keeps its place a moment", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal("window", { matchMedia: () => ({ matches: false }) });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("the keys follow the order on screen while it holds, then the final one", () => {
    const hold = new DoneHold();
    const open: Line[] = [{ id: 1, done_at: null }, { id: 2, done_at: null }, { id: 3, done_at: null }];
    expect(ids(hold.order(open, true))).toEqual([1, 2, 3]);
    const ticked: Line[] = [{ id: 1, done_at: 5 }, ...open.slice(1)];
    expect(ids(hold.order(ticked, true))).toEqual([1, 2, 3]);
    expect(ids(shownOrder(ticked, true))).toEqual([1, 2, 3]);
    vi.advanceTimersByTime(HOLD);
    expect(ids(hold.order(ticked, true))).toEqual([2, 3, 1]);
    expect(ids(shownOrder(ticked, true))).toEqual([2, 3, 1]);
    hold.stop();
  });

  it("a list gone stops holding its lines", () => {
    const hold = new DoneHold();
    hold.order([{ id: 7, done_at: null }, { id: 8, done_at: null }], true);
    const ticked: Line[] = [{ id: 7, done_at: 5 }, { id: 8, done_at: null }];
    hold.order(ticked, true);
    expect(ids(shownOrder(ticked, true))).toEqual([7, 8]);
    hold.stop();
    expect(ids(shownOrder(ticked, true))).toEqual([8, 7]);
  });

  it("a line gone from the list is forgotten: back already done, it holds nothing", () => {
    const hold = new DoneHold();
    hold.order([{ id: 1, done_at: null }, { id: 2, done_at: null }], true);
    hold.order([{ id: 1, done_at: null }], true);
    // (Back, ticked elsewhere meanwhile: drawn done, at the end at once.)
    expect(ids(hold.order([{ id: 2, done_at: 5 }, { id: 1, done_at: null }], true))).toEqual([1, 2]);
    hold.stop();
  });
});
