// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A new note from the board's right-click menu: "New note" puts an empty one where you clicked,
// "New note from a template…" opens the New note window, whose note lands there too. Here: where
// on the board that is (the column clicked, under the note above the spot: the board never
// leaves gaps), its space in "All", and the built-in templates (Today, Ideas, Plan) with the
// blank lines they start with.
import { moveTo, type Box, type Step } from "./arrange";
import { t, type Key } from "./i18n.svelte";
import type { Template, TemplateLine } from "./store.svelte";
import type { Space, SpaceView } from "./types";

/** A point on the board, in its own pixels (before the note-size zoom). */
export interface BoardPoint {
  x: number;
  y: number;
}

/** Where a new note goes for a spot clicked on the board. */
export interface SpotPlace {
  /** Its column: the one clicked, or, when that one is empty, the first empty one from the left
   * (the board never starts with an empty column). */
  c: number;
  /** The column clicked (where it appears, before it glides to its place). */
  clicked: number;
  /** The board's steps with the new note in them, under the notes of that column above the
   * spot, before those below (the other columns as they are). */
  seq: Step[];
  /** The note it sits under in that column (null: it is the column's first). */
  above: number | null;
  /** The note that comes after it in that column (null: it is the last). */
  below: number | null;
}

/** The column under a point: the one it is in, a click in the gap between two going to the
 * nearer one; clamped to the board. */
export function columnAt(x: number, cols: number, colW: number, gap: number): number {
  const c = Math.floor((x + gap / 2) / (colW + gap));
  return Math.max(0, Math.min(cols - 1, c));
}

/** Where a new note (`id`) goes for a spot clicked on the board: in the column clicked, after
 * the notes of that column whose middle is above the spot (as a carried note goes in), so it
 * sits right under the note above, never with a gap; the notes below make room. Clicked in an
 * empty column with an empty one on its left: the first empty column from the left, at its top
 * (the board never leaves a column empty before a note). */
export function spotPlace(seq: Step[], pos: Map<number, Box>, id: number, at: BoardPoint, cols: number, colW: number, gap: number): SpotPlace {
  const clicked = columnAt(at.x, cols, colW, gap);
  // The notes of a column, in the column's order (a wide note covering it counts too).
  const inColumn = (s: Step, c: number) => {
    if (s.id === id) return false;
    const b = pos.get(s.id);
    if (!b) return false;
    const k = Math.round(b.x / (colW + gap));
    return k === c || (b.w > colW * 1.5 && k + 1 === c);
  };
  const empty = (c: number) => !seq.some((s) => inColumn(s, c));
  let c = clicked;
  if (empty(clicked)) for (let k = 0; k < clicked; k++) if (empty(k)) {
    c = k;
    break;
  }
  const next = moveTo(seq, pos, id, c, c === clicked ? at.y : 0, colW, gap, false);
  const i = next.findIndex((s) => s.id === id);
  const inCol = (s: Step) => inColumn(s, c);
  const above = next.slice(0, i).filter(inCol).at(-1)?.id ?? null;
  const below = next.slice(i + 1).find(inCol)?.id ?? null;
  return { c, clicked, seq: next, above, below };
}

/** The new note's space: the board's; in "All", that of the notes around the spot (the one
 * above it, else the one below), else Work. */
export function spaceForSpot(view: SpaceView, above: Space | undefined, below: Space | undefined): Space {
  if (view !== "all") return view;
  return above ?? below ?? "work";
}

// ---- Built-in templates ----

/** How many blank lines of each kind a note waits with (filled in, or taken away with
 * Backspace, one at a time). Never saved: they show while Dobo runs. */
export interface Blanks {
  task: number;
  idea: number;
}
export const NO_BLANKS: Blanks = { task: 0, idea: 0 };

/** The blank lines waiting in a note, each by an id of its own (so filling in or taking away
 * one takes that one, whichever it is, and the others keep their fields as they are). */
export interface BlankIds {
  task: number[];
  idea: number[];
}
export const NO_BLANK_IDS: BlankIds = { task: [], idea: [] };

let blankSeq = 0;
/** Ids for a template's blank lines. */
export function blankIds(b: Blanks): BlankIds {
  const ids = (n: number) => Array.from({ length: n }, () => ++blankSeq);
  return { task: ids(b.task), idea: ids(b.idea) };
}

/** The blanks of every note without one (filled in, or taken away): a note left without any
 * goes from the list. The same list when it wasn't there. */
export function withoutBlank(all: Record<number, BlankIds>, noteId: number, kind: "task" | "idea", id: number): Record<number, BlankIds> {
  const b = all[noteId];
  if (!b?.[kind].includes(id)) return all;
  const next = { ...b, [kind]: b[kind].filter((x) => x !== id) };
  const out = { ...all };
  if (next.task.length || next.idea.length) out[noteId] = next;
  else delete out[noteId];
  return out;
}

/** Where the keyboard goes once a blank is filled in with Enter (the next blank of its kind, else
 * the first one still waiting) or taken away with Backspace (the one before, else the next);
 * null: none of that kind waits. */
export function nextBlank(ids: number[], id: number, how: "enter" | "backspace"): number | null {
  const i = ids.indexOf(id);
  const rest = ids.filter((x) => x !== id);
  if (!rest.length) return null;
  if (i < 0) return rest[0];
  if (how === "enter") return ids[i + 1] ?? rest[0];
  return ids[i - 1] ?? ids[i + 1] ?? null;
}

/** A line of a template left blank (the built-ins' "fill me in" lines). */
export const isBlank = (l: TemplateLine) => !l.text.trim();

/** The blank lines a template starts a note with. */
export function blanksOf(t: Template | null): Blanks {
  const lines = t?.lines.filter(isBlank) ?? [];
  return { task: lines.filter((l) => l.kind === "task").length, idea: lines.filter((l) => l.kind === "idea").length };
}

const blank = (kind: TemplateLine["kind"], n: number): TemplateLine[] => Array.from({ length: n }, () => ({ kind, text: "" }));

interface BuiltIn {
  id: number;
  name: Key;
  /** What it is, under its name in the New note window. */
  about: Key;
  color: string;
  lines: TemplateLine[];
}
// (Ids below zero: a saved template's id is the time it was saved.)
const BUILT_INS: BuiltIn[] = [
  { id: -1, name: "Today", about: "A few tasks for the day", color: "Butter", lines: blank("task", 3) },
  { id: -2, name: "Ideas", about: "Room for a few ideas", color: "Lilac", lines: blank("idea", 3) },
  { id: -3, name: "Plan", about: "A few tasks, then ideas", color: "Sage", lines: [...blank("task", 3), ...blank("idea", 2)] },
];

/** Dobo's own templates, first in the New note window (never hidden, never deleted): named in
 * the language of the moment (a note made from one keeps the name it was given then). */
export function builtInTemplates(words: (k: Key) => string = t): (Template & { about: string })[] {
  return BUILT_INS.map((b) => ({ id: b.id, name: words(b.name), about: words(b.about), palette: "soft", color: b.color, paper: "plain", lines: b.lines.map((l) => ({ ...l })) }));
}

export const isBuiltIn = (id: number | null | undefined) => id != null && id < 0;

// ---- A new note's title, waiting to be typed ----

/** What a new note's waiting title does as it is left: "name" (words were typed), "keep" (Enter
 * with nothing typed, or a click elsewhere in the note: it stays "New note"), "drop" (Esc, or a
 * click away, with nothing typed: the note goes again, as if never made), or "wait" (the window
 * was put aside for another: the field stays, and has the keys when it's back). */
export function freshEnd(how: "enter" | "esc" | "blur", typed: string, opts: { inside?: boolean; windowFocused?: boolean } = {}): "name" | "keep" | "drop" | "wait" {
  if (how === "blur" && opts.windowFocused === false) return "wait";
  if (typed.trim()) return "name";
  if (how === "enter" || (how === "blur" && opts.inside)) return "keep";
  return "drop";
}

/** How soon after a press in the note its title's blur counts as coming from that press. */
const PRESS_MS = 400;
/** The waiting title lost the keyboard to a press inside the note (on its head, its "Empty…"
 * line: places that take no keyboard, so the blur doesn't say where it went), pressed at `at`. */
export function pressedIn(at: number | null, now: number): boolean {
  return at != null && now >= at && now - at < PRESS_MS;
}
