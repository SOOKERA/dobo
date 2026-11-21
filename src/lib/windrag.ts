// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The floating window has no title bar of its own: it is moved by its top area, whatever is open.
// The top row of each view (the board's header, Settings, the Archive, Tidy, the picture viewer,
// the first-run window) carries `data-drag-area` while the window can be moved. A press there,
// away from its buttons and fields, starts Windows' own move (so Aero Snap still works). A veil
// laid over the top (Search's, a new note's) carries `data-drag-through`: a press on it reaches
// the area under it, and moving the window doesn't close what the veil belongs to.

export const AREA = "[data-drag-area]";
export const THROUGH = "[data-drag-through]";
/** What stays clickable (or typable) inside a drag area. */
export const CONTROLS = [
  "button",
  "a[href]",
  "input",
  "textarea",
  "select",
  "label",
  "summary",
  "[contenteditable]:not([contenteditable='false'])",
  "[role=button]",
  "[role=link]",
  "[role=menu]",
  "[role=menuitem]",
  "[role=menuitemcheckbox]",
  "[role=menuitemradio]",
  "[role=radio]",
  "[role=radiogroup]",
  "[role=checkbox]",
  "[role=switch]",
  "[role=slider]",
  "[role=spinbutton]",
  "[role=tab]",
  "[role=tablist]",
  "[role=option]",
  "[role=listbox]",
  "[role=combobox]",
  "[role=separator]",
  "[role=textbox]",
  "[role=searchbox]",
  // (Anything reachable by Tab, or picked up and carried, is a control too.)
  "[tabindex]:not([tabindex^='-'])",
  "[draggable=true]",
  "video",
  "audio",
  "iframe",
  "[data-no-drag]",
].join(", ");

/** Does a press on `el` move the window? (In a drag area, and not on one of its controls.) */
export function movesWindow(el: Element | null | undefined): boolean {
  const area = el?.closest(AREA);
  if (!el || !area) return false;
  const c = el.closest(CONTROLS);
  return !c || !area.contains(c);
}

/** What a press at (x, y) lands on, seen through the veils that let a window drag through. */
export function pressedAt(target: Element | null, x: number, y: number, doc: Pick<Document, "elementsFromPoint"> = document): Element | null {
  if (!target?.closest(THROUGH)) return target;
  return doc.elementsFromPoint(x, y).find((e) => !e.closest(THROUGH)) ?? null;
}

/** Does this press (or the click it ends in) move the window? Click-away handlers ignore it. */
export function isWindowDrag(e: MouseEvent, doc: Pick<Document, "elementsFromPoint"> = document): boolean {
  const t = e.target as Element | null;
  if (e.button !== 0 || typeof t?.closest !== "function") return false;
  return movesWindow(pressedAt(t, e.clientX, e.clientY, doc));
}

/** Listens for presses that move the window. Returns the cleanup. */
export function installWindowDrag(start: () => unknown, win: Window = window): () => void {
  const down = (e: PointerEvent) => {
    if (!e.isPrimary || !isWindowDrag(e, win.document)) return;
    // No text selection, no drag of a word or a picture.
    e.preventDefault();
    // (A move refused, the window minimised meanwhile: nothing to do, no stray rejection.)
    void Promise.resolve(start()).catch(() => {});
  };
  const md = (e: MouseEvent) => {
    if (isWindowDrag(e, win.document)) e.preventDefault();
  };
  win.addEventListener("pointerdown", down, true);
  win.addEventListener("mousedown", md, true);
  return () => {
    win.removeEventListener("pointerdown", down, true);
    win.removeEventListener("mousedown", md, true);
  };
}
