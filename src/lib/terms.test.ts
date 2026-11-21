// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { inline, parseTerms } from "./terms";

describe("the terms screen", () => {
  it("reads TERMS.md as headings, paragraphs and list items, every word kept", () => {
    const md = Object.values(import.meta.glob<string>("/TERMS.md", { query: "?raw", import: "default", eager: true }))[0];
    const blocks = parseTerms(md);
    expect(blocks[0]).toEqual({ kind: "h1", text: "Terms of use" });
    expect(blocks.some((b) => b.kind === "h2" && b.text === "The short version")).toBe(true);
    expect(blocks.filter((b) => b.kind === "li").length).toBeGreaterThan(4);
    const words = (s: string) => s.replace(/\]\([^)]*\)/g, "").replace(/[^\p{L}\p{N}']+/gu, " ").split(" ").filter(Boolean);
    const shown = blocks.map((b) => ("text" in b ? b.text : b.parts.map((p) => p.text).join(""))).join(" ");
    expect(words(shown)).toEqual(words(md));
    // Never anything but text: no HTML in what is shown.
    expect(shown).not.toMatch(/<[a-z]/i);
  });

  it("finds bold words and links", () => {
    expect(inline("Dobo comes **without any warranty**: see [sookera.com/terms](https://sookera.com/terms).")).toEqual([
      { text: "Dobo comes " },
      { text: "without any warranty", bold: true },
      { text: ": see " },
      { text: "sookera.com/terms", url: "https://sookera.com/terms" },
      { text: "." },
    ]);
  });
});
