// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// What a typed line means: its text, a time (French or English: "15h", "demain 9h", "3pm",
// "tomorrow") and a #group naming the note it goes into.
import * as chrono from "chrono-node";
import { findRepeat, firstOccurrence } from "./repeat";
import type { Kind, Note, Repeat, Space } from "./types";
import { noteTitle } from "./names";

export interface Parsed {
  text: string;
  due: Date | null;
  tag: string | null;
  /** The line ended with "!": a deadline (the reminder rhythm) instead of a plain reminder. */
  deadline: boolean;
  /** "every Monday", "tous les jours"… */
  repeat: Repeat | null;
}

const TAG = /(^|\s)#([\p{L}\p{N}_\-·]+)/u;

export function normalize(s: string): string {
  return s
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "");
}

// French words chrono doesn't know: "midi", "minuit" (never the "midi" of "après-midi") and
// "après-demain" (else read as "demain", a day early).
const fr = chrono.fr.casual.clone();
fr.parsers.push(
  {
    pattern: () => /(?<![\p{L}\p{N}-])(midi|minuit)(?![\p{L}\p{N}])/iu,
    extract: (_c, m) => (m[1].toLowerCase() === "midi" ? { hour: 12, minute: 0 } : { hour: 0, minute: 0 }),
  },
  {
    pattern: () => /(?<![\p{L}\p{N}])apr[eè]s[\s-]?demain(?![\p{L}\p{N}])/iu,
    extract: (c) => {
      const d = new Date(c.refDate);
      d.setDate(d.getDate() + 2);
      return { day: d.getDate(), month: d.getMonth() + 1, year: d.getFullYear() };
    },
  },
);

/** The time a result means. A day without an hour ("tomorrow") is 09:00; a part of the day
 * ("tonight", "ce soir", "this afternoon") keeps the hour it implies, today's even once that hour
 * has passed (then the next full hour). A bare "at 3" (no am/pm, no minutes, no "h") is the
 * afternoon, never 3 in the night. */
function dueOf(r: chrono.ParsedResult, ref: Date): Date {
  const d = r.start.date();
  if (!r.start.isCertain("hour")) {
    if (!/soir|evening|tonight|night|nuit|apr[eè]s-?midi|afternoon/i.test(r.text)) {
      d.setHours(9, 0, 0, 0);
      return d;
    }
    d.setMinutes(0, 0, 0);
    if (/^(?:ce|cet|cette|this)\s|^tonight/i.test(r.text.trim())) {
      const today = new Date(ref.getFullYear(), ref.getMonth(), ref.getDate(), d.getHours());
      if (today.getTime() > ref.getTime()) return today;
      const soon = new Date(ref);
      soon.setHours(soon.getHours() + 1, 0, 0, 0);
      return soon;
    }
  } else if (!r.start.isCertain("meridiem") && /(?:^|\s)(?:at|@|à|a)\s*[1-6](?:$|\s)/i.test(r.text)) {
    const h = d.getHours();
    if (h >= 1 && h <= 6) {
      // No day written ("call at 3"): today's afternoon while it's still to come.
      if (!r.start.isCertain("day") && !r.start.isCertain("weekday")) {
        const today = new Date(ref.getFullYear(), ref.getMonth(), ref.getDate(), h + 12, d.getMinutes());
        if (today.getTime() <= ref.getTime()) today.setDate(today.getDate() + 1);
        return today;
      }
      d.setHours(h + 12);
    }
  }
  return d;
}

function bestTime(text: string, ref: Date): chrono.ParsedResult | null {
  const opts = { forwardDate: true };
  const found = [...fr.parse(text, ref, opts), ...chrono.en.parse(text, ref, opts)];
  // Bare numbers ("2 bags") are not times.
  const real = found.filter((r) => /[a-zà-ÿ:]/i.test(r.text));
  if (!real.length) return null;
  return real.sort((a, b) => b.text.length - a.text.length)[0];
}

/** `tags: false` (a line added in a note itself): a #word stays in the text. */
export function parseCapture(raw: string, kind: Kind, ref = new Date(), { tags = true }: { tags?: boolean } = {}): Parsed {
  let text = raw;
  let tag: string | null = null;
  const m = tags ? text.match(TAG) : null;
  if (m) {
    tag = m[2];
    text = text.replace(m[0], m[1]);
  }
  let due: Date | null = null;
  let deadline = false;
  let repeat: Repeat | null = null;
  if (kind === "task") {
    const bang = text.match(/\s*!\s*$/);
    if (bang && text.trim().length > 1) {
      deadline = true;
      text = text.slice(0, bang.index);
    }
    const rep = findRepeat(text);
    if (rep) {
      repeat = rep.rule;
      text = text.slice(0, rep.index) + " " + text.slice(rep.index + rep.length);
    }
    const r = bestTime(text, ref);
    if (r) {
      due = dueOf(r, ref);
      text = text.slice(0, r.index) + text.slice(r.index + r.text.length);
      // Leftover joining words: "Call Chanel à" / "Call Chanel at" (or the whole line: "le").
      text = text.replace(/(^|\s+)(à|a|at|le|on|pour|for)\s*$/i, "");
    }
    if (repeat) {
      // The first occurrence, at the time typed (else 09:00).
      const h = due ? due.getHours() : 9;
      const m = due ? due.getMinutes() : 0;
      due = firstOccurrence(repeat, h, m, ref);
      if (repeat.freq === "week" && !repeat.days?.length) repeat = { ...repeat, days: [due.getDay()] };
      if (repeat.freq === "month" && !repeat.monthday) repeat = { ...repeat, monthday: due.getDate() };
    }
  }
  text = text.replace(/\s{2,}/g, " ").replace(/\s+([,.;:])/g, "$1").trim().replace(/[,;:]$/, "").trim();
  return { text, due, tag, deadline, repeat };
}

export interface FoundDate {
  index: number;
  length: number;
  due: Date;
  /** Clear enough to set by itself ("demain 9h", "lundi 14h30", "tomorrow 3pm"). */
  certain: boolean;
}

/** A date or time written inside a line (smart dates in notes); the text is left as it is. */
export function findDate(text: string, ref = new Date()): FoundDate | null {
  const r = bestTime(text, ref);
  if (!r) return null;
  const d = dueOf(r, ref);
  const hour = r.start.isCertain("hour");
  const day = r.start.isCertain("day") || r.start.isCertain("weekday") || /\b(demain|tomorrow|today|aujourd|ce soir|tonight)/i.test(r.text);
  return { index: r.index, length: r.text.length, due: d, certain: hour && day };
}

/** The note a #tag names: its whole name first, then a name starting with it, then one containing
 * it (case, accents, spaces and punctuation ignored); at each of those, a note of `space` (the
 * one new lines go to) before the others. A note is known by its name underneath and by the name
 * it shows ("Inbox" is also "À trier" in French). */
export function findNoteForTag(tag: string, notes: Note[], space?: Space): Note | null {
  const t = normalize(tag);
  if (!t) return null;
  // (In the board's order, as the notes are read at start and as the # list shows them: after a
  // card is dragged the list in memory keeps its old order.)
  const live = notes.filter((n) => !n.archived).sort((a, b) => a.sort - b.sort || a.id - b.id);
  const ordered = space ? [...live.filter((n) => n.space === space), ...live.filter((n) => n.space !== space)] : live;
  const names = (n: Note) => [normalize(n.title), normalize(noteTitle(n))];
  return (
    ordered.find((n) => names(n).some((x) => x === t)) ??
    ordered.find((n) => names(n).some((x) => x.startsWith(t))) ??
    ordered.find((n) => names(n).some((x) => x.includes(t))) ??
    null
  );
}

// ---- # suggestions while typing ----

/** The #tag that names a note whole, in the words it shows: "Runway · Grand Palais" →
 * "runway-grand-palais" (letters and digits kept, accents too; anything else joins with "-").
 * findNoteForTag finds that same note from it (a whole-name match comes first). Empty when the
 * name has no letter or digit at all. */
export function tagOf(n: { title: string }): string {
  return noteTitle(n)
    .toLocaleLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");
}

/** The #word being typed at the caret: where its "#" is, where it ends, and what follows the "#"
 * so far. Null when the caret isn't in one (a "#" counts at the start or after a space). */
export function tagAt(text: string, caret: number): { start: number; end: number; query: string } | null {
  const before = text.slice(0, caret);
  const m = before.match(/(^|\s)#([\p{L}\p{N}_\-·]*)$/u);
  if (!m) return null;
  const start = caret - m[2].length - 1;
  // (The word goes on after the caret too, up to the next space.)
  const rest = text.slice(caret).match(/^[\p{L}\p{N}_\-·]*/u)?.[0] ?? "";
  return { start, end: caret + rest.length, query: m[2] };
}

/** The notes a #word being typed may mean, best first: the same order as findNoteForTag (whole
 * name, starts with, contains; `space`'s notes first at each), then the board's order. A bare "#"
 * offers every note. Notes whose tag would name another one (the same name twice) show once. */
export function suggestNotes(query: string, notes: Note[], space: Space, limit = 6): Note[] {
  const q = normalize(query);
  const live = notes.filter((n) => !n.archived && tagOf(n));
  const names = (n: Note) => [normalize(n.title), normalize(noteTitle(n))];
  const rank = (n: Note): number => {
    if (!q) return n.title === "Inbox" ? 3 : 2;
    const ns = names(n);
    return ns.some((x) => x === q) ? 0 : ns.some((x) => x.startsWith(q)) ? 1 : ns.some((x) => x.includes(q)) ? 2 : -1;
  };
  const seen = new Set<string>();
  return live
    .map((n) => ({ n, r: rank(n) }))
    .filter((x) => x.r >= 0)
    .sort((a, b) => a.r - b.r || Number(b.n.space === space) - Number(a.n.space === space) || a.n.sort - b.n.sort || a.n.id - b.n.id)
    .map((x) => x.n)
    .filter((n) => {
      const k = normalize(tagOf(n));
      if (seen.has(k)) return false;
      seen.add(k);
      return true;
    })
    .slice(0, limit);
}

/** The text with the #word at `at` replaced by the note's tag and one space after it; and where
 * the caret goes (after that space). */
export function completeTag(text: string, at: { start: number; end: number }, n: { title: string }): { text: string; caret: number } {
  const tag = `#${tagOf(n)}`;
  const after = text.slice(at.end);
  const glue = after.startsWith(" ") ? "" : " ";
  return { text: text.slice(0, at.start) + tag + glue + after, caret: at.start + tag.length + 1 };
}

export function titleFromTag(tag: string): string {
  const words = tag.replace(/[-_]+/g, " ").trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}
