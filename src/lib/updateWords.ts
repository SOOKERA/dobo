// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Settings › Updates (modelled on Pako's): one line "Dobo 1.2.0 · Installed" with where the
// update stands under it, and one button beside it. What it says, from the update's state
// (src/lib/updates.svelte.ts, src-tauri/src/updates.rs View).
import type { UpdateView } from "./backend";
import { APP_NAME } from "./app";
import { dateAndTime, t } from "./i18n.svelte";

/** Where the update stands: a check running; a new version downloaded and checked (Restart
 * now); a newer one that exists but isn't ready (its error, if any, is said under the line);
 * nothing newer after a check that worked; or nothing known yet this session. */
export type UpdateState = "checking" | "ready" | "out" | "current" | "idle";

export function updateState(u: UpdateView | null): UpdateState {
  if (!u) return "idle";
  if (u.busy) return "checking";
  if (u.ready && u.latest) return "ready";
  if (u.latest) return "out";
  if (u.checked && !u.error) return "current";
  return "idle";
}

/** An update's notes in the reader's language: latest.json carries the English, then the French
 * after a line "En français : " (scripts/update-notes.mjs); without one, the notes as they are. */
export function notesIn(notes: string, lang: "en" | "fr"): string {
  const m = /\n[ \t]*\n[ \t]*En français[ \u00a0\u202f]?:[ \t]*/.exec(notes);
  if (!m) return notes.trim();
  return (lang === "fr" ? notes.slice(m.index + m[0].length) : notes.slice(0, m.index)).trim();
}

/** The words under the version line. `auto`: the automatic check is on (then it says so, unless
 * this copy never looks: a development or test build). */
export function updateWords(u: UpdateView | null, auto: boolean, when: (ms: number) => string = (ms) => dateAndTime(new Date(ms))): string {
  const st = updateState(u);
  if (st === "checking") return t("Checking for updates…");
  if (st === "ready") return t("{app} {version} is ready. It starts the next time {app} opens.", { app: APP_NAME, version: u!.latest! });
  // (Its error says why under the line; without one, the daily check left it, as a version that
  // was put back once: Check for updates gets it.)
  if (st === "out") return u!.error ? t("{app} {version} is out.", { app: APP_NAME, version: u!.latest! }) : t("{app} {version} is out. Check for updates to get it.", { app: APP_NAME, version: u!.latest! });
  const last = u?.last_checked;
  // (The date and time stay together on one line: "Tue 24 Nov, 10:24".)
  const checked = last ? t("Last checked {when}.", { when: when(last).replace(/ /g, " ") }) : t("Not checked yet.");
  const looks = auto && u?.enabled !== false;
  return [st === "current" ? t("You’re up to date.") : "", checked, looks ? t("{app} looks once a day.", { app: APP_NAME }) : ""].filter(Boolean).join(" ");
}
