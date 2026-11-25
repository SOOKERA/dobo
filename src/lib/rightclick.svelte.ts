// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo's own menu for a right-click on a note or a line (a long press with a finger or a pen
// too: Windows sends the same right-click), and for Shift+F10 or the Menu key on the board's
// selected note or line. It is the same menu as their ⋯: App works out what was right-clicked
// (contextmenu.ts `menuTargetOf`) and asks for it here; the note (Board) or the task (ItemRow)
// opens its own menu, at the pointer, or at its ⋯ from the keyboard. An idea has no ⋯: App shows
// its short menu itself.
import { menuCap, placeMenu, type Point } from "./contextmenu";

export type { Point };

export interface MenuAsk {
  kind: "note" | "line";
  id: number;
  /** Where the pointer was (the window's pixels); null from the keyboard: at its ⋯. */
  at: Point | null;
}

class RightClick {
  /** The menu asked for, until its note or its line takes it. */
  ask = $state.raw<MenuAsk | null>(null);
  /** A note is being carried or widened (Board): no menu opens under it. */
  carrying = $state(false);
}
export const rightClick = new RightClick();

/** Asks a note or a line for its menu (`at`: the pointer, or null for its ⋯ place). */
export function askMenu(kind: MenuAsk["kind"], id: number, at: Point | null) {
  rightClick.ask = { kind, id, at };
  // (Nobody took it, the line not shown: it is forgotten, never opened later by surprise.)
  setTimeout(() => {
    if (rightClick.ask?.kind === kind && rightClick.ask.id === id) rightClick.ask = null;
  }, 250);
}

/** The menu asked of this note or line, taken (undefined when it wasn't asked). */
export function takeMenu(kind: MenuAsk["kind"], id: number): { at: Point | null } | undefined {
  const a = rightClick.ask;
  if (!a || a.kind !== kind || a.id !== id) return undefined;
  rightClick.ask = null;
  return { at: a.at };
}

// ---- The keyboard's right-click (Shift+F10, the Menu key) ----
// Windows sends the page its own right-click for these keys too: the Menu key's when it comes
// back up, Shift+F10's as it goes down. When Dobo opened the menu itself from the key, that
// right-click is the same press, not another one: it is set aside (`swallowed`).
let keyAt = -Infinity;
let opened = -Infinity;

/** Shift+F10 or the Menu key (pressed, or let go)? */
export function isMenuKey(e: KeyboardEvent): boolean {
  return (e.key === "ContextMenu" && !e.ctrlKey && !e.altKey && !e.metaKey) || (e.key === "F10" && e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey);
}
/** Notes the menu keys going down and up (App, on the window). */
export function noteMenuKey(e: KeyboardEvent) {
  if (!isMenuKey(e)) return;
  keyAt = performance.now();
  if (opened > -Infinity) opened = keyAt;
}
/** Dobo opened the menu from the key: the right-click Windows sends for it is set aside. */
export function openedByKey() {
  opened = performance.now();
}
/** Is this right-click the keyboard's (just after Shift+F10 or the Menu key)? */
export function fromKeys(e: MouseEvent): boolean {
  // (Chromium's own sign: no button. The key's time as well, in case.)
  return e.button < 0 || performance.now() - keyAt < 400;
}
/** The right-click Windows sends for a menu key Dobo already answered: true once, then not. */
export function swallowed(): boolean {
  const yes = performance.now() - opened < 400;
  opened = -Infinity;
  return yes;
}

/** A menu opened at the pointer (`use:atPointer={{ at, motion }}`; `at` null: where its CSS puts
 * it, at its ⋯):
 * to the right of it and below, else flipped or moved in to stay whole inside the board (or the
 * window), growing from the pointer; never taller than the board, scrolling inside itself past it. `motion` (the menu's `pop` options, if given) gets the
 * way it comes in: from above, or from below when it opens above the pointer. (In the board's
 * zoom: placed in its own pixels.) */
export function atPointer(node: HTMLElement, opts: { at: Point | null | undefined; motion?: { y?: number }; margin?: number; fit?: unknown }) {
  let motion = opts.motion;
  // (`margin`: what it keeps clear of the board's edges, 28 px by default; `fit`: a card that
  // tightens itself to fit, placed again once it has.)
  let margin = opts.margin;
  const place = (p: Point | null | undefined) => {
    const s = node.style;
    const host = node.offsetParent as HTMLElement | null;
    const h = host?.getBoundingClientRect();
    const z = host?.offsetWidth && h ? h.width / host.offsetWidth : 1;
    const box = (node.closest(".scroll") ?? node.closest(".shell") ?? document.documentElement).getBoundingClientRect();
    const room = {
      left: Math.max(0, box.left),
      top: Math.max(0, box.top),
      right: Math.min(window.innerWidth, box.right),
      bottom: Math.min(window.innerHeight, box.bottom),
    };
    // (Never taller than the board's room, at the pointer or at its ⋯: past it, it scrolls inside
    // itself, so a long menu, a task's with Repeat open, is never cut off.)
    // (Its scroll is in app.css: Svelte puts the inline overflow back as it was once it is in.)
    s.maxHeight = `${menuCap(room, z, margin).toFixed(1)}px`;
    node.dataset.capped = "";
    if (!p) {
      s.left = s.top = s.right = s.transformOrigin = "";
      return;
    }
    if (!host || !h) return;
    const r = placeMenu(p, node.offsetWidth * z, node.offsetHeight * z, room);
    s.right = "auto";
    s.left = `${((r.left - h.left) / z).toFixed(1)}px`;
    s.top = `${((r.top - h.top) / z).toFixed(1)}px`;
    s.transformOrigin = `${(r.ox / z).toFixed(1)}px ${(r.oy / z).toFixed(1)}px`;
    if (motion) motion.y = r.up ? 4 : -4;
  };
  place(opts.at);
  // (Scrolling inside itself: which edges have more past them, for their fade in app.css. Its
  // parts growing, a task's Repeat opening, can make it scroll without it changing size.)
  // (Its padding, for the fades to reach its very edges: sticky stops short of a scroller's padding.)
  const pad = getComputedStyle(node);
  node.style.setProperty("--cap-pad-top", pad.paddingTop || "0px");
  node.style.setProperty("--cap-pad-bottom", pad.paddingBottom || "0px");
  const edges = () => {
    node.toggleAttribute("data-more-up", node.scrollTop > 0.5);
    node.toggleAttribute("data-more-down", node.scrollTop + node.clientHeight < node.scrollHeight - 0.5);
  };
  const grew = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(edges);
  const watch = () => {
    if (!grew) return;
    grew.observe(node);
    for (const c of Array.from(node.children)) grew.observe(c);
  };
  const added = typeof MutationObserver === "undefined" ? null : new MutationObserver(() => (watch(), edges()));
  watch();
  added?.observe(node, { childList: true });
  node.addEventListener("scroll", edges, { passive: true });
  return {
    update(o: typeof opts) {
      motion = o.motion;
      margin = o.margin;
      place(o.at);
      edges();
    },
    destroy() {
      grew?.disconnect();
      added?.disconnect();
      node.removeEventListener("scroll", edges);
    },
  };
}
