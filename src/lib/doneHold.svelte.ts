// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A note that puts its done tasks at the end ("Move done tasks to the end"): a task just ticked
// shows its tick where it is a moment (HOLD), then glides to the end while the others slide up
// into its place (`glideOrder`). Unticked within that moment, it never moves. The same for steps.
// Switching the note's choice on or off glides its lines too.
import { untrack } from "svelte";
import { doneOrder } from "./doneOrder";
import { GLIDE, reduced } from "./motion";

/** How long a task just ticked keeps its place before it goes to the end. */
export const HOLD = 400;

/** The lines held in their place now, in every list (how many lists hold each): the keys walk
 * them in the order they show (`shownOrder`). */
const holding = new Map<number, number>();
const hold = (id: number) => holding.set(id, (holding.get(id) ?? 0) + 1);
const unhold = (id: number) => {
  const n = (holding.get(id) ?? 0) - 1;
  if (n > 0) holding.set(id, n);
  else holding.delete(id);
};

/** A list in the order it shows: its done lines last when `last`, but a line ticked a moment ago
 * still in its place (keys.ts walks the lines as they are on screen). */
export function shownOrder<T extends { id: number; done_at: number | null }>(list: readonly T[], last: boolean | undefined): T[] {
  return heldOrder(list, last, (id) => holding.has(id));
}

function heldOrder<T extends { id: number; done_at: number | null }>(list: readonly T[], last: boolean | undefined, held: (id: number) => boolean): T[] {
  if (!last || !list.some((i) => i.done_at != null && held(i.id))) return doneOrder(list, last);
  return [...list.filter((i) => i.done_at == null || held(i.id)), ...list.filter((i) => i.done_at != null && !held(i.id))];
}

/** A list's done lines in the order they show (`doneOrder`), a line ticked a moment ago kept in
 * its place for HOLD first. One per list; `stop()` when it goes. */
export class DoneHold {
  /** Each line as last seen: done or not. */
  #seen = new Map<number, boolean>();
  /** Lines just ticked, still in their place (each with its timer). */
  #held = new Map<number, ReturnType<typeof setTimeout>>();
  /** A hold ended: the order is looked at again. */
  #ended = $state(0);

  order<T extends { id: number; done_at: number | null }>(list: readonly T[], last: boolean | undefined): T[] {
    void this.#ended;
    for (const it of list) {
      const done = it.done_at != null;
      const was = this.#seen.get(it.id);
      this.#seen.set(it.id, done);
      if (!done) this.#release(it.id);
      // (Only a tick seen here: a list drawn with its done lines already done holds nothing.)
      else if (last && was === false && !this.#held.has(it.id) && !reduced()) {
        hold(it.id);
        this.#held.set(
          it.id,
          setTimeout(() => {
            this.#held.delete(it.id);
            unhold(it.id);
            this.#ended++;
          }, HOLD),
        );
      }
    }
    // (Lines gone from the list are forgotten: one back later is seen afresh.)
    if (this.#seen.size > list.length) {
      const here = new Set(list.map((i) => i.id));
      for (const id of [...this.#seen.keys()]) if (!here.has(id)) this.#seen.delete(id);
      for (const id of [...this.#held.keys()]) if (!here.has(id)) this.#release(id);
    }
    return heldOrder(list, last, (id) => this.#held.has(id));
  }

  #release(id: number) {
    const t = this.#held.get(id);
    if (t == null) return;
    clearTimeout(t);
    this.#held.delete(id);
    unhold(id);
  }

  stop() {
    for (const id of [...this.#held.keys()]) this.#release(id);
  }
}

/** A list's lines changing places (the same lines, in a new order): each glides from where it
 * was to where it goes, instead of jumping (reduced motion: they just change places). Lines
 * coming or going are left to their own motion. Call while the component is made; `list` is the
 * element holding the lines, `ids` their order. */
export function glideOrder(list: () => HTMLElement | null | undefined, ids: () => readonly number[]) {
  let seen: readonly number[] | null = null;
  let before: Map<Element, number> | null = null;
  let focused: HTMLElement | null = null;
  // (Where each was, just before the page changes, and what had the keyboard in the list.)
  $effect.pre(() => {
    const now = ids();
    untrack(() => {
      before = null;
      focused = null;
      const el = list();
      if (!el || !seen || !sameLines(seen, now)) return;
      const a = document.activeElement;
      if (a instanceof HTMLElement && a !== el && el.contains(a)) focused = a;
      if (reduced()) return;
      before = new Map([...el.children].map((c) => [c, c.getBoundingClientRect().top]));
    });
  });
  // (And where each is now. A line moved in the page loses the keyboard: it is given back.)
  $effect(() => {
    const now = ids();
    untrack(() => {
      const was = before;
      const had = focused;
      before = null;
      focused = null;
      seen = now;
      const a = document.activeElement;
      if (had?.isConnected && a !== had && (a == null || a === document.body)) had.focus({ preventScroll: true });
      const el = list();
      if (!el || !was) return;
      for (const c of el.children as HTMLCollectionOf<HTMLElement>) {
        const from = was.get(c);
        if (from == null) continue;
        // (A glide under way: the new one starts from where the line is seen.)
        for (const a of c.getAnimations()) if (a.id === "glide-order") a.cancel();
        const dy = from - c.getBoundingClientRect().top;
        if (Math.abs(dy) < 1) continue;
        c.animate([{ transform: `translateY(${dy.toFixed(2)}px)` }, { transform: "none" }], { id: "glide-order", duration: 220, easing: GLIDE });
      }
    });
  });
}

/** The same lines, in another order. */
function sameLines(a: readonly number[], b: readonly number[]): boolean {
  if (a.length !== b.length || a.every((x, i) => x === b[i])) return false;
  const s = new Set(a);
  return b.every((x) => s.has(x));
}
