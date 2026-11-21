// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The docked panel's width (the same rules as the Rust side, window.rs, which is the one that
// really sizes the window; this copy serves the browser preview and the Settings label).

export const DOCK_MIN = 420;
export const DOCK_MAX = 1200;
export const DOCK_PRESETS: [number, "Narrow" | "Medium" | "Wide"][] = [
  [560, "Narrow"],
  [720, "Medium"],
  [880, "Wide"],
];
const SNAP = 16;

/** A wanted width made sensible for a screen `workW` px wide: a gentle snap to the presets
 * (while dragging), then at least 420 and at most the smaller of 1200 or 70% of the screen. */
export function dockWidth(raw: number, workW: number, snap: boolean): number {
  let w = Number.isFinite(raw) ? raw : 720;
  if (snap) {
    const p = DOCK_PRESETS.find(([v]) => Math.abs(w - v) <= SNAP);
    if (p) w = p[0];
  }
  const max = Math.max(DOCK_MIN, Math.min(DOCK_MAX, workW * 0.7));
  return Math.round(Math.min(max, Math.max(DOCK_MIN, w)));
}

/** The width remembered for a screen, else the last one chosen, else Medium. */
export function widthForScreen(widths: unknown, key: string, global: unknown): number {
  const ok = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v) && v > 0;
  const own = widths && typeof widths === "object" ? (widths as Record<string, unknown>)[key] : undefined;
  return ok(own) ? own : ok(global) ? global : 720;
}

/** "Medium", or null when the width is one of your own. */
export function presetName(w: number): string | null {
  return DOCK_PRESETS.find(([v]) => v === Math.round(w))?.[1] ?? null;
}
