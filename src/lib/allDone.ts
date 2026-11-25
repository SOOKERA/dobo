// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A note whose tasks are all done says so (a big faint check, the card softened, "All done").
import { nextAfterDone } from "./repeat";
import type { Item } from "./types";

/** A repeating task just ticked, before the backend has moved it on (a moment later it is
 * ticked at its next time, `repeat.ticked`, or open again): counted as open, so the note never
 * flashes "All done" when it comes straight back. Once it waits, ticked, for its next day, it
 * is done like any other until then. */
function comesBack(it: Item, now: number): boolean {
  return it.repeat != null && !it.repeat.ticked && it.due_at != null && nextAfterDone(it.repeat, it.due_at, now) != null;
}

/** True when the note has at least one task and every one of them is done. Only its own lines
 * count, the way its "2 to do" line counts them: steps belong to their task (a done task is done
 * whatever its steps say; an open task with every step ticked is still open), and ideas are never
 * to do. Never for the Inbox. */
export function allDone(items: readonly Item[], inbox: boolean, now = Date.now()): boolean {
  if (inbox) return false;
  let any = false;
  for (const it of items) {
    if (it.parent_id != null || it.kind !== "task") continue;
    if (it.done_at == null || comesBack(it, now)) return false;
    any = true;
  }
  return any;
}
