// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { composing } from "./compose";

describe("keys while a character is being composed", () => {
  it("are the composition's: during it, and the keydown an IME took (229)", () => {
    expect(composing({ isComposing: true, keyCode: 13 })).toBe(true);
    expect(composing({ isComposing: false, keyCode: 229 })).toBe(true);
    expect(composing({ isComposing: false, keyCode: 13 })).toBe(false);
  });

  // Every text field that acts on a key (Enter files, renames, saves; Esc ends the edit) lets a
  // composed character finish first: an accent's dead key or an IME's word never commits.
  it("never commit a text field", () => {
    const sources = import.meta.glob<string>("/src/**/*.svelte", { query: "?raw", import: "default", eager: true });
    const fields = Object.entries(sources).filter(([, s]) => /<(input|textarea)\b/.test(s) && /key === "(Enter|Escape)"/.test(s));
    expect(fields.length).toBeGreaterThan(15);
    expect(fields.filter(([, s]) => !/composing\(e\)/.test(s)).map(([f]) => f)).toEqual([]);
  });
});
