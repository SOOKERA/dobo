// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Copy text (a line) and Copy as text (a note), from their menus.
import { describe, expect, it } from "vitest";
import { lineText, noteAsText } from "./astext";
import type { Item } from "./types";

let next = 1;
const line = (text: string, more: Partial<Item> = {}): Item =>
  ({ id: next++, note_id: 1, kind: "task", text, due_at: null, done_at: null, sort: 0, created_at: 0, parent_id: null, alarm: "reminder", rhythm: null, link: null, file: null, repeat: null, ring_state: "idle", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null, ...more }) as Item;

describe("a line as text", () => {
  it("is its words, with its link's address when they don't hold it", () => {
    const link = { url: "https://example.com/a", title: "A", site: "example.com", image: null };
    expect(lineText(line("  Call Léa  "))).toBe("Call Léa");
    expect(lineText(line("Read https://example.com/a", { link }))).toBe("Read https://example.com/a");
  });
  it("then its link and its file, each on its own line", () => {
    const link = { url: "https://example.com/a", title: "A", site: "example.com", image: null };
    expect(lineText(line("Read this", { link }))).toBe("Read this\nhttps://example.com/a");
    expect(lineText(line("Sign it", { file: String.raw`C:\Docs\contract.pdf` }))).toBe(String.raw`Sign it` + "\n" + String.raw`C:\Docs\contract.pdf`);
    expect(lineText(line("Both", { link, file: String.raw`D:\a.txt` }))).toBe("Both\nhttps://example.com/a\n" + String.raw`D:\a.txt`);
    expect(lineText(line("", { link }))).toBe("https://example.com/a");
  });
});

describe("a note as text", () => {
  it("its title, its tasks as a checklist (to do, then done, steps under each), then its ideas", () => {
    const done = line("Order the mouse", { done_at: 5 });
    const open = line("Call Léa");
    const idea = line("A mouse with a dial", { kind: "idea" });
    const steps = new Map([[open.id, [line("Find the number", { done_at: 3 }), line("Ask about Friday")]]]);
    expect(noteAsText("Logitech", [done, open, idea], (id) => steps.get(id) ?? [])).toBe(
      ["Logitech", "", "- [ ] Call Léa", "  - [ ] Ask about Friday", "  - [x] Find the number", "- [x] Order the mouse", "", "- A mouse with a dial"].join("\n"),
    );
  });
  it("a line's link or file stays under it, indented", () => {
    const link = { url: "https://example.com/a", title: "A", site: "example.com", image: null };
    const task = line("Read this", { link });
    const step = line("Sign it", { file: String.raw`C:\x.pdf` });
    const idea = line("Look", { kind: "idea", file: String.raw`D:\y.png` });
    expect(noteAsText("N", [task, idea], (id) => (id === task.id ? [step] : []))).toBe(
      ["N", "", "- [ ] Read this", "      https://example.com/a", "  - [ ] Sign it", String.raw`        C:\x.pdf`, "", "- Look", String.raw`  D:\y.png`].join("\n"),
    );
  });
  it("an empty note is its title; ideas alone follow it", () => {
    expect(noteAsText("Hub", [], () => [])).toBe("Hub");
    expect(noteAsText("Hub", [line("Mini project", { kind: "idea" })], () => [])).toBe("Hub\n\n- Mini project");
  });
});
