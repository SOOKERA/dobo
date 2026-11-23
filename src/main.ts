// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { mount, unmount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import DumpBar from "./components/DumpBar.svelte";
import Recovery from "./components/Recovery.svelte";
import { backendPromise, inTauri, type Backend, type RecoveryChoice, type RecoveryProblem } from "./lib/backend";
import { applyMode } from "./lib/look.svelte";
import { darkBySwitch, readDarkMode } from "./lib/looks";
import { startTips } from "./lib/tip";
import { fieldsMenuOnly } from "./lib/contextmenu";
import { dateFmt, i18n, readChoice, t } from "./lib/i18n.svelte";
import { store } from "./lib/store.svelte";

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
// WebView2's right-click menu in fields only (left whole in a development build, for Inspect).
if (inTauri && import.meta.env.PROD) fieldsMenuOnly();
// Until the settings are read: as Windows says (no flash of the wrong mode).
applyMode(window.matchMedia("(prefers-color-scheme: dark)").matches);

const target = document.getElementById("app")!;

// One page, two windows: the board, and the quick dump bar. Nothing is drawn before the language
// is known (Windows may speak another language than the web view says): no first frame in the
// wrong words.
const app = backendPromise.then(async (b) => {
  await b.languageNow().then(
    (l) => {
      i18n.system = l.system;
      i18n.choice = readChoice(l.choice);
      i18n.hour12 = !!l.hour12;
      document.documentElement.lang = i18n.lang;
    },
    () => {},
  );
  // Windows switched between a 12- and a 24-hour clock: every time shown follows.
  b.onEvent("clock-format", (v) => (i18n.hour12 = v === true));
  if (b.label === "dump") {
    document.documentElement.classList.add("dump");
    followMode(b);
    return mount(DumpBar, { target });
  }
  // The notes didn't open at start: the recovery screen first (recovery.rs), the board once they
  // are open.
  const problem = await b.recoveryState().catch(() => null);
  if (problem) return recover(b, problem);
  return mount(App, { target });
});

/** The recovery screen; once the notes are open, it leaves and the board comes in (with a word on
 * what was done). `error`: why the board couldn't come in after an earlier choice; `stuck`: the
 * choice that worked, when nothing is waiting on a choice any more (that reason, Try again to
 * bring the board in, and Quit; never the choices of before). */
function recover(b: Backend, problem: RecoveryProblem, error = "", stuck: RecoveryChoice | null = null): Promise<ReturnType<typeof mount>> {
  return new Promise<ReturnType<typeof mount>>((resolve) => {
    const screen = mount(Recovery, {
      target,
      props: {
        backend: b,
        problem,
        error,
        stuck: !!stuck,
        onDone: async (picked: RecoveryChoice, was: RecoveryProblem) => {
          // (Try again on a stuck screen finishes the choice that worked: its word, once the board is in.)
          const choice = stuck ?? picked;
          await unmount(screen, { outro: true });
          // (The name the damaged file was put aside as, once it was: by this choice, or before.)
          let kept: string | null;
          try {
            kept = (await b.recoveryContinue()) ?? was.kept_as;
          } catch (e) {
            // The board still can't come in: the screen again, saying why (never a blank window).
            // Nothing waiting on a choice any more: the reason, Try again and Quit, never the old choices.
            const now = await b.recoveryState().catch(() => null);
            return resolve(recover(b, now ?? was, String(e), now ? null : choice));
          }
          // (The notes can be read now: their own language setting, if they have one.)
          await b.languageNow().then((l) => ((i18n.choice = readChoice(l.choice)), (document.documentElement.lang = i18n.lang)), () => {});
          const day = (ms: number) => dateFmt({ day: "numeric", month: "long" }).format(new Date(ms));
          const word =
            choice === "restore" && was.backup
              ? kept
                ? t("Your notes from {when} are back. The damaged file is kept as {name}.", { when: day(was.backup.at), name: kept })
                : t("Your notes from {when} are back.", { when: day(was.backup.at) })
              : choice === "empty" && kept
                ? t("A new board. The damaged file is kept as {name} in the data folder.", { name: kept })
                : choice === "repair"
                  ? kept
                    ? t("Your notes are back, repaired. The damaged file is kept as {name}.", { name: kept })
                    : t("Your notes are back, repaired.")
                  : choice === "putback"
                    ? t("Your notes are back.")
                    : null;
          // (Said once the board is in, never under the first-run window or the terms.)
          resolve(mount(App, { target, props: { welcome: word } }));
        },
      },
    });
  });
}

export default app;
