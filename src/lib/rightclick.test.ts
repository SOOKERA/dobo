// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A note's or a line's menu asked for (right-click, Shift+F10, the Menu key), and the keyboard's
// own right-click set aside when Dobo already answered its key.
import { describe, expect, it, vi } from "vitest";
import { askMenu, fromKeys, isMenuKey, noteMenuKey, openedByKey, rightClick, swallowed, takeMenu } from "./rightclick.svelte";

const key = (k: string, more: Partial<KeyboardEvent> = {}) => ({ key: k, shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, ...more }) as KeyboardEvent;
const click = (button: number) => ({ button }) as MouseEvent;

describe("the menu asked for", () => {
  it("is taken once, by the note or the line it is for", () => {
    askMenu("line", 7, { x: 10, y: 20 });
    expect(takeMenu("note", 7)).toBeUndefined();
    expect(takeMenu("line", 8)).toBeUndefined();
    expect(takeMenu("line", 7)).toEqual({ at: { x: 10, y: 20 } });
    expect(takeMenu("line", 7)).toBeUndefined();
  });
  it("nobody taking it (the line not shown), it is forgotten, never opened later", () => {
    vi.useFakeTimers();
    askMenu("note", 3, null);
    vi.advanceTimersByTime(300);
    expect(rightClick.ask).toBeNull();
    vi.useRealTimers();
  });
});

describe("the menu keys", () => {
  it("are Shift+F10 and the Menu key, without Ctrl or Alt", () => {
    expect(isMenuKey(key("ContextMenu"))).toBe(true);
    expect(isMenuKey(key("F10", { shiftKey: true }))).toBe(true);
    expect(isMenuKey(key("F10"))).toBe(false);
    expect(isMenuKey(key("F10", { shiftKey: true, ctrlKey: true }))).toBe(false);
    expect(isMenuKey(key("ContextMenu", { altKey: true }))).toBe(false);
  });
  it("a right-click with no button, or just after a menu key, is the keyboard's", () => {
    expect(fromKeys(click(-1))).toBe(true);
    expect(fromKeys(click(2))).toBe(false);
    noteMenuKey(key("ContextMenu"));
    expect(fromKeys(click(2))).toBe(true);
  });
  it("the keyboard's right-click for a menu Dobo opened from the key is set aside, once", () => {
    expect(swallowed()).toBe(false);
    openedByKey();
    // (The Menu key's comes as the key goes back up.)
    noteMenuKey(key("ContextMenu"));
    expect(swallowed()).toBe(true);
    expect(swallowed()).toBe(false);
  });
});
