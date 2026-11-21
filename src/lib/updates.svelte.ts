// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo's own update, as the window knows it (src-tauri/src/updates.rs): what Settings › Updates
// shows, kept up to date by the "update-status" events (the daily check, Check for updates).
// When a new version has been downloaded and checked by the daily check, the board says so
// once per version, with Restart now (it's used at the next start anyway).
import { store } from "./store.svelte";
import type { UpdateView } from "./backend";
import { APP_NAME } from "./app";
import { t } from "./i18n.svelte";

export const updates = $state<{ v: UpdateView | null }>({ v: null });

let listening = false;

/** Starts following the update's state (once; App.svelte at start). */
export function watchUpdates() {
  if (listening) return;
  listening = true;
  store.backend.onEvent("update-status", (p) => took(p as UpdateView, true));
  store.backend
    .updateStatus()
    .then((v) => took(v, false))
    .catch(() => {});
}

function took(v: UpdateView, said: boolean) {
  updates.v = v;
  if (!said || !v.ready || !v.latest || store.settings.update_said === v.latest || checkingByHand) return;
  void store.setSetting("update_said", v.latest);
  const version = v.latest;
  store.showToast(() => t("{app} {version} is ready. It starts the next time {app} opens.", { app: APP_NAME, version }), { label: () => t("Restart now"), run: () => void restartNow() }, 12_000);
}

let checkingByHand = false;

/** "Check for updates" (Settings shows the result itself, no toast). */
export async function checkNow() {
  checkingByHand = true;
  try {
    const v = await store.backend.updateCheck();
    updates.v = v;
    if (v.ready && v.latest) void store.setSetting("update_said", v.latest);
  } catch (e) {
    updates.v = { ...(updates.v ?? { current: "", latest: null, notes: null, ready: false, checked: true, last_checked: null }), busy: false, error: String(e) };
  } finally {
    checkingByHand = false;
  }
}

/** "Restart now": Dobo quits and starts the new version. */
export async function restartNow() {
  try {
    await store.backend.updateRestart();
  } catch (e) {
    store.showToast(String(e));
  }
}
