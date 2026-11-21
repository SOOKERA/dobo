// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// One meaning for Esc everywhere: it closes the topmost thing that is open (a menu, the
// picture viewer, search, tidying, settings). With nothing open, a docked panel slides away.
// Esc never throws away what you typed: inline edits keep their text, and the entry line and
// the quick dump bar keep their draft for next time.
//
// Back / Forward (the mouse's side buttons, Alt+Left / Alt+Right, Backspace outside a text
// field) act on the same stack: a view with pages of its own (Style, Settings) goes up or down
// one level; anything else (a menu, a dialog, a view without pages) simply closes on Back.

export interface Nav {
  back: () => void;
  forward?: () => void;
}
interface Layer {
  close: () => void;
  nav?: Nav;
  /** A card beside the board (look back, a reminder's "later" choices): the board's own keys
   * still work while it shows. */
  beside?: boolean;
}
const stack: Layer[] = [];

/** Registers something that Esc closes (and, with `nav`, how it goes back and forward);
 * returns the function to call when it closes. */
export function pushLayer(close: () => void, nav?: Nav, opts: { beside?: boolean } = {}): () => void {
  const l: Layer = { close, nav, beside: opts.beside };
  stack.push(l);
  return () => {
    const i = stack.lastIndexOf(l);
    if (i >= 0) stack.splice(i, 1);
  };
}

/** Closes the topmost layer. Returns false when nothing was open. */
export function closeTopLayer(): boolean {
  const l = stack.pop();
  if (!l) return false;
  l.close();
  return true;
}

export function layerCount(): number {
  return stack.length;
}

/** Is something open over the board (not just a card beside it)? */
export function boardCovered(): boolean {
  return stack.some((l) => !l.beside);
}

/** Back one level in whatever is on top. False when nothing is open (the board stays as it is). */
export function layerBack(): boolean {
  const l = stack[stack.length - 1];
  if (!l) return false;
  if (l.nav) l.nav.back();
  else closeTopLayer();
  return true;
}

/** Forward again, if what is on top has somewhere to go. */
export function layerForward(): boolean {
  const l = stack[stack.length - 1];
  if (!l?.nav?.forward) return false;
  l.nav.forward();
  return true;
}
