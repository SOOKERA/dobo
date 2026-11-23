// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { nextBox, type Box } from "./spatial";

const box = (left: number, top: number, w: number, h: number): Box => ({ left, right: left + w, top, bottom: top + h });

// The Inbox strip across the top, then two columns stored in the order the audit measured:
// Logitech (left, top), Hub (right, top), Chanel VR (right, 2nd), Runway (left, 2nd), and a wide
// note across both columns at the bottom.
const strip = box(0, 0, 660, 50);
const logitech = box(0, 70, 320, 300);
const hub = box(340, 70, 320, 200);
const chanel = box(340, 290, 320, 260);
const runway = box(0, 390, 320, 200);
const wide = box(0, 610, 660, 120);
const board = [strip, logitech, hub, chanel, runway, wide];

describe("the board's arrows follow the notes on screen", () => {
  it("→ goes across to the next column, level with the line", () => {
    expect(nextBox(board, 1, "right", 100)).toBe(2); // Logitech's first line → Hub
    expect(nextBox(board, 1, "right", 330)).toBe(3); // Logitech's last line → Chanel VR beside it
    expect(nextBox(board, 4, "right", 420)).toBe(3); // Runway → Chanel VR, not down the order
    expect(nextBox(board, 2, "right", 100)).toBe(null); // nothing right of Hub
  });

  it("← comes back to the column on the left", () => {
    expect(nextBox(board, 3, "left", 300)).toBe(1); // Chanel VR's top → Logitech, beside it
    expect(nextBox(board, 3, "left", 500)).toBe(4); // Chanel VR's bottom → Runway
    expect(nextBox(board, 2, "left", 100)).toBe(1);
  });

  it("↓ goes to the note right below in the same column", () => {
    expect(nextBox(board, 2, "down", 0)).toBe(3); // Hub → Chanel VR
    expect(nextBox(board, 1, "down", 0)).toBe(4); // Logitech → Runway, not Hub
    expect(nextBox(board, 4, "down", 0)).toBe(5); // Runway → the wide note
    expect(nextBox(board, 5, "down", 0)).toBe(null);
  });

  it("↓ from the Inbox strip goes to the first note at the left", () => {
    expect(nextBox(board, 0, "down", 0)).toBe(1);
  });

  it("↑ goes to the note above, then the Inbox strip", () => {
    expect(nextBox(board, 3, "up", 0)).toBe(2); // Chanel VR → Hub
    expect(nextBox(board, 4, "up", 0)).toBe(1); // Runway → Logitech
    expect(nextBox(board, 1, "up", 0)).toBe(0); // Logitech → the strip
    expect(nextBox(board, 2, "up", 0)).toBe(0);
    expect(nextBox(board, 0, "up", 0)).toBe(null);
    // The wide note: the nearest one above it, Runway (it ends lowest).
    expect(nextBox(board, 5, "up", 0)).toBe(4);
  });

  it("a note not on screen is left out", () => {
    expect(nextBox([strip, logitech, null, chanel], 1, "right", 100)).toBe(3);
    expect(nextBox([null, logitech], 0, "down", 0)).toBe(null);
  });
});
