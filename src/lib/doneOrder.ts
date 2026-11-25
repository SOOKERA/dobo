// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Where a note's done tasks show: where they are, among the others (the default: a list of steps
// keeps its order), or after the open ones when the note asks for it ("Move done tasks to the
// end" in its ⋯ menu). Only how they show: their own order is never changed.

export function doneOrder<T extends { done_at: number | null }>(list: readonly T[], last: boolean | undefined): T[] {
  if (!last) return list as T[];
  return [...list.filter((i) => i.done_at == null), ...list.filter((i) => i.done_at != null)];
}
