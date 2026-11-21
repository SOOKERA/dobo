// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// On ruled and grid paper every row sits on a line (24 px). Blocks that aren't one row high
// (pictures, link cards, a wrapped title) get padded down to the next line.
import { store } from "./store.svelte";

export const RHYTHM = 24;

export function snap(node: HTMLElement, on: boolean) {
  let active = on;
  let base: string | null = null;
  const fit = () => {
    if (base == null) base = node.style.paddingBottom;
    node.style.paddingBottom = base;
    if (!active) return;
    const h = node.getBoundingClientRect().height / (store.zoom || 1);
    const extra = (RHYTHM - (Math.round(h) % RHYTHM)) % RHYTHM;
    if (extra) node.style.paddingBottom = `${extra + (parseFloat(getComputedStyle(node).paddingBottom) || 0)}px`;
  };
  const ro = new ResizeObserver(() => requestAnimationFrame(fit));
  ro.observe(node);
  fit();
  return {
    update(v: boolean) {
      active = v;
      fit();
    },
    destroy: () => ro.disconnect(),
  };
}
