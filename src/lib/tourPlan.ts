// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The quick tour's plan: its steps, in the order of a first day (dump a thought from anywhere →
// it waits in the Inbox → sort it into notes → give it a time as you type, or a deadline with !
// → or drag it onto the hours → or onto a bigger task, as one of its steps → the board's look,
// in Style → the "?", where the tour, the keys, the Archive and Help always are), where its
// bubble goes beside what a step points at, and the way from the Style button down to the "?".
// Nothing here touches the page.
import { between, glide, type Box4 } from "./motion";

export const STEPS = ["dump", "inbox", "tidy", "time", "ruler", "step", "style", "more"] as const;
export type StepId = (typeof STEPS)[number];

/** Where the bubble would rather sit: under what the step points at, above it, or (nothing to
 * point at) in the middle. */
export type Prefer = "below" | "above" | "center";

export const PREFER: Record<StepId, Prefer> = {
  dump: "center",
  inbox: "below",
  tidy: "below",
  time: "below",
  ruler: "below",
  step: "below",
  style: "below",
  more: "above",
};

/** The bubble's top-left corner inside a window W × H, beside `t` (null: nothing to point at).
 * It keeps a 16 px margin from the edges and a 12 px gap from what it points at; when neither
 * above nor below has room, it goes beside, else over the bottom of the window. `fitH`: the
 * height the side is chosen for (while the bubble's words change and its height eases, the
 * taller of the two, so it doesn't swap sides back and forth on the way). */
export function placeBubble(t: Box4 | null, bw: number, bh: number, W: number, H: number, prefer: Prefer, fitH = bh, m = 16, gap = 12): { x: number; y: number } {
  const clampX = (x: number) => Math.max(m, Math.min(W - m - bw, x));
  const clampY = (y: number) => Math.max(m, Math.min(H - m - bh, y));
  if (!t || prefer === "center") return { x: clampX((W - bw) / 2), y: clampY((H - bh) * 0.44) };
  const cx = clampX(t.x + t.w / 2 - bw / 2);
  const below = t.y + t.h + gap;
  const above = t.y - gap - bh;
  const fh = Math.max(bh, fitH);
  const fits = { below: below + fh <= H - m, above: t.y - gap - fh >= m };
  for (const side of prefer === "above" ? (["above", "below"] as const) : (["below", "above"] as const)) {
    if (fits[side]) return { x: cx, y: side === "below" ? below : above };
  }
  const right = t.x + t.w + gap;
  const left = t.x - gap - bw;
  if (right + bw <= W - m) return { x: right, y: clampY(t.y) };
  if (left >= m) return { x: left, y: clampY(t.y) };
  return { x: cx, y: H - m - bh };
}

/** The part of `a` inside `b` (null when they don't meet). */
export function clip(a: Box4, b: Box4): Box4 | null {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  const r = Math.min(a.x + a.w, b.x + b.w);
  const bot = Math.min(a.y + a.h, b.y + b.h);
  return r - x < 1 || bot - y < 1 ? null : { x, y, w: r - x, h: bot - y };
}

/** The smallest box around both. */
export function union(a: Box4, b: Box4): Box4 {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return { x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y };
}

/** A time a little later today, the way people type it ("2pm" / "14h"); late in the evening,
 * tomorrow morning. */
export function laterWords(now: Date, lang: "en" | "fr"): string {
  const h = now.getHours() + 2;
  if (h >= 23) return lang === "fr" ? "demain 9h" : "tomorrow 9am";
  if (lang === "fr") return `${h}h`;
  return `${h % 12 || 12}${h < 12 ? "am" : "pm"}`;
}

/** What the tour shows at a moment: its window (two when a step lights two things; `b` sits on
 * `a` otherwise), their corners, how much their rings show (`two`: the second's), the bubble's
 * top-left corner and how much it shows. */
export type Pose = { a: Box4; b: Box4; r: number; ring: number; two: number; bx: number; by: number; show: number };

/** Eases in and out (a longer way: it sets off gently and settles). */
const inOut = (t: number) => (t <= 0 ? 0 : t >= 1 ? 1 : t < 0.5 ? 4 * t * t * t : 1 - Math.pow(2 - 2 * t, 3) / 2);
/** The eased part of [a, b] that time u has gone through. */
const part = (u: number, a: number, b: number) => inOut((u - a) / (b - a));
const mix = (u: number, v: number, e: number) => u + (v - u) * e;

/** A box along a soft corner: from `s` sideways first (along the top bar), then down (the side
 * of the panel) to `m`; it never cuts across the board. Its size changes on the way. */
function round(s: Box4, m: Box4, e: number): Box4 {
  const sx = s.x + s.w / 2;
  const sy = s.y + s.h / 2;
  const mx = m.x + m.w / 2;
  const my = m.y + m.h / 2;
  // A quadratic curve with its corner at (mx, sy): it leaves sideways and arrives from above.
  const k = 1 - e;
  const cx = k * k * sx + 2 * k * e * mx + e * e * mx;
  const cy = k * k * sy + 2 * k * e * sy + e * e * my;
  const w = mix(s.w, m.w, e);
  const h = mix(s.h, m.h, e);
  return { x: cx - w / 2, y: cy - h / 2, w, h };
}

/** How the bubble keeps out of the window's way between the Style button and the "?":
 * "aside", it goes down beside the window's way and slides over the corner once the window is
 * under it; "dip" (no room beside: a narrow panel), it fades where it is and comes back over the
 * corner once the window has passed. */
export type Way = "aside" | "dip";

/** The tour at time u (0 → 1) of the way from the Style step's pose `s` (the window on the
 * header's Style button, the bubble under it) to the last step's `m` (the window on the "?" at
 * the board's corner, the bubble over it). Going back runs it from 1 to 0. `bw`: the bubble's
 * width. The window goes along the top bar and down the side (0 → 0.8); the bubble never has it
 * under it (see Way). */
export function cornerWay(s: Pose, m: Pose, u: number, bw: number, way: Way, gap = 12): Pose {
  const e = part(u, 0, 0.8);
  const a = round(s.a, m.a, e);
  const b = round(s.b, m.b, e);
  const base = { a, b, r: mix(s.r, m.r, e), ring: mix(s.ring, m.ring, e), two: mix(s.two, m.two, e) };
  if (way === "dip") {
    // Out where it is (drifting a touch the way the window goes), in over the corner (arriving
    // from that way), once the window is there.
    if (u < 0.5) {
      const o = part(u, 0, 0.2);
      return { ...base, bx: s.bx, by: s.by + 6 * o, show: s.show * (1 - o) };
    }
    const i = part(u, 0.74, 1);
    return { ...base, bx: m.bx, by: m.by - 6 * (1 - i), show: m.show * i };
  }
  // Down beside the window's way first (left of the side it comes down), then over the corner.
  const keep = Math.min(s.bx, Math.min(m.a.x, m.b.x) - gap - bw);
  const bx = s.bx + (keep - s.bx) * part(u, 0, 0.42) + (m.bx - keep) * part(u, 0.62, 1);
  const by = mix(s.by, m.by, part(u, 0.06, 0.7));
  return { ...base, bx, by, show: mix(s.show, m.show, e) };
}

/** The bubble (bw × bh at the pose's place) over the window, or less than `margin` from it. */
export function covers(p: Pose, bw: number, bh: number, margin = 6): boolean {
  if (p.show < 0.02) return false;
  const hit = (w: Box4) =>
    w.w > 0 && w.x < p.bx + bw + margin && w.x + w.w > p.bx - margin && w.y < p.by + bh + margin && w.y + w.h > p.by - margin;
  return hit(p.a) || (p.two > 0.02 && hit(p.b));
}

/** "aside" when there is room for the bubble to keep out of the window's way all along (looked
 * at from end to end, with the taller of the two bubbles), else "dip". */
export function cornerPlan(s: Pose, m: Pose, bw: number, bh: number, m0 = 16): Way {
  if (Math.min(m.a.x, m.b.x) - 12 - bw < m0) return "dip";
  for (let k = 0; k <= 60; k++) if (covers(cornerWay(s, m, k / 60, bw, "aside"), bw, bh, 4)) return "dip";
  return "aside";
}

/** Any other move between two steps: the window and the bubble glide straight to their new
 * places together ("glide"), unless the window would pass under the bubble on the way (a step
 * that shows its bubble over what it lights, the next one under): then the bubble fades where it
 * is and comes back at its new place once the window is there ("dip"). (A window opening from the
 * bubble's middle, or closing into it, is meant to: it glides.) */
/** (Dipping, the part of the move's time the bubble is still where it was, fading out.) */
export const DIP_OUT = 0.43;
export function straightWay(f: Pose, t: Pose, p: number, way: "glide" | "dip", lift = 0, late = false): Pose {
  // (Dipping, the window sets off once the bubble has gone; `late`, it is already on its way (a
  // glide that turned into a dip): it goes on without stopping as the bubble fades quickly.)
  const e = way === "glide" ? glide(p) : late ? glide(Math.min(1, p / 0.85)) : glide(Math.min(1, Math.max(0, (p - 0.2) / 0.65)));
  const at = { a: between(f.a, t.a, e), b: between(f.b, t.b, e), r: mix(f.r, t.r, e), ring: mix(f.ring, t.ring, e), two: mix(f.two, t.two, e) };
  if (way === "glide") return { ...at, bx: mix(f.bx, t.bx, e), by: mix(f.by, t.by, e), show: mix(f.show, t.show, e) };
  const dy = Math.sign(t.by - f.by) * 6;
  if (p < DIP_OUT) {
    // (`lift`: how much taller the bubble has grown meanwhile, when it sits above its window: it
    // grows upwards, so its bottom never comes nearer the window as it fades.)
    const o = part(p, 0, late ? 0.14 : 0.2);
    return { ...at, bx: f.bx, by: f.by + dy * o - lift, show: f.show * (1 - o) };
  }
  const i = part(p, 0.66, 1);
  return { ...at, bx: t.bx, by: t.by - dy * (1 - i), show: t.show * i };
}

/** "dip" when gliding straight would take the window under the bubble (bw × bh). */
export function straightPlan(f: Pose, t: Pose, bw: number, bh: number): "glide" | "dip" {
  if (f.ring < 0.5 || t.ring < 0.5) return "glide";
  for (let k = 1; k < 60; k++) if (covers(straightWay(f, t, k / 60, "glide"), bw, bh, 2)) return "dip";
  return "glide";
}

/** With less motion, when each window fades (in, then out, as parts of the move's time): the
 * new one comes while no bubble shows over it and the old one goes while none shows over it;
 * otherwise the new one is lit before the old one goes, so where both light the same place it
 * never dims in between. */
export type Fade = { in: [number, number]; out: [number, number] };

/** The fade for a move from `f` to `t` with less motion (the bubble, bw × bh, fades out where it
 * was over the first half and in at its new place over the second). */
export function fadePlan(f: Pose, t: Pose, bw: number, bh: number): Fade {
  const underOld = t.ring > 0.02 && covers({ ...t, bx: f.bx, by: f.by, show: 1 }, bw, bh, 2);
  const underNew = f.ring > 0.02 && covers({ ...f, bx: t.bx, by: t.by, show: 1 }, bw, bh, 2);
  if (underOld) return underNew ? { in: [0.5, 0.75], out: [0, 0.5] } : { in: [0.5, 0.75], out: [0.75, 1] };
  return underNew ? { in: [0, 0.25], out: [0.25, 0.5] } : { in: [0, 0.5], out: [0.5, 1] };
}

/** The move at time p (0 → 1) with less motion: where things show (the new window, the bubble
 * fading out where it was, then in at its new place), how lit the new window is and the old one
 * (`lit0`: how lit it was when the move began). The same window, as when only the bubble moves
 * within a step, stays lit. */
export function fadeWay(f: Pose, t: Pose, p: number, plan: Fade, lift = 0, lit0 = 1): { pose: Pose; lit: number; old: number } {
  const q = ([a, b]: [number, number]) => Math.min(1, Math.max(0, (p - a) / (b - a)));
  const pose = p < 0.5 ? { ...t, bx: f.bx, by: f.by - lift, show: f.show * (1 - 2 * p) } : { ...t, show: t.show * (2 * p - 1) };
  const near = (u: Box4, v: Box4) => Math.abs(u.x - v.x) < 1 && Math.abs(u.y - v.y) < 1 && Math.abs(u.w - v.w) < 1 && Math.abs(u.h - v.h) < 1;
  if (near(f.a, t.a) && near(f.b, t.b) && Math.abs(f.two - t.two) < 0.02) return { pose, lit: Math.max(lit0, q(plan.in)), old: 0 };
  return { pose, lit: q(plan.in), old: lit0 * (1 - q(plan.out)) };
}
