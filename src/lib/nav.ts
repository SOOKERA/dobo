// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Pages inside a view (Style › Themes › Edit theme): where you are, the way you came, and the
// pages you just backed out of (so Forward, the mouse's forward button, can go there again).
// Like a browser: going somewhere new forgets the forward pages.

export interface Pages<P> {
  /** The pages above this one, in the order walked (the first is the view's home). */
  trail: P[];
  here: P;
  /** Backed out of, the next one to go forward to last. */
  ahead: P[];
}

export const pagesAt = <P>(here: P, trail: P[] = []): Pages<P> => ({ trail, here, ahead: [] });

/** Opens a page from here. */
export function goTo<P>(n: Pages<P>, p: P): Pages<P> {
  return { trail: [...n.trail, n.here], here: p, ahead: [] };
}

/** Straight up to level `i` of the trail (one level up = the last one). Null when there is no
 * such level (Back from the view's home leaves the view). */
export function upTo<P>(n: Pages<P>, i: number): Pages<P> | null {
  if (i < 0 || i >= n.trail.length) return null;
  // The pages left behind, so Forward walks them down again one at a time.
  const left = [...n.trail.slice(i + 1), n.here];
  return { trail: n.trail.slice(0, i), here: n.trail[i], ahead: [...n.ahead, ...left.reverse()] };
}

export const back = <P>(n: Pages<P>) => upTo(n, n.trail.length - 1);

/** Forward to the page just backed out of; null when there is none. */
export function forward<P>(n: Pages<P>): Pages<P> | null {
  if (!n.ahead.length) return null;
  return { trail: [...n.trail, n.here], here: n.ahead[n.ahead.length - 1], ahead: n.ahead.slice(0, -1) };
}

/** The page changed from elsewhere (an import opened Themes): up if it is above us, else a new
 * page under its own parents. */
export function follow<P>(n: Pages<P>, p: P, lineage: (p: P) => P[]): { pages: Pages<P>; up: boolean } {
  if (p === n.here) return { pages: n, up: false };
  const i = n.trail.indexOf(p);
  if (i >= 0) return { pages: upTo(n, i)!, up: true };
  return { pages: pagesAt(p, lineage(p)), up: false };
}

// ---- One press, one step ----
// The same press can arrive twice by two ways (the mouse button as a page event and again as a
// Windows app command; a keyboard's Browser Back key as a key and as an app command): the second
// one, from the other way, the same direction, within a moment, is ignored. Two quick presses
// that come the same way are two steps. (A held key's repeats are left out where keys are read.)
export type NavSource = "mouse" | "key" | "app";
let last: { source: NavSource; dir: string; at: number } | null = null;
export function once(source: NavSource, dir: string, now: number, gap = 250): boolean {
  if (last && last.source !== source && last.dir === dir && now - last.at < gap) {
    // That pair is done: a third press (whichever way) is a step again.
    last = null;
    return false;
  }
  last = { source, dir, at: now };
  return true;
}
