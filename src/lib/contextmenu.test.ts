// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { copyOffer, copyText, menuCap, menuTargetOf, placeMenu, selectedAt } from "./contextmenu";

describe("the Copy menu (outside fields, in the app)", () => {
  // (No page in these tests: just enough of one.)
  class El {
    constructor(private sel: Record<string, unknown> = {}) {}
    closest(s: string) {
      return this.sel[s] ?? null;
    }
  }
  beforeEach(() => vi.stubGlobal("Element", El));
  const el = (sel: Record<string, unknown> = {}) => new El(sel) as unknown as EventTarget;
  afterEach(() => vi.unstubAllGlobals());

  it("offers the selected words", () => {
    expect(copyOffer(el(), String.raw`  C:\Users\Jo\Dobo  `, true)).toEqual({ text: String.raw`C:\Users\Jo\Dobo`, link: null });
  });
  it("offers a link's address, web or mail only", () => {
    expect(copyOffer(el({ "a[href]": { href: "https://dobo.sookera.com/help" } }), "", true)).toEqual({ text: "", link: "https://dobo.sookera.com/help" });
    expect(copyOffer(el({ "a[href]": { href: "javascript:void 0" } }), "", true)).toBeNull();
  });
  it("offers nothing with nothing to copy, in a field (WebView2's own menu), or in the browser preview", () => {
    expect(copyOffer(el(), "", true)).toBeNull();
    const field = el({ "textarea, [contenteditable]:not([contenteditable='false']), input:not([type]), input[type='text'], input[type='search'], input[type='url'], input[type='email'], input[type='number'], input[type='tel'], input[type='password'], input[type='time'], input[type='date']": {} });
    expect(copyOffer(field, "words", true)).toBeNull();
    expect(copyOffer(el(), "words", false)).toBeNull();
  });
});

describe("the words offered are the ones right-clicked", () => {
  // Two lines selected: (10,10)-(110,30) and (10,30)-(60,50).
  const rect = (left: number, top: number, right: number, bottom: number) => ({ left, top, right, bottom, width: right - left, height: bottom - top });
  const sel = (rects = [rect(10, 10, 110, 30), rect(10, 30, 60, 50)], text = "two lines") => ({
    isCollapsed: false,
    rangeCount: 1,
    getRangeAt: () => ({ getClientRects: () => rects }) as unknown as Range,
    toString: () => text,
  });
  it("on the selected words (or a few pixels off them): the words", () => {
    expect(selectedAt(sel(), 50, 20)).toBe("two lines");
    expect(selectedAt(sel(), 58, 45)).toBe("two lines");
    expect(selectedAt(sel(), 112, 20)).toBe("two lines");
  });
  it("anywhere else (words selected elsewhere on the page), or nothing selected: none", () => {
    expect(selectedAt(sel(), 100, 45)).toBe("");
    expect(selectedAt(sel(), 300, 300)).toBe("");
    expect(selectedAt({ ...sel(), isCollapsed: true }, 50, 20)).toBe("");
    expect(selectedAt(null, 50, 20)).toBe("");
    expect(selectedAt(sel([rect(0, 0, 0, 0)]), 0, 0)).toBe("");
  });
});

describe("a right-click on the board opens the note's or the line's own menu", () => {
  // (No page here: an element is what its `closest` finds for each selector.)
  class El {
    constructor(private found: Record<string, { dataset: Record<string, string> } | object> = {}) {}
    closest(s: string) {
      for (const [k, v] of Object.entries(this.found)) if (s.split(", ").some((part) => part === k)) return v;
      return null;
    }
  }
  beforeEach(() => vi.stubGlobal("Element", El));
  afterEach(() => vi.unstubAllGlobals());
  const on = (found: Record<string, object>) => menuTargetOf(new El(found) as unknown as EventTarget);

  it("a line (its words, its checkbox, its pictures or link: data-menu-line) opens the line's menu", () => {
    expect(on({ "[data-line]": { dataset: { line: "12" } }, "[data-drop-note]": { dataset: { dropNote: "3" } } })).toEqual({ kind: "line", id: 12 });
    expect(on({ "[data-menu-line]": { dataset: { menuLine: "7" } }, "[data-drop-note]": { dataset: { dropNote: "3" } } })).toEqual({ kind: "line", id: 7 });
  });
  it("a note's title, its buttons, its pictures open the note's menu", () => {
    expect(on({ "[data-drop-note]": { dataset: { dropNote: "3" } }, ".slot": {} })).toEqual({ kind: "note", id: 3 });
  });
  it("a field keeps WebView2's menu; an open menu, a link: nothing of Dobo's", () => {
    expect(on({ textarea: {}, "[data-line]": { dataset: { line: "12" } } })).toEqual({ kind: "none" });
    expect(on({ "[role='dialog']": {}, "[data-line]": { dataset: { line: "12" } } })).toEqual({ kind: "none" });
    expect(on({ "a[href]": {}, "[data-drop-note]": { dataset: { dropNote: "3" } } })).toEqual({ kind: "none" });
  });
  it("the Inbox strip's head: nothing; the empty board: its own menu (null)", () => {
    expect(on({ "[data-inbox-strip]": {} })).toEqual({ kind: "none" });
    expect(on({})).toBeNull();
    expect(menuTargetOf(null)).toBeNull();
  });
});

describe("a menu opened at the pointer stays whole inside the board", () => {
  const room = { left: 0, top: 100, right: 400, bottom: 700 };
  it("opens to the right of the pointer and below it, growing from it", () => {
    expect(placeMenu({ x: 50, y: 150 }, 200, 300, room)).toEqual({ left: 50, top: 150, up: false, ox: 0, oy: 0 });
  });
  it("flips to the left near the right edge, and above near the bottom", () => {
    expect(placeMenu({ x: 300, y: 600 }, 200, 300, room)).toEqual({ left: 100, top: 300, up: true, ox: 200, oy: 300 });
  });
  it("too wide for either side: moved in, 8 px from the edge", () => {
    expect(placeMenu({ x: 150, y: 150 }, 300, 100, room)).toMatchObject({ left: 92, ox: 58 });
  });
  it("too tall for either side: moved up just enough to show whole, the pointer on it", () => {
    expect(placeMenu({ x: 50, y: 450 }, 200, 400, room)).toEqual({ left: 50, top: 292, up: false, ox: 0, oy: 158 });
  });
  it("taller than the board: its top shows (the board scrolls for the rest)", () => {
    expect(placeMenu({ x: 50, y: 450 }, 200, 800, room)).toMatchObject({ top: 108, oy: 342 });
  });
  it("is never taller than the board's room (it scrolls inside itself), in the board's zoom", () => {
    expect(menuCap(room)).toBe(572);
    expect(menuCap(room, 1.25)).toBeCloseTo(457.6);
    expect(menuCap({ left: 0, top: 0, right: 400, bottom: 60 })).toBe(120);
  });
});

describe("Copy says whether it copied", () => {
  // (No page in these tests: just enough of one for the hidden box.)
  const page = (copies: boolean) =>
    ({
      getSelection: () => null,
      activeElement: null,
      body: { append() {} },
      createElement: () => ({ value: "", readOnly: false, style: {}, setAttribute() {}, select() {}, remove() {} }),
      execCommand: () => copies,
    }) as unknown as Document;
  afterEach(() => vi.unstubAllGlobals());

  it("copied through the clipboard", async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    expect(await copyText("x", page(false))).toBe(true);
    expect(writeText).toHaveBeenCalledWith("x");
  });

  it("the clipboard refused: through the page, and whether that worked", async () => {
    vi.stubGlobal("navigator", { clipboard: { writeText: async () => Promise.reject(new Error("no")) } });
    expect(await copyText("x", page(true))).toBe(true);
    expect(await copyText("x", page(false))).toBe(false);
  });
});
