// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Arrow keys between the choices of a row or a grid (style cards, theme cards, mixes, colour
// swatches, background chips, the segmented choices): Left / Right step to the previous / next,
// Up / Down to the one above / below (by where it sits on screen), Home / End to the first /
// last. Focus moves; Enter or Space picks, as with any button.

/** Containers whose buttons are one set of choices. */
const SETS = ".grid3, .swatches, .papers, .seg, .tabs, .days, .modes, .opts, .chips, .bgchips, .row-sw, [role='radiogroup'], [data-grid]";

function typing(el: Element | null): boolean {
  const a = el as HTMLElement | null;
  return !!a && (a.tagName === "INPUT" || a.tagName === "TEXTAREA" || a.isContentEditable);
}

/** The choices of a set: its buttons that belong to it (not to a set inside it), not the ⋯
 * buttons, not disabled, visible. */
export function choicesOf(set: Element): HTMLElement[] {
  return [...set.querySelectorAll<HTMLElement>("button")].filter(
    (b) => b.closest(SETS) === set && !b.matches(".more, .x, [disabled]") && b.offsetParent !== null,
  );
}

/** Which choice an arrow goes to, from the boxes of the choices (pure, for tests). */
export function nextIndex(boxes: { x: number; y: number }[], from: number, key: string): number {
  const n = boxes.length;
  if (key === "ArrowRight") return Math.min(n - 1, from + 1);
  if (key === "ArrowLeft") return Math.max(0, from - 1);
  if (key === "Home") return 0;
  if (key === "End") return n - 1;
  const here = boxes[from];
  const down = key === "ArrowDown";
  // The nearest row above / below, then the closest one in it.
  const rows = boxes.map((b, i) => ({ ...b, i })).filter((b) => (down ? b.y > here.y + 4 : b.y < here.y - 4));
  if (!rows.length) return from;
  const rowY = down ? Math.min(...rows.map((b) => b.y)) : Math.max(...rows.map((b) => b.y));
  const row = rows.filter((b) => Math.abs(b.y - rowY) <= 4);
  return row.reduce((a, b) => (Math.abs(b.x - here.x) < Math.abs(a.x - here.x) ? b : a)).i;
}

/** Handles an arrow / Home / End on a focused choice; true when it moved focus. */
export function gridKey(e: KeyboardEvent): boolean {
  if (e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return false;
  if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) return false;
  const a = document.activeElement as HTMLElement | null;
  if (!a || typing(a) || a.tagName !== "BUTTON") return false;
  const set = a.closest(SETS);
  if (!set) return false;
  const all = choicesOf(set);
  const from = all.indexOf(a);
  if (from < 0 || all.length < 2) return false;
  const boxes = all.map((b) => {
    const r = b.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  });
  const to = nextIndex(boxes, from, e.key);
  if (to === from) return true;
  all[to].focus();
  all[to].scrollIntoView({ block: "nearest" });
  return true;
}
