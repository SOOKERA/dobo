// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// On ruled and grid paper every row sits on a line (24 px). Blocks that aren't one row high
// (pictures, link cards, a wrapped title) get padded down to the next line.
//
// All the blocks that changed size in a frame are fitted together: every height is read first,
// then every padding written, so the page is laid out once for all of them (not once per block,
// as when each block reset its padding and then measured itself). A block's height is read with
// the room it was given taken off, so nothing needs to be written before reading.
import { store } from "./store.svelte";

export const RHYTHM = 24;

interface Snap {
  node: HTMLElement;
  active: boolean;
  /** The block's own inline padding, before any room was added. */
  base: string;
  /** The room added (px), 0 when none. */
  added: number;
}

const byNode = new WeakMap<Element, Snap>();
const due = new Set<Snap>();
let planned: "frame" | "now" | null = null;

function run() {
  planned = null;
  const list = [...due].filter((s) => s.node.isConnected && (s.active || s.added));
  due.clear();
  fit(list);
}

function fit(list: Snap[]) {
  const zoom = store.zoom || 1;
  // Read everything...
  const heights = list.map((s) => (s.active ? s.node.getBoundingClientRect().height / zoom : 0));
  // (A block easing its own size in or out is seen at a passing size: it was fitted as it came,
  // and is looked at again if its size still changes once it is still.)
  const moving = list.map((s) => s.node.getAnimations().length > 0);
  const want = list.map((s, i) => {
    if (!s.active || moving[i]) return 0;
    const h = heights[i] - s.added;
    return (RHYTHM - (Math.round(h) % RHYTHM)) % RHYTHM;
  });
  const own = list.map((s, i) => (want[i] ? (parseFloat(getComputedStyle(s.node).paddingBottom) || 0) - s.added : 0));
  // ...then write.
  list.forEach((s, i) => {
    if (moving[i]) return;
    const extra = want[i];
    if (extra === s.added) return;
    s.added = extra;
    s.node.style.paddingBottom = extra ? `${extra + own[i]}px` : s.base;
  });
}

/** Fits `s` with the others: in the next frame (it was resized), or right after the current
 * update (it just came, or was turned on or off: before it is first shown). */
function plan(s: Snap, when: "frame" | "now") {
  due.add(s);
  if (planned === "now" || planned === when) return;
  if (when === "now") {
    planned = "now";
    queueMicrotask(run);
  } else {
    planned = "frame";
    requestAnimationFrame(() => planned === "frame" && run());
  }
}

let ro: ResizeObserver | null = null;
const observer = () =>
  (ro ??= new ResizeObserver((entries) => {
    for (const e of entries) {
      const s = byNode.get(e.target);
      if (s && (s.active || s.added)) plan(s, "frame");
    }
  }));

// A block that comes on its own (a link card added to a line, a note unfolding) is fitted at
// once, before it starts easing in: its room opens straight to its fitted height. Many coming
// together (the board drawn, a space shown) are fitted together right after.
let cameNow = 0;
function came(s: Snap) {
  if (cameNow++ === 0) queueMicrotask(() => (cameNow = 0));
  if (cameNow <= 2) fit([s]);
  else plan(s, "now");
}

export function snap(node: HTMLElement, on: boolean) {
  const s: Snap = { node, active: on, base: node.style.paddingBottom, added: 0 };
  byNode.set(node, s);
  observer().observe(node);
  if (on) came(s);
  return {
    update(v: boolean) {
      if (v === s.active) return;
      s.active = v;
      plan(s, "now");
    },
    destroy() {
      ro?.unobserve(node);
      byNode.delete(node);
      due.delete(s);
    },
  };
}
