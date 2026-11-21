// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Where each note sits on the board.
//
// The board is columns. A note's place is a "step": its column (a wide note takes that column
// and the next one) and its turn in the placing sequence. Notes are stacked in sequence order,
// each at the bottom of its own column(s), so within a column the sequence order is the
// top-to-bottom order.
//
// Without a saved arrangement, notes take their saved order and each goes to the column that is
// free the highest (masonry). Once the user drags a note, the board keeps the columns it shows:
// the steps are saved, and moving a note only changes its own column's order (and the column it
// left), never the other columns. New notes go to the column free the highest, at the bottom.

export interface Step {
  id: number;
  c: number;
}

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Arranged {
  pos: Map<number, Box>;
  /** The placing sequence actually used (columns clamped to the current count). */
  seq: Step[];
  height: number;
}

export interface ArrangeInput {
  /** Notes in their saved order. */
  ids: number[];
  wide: (id: number) => boolean;
  height: (id: number) => number;
  cols: number;
  colW: number;
  gap: number;
  /** Saved steps for this column count, if any. */
  saved?: Step[] | null;
}

export function arrange(inp: ArrangeInput): Arranged {
  const { ids, cols, colW, gap } = inp;
  const pos = new Map<number, Box>();
  const seq: Step[] = [];
  const tops = new Array(cols).fill(0);
  const isWide = (id: number) => cols >= 2 && inp.wide(id);

  const put = (id: number, c: number) => {
    const h = inp.height(id);
    if (isWide(id)) {
      c = Math.max(0, Math.min(cols - 2, c));
      const y = Math.max(tops[c], tops[c + 1]);
      pos.set(id, { x: c * (colW + gap), y, h, w: colW * 2 + gap });
      tops[c] = tops[c + 1] = y + h + gap;
    } else {
      c = Math.max(0, Math.min(cols - 1, c));
      pos.set(id, { x: c * (colW + gap), y: tops[c], h, w: colW });
      tops[c] += h + gap;
    }
    seq.push({ id, c });
  };
  const lowestCol = () => {
    let c = 0;
    for (let i = 1; i < cols; i++) if (tops[i] < tops[c] - 1) c = i;
    return c;
  };
  // A wide note takes the two neighbouring columns that are free the highest.
  const widePair = () => {
    let c = 0;
    for (let i = 1; i < cols - 1; i++) if (Math.max(tops[i], tops[i + 1]) < Math.max(tops[c], tops[c + 1]) - 1) c = i;
    return c;
  };
  const holeBig = () => {
    const c = widePair();
    return Math.abs(tops[c] - tops[c + 1]) > 80;
  };

  const present = new Set(ids);
  const listed = new Set<number>();
  if (inp.saved) {
    for (const s of inp.saved) {
      if (!present.has(s.id) || listed.has(s.id)) continue;
      listed.add(s.id);
      put(s.id, s.c);
    }
  }

  // Masonry for everything not placed yet (all notes when nothing is saved). While a wide note
  // would leave a big hole beside it, the next normal notes fill the hole first.
  const rest = ids.filter((id) => !listed.has(id));
  const waiting: number[] = [];
  rest.forEach((id, k) => {
    if (isWide(id)) {
      const moreNormal = rest.slice(k + 1).some((x) => !isWide(x));
      if (holeBig() && moreNormal) waiting.push(id);
      else put(id, widePair());
      return;
    }
    put(id, lowestCol());
    while (waiting.length && !holeBig()) {
      const w = waiting.shift()!;
      put(w, widePair());
    }
  });
  while (waiting.length) {
    const w = waiting.shift()!;
    put(w, widePair());
  }

  return { pos, seq, height: Math.max(0, ...tops) - gap };
}

/** Columns a box covers. */
export function colsOf(b: Box, colW: number, gap: number): number[] {
  const c = Math.round(b.x / (colW + gap));
  return b.w > colW * 1.5 ? [c, c + 1] : [c];
}

/**
 * Moves `id` to column `c` (its left column when wide), between the notes of that column whose
 * middle is above `cy` and those below. Other columns keep their order; only the notes of the
 * column it leaves and the column it joins change place.
 */
export function moveTo(seq: Step[], pos: Map<number, Box>, id: number, c: number, cy: number, colW: number, gap: number, wide: boolean): Step[] {
  const others = seq.filter((s) => s.id !== id);
  const target = wide ? [c, c + 1] : [c];
  let at = -1;
  others.forEach((s, i) => {
    const b = pos.get(s.id);
    if (!b) return;
    if (!colsOf(b, colW, gap).some((k) => target.includes(k))) return;
    if (b.y + b.h / 2 <= cy) at = i;
  });
  // Nothing above it in that column: it goes first in that column, which means just before the
  // column's first note (or anywhere when the column is empty: other columns don't care).
  const next = [...others];
  next.splice(at + 1, 0, { id, c });
  return next;
}

/** Same arrangement? (to skip needless updates). */
export function sameSeq(a: Step[], b: Step[]): boolean {
  return a.length === b.length && a.every((s, i) => s.id === b[i].id && s.c === b[i].c);
}

/** Saved columns for every view and column count ("view:cols" → steps). */
export type SavedSteps = Record<string, Step[]>;

/** The saved columns without a deleted note (null when it wasn't in any). */
export function dropFromSteps(all: SavedSteps | null | undefined, id: number): SavedSteps | null {
  if (!all) return null;
  let changed = false;
  const next: SavedSteps = {};
  for (const [k, v] of Object.entries(all)) {
    const f = v.filter((s) => s.id !== id);
    if (f.length !== v.length) changed = true;
    next[k] = f;
  }
  return changed ? next : null;
}

/** The saved columns without these notes (null when none of them was in any). */
export function dropNotesFromSteps(all: SavedSteps | null | undefined, ids: number[]): SavedSteps | null {
  let out: SavedSteps | null = null;
  for (const id of ids) out = dropFromSteps(out ?? all, id) ?? out;
  return out;
}

/** Undoing a delete: the note goes back to the place it had in each saved arrangement (still
 * there), after the note that was before it (or first). */
export function putBackInSteps(now: SavedSteps | null | undefined, before: SavedSteps, id: number): SavedSteps | null {
  if (!now) return null;
  let changed = false;
  const next: SavedSteps = { ...now };
  for (const [k, old] of Object.entries(before)) {
    const cur = now[k];
    const i = old.findIndex((s) => s.id === id);
    if (!cur || i < 0 || cur.some((s) => s.id === id)) continue;
    const prev = i > 0 ? old[i - 1].id : null;
    const at = prev == null ? 0 : cur.findIndex((s) => s.id === prev) + 1;
    next[k] = [...cur.slice(0, at), old[i], ...cur.slice(at)];
    changed = true;
  }
  return changed ? next : null;
}
