// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The Inbox strip's lines, in the order it shows them: by space (Work first, when All shows
// both), then open tasks, done tasks and ideas, as in a note. The board's keys walk them in that
// order too.
import { store } from "./store.svelte";
import type { Item, Picture, Space } from "./types";

export interface InboxGroup {
  space: Space;
  tasks: Item[];
  ideas: Item[];
  pictures: Picture[];
}

const bySort = (a: Item, b: Item) => a.sort - b.sort || a.id - b.id;

export function inboxGroups(): InboxGroup[] {
  const notes = store.stripNotes;
  const spaces = [...new Set(notes.map((n) => n.space))];
  return spaces
    .map((space) => {
      const mine = notes.filter((n) => n.space === space);
      const top = mine.flatMap((n) => store.itemsByNote.get(n.id) ?? []).sort(bySort);
      return {
        space,
        tasks: [...top.filter((i) => i.kind === "task" && i.done_at == null), ...top.filter((i) => i.kind === "task" && i.done_at != null)],
        ideas: top.filter((i) => i.kind === "idea"),
        pictures: mine.flatMap((n) => store.picturesByNote.get(n.id) ?? []),
      };
    })
    .filter((g) => g.tasks.length || g.ideas.length || g.pictures.length);
}

/** Every line the strip shows, steps under their line (open steps first). */
export function inboxLines(): Item[] {
  if (!store.inboxShown) return [];
  const withSteps = (it: Item) => {
    const s = store.stepsOf.get(it.id) ?? [];
    return [it, ...s.filter((x) => x.done_at == null), ...s.filter((x) => x.done_at != null)];
  };
  return inboxGroups().flatMap((g) => [...g.tasks.flatMap(withSteps), ...g.ideas.flatMap(withSteps)]);
}
