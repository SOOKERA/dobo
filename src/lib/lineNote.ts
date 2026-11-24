// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A carried line (a task, an idea, a step, an Inbox line) dropped on the dashed "+ New note" card
// at the end of the board, or on the board's empty space, becomes a new note: titled with its
// words, and the line itself inside it as its first line, whole (its time, steps, link and
// pictures with it). The store makes it (store.noteFromLine), as one step Undo takes back.
import { arrange, type ArrangeInput, type Box } from "./arrange";
import type { Item, Space, SpaceView } from "./types";

/** About how long the new note's title can be (the line inside keeps all its words). */
export const TITLE_MAX = 60;

/** The new note's title: the line's words on one line, cut at about TITLE_MAX characters on a
 * word, with "…" when cut. One long word (a link) is cut inside it. Characters, not UTF-16
 * units: an emoji is never cut in half. */
export function titleFromLine(text: string, max = TITLE_MAX): string {
  const words = text.replace(/\s+/g, " ").trim();
  const chars = [...words];
  if (chars.length <= max) return words;
  const head = chars.slice(0, max + 1).join("");
  const at = head.lastIndexOf(" ");
  // (A word boundary in its second half; earlier would leave too short a title.)
  const cut = at > 0 && [...head.slice(0, at)].length >= max / 2 ? head.slice(0, at) : chars.slice(0, max).join("");
  // (No comma, colon or dash left hanging before the "…".)
  return cut.replace(/[\s,;:.\-–—]+$/u, "") + "…";
}

/** The space the new note goes to: the board's; in "All", the line's own (its note's). */
export function spaceForLine(view: SpaceView, lineSpace: Space | undefined, fallback: Space): Space {
  return view === "all" ? (lineSpace ?? fallback) : view;
}

/** The line as it is once inside the new note: its first line (top level, first in turn), with
 * everything else it had (words, kind, time, repeat, link, file, done or not). Its steps and
 * pictures follow it there (store.ts). */
export function lineInNote(line: Item, noteId: number): Item {
  return { ...line, note_id: noteId, parent_id: null, sort: 0 };
}

/** Where a new note added at the end of the board would sit: the place `arrange` gives it, with
 * the board as it is (the dashed "+ New note" card waits there while a line is carried, so the
 * new note grows in where the card was). Its height is the card's. */
export function nextNoteSlot(inp: ArrangeInput, h: number): Box | null {
  const PH = -1;
  const a = arrange({ ...inp, ids: [...inp.ids, PH], wide: (id) => id !== PH && inp.wide(id), height: (id) => (id === PH ? h : inp.height(id)) });
  return a.pos.get(PH) ?? null;
}
