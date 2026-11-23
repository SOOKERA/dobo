// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// WebView2's own right-click menu (Back, Refresh, Save as, Print…) only where it helps: in a
// field or an editable place, for cut, copy, paste and spelling. Elsewhere a right-click (the
// Menu key, Shift+F10) shows Dobo's own menu where it has one (the board's empty space, a note
// colour), which those places open themselves; on selected words or a link, Dobo's Copy menu
// (App: `copyOffer`); else nothing.

/** Fields whose words can be cut, copied or pasted (not a checkbox, a slider or a button). */
const EDITABLE = "textarea, [contenteditable]:not([contenteditable='false']), input:not([type]), input[type='text'], input[type='search'], input[type='url'], input[type='email'], input[type='number'], input[type='tel'], input[type='password'], input[type='time'], input[type='date']";

/** May WebView2's own menu open for a right-click on `target`? */
export function browserMenuAllowed(target: EventTarget | null): boolean {
  return target instanceof Element && !!target.closest(EDITABLE);
}

let fieldsOnly = false;

/** Keeps WebView2's menu to fields (the app window only: the browser preview keeps its menu). */
export function fieldsMenuOnly(doc: Document = document) {
  fieldsOnly = true;
  doc.addEventListener("contextmenu", (e) => {
    if (!browserMenuAllowed(e.target)) e.preventDefault();
  });
}

export interface CopyOffer {
  /** The selected words (empty: none). */
  text: string;
  /** The link right-clicked, if any. */
  link: string | null;
}

/** The words selected, when (`x`, `y`) is on them (within `slack` pixels of one of their lines),
 * else "": words selected somewhere else on the page are never offered for a right-click here.
 * (A right-click from the keyboard, Menu key or Shift+F10, comes at the selection's first line.) */
export function selectedAt(sel: Pick<Selection, "isCollapsed" | "rangeCount" | "getRangeAt" | "toString"> | null, x: number, y: number, slack = 4): string {
  if (!sel || sel.isCollapsed || !sel.rangeCount) return "";
  for (let i = 0; i < sel.rangeCount; i++) {
    for (const r of Array.from(sel.getRangeAt(i).getClientRects())) {
      if (r.width + r.height > 0 && x >= r.left - slack && x <= r.right + slack && y >= r.top - slack && y <= r.bottom + slack) return sel.toString();
    }
  }
  return "";
}

/** Copies `what` itself, without the clipboard API: through a hidden box of its own, never what
 * happens to be selected (Copy link would copy the selected words). The selection and the
 * keyboard are where they were afterwards. */
export function copyThroughPage(what: string, doc: Document = document): boolean {
  const sel = doc.getSelection();
  const kept = sel ? Array.from({ length: sel.rangeCount }, (_, i) => sel.getRangeAt(i)) : [];
  const was = doc.activeElement as HTMLElement | null;
  const box = doc.createElement("textarea");
  box.value = what;
  box.readOnly = true;
  box.setAttribute("aria-hidden", "true");
  box.style.cssText = "position:fixed;left:-9999px;top:0;width:1px;height:1px;opacity:0";
  doc.body.append(box);
  box.select();
  let ok = false;
  try {
    ok = doc.execCommand("copy");
  } catch {
    ok = false;
  }
  box.remove();
  if (sel) {
    sel.removeAllRanges();
    for (const r of kept) sel.addRange(r);
  }
  if (was?.isConnected && was !== doc.body) was.focus({ preventScroll: true });
  return ok;
}

/** What Dobo's Copy menu offers for a right-click on `target` (outside a field, where WebView2's
 * own menu is kept off): the words selected under it (`selectedAt`), the link under it; null
 * when neither (or when WebView2's menu shows anyway: a field, or the browser preview). */
export function copyOffer(target: EventTarget | null, selected: string, on = fieldsOnly): CopyOffer | null {
  if (!on || browserMenuAllowed(target)) return null;
  const a = target instanceof Element ? target.closest<HTMLAnchorElement>("a[href]") : null;
  const link = a?.href && /^(https?|mailto):/i.test(a.href) ? a.href : null;
  const text = selected.trim();
  return text || link ? { text, link } : null;
}

/** Copies `what`: through the clipboard, else through the page (`copyThroughPage`). Whether it
 * was copied. */
export async function copyText(what: string, doc?: Document): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(what);
    return true;
  } catch {
    return copyThroughPage(what, doc);
  }
}

// ---- Dobo's own menu for a right-click on the board ----

/** What a right-click on the board opens: a line's menu (a task's or a step's ⋯ menu, an idea's
 * own short one), a note's (its ⋯ menu), or "none": on the board, but nothing to open there (the
 * Inbox strip's head, an open menu, a link, a field's own menu). null: not on a note or a line
 * (the empty board, with its own menu). */
export type MenuTarget = { kind: "line" | "note"; id: number } | { kind: "none" } | null;

/** The note or the line a right-click on `target` is for. A line's pictures, link and file are
 * the line's (`data-menu-line`); a note's pictures, its title and its buttons are the note's. */
export function menuTargetOf(target: EventTarget | null): MenuTarget {
  if (!(target instanceof Element)) return null;
  // (A field keeps WebView2's menu; an open menu, a link, words: their own.)
  if (browserMenuAllowed(target) || target.closest("[role='dialog'], [role='menu'], [role='alertdialog'], a[href]")) return { kind: "none" };
  const line = target.closest<HTMLElement>("[data-line], [data-menu-line]");
  const id = Number(line?.dataset.line ?? line?.dataset.menuLine);
  if (line && id) return { kind: "line", id };
  const note = target.closest<HTMLElement>("[data-drop-note]");
  if (note && Number(note.dataset.dropNote)) return { kind: "note", id: Number(note.dataset.dropNote) };
  return target.closest(".slot, [data-inbox-strip]") ? { kind: "none" } : null;
}

export interface Point {
  x: number;
  y: number;
}
export interface Room {
  left: number;
  top: number;
  right: number;
  bottom: number;
}
export interface Placed {
  left: number;
  top: number;
  /** Opened above the pointer (no room below). */
  up: boolean;
  /** Where the pointer is on it: what it grows from. */
  ox: number;
  oy: number;
}

/** Where a menu `w` × `h` opened at the pointer `at` goes inside `room` (all in the window's
 * pixels): to the right of the pointer and below it; to the left or above where there is no room
 * (and there is on the other side); else moved in just enough to show whole, `gap` from the
 * edges (a menu taller than the room shows its top). */
export function placeMenu(at: Point, w: number, h: number, room: Room, gap = 8): Placed {
  const toLeft = at.x + w > room.right - gap && at.x - w >= room.left + gap;
  const left = Math.max(room.left + gap, Math.min(toLeft ? at.x - w : at.x, room.right - gap - w));
  const below = at.y + h <= room.bottom - gap;
  const up = !below && at.y - h >= room.top + gap;
  const top = below ? at.y : up ? at.y - h : Math.max(room.top + gap, room.bottom - gap - h);
  const clamp = (v: number, max: number) => Math.max(0, Math.min(v, max));
  return { left, top, up, ox: clamp(at.x - left, w), oy: clamp(at.y - top, h) };
}

/** How tall a menu may be inside `room` (the window's pixels), in its own pixels (`z`: the board's
 * zoom): the room's height less `margin` (the 12 px above and 16 px below the board keeps when
 * it scrolls a menu into view), never under 120. Past it, the menu scrolls inside itself. */
export function menuCap(room: Room, z = 1, margin = 28): number {
  return Math.max(120, (room.bottom - room.top - margin) / (z || 1));
}
