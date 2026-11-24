// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The Style panel's motion (transform and opacity only; reduced motion = a short crossfade):
// the selection ring glides from chip to chip, a row's title slides out and the new one in,
// swatches are dealt left to right landing with a slight tilt, what follows a section that
// changed height eases to its new place.
import { cubicInOut, cubicOut } from "svelte/easing";

export const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Lands with a slight tilt that settles (a sticky note pressed down). */
const settle = (t: number) => {
  const c = 1.6;
  return 1 + (c + 1) * Math.pow(t - 1, 3) + c * Math.pow(t - 1, 2);
};

/** New swatches, dealt left to right ~30 ms apart. */
export function deal(_node: Element, { i = 0 }: { i?: number } = {}) {
  if (reduced()) return { duration: 120, css: (t: number) => `opacity:${t}` };
  return {
    delay: 60 + i * 30,
    duration: 380,
    css: (t: number) => {
      const e = settle(t);
      const u = 1 - t;
      return `opacity:${Math.min(1, t * 2.2)};transform:translateY(${(12 * (1 - cubicOut(t))).toFixed(2)}px) rotate(${(-6 * u * u).toFixed(2)}deg) scale(${(0.9 + 0.1 * e).toFixed(3)})`;
    },
  };
}

/** Old swatches sink and fade. */
export function sink(_node?: Element) {
  if (reduced()) return { duration: 100, css: (t: number) => `opacity:${t}` };
  return { duration: 160, css: (t: number) => `opacity:${t};transform:translateY(${((1 - t) * 6).toFixed(2)}px) scale(${(0.96 + 0.04 * t).toFixed(3)})` };
}

/** A row title: the old one slides up and out, the new one comes in from below. */
export function titleIn(_node?: Element) {
  if (reduced()) return { duration: 120, css: (t: number) => `opacity:${t}` };
  return { delay: 40, duration: 240, easing: cubicOut, css: (t: number) => `opacity:${t};transform:translateY(${((1 - t) * 9).toFixed(2)}px)` };
}
export function titleOut(_node?: Element) {
  if (reduced()) return { duration: 100, css: (t: number) => `opacity:${t}` };
  return { duration: 180, css: (t: number) => `opacity:${t};transform:translateY(${(-(1 - t) * 9).toFixed(2)}px)` };
}

/** A page of the panel slides in over the main one (and back). */
export function pageIn(_node: Element, { back = false }: { back?: boolean } = {}) {
  if (reduced()) return { duration: 140, css: (t: number) => `opacity:${t}` };
  return { duration: 320, easing: cubicOut, css: (t: number) => `opacity:${Math.min(1, t * 1.5)};transform:translateX(${((1 - t) * (back ? -28 : 100)).toFixed(2)}${back ? "px" : "%"})` };
}
export function pageOut(_node: Element, { back = false }: { back?: boolean } = {}) {
  if (reduced()) return { duration: 100, css: (t: number) => `opacity:${t}` };
  return { duration: 260, easing: cubicOut, css: (t: number) => `opacity:${t};transform:translateX(${((1 - t) * (back ? 100 : -28)).toFixed(2)}${back ? "%" : "px"})` };
}

/** The selection ring glides from the chip it was on to the one picked. The grid gets one ring;
 * the picked child carries data-sel. */
export function glideRing(grid: HTMLElement, _key: unknown) {
  const ring = document.createElement("span");
  ring.className = "glide-ring";
  ring.setAttribute("aria-hidden", "true");
  grid.appendChild(ring);
  let first = true;
  const place = () => {
    const picked = grid.querySelector<HTMLElement>(":scope > [data-sel='true']");
    const sel = picked?.querySelector<HTMLElement>("[data-ring]") ?? picked;
    if (!sel) {
      ring.style.opacity = "0";
      return;
    }
    const g = grid.getBoundingClientRect();
    const r = sel.getBoundingClientRect();
    const radius = getComputedStyle(sel).borderRadius;
    ring.style.width = `${r.width}px`;
    ring.style.height = `${r.height}px`;
    ring.style.borderRadius = radius;
    ring.style.transition = first || reduced() ? "none" : "";
    ring.style.transform = `translate(${r.left - g.left + grid.scrollLeft}px, ${r.top - g.top + grid.scrollTop}px)`;
    ring.style.opacity = "1";
    first = false;
  };
  requestAnimationFrame(place);
  const ro = new ResizeObserver(() => {
    first = true;
    place();
  });
  ro.observe(grid);
  return {
    update() {
      requestAnimationFrame(place);
    },
    destroy() {
      ro.disconnect();
      ring.remove();
    },
  };
}

/** When this section changes height, what follows it eases to its new place (instead of
 * jumping), so the panel seems to grow or shrink smoothly. */
export function easeFollowers(node: HTMLElement) {
  let last = node.getBoundingClientRect().height;
  const ro = new ResizeObserver(() => {
    const h = node.getBoundingClientRect().height;
    const d = h - last;
    last = h;
    if (!d || reduced()) return;
    for (let el = node.nextElementSibling; el; el = el.nextElementSibling)
      (el as HTMLElement).animate([{ transform: `translateY(${-d}px)` }, { transform: "none" }], { duration: 280, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
  });
  ro.observe(node);
  return { destroy: () => ro.disconnect() };
}

/** A cubic-bezier(0.3, 0.8, 0.25, 1) look-alike for Svelte transitions (the app's "glide"). */
export const glide = (t: number) => 1 - Math.pow(1 - t, 3.4);
export const GLIDE = "cubic-bezier(0.3, 0.8, 0.25, 1)";

/** A box whose content changes height eases to its new height instead of jumping, so what sits
 * below glides with it (the room opens and closes in step). Use on an outer element whose only
 * child holds the content; that child is measured. Interrupted changes start from where the box
 * is seen. While it moves, the box clips (with a small margin so rings and shadows show), unless
 * `clip` is false: a card of its own that folds away over what rises under it stays whole
 * instead of being cut. Content that is already easing its own height (a card's own
 * transition) is left to it. */
export function easeHeight(node: HTMLElement, { grow = 320, shrink = 260, clip = true }: { grow?: number; shrink?: number; clip?: boolean } = {}) {
  const inner = node.firstElementChild as HTMLElement | null;
  if (!inner) return {};
  const hOf = (el: HTMLElement) => el.getBoundingClientRect().height;
  let last = hOf(inner);
  // What the box adds around its content (its own border and padding, or a margin of the content
  // that stays inside it): kept, so the box eases between its real heights, not its content's.
  let pad = hOf(node) - last;
  let anim: Animation | null = null;
  const ro = new ResizeObserver(() => {
    const h = hOf(inner);
    if (Math.abs(h - last) < 0.5) return;
    const from = anim ? hOf(node) : last + pad;
    if (!anim) pad = hOf(node) - h;
    last = h;
    anim?.cancel();
    anim = null;
    // Content that is already easing its own height (a card arriving or leaving) is followed
    // as it is, not smoothed a second time.
    if (reduced() || !node.isConnected || inner.getAnimations().length) return;
    const to = h + pad;
    if (clip) {
      node.style.overflow = "clip";
      node.style.overflowClipMargin = "8px";
    }
    const a = node.animate([{ height: `${from}px` }, { height: `${to}px` }], { duration: to > from ? grow : shrink, easing: GLIDE });
    anim = a;
    const done = () => {
      // (Another one took over: it keeps the clip. Interrupted with none after it: let go.)
      if (anim && anim !== a) return;
      if (anim === a) anim = null;
      node.style.overflow = "";
      node.style.overflowClipMargin = "";
    };
    a.onfinish = done;
    a.oncancel = done;
  });
  ro.observe(inner);
  return {
    destroy() {
      ro.disconnect();
      anim?.cancel();
    },
  };
}

/** Something stacked in one grid cell with what replaces it: while it leaves it steps out of
 * the flow (so the box can already ease to the newcomer's size) and fades, folding up a little. */
export function leave(_node: Element, { duration = 140, y = -4, scaleY = 1 }: { duration?: number; y?: number; scaleY?: number } = {}) {
  if (reduced()) return { duration: 90, css: (t: number) => `position:absolute;inset:0 0 auto 0;opacity:${t}` };
  return {
    duration,
    easing: cubicOut,
    css: (t: number, u: number) =>
      `position:absolute;inset:0 0 auto 0;opacity:${t.toFixed(3)};transform:translateY(${(u * y).toFixed(2)}px)${scaleY === 1 ? "" : ` scaleY(${(1 - u * (1 - scaleY)).toFixed(4)})`}`,
  };
}
/** ...and the newcomer arrives once the old one is mostly gone (never two strong at once). */
export function arrive(_node: Element, { delay = 90, duration = 240, y = 4, scaleY = 1 }: { delay?: number; duration?: number; y?: number; scaleY?: number } = {}) {
  if (reduced()) return { delay: Math.min(delay, 60), duration: 120, css: (t: number) => `opacity:${t}` };
  return {
    delay,
    duration,
    easing: glide,
    css: (t: number, u: number) => `opacity:${t.toFixed(3)};transform:translateY(${(u * y).toFixed(2)}px)${scaleY === 1 ? "" : ` scaleY(${(1 - u * (1 - scaleY)).toFixed(4)})`}`,
  };
}

/** Something that arrives in a row (a chip, a pill): the room opens sideways (its width and the
 * gap before the next one), then it fades in; it leaves the same way, backwards. */
export function widen(node: Element, { gap = 8, duration = 260, delay = 0 }: { gap?: number; duration?: number; delay?: number } = {}, opts?: { direction?: "in" | "out" | "both" }) {
  const out = opts?.direction === "out";
  const w = (node as HTMLElement).getBoundingClientRect().width;
  if (reduced()) return { duration: out ? 80 : 120, css: (t: number) => `opacity:${t}` };
  return {
    delay: out ? 0 : delay,
    duration: out ? Math.round(duration * 0.8) : duration,
    easing: cubicOut,
    css: (t: number) => {
      const room = Math.min(1, t / 0.6);
      const show = Math.max(0, (t - 0.3) / 0.7);
      return `overflow:hidden;flex-shrink:0;width:${(w * room).toFixed(2)}px;margin-right:${(-gap * (1 - room)).toFixed(2)}px;opacity:${show.toFixed(3)}`;
    },
  };
}

/** A small mark (a dot, a badge, a count) pops in / shrinks away. */
export function blip(_node: Element, { delay = 0, duration = 220 }: { delay?: number; duration?: number } = {}, opts?: { direction?: "in" | "out" | "both" }) {
  const out = opts?.direction === "out";
  if (reduced()) return { duration: out ? 80 : 120, css: (t: number) => `opacity:${t}` };
  return {
    delay: out ? 0 : delay,
    duration: out ? Math.round(duration * 0.7) : duration,
    easing: cubicOut,
    css: (t: number) => `opacity:${Math.min(1, t * 1.6).toFixed(3)};scale:${(0.4 + 0.6 * t).toFixed(4)}`,
  };
}

/** `animate:` for a list whose items keep their size: neighbours glide to their new place
 * (translate only, so text is never stretched). */
export function glideTo(_node: Element, { from, to }: { from: DOMRect; to: DOMRect }, { duration = 260 }: { duration?: number } = {}) {
  const dx = from.left - to.left;
  const dy = from.top - to.top;
  if ((!dx && !dy) || reduced()) return { duration: 0 };
  return { duration, easing: glide, css: (_t: number, u: number) => `transform:translate(${(u * dx).toFixed(2)}px,${(u * dy).toFixed(2)}px)` };
}

/** A grid of lines (the Inbox strip's two columns): when a line comes or goes, the others glide
 * to their new cells instead of jumping. A line on its way out (Svelte makes it inert while its
 * own transition plays) steps out of the flow where it is, so the others can already move into
 * its room. Use on the grid itself (position: relative). */
export function gridGlide(grid: HTMLElement) {
  type At = { x: number; y: number };
  // (Their places in the grid's layout, not as seen: a glide under way doesn't count.)
  const where = () => {
    const m = new Map<Element, At>();
    for (const el of grid.children as HTMLCollectionOf<HTMLElement>) if (!el.inert) m.set(el, { x: el.offsetLeft, y: el.offsetTop });
    return m;
  };
  // (The places are first read when the browser has laid the grid out anyway, as it is first
  // seen: reading them as the grid is made would lay the page out once more, for every line of
  // it, just as the strip unfolds.)
  let last: Map<Element, At> | null = null;
  const mo = new MutationObserver((list) => {
    // (Only lines coming, going or leaving count, not what changes inside them.)
    if (!list.some((r) => r.target === grid || (r.target as Element).parentElement === grid)) return;
    last ??= where();
    for (const rec of list) {
      const el = rec.target as HTMLElement;
      if (rec.type !== "attributes" || el.parentElement !== grid) continue;
      if (el.inert && !el.dataset.out) {
        // Out of the flow, where it is seen, as wide as it was.
        const at = last.get(el);
        el.dataset.out = "1";
        el.style.position = "absolute";
        el.style.left = `${at?.x ?? el.offsetLeft}px`;
        el.style.top = `${at?.y ?? el.offsetTop}px`;
        el.style.width = `${el.offsetWidth}px`;
      } else if (!el.inert && el.dataset.out) {
        // (Its way out was called off: back in its place.)
        delete el.dataset.out;
        el.style.position = el.style.left = el.style.top = el.style.width = "";
      }
    }
    const now = where();
    if (!reduced())
      for (const [el, p] of now) {
        const was = last.get(el);
        if (!was) continue;
        const dx = was.x - p.x;
        const dy = was.y - p.y;
        if (Math.abs(dx) < 1 && Math.abs(dy) < 1) continue;
        (el as HTMLElement).animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: 300, easing: GLIDE });
      }
    last = now;
  });
  // (A line's own `inert` is seen only with the subtree.)
  mo.observe(grid, { childList: true, attributes: true, subtree: true, attributeFilter: ["inert"] });
  // (What moves without a line coming or going, a line growing or the width changing, is taken
  // as it is: nothing glides from an old place.)
  const ro = new ResizeObserver(() => (last = where()));
  ro.observe(grid);
  return {
    destroy() {
      mo.disconnect();
      ro.disconnect();
    },
  };
}

/** A picture fades in once it has loaded (one already in the cache just shows: nothing moves
 * when a note is drawn again). Pair with CSS: `img { opacity: 0 }`, `img.loaded { opacity: 1 }`. */
export function imgFade(node: HTMLImageElement) {
  if (node.complete && node.naturalWidth) {
    node.classList.add("loaded", "instant");
    return {};
  }
  const on = () => node.classList.add("loaded");
  node.addEventListener("load", on, { once: true });
  return { destroy: () => node.removeEventListener("load", on) };
}

/** A mark drawn by hand (a path with pathLength="1": a note's "all done" check): it fades in
 * while its stroke runs from its start to its end, and leaves the same way, backwards, faster. */
export function drawMark(_node: Element, { delay = 0, duration = 620 }: { delay?: number; duration?: number } = {}, opts?: { direction?: "in" | "out" | "both" }) {
  const out = opts?.direction === "out";
  if (reduced()) return { duration: out ? 100 : 140, css: (t: number) => `opacity:${t}` };
  return {
    delay: out ? 0 : delay,
    duration: out ? Math.round(duration * 0.65) : duration,
    easing: cubicInOut,
    css: (t: number) => `opacity:${Math.min(1, t * 2.5).toFixed(3)};stroke-dasharray:1 1;stroke-dashoffset:${(1 - t).toFixed(4)}`,
  };
}

/** A quick fade (markers, hints): `fade` with the app's timing and reduced motion. */
export function fadeQuick(_node: Element, { delay = 0, duration = 140 }: { delay?: number; duration?: number } = {}, opts?: { direction?: "in" | "out" | "both" }) {
  const out = opts?.direction === "out";
  return { delay: out ? 0 : delay, duration: out ? Math.round(duration * 0.75) : duration, css: (t: number) => `opacity:${t.toFixed(3)}` };
}

// ---- Views, dialogs and menus: one family of motion, both ways ----
// Each one leaves the way it came: the same function runs backwards on the way out (Svelte plays
// an outro with t going from 1 to 0), a little faster. Transform and opacity only; reduced motion
// = a short crossfade.

type Move = { x?: number; y?: number; scale?: number; duration?: number; delay?: number };

function move(_node: Element, { x = 0, y = 0, scale = 1, duration = 280, delay = 0 }: Move, opts?: { direction?: "in" | "out" | "both" }) {
  const out = opts?.direction === "out";
  if (reduced()) return { duration: out ? 100 : 140, css: (t: number) => `opacity:${t}` };
  return {
    delay: out ? 0 : delay,
    duration: out ? Math.round(duration * 0.75) : duration,
    easing: cubicOut,
    css: (t: number, u: number) => {
      const s = scale === 1 ? "" : ` scale(${(1 - u * (1 - scale)).toFixed(4)})`;
      return `opacity:${Math.min(1, t * 1.4).toFixed(3)};transform:translate(${(u * x).toFixed(2)}px,${(u * y).toFixed(2)}px)${s}`;
    },
  };
}

/** A full view over the board (Settings, Tidy): comes down a touch and fades in; leaves the same way. */
export const sheet = (n: Element, p: Move = {}, o?: { direction?: "in" | "out" | "both" }) => move(n, { y: -8, duration: 280, ...p }, o);
/** A side panel (Style): slides in from its edge, goes back to it. */
export const side = (n: Element, p: Move = {}, o?: { direction?: "in" | "out" | "both" }) => move(n, { x: 24, duration: 320, ...p }, o);
/** A dialog or a card that opens in place: rises slightly and fades. */
export const rise = (n: Element, p: Move = {}, o?: { direction?: "in" | "out" | "both" }) => move(n, { y: 8, scale: 0.99, duration: 260, ...p }, o);
/** A menu: grows from its anchor (set transform-origin on the menu). */
export const pop = (n: Element, p: Move = {}, o?: { direction?: "in" | "out" | "both" }) => move(n, { y: -4, scale: 0.97, duration: 180, ...p }, o);
/** A veil or a dimmed backdrop. */
export function veil(_node: Element, { duration = 200 }: { duration?: number } = {}, opts?: { direction?: "in" | "out" | "both" }) {
  return { duration: opts?.direction === "out" ? Math.round(duration * 0.75) : duration, css: (t: number) => `opacity:${t}` };
}

/** Breadcrumb text: the old words leave before the new ones arrive, so the two are never both
 * strong (the weaker one stays under 35 % whenever both show). Both sit in the same grid cell. */
export function crumbOut(_node?: Element) {
  if (reduced()) return { duration: 80, css: (t: number) => `opacity:${t}` };
  return { duration: 120, css: (t: number) => `opacity:${t};transform:translateY(${(-(1 - t) * 5).toFixed(2)}px)` };
}
export function crumbIn(_node?: Element) {
  if (reduced()) return { delay: 80, duration: 100, css: (t: number) => `opacity:${t}` };
  return { delay: 110, duration: 200, easing: cubicOut, css: (t: number) => `opacity:${t};transform:translateY(${((1 - t) * 5).toFixed(2)}px)` };
}

/** A card that sits in the column above the board (Now card, look back, a banner): it fades and
 * lifts, and the room it takes opens and closes with it, so the board below glides instead of
 * jumping when it goes. Also rows, sections and lines that appear in place. `room` = the part
 * of the time the room takes to open (1 = all of it, like a fold), `show` = when the content
 * starts to show, `gap` = the flex/grid gap before it in its list (closed with it). */
export function card(
  node: Element,
  { y = -6, duration = 320, room: roomPart = 0.55, show: showFrom = 0.35, gap = 0 }: { y?: number; duration?: number; room?: number; show?: number; gap?: number } = {},
  opts?: { direction?: "in" | "out" | "both" },
) {
  const out = opts?.direction === "out";
  // (Reduced motion only fades: nothing to measure.)
  if (reduced()) return { duration: out ? 100 : 140, css: (t: number) => `opacity:${t}` };
  const cs = getComputedStyle(node);
  const h = parseFloat(cs.height);
  const mt = parseFloat(cs.marginTop);
  const mb = parseFloat(cs.marginBottom);
  const pt = parseFloat(cs.paddingTop);
  const pb = parseFloat(cs.paddingBottom);
  const bt = parseFloat(cs.borderTopWidth);
  const bb = parseFloat(cs.borderBottomWidth);
  return {
    duration: out ? Math.round(duration * 0.8) : duration,
    easing: cubicOut,
    css: (t: number, u: number) => {
      // The room opens in the first part (in) / closes in the last part (out); the card fades
      // and lifts over the rest, so it never shows squashed.
      const room = Math.min(1, t / roomPart);
      const show = Math.max(0, (t - showFrom) / (1 - showFrom));
      return (
        `overflow:clip;height:${(h * room).toFixed(2)}px;margin-top:${(mt * room - gap * (1 - room)).toFixed(2)}px;margin-bottom:${(mb * room).toFixed(2)}px;` +
        `padding-top:${(pt * room).toFixed(2)}px;padding-bottom:${(pb * room).toFixed(2)}px;border-top-width:${(bt * room).toFixed(2)}px;border-bottom-width:${(bb * room).toFixed(2)}px;` +
        `opacity:${show.toFixed(3)};transform:translateY(${(u * y).toFixed(2)}px)`
      );
    },
  };
}

/** A row, a line or a section that appears in place (a task, a step, a link card, a settings
 * line): its room opens over most of the time while it fades in, so neighbours glide; it folds
 * away the same way. */
export const fold = (n: Element, p: { y?: number; duration?: number; room?: number; show?: number; gap?: number } = {}, o?: { direction?: "in" | "out" | "both" }) =>
  card(n, { y: -4, duration: 280, room: 0.85, show: 0.25, ...p }, o);

// ---- The quick tour ----

/** A step of a walk-through: the old words slide out the way you are going and are mostly gone
 * before the new ones slide in from the other side (never two strong at once). Going back runs
 * it the other way. `dir`: 1 forward, -1 back. Both sit in one grid cell. */
export function stepOut(_node: Element, { dir = 1 }: { dir?: number } = {}) {
  if (reduced()) return { duration: 90, css: (t: number) => `position:absolute;inset:0 0 auto 0;opacity:${t}` };
  return { duration: 150, easing: cubicOut, css: (t: number, u: number) => `position:absolute;inset:0 0 auto 0;opacity:${t.toFixed(3)};transform:translateX(${(-dir * u * 18).toFixed(2)}px)` };
}
export function stepIn(_node: Element, { dir = 1 }: { dir?: number } = {}) {
  if (reduced()) return { delay: 60, duration: 140, css: (t: number) => `opacity:${t}` };
  return { delay: 110, duration: 300, easing: glide, css: (t: number, u: number) => `opacity:${t.toFixed(3)};transform:translateX(${(dir * u * 22).toFixed(2)}px)` };
}

export interface Box4 {
  x: number;
  y: number;
  w: number;
  h: number;
}
/** Partway from one box to another (p from 0 to 1). */
export const between = (a: Box4, b: Box4, p: number): Box4 => ({ x: a.x + (b.x - a.x) * p, y: a.y + (b.y - a.y) * p, w: a.w + (b.w - a.w) * p, h: a.h + (b.h - a.h) * p });
