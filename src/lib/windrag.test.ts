// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it, vi } from "vitest";
import { CONTROLS, installWindowDrag, isWindowDrag, movesWindow, pressedAt } from "./windrag";

// A tiny stand-in for the DOM (the tests run without one): elements with a tag, attributes and a
// parent, and `closest` for the simple selectors windrag uses.
class El {
  parent: El | null = null;
  constructor(
    public tag: string,
    public attrs: Record<string, string> = {},
    public kids: El[] = [],
  ) {
    for (const k of kids) k.parent = this;
  }
  matches(sel: string): boolean {
    return sel.split(",").some((one) => matchOne(this, one.trim()));
  }
  closest(sel: string): El | null {
    for (let e: El | null = this; e; e = e.parent) if (e.matches(sel)) return e;
    return null;
  }
  contains(o: El): boolean {
    for (let e: El | null = o; e; e = e.parent) if (e === this) return true;
    return false;
  }
}
function matchOne(e: El, sel: string): boolean {
  const not = /:not\((.+)\)$/.exec(sel);
  if (not) {
    if (matchOne(e, not[1])) return false;
    sel = sel.slice(0, not.index);
  }
  const tag = /^[a-z]+/.exec(sel)?.[0];
  if (tag && tag !== e.tag) return false;
  for (const m of sel.matchAll(/\[([\w-]+)(?:(\^?=)['"]?([^'"\]]*)['"]?)?\]/g)) {
    const v = e.attrs[m[1]];
    if (v === undefined || (m[3] !== undefined && (m[2] === "^=" ? !v.startsWith(m[3]) : v !== m[3]))) return false;
  }
  return true;
}
const el = (tag: string, attrs: Record<string, string> = {}, ...kids: El[]) => new El(tag, attrs, kids);
const as = (e: El | null) => e as unknown as Element;

// The board's header, floating: a drag area with the name, a group of buttons, and a menu.
const name = el("span", { class: "name" });
const viewBtn = el("button");
const icon = el("svg");
const toolBtn = el("button", {}, icon);
const gap = el("div", { role: "group" }, viewBtn);
const menuItem = el("button", { role: "menuitem" });
const menuGap = el("div", { role: "menu" }, menuItem);
const field = el("input");
const label = el("label");
const bar = el("header", { "data-drag-area": "" }, name, gap, toolBtn, menuGap, field, label);
const board = el("div", { class: "scroll" });
const veil = el("div", { "data-drag-through": "" });
const shell = el("div", { class: "shell" }, bar, board, veil);

describe("what moves the window", () => {
  it("is the empty part of a drag area, its words included", () => {
    expect(movesWindow(as(bar))).toBe(true);
    expect(movesWindow(as(name))).toBe(true);
    expect(movesWindow(as(gap))).toBe(true);
  });
  it("is never a button, a field, a menu or what is in them", () => {
    for (const e of [viewBtn, toolBtn, icon, menuGap, menuItem, field, label]) expect(movesWindow(as(e))).toBe(false);
  });
  it("is nothing outside a drag area (docked: there is none)", () => {
    expect(movesWindow(as(board))).toBe(false);
    expect(movesWindow(as(shell))).toBe(false);
    expect(movesWindow(null)).toBe(false);
  });
  it("is still the area when the area itself sits inside a control-like layer", () => {
    // (A sheet that is focusable as a whole, its top a drag area.)
    const crumb = el("span");
    const top = el("div", { "data-drag-area": "" }, crumb);
    el("div", { role: "button" }, top);
    expect(movesWindow(as(crumb))).toBe(true);
  });
  it("keeps every kind of control clickable", () => {
    for (const s of CONTROLS.split(", ")) {
      const m = /^([a-z]+)?(?:\[([\w-]+)(?:=([\w-]+))?\])?/.exec(s)!;
      if (s.includes(":not")) continue;
      const c = el(m[1] ?? "div", m[2] ? { [m[2]]: m[3] ?? "" } : {});
      el("div", { "data-drag-area": "" }, c);
      expect(movesWindow(as(c)), s).toBe(false);
    }
    const ce = el("div", { contenteditable: "true" });
    const off = el("div", { contenteditable: "false" });
    el("div", { "data-drag-area": "" }, ce, off);
    expect(movesWindow(as(ce))).toBe(false);
    expect(movesWindow(as(off))).toBe(true);
    // Reachable by Tab: a control; focusable only from code (a sheet's own -1): not.
    const tab = el("div", { tabindex: "0" });
    const code = el("div", { tabindex: "-1" });
    el("div", { "data-drag-area": "" }, tab, code);
    expect(movesWindow(as(tab))).toBe(false);
    expect(movesWindow(as(code))).toBe(true);
  });
});

describe("a press on a veil over the top", () => {
  const doc = (...under: El[]) => ({ elementsFromPoint: () => [veil, ...under].map(as) });
  it("reaches what is under it", () => {
    expect(pressedAt(as(veil), 10, 10, doc(name, bar, shell))).toBe(as(name));
    expect(pressedAt(as(veil), 10, 10, doc(toolBtn, bar))).toBe(as(toolBtn));
    expect(pressedAt(as(veil), 10, 400, doc(board, shell))).toBe(as(board));
  });
  it("leaves other presses as they are", () => {
    expect(pressedAt(as(name), 10, 10, doc())).toBe(as(name));
  });
});

describe("the press itself", () => {
  const ev = (target: El, button = 0, isPrimary = true) => ({ target, button, isPrimary, clientX: 5, clientY: 5, preventDefault: vi.fn() }) as unknown as PointerEvent;
  const fakeWin = (under: El[] = []) => {
    const on = new Map<string, (e: Event) => void>();
    const win = {
      document: { elementsFromPoint: () => under.map(as) },
      addEventListener: (t: string, f: (e: Event) => void, capture: boolean) => (expect(capture).toBe(true), on.set(t, f)),
      removeEventListener: (t: string) => on.delete(t),
    };
    return { win: win as unknown as Window, on };
  };

  it("starts the move on the left button only, away from controls, without selecting", () => {
    const start = vi.fn();
    const { win, on } = fakeWin();
    const stop = installWindowDrag(start, win);
    const down = on.get("pointerdown")!;
    const e = ev(name);
    down(e);
    expect(start).toHaveBeenCalledTimes(1);
    expect(e.preventDefault).toHaveBeenCalled();
    down(ev(name, 2));
    down(ev(name, 0, false));
    down(ev(toolBtn));
    down(ev(board));
    expect(start).toHaveBeenCalledTimes(1);
    // (The mouse's own press after it: no word selected, no text dragged.)
    const md = ev(name) as unknown as MouseEvent;
    on.get("mousedown")!(md);
    expect(md.preventDefault).toHaveBeenCalled();
    stop();
    expect(on.size).toBe(0);
  });
  it("goes through a veil to the header, and click-away handlers can tell", () => {
    const start = vi.fn();
    const { win, on } = fakeWin([veil, name, bar]);
    installWindowDrag(start, win);
    on.get("pointerdown")!(ev(veil));
    expect(start).toHaveBeenCalledTimes(1);
    expect(isWindowDrag(ev(veil), win.document)).toBe(true);
    expect(isWindowDrag(ev(board), win.document)).toBe(false);
  });
  it("doesn't move the window from a veil over a button", () => {
    const start = vi.fn();
    const { win, on } = fakeWin([veil, toolBtn, bar]);
    installWindowDrag(start, win);
    on.get("pointerdown")!(ev(veil));
    expect(start).not.toHaveBeenCalled();
  });
});
