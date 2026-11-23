// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { digitKey, firesWhileTyping, latinKey, menuKey, sizeKey } from "./keyboard";
import inApp from "./in-app-keys.json";

describe("keys on any keyboard", () => {
  it("a letter is what it types when that is Latin (AZERTY's A is a), else its place", () => {
    expect(latinKey({ key: "a", code: "KeyQ" })).toBe("a");
    expect(latinKey({ key: "S", code: "KeyS" })).toBe("s");
    // Russian: the S key types "ы", the F key "а".
    expect(latinKey({ key: "ы", code: "KeyS" })).toBe("s");
    expect(latinKey({ key: "а", code: "KeyF" })).toBe("f");
    expect(latinKey({ key: ",", code: "Comma" })).toBe(",");
  });
  it("the top row's digits count without Shift on AZERTY, and the number pad's", () => {
    expect(digitKey({ key: "&", code: "Digit1" })).toBe(1);
    expect(digitKey({ key: "à", code: "Digit0" })).toBe(0);
    expect(digitKey({ key: "7", code: "Numpad7" })).toBe(7);
    expect(digitKey({ key: "3", code: "Digit3" })).toBe(3);
    expect(digitKey({ key: "a", code: "KeyA" })).toBeNull();
  });
  it("Ctrl + / − / 0 for the note size", () => {
    expect(sizeKey({ key: "=", code: "Equal" })).toBe(1);
    expect(sizeKey({ key: "+", code: "NumpadAdd" })).toBe(1);
    expect(sizeKey({ key: "-", code: "Digit6" })).toBe(-1);
    expect(sizeKey({ key: "à", code: "Digit0" })).toBe(0);
    expect(sizeKey({ key: "0", code: "Numpad0" })).toBe(0);
    expect(sizeKey({ key: "a", code: "KeyQ" })).toBeNull();
  });
});

describe("the keys Dobo keeps for itself (in-app-keys.json, read by Rust too)", () => {
  type K = { accel: string; what: string; key: string; code: string; ctrl?: boolean; shift?: boolean; alt?: boolean };
  const press = (k: K) => ({ key: k.key, code: k.code, ctrlKey: !!k.ctrl, shiftKey: !!k.shift, altKey: !!k.alt, metaKey: false });
  it("each is a key the page's handlers use, for what it says", () => {
    for (const k of inApp.keys as K[]) {
      const e = press(k);
      const used =
        k.what === "size" ? e.ctrlKey && sizeKey(e) !== null
        : k.what === "undo" ? e.ctrlKey && ["z", "y"].includes(latinKey(e))
        : k.what === "style" ? e.ctrlKey && e.shiftKey && latinKey(e) === "s"
        : k.what === "settings" ? e.ctrlKey && (latinKey(e) === "p" || e.key === ",")
        : k.what === "move" ? e.ctrlKey && (e.key === "ArrowUp" || e.key === "ArrowDown")
        : k.what === "back" ? e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")
        : false;
      expect(used, k.accel).toBe(true);
    }
  });
  it("AZERTY's minus and underscore keys (6 and 8) are kept for the note size", () => {
    const accels = (inApp.keys as K[]).map((k) => k.accel);
    expect(accels).toContain("Ctrl+Digit6");
    expect(accels).toContain("Ctrl+Digit8");
    expect(accels).toContain("Ctrl+ArrowDown");
  });
});

describe("a shortcut that would fire while typing", () => {
  const k = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "metaKey" | "shiftKey", boolean>> = {}) => ({ code, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false, ...mods });
  it("a key alone, or Shift with a key that types, is refused", () => {
    for (const e of [k("KeyA"), k("KeyA", { shiftKey: true }), k("F5"), k("ArrowLeft", { shiftKey: true }), k("Enter", { shiftKey: true })]) expect(firesWhileTyping(e)).toBe(true);
  });
  it("Shift with a key that never types, or Ctrl / Alt / Win, is fine", () => {
    for (const e of [k("F12", { shiftKey: true }), k("F24", { shiftKey: true }), k("Pause", { shiftKey: true }), k("KeyA", { ctrlKey: true }), k("KeyD", { metaKey: true })]) expect(firesWhileTyping(e)).toBe(false);
  });
  it("Windows' menu key (Shift+F10, the Menu key) is never a shortcut", () => {
    for (const e of [k("F10", { shiftKey: true }), k("ContextMenu"), k("ContextMenu", { ctrlKey: true })]) expect(menuKey(e)).toBe(true);
    for (const e of [k("F10"), k("F10", { shiftKey: true, ctrlKey: true }), k("F10", { altKey: true }), k("F11", { shiftKey: true })]) expect(menuKey(e)).toBe(false);
  });
});
