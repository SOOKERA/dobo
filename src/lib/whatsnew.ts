// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// What's new, once after an update (as Pako's): docs/help/whats-new.md (fr/whats-new.md in
// French), bundled with the app, one entry per "## x.y.z" section. After an update the window
// shows every version newer than the one you had (`whats_new_seen`, noted by src-tauri's
// whatsnew.rs the first time) up to the one running, newest first, then notes the running one.
// A small, safe Markdown reader for exactly what those files use: paragraphs, ### headings,
// "- " lists, **bold**, `code` and [links](…). Never HTML: everything becomes text.
import enMd from "../../docs/help/whats-new.md?raw";
import frMd from "../../docs/help/fr/whats-new.md?raw";
import { APP_NAME } from "./app";

export type Inline = { kind: "text"; text: string } | { kind: "bold"; text: string } | { kind: "code"; text: string } | { kind: "link"; text: string; href: string };

export type Block = { kind: "heading"; level: number; text: string } | { kind: "para"; inline: Inline[] } | { kind: "list"; items: Inline[][] };

export interface Entry {
  /** The section's heading: a version ("1.2.0"), the same in both languages. */
  version: string;
  blocks: Block[];
}

/** A line's words as plain, **bold**, `code` and [linked](…) parts, in order. A link's words
 * lose their own marks. Anything else (HTML too) is text. */
export function inline(src: string): Inline[] {
  const out: Inline[] = [];
  let text = "";
  const flush = () => {
    if (text) out.push({ kind: "text", text });
    text = "";
  };
  let i = 0;
  while (i < src.length) {
    const rest = src.slice(i);
    let m: RegExpMatchArray | null;
    if ((m = rest.match(/^\*\*(.+?)\*\*/))) {
      flush();
      out.push({ kind: "bold", text: m[1] });
    } else if ((m = rest.match(/^`([^`]+)`/))) {
      flush();
      out.push({ kind: "code", text: m[1] });
    } else if ((m = rest.match(/^\[([^\]]+)\]\(([^)\s]+)\)/))) {
      flush();
      out.push({ kind: "link", text: plain(m[1]), href: m[2] });
    } else {
      text += src[i++];
      continue;
    }
    i += m[0].length;
  }
  flush();
  return out;
}

/** A line's words without their marks. */
export function plain(src: string): string {
  return inline(src)
    .map((p) => p.text)
    .join("");
}

const ITEM = /^\s{0,3}[-*+]\s+(.*)$/;

export function parse(md: string): Block[] {
  const blocks: Block[] = [];
  let para: string[] = [];
  let list: string[] | null = null;
  const endPara = () => {
    if (para.length) blocks.push({ kind: "para", inline: inline(para.join(" ")) });
    para = [];
  };
  const endList = () => {
    if (list) blocks.push({ kind: "list", items: list.map(inline) });
    list = null;
  };
  const lines = md.replace(/^﻿/, "").replace(/\r\n?/g, "\n").split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let m: RegExpMatchArray | null;
    if ((m = line.match(/^(#{1,6})\s+(.*?)\s*#*\s*$/))) {
      endPara();
      endList();
      blocks.push({ kind: "heading", level: m[1].length, text: plain(m[2]) });
    } else if ((m = line.match(ITEM))) {
      endPara();
      (list ??= []).push(m[1]);
    } else if (!line.trim()) {
      endPara();
      // (A blank line between items keeps the list going.)
      const next = lines.slice(i + 1).find((l) => l.trim());
      if (list && !(next && ITEM.test(next))) endList();
    } else if (list && /^\s{2,}\S/.test(line)) {
      list[list.length - 1] += " " + line.trim();
    } else {
      endList();
      para.push(line.trim());
    }
  }
  endPara();
  endList();
  return blocks;
}

/** whats-new.md, one entry per "## <version>" section, in the file's order (newest first). */
export function entriesOf(md: string): Entry[] {
  const out: Entry[] = [];
  for (const b of parse(md)) {
    if (b.kind === "heading" && b.level === 1) continue;
    if (b.kind === "heading" && b.level === 2) out.push({ version: b.text, blocks: [] });
    else out[out.length - 1]?.blocks.push(b);
  }
  return out;
}

/** In English order; each version in the translation's words when it has that section (with
 * something in it), in English otherwise. */
export function buildEntries(en: string, local?: string): Entry[] {
  const english = entriesOf(en);
  if (!local) return english;
  const translated = new Map(entriesOf(local).map((e) => [e.version, e]));
  return english.map((e) => {
    const l = translated.get(e.version);
    return l && l.blocks.length ? l : e;
  });
}

const built = new Map<string, Entry[]>();
/** The bundled What's new, in the language shown. */
export function bundled(lang: string): Entry[] {
  const key = lang === "fr" ? "fr" : "en";
  let e = built.get(key);
  if (!e) built.set(key, (e = buildEntries(enMd, key === "fr" ? frMd : undefined)));
  return e;
}

// ---- Versions ----

/** "1.2.10" → [1, 2, 10]. Null for what isn't a version ("Next version", "1.2"). */
export function versionParts(text: unknown): number[] | null {
  if (typeof text !== "string") return null;
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(text.trim());
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** Negative when a is older than b, 0 when the same, positive when newer. */
export function compareVersions(a: number[], b: number[]): number {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

/** Every version newer than `seen` and not newer than `now`, newest first (an update can skip a
 * few: 1.1.0 → 1.2.0 shows 1.2.0 and 1.1.1). Headings that aren't versions never show; a version
 * written twice shows once. */
export function sinceSeen(entries: Entry[], seen: string, now: string): Entry[] {
  const n = versionParts(now);
  const s = versionParts(seen);
  if (!n || !s) return [];
  const taken = new Set<string>();
  return entries
    .map((e) => ({ e, v: versionParts(e.version) }))
    .filter((x): x is { e: Entry; v: number[] } => !!x.v && compareVersions(x.v, n) <= 0 && compareVersions(x.v, s) > 0)
    .filter(({ v }) => !taken.has(v.join(".")) && !!taken.add(v.join(".")))
    .sort((a, b) => compareVersions(b.v, a.v))
    .map((x) => x.e);
}

/** The running version's own section; else the newest one not newer than it (a build between
 * two releases); else none. */
export function ownSection(entries: Entry[], now: string): Entry[] {
  const n = versionParts(now);
  if (!n) return [];
  const older = entries
    .map((e) => ({ e, v: versionParts(e.version) }))
    .filter((x): x is { e: Entry; v: number[] } => !!x.v && compareVersions(x.v, n) <= 0)
    .sort((a, b) => compareVersions(b.v, a.v));
  return older.length ? [older[0].e] : [];
}

/** The setting's value for an update from a version that can't be told (whatsnew.rs). */
export const UNKNOWN = "unknown";

export interface Shown {
  entries: Entry[];
  /** The version you had, when several show ("What's new since Dobo 1.1.0"). */
  since: string | null;
}

/** What a start shows, from `seen` (the setting) and `now` (the running version):
 * - `show`: an update; these sections, then `now` is noted;
 * - `note`: nothing to show, but `now` is noted (a first install: no setting yet; or an update
 *   whose versions have no words);
 * - `none`: nothing to do (already seen, or a newer one was: gone back to an older version). */
export function planAfterStart(seen: unknown, now: string, entries: Entry[]): ({ kind: "show" } & Shown) | { kind: "note" } | { kind: "none" } {
  const n = versionParts(now);
  if (!n) return { kind: "none" };
  if (seen == null || seen === "") return { kind: "note" };
  const s = versionParts(seen);
  if (s && compareVersions(s, n) >= 0) return { kind: "none" };
  // (From a version that can't be told, or a value that can't be read: only this version's own
  // part, never an older one's.)
  const shown = s ? sinceSeen(entries, seen as string, now) : entries.filter((e) => versionParts(e.version)?.join(".") === n.join(".")).slice(0, 1);
  if (!shown.length) return { kind: "note" };
  return { kind: "show", entries: shown, since: s && shown.length > 1 ? s.join(".") : null };
}

/** What's around as the start's What's new comes to show (a moment after the panel shows). */
export interface StartGates {
  /** The window is hidden, or only its strip of titles shows (Peek), or it isn't shown yet. */
  hidden: boolean;
  /** The first-run window or the terms are up (What's new comes after them). */
  setup: boolean;
  /** Something covers the board (a view, the tour) or a line is carried. */
  covered: boolean;
  /** Words are being typed (a field with something in it has the keyboard). */
  typing: boolean;
}

/** What to do then, for a start's plan (planAfterStart): "show" it (and note it), "note" this
 * version without showing anything, "done" (nothing to do), or "wait" for the next time the panel
 * shows (never over the first-run window, the terms, a view, the tour, a carry, or someone
 * typing). */
export function whatsNewStep(plan: ReturnType<typeof planAfterStart>["kind"], g: StartGates): "show" | "note" | "done" | "wait" {
  if (g.hidden || g.setup) return "wait";
  if (plan === "none") return "done";
  if (plan === "note") return "note";
  if (g.covered || g.typing) return "wait";
  return "show";
}

/** The view's title: one version's own, or, for several, since the one you had. */
export function titleOf(shown: Shown, say: (key: "What’s new since {app} {v}" | "What’s new in {app} {v}", p: { app: string; v: string }) => string): string {
  if (shown.since && shown.entries.length > 1) return say("What’s new since {app} {v}", { app: APP_NAME, v: shown.since });
  return say("What’s new in {app} {v}", { app: APP_NAME, v: shown.entries[0]?.version ?? "" });
}

// ---- Links ----

/** A link's place as people read it, for its tooltip ("dobo.sookera.com/fr/help/…#mises-à-jour",
 * not "%C3%A0"); the link itself stays encoded. */
export function linkTip(url: string): string {
  let u = url;
  try {
    u = decodeURI(url);
  } catch {
    /* as it is */
  }
  return u.replace(/^https:\/\//, "");
}

/** Dobo's website. */
export const SITE = "https://dobo.sookera.com";

/** Where a link opens: a guide ("the-board.md", "install-and-update.md#updates") on Dobo's
 * website in the language shown (dobo.sookera.com/help/the-board, /fr/help/the-board); a web
 * page as it is (http and https only). Anything else opens nothing. */
export function linkTarget(href: string, lang: string): string | null {
  if (/^https?:\/\/[^\s]+$/i.test(href)) return href;
  const m = /^([a-z0-9][a-z0-9-]*)\.md(?:#(.+))?$/i.exec(href);
  if (!m) return null;
  const page = m[1].toLowerCase() === "readme" ? "" : `/${m[1].toLowerCase()}`;
  return `${SITE}${lang === "fr" ? "/fr" : ""}/help${page}${m[2] ? `#${encodeURIComponent(m[2])}` : ""}`;
}

/** Every version, on the website. */
export const whatsNewPage = (lang: string) => linkTarget("whats-new.md", lang)!;
