// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// English where the key alone is not the words: plurals ({ one, other }) and keys that carry a
// note to tell two uses apart ("Every (weeks)" shows "Every"). Every other English string is its
// own key (see ../i18n.svelte.ts).
import type { Entry, Key } from "../i18n.svelte";

export const en: Partial<Record<Key, Entry>> = {
  "Added {n} notes from Sticky Notes.": { one: "Added {n} note from Sticky Notes.", other: "Added {n} notes from Sticky Notes." },
  "{n} ideas": { one: "{n} idea", other: "{n} ideas" },
  "{n} results": { one: "{n} result", other: "{n} results" },
  "{n} tasks": { one: "{n} task", other: "{n} tasks" },
  "{a} of {n} steps done": { one: "{a} of {n} step done", other: "{a} of {n} steps done" },
  "Every (weeks)": "Every",
  "day(s)": { one: "day", other: "days" },
  "week(s)": { one: "week", other: "weeks" },
  "month(s)": { one: "month", other: "months" },
  "Bring in your {n} Windows Sticky Notes?": { one: "Bring in your note from Windows Sticky Notes?", other: "Bring in your {n} Windows Sticky Notes?" },
  "Your {n} sticky notes are on the board now.": { one: "Your sticky note is on the board now.", other: "Your {n} sticky notes are on the board now." },
  "Start (button)": "Start",
  "Copied (text)": "Copied",
  "Stopped (problem)": "Stopped",
  "Couldn’t start (problem)": "Couldn’t start",
  "Dark (look)": "Dark",
  "Light (look)": "Light",
  "English (the language)": "English",
  "Too many reports from this connection. Try again in {n} minutes, or use Send by email.": {
    one: "Too many reports from this connection. Try again in {n} minute, or use Send by email.",
    other: "Too many reports from this connection. Try again in {n} minutes, or use Send by email.",
  },
  "{n} themes": { one: "{n} theme", other: "{n} themes" },
  "{n} styles": { one: "{n} style", other: "{n} styles" },
  "{n} mixes": { one: "{n} mix", other: "{n} mixes" },
  "{n} colours": { one: "{n} colour", other: "{n} colours" },
  "Deleted “{name}” (look)": "Deleted “{name}”",
  "{n} new": { one: "{n} new", other: "{n} new" },
  "Next (tour)": "Next",
  "folded (inbox strip)": "folded",
  "Done (tour)": "Done",
  "Deleted {n} archived notes": { one: "Deleted {n} archived note", other: "Deleted {n} archived notes" },
  "{n} notes put away, their reminders quiet. Restore or delete them there.": {
    one: "{n} note put away, its reminders quiet. Restore or delete it there.",
    other: "{n} notes put away, their reminders quiet. Restore or delete them there.",
  },
  "{n} notes": { one: "{n} note", other: "{n} notes" },
  "{n} lines": { one: "{n} line", other: "{n} lines" },
  "{n} notes match": { one: "{n} note matches", other: "{n} notes match" },
  "and {n} more": { one: "and {n} more", other: "and {n} more" },
  "and {n} more lines": { one: "and {n} more line", other: "and {n} more lines" },
  "{n} pictures": { one: "{n} picture", other: "{n} pictures" },
  "Delete all {n} archived notes?": { one: "Delete {n} archived note?", other: "Delete all {n} archived notes?" },
  "Small (size)": "Small",
  "Normal (size)": "Normal",
  "Large (size)": "Large",
  "Studio (sample note)": "Studio",
  // (As the French says it: "My theme (copy)".)
  "{name} copy": "{name} (copy)",
};
