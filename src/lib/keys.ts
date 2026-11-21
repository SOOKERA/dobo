// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The whole board by keyboard (Notes board):
//   Up / Down        move between lines (Up from the first line: the note itself)
//   Left / Right     move between notes
//   Space            tick the task
//   Enter / F2       edit the line (on the note: rename it)
//   Backspace        task <-> idea
//   Delete           delete the line
//   Ctrl+Up / Down   move the line
//   F                fold or unfold the note (the Inbox strip too)
//   Ctrl+Z / Ctrl+Y  undo / redo (also Ctrl+Shift+Z)
//   Esc              leave the board
// Down in the empty entry line steps onto the board. The Inbox strip comes first (its head, then
// its lines while it is open), then the notes; Enter on its head opens or folds it.
import { store } from "./store.svelte";
import { inboxLines } from "./inbox";
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
 * Svelte makes the leaving one inert. */
const LIVE = ":not([inert], [inert] *)";
export const liveLine = (id: number) => `[data-line="${id}"]${LIVE}`;
export const liveNote = (id: number) => `[data-drop-note="${id}"]${LIVE}`;

function reveal() {
  const f = store.focus;
  if (!f) return;
  const el = document.querySelector(f.item != null ? liveLine(f.item) : isStrip(f.note) ? "[data-inbox-strip]" : liveNote(f.note));
  el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
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

/** Undo / redo (Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z), anywhere outside a text field (inside one, the
 * field's own undo works); returns true when it used the key. Also on the Archive page, whose
 * Delete and Empty archive say Ctrl+Z brings them back. */
export function undoKey(e: KeyboardEvent): boolean {
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && !e.altKey && !typing()) {
    const k = e.key.toLowerCase();
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

  switch (e.key) {
    case "ArrowDown":
      if (ctrl && line) {
        void store.moveLine(line, 1);
        return true;
      }
      if (li + 1 < lines.length) store.focus = { note: note.id, item: lines[li + 1].id };
      else if (notes[ni + 1]) store.focus = { note: notes[ni + 1].id, item: null };
      reveal();
      return true;
    case "ArrowUp":
      if (ctrl && line) {
        void store.moveLine(line, -1);
        return true;
      }
      if (li > 0) store.focus = { note: note.id, item: lines[li - 1].id };
      else if (li === 0) store.focus = { note: note.id, item: null };
      else if (notes[ni - 1]) {
        const prev = notes[ni - 1].lines;
        store.focus = { note: notes[ni - 1].id, item: prev[prev.length - 1]?.id ?? null };
      }
      reveal();
      return true;
    case "ArrowLeft":
    case "ArrowRight": {
      const to = notes[ni + (e.key === "ArrowRight" ? 1 : -1)];
      if (!to) return true;
      const toLines = to.lines;
      const keep = li < 0 ? null : (toLines[Math.min(li, toLines.length - 1)]?.id ?? null);
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
    case "F":
      if (ctrl || e.altKey) return false;
      if (note.inbox) store.toggleInbox();
      else {
        const n = store.notes.find((x) => x.id === note.id);
        if (n) void store.setFolded(n, !n.folded);
      }
      store.focus = { note: note.id, item: null };
      return true;
    case "Escape":
      store.focus = null;
      return true;
  }
  return false;
}
