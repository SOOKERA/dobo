// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A line, or a whole note, as plain text for the clipboard (Copy text, Copy as text): a note's
// title, its tasks as a checklist ("- [ ]", "- [x]", the way most notes apps and Markdown read
// them pasted), their steps under them, then its ideas.
import type { Item } from "./types";

/** A line's words, then what is attached to it, each on its own line: its link's address (when
 * the words don't already hold it), its file's path. */
export function lineText(it: Pick<Item, "text" | "link" | "file">): string {
  const text = it.text.trim();
  const url = it.link?.url;
  const more = [url && !text.includes(url) ? url : null, it.file?.trim() || null].filter((x): x is string => !!x);
  return [text, ...more].filter(Boolean).join("\n");
}

/** A note as text: its title, its tasks (to do, then done, each with its steps), then its ideas
 * (with theirs), as the note shows them. */
export function noteAsText(title: string, lines: readonly Item[], stepsOf: (id: number) => readonly Item[]): string {
  const ordered = (list: readonly Item[]) => [...list.filter((i) => i.done_at == null), ...list.filter((i) => i.done_at != null)];
  // (A line's link or file, on the lines under it, lined up with its words to stay with it in a
  // list.)
  const under = (it: Item, head: string) => head + lineText(it).replace(/\n/g, "\n" + " ".repeat(head.length));
  const task = (it: Item, indent = "") => under(it, `${indent}- [${it.done_at != null ? "x" : " "}] `);
  const withSteps = (it: Item, head: string) => [head, ...ordered(stepsOf(it.id)).map((s) => task(s, "  "))];
  const tasks = ordered(lines.filter((i) => i.kind === "task")).flatMap((it) => withSteps(it, task(it)));
  const ideas = lines.filter((i) => i.kind === "idea").flatMap((it) => withSteps(it, under(it, "- ")));
  return [title.trim(), tasks.join("\n"), ideas.join("\n")].filter(Boolean).join("\n\n");
}
