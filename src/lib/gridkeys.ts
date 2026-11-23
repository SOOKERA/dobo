// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Arrow keys between the choices of a row or a grid (style cards, theme cards, mixes, colour
// swatches, background chips, the segmented choices): Left / Right step to the previous / next,
// Up / Down to the one above / below (by where it sits on screen), Home / End to the first /
// last. Focus moves; Enter or Space picks, as with any button. In a radio group too: the arrows
// only move the keyboard, so passing a choice never picks it (each pick is saved, can be undone,
// may move the window to another screen or restyle the whole app); Space or Enter picks. One Tab
// stop per group (`roving`): the choice the keyboard is on, else the chosen one. In a row of tabs (a palette's Soft / Vivid / Custom), which
// only shows another set of choices, the tab follows the arrows.

/** Containers whose buttons are one set of choices. */
const SETS = ".grid3, .swatches, .papers, .seg, .tabs, .days, .modes, .opts, .chips, .bgchips, .row-sw, [role='radiogroup'], [role='tablist'], [data-grid]";

function typing(el: Element | null): boolean {
  const a = el as HTMLElement | null;
  return !!a && (a.tagName === "INPUT" || a.tagName === "TEXTAREA" || a.isContentEditable);
}

/** The choices of a set: its buttons that belong to it (not to a set inside it), not the ⋯
 * buttons, not disabled, visible. */
export function choicesOf(set: Element): HTMLElement[] {
  // (A radio group's choices are its radios, a tab row's its tabs: not a Delete beside one.)
  const only = set.matches("[role='radiogroup']") ? "[role='radio']" : set.matches("[role='tablist']") ? "[role='tab']" : null;
  return [...set.querySelectorAll<HTMLElement>("button")].filter(
    (b) => b.closest(SETS) === set && !b.matches(".more, .x, [disabled]") && (!only || b.matches(only)) && b.offsetParent !== null,
  );
}

/** The radios of a radio group (or the tabs of a tab row) that are its own. */
function ownRadios(group: Element): HTMLElement[] {
  return [...group.querySelectorAll<HTMLElement>("[role='radio'], [role='tab']")].filter((b) => b.closest("[role='radiogroup'], [role='tablist']") === group);
}

/** On a radio group (or a row of tabs): one Tab stop, as WAI-ARIA has it. While the keyboard is
 * in the group, the choice it is on (the arrows go between them, gridKey, without picking), so
 * Tab goes on past the group from there; once the keyboard has left it, the chosen one again
 * (the first that can be chosen while none is). Kept as the choice changes. */
export function roving(group: HTMLElement) {
  const sync = () => {
    const all = ownRadios(group);
    const here = group.ownerDocument.activeElement;
    const on =
      all.find((b) => b === here && !b.hasAttribute("disabled")) ??
      all.find((b) => (b.getAttribute("aria-checked") ?? b.getAttribute("aria-selected")) === "true" && !b.hasAttribute("disabled")) ??
      all.find((b) => !b.hasAttribute("disabled"));
    for (const b of all) {
      const ti = b === on ? 0 : -1;
      if (b.tabIndex !== ti) b.tabIndex = ti;
    }
  };
  sync();
  const mo = new MutationObserver(sync);
  mo.observe(group, { subtree: true, childList: true, attributes: true, attributeFilter: ["aria-checked", "aria-selected", "disabled"] });
  // (Leaving: only once the keyboard is outside the group; focus going from one choice to the
  // next stays in it. Seen after the move, when the document's focus is where it went.)
  const left = (e: FocusEvent) => {
    const to = e.relatedTarget as Node | null;
    if (!to || !group.contains(to)) queueMicrotask(sync);
  };
  group.addEventListener("focusin", sync);
  group.addEventListener("focusout", left);
  return {
    destroy: () => {
      mo.disconnect();
      group.removeEventListener("focusin", sync);
      group.removeEventListener("focusout", left);
    },
  };
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
  // (In a tab row the tab follows: it only shows its panel. A radio waits for Space or Enter.)
  if (all[to].matches("[role='tab']")) all[to].click();
  return true;
}
