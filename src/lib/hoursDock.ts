// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Where today's hours wait while a task is carried (HoursRuler.svelte). Picking up a task moves
// nothing: a quiet strip, "Drop here for a time", floats just under the Inbox strip, over the
// top of the notes (never taking room). Carried onto it (or near it) for a moment, it opens into
// the hours, floating there over the notes; carried on down (or back up), they fold away after a
// short moment, so the edge never flickers; back on the strip, they open again.

/** How long the slip rests on (or near) the strip before the hours open: a line carried straight
 * past it, from the Inbox down to a note, goes by without opening them. */
export const OPEN_AFTER = 110;
/** How long the slip stays away from the open hours before they fold. */
export const FOLD_AFTER = 300;
/** How near the strip counts as on it. */
export const NEAR = 12;
/** How far round the open hours the slip can stray and keep them open. */
export const STAY = 24;

/** The gap under the Inbox strip (its margin before the notes: InboxStrip.svelte), at note size 1. */
export const INBOX_GAP = 18;
/** The strip's own gap: under the Inbox, or from the top of the board when the Inbox is out of
 * sight. Small: the strip stays in the Inbox's gap and over the notes' top edge, clear of their
 * titles. */
export const GAP = 4;
/** The room kept free at the bottom of the board: the note sizes and "?" sit there (SizeControl). */
export const FOOT = 56;
/** The board's own padding at its bottom (App.svelte's .scroll), where a sticky place stops. */
export const BOARD_PAD = 24;
/** The open hours' height (HoursRuler's card, its border in). */
export const OPEN_H = 118;
/** The strip's height (its border in). */
export const STRIP_H = 29;

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Whether a point is in a box, or within `pad` of it. */
export function within(b: Box | null | undefined, x: number, y: number, pad = 0): boolean {
  return !!b && x >= b.left - pad && x <= b.right + pad && y >= b.top - pad && y <= b.bottom + pad;
}

export interface Dock {
  /** The hours are open. */
  open: boolean;
  /** The slip has been off the strip since it was picked up: a line picked up right under the
   * strip doesn't open the hours before it has moved off it once. */
  armed: boolean;
  /** Since when the slip has been on the strip (folded) or away from the hours (open). */
  since: number | null;
}

export const DOCK: Dock = { open: false, armed: false, since: null };

/** The dock after the slip is seen in (`inside`) or out of its place: the strip's when folded,
 * the open hours' when open. */
export function dockAt(d: Dock, inside: boolean, now: number): Dock {
  if (!d.open) {
    if (!inside) return d.armed && d.since == null ? d : { open: false, armed: true, since: null };
    if (!d.armed) return d;
    const since = d.since ?? now;
    if (now - since >= OPEN_AFTER) return { open: true, armed: true, since: null };
    return d.since === since ? d : { ...d, since };
  }
  if (inside) return d.since == null ? d : { ...d, since: null };
  const since = d.since ?? now;
  if (now - since >= FOLD_AFTER) return { open: false, armed: true, since: null };
  return d.since === since ? d : { ...d, since };
}

/** When the dock must be looked at again with the pointer still (null: only when it moves). */
export function dockWake(d: Dock): number | null {
  return d.since == null ? null : d.since + (d.open ? FOLD_AFTER : OPEN_AFTER);
}

/** Where the strip sits, in px on screen, at a note size (`zoom`): the place it hangs from comes
 * right after the Inbox's gap; the strip goes up into that gap (`drop`, from the place). Sticky,
 * inside the board's padding (where a sticky place stops): with the Inbox out of sight, the
 * strip stays GAP under the board's top padding, where the notes' own top is (`stick`); with a
 * long Inbox running off the bottom, it stays whole in sight, above the note sizes (`bottom`). */
export function dockPlace(zoom: number): { stick: number; drop: number; bottom: number } {
  const gap = INBOX_GAP * zoom;
  const r = (n: number) => Math.round(n * 100) / 100;
  return { stick: r(gap), drop: r(GAP - gap), bottom: r(STRIP_H + GAP - gap + FOOT - BOARD_PAD) };
}

/** How far the hours rise as they open from a strip whose top is at `top`, so they show whole on
 * the board (`area`: the part of it in sight). Only when there isn't room under the strip (a long
 * Inbox, a short window), clear of the note sizes, and never past the board's top. */
export function dockLift(top: number, area: { top: number; bottom: number }): number {
  const over = top + OPEN_H - (area.bottom - FOOT);
  return Math.max(0, Math.min(over, top - (area.top + GAP)));
}

/** The hours show while a slip lands on one of them; a slip landing on a new note's card leaves
 * them (and the board's dim) as they are. */
export function hourLanding(landings: readonly { kind: "hour" | "new" }[]): boolean {
  return landings.some((l) => l.kind === "hour");
}

/** How far a carried line scrolls the board in one step at `y` (0: not at all): near its top or
 * bottom edge (within `edge`), never over an hour, and on or near the strip or the open hours
 * only above their top (`dockTop`): stuck under the board's top, they would otherwise hold the
 * whole top edge, and nothing above the board's top could be reached. */
export function edgeStep(y: number, area: { top: number; bottom: number }, edge: number, o: { overHour: boolean; atDock: boolean; dockTop: number | null }): number {
  if (o.overHour) return 0;
  if (o.atDock && !(o.dockTop != null && y < o.dockTop)) return 0;
  if (y > area.bottom - edge) return Math.ceil((y - (area.bottom - edge)) / 4);
  if (y < area.top + edge) return -Math.ceil((area.top + edge - y) / 4);
  return 0;
}

/** Whether the carried slip (away from the hours) rides on the pointer's left: its words would
 * cross the panel's right edge (`edge`) on the right, and there is more room on the left. Once there, it
 * comes back only with room to spare (no flicking from side to side at the edge). */
export function slipOnLeft(x: number, w: number, edge: number, was: boolean): boolean {
  const right = edge - SLIP_EDGE - 2 - (x + 14);
  const left = x - 14 - SLIP_EDGE;
  if (w <= right - (was ? 24 : 0)) return false;
  return was || left > right;
}
/** How near the window's edges the carried slip may go. */
export const SLIP_EDGE = 8;
