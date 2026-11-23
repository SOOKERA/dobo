// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it, vi } from "vitest";
import { isTopModal, modal, tabTarget } from "./modal";

describe("Tab inside a view", () => {
  it("goes round its controls, both ways", () => {
    expect(tabTarget(3, 2, false)).toBe(0);
    expect(tabTarget(3, 0, true)).toBe(2);
  });
  it("lets the browser take the next one in between", () => {
    expect(tabTarget(3, 0, false)).toBeNull();
    expect(tabTarget(3, 2, true)).toBeNull();
  });
  it("comes in from outside (the board, nowhere) at the first, or the last going back", () => {
    expect(tabTarget(4, -1, false)).toBe(0);
    expect(tabTarget(4, -1, true)).toBe(3);
  });
  it("does nothing in a view without controls", () => {
    expect(tabTarget(0, -1, false)).toBeNull();
  });
});

describe("a view opened again while it fades out", () => {
  it("is back on top, with the keyboard, and gives it back only once it really goes", async () => {
    // (No page in these tests: just enough of one for the view and the keyboard.)
    const field = { focus: vi.fn() };
    const node = Object.assign(new EventTarget(), {
      isConnected: true,
      closest: () => null,
      contains: (el: unknown) => el === field,
      hasAttribute: () => false,
      querySelector: () => field,
      querySelectorAll: () => [],
    }) as unknown as HTMLElement;
    const was = { isConnected: true, closest: () => null, focus: vi.fn(), blur: vi.fn() };
    const body = {};
    vi.stubGlobal("document", { activeElement: was, body, addEventListener: () => {}, getElementById: () => null, querySelectorAll: () => [] });
    vi.stubGlobal("requestAnimationFrame", () => 0);
    vi.stubGlobal("cancelAnimationFrame", () => {});
    try {
      const view = modal(node, { initial: "#field" });
      expect(isTopModal(node)).toBe(true);
      expect(field.focus).toHaveBeenCalledTimes(1);
      // Closing: off the stack at once; then opened again in the same moment (Svelte turns the
      // fade round), before the keyboard was given back.
      node.dispatchEvent(new Event("outrostart"));
      expect(isTopModal(node)).toBe(false);
      (document as unknown as { activeElement: unknown }).activeElement = field;
      node.dispatchEvent(new Event("introstart"));
      expect(isTopModal(node)).toBe(true);
      expect(field.focus).toHaveBeenCalledTimes(2);
      await Promise.resolve();
      expect(was.focus).not.toHaveBeenCalled();
      // Really closing now: the keyboard goes back to what had it before the view first opened.
      node.dispatchEvent(new Event("outrostart"));
      await Promise.resolve();
      expect(was.focus).toHaveBeenCalledTimes(1);
      expect(isTopModal(node)).toBe(false);
      view.destroy();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
