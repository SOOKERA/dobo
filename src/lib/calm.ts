// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Work done ahead of time (the Style panel's pictures, the first open of the big views) waits
// for a calm moment: nothing pressed, typed or scrolled for a little while, nothing on screen
// moving (a view opening, the panel sliding in, a note being carried), and the browser idle with
// time to spare. Then one piece at a time: whatever the user does next comes first.
const QUIET = 2000;
let last = -Infinity;

let listening = false;
function listen() {
  if (listening) return;
  listening = true;
  const mark = () => (last = performance.now());
  const opts = { capture: true, passive: true } as const;
  for (const type of ["pointerdown", "keydown", "wheel"]) window.addEventListener(type, mark, opts);
  // (A carry or a drag: the pointer moving with a button down.)
  window.addEventListener("pointermove", (e) => e.buttons && mark(), opts);
}

/** Something on screen is moving (a transition or an animation that ends: not a looping one). */
function moving(): boolean {
  if (typeof document.getAnimations !== "function") return false;
  return document.getAnimations().some((a) => {
    if (a.playState !== "running") return false;
    const end = a.effect?.getComputedTiming().endTime;
    return typeof end === "number" && Number.isFinite(end);
  });
}

// Someone is waiting for what work ahead holds up (a panel asking for its pictures): every wait
// ends at once, to look at `stop` again.
const kicks = new Set<() => void>();
export function interrupt() {
  for (const k of [...kicks]) k();
}
function orKick<T>(p: Promise<T>): Promise<T | null> {
  let k!: () => void;
  const kicked = new Promise<null>((r) => kicks.add((k = () => r(null))));
  return Promise.race([p, kicked]).finally(() => kicks.delete(k));
}
const sleep = (ms: number) => orKick(new Promise<void>((r) => setTimeout(r, ms)));
const idleCallback = (timeout: number) =>
  orKick(
    new Promise<IdleDeadline | null>((r) => {
      if ("requestIdleCallback" in window) requestIdleCallback((d) => r(d), { timeout });
      else setTimeout(() => r(null), 50);
    }),
  );

/** Calm right now (no input for a while, nothing moving)? */
export function calmNow(): boolean {
  listen();
  return performance.now() - last >= QUIET && !moving();
}

/** Waits for a calm, idle moment with at least `need` ms to spare; gives it (null when the
 * browser has no idle callbacks). `stop`: something more urgent came, stop waiting (null). */
export async function calmMoment(stop?: () => boolean, need = 8): Promise<IdleDeadline | null> {
  listen();
  for (;;) {
    if (stop?.()) return null;
    const wait = QUIET - (performance.now() - last);
    if (wait > 0) {
      await sleep(Math.min(wait, 250));
      continue;
    }
    const d = await idleCallback(1000);
    if (stop?.()) return null;
    if (d === null && "requestIdleCallback" in window) continue;
    if (performance.now() - last < QUIET) continue;
    if (d && (d.didTimeout || d.timeRemaining() < need)) continue;
    if (moving()) {
      await sleep(200);
      continue;
    }
    return d;
  }
}
