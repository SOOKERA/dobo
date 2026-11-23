// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A view or a dialog over the board takes the keyboard while it shows and gives it back when it
// goes: `use:modal={{ initial: "#tidy-text" }}`.
// - On open: what had the keyboard is noted, and the view's first control (or `initial`) takes it.
// - While open: Tab and Shift+Tab go round the view's own controls (and the toast's, which shows
//   over everything), never into the board under it (App also makes the board inert).
// - On close (as soon as it starts to fade): the keyboard goes back to what had it, unless
//   something else took it meanwhile (a view opening in its place). When that is gone, the entry
//   line takes it; when it was nowhere (the board's own keys), it goes back to nowhere.
// Views beside the board (the Style panel) use it with `trap: false`: they take the keyboard and
// give it back, and Tab may go on to the board.

export interface ModalOpts {
  /** What takes the keyboard on open: a selector inside the view, "self" (the view itself, with
   * tabindex="-1"), "none" (it stays where it is), or a function. By default the view's first
   * control. */
  initial?: string | (() => HTMLElement | null | undefined);
  /** Tab stays inside (default true). Without (beside the board), it never takes the keyboard
   * from a view that keeps it. */
  trap?: boolean;
  /** Give the keyboard back on close (default true; a function decides then). */
  restore?: boolean | (() => boolean);
}

interface Open {
  node: HTMLElement;
  opts: ModalOpts;
  was: Element | null;
}
const stack: Open[] = [];

/** Also reachable by Tab from inside a view: the toast (its Undo). */
const ALSO = "[data-modal-also]";

const FOCUSABLE = "button, a[href], input, select, textarea, summary, [tabindex], [contenteditable]:not([contenteditable='false'])";

function shown(el: HTMLElement): boolean {
  if (typeof el.checkVisibility === "function") return el.checkVisibility({ visibilityProperty: true });
  return el.getClientRects().length > 0;
}

/** The controls Tab stops at inside `root`, in order. */
export function tabbables(root: Element): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => {
    if (el.tabIndex < 0 || (el as HTMLButtonElement).disabled || el.closest("[inert]")) return false;
    // (A radio button group is one stop: its checked one.)
    if (el instanceof HTMLInputElement && el.type === "radio" && !el.checked && el.name && root.querySelector(`input[type=radio][name="${CSS.escape(el.name)}"]:checked`)) return false;
    return shown(el);
  });
}

/** Where Tab goes from `from` (its index in `list`, -1 when outside the view): round the list,
 * or null to let the browser take the next one itself (pure, for tests). */
export function tabTarget(count: number, from: number, back: boolean): number | null {
  if (!count) return null;
  if (from < 0) return back ? count - 1 : 0;
  if (back && from === 0) return count - 1;
  if (!back && from === count - 1) return 0;
  return null;
}

/** The view on top that keeps the keyboard (not a card or panel beside the board). */
function topTrap(): Open | undefined {
  for (let i = stack.length - 1; i >= 0; i--) if (stack[i].opts.trap !== false) return stack[i];
  return undefined;
}

/** Is `node` the view on top (the one that has the keyboard)? */
export function isTopModal(node: Element | null | undefined): boolean {
  return !!node && topTrap()?.node === node;
}

function onTab(e: KeyboardEvent) {
  if (e.key !== "Tab" || e.ctrlKey || e.altKey || e.metaKey || e.defaultPrevented) return;
  const top = topTrap();
  if (!top || !top.node.isConnected || top.node.closest("[inert]")) return;
  const list = [...tabbables(top.node), ...[...document.querySelectorAll(ALSO)].flatMap((a) => tabbables(a))];
  const a = document.activeElement as HTMLElement | null;
  const i = a ? list.indexOf(a) : -1;
  // (Inside the view but on something Tab doesn't stop at, like the view itself: the browser goes
  // on from there, which stays inside.)
  if (i < 0 && a && top.node.contains(a) && a !== top.node) return;
  const to = tabTarget(list.length, i, e.shiftKey);
  if (to == null) return;
  e.preventDefault();
  list[to].focus();
}
let listening = false;

/** Takes `el` the keyboard without scrolling (a field also shows where you type). */
function put(el: HTMLElement) {
  el.focus({ preventScroll: true });
}

function pick(node: HTMLElement, opts: ModalOpts): HTMLElement | null {
  const i = opts.initial;
  if (typeof i === "function") return i() ?? null;
  if (i === "self") return node;
  if (i === "none") return null;
  if (i) return node.querySelector<HTMLElement>(i);
  return tabbables(node)[0] ?? null;
}

/** Gives the keyboard back: to `was`, unless something else took it (`node` is what is going). */
export function giveBack(node: HTMLElement, was: Element | null) {
  const a = document.activeElement;
  if (a && a !== document.body && !node.contains(a)) return;
  if (!was || was === document.body) {
    // (It was nowhere, as when the board's own keys were used: back to nowhere.)
    if (a && a !== document.body) (a as HTMLElement).blur();
    return;
  }
  const to = (was as HTMLElement).isConnected && !was.closest("[inert]") ? (was as HTMLElement) : document.getElementById("capture");
  if (to && !to.closest("[inert]")) to.focus({ preventScroll: true });
  else if (a && a !== document.body) (a as HTMLElement).blur();
}

export function modal(node: HTMLElement, opts: ModalOpts = {}) {
  const entry: Open = { node, opts, was: document.activeElement };
  stack.push(entry);
  if (!listening) {
    listening = true;
    document.addEventListener("keydown", onTab, true);
  }
  // Now (the view is in the page, its motion starting); what it shows a frame later, then.
  const take = () => {
    if (entry.opts.initial === "none") return true;
    if (!node.isConnected || node.closest("[inert]")) return false;
    // (Beside the board: not while a view over it has the keyboard. Looked at again a frame
    // later, once: that view may be the one stepping away for it, Tidy before Style opens.)
    if (entry.opts.trap === false && topTrap()) return false;
    if (entry.opts.trap !== false && !isTopModal(node)) return true;
    const el = pick(node, entry.opts);
    if (el) put(el);
    else if (node.hasAttribute("tabindex")) put(node);
    return !!el;
  };
  let frame = take() ? 0 : requestAnimationFrame(() => ((frame = 0), take()));
  let gone = false;
  const go = () => {
    if (gone) return;
    gone = true;
    if (frame) cancelAnimationFrame(frame);
    const i = stack.indexOf(entry);
    if (i >= 0) stack.splice(i, 1);
    const r = entry.opts.restore ?? true;
    // (Once the page has settled: what was under the view is no longer inert then, and a view
    // opening in its place has taken the keyboard. Not if it came back meanwhile: below.)
    if (typeof r === "function" ? r() : r) queueMicrotask(() => gone && giveBack(node, entry.was));
  };
  node.addEventListener("outrostart", go);
  // Opened again while it was fading out: Svelte keeps the same view and turns its fade round
  // (this action doesn't run again), so it is put back on top and takes the keyboard again,
  // with what had it before it first opened still noted.
  const back = () => {
    if (!gone) return;
    gone = false;
    if (!stack.includes(entry)) stack.push(entry);
    if (frame) cancelAnimationFrame(frame);
    frame = take() ? 0 : requestAnimationFrame(() => ((frame = 0), take()));
  };
  node.addEventListener("introstart", back);
  // What had the keyboard went (a question replacing its button, a card removed): it comes back
  // to the view, never to nowhere (which is the board's).
  const fell = (e: FocusEvent) => {
    if (e.relatedTarget || entry.opts.trap === false) return;
    setTimeout(() => {
      const a = document.activeElement;
      if (gone || (a && a !== document.body) || !node.isConnected || node.closest("[inert]") || !isTopModal(node)) return;
      const to = node.hasAttribute("tabindex") ? node : pick(node, entry.opts);
      to?.focus({ preventScroll: true });
    });
  };
  node.addEventListener("focusout", fell);
  return {
    update(o: ModalOpts) {
      entry.opts = o;
    },
    destroy() {
      node.removeEventListener("outrostart", go);
      node.removeEventListener("introstart", back);
      node.removeEventListener("focusout", fell);
      go();
    },
  };
}

/** A popover (a note's menu, a task's menu) closes when Tab takes the keyboard out of it, and Tab
 * goes on: `use:closeOnLeave={onClose}`. (A click elsewhere is the popover's own business: its
 * button toggles it.) */
export function closeOnLeave(node: HTMLElement, close: () => void) {
  let run = close;
  let tabbing = false;
  const key = (e: KeyboardEvent) => {
    if (e.key !== "Tab") return;
    tabbing = true;
    setTimeout(() => (tabbing = false));
  };
  const out = (e: FocusEvent) => {
    const to = e.relatedTarget as Node | null;
    if (tabbing && to && !node.contains(to)) run();
  };
  node.addEventListener("keydown", key);
  node.addEventListener("focusout", out);
  return {
    update(c: () => void) {
      run = c;
    },
    destroy() {
      node.removeEventListener("keydown", key);
      node.removeEventListener("focusout", out);
    },
  };
}

/** A popover opened near the board's lower edge (a note's menu, a task's menu): the board scrolls
 * just enough to show it whole, never so far that its top goes out of sight: `use:keepInView`.
 * Given a value (`use:keepInView={asking}`), it looks again once that changes and the popover has
 * grown (a question unfolding in it). (Measured from its layout size, which the pop it opens
 * with doesn't change; the board's zoom scales it.) */
export function keepInView(node: HTMLElement, _grows?: unknown) {
  const check = () => {
    const sc = node.closest<HTMLElement>(".scroll");
    const host = node.offsetParent as HTMLElement | null;
    if (!sc || !host || !node.isConnected) return;
    const box = sc.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    const z = host.offsetHeight ? h.height / host.offsetHeight : 1;
    const top = h.top + node.offsetTop * z;
    const bottom = top + node.offsetHeight * z;
    const by = Math.min(bottom - (box.bottom - 16), top - box.top - 12);
    if (by > 0) sc.scrollBy({ top: by, behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
  };
  const frame = requestAnimationFrame(check);
  let later = 0;
  return {
    update() {
      clearTimeout(later);
      later = window.setTimeout(check, 300);
    },
    destroy() {
      cancelAnimationFrame(frame);
      clearTimeout(later);
    },
  };
}

/** Takes the keyboard to `el` once it is in the page (a confirmation's safe choice, a field that
 * just opened): `use:takeFocus`. */
export function takeFocus(el: HTMLElement, on: boolean | undefined = true) {
  if (on !== false) queueMicrotask(() => el.isConnected && !el.closest("[inert]") && put(el));
}

/** A field that just opened (a rename) takes the keyboard, its text selected: `use:focusSelect`.
 * (Svelte's `autofocus` only does it when the keyboard is nowhere, never from the button or menu
 * that opened it.) */
export function focusSelect(el: HTMLInputElement) {
  el.focus({ preventScroll: true });
  el.select();
}
