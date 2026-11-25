// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { doneOrder } from "./doneOrder";

const steps = [
  { id: 1, done_at: 5 },
  { id: 2, done_at: null },
  { id: 3, done_at: 9 },
  { id: 4, done_at: null },
];

describe("where done tasks show", () => {
  it("where they are, by default: a list of steps keeps its order", () => {
    expect(doneOrder(steps, false).map((s) => s.id)).toEqual([1, 2, 3, 4]);
    expect(doneOrder(steps, undefined).map((s) => s.id)).toEqual([1, 2, 3, 4]);
  });
  it("after the open ones when the note asks for it, each part in its own order", () => {
    expect(doneOrder(steps, true).map((s) => s.id)).toEqual([2, 4, 1, 3]);
  });
});
