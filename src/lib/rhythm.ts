// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The deadline rhythms (the presets come from Rust so both sides agree).
import { backendPromise } from "./backend";
import type { Item, PresetName, Rhythm } from "./types";

let cache: Promise<Record<"gentle" | "normal" | "insistent", Rhythm>> | null = null;

export function rhythmPresets() {
  cache ??= backendPromise.then((b) => b.rhythmPresets());
  return cache;
}

/** The rhythm an item uses: its own, else the default from Settings. */
export function presetOf(item: Item, settings: Record<string, unknown>): PresetName {
  const own = item.rhythm?.preset;
  if (own) return own;
  const d = settings.deadline_preset;
  return d === "gentle" || d === "insistent" || d === "custom" ? d : "normal";
}
