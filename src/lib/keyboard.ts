// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo's letter and digit keys on any keyboard layout: AZERTY, whose top row types "&", "é"…
// without Shift; Cyrillic or Greek, whose letters aren't Latin.

/** The letter a key gives, on any keyboard: what it types when that is a Latin letter (so AZERTY's
 * A is "a"), else the letter of its place on the keyboard (a Cyrillic or Greek layout's S key is
 * "s"), so Ctrl+Shift+S, F and Ctrl+Z work there too. */
export function latinKey(e: Pick<KeyboardEvent, "key" | "code">): string {
  if (/^[a-z]$/i.test(e.key)) return e.key.toLowerCase();
  if (/^Key[A-Z]$/.test(e.code)) return e.code.slice(3).toLowerCase();
  return e.key.toLowerCase();
}

/** The digit 1–9 (or 0) a key stands for, on any keyboard: AZERTY's top row without Shift types
 * "&", "é"…, the number pad its own; null for any other key. */
export function digitKey(e: Pick<KeyboardEvent, "key" | "code">): number | null {
  if (/^[0-9]$/.test(e.key)) return Number(e.key);
  const m = /^(?:Digit|Numpad)([0-9])$/.exec(e.code);
  return m ? Number(m[1]) : null;
}

/** Ctrl + / Ctrl − / Ctrl 0 (the note size): +1, -1, 0, or null, on any keyboard. */
export function sizeKey(e: Pick<KeyboardEvent, "key" | "code">): -1 | 0 | 1 | null {
  if (e.key === "=" || e.key === "+" || e.code === "NumpadAdd") return 1;
  if (e.key === "-" || e.key === "_" || e.code === "NumpadSubtract") return -1;
  if (digitKey(e) === 0) return 0;
  return null;
}

/** Windows' menu key, Shift+F10 (as in Rust, `menu_key`): never a shortcut, it would stop opening
 * the right-click menu in every app. (The Menu key itself is no key a shortcut is recorded with.) */
export function menuKey(e: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">): boolean {
  return e.code === "ContextMenu" || (e.code === "F10" && e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey);
}

/** A shortcut that would be taken from every app as people use it (as in Rust,
 * `fires_while_typing`): no Ctrl, Alt or Win, unless it is Shift with a key that never types
 * (F1…F24, Pause, Scroll Lock). */
export function firesWhileTyping(e: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">): boolean {
  if (e.ctrlKey || e.altKey || e.metaKey) return false;
  const neverTypes = /^F([1-9]|1[0-9]|2[0-4])$/.test(e.code) || e.code === "Pause" || e.code === "ScrollLock";
  return !(e.shiftKey && neverTypes);
}
