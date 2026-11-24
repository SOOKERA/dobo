// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FRENCH, introOf, updateNotes } from "./update-notes.mjs";
import { notesIn } from "../src/lib/updateWords";

const EN = "# What's new\n\n## 1.2.0\n\nDobo 1.2.0 keeps your notes **in place**\nwhile you carry a task.\n\n### Highlights\n\n- One.\n\n## 1.1.1\n\nOlder.\n";
const FR = "# Nouveautés\n\n## 1.2.0\n\nDobo 1.2.0 garde vos notes en place : voyez !\n\n### L’essentiel\n\n- Un.\n";

describe("an update's notes in latest.json", () => {
  it("take each language's first paragraph of the version's section", () => {
    expect(introOf(EN, "1.2.0")).toBe("Dobo 1.2.0 keeps your notes in place while you carry a task.");
    expect(introOf(EN, "1.1.1")).toBe("Older.");
    expect(introOf("## Dobo 1.3.0\r\n\r\nWith its name.\r\n", "1.3.0")).toBe("With its name.");
    expect(introOf(EN, "1.3.0")).toBeNull();
    // (French keeps its own spaces.)
    expect(introOf(FR, "1.2.0")).toBe("Dobo 1.2.0 garde vos notes en place : voyez !");
  });

  it("carry the English, then the French after a plain line", () => {
    const notes = updateNotes(EN, FR, "1.2.0");
    expect(notes).toBe(`Dobo 1.2.0 keeps your notes in place while you carry a task.${FRENCH}Dobo 1.2.0 garde vos notes en place : voyez !`);
    // No French section: the English alone.
    expect(updateNotes(EN, FR, "1.1.1")).toBe("Older.");
    expect(updateNotes(EN, null, "1.2.0")).toBe("Dobo 1.2.0 keeps your notes in place while you carry a task.");
  });

  it("show the reader's language in Dobo, the whole text when there is no French part", () => {
    const notes = updateNotes(EN, FR, "1.2.0");
    expect(notesIn(notes, "en")).toBe("Dobo 1.2.0 keeps your notes in place while you carry a task.");
    expect(notesIn(notes, "fr")).toBe("Dobo 1.2.0 garde vos notes en place : voyez !");
    expect(notesIn("Only English.", "fr")).toBe("Only English.");
    expect(notesIn("Only English.", "en")).toBe("Only English.");
    // (Written by hand with a plain space before the colon: still found.)
    expect(notesIn("One.\n\nEn français : Un.", "fr")).toBe("Un.");
  });

  it("are there for every version in both files", () => {
    const help = join(import.meta.dirname, "..", "docs", "help");
    const en = readFileSync(join(help, "whats-new.md"), "utf8");
    const fr = readFileSync(join(help, "fr", "whats-new.md"), "utf8");
    for (const [, v] of en.matchAll(/^## (\d+\.\d+\.\d+)\s*$/gm)) {
      const notes = updateNotes(en, fr, v);
      expect(notes, v).toBeTruthy();
      expect(notesIn(notes, "fr"), v).not.toBe(notesIn(notes, "en"));
    }
  });
});
