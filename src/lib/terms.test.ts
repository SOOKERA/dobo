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

  it("has a French courtesy translation in step with the English", () => {
    const files = import.meta.glob<string>(["/TERMS.md", "/TERMS.fr.md"], { query: "?raw", import: "default", eager: true });
    const read = (lang: "en" | "fr") => files[lang === "fr" ? "/TERMS.fr.md" : "/TERMS.md"];
    const [en, fr] = [parseTerms(read("en")), parseTerms(read("fr"))];
    expect(fr[0]).toEqual({ kind: "h1", text: "Conditions d’utilisation" });
    // As many sections, list items and links as the English: a section added there is added here.
    const shape = (bs: typeof en) => bs.map((b) => (b.kind === "p" || b.kind === "li" ? `${b.kind}${b.parts.filter((p) => p.url).map((p) => p.url).join()}` : b.kind));
    const frShape = shape(fr).filter((_, i) => !(fr[i].kind === "p" && (fr[i] as { parts: { text: string }[] }).parts[0].text.startsWith("Traduction de courtoisie")));
    expect(frShape).toEqual(shape(en));
    // The same date, and the English named as the version that counts.
    const date = (bs: typeof en) => (bs[1] as { parts: { text: string }[] }).parts[0].text;
    const months = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    const enDate = new Date(date(en).replace("Last updated ", "") + " 12:00");
    expect(date(fr)).toBe(`Dernière mise à jour le ${enDate.getDate()} ${months[enDate.getMonth()]} ${enDate.getFullYear()}`);
    expect(read("fr")).toMatch(/seule la version anglaise de ces conditions \(TERMS\.md\) fait foi/);
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
