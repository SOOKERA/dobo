// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Each space's one Inbox: its first note called "Inbox" that isn't in the archive (the backend
// keeps the same rule: db.rs `FIRST_INBOXES`). A second note called "Inbox" (an Inbox archived
// before 1.0 and put back, a note given that name) is a plain note on the board.
import { inMemory } from "./sample";
import type { Item, Note, Picture, Space } from "./types";

/** Could be a space's Inbox: a note called "Inbox" underneath, not in the archive. */
export const isInbox = (n: Pick<Note, "title" | "archived">): boolean => n.title === "Inbox" && !n.archived;

/** Each space's Inbox (its id), for the spaces that have one. The tour's sample Inbox (in memory,
 * made only for a space without one) is one only while the space has no saved one. */
export function firstInboxes(notes: readonly Note[]): Map<Space, number> {
  const first = new Map<Space, number>();
  for (const n of notes) {
    if (!isInbox(n)) continue;
    const f = first.get(n.space);
    if (f == null || (inMemory(f) && !inMemory(n.id)) || (inMemory(f) === inMemory(n.id) && n.id < f)) first.set(n.space, n.id);
  }
  return first;
}

/** The same among the saved notes only: what the backend sees (whether a space keeps its Inbox
 * is checked on these; the tour's sample never makes one be made). */
export const savedInboxes = (notes: readonly Note[]): Map<Space, number> => firstInboxes(notes.filter((n) => !inMemory(n.id)));

/** The spaces that had their Inbox before a change and have none after it (an undo archived,
 * renamed or moved it, a redo deleted it). Another note called "Inbox" left in the space takes
 * its place by itself: that space isn't one of these. */
export function inboxesLost(before: ReadonlyMap<Space, number>, after: ReadonlyMap<Space, number>): Space[] {
  return [...before.keys()].filter((space) => !after.has(space));
}

/** The lines (top level: their steps follow them) and loose pictures a note holds now that it
 * didn't hold then. */
export function arrivedSince(
  items: readonly Item[],
  pictures: readonly Picture[],
  noteId: number,
  then: { items: ReadonlySet<number>; pictures: ReadonlySet<number> },
): { items: Item[]; pictures: Picture[] } {
  return {
    items: items.filter((i) => i.note_id === noteId && i.parent_id == null && !then.items.has(i.id) && !inMemory(i.id)),
    pictures: pictures.filter((p) => p.note_id === noteId && p.item_id == null && !then.pictures.has(p.id) && !inMemory(p.id)),
  };
}
