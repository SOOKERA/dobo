// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A key pressed while a character is being composed (a French dead key's "^" before its "ê", an
// IME's word): Enter, Space, Tab, Esc, the arrows and Backspace then belong to the composition
// (they finish, pick or cancel it), never to the field's own commit, nor to the app's keys.
// (229: what the browser reports for a key the IME took, e.g. the keydown that ends it.)
export function composing(e: Pick<KeyboardEvent, "isComposing" | "keyCode">): boolean {
  return e.isComposing || e.keyCode === 229;
}
