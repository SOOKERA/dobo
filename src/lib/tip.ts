// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// One tooltip for the whole app: a control's name and, when it has one, its shortcut as small
// key chips ("Style  Ctrl Shift S"). The browser's own tooltips (title="…") are slow, plain and
// sometimes don't show at all in the app's web view, so none are used.
//
// A control says what to show with attributes (kept live by Svelte, so a remapped shortcut shows
// as it is now):
//   data-tip="Style"                 the name (falls back to aria-label on icon-only buttons)
//   data-tip-keys="Ctrl+Shift+S"     its shortcut; several ways with "|": "Ctrl+F|/"
//   data-tip-keys="Esc|Alt+Left"     …shown as "Esc  or  Alt Left"
//   data-tip-side="left"             beside the control, on its left (where below would leave
//                                    the window's visible part, like the quick dump bar's)
//
// Timing like a desktop app: after ~400 ms resting on a control; when moving on to a neighbour
// while one is showing it glides there (or shows at once if one just was); gone on leaving,
// pressing, scrolling, typing; back after a press if the pointer rests on the control (and the
// press opened nothing); gone as soon as what holds the control starts to leave.
// Keyboard focus shows it too (focus-visible), a click's focus doesn't. One element, reused;
// placed below the control (above when there's no room), always inside the window.
import { keyName, t as tr } from "./i18n.svelte";

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Where the tooltip goes for a control (pure, for tests): under it, centred, else above; kept
 * `margin` inside the window. `side: "left"`: beside it on its left, centred on it. */
export function placeTip(
  anchor: Box,
  tip: { width: number; height: number },
  view: { width: number; height: number },
  gap = 8,
  margin = 8,
  side: "below" | "left" = "below",
): { x: number; y: number; below: boolean } {
  if (side === "left") {
    const x = Math.max(margin, anchor.left - gap - tip.width);
    const y = Math.max(margin, Math.min(view.height - margin - tip.height, anchor.top + anchor.height / 2 - tip.height / 2));
    return { x: Math.round(x), y: Math.round(y), below: true };
  }
  const cx = anchor.left + anchor.width / 2;
  let x = cx - tip.width / 2;
  x = Math.max(margin, Math.min(view.width - margin - tip.width, x));
  if (tip.width > view.width - 2 * margin) x = margin;
  const under = anchor.top + anchor.height + gap;
  const over = anchor.top - gap - tip.height;
  const below = under + tip.height <= view.height - margin || over < margin;
  let y = below ? under : over;
  y = Math.max(margin, Math.min(view.height - margin - tip.height, y));
  return { x: Math.round(x), y: Math.round(y), below };
}

/** "Ctrl+Shift+S|Ctrl+," → [["Ctrl","Shift","S"],["Ctrl",","]]. A "+" key itself stays ("Ctrl++"). */
export function splitKeys(keys: string | null | undefined): string[][] {
  if (!keys) return [];
  return keys
    .split("|")
    .map((k) => k.trim())
    .filter(Boolean)
    .map((k) => {
      const parts = k.split("+");
      const out: string[] = [];
      for (let i = 0; i < parts.length; i++) {
        // "Ctrl++": an empty part means the + key.
        if (parts[i] === "" && i > 0) {
          out.push("+");
          i++;
        } else out.push(parts[i].trim());
      }
      return out.filter(Boolean);
    });
}

/** What a control's tooltip says, or null (a button whose words already show needs none). */
export function tipOf(el: Element): { label: string; keys: string[][] } | null {
  const label = el.getAttribute("data-tip") ?? (iconOnly(el) ? el.getAttribute("aria-label") : null);
  if (!label) return null;
  return { label, keys: splitKeys(el.getAttribute("data-tip-keys")) };
}

function iconOnly(el: Element): boolean {
  return !(el.textContent ?? "").trim();
}

const TARGET = "[data-tip], button[aria-label], [role='button'][aria-label], a[aria-label]";
const DELAY = 400;
/** After a tooltip goes, the next one within this time shows at once (moving along a toolbar). */
const WARM = 350;

let el: HTMLDivElement | null = null;
let current: Element | null = null;
let timer = 0;
let shownAt = 0;
let hiddenAt = -1e9;
let pressed: Element | null = null;
/** Menus and dialogs open when the pointer went down (a press that opens one isn't re-tipped). */
let popupsAtPress = 0;
const POPUPS = "[role='menu'], [role='dialog'], [role='listbox']";

function node(): HTMLDivElement {
  if (el) return el;
  el = document.createElement("div");
  el.className = "ztip";
  el.setAttribute("role", "tooltip");
  el.setAttribute("aria-hidden", "true");
  // Its words fade with the view's when the language changes.
  el.setAttribute("data-lang-fade", "");
  document.body.appendChild(el);
  return el;
}

function render(t: { label: string; keys: string[][] }) {
  const n = node();
  n.replaceChildren();
  const name = document.createElement("span");
  name.className = "ztip-name";
  name.textContent = t.label;
  n.appendChild(name);
  if (t.keys.length) {
    const ks = document.createElement("span");
    ks.className = "ztip-keys";
    t.keys.forEach((combo, i) => {
      if (i) {
        const or = document.createElement("span");
        or.className = "ztip-or";
        or.textContent = tr("or");
        ks.appendChild(or);
      }
      for (const k of combo) {
        const kbd = document.createElement("kbd");
        kbd.textContent = keyName(k);
        ks.appendChild(kbd);
      }
    });
    n.appendChild(ks);
  }
}

function show(target: Element) {
  const t = tipOf(target);
  if (!t || !target.isConnected || target.closest("[inert]")) return hide();
  const n = node();
  // Moving on to a neighbour while one shows: it stays up and glides over (no dip).
  const glide = current !== null && n.classList.contains("on") && !reducedMotion();
  current = target;
  render(t);
  // One read of the control and the tooltip, one write.
  if (!glide) {
    n.classList.remove("on");
    n.style.transition = "";
    n.style.transform = "translate(-9999px, 0)";
  }
  const a = target.getBoundingClientRect();
  const size = { width: n.offsetWidth, height: n.offsetHeight };
  const side = target.getAttribute("data-tip-side") === "left" ? "left" : "below";
  const p = placeTip(a, size, { width: window.innerWidth, height: window.innerHeight }, 8, 8, side);
  n.classList.toggle("above", !p.below);
  if (glide) n.style.transition = "transform 0.14s cubic-bezier(0.3, 0.8, 0.25, 1), opacity 0.14s ease, translate 0.18s cubic-bezier(0.2, 0.8, 0.2, 1)";
  n.style.transform = `translate(${p.x}px, ${p.y}px)`;
  // Next frame: fade in from a touch lower (or higher).
  if (!glide) requestAnimationFrame(() => current === target && n.classList.add("on"));
  shownAt = performance.now();
}

const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function hideTip() {
  clearTimeout(timer);
  timer = 0;
  if (current) hiddenAt = performance.now();
  current = null;
  el?.classList.remove("on");
}

/** Makes the tooltip's element and lays out a sample once, in an idle moment, so the first
 * tooltip after launch costs no more than the 100th. */
export function warmTip() {
  if (el) return;
  const n = node();
  render({ label: "Style", keys: [["Ctrl", "Shift", "S"]] });
  n.style.transform = "translate(-9999px, 0)";
  void n.offsetWidth;
  n.replaceChildren();
}
const hide = hideTip;
/** Gone on purpose (pressing, scrolling, a drag): the next one waits its full delay again, so
 * tooltips don't flicker on and off under a scrolling list. */
function dismiss() {
  hide();
  hiddenAt = -1e9;
}
/** The last key press: only focus that follows one (Tab, arrows, Esc back to a button) is
 * keyboard focus worth a tooltip, not focus coming back with the window. */
let keyAt = -1e9;

function over(e: PointerEvent) {
  if (e.pointerType === "touch") return;
  // A button held: a drag (a note, a line to the hours, a width) passes over controls; none shows,
  // except drop spots made for that moment (the hours ruler's dots and Tomorrow: `data-tip-drag`).
  const target = e.target as Element | null;
  if (e.buttons && !target?.closest?.("[data-tip-drag]")) {
    if (current || timer) dismiss();
    return;
  }
  const t = target?.closest?.(TARGET) ?? null;
  if (t === current || (t && t === pressed)) return;
  clearTimeout(timer);
  if (!t || !tipOf(t)) {
    if (current) hide();
    return;
  }
  // One showing: it glides over to this one. One just gone: this one shows at once.
  if (current) return show(t);
  const warm = performance.now() - hiddenAt < WARM;
  if (warm) show(t);
  // Still under the pointer then (a list may have scrolled it away meanwhile).
  else timer = window.setTimeout(() => t.matches(":hover") && show(t), DELAY);
}

function out(e: PointerEvent) {
  const to = e.relatedTarget as Element | null;
  const from = (e.target as Element | null)?.closest?.(TARGET) ?? null;
  if (from && to && from.contains(to)) return;
  if (!to || !to.closest?.(TARGET)) {
    if (from === pressed) pressed = null;
    clearTimeout(timer);
    if (current) hide();
  }
}

/** A field being typed in (its focus counts as visible even from a click): no tooltip over it. */
function typing(t: Element): boolean {
  if (t instanceof HTMLTextAreaElement || (t as HTMLElement).isContentEditable) return true;
  return t instanceof HTMLInputElement && !["button", "checkbox", "radio", "range", "color", "submit", "reset", "file"].includes(t.type);
}

/** Starts the tooltips for this window (once). */
let started = false;
export function startTips() {
  if (started) return;
  started = true;
  // Nobody gets the browser's own tooltip on top of ours.
  document.addEventListener("pointerover", over, true);
  document.addEventListener("pointerout", out, true);
  document.addEventListener(
    "pointerdown",
    (e) => {
      pressed = (e.target as Element | null)?.closest?.(TARGET) ?? null;
      popupsAtPress = document.querySelectorAll(POPUPS).length;
      dismiss();
    },
    true,
  );
  // A press done, the pointer still resting on the control: its tooltip comes back after the
  // usual delay (it may say something new: "Pinned"), unless the press opened something.
  document.addEventListener(
    "pointerup",
    (e) => {
      const p = pressed;
      if (!p || e.pointerType === "touch") return;
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        if (pressed !== p || !p.isConnected || !p.matches(":hover")) return;
        if (p.getAttribute("aria-expanded") === "true" || document.querySelectorAll(POPUPS).length > popupsAtPress) return;
        pressed = null;
        show(p);
      }, DELAY + 200);
    },
    true,
  );
  document.addEventListener(
    "keydown",
    () => {
      keyAt = performance.now();
      // A hover's tooltip still waiting: typing cancels it.
      if (!current) {
        clearTimeout(timer);
        timer = 0;
      } else if (keyAt - shownAt > 30) hide();
    },
    true,
  );
  document.addEventListener("scroll", () => (current || timer) && dismiss(), true);
  document.addEventListener("wheel", () => (current || timer) && dismiss(), { capture: true, passive: true });
  window.addEventListener("blur", dismiss);
  // Hidden to the tray or the edge: nothing left showing for when it comes back.
  document.addEventListener("visibilitychange", () => document.hidden && dismiss());
  // Keyboard focus shows it; a click's focus doesn't.
  document.addEventListener(
    "focusin",
    (e) => {
      const t = (e.target as Element | null)?.closest?.(TARGET) ?? null;
      if (!t || !(e.target as Element).matches(":focus-visible") || typing(e.target as Element)) return;
      if (performance.now() - keyAt > 600) return;
      clearTimeout(timer);
      timer = window.setTimeout(() => show(t), current ? 0 : 120);
    },
    true,
  );
  document.addEventListener("focusout", (e) => (e.target as Element | null)?.closest?.(TARGET) === current && hide(), true);
  // A control removed while its tooltip shows, or what holds it starting to leave (a panel or a
  // menu closing: Svelte makes it inert): the tooltip goes too.
  new MutationObserver(() => current && (!current.isConnected || current.closest("[inert]")) && hide()).observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["inert"],
  });
  // The language changed while one shows: it says it again in the new words (still faded out).
  window.addEventListener("dobo-language", () => current && show(current));
  // Ready before it's first needed.
  const idle = (fn: () => void) => ("requestIdleCallback" in window ? requestIdleCallback(fn, { timeout: 3000 }) : setTimeout(fn, 1500));
  idle(warmTip);
}
