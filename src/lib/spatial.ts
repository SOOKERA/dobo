// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board's arrows go where the notes sit on screen, not in the order they are stored: → to
// the note in the next column across from the line you are on, ↓ past a note's last line to the
// note right below it in the same column, ↑ to the one above (the Inbox strip, across the top,
// is above every note).

export interface Box {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export type Way = "left" | "right" | "up" | "down";

/** (A few pixels of slack: columns line up, but borders and rounding don't always.) */
const SLACK = 4;

const overlapX = (a: Box, b: Box) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > SLACK;
/** How far `y` is from a box's height (0 inside it). */
const offY = (b: Box, y: number) => (y < b.top ? b.top - y : y > b.bottom ? y - b.bottom : 0);

/** Which box an arrow leads to from box `from`; `y` is the height of what has the keyboard (the
 * line, or the note's title). Null when there is nothing that way. */
export function nextBox(boxes: (Box | null)[], from: number, way: Way, y: number): number | null {
  const here = boxes[from];
  if (!here) return null;
  const others = boxes.map((b, i) => ({ b, i })).filter((o): o is { b: Box; i: number } => !!o.b && o.i !== from);
  if (way === "left" || way === "right") {
    // The nearest column that way, then the note in it level with the line (or nearest to it).
    const side = others.filter(({ b }) => (way === "right" ? b.left >= here.right - SLACK : b.right <= here.left + SLACK));
    if (!side.length) return null;
    const edge = way === "right" ? Math.min(...side.map(({ b }) => b.left)) : Math.max(...side.map(({ b }) => b.right));
    const column = side.filter(({ b }) => Math.abs((way === "right" ? b.left : b.right) - edge) <= SLACK);
    return column.reduce((a, c) => (offY(c.b, y) < offY(a.b, y) || (offY(c.b, y) === offY(a.b, y) && c.b.top < a.b.top) ? c : a)).i;
  }
  // Up / down: the nearest one above / below that shares some of this one's width (the leftmost
  // when two start at the same height, as the first row under the Inbox strip does).
  const line = others.filter(({ b }) => overlapX(b, here) && (way === "down" ? b.top >= here.bottom - SLACK : b.bottom <= here.top + SLACK));
  if (!line.length) return null;
  const dist = (b: Box) => (way === "down" ? b.top - here.bottom : here.top - b.bottom);
  return line.reduce((a, c) => (dist(c.b) < dist(a.b) - SLACK || (Math.abs(dist(c.b) - dist(a.b)) <= SLACK && c.b.left < a.b.left) ? c : a)).i;
}
