// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A double-click on a note head's buttons (colour dot, ⋯, fold) never starts renaming the note.
import { describe, expect, it } from "vitest";
import { fromControl } from "./dblclick";

// A small stand-in for an element: `closest` walks up its own chain of tags.
interface Fake {
  tag: string;
  role?: string;
  parent: Fake | null;
  closest(selector: string): unknown;
}
function el(tag: string, parent: Fake | null = null, role?: string): Fake {
  const node: Fake = {
    tag,
    role,
    parent,
    closest(selector: string): unknown {
      const wanted = selector.split(",").map((s) => s.trim());
      for (let at: Fake | null = node; at; at = at.parent) {
        if (wanted.includes(at.tag) || (at.role && wanted.includes(`[role=${at.role}]`))) return at;
      }
      return null;
    },
  };
  return node;
}

describe("a double-click in a note's head", () => {
  const head = el("div");
  const title = el("h3", head);
  const chev = el("button", head);
  const icon = el("svg", chev);

  it("on the title or the head itself renames", () => {
    expect(fromControl({ target: title as unknown as EventTarget }, head)).toBe(false);
    expect(fromControl({ target: head as unknown as EventTarget }, head)).toBe(false);
  });
  it("on a button, or the icon inside it, doesn't", () => {
    expect(fromControl({ target: chev as unknown as EventTarget }, head)).toBe(true);
    expect(fromControl({ target: icon as unknown as EventTarget }, head)).toBe(true);
    expect(fromControl({ target: el("span", null, "button") as unknown as EventTarget }, head)).toBe(true);
  });
  it("sent by a key (F2, Enter) to the head still renames", () => {
    expect(fromControl({ target: head as unknown as EventTarget }, head)).toBe(false); // keys.ts sends it to .head
    expect(fromControl({ target: null })).toBe(false);
  });
  it("is what NoteCard's head checks before renaming", () => {
    const card = Object.values(import.meta.glob<string>("/src/components/NoteCard.svelte", { query: "?raw", import: "default", eager: true }))[0];
    expect(card).toMatch(/class="head"[^>]*ondblclick=\{\(e\) => !fromControl\(e, e\.currentTarget\) && startTitle\(\)\}/);
  });
});
