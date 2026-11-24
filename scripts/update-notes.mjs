// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//
// The notes of an update in latest.json (shown in Settings › Updates while it is ready), for
// scripts/release.ps1: the first paragraph of the version's section in docs/help/whats-new.md,
// then the French one from docs/help/fr/whats-new.md after a plain line, "En français : ". Dobo
// 1.2 and later show the reader's language (src/lib/updateWords.ts, notesIn); Dobo 1.1 shows the
// text as it is, which still reads well: the English, then the French, said as such.
//
//   node scripts/update-notes.mjs <version> <out file>      (written in UTF-8)
//
// Exit code: 0 written, 1 no such section (or no paragraph in it) in the English file, 2 wrong
// arguments. introOf and updateNotes are pure (scripts/update-notes.test.mjs).
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** What comes before the French part (U+00A0 before the colon, as French typography wants). */
export const FRENCH = "\n\nEn français : ";

/** The first paragraph of a version's section ("## 1.2.0" or "## Dobo 1.2.0"): not a heading,
 * not a list; its bold marks taken out, its lines joined. Null when there is none. */
export function introOf(md, version) {
  const lines = md.replace(/\r\n?/g, "\n").split("\n");
  const head = new RegExp(`^##[ \\t]+(Dobo[ \\t]+)?${version.replace(/\./g, "\\.")}[ \\t]*$`);
  const at = lines.findIndex((l) => head.test(l));
  if (at < 0) return null;
  const end = lines.findIndex((l, i) => i > at && /^##[ \t]/.test(l));
  const body = lines.slice(at + 1, end < 0 ? undefined : end).join("\n");
  const para = body
    .split(/\n[ \t]*\n/)
    .map((p) => p.trim())
    .find((p) => p && !p.startsWith("#") && !p.startsWith("-"));
  // (Only plain spaces and line ends are joined: French keeps its U+00A0 and U+202F.)
  return para ? para.replace(/\*\*/g, "").replace(/[ \t\n]+/g, " ").trim() : null;
}

/** The notes: the English intro, then the French one when there is one. */
export function updateNotes(en, fr, version) {
  const e = introOf(en, version);
  if (!e) return null;
  const f = fr ? introOf(fr, version) : null;
  return f ? `${e}${FRENCH}${f}` : e;
}

if (import.meta.main) {
  const [version, out] = process.argv.slice(2);
  if (!version || !out) {
    console.error("Usage: node scripts/update-notes.mjs <version> <out file>");
    process.exit(2);
  }
  const help = join(import.meta.dirname, "..", "docs", "help");
  const notes = updateNotes(readFileSync(join(help, "whats-new.md"), "utf8"), readFileSync(join(help, "fr", "whats-new.md"), "utf8"), version);
  if (!notes) {
    console.error(`docs/help/whats-new.md has no first paragraph under '## ${version}'.`);
    process.exit(1);
  }
  writeFileSync(out, notes, "utf8");
}
