// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { nextIndex } from "./gridkeys";

// Three cards per row: 0 1 2 / 3 4 5 / 6 7
const grid = [0, 1, 2, 3, 4, 5, 6, 7].map((i) => ({ x: 50 + (i % 3) * 100, y: 40 + Math.floor(i / 3) * 80 }));

describe("arrow keys between choices", () => {
  it("steps left and right, stopping at the ends", () => {
    expect(nextIndex(grid, 1, "ArrowRight")).toBe(2);
    expect(nextIndex(grid, 2, "ArrowRight")).toBe(3);
    expect(nextIndex(grid, 7, "ArrowRight")).toBe(7);
    expect(nextIndex(grid, 0, "ArrowLeft")).toBe(0);
  });
  it("goes to the one above or below", () => {
    expect(nextIndex(grid, 1, "ArrowDown")).toBe(4);
    expect(nextIndex(grid, 4, "ArrowUp")).toBe(1);
    expect(nextIndex(grid, 5, "ArrowDown")).toBe(7); // the last row is short: the closest one
    expect(nextIndex(grid, 0, "ArrowUp")).toBe(0);
  });
  it("Home and End go to the first and last", () => {
    expect(nextIndex(grid, 4, "Home")).toBe(0);
    expect(nextIndex(grid, 4, "End")).toBe(7);
  });
  it("in a single row, Up and Down stay put", () => {
    const row = [0, 1, 2].map((i) => ({ x: i * 40, y: 10 }));
    expect(nextIndex(row, 1, "ArrowDown")).toBe(1);
  });
});
