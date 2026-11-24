// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { isKey, t, tMaybe, type Key } from "./i18n.svelte";

// What an undo (or a redo) did, said the way people say it ("Task back to do", not "Undone:
// Done"). Changes without their own words keep "Undone: {what}" / "Redone: {what}".
const BACK: Partial<Record<Key, [Key, Key]>> = {
  Done: ["Task back to do", "Task ticked again"],
  "Not done": ["Task ticked again", "Task back to do"],
  "Task added": ["Task taken out", "Task added again"],
  "Idea added": ["Idea taken out", "Idea added again"],
  "Step added": ["Step taken out", "Step added again"],
  Deleted: ["Task put back", "Task deleted again"],
  "Idea deleted": ["Idea put back", "Idea deleted again"],
  "Note added": ["Note taken out", "Note added again"],
  "Note made from a line": ["Line back in its place", "Note made again"],
  "Note deleted": ["Note put back", "Note deleted again"],
  Archived: ["Note out of the archive", "Note archived again"],
  Restored: ["Note back in the archive", "Note back on the board"],
  "Archive emptied": ["Archived notes put back", "Archive emptied again"],
  "Made an idea": ["A task again", "An idea again"],
  "Made a task": ["An idea again", "A task again"],
  Folded: ["Note unfolded", "Note folded"],
  Unfolded: ["Note folded", "Note unfolded"],
  "Made wide": ["Note back to its width", "Note made wide again"],
  "Made normal": ["Note made wide again", "Note back to its width"],
  "Moved to Work": ["Back in Personal", "In Work again"],
  "Moved to Personal": ["Back in Work", "In Personal again"],
  Edited: ["Edit undone", "Edit redone"],
  "Edited with a time": ["Edit undone", "Edit redone"],
  Renamed: ["Old name back", "New name back"],
  "Colour changed": ["Old colour back", "New colour back"],
  "Paper changed": ["Old paper back", "New paper back"],
  "Line moved": ["Line back in its place", "Line moved again"],
  Moved: ["Line back in its place", "Line moved again"],
  "Time set": ["Time change undone", "Time change redone"],
  "Time removed": ["Time change undone", "Time change redone"],
  "Repeat set": ["Repeat change undone", "Repeat change redone"],
  "Repeat removed": ["Repeat change undone", "Repeat change redone"],
  "Deadline set": ["Reminder change undone", "Reminder change redone"],
  "Reminder set": ["Reminder change undone", "Reminder change redone"],
};
export function backWords(label: string | (() => string), way: 0 | 1): string {
  if (typeof label === "string" && isKey(label) && BACK[label]) return t(BACK[label]![way]);
  const what = typeof label === "function" ? label() : tMaybe(label);
  return t(way ? "Redone: {what}" : "Undone: {what}", { what });
}
