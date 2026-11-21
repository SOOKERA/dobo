// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import DumpBar from "./components/DumpBar.svelte";
import { backendPromise, inTauri, type Backend } from "./lib/backend";
import { applyMode } from "./lib/look.svelte";
import { darkBySwitch, readDarkMode } from "./lib/looks";
import { startTips } from "./lib/tip";
import { i18n, readChoice } from "./lib/i18n.svelte";

/** The quick dump bar follows the same dark-mode switch (it has no board of its own). */
function followMode(b: Backend) {
  const q = window.matchMedia("(prefers-color-scheme: dark)");
  let said: boolean | null = null;
  const show = () => applyMode(said ?? q.matches);
  const ask = () =>
    b.darkModeNow().then((d) => {
      said = d.dark ?? darkBySwitch(readDarkMode(d), new Date());
      show();
    });
  q.addEventListener("change", show);
  b.onEvent("dark-mode", () => void ask());
  void ask();
}

if (!inTauri) document.documentElement.classList.add("preview");
startTips();
// Until the settings are read: as Windows says (no flash of the wrong mode).
applyMode(window.matchMedia("(prefers-color-scheme: dark)").matches);

// One page, two windows: the board, and the quick dump bar. Nothing is drawn before the language
// is known (Windows may speak another language than the web view says): no first frame in the
// wrong words.
const app = backendPromise.then(async (b) => {
  await b.languageNow().then(
    (l) => {
      i18n.system = l.system;
      i18n.choice = readChoice(l.choice);
      document.documentElement.lang = i18n.lang;
    },
    () => {},
  );
  if (b.label === "dump") {
    document.documentElement.classList.add("dump");
    followMode(b);
  }
  return mount(b.label === "dump" ? DumpBar : App, { target: document.getElementById("app")! });
});

export default app;
