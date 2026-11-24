// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A double-click that lands on a button (or another control) inside a header belongs to that
// control: it just does its own thing twice (fold, then unfold). Only a double-click on the
// header itself, its title or its plain text, starts renaming.

type Closest = { closest?: (selector: string) => unknown };

const CONTROLS = "button, a, input, textarea, select, [role=button], [role=menuitem], [contenteditable]";

/** True when the double-click came from a control inside `within` (or from `within` being one). */
export function fromControl(e: { target: EventTarget | null }, within?: unknown): boolean {
  const target = e.target as Closest | null;
  if (!target || typeof target.closest !== "function") return false;
  const hit = target.closest(CONTROLS);
  return !!hit && hit !== within;
}
