// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The whole board by keyboard (Notes board):
//   Up / Down        move between lines (Up from the first line: the note itself; past the
//                    ends, the note below / above in the same column: spatial.ts)
//   Left / Right     move to the note in the next column, level with the line
//   Space            tick the task
//   Enter / F2       edit the line (on the note: rename it)
//   Backspace        task <-> idea
//   Delete           delete the line
//   Ctrl+Up / Down   move the line
//   F                fold or unfold the note (the Inbox strip too)
//   Shift+F10 / Menu the note's or the line's menu (as its ⋯; an idea's own short one)
//   Ctrl+Z / Ctrl+Y  undo / redo (also Ctrl+Shift+Z)
//   Esc              leave the board
// Down in the empty entry line steps onto the board. The Inbox strip comes first (its head, then
// its lines while it is open), then the notes; Enter on its head opens or folds it.
import { store } from "./store.svelte";
import { inboxLines } from "./inbox";
import { latinKey } from "./keyboard";
import { t } from "./i18n.svelte";
import { noteTitle } from "./names";
import { nextBox, type Box, type Way } from "./spatial";
import { askMenu, isMenuKey, openedByKey } from "./rightclick.svelte";
import type { Item } from "./types";

/** A note's lines in the order they show (tasks, done tasks, then ideas; steps under each). */
export function linesOf(noteId: number): Item[] {
  const note = store.notes.find((n) => n.id === noteId);
  if (!note || note.folded) return [];
  const top = store.itemsByNote.get(noteId) ?? [];
  const withSteps = (it: Item) => {
    const s = store.stepsOf.get(it.id) ?? [];
    return [it, ...s.filter((x) => x.done_at == null), ...s.filter((x) => x.done_at != null)];
  };
  const tasks = [...top.filter((i) => i.kind === "task" && i.done_at == null), ...top.filter((i) => i.kind === "task" && i.done_at != null)];
  const ideas = top.filter((i) => i.kind === "idea");
  return [...tasks.flatMap(withSteps), ...ideas.flatMap(withSteps)];
}

/** What the keys walk through: the Inbox strip (one stop, whichever space's Inbox), then the
 * notes on the board. */
interface Stop {
  id: number;
  inbox: boolean;
  lines: Item[];
}
export function stops(): Stop[] {
  // (A space without an Inbox yet still shows its strip, "empty": the stop is there too, as 0.)
  const list: Stop[] = [{ id: store.stripNotes[0]?.id ?? STRIP, inbox: true, lines: inboxLines() }];
  for (const n of store.boardNotes) list.push({ id: n.id, inbox: false, lines: linesOf(n.id) });
  return list;
}
/** The strip's stop when its space has no Inbox note yet (saved ids start at 1). */
export const STRIP = 0;
/** Is this focused "note" the Inbox strip? */
export const isStrip = (id: number): boolean => id === STRIP || store.inboxIds.has(id);

/** Not a copy on its way out: a line moved to another note is shown in both for a moment, and
 * Svelte makes the leaving one inert. (The board made inert under a view, `data-cover`, is still
 * the board.) */
const LIVE = ":not([inert]:not([data-cover]), [inert]:not([data-cover]) *)";
export const liveLine = (id: number) => `[data-line="${id}"]${LIVE}`;
export const liveNote = (id: number) => `[data-drop-note="${id}"]${LIVE}`;

function reveal() {
  const f = store.focus;
  if (!f) return;
  const el = document.querySelector(f.item != null ? liveLine(f.item) : isStrip(f.note) ? "[data-inbox-strip]" : liveNote(f.note));
  el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
}

/** Where the stops sit on screen (null for one that isn't shown). */
function boxesOf(list: Stop[]): (Box | null)[] {
  return list.map((s) => {
    const el = document.querySelector(s.inbox ? "[data-inbox-strip]" : liveNote(s.id));
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return r.width || r.height ? { left: r.left, right: r.right, top: r.top, bottom: r.bottom } : null;
  });
}
/** The middle height of a line on screen. */
function lineY(id: number): number | null {
  const el = document.querySelector(liveLine(id));
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return r.top + r.height / 2;
}
/** The stop an arrow leads to, by where the notes sit (`y`: the height of what has the keyboard),
 * or by their order when the board can't be measured. */
function stopThere(list: Stop[], from: number, way: Way, y: number | null): number | null {
  const boxes = boxesOf(list);
  const here = boxes[from];
  if (!here) {
    const i = from + (way === "right" || way === "down" ? 1 : -1);
    return i >= 0 && i < list.length ? i : null;
  }
  return nextBox(boxes, from, way, y ?? here.top + 20);
}

function focusFirst(): boolean {
  const n = stops()[0];
  if (!n) return false;
  // (The Inbox: its head first, whose Enter opens it.)
  store.focus = { note: n.id, item: n.inbox ? null : (n.lines[0]?.id ?? null) };
  reveal();
  return true;
}

function typing(): boolean {
  const a = document.activeElement as HTMLElement | null;
  return !!a && (a.tagName === "INPUT" || a.tagName === "TEXTAREA" || a.isContentEditable);
}

/** Where the board's keyboard is, in words, for screen readers (App says it, politely, once the
 * keys rest): "Task, not done: Call Léa · Logitech · 2 of 5", "Note: Logitech, folded", "Inbox,
 * 4 waiting". Empty off the board. */
export function focusWords(): string {
  const f = store.focus;
  if (!f) return "";
  const strip = isStrip(f.note);
  const note = strip ? null : store.notes.find((n) => n.id === f.note);
  if (!strip && !note) return "";
  if (f.item == null) {
    if (strip) {
      const n = store.inboxWaiting.length;
      return [t("Inbox"), n ? t("{n} waiting", { n }) : t("empty"), ...(store.inboxShown ? [] : [t("folded (inbox strip)")])].join(", ");
    }
    return t("{what}: {text}", { what: t("Note"), text: noteTitle(note) }) + (note!.folded ? `, ${t("folded")}` : "");
  }
  const lines = strip ? inboxLines() : linesOf(f.note);
  const i = lines.findIndex((l) => l.id === f.item);
  if (i < 0) return "";
  const it = lines[i];
  const done = it.done_at != null;
  const what = it.parent_id != null ? (done ? t("Step, done") : t("Step, not done")) : it.kind === "idea" ? t("Idea") : done ? t("Task, done") : t("Task, not done");
  return `${t("{what}: {text}", { what, text: it.text })} · ${strip ? t("Inbox") : noteTitle(note)} · ${t("{a} of {n}", { a: i + 1, n: lines.length })}`;
}

/** Undo / redo (Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z), anywhere outside a text field (inside one, the
 * field's own undo works); returns true when it used the key. Also on the Archive page, whose
 * Delete and Empty archive say Ctrl+Z brings them back. */
export function undoKey(e: KeyboardEvent): boolean {
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && !e.altKey && !typing()) {
    const k = latinKey(e);
    if (k === "z" && !e.shiftKey) {
      void store.undo();
      return true;
    }
    if (k === "y" || (k === "z" && e.shiftKey)) {
      void store.redo();
      return true;
    }
  }
  return false;
}

/** Handles a key for the board; returns true when it used it. */
export function boardKey(e: KeyboardEvent): boolean {
  const ctrl = e.ctrlKey || e.metaKey;
  if (undoKey(e)) return true;

  // From the empty entry line, Down steps onto the board.
  const a = document.activeElement as HTMLInputElement | null;
  if (e.key === "ArrowDown" && a?.id === "capture" && !a.value) {
    a.blur();
    return focusFirst();
  }
  if (typing()) return false;
  // (A button reached with Tab, such as a note's "+ Add", takes its own Enter and Space.)
  if ((e.key === "Enter" || e.key === " ") && a && a !== document.body && a.matches("button, a[href], select, summary, [role='button'], [role='menuitem'], [role='option'], [role='switch']")) return false;

  const f = store.focus;
  if (!f) {
    if (e.key === "ArrowDown" || e.key === "ArrowRight") return focusFirst();
    return false;
  }
  const notes = stops();
  const ni = notes.findIndex((n) => n.id === f.note || (n.inbox && isStrip(f.note)));
  if (ni < 0) {
    store.focus = null;
    return false;
  }
  const note = notes[ni];
  const lines = note.lines;
  const li = f.item == null ? -1 : lines.findIndex((l) => l.id === f.item);
  const line = li >= 0 ? lines[li] : null;

  switch (e.key.length === 1 && latinKey(e) === "f" ? "f" : e.key) {
    case "ArrowDown":
      if (ctrl && line) {
        void store.moveLine(line, 1);
        return true;
      }
      if (li + 1 < lines.length) store.focus = { note: note.id, item: lines[li + 1].id };
      else {
        // Past the last line: the note right below, in the same column.
        const to = stopThere(notes, ni, "down", null);
        if (to != null) store.focus = { note: notes[to].id, item: null };
      }
      reveal();
      return true;
    case "ArrowUp":
      if (ctrl && line) {
        void store.moveLine(line, -1);
        return true;
      }
      if (li > 0) store.focus = { note: note.id, item: lines[li - 1].id };
      else if (li === 0) store.focus = { note: note.id, item: null };
      else {
        // From the note's title: the last line of the note right above (or the Inbox strip).
        const to = stopThere(notes, ni, "up", null);
        if (to != null) {
          const prev = notes[to].lines;
          store.focus = { note: notes[to].id, item: prev[prev.length - 1]?.id ?? null };
        }
      }
      reveal();
      return true;
    case "ArrowLeft":
    case "ArrowRight": {
      // The note in the next column across, level with the line (from the Inbox strip, across
      // the whole board, → goes down to the first note).
      const right = e.key === "ArrowRight";
      const y = line ? lineY(line.id) : null;
      let ti = stopThere(notes, ni, right ? "right" : "left", y);
      if (ti == null && note.inbox && right) ti = stopThere(notes, ni, "down", null);
      if (ti == null) return true;
      const to = notes[ti];
      // On a line: the line across from it; on a title: the title.
      let keep: number | null = null;
      if (line && to.lines.length) {
        let best = Infinity;
        for (const l of to.lines) {
          const ly = lineY(l.id);
          const d = ly == null || y == null ? Infinity : Math.abs(ly - y);
          if (d < best) [best, keep] = [d, l.id];
        }
        keep ??= to.lines[Math.min(li, to.lines.length - 1)].id;
      }
      store.focus = { note: to.id, item: keep };
      reveal();
      return true;
    }
    case " ":
    case "Spacebar":
      if (line?.kind === "task") void store.toggleDone(line);
      return true;
    case "F2":
    case "Enter": {
      // The Inbox's head: Enter opens or folds it (it has no name to change).
      if (!line && note.inbox) {
        if (e.key === "Enter") store.toggleInbox();
        return true;
      }
      const sel = line ? `${liveLine(line.id)} ${line.kind === "idea" ? ".idea-text" : ".text"}` : `${liveNote(note.id)} .head`;
      document.querySelector(sel)?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      return true;
    }
    case "Backspace":
      if (line && !line.parent_id) void store.flipKind(line);
      return true;
    case "Delete":
      if (line) {
        const after = lines[li + 1] ?? lines[li - 1] ?? null;
        void store.deleteItem(line);
        store.focus = { note: note.id, item: after && after.id !== line.id ? after.id : null };
      }
      return true;
    case "f":
      if (ctrl || e.altKey) return false;
      if (note.inbox) store.toggleInbox();
      else {
        const n = store.notes.find((x) => x.id === note.id);
        if (n) void store.setFolded(n, !n.folded);
      }
      store.focus = { note: note.id, item: null };
      return true;
    case "ContextMenu":
    case "F10":
      // Its menu, as its ⋯ would open it (the Inbox strip's head has none).
      if (!isMenuKey(e) || (note.inbox && !line)) return false;
      askMenu(line ? "line" : "note", line?.id ?? note.id, null);
      openedByKey();
      return true;
    case "Escape":
      store.focus = null;
      return true;
  }
  return false;
}
