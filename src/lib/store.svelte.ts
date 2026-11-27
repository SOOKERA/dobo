// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { backendPromise, onSaveFailed, type Backend } from "./backend";
import { findNoteForTag, parseCapture, titleFromTag, type Parsed } from "./capture";
import { defaultCustom, rotation, type CustomColour } from "./palette";
import { History, changedKeys, withFields, type Step as HistoryStep } from "./history";
import { dropFromSteps, dropNotesFromSteps, putBackInSteps, type SavedSteps, type Step } from "./arrange";
import { dayKey, defaultDay, inWorkHours, lookbackDay, spanDay, type DaySettings } from "./plan";
import { backWords } from "./undoWords";
import { SAMPLE_ITEM, inMemory } from "./sample";
import { i18n, keysText, readChoice, t as tr, tMaybe, type Key, type Words } from "./i18n.svelte";
import { noteTitle } from "./names";
import { APP_NAME } from "./app";
import { arrivedSince, firstInboxes, inboxesLost, isInbox, savedInboxes } from "./inboxes";
import { thumbs } from "./thumbs.svelte";
import { copyText } from "./contextmenu";
import { lineInNote, titleFromLine } from "./lineNote";
import { blankIds, blanksOf, isBlank, spaceForSpot, withoutBlank, type BlankIds, type BoardPoint } from "./newNote";
import { daysApart, nextOne, shiftDays, waitsUntil } from "./doneLog";
import type {
  Alarm,
  Done,
  Item,
  Kind,
  LinkPreview,
  Note,
  PaletteName,
  Paper,
  Picture,
  Repeat,
  RhythmChoice,
  RingAction,
  RingEvent,
  Space,
  SpaceView,
  WindowInfo,
} from "./types";

export type NoteSize = "compact" | "comfortable" | "large";
export const SIZE_ZOOM: Record<NoteSize, number> = { compact: 0.88, comfortable: 1, large: 1.15 };
export const SIZES: NoteSize[] = ["compact", "comfortable", "large"];

export interface TemplateLine {
  kind: Kind;
  text: string;
  /** A deadline (shown with "!"). */
  deadline?: boolean;
  steps?: string[];
}
export interface Template {
  id: number;
  name: string;
  palette: PaletteName;
  color: string;
  paper: Paper;
  /** Its notes put their done tasks at the end (else where they are). */
  done_last?: boolean;
  lines: TemplateLine[];
}

/** What the board does for a new note at a spot of it (Board.svelte). */
export interface BoardHands {
  /** A point of the window on the board, in the board's own pixels (null: off the board). */
  point(client: { x: number; y: number }): BoardPoint | null;
  /** The notes around a spot, in its column: the one a new note would sit under, the next. */
  around(at: BoardPoint): { above: number | null; below: number | null };
  /** The new note goes there: the board's steps saved with it in (that saving), and it grows in
   * from the spot. */
  place(id: number, at: BoardPoint): Promise<void> | void;
}

export interface Toast {
  id: number;
  text: Words;
  action?: { label: Words; run: () => void };
  /** A second button ("See archive"). */
  more?: { label: Words; run: () => void };
}

export interface WorkHours {
  on: boolean;
  /** 0 = Sunday … 6 = Saturday. */
  days: number[];
  from: string;
  until: string;
}
export const defaultWorkHours: WorkHours = { on: false, days: [1, 2, 3, 4, 5], from: "09:00", until: "19:00" };

export interface Shortcuts {
  toggle: string;
  dump: string;
  search: string;
}
export const defaultShortcuts: Shortcuts = { toggle: "Ctrl+Shift+Space", dump: "Ctrl+Shift+D", search: "Ctrl+F" };

const RING_ORDER: Record<string, number> = { check: 0, ringing: 1, early: 2 };

/** The reminder clock's own fields: never put back by an undo (the backend keeps its own). */
const RING_KEYS = new Set<keyof Item>(["ring_state", "rings", "last_ring_at", "snooze_until", "on_it_at", "check_at"]);

/** The list of nothing: one for all (a new empty list each time would look like a change). */
export const NO_ITEMS: Item[] = [];
export const NO_PICTURES: Picture[] = [];

/** Each list of `next` that holds the same things, in the same order, as its list in `prev` is
 * that old list again: what shows a list that didn't change (a note's lines) isn't redone. */
function keepSame<K, V>(prev: Map<K, V[]> | null, next: Map<K, V[]>): Map<K, V[]> {
  if (!prev) return next;
  for (const [k, list] of next) {
    const was = prev.get(k);
    if (was && was.length === list.length && was.every((v, i) => v === list[i])) next.set(k, was);
  }
  return next;
}

/** The same plain value (fields in any order). */
function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || !a || !b || Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  return ka.every((k) => Object.prototype.hasOwnProperty.call(b, k) && sameValue((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));
}

/** The rows read again: each one that is the same as before is the old object (and when all of
 * them are, in the same order, the old list). */
function keepRows<T extends { id: number }>(was: T[], next: T[]): T[] {
  const old = new Map(was.map((r) => [r.id, r]));
  let same = was.length === next.length;
  const out = next.map((r, i) => {
    const o = old.get(r.id);
    const keep = o && sameValue(o, r) ? o : r;
    if (keep !== was[i]) same = false;
    return keep;
  });
  return same ? was : out;
}

/** Unticking a repeating task's ticked copy (`prev`) came back as the open line it already was
 * (the board kept that copy from before it opened again): a real untick goes back to the time
 * ticked, with its count as it was; this one is still at its time, with its count. */
export function staleUntick(prev: Item, saved: Item): boolean {
  return prev.done_at != null && !!prev.repeat?.ticked && saved.done_at == null && !saved.repeat?.ticked && saved.due_at === prev.due_at && saved.repeat?.done === prev.repeat.done;
}

/** How long a note takes to fade off the board (Board.svelte's noteLeaves). */
const LEAVE_MS = 240;

class Store {
  // The notes, lines and pictures are plain objects, never changed in place: a change is always
  // a new object in a new list (replaceItem, upsertItem…). The board reads them as they are,
  // without a watched copy of every field of every line.
  notes = $state.raw<Note[]>([]);
  items = $state.raw<Item[]>([]);
  pictures = $state.raw<Picture[]>([]);
  /** Repeating tasks' history: each time one was done (the History card, the look back). */
  doneLog = $state.raw<Done[]>([]);
  settings = $state<Record<string, unknown>>({});
  view = $state<SpaceView>("all");
  win = $state<WindowInfo>({ mode: "docked", chosen: "docked", docked_possible: true, side: "right", float_on_top: false, width: 720 });
  loaded = $state(false);
  error = $state<string | null>(null);
  /** The note new lines go into when no #group is typed (null: the Inbox). */
  target = $state<number | null>(null);
  pinned = $state(false);
  now = $state(new Date());
  /** "Switched to Work for this reminder · Back to Personal". */
  banner = $state<{ space: Space; back: SpaceView } | null>(null);
  /** A note that just got jumped to (it glows for a moment). */
  glow = $state<{ id: number; at: number } | null>(null);
  /** Keyboard focus on the board: a note, and maybe one of its lines. */
  focus = $state<{ note: number; item: number | null } | null>(null);
  /** The short message at the bottom ("Deleted · Undo", "Undone · Redo"). */
  toast = $state<Toast | null>(null);
  /** The time ruler: open while a task is carried, or by "Set a time". */
  ruler = $state<{ item: Item; mode: "drag" | "click" | "demo"; at?: number | null } | null>(null);
  /** The Now card's "Later…" choices are open for this item. */
  laterFor = $state<number | null>(null);
  /** The narrow strip of titles at the screen edge is showing (not the whole panel). */
  peek = $state(false);
  /** The evening look back, opened from the menu (any time). */
  lookbackOpen = $state(false);
  /** The day it was opened for (its notification, clicked the next day: that day's), else null. */
  lookbackFor = $state<number | null>(null);
  templatesOpen = $state(false);
  /** The Archive page, and where it was opened from (Back goes there). */
  archiveOpen = $state<"board" | "settings" | null>(null);
  /** A full view covers the board (Settings, the Archive, Search, Tidy…; App.svelte keeps it). */
  over = $state(false);
  /** Notes put back from the archive while a view covers the board: their cards settle in once
   * the board shows again (Board.svelte). */
  arriving = $state<number[]>([]);
  /** The board has measured every note once (Board.svelte): it is laid out as it will show. */
  boardSettled = false;
  private settledWaiters: (() => void)[] = [];
  /** Resolves once the board has measured every note (at once when it has). */
  whenBoardSettled(): Promise<void> {
    return this.boardSettled ? Promise.resolve() : new Promise((r) => this.settledWaiters.push(r));
  }
  markBoardSettled() {
    if (this.boardSettled) return;
    this.boardSettled = true;
    for (const r of this.settledWaiters.splice(0)) r();
  }
  /** The board plays a note's way out (archived, deleted) before it goes; set by Board.svelte. */
  noteLeaves: ((ids: number[]) => Promise<void>) | null = null;
  /** A line lifts off its note as a slip, waits `after` ms there (the note fading away), then
   * glides back to wherever the line is by then (an undo of a note made from it); App.svelte. */
  lineLifts: ((itemId: number, after: number) => void) | null = null;
  /** The board's hands for a new note at a spot (Board.svelte): what is around a spot, and the
   * new note put there (the board's steps saved with it in). */
  board: BoardHands | null = null;
  /** A note just made from the board's right-click menu: its title waits to be typed. */
  fresh = $state<number | null>(null);
  /** Blank lines waiting in notes made from a template (the built-ins'), by note: shown while
   * Dobo runs, never saved (NoteCard). */
  blanks = $state.raw<Record<number, BlankIds>>({});
  /** "New note from a template…" from the board's right-click menu: where its note goes. */
  templateAt: BoardPoint | null = null;
  history = new History();
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  private api!: Backend;

  get day(): DaySettings {
    return { ...defaultDay, ...((this.settings.day as Partial<DaySettings>) ?? {}) };
  }

  get size(): NoteSize {
    const s = this.settings.note_size;
    return s === "compact" || s === "large" ? s : "comfortable";
  }

  get zoom(): number {
    return SIZE_ZOOM[this.size];
  }

  /** Dates written inside notes: "ask" (a chip offers the reminder), "auto" (when certain), "off". */
  get smartDates(): "ask" | "auto" | "off" {
    const s = this.settings.smart_dates;
    return s === "auto" || s === "off" ? s : "ask";
  }

  get templates(): Template[] {
    return Array.isArray(this.settings.templates) ? (this.settings.templates as Template[]) : [];
  }

  /** The day whose look back shows now (or today, when opened from the menu: the evening's
   * until it is closed, then today's so far; in the night part of work hours that cross
   * midnight, "today" is the day that night began, so what was ticked since it began is in it). */
  get lookback(): Date | null {
    const d = lookbackDay(this.now.getTime(), this.day, this.workHours);
    const closed = !!d && this.settings.lookback_dismissed === dayKey(d);
    if (this.lookbackOpen) return this.lookbackFor != null ? new Date(this.lookbackFor) : d && !closed ? d : spanDay(this.now.getTime(), this.workHours);
    return d && !closed ? d : null;
  }

  /** Copies from a menu (Copy text, Copy as text, Copy, Copy link), and says it did, briefly
   * ("Copied" never takes the place of a message with a button, an Undo), or that it couldn't. */
  async copy(what: string) {
    if (!(await copyText(what))) this.showToast(() => tr("Couldn’t copy"), undefined, 3200);
    else if (!this.toast?.action && !this.toast?.more) this.showToast(() => tr("Copied (text)"), undefined, 1800);
  }

  showToast(text: Words, action?: Toast["action"], ms = 5200, more?: Toast["more"]) {
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toast = { id: Date.now(), text, action, more };
    this.toastTimer = setTimeout(() => (this.toast = null), ms);
  }

  /** A shortcut set long ago on a key Dobo now uses inside (Ctrl+Shift+S, Ctrl+Shift+P, Ctrl+,),
   * one that would fire while typing (Shift+A), or Windows' menu key (Shift+F10), went back to its
   * default at start (Rust; search to a key no global shortcut uses):
   * said once, then forgotten. */
  private sayMovedShortcuts() {
    const moved = this.settings.shortcut_moved as { from: string; to: string; why?: string }[] | null | undefined;
    if (!Array.isArray(moved) || !moved.length) return;
    // Both may have moved (Open or hide and Quick dump): each is said.
    const say = (m: { from: string; to: string; why?: string }) =>
      m.why === "menu"
        ? tr("{old} is Windows’ menu key: your shortcut moved to {keys}", { old: keysText(m.from), keys: keysText(m.to) })
        : m.why === "typing"
          ? tr("{old} would fire while you type: your shortcut moved to {keys}", { old: keysText(m.from), keys: keysText(m.to) })
          : tr("{old} is used inside {app} now: your shortcut moved to {keys}", { old: keysText(m.from), keys: keysText(m.to), app: APP_NAME });
    this.showToast(() => moved.map(say).join(" · "), undefined, 10_000);
    void this.setSetting("shortcut_moved", null);
  }

  /** Global shortcuts another app holds (Rust, at start or after a restore): said once, with
   * where to choose another. */
  sayTakenShortcuts(list: unknown) {
    if (!Array.isArray(list) || !list.length) return;
    const keys = (list as { keys: string }[]).map((x) => keysText(String(x.keys)));
    this.showToast(() => keys.map((k) => tr("{keys} is taken by another app, so it does nothing now. Choose another in Settings › Keyboard", { keys: k })).join(" · "), undefined, 12_000);
  }

  async undo() {
    const label = await this.history.undo();
    if (label) this.showToast(() => backWords(label, 0), { label: () => tr("Redo"), run: () => void this.redo() });
    else this.showToast(() => tr("Nothing to undo"));
  }

  async redo() {
    const label = await this.history.redo();
    if (label) this.showToast(() => backWords(label, 1), { label: () => tr("Undo"), run: () => void this.undo() });
    else this.showToast(() => tr("Nothing to redo"));
  }

  /** Saves a line; with a label, the change can be undone. */
  private async putItem(next: Item, label: Key | null): Promise<Item> {
    // The tour's sample is never saved nor undone.
    if (inMemory(next.id) || inMemory(next.note_id) || inMemory(next.parent_id)) return next;
    // Plain copies (not the board's live objects), for the backend and for undo.
    const found = this.items.find((x) => x.id === next.id);
    const prev = found ? ($state.snapshot(found) as Item) : undefined;
    next = $state.snapshot(next) as Item;
    // (A ticked repeating task waiting for its next day: the change may open it at once, its
    // steps fresh; as they are now, for that Undo.)
    const steps = label && prev && waitsUntil(prev, this.day) != null ? this.snapshotItem(prev).items.filter((x) => x.id !== prev.id) : null;
    this.replaceItem(next);
    const had = this.doneLog;
    const shown = this.logTick(prev, next);
    const unshown = had.filter((r) => !this.doneLog.includes(r));
    const saved = await this.api.updateItem(next);
    this.replaceItem(saved);
    // (A tick the backend didn't take, the copy being from another moment of the line, db.rs
    // `not_stale`: no time of it in the history. An untick it didn't take: its time back.)
    if (shown != null && saved.done_at != null && saved.done_at !== next.done_at) this.doneLog = this.doneLog.filter((r) => r.id !== shown);
    if (unshown.length && saved.done_at != null) this.doneLog = [...this.doneLog, ...unshown.filter((r) => !this.doneLog.some((x) => x.id === r.id))];
    if (label && prev) {
      // (With what the backend changed with it: "Repeat removed" on a ticked task also takes it
      // back to the time ticked, and its Undo gives that time back too. Not when the backend
      // kept the line's own "done", time and rule (a copy from another moment, `not_stale`), nor
      // when the change opened it at once: those are not this change's, and an Undo of them
      // would undo what the copy never saw.)
      const kept = saved.done_at !== next.done_at || (saved.done_at == null && !!next.repeat?.ticked && !saved.repeat?.ticked);
      const keys = [...new Set([...changedKeys(prev, next), ...(kept ? [] : changedKeys(prev, saved))])].filter((k) => !RING_KEYS.has(k));
      this.history.record({ label, undo: () => (steps ? this.backToWaiting(prev, keys, steps) : this.revertItem(prev, keys)), redo: () => this.revertItem(next, keys) });
    }
    return saved;
  }

  /** Undo of a change to a ticked repeating task waiting for its next day (`prev`; `steps`, its
   * steps then). When that change opened it at once (its time set earlier today), it goes back
   * to waiting exactly as it was: ticked, its time, its rule and count, its steps as they were,
   * its history as it is (never a second time in it), all in one go (`undoTick`). Otherwise
   * (still waiting, open on its own day since, ticked again), the change alone is undone. */
  private async backToWaiting(prev: Item, keys: (keyof Item)[], steps: Item[]) {
    const now = this.items.find((x) => x.id === prev.id);
    const opens = waitsUntil(prev, this.day);
    if (now && now.done_at == null && opens != null && Date.now() < opens && (now.repeat?.done ?? 0) === (prev.repeat?.done ?? 0)) {
      const want = withFields($state.snapshot(now) as Item, prev, [...new Set<keyof Item>([...keys, "done_at", "due_at", "repeat"])]);
      return this.undoTick(want, null, steps, prev.repeat?.ticked?.due ?? null);
    }
    return this.revertItem(prev, keys);
  }

  /** An undo or redo of a change to a line: only the fields that change made, onto the line as it
   * is now (`withFields`). */
  private async revertItem(want: Item, keys: (keyof Item)[]) {
    const now = this.items.find((x) => x.id === want.id);
    if (!now) return void (await this.putItem(want, null));
    const line = withFields($state.snapshot(now) as Item, want, keys);
    // (A change made while it waited ticked for its next day, undone or redone once it is open
    // again: its time and its rule, never its tick. An open line never takes a `ticked`, db.rs
    // `not_stale`, which would keep the line and lose the Undo; nor a tick from it: it would be
    // open by now anyway.)
    if (now.done_at == null && line.repeat?.ticked) {
      const repeat = { ...line.repeat };
      delete repeat.ticked;
      line.repeat = repeat;
      line.done_at = null;
    }
    await this.putItem(line, null);
  }

  /** The same for a note. */
  private async revertNote(want: Note, keys: (keyof Note)[]) {
    const now = this.notes.find((x) => x.id === want.id);
    await this.putNote(now ? withFields($state.snapshot(now) as Note, want, keys) : want, null);
  }

  private async putNote(next: Note, label: Key | null): Promise<Note> {
    if (inMemory(next.id)) return next;
    const found = this.notes.find((x) => x.id === next.id);
    const prev = found ? ($state.snapshot(found) as Note) : undefined;
    const had = savedInboxes(this.notes);
    next = $state.snapshot(next) as Note;
    this.replaceNote(next);
    const saved = await this.api.updateNote(next);
    this.replaceNote(saved);
    await this.keepInboxes(had);
    if (label && prev) {
      const keys = changedKeys(prev, next);
      this.history.record({ label, undo: () => this.revertNote(prev, keys), redo: () => this.revertNote(next, keys) });
    }
    return saved;
  }

  /** Puts back lines (with their steps and pictures) and notes after a delete. */
  private async restoreAll(notes: Note[], items: Item[], pictures: Picture[]) {
    notes = notes.filter((n) => !inMemory(n.id));
    items = items.filter((i) => !inMemory(i.id) && !inMemory(i.note_id));
    const had = savedInboxes(this.notes);
    await this.api.restore(notes, items, pictures);
    const has = (list: { id: number }[], id: number) => list.some((x) => x.id === id);
    this.notes = [...this.notes.filter((n) => !has(notes, n.id)), ...notes];
    this.items = [...this.items.filter((i) => !has(items, i.id)), ...items];
    this.pictures = [...this.pictures.filter((p) => !has(pictures, p.id)), ...pictures];
    await this.keepInboxes(had);
  }

  /** A space whose Inbox just went (an undo archived, renamed or moved it, a redo deleted it)
   * gets one back at once: the next note called "Inbox", else a new one (the backend's
   * `ensure_inbox`, which keeps the same rule). Dumps, the strip and the tray's count never find
   * a space without its Inbox. (Saved notes only: the tour's sample Inbox going away, or standing
   * in, never makes one be made.) */
  private async keepInboxes(had: ReadonlyMap<Space, number>) {
    for (const space of inboxesLost(had, savedInboxes(this.notes))) await this.inboxId(space);
  }

  /** Moves lines (their steps and pictures follow) and loose pictures to the end of a note,
   * without a step of their own in the undo list (they go with the change that moves them). */
  private async shiftInto(noteId: number, lines: Item[], pictures: Picture[]) {
    let sort = this.items.filter((x) => x.note_id === noteId && x.parent_id == null).reduce((m, x) => Math.max(m, x.sort), -1);
    for (const it of lines) {
      const now = this.items.find((x) => x.id === it.id);
      if (!now) continue;
      const under = this.subtreeIds(now.id);
      this.items = this.items.map((x) => (x.id !== now.id && under.has(x.id) ? { ...x, note_id: noteId } : x));
      this.pictures = this.pictures.map((p) => (p.item_id != null && under.has(p.item_id) ? { ...p, note_id: noteId } : p));
      await this.putItem({ ...now, note_id: noteId, sort: (sort += 1) }, null);
    }
    for (const p of pictures) {
      if (!this.pictures.some((x) => x.id === p.id)) continue;
      await this.api.releasePicture(p.id, noteId);
      this.pictures = this.pictures.map((x) => (x.id === p.id ? { ...x, item_id: null, note_id: noteId } : x));
    }
  }

  /** The notes, lines and pictures read again from the notes file (the tour's sample stays). */
  private async refresh() {
    const all = await this.api.loadAll();
    this.notes = [...all.notes, ...this.notes.filter((n) => inMemory(n.id))];
    this.items = [...all.items, ...this.items.filter((i) => inMemory(i.id))];
    this.pictures = all.pictures ?? [];
    this.doneLog = all.done_log ?? [];
    thumbs.fill(this.api, this.pictures);
  }

  private removeLocal(itemIds: number[]) {
    this.items = this.items.filter((x) => !itemIds.includes(x.id));
    this.pictures = this.pictures.filter((p) => p.item_id == null || !itemIds.includes(p.item_id));
  }

  /** A line and every line under it (its steps, their steps…). */
  private subtreeIds(id: number): Set<number> {
    const ids = new Set([id]);
    for (let grew = true; grew; ) {
      grew = false;
      for (const x of this.items)
        if (x.parent_id != null && ids.has(x.parent_id) && !ids.has(x.id)) {
          ids.add(x.id);
          grew = true;
        }
    }
    return ids;
  }

  /** A line and everything that goes with it (the backend deletes and moves all of it). */
  private snapshotItem(it: Item) {
    const under = this.subtreeIds(it.id);
    const items = this.items.filter((x) => under.has(x.id));
    const ids = items.map((x) => x.id);
    const pictures = this.pictures.filter((p) => p.item_id != null && ids.includes(p.item_id));
    return { items: structuredClone($state.snapshot(items)) as Item[], pictures: structuredClone($state.snapshot(pictures)) as Picture[] };
  }

  get custom(): CustomColour[] {
    const c = this.settings.custom_palette as CustomColour[] | undefined;
    return Array.isArray(c) && c.length === 10 ? c : defaultCustom;
  }

  get workHours(): WorkHours {
    return { ...defaultWorkHours, ...((this.settings.work_hours as Partial<WorkHours>) ?? {}) };
  }

  get shortcuts(): Shortcuts {
    const s = (k: keyof Shortcuts) => (typeof this.settings[`shortcut_${k}`] === "string" ? (this.settings[`shortcut_${k}`] as string) : defaultShortcuts[k]);
    return { toggle: s("toggle"), dump: s("dump"), search: s("search") };
  }

  /** Space new things go to: the one showing, else the work hours' one, else Work. */
  get space(): Space {
    if (this.view !== "all") return this.view;
    return this.workHours.on ? this.clockSpace(this.now) : "work";
  }

  visibleNotes = $derived(
    this.notes
      .filter((n) => !n.archived && (this.view === "all" || n.space === this.view))
      .sort((a, b) => a.sort - b.sort || a.id - b.id),
  );

  // (The lists as last given: a list whose lines are all the same is given again as it was.)
  private lastLines: { top: Map<number, Item[]>; steps: Map<number, Item[]> } | null = null;
  private lastPictures: Map<number, Picture[]> | null = null;

  /** Top-level lines of each note, and the steps of each line: both from one sorted pass. */
  private lines = $derived.by(() => {
    const top = new Map<number, Item[]>();
    const steps = new Map<number, Item[]>();
    for (const it of [...this.items].sort((a, b) => a.sort - b.sort || a.id - b.id)) {
      const map = it.parent_id == null ? top : steps;
      const key = it.parent_id ?? it.note_id;
      const list = map.get(key);
      if (list) list.push(it);
      else map.set(key, [it]);
    }
    const last = this.lastLines;
    return (this.lastLines = { top: keepSame(last?.top ?? null, top), steps: keepSame(last?.steps ?? null, steps) });
  });

  /** Top-level lines of each note (steps are under their task). */
  itemsByNote = $derived(this.lines.top);

  stepsOf = $derived(this.lines.steps);

  picturesByNote = $derived.by(() => {
    const map = new Map<number, Picture[]>();
    for (const p of [...this.pictures].sort((a, b) => a.sort - b.sort || a.id - b.id)) {
      if (p.item_id != null) continue;
      const list = map.get(p.note_id);
      if (list) list.push(p);
      else map.set(p.note_id, [p]);
    }
    return (this.lastPictures = keepSame(this.lastPictures, map));
  });

  private lastItemPictures: Map<number, Picture[]> | null = null;
  picturesByItem = $derived.by(() => {
    const map = new Map<number, Picture[]>();
    for (const p of this.pictures) {
      if (p.item_id == null) continue;
      const list = map.get(p.item_id);
      if (list) list.push(p);
      else map.set(p.item_id, [p]);
    }
    return (this.lastItemPictures = keepSame(this.lastItemPictures, map));
  });

  /** Open timed tasks in view, soonest first (overdue first). */
  timed = $derived.by(() => {
    const visible = new Set(this.visibleNotes.map((n) => n.id));
    return this.items
      .filter((i) => i.kind === "task" && i.due_at != null && i.done_at == null && visible.has(i.note_id))
      .sort((a, b) => (a.due_at ?? 0) - (b.due_at ?? 0));
  });

  /** What is ringing now (the Now card), in every space: reminders always ring, except in
   * archived notes (put away, silent until they come back). */
  ringing = $derived.by(() => {
    const archived = new Set(this.notes.filter((n) => n.archived).map((n) => n.id));
    return this.items
      .filter((i) => i.done_at == null && i.ring_state in RING_ORDER && !archived.has(i.note_id))
      .sort((a, b) => RING_ORDER[a.ring_state] - RING_ORDER[b.ring_state] || (a.due_at ?? 0) - (b.due_at ?? 0));
  });

  /** Each space's one Inbox: its first note called "Inbox" (the one the backend files into). A
   * second one (an Inbox archived before 1.0 and put back, a note given that name) is a plain
   * note on the board, which can be renamed, archived or deleted like any other. */
  inboxIds = $derived(new Set(firstInboxes(this.notes).values()));
  /** The Inbox of a space, when there is one. */
  inboxOf(space: Space): Note | undefined {
    return this.notes.find((n) => n.space === space && this.inboxIds.has(n.id));
  }

  // ---- The Inbox: a strip across the top of the board, not a note card ----
  // Each space has its own Inbox (a note called "Inbox" underneath, as it always was). The board
  // shows the other notes; the strip holds the Inbox's lines (both spaces' when All shows).

  /** The notes on the board: those showing, without the Inbox. */
  boardNotes = $derived(this.visibleNotes.filter((n) => !this.inboxIds.has(n.id)));
  /** The Inbox notes the strip shows (Work's first): one, or both spaces' when All shows. */
  stripNotes = $derived(
    this.visibleNotes.filter((n) => this.inboxIds.has(n.id)).sort((a, b) => Number(a.space === "personal") - Number(b.space === "personal") || a.id - b.id),
  );
  /** Opened by a click (until folded again by a click or Esc). */
  inboxOpen = $state(false);
  /** Held open by the quick tour while it shows the Inbox (not saved, not the user's choice). */
  inboxTour = $state(false);
  /** The pin: the strip stays open, also the next time (remembered). */
  get inboxPinned(): boolean {
    return this.settings.inbox_pinned === true;
  }
  get inboxShown(): boolean {
    return this.inboxPinned || this.inboxOpen || this.inboxTour;
  }
  /** A click on the strip (or its chevron, Enter, F): opens it, or folds it (a pinned strip
   * folded by hand lets go of its pin). */
  toggleInbox() {
    if (this.inboxOpen || this.inboxPinned) {
      this.inboxOpen = false;
      if (this.inboxPinned) void this.setSetting("inbox_pinned", false);
    } else this.inboxOpen = true;
  }
  /** The pin; unpinned, the strip stays open until it is folded. */
  setInboxPinned(on: boolean) {
    if (!on) this.inboxOpen = true;
    void this.setSetting("inbox_pinned", on);
  }

  /** The archive, newest first (notes archived before the time was kept come last). */
  archived = $derived(
    this.notes.filter((n) => n.archived && !inMemory(n.id)).sort((a, b) => (b.archived_at ?? 0) - (a.archived_at ?? 0) || b.sort - a.sort || b.id - a.id),
  );

  /** Each note's space (a look-up instead of a search through the notes for every line). */
  private spaceOfNote = $derived(new Map(this.notes.map((n) => [n.id, n.space])));
  /** The notes that put their done tasks at the end (a look-up for every line and step, never a
   * search through the notes). */
  doneLastIds = $derived(new Set(this.notes.filter((n) => n.done_last).map((n) => n.id)));

  /** Open lines waiting in the Inbox (of the space showing; both when All). The tour's sample
   * line counts, as if just dumped, also while "Or make it a step" shows it as one; the bigger
   * task it is dropped onto there doesn't (the badge stays still under the tour's veil). */
  inboxWaiting = $derived(
    this.items
      .filter((i) => (i.id === SAMPLE_ITEM || (i.parent_id == null && !inMemory(i.id))) && i.done_at == null && this.inboxIds.has(i.note_id))
      .filter((i) => this.view === "all" || this.spaceOfNote.get(i.note_id) === this.view)
      .sort((a, b) => a.sort - b.sort || a.id - b.id),
  );

  /** When the board last showed what is saved after a change that couldn't be (one at a time). */
  private resynced = 0;

  /** A change couldn't be saved: said, and the board shows the notes as they really are saved
   * (never a change on screen that would be gone at the next start). */
  private saveFailed(e: unknown) {
    const now = Date.now();
    if (now - this.resynced < 2000) return;
    this.resynced = now;
    this.showToast(() => tr("That change couldn’t be saved: {error}", { error: String(e) }), undefined, 10_000);
    void this.reload().catch(() => {});
  }

  async init() {
    try {
      // (The language was asked before anything was drawn: main.ts.)
      this.api = await backendPromise;
      onSaveFailed((e) => this.saveFailed(e));
      await this.reload();
      const v = this.settings.view;
      if (v === "work" || v === "personal" || v === "all") this.view = v;
      this.win = await this.api.windowInfo();
      this.applyClock(true);
      (await import("./look.svelte")).look.init();
      this.loaded = true;
      this.sayMovedShortcuts();
    } catch (e) {
      this.error = String(e);
    }
    this.api.onEvent("item-updated", (p) => this.upsertItem(p as Item));
    // A repeating task's history, as the notes file has it now.
    this.api.onEvent("done-log", (p) => {
      const { item_id, rows } = p as { item_id: number; rows: Done[] };
      this.setLog(item_id, rows);
    });
    // A picture pasted in the quick dump bar.
    this.api.onEvent("picture-added", (p) => {
      const pic = p as Picture;
      this.pictures = [...this.pictures.filter((x) => x.id !== pic.id), pic];
      thumbs.soon(this.api, pic);
    });
    this.api.onEvent("ring", (p) => this.onRing(p as RingEvent));
    this.api.onEvent("reload", () => void this.reload());
    // Notes' pictures seen failing are tried again after a restore and when the window shows
    // again (a drive plugged back meanwhile): calmly, each once (thumbs.svelte.ts).
    const retry = () => void thumbs.retry((path) => this.api.fileSrc(path));
    this.api.onEvent("reload", retry);
    document.addEventListener("visibilitychange", () => !document.hidden && retry());
    // The automatic backup didn't work (once, until one works again): never a silent failure.
    this.api.onEvent("backup-failed", (e) => this.showToast(() => tr("Today’s backup didn’t work: {error}. Settings › Backups", { error: String(e) }), undefined, 12_000));
    setInterval(() => {
      this.now = new Date();
      this.applyClock(false);
    }, 20_000);
  }

  async reload() {
    const all = await this.api.loadAll();
    // (What is read again as it was stays the very same object: only what changed is redrawn.)
    this.notes = keepRows(this.notes, all.notes);
    this.items = keepRows(this.items, all.items);
    this.pictures = keepRows(this.pictures, all.pictures ?? []);
    this.doneLog = keepRows(this.doneLog, all.done_log ?? []);
    thumbs.fill(this.api, this.pictures);
    if (!sameValue($state.snapshot(this.settings), all.settings)) this.settings = all.settings;
    i18n.choice = readChoice(all.settings.language);
    document.documentElement.lang = i18n.lang;
    // The Inbox was a card on the board until 1.0: its places in the board's saved columns go.
    const steps = dropNotesFromSteps(this.settings.board_steps as SavedSteps | undefined, [...this.inboxIds]);
    if (steps) await this.setSetting("board_steps", steps);
  }

  get backend() {
    return this.api;
  }

  noteOf(it: Item): Note | undefined {
    return this.notes.find((n) => n.id === it.note_id);
  }

  private replaceNote(n: Note) {
    this.notes = this.notes.map((x) => (x.id === n.id ? n : x));
  }
  /** A line's history, all of it. */
  private setLog(itemId: number, rows: Done[]) {
    this.doneLog = [...this.doneLog.filter((r) => r.item_id !== itemId), ...rows].sort((a, b) => a.done_at - b.done_at || a.id - b.id);
  }

  /** The history a tick or an untick makes, shown at once (the backend's own comes a moment
   * later: `done-log`). Returns the id of the time a tick shows (null: none). */
  private logTick(before: Item | undefined, after: Item): number | null {
    if (!before || after.kind !== "task" || !(after.repeat ?? before.repeat)) return null;
    if (before.done_at == null && after.done_at != null) {
      const id = -(++this.provisional);
      this.doneLog = [...this.doneLog, { id, item_id: after.id, due_at: before.due_at, done_at: after.done_at }];
      return id;
    }
    if (before.done_at != null && after.done_at == null) this.doneLog = this.doneLog.filter((r) => !(r.item_id === after.id && r.done_at === before.done_at));
    return null;
  }
  private provisional = 0;

  /** A line's history (oldest first). */
  logOf(itemId: number): Done[] {
    return this.doneLog.filter((r) => r.item_id === itemId);
  }

  private replaceItem(i: Item) {
    this.items = this.items.map((x) => (x.id === i.id ? i : x));
  }
  upsertItem(i: Item) {
    if (this.items.some((x) => x.id === i.id)) this.replaceItem(i);
    else this.items = [...this.items, i];
  }

  async setView(v: SpaceView) {
    this.view = v;
    this.banner = null;
    await this.setSetting("view", v);
  }

  async setSetting(key: string, value: unknown) {
    // (A plain copy for the backend: a value made from the settings, like the board's saved
    // columns, may still hold their live objects.)
    value = $state.snapshot(value);
    this.settings = { ...this.settings, [key]: value };
    await this.api.setSetting(key, value);
  }

  // ---- Work hours: the clock switches the space showing (never the reminders) ----

  clockSpace(d: Date): Space {
    // (Hours that cross midnight, 22:00 to 06:00, count for the day they start.)
    return inWorkHours(this.workHours, d) ? "work" : "personal";
  }

  /** Switches the space when the clock crosses a work-hours boundary. A switch by hand stays
   * until the next boundary. */
  private applyClock(atStart: boolean) {
    if (!this.workHours.on) return;
    void atStart;
    const s = this.clockSpace(new Date());
    if (s !== this.settings.clock_space_last) {
      void this.setSetting("clock_space_last", s);
      void this.setView(s);
    }
  }

  async setWorkHours(w: WorkHours) {
    await this.setSetting("work_hours", w);
    if (w.on) {
      const s = this.clockSpace(new Date());
      await this.setSetting("clock_space_last", s);
      await this.setView(s);
    }
  }

  // ---- Reminders ----

  private onRing(e: RingEvent) {
    this.upsertItem(e.item);
    if (this.view !== "all" && this.view !== e.space) {
      const back = this.view;
      this.view = e.space;
      this.banner = { space: e.space, back };
    }
    // A ringing task unfolds its note.
    const n = this.notes.find((x) => x.id === e.item.note_id);
    if (n?.folded) void this.putNote({ ...n, folded: false }, null);
  }

  async answer(it: Item, action: RingAction) {
    const next = await this.api.answerRing(it.id, action);
    this.upsertItem(next);
    if (this.laterFor === it.id) this.laterFor = null;
  }

  async setAlarm(it: Item, alarm: Alarm, rhythm: RhythmChoice | null = it.rhythm) {
    await this.putItem({ ...it, alarm, rhythm: alarm === "deadline" ? (rhythm ?? null) : null }, alarm === "deadline" ? "Deadline set" : "Reminder set");
  }

  async setDue(it: Item, due: number | null) {
    const next: Item = { ...it, due_at: due, kind: due != null ? "task" : it.kind };
    // A repeat keeps its pattern but follows the new time.
    await this.putItem(next, due == null ? "Time removed" : "Time set");
  }

  /** A repeat needs a time: without one, the first occurrence is given one (09:00). */
  /** `due`: the time it is given with it (null: none any more; left out: the time it has). */
  async setRepeat(it: Item, repeat: Repeat | null, due?: number | null) {
    await this.putItem({ ...it, repeat, kind: "task", due_at: due === undefined ? it.due_at : due }, repeat ? "Repeat set" : "Repeat removed");
  }

  // ---- Lines ----

  /** The colour a new note starts in: the one chosen in Style, else the next one in turn. */
  startColour(): { palette: PaletteName; color: string } {
    const v = this.settings.new_note_colour as { palette?: string; color?: string } | null | undefined;
    if (v && typeof v.color === "string" && (v.palette === "soft" || v.palette === "vivid" || v.palette === "custom")) return { palette: v.palette, color: v.color };
    return { palette: "soft", color: this.nextColour() };
  }

  nextColour(): string {
    const used = this.notes.filter((n) => n.palette === "soft").map((n) => n.color);
    return rotation.find((c) => !used.includes(c)) ?? rotation[this.notes.length % rotation.length];
  }

  async inboxId(space: Space): Promise<number> {
    const id = await this.api.inboxFor(space);
    if (!this.notes.some((n) => n.id === id)) {
      const all = await this.api.loadAll();
      this.notes = all.notes;
    }
    return id;
  }

  /** A line typed in the capture box. Returns the item made, or null. */
  async capture(raw: string, kind: Kind): Promise<Item | null> {
    const p = parseCapture(raw, kind);
    if (!p.text) return null;
    let noteId: number | null = null;
    if (p.tag) {
      const found = findNoteForTag(p.tag, this.notes, this.space);
      if (found) noteId = found.id;
      else {
        const c = this.startColour();
        const n = await this.api.createNote(titleFromTag(p.tag), this.space, c.palette, c.color);
        this.notes = [...this.notes, n];
        noteId = n.id;
      }
    } else if (this.target != null && !inMemory(this.target) && this.notes.some((n) => n.id === this.target && !n.archived)) {
      noteId = this.target;
    } else {
      noteId = await this.inboxId(this.space);
    }
    return this.createLine(noteId, p, kind);
  }

  /** A line added in a note itself (its "+ Add"): a time, a repeat and a "!" deadline are read
   * as in the entry line; a #word stays in the text (it is already where it goes). */
  async addLine(noteId: number, raw: string, kind: Kind): Promise<Item | null> {
    if (inMemory(noteId)) return null;
    const p = parseCapture(raw, kind, new Date(), { tags: false });
    if (!p.text) return null;
    return this.createLine(noteId, p, kind);
  }

  private async createLine(noteId: number, p: Parsed, kind: Kind): Promise<Item> {
    let it = await this.api.createItem({
      noteId,
      kind,
      text: p.text,
      dueAt: p.due ? p.due.getTime() : null,
      alarm: p.deadline && p.due ? "deadline" : "reminder",
    });
    if (p.repeat && kind === "task") it = await this.api.updateItem({ ...it, repeat: p.repeat });
    this.items = [...this.items, it];
    this.recordAdded(it, kind === "task" ? "Task added" : "Idea added");
    return it;
  }

  /** Adding can be undone (the line goes) and redone (it comes back as it was). */
  private recordAdded(it: Item, label: Key) {
    this.history.record({
      label,
      undo: async () => {
        const snap = this.snapshotItem(this.items.find((x) => x.id === it.id) ?? it);
        this.removeLocal(snap.items.map((x) => x.id));
        await this.api.deleteItem(it.id);
        added = snap;
      },
      redo: async () => {
        if (added) await this.restoreAll([], added.items, added.pictures);
      },
    });
    let added: { items: Item[]; pictures: Picture[] } | null = null;
  }

  async addStep(parent: Item, text: string): Promise<Item | null> {
    const t = text.trim();
    if (!t || inMemory(parent.id)) return null;
    const p = parseCapture(t, "task");
    const it = await this.api.createItem({ noteId: parent.note_id, kind: "task", text: p.text || t, dueAt: p.due ? p.due.getTime() : null, parentId: parent.id, alarm: p.deadline && p.due ? "deadline" : "reminder" });
    this.items = [...this.items, it];
    this.recordAdded(it, "Step added");
    return it;
  }

  /** Ticks a task, or unticks it. A repeating task stays one line: ticked, it goes into its
   * history and waits, ticked, for its next occurrence's day (the backend opens it again then,
   * its steps fresh); unticked while it waits, it is back as it was. */
  async toggleDone(it: Item) {
    if (inMemory(it.id)) return;
    const prev = $state.snapshot(this.items.find((x) => x.id === it.id) ?? it) as Item;
    // (Its steps as they were: opening again makes them fresh, and Undo gives them back.)
    const steps = prev.repeat && prev.done_at == null ? this.snapshotItem(prev).items.filter((x) => x.id !== prev.id) : [];
    const next = { ...prev, done_at: prev.done_at == null ? Date.now() : null };
    const saved = await this.putItem(next, null);
    // (A ticked copy kept from before it opened again: the backend keeps the open line as it is,
    // `not_stale`. Nothing changed, nothing to undo: an Undo would tick it again.)
    if (staleUntick(prev, saved)) return;
    // (The same for a tick of an open copy kept from before a tick it never saw, a
    // notification's Done: the line stays ticked as it was.)
    if (next.done_at != null && saved.done_at != null && saved.done_at !== next.done_at) return;
    // (And an untick it never took: a second click on its box before the tick came back, from a
    // copy that never saw it move on. It stays ticked, with nothing to undo.)
    if (next.done_at == null && saved.done_at != null) return;
    // (Yesterday's ticked this morning: today's is already here, open at once. Said, so the tick
    // that went never looks lost.)
    if (next.done_at != null && saved.done_at == null && saved.due_at != null && saved.repeat) {
      const due = saved.due_at;
      this.showToast(() => tr("Done · {next}", { next: nextOne(due, new Date()) }), { label: () => tr("Undo"), run: () => void this.undo() });
    }
    this.history.record({
      label: next.done_at != null ? "Done" : "Not done",
      undo: () => (next.done_at != null && prev.repeat ? this.untickRepeat(prev, next.done_at, saved, steps) : this.revertItem(prev, ["done_at"])),
      redo: () => this.revertItem(next, ["done_at"]),
    });
  }

  /** Undo of a repeating task's "Done" (`doneAt`, from `prev`; `steps`, its steps then): back
   * to the occurrence ticked, that time out of its history. While it still waits for its next
   * day, unticking it does all that (the backend gives back what the tick moved). Ticked again
   * since at another time (its next one, from a notification), only that time leaves its
   * history: the line stays as it is, never two times undone at once. Once open again (its next
   * day came, or at once) with only that tick behind it (`ticked`: the line as that tick left
   * it), its time and its rule go back as they were, its steps too, and that time leaves its
   * history, all in one go (`undoTick`: the reminder clock never rings it in between); moved on
   * further since (ticked again and open again), only that time leaves it. */
  private async untickRepeat(prev: Item, doneAt: number, ticked: Item, steps: Item[] = []) {
    const now = this.items.find((x) => x.id === prev.id);
    if (!now || now.done_at === doneAt) return this.revertItem(prev, ["done_at"]);
    this.doneLog = this.doneLog.filter((r) => !(r.item_id === prev.id && r.done_at === doneAt));
    // (Counted by the rule's count, as that tick left it: a tick a few days late moves it on past
    // the ones missed in one go.)
    if (now.done_at == null && (now.repeat?.done ?? 0) === (ticked.repeat?.done ?? 0)) {
      return this.undoTick(withFields($state.snapshot(now) as Item, prev, ["done_at", "due_at", "repeat"]), doneAt, steps, prev.due_at);
    }
    this.setLog(prev.id, await this.api.forgetDone(prev.id, doneAt));
  }

  /** The line put back as it was before what opened it (`want`), in one go with its steps and,
   * with `forget`, that tick's time out of its history (the backend's `undo_tick`). The steps
   * given back are those its opening made fresh (unticked, moved on by as many days as it moved,
   * from the occurrence `from` to the one it is open at: the backend's `fresh_steps`), back
   * exactly as they were, their reminders too, and the times they were done out of the history
   * again (the opening kept them there); one changed since (ticked, its time changed, moved
   * under another line) stays as it is. */
  private async undoTick(want: Item, forget: number | null, steps: Item[], from: number | null) {
    const now = this.items.find((x) => x.id === want.id);
    const days = from != null && now?.due_at != null ? daysApart(from, now.due_at) : null;
    const back = steps.filter((was) => {
      const s = this.items.find((x) => x.id === was.id);
      if (!s || s.kind !== "task" || s.repeat || s.parent_id !== was.parent_id) return false;
      return s.done_at == null && (was.due_at == null ? s.due_at == null : days != null && s.due_at === shiftDays(was.due_at, days));
    });
    const r = await this.api.undoTick(want, forget, back);
    this.replaceItem(r.item);
    for (const s of r.steps) this.replaceItem(s);
    const given = new Set(back.map((s) => s.id));
    const gone = new Set(r.steps.flatMap((s) => (given.has(s.id) && s.done_at != null ? [`${s.id}:${s.done_at}`] : [])));
    this.doneLog = this.doneLog.filter((x) => !gone.has(`${x.item_id}:${x.done_at}`));
    this.setLog(want.id, r.log);
  }

  async editItem(it: Item, text: string, due?: number | null) {
    const t = text.trim();
    if (!t) return this.deleteItem(it);
    const next: Item = { ...it, text: t, ...(due !== undefined ? { due_at: due, kind: "task" as const } : {}) };
    if (next.text === it.text && next.due_at === it.due_at) return;
    await this.putItem(next, due !== undefined && due !== it.due_at ? "Edited with a time" : "Edited");
  }

  async flipKind(it: Item) {
    const next: Item = { ...it, kind: it.kind === "task" ? "idea" : "task", due_at: it.kind === "task" ? null : it.due_at, repeat: it.kind === "task" ? null : it.repeat };
    await this.putItem(next, next.kind === "idea" ? "Made an idea" : "Made a task");
  }

  async deleteItem(it: Item) {
    if (inMemory(it.id)) return;
    const snap = this.snapshotItem(it);
    this.removeLocal(snap.items.map((x) => x.id));
    if (this.focus?.item === it.id) this.focus = { note: it.note_id, item: null };
    await this.api.deleteItem(it.id);
    const label: Key = it.kind === "idea" ? "Idea deleted" : "Deleted";
    this.history.record({
      label,
      undo: () => this.restoreAll([], snap.items, snap.pictures),
      redo: async () => {
        this.removeLocal(snap.items.map((x) => x.id));
        await this.api.deleteItem(it.id);
      },
    });
    this.showToast(() => tr("{what}: “{text}”", { what: tr(label), text: it.text.length > 40 ? it.text.slice(0, 38) + "…" : it.text }), { label: () => tr("Undo"), run: () => void this.undo() });
  }

  /** Ctrl+Up / Ctrl+Down: swaps a line with its neighbour as shown (same note, same level, same
   * kind; done tasks among their own only when the note puts them at the end). */
  async moveLine(it: Item, dir: -1 | 1) {
    if (inMemory(it.id)) return;
    const apart = this.doneLastIds.has(it.note_id);
    const sibs = this.items
      .filter((x) => x.note_id === it.note_id && (x.parent_id ?? null) === (it.parent_id ?? null) && x.kind === it.kind && (!apart || (x.done_at == null) === (it.done_at == null)))
      .sort((a, b) => a.sort - b.sort || a.id - b.id);
    const i = sibs.findIndex((x) => x.id === it.id);
    const other = sibs[i + dir];
    if (!other || inMemory(other.id)) return;
    const a = { ...it, sort: other.sort === it.sort ? other.sort + dir * 0.5 : other.sort };
    const b = { ...other, sort: it.sort };
    const prevA = it;
    const prevB = other;
    await Promise.all([this.putItem(a, null), this.putItem(b, null)]);
    this.history.record({
      label: "Line moved",
      undo: async () => void (await Promise.all([this.revertItem(prevA, ["sort"]), this.revertItem(prevB, ["sort"])])),
      redo: async () => void (await Promise.all([this.revertItem(a, ["sort"]), this.revertItem(b, ["sort"])])),
    });
  }

  /** Moves a line: into a note (top level) or under another line (a step). Its steps and own
   * pictures follow it. */
  async moveItem(it: Item, to: { noteId: number; parentId?: number | null; kind?: Kind; dueAt?: number | null; alarm?: Alarm; text?: string }) {
    if (to.parentId === it.id || inMemory(it.id) || inMemory(to.noteId) || inMemory(to.parentId)) return;
    const siblings = this.items.filter((x) => x.note_id === to.noteId && (x.parent_id ?? null) === (to.parentId ?? null) && x.id !== it.id);
    const sort = siblings.reduce((m, x) => Math.max(m, x.sort), -1) + 1;
    const next: Item = {
      ...it,
      note_id: to.noteId,
      parent_id: to.parentId ?? null,
      kind: to.kind ?? (to.parentId != null ? "task" : it.kind),
      due_at: to.dueAt !== undefined ? to.dueAt : it.due_at,
      alarm: to.alarm ?? it.alarm,
      text: to.text?.trim() || it.text,
      sort,
    };
    if (next.kind === "idea") next.due_at = null;
    const follow = (noteId: number) => this.carryUnder(it.id, noteId);
    // A picture dumped with the line becomes the note's own once it is put away (part of the same
    // step: an undo gives the line its pictures back, a redo lets them go again).
    const freed = this.inboxIds.has(to.noteId) ? [] : (structuredClone($state.snapshot(this.pictures.filter((p) => p.item_id === it.id))) as Picture[]);
    const release = async () => {
      for (const p of freed) {
        if (!this.pictures.some((x) => x.id === p.id)) continue;
        await this.api.releasePicture(p.id, to.noteId);
        this.pictures = this.pictures.map((x) => (x.id === p.id ? { ...x, item_id: null, note_id: to.noteId } : x));
      }
    };
    const takeBack = async () => {
      const here = freed.filter((p) => this.pictures.some((x) => x.id === p.id));
      if (!here.length) return;
      await this.api.restore([], [], here);
      const byId = new Map(here.map((p) => [p.id, p]));
      this.pictures = this.pictures.map((x) => {
        const p = byId.get(x.id);
        return p ? { ...x, item_id: p.item_id, note_id: p.note_id } : x;
      });
    };
    follow(to.noteId);
    const before = this.items.find((x) => x.id === it.id) ?? it;
    await this.putItem(next, null);
    const back = { ...before, sort: it.sort };
    const keys = changedKeys(back, next);
    this.history.record({
      label: "Moved",
      undo: async () => {
        await takeBack();
        follow(back.note_id);
        await this.revertItem(back, keys);
      },
      redo: async () => {
        follow(next.note_id);
        await this.revertItem(next, keys);
        await release();
      },
    });
    await release();
  }

  /** Everything under a line (its steps, their steps, their pictures) follows it into a note,
   * here as the backend does it when the line is saved there. */
  private carryUnder(id: number, noteId: number) {
    const under = this.subtreeIds(id);
    this.items = this.items.map((x) => (x.id !== id && under.has(x.id) ? { ...x, note_id: noteId } : x));
    this.pictures = this.pictures.map((p) => (p.item_id != null && under.has(p.item_id) ? { ...p, note_id: noteId } : p));
  }

  /** A carried line dropped on "+ New note" (or "Make a new note of it" in its menu): a new note
   * in `space` at the end of the board; or dropped on the board's empty space (`at`): there, as
   * right-click › New note. Titled with the line's words (lineNote.ts), and the line itself moved
   * inside as its first line, whole: its time, steps, link and pictures go with it (the pictures
   * stay its own, under it). Where it came from stays as it is, empty or not (the Inbox stays the
   * Inbox). One step Undo takes back whole: the line lifts off the note as it fades and goes back
   * to its place, the note goes (and its place in the board's saved columns); Redo brings the
   * note back, with the same id, and the line into it. */
  async noteFromLine(it: Item, space: Space, at: BoardPoint | null = null): Promise<Note | null> {
    const found = this.items.find((x) => x.id === it.id);
    if (!found || inMemory(found.id) || inMemory(found.note_id)) return null;
    const prev = $state.snapshot(found) as Item;
    const cut = titleFromLine(prev.text);
    // (Called "Inbox", it would be taken for one.)
    const title = cut && cut !== "Inbox" ? cut : tr("New note");
    const c = this.startColour();
    const n = await this.api.createNote(title, space, c.palette, c.color);
    this.notes = [...this.notes, n];
    // (In the same moment as it comes on the board: never seen at the end first.)
    if (at) void this.board?.place(n.id, at);
    const id = n.id;
    const next = lineInNote(prev, id);
    const keys = changedKeys(prev, next);
    const into = async () => {
      this.carryUnder(prev.id, id);
      await this.revertItem(next, keys);
    };
    await into();
    let kept: { notes: Note[]; items: Item[]; pictures: Picture[] } | null = null;
    let steps: SavedSteps | null = null;
    const back = async () => {
      if (!this.over && this.visibleNotes.some((x) => x.id === id)) this.lineLifts?.(prev.id, LEAVE_MS);
      await this.leaveBoard([id]);
      this.carryUnder(prev.id, prev.note_id);
      await this.revertItem(prev, keys);
      // (Its places in the board's saved columns go with it, and come back with a redo.)
      steps = structuredClone($state.snapshot(this.settings.board_steps ?? null)) as SavedSteps | null;
      const pruned = dropFromSteps(this.settings.board_steps as SavedSteps | undefined, id);
      if (pruned) await this.setSetting("board_steps", pruned);
      // What the note holds besides (nothing, unless something reached it without a step of its
      // own) goes with it, and comes back with a redo.
      kept = structuredClone(
        $state.snapshot({
          notes: this.notes.filter((x) => x.id === id),
          items: this.items.filter((x) => x.note_id === id),
          pictures: this.pictures.filter((x) => x.note_id === id),
        }),
      ) as { notes: Note[]; items: Item[]; pictures: Picture[] };
      this.notes = this.notes.filter((x) => x.id !== id);
      this.items = this.items.filter((x) => x.note_id !== id);
      this.pictures = this.pictures.filter((x) => x.note_id !== id);
      if (this.target === id) this.target = null;
      if (this.focus?.note === id) this.focus = null;
      await this.api.deleteNote(id);
    };
    const again = async () => {
      if (!kept) return;
      if (steps) {
        const put = putBackInSteps(this.settings.board_steps as SavedSteps | undefined, steps, id);
        if (put) await this.setSetting("board_steps", put);
      }
      await this.restoreAll(kept.notes, kept.items, kept.pictures);
      await into();
    };
    this.history.record({ label: "Note made from a line", undo: back, redo: again });
    this.showToast(() => tr("New note “{name}”", { name: noteTitle(n) }), { label: () => tr("Undo"), run: () => void this.undo() });
    return n;
  }

  /** "Keep for later" in the tidy view: to the end of the Inbox. */
  async later(it: Item) {
    if (inMemory(it.id)) return;
    const max = this.items.filter((x) => x.note_id === it.note_id).reduce((m, x) => Math.max(m, x.sort), 0);
    const next = { ...it, sort: max + 1 };
    this.replaceItem(next);
    await this.api.updateItem(next);
  }

  async setLink(it: Item, link: LinkPreview | null) {
    if (inMemory(it.id)) return;
    const next = { ...it, link };
    this.replaceItem(next);
    await this.api.updateItem(next);
  }

  // ---- Notes ----

  async setColour(n: Note, palette: PaletteName, color: string) {
    await this.putNote({ ...n, palette, color }, "Colour changed");
  }

  async setNoteSpace(n: Note, space: Space) {
    // (The Inbox stays in its space; a note called "Inbox" doesn't go and become another space's,
    // off the board for good.)
    if (this.inboxIds.has(n.id)) return;
    if (isInbox(n)) {
      const there = this.inboxOf(space);
      if (!there || n.id < there.id) return;
    }
    await this.putNote({ ...n, space }, space === "work" ? "Moved to Work" : "Moved to Personal");
  }

  async setWide(n: Note, wide: boolean) {
    await this.putNote({ ...n, wide }, wide ? "Made wide" : "Made normal");
  }

  async setFolded(n: Note, folded: boolean) {
    await this.putNote({ ...n, folded }, folded ? "Folded" : "Unfolded");
  }

  /** Its done tasks after the open ones, or where they are (the default). */
  async setDoneLast(n: Note, done_last: boolean) {
    await this.putNote({ ...n, done_last }, done_last ? "Done tasks moved to the end" : "Done tasks kept in place");
  }

  async setPaper(n: Note, paper: Paper) {
    await this.putNote({ ...n, paper }, "Paper changed");
  }

  async rename(n: Note, title: string) {
    const t = title.trim();
    if (!t || t === n.title || this.inboxIds.has(n.id)) return;
    // (Named "Inbox", it would become its space's Inbox, off the board and never renamed back.)
    if (t === "Inbox") {
      const inbox = this.inboxOf(n.space);
      if (!inbox || n.id < inbox.id) return;
    }
    await this.putNote({ ...n, title: t }, "Renamed");
  }

  /** The board plays the cards' way out first (those that show on it). */
  private async leaveBoard(ids: number[]) {
    const shown = ids.filter((id) => this.visibleNotes.some((n) => n.id === id));
    if (shown.length && this.noteLeaves) await this.noteLeaves(shown);
  }

  /** Into the archive: off the board, its reminders quiet, kept as it is (the Archive page). */
  async archive(n: Note) {
    // (The Inbox is never put away.)
    if (inMemory(n.id) || this.inboxIds.has(n.id)) return;
    const found = this.notes.find((x) => x.id === n.id);
    if (!found || found.archived) return;
    const prev = $state.snapshot(found) as Note;
    const next: Note = { ...prev, archived: true };
    if (this.target === n.id) this.target = null;
    if (this.focus?.note === n.id) this.focus = null;
    const put = async () => {
      await this.leaveBoard([n.id]);
      await this.revertNote(next, ["archived"]);
    };
    await put();
    this.history.record({ label: "Archived", undo: () => this.revertNote(prev, ["archived", "archived_at"]), redo: put });
    this.showToast(
      () => tr("Archived “{name}”", { name: noteTitle(n) }),
      { label: () => tr("Undo"), run: () => void this.undo() },
      6500,
      { label: () => tr("See archive"), run: () => this.openArchive("board") },
    );
  }

  /** Out of the archive, back on the board at its place. While the Archive page covers the
   * board, its card settles in once the board shows again. */
  async unarchive(n: Note) {
    if (inMemory(n.id)) return;
    const found = this.notes.find((x) => x.id === n.id);
    if (!found || !found.archived) return;
    const prev = $state.snapshot(found) as Note;
    const next: Note = { ...prev, archived: false, archived_at: null };
    // What it holds as it comes out. An Inbox archived before 1.0 comes back as its space's Inbox
    // (the first note called "Inbox"); what reaches it meanwhile (dumps, lines dropped on the
    // strip) is the Inbox's, not the archive's: taking the restore back leaves those in the
    // space's Inbox (the next note called "Inbox", or a new one), and a redo brings them along.
    const own = {
      items: new Set(this.items.filter((i) => i.note_id === n.id).map((i) => i.id)),
      pictures: new Set(this.pictures.filter((p) => p.note_id === n.id && p.item_id == null).map((p) => p.id)),
    };
    // (A space without an Inbox before this one came out is left without one again when it goes
    // back with nothing for another to hold.)
    const hadInbox = savedInboxes(this.notes).has(n.space);
    let handed: { to: number; items: Item[]; pictures: Picture[]; made: boolean } | null = null;
    const put = async () => {
      if (this.over) this.arriving = [...this.arriving.filter((id) => id !== n.id), n.id];
      await this.revertNote(next, ["archived", "archived_at"]);
      const h = handed;
      handed = null;
      if (!h) return;
      const to = h.to;
      // (As they are now in the notes file: a line dumped from the quick dump bar meanwhile too.)
      await this.refresh();
      // An Inbox made only to stand in for this one: everything that reached it comes along (it
      // was this Inbox's in the meantime). Another note called "Inbox": those it was handed that
      // are still there.
      const lines = this.items.filter((x) => x.note_id === to && x.parent_id == null && !inMemory(x.id) && (h.made || h.items.some((i) => i.id === x.id)));
      const pictures = this.pictures.filter((x) => x.note_id === to && x.item_id == null && (h.made || h.pictures.some((p) => p.id === x.id)));
      await this.shiftInto(n.id, lines, pictures);
      // The stand-in goes again once it is left empty (it would be a stray card).
      if (h.made && !this.inboxIds.has(to) && !this.items.some((i) => i.note_id === to) && !this.pictures.some((p) => p.note_id === to)) {
        this.notes = this.notes.filter((x) => x.id !== to);
        await this.api.deleteNote(to);
      }
    };
    const back = async () => {
      this.arriving = this.arriving.filter((id) => id !== n.id);
      const wasInbox = this.inboxIds.has(n.id);
      const known = new Set(this.notes.map((x) => x.id));
      await this.leaveBoard([n.id]);
      await this.revertNote(prev, ["archived", "archived_at"]);
      if (!wasInbox) return;
      // (The space has its Inbox again by now: `keepInboxes`.)
      const to = await this.inboxId(n.space);
      if (to === n.id) return;
      // What reached it while it was out, up to the moment it went back in (read again from the
      // notes file: a line dumped from the quick dump bar while its card was leaving too).
      await this.refresh();
      const arrived = arrivedSince(this.items, this.pictures, n.id, own);
      const made = !known.has(to);
      if (made && !hadInbox && !arrived.items.length && !arrived.pictures.length && (await this.api.dropEmptyInbox(to))) {
        this.notes = this.notes.filter((x) => x.id !== to);
        return;
      }
      await this.shiftInto(to, arrived.items, arrived.pictures);
      handed = { to, ...arrived, made };
    };
    await put();
    this.history.record({ label: "Restored", undo: back, redo: put });
    this.showToast(() => tr("“{name}” is back on the board", { name: noteTitle(n) }), { label: () => tr("Undo"), run: () => void this.undo() });
  }

  /** Opens Report a problem (App.svelte listens). */
  openReport(from: "board" | "settings") {
    window.dispatchEvent(new CustomEvent("dobo-report", { detail: from }));
  }

  /** Opens What's new: the running version's own part (App.svelte listens). */
  openWhatsNew(from: "board" | "settings") {
    window.dispatchEvent(new CustomEvent("dobo-whatsnew", { detail: from }));
  }

  /** Opens the Archive page (App.svelte listens). */
  openArchive(from: "board" | "settings") {
    window.dispatchEvent(new CustomEvent("dobo-archive", { detail: from }));
  }

  /** Deletes notes with their lines and pictures, as one step Undo takes back whole (until
   * Dobo closes: the pictures' files are kept a day). */
  private async deleteNotes(list: Note[], label: Key): Promise<boolean> {
    // (Never the Inbox.)
    list = list.filter((n) => !inMemory(n.id) && !this.inboxIds.has(n.id));
    if (!list.length) return false;
    const ids = new Set(list.map((n) => n.id));
    // What goes, read from the notes file each time it goes (a redo too): what reached those notes
    // since the first time, without a step of its own here (a repeat's next occurrence made by a
    // reminder's "Done", a line dumped from the quick dump bar), comes back with an undo too.
    const take = async () => {
      const all = await this.api.loadAll();
      return {
        notes: all.notes.filter((x) => ids.has(x.id)),
        items: all.items.filter((x) => ids.has(x.note_id)),
        pictures: (all.pictures ?? []).filter((x) => ids.has(x.note_id)),
        // Their places in the board's saved columns go with them (and come back with an undo).
        steps: structuredClone($state.snapshot(this.settings.board_steps ?? null)) as SavedSteps | null,
      };
    };
    let snap: Awaited<ReturnType<typeof take>> = { notes: [], items: [], pictures: [], steps: null };
    const drop = async () => {
      snap = await take();
      const had = savedInboxes(this.notes);
      await this.leaveBoard([...ids]);
      this.notes = this.notes.filter((x) => !ids.has(x.id));
      this.items = this.items.filter((x) => !ids.has(x.note_id));
      this.pictures = this.pictures.filter((x) => !ids.has(x.note_id));
      if (this.target != null && ids.has(this.target)) this.target = null;
      if (this.focus && ids.has(this.focus.note)) this.focus = null;
      let pruned: SavedSteps | null = null;
      for (const id of ids) pruned = dropFromSteps(pruned ?? (this.settings.board_steps as SavedSteps | undefined), id) ?? pruned;
      if (pruned) await this.setSetting("board_steps", pruned);
      for (const id of ids) await this.api.deleteNote(id);
      await this.keepInboxes(had);
    };
    const back = async () => {
      const steps = snap.steps;
      if (steps) {
        let again: SavedSteps | null = null;
        for (const id of ids) again = putBackInSteps(again ?? (this.settings.board_steps as SavedSteps | undefined), steps, id) ?? again;
        if (again) await this.setSetting("board_steps", again);
      }
      await this.restoreAll(snap.notes, snap.items, snap.pictures);
    };
    try {
      await drop();
    } finally {
      // (Recorded even when a delete failed partway, the disk full say: those already gone
      // can still be put back. Undo puts back what was read, whatever is still there.)
      this.history.record({ label, undo: back, redo: drop });
    }
    return true;
  }

  async deleteNote(n: Note) {
    if (!(await this.deleteNotes([n], "Note deleted"))) return;
    this.showToast(() => tr("Deleted “{name}”", { name: noteTitle(n) }), { label: () => tr("Undo"), run: () => void this.undo() });
  }

  /** Every archived note, deleted as one step (Undo brings them all back). */
  async emptyArchive() {
    const list = [...this.archived];
    if (!(await this.deleteNotes(list, "Archive emptied"))) return;
    this.showToast(() => tr("Deleted {n} archived notes", { n: list.length }), { label: () => tr("Undo"), run: () => void this.undo() }, 8000);
  }

  // ---- Templates ----

  async saveTemplate(n: Note) {
    if (inMemory(n.id)) return;
    const lines: TemplateLine[] = (this.itemsByNote.get(n.id) ?? [])
      // (A repeating task ticked and waiting for its next day is still one of its tasks.)
      .filter((i) => (i.done_at == null || i.repeat?.ticked) && !inMemory(i.id))
      .map((i) => ({
        kind: i.kind,
        text: i.text,
        deadline: i.alarm === "deadline" || undefined,
        steps: (this.stepsOf.get(i.id) ?? []).map((s) => s.text),
      }));
    const t: Template = { id: Date.now(), name: n.title, palette: n.palette, color: n.color, paper: n.paper ?? "plain", ...(n.done_last ? { done_last: true } : {}), lines };
    // (One of the same name is replaced: Undo brings it back.)
    const said = () => tr("Saved “{name}” as a template", { name: noteTitle(n) });
    await this.putTemplates([...this.templates.filter((x) => x.name !== t.name), t], said);
    this.showToast(said, { label: () => tr("Undo"), run: () => void this.undo() });
  }

  async deleteTemplate(id: number) {
    const gone = this.templates.find((t) => t.id === id);
    if (!gone) return;
    const said = () => tr("Template “{name}” deleted", { name: gone.name });
    await this.putTemplates(
      this.templates.filter((t) => t.id !== id),
      said,
    );
    this.showToast(said, { label: () => tr("Undo"), run: () => void this.undo() });
  }

  /** The templates changed as one step Undo takes back (a template deleted or replaced). */
  private async putTemplates(next: Template[], label: () => string) {
    const prev = structuredClone($state.snapshot(this.templates)) as Template[];
    next = structuredClone($state.snapshot(next)) as Template[];
    await this.setSetting("templates", next);
    this.history.record({ label, undo: () => this.setSetting("templates", prev), redo: () => this.setSetting("templates", next) });
  }

  /** A new note, empty or from a template. From the board's right-click menu (`at`): it lands
   * where it was clicked, in the column clicked under the note above (Board.svelte), in the
   * space of that spot; `fresh`, its title waits to be typed (NoteCard: Esc with nothing typed
   * takes it away again). A template's blank lines (the built-ins') wait in it to be filled in,
   * never saved as lines. */
  async fromTemplate(t: Template | null, title: string, opts: { at?: BoardPoint | null; fresh?: boolean } = {}): Promise<Note> {
    const name = title.trim() || t?.name || tr("New note");
    const c = this.startColour();
    const at = opts.at ?? null;
    const space = at ? this.spaceAt(at) : this.space;
    // The board as it was (its saved columns, the notes' order): a new note taken away again at
    // once (Esc) leaves it so, as if never made (dropNew).
    const was = at ? { steps: structuredClone($state.snapshot(this.settings.board_steps ?? null)) as SavedSteps | null, order: [...this.notes].sort((a, b) => a.sort - b.sort).map((x) => x.id) } : null;
    let n = await this.api.createNote(name, space, t?.palette ?? c.palette, t?.color ?? c.color);
    if (t && (t.paper !== "plain" || t.done_last)) n = await this.api.updateNote({ ...n, paper: t.paper, done_last: !!t.done_last });
    this.notes = [...this.notes, n];
    // (In the same moment as it comes on the board: it is never seen at the end first.)
    // (Only a note whose title waits is remembered so: only it can be taken away as if never made.)
    if (at && was) {
      const saving = this.board?.place(n.id, at) ?? undefined;
      if (opts.fresh) this.placed.set(n.id, { ...was, saving });
    }
    // (Its blank lines with it: never "Empty" for a moment.)
    const blanks = blanksOf(t);
    if (blanks.task || blanks.idea) this.blanks = { ...this.blanks, [n.id]: blankIds(blanks) };
    for (const line of t?.lines ?? []) {
      if (isBlank(line)) continue;
      const it = await this.api.createItem({ noteId: n.id, kind: line.kind, text: line.text, alarm: line.deadline ? "deadline" : "reminder" });
      this.items = [...this.items, it];
      for (const s of line.steps ?? []) {
        const step = await this.api.createItem({ noteId: n.id, kind: "task", text: s, parentId: it.id });
        this.items = [...this.items, step];
      }
    }
    // Undo takes it away with what it holds then; redo puts that back as it was, with the same
    // ids (a new note made again would leave the next undo nothing to take back: ids are never
    // given twice).
    const id = n.id;
    let kept: { notes: Note[]; items: Item[]; pictures: Picture[]; steps: SavedSteps | null } | null = null;
    const step = {
      label: "Note added" as Key,
      undo: async () => {
        if (this.fresh === id) this.fresh = null;
        this.placed.delete(id);
        this.added.delete(id);
        await this.leaveBoard([id]);
        kept = structuredClone(
          $state.snapshot({
            notes: this.notes.filter((x) => x.id === id),
            items: this.items.filter((x) => x.note_id === id),
            pictures: this.pictures.filter((x) => x.note_id === id),
            steps: (this.settings.board_steps ?? null) as SavedSteps | null,
          }),
        ) as { notes: Note[]; items: Item[]; pictures: Picture[]; steps: SavedSteps | null };
        // (Its places in the board's saved columns go with it, and come back with a redo.)
        const pruned = dropFromSteps(this.settings.board_steps as SavedSteps | undefined, id);
        if (pruned) await this.setSetting("board_steps", pruned);
        this.notes = this.notes.filter((x) => x.id !== id);
        this.items = this.items.filter((x) => x.note_id !== id);
        this.pictures = this.pictures.filter((x) => x.note_id !== id);
        if (this.target === id) this.target = null;
        if (this.focus?.note === id) this.focus = null;
        await this.api.deleteNote(id);
      },
      redo: async () => {
        if (!kept) return;
        if (kept.steps) {
          const again = putBackInSteps(this.settings.board_steps as SavedSteps | undefined, kept.steps, id);
          if (again) await this.setSetting("board_steps", again);
        }
        await this.restoreAll(kept.notes, kept.items, kept.pictures);
        this.focusNote(id);
      },
    };
    this.history.record(step);
    // (Only a note whose title waits can be taken away as if never made: only it is remembered.)
    if (opts.fresh) {
      this.added.set(id, step);
      this.fresh = id;
    }
    // (Made where it was clicked, it is in sight already: no glow, no scrolling to it.)
    if (!at) this.focusNote(n.id);
    return n;
  }

  /** The New note window (`at`: from the board's right-click menu, where its note goes). */
  openTemplates(at: BoardPoint | null = null) {
    this.templateAt = at;
    this.templatesOpen = true;
  }

  /** A new note's own step in the undo list (to forget it when the note is taken away at once). */
  private added = new Map<number, HistoryStep>();
  /** A new note put at a spot of the board: the board as it was before (its saved columns, the
   * notes' order), and the board's saving of its new place. */
  private placed = new Map<number, { steps: SavedSteps | null; order: number[]; saving?: Promise<void> | void }>();

  /** The space for a new note at a spot of the board: the board's; in "All", that of the notes
   * around it, else Work. */
  spaceAt(at: BoardPoint): Space {
    if (this.view !== "all") return this.view;
    const near = this.board?.around(at) ?? { above: null, below: null };
    const of = (id: number | null) => (id == null ? undefined : this.notes.find((n) => n.id === id)?.space);
    return spaceForSpot(this.view, of(near.above), of(near.below));
  }

  /** The new note's title, typed as it waits (part of making it: one Undo takes the note away,
   * named; no step of its own). */
  async nameNew(n: Note, title: string) {
    if (this.fresh === n.id) this.fresh = null;
    this.placed.delete(n.id);
    this.added.delete(n.id);
    const found = this.notes.find((x) => x.id === n.id);
    const t = title.trim();
    if (!found || !t || t === found.title || t === "Inbox") return;
    await this.putNote({ ...($state.snapshot(found) as Note), title: t }, null);
  }

  /** The new note left as it came (no title typed, nothing in it): Esc, or a click away, takes
   * it away again (it fades off), as if it never was: nothing to undo. */
  async dropNew(id: number): Promise<boolean> {
    if (this.fresh === id) this.fresh = null;
    const step = this.added.get(id);
    const n = this.notes.find((x) => x.id === id);
    if (!step || !n || this.items.some((i) => i.note_id === id) || this.pictures.some((p) => p.note_id === id)) return false;
    if (!this.history.forget(step)) return false;
    this.added.delete(id);
    // The board as it was: its saved columns (every column count), the notes' order. (Read
    // before the undo, which forgets it.)
    const was = this.placed.get(id);
    this.placed.delete(id);
    await step.undo();
    if (was) {
      await was.saving;
      await this.setSetting("board_steps", was.steps ?? {});
      const ids = was.order.filter((x) => this.notes.some((n) => n.id === x));
      await this.reorder([...ids, ...this.notes.filter((n) => !ids.includes(n.id)).sort((a, b) => a.sort - b.sort).map((n) => n.id)]);
    }
    return true;
  }

  /** A blank line filled in: it becomes a real line of the note (at the end of its kind), and
   * that blank goes. */
  async fillBlank(noteId: number, kind: Kind, blank: number, raw: string): Promise<Item | null> {
    const made = await this.addLine(noteId, raw, kind);
    if (made) this.dropBlank(noteId, kind, blank);
    return made;
  }

  /** One blank line taken from the note (filled in, or taken away with Backspace). */
  dropBlank(noteId: number, kind: Kind, blank: number) {
    this.blanks = withoutBlank(this.blanks, noteId, kind, blank);
  }

  // ---- The evening look back ----

  /** Opens the look back: on `day` (its notification's), else the evening's or today's. */
  openLookback(day: number | null = null) {
    this.lookbackFor = day;
    this.lookbackOpen = true;
  }

  /** Puts the look back away without saying it was seen (Esc, another view opening). */
  hideLookback() {
    this.lookbackOpen = false;
    this.lookbackFor = null;
  }

  /** Closes the look back. The day it showed is said done: the evening's own card for that day,
   * or the day of the notification it was opened from (an older one's never takes away today's
   * evening card). One opened by hand earlier in the day just closes (the evening's still
   * comes). */
  async dismissLookback() {
    const d = this.lookbackFor != null ? new Date(this.lookbackFor) : lookbackDay(this.now.getTime(), this.day, this.workHours);
    this.hideLookback();
    // Never an older day over a later one ("YYYY-MM-DD" sorts as days do): an older look back
    // closed after tonight's would bring tonight's back.
    const stored = this.settings.lookback_dismissed;
    if (d && !(typeof stored === "string" && stored > dayKey(d))) await this.setSetting("lookback_dismissed", dayKey(d));
  }

  async newNote(title: string, space: Space = this.space): Promise<Note> {
    const c = this.startColour();
    const n = await this.api.createNote(title, space, c.palette, c.color);
    this.notes = [...this.notes, n];
    return n;
  }

  /** New order of the visible notes; hidden ones keep their place after them. */
  async reorder(visibleIds: number[]) {
    visibleIds = visibleIds.filter((id) => !inMemory(id));
    const hidden = this.notes.filter((n) => !visibleIds.includes(n.id)).sort((a, b) => a.sort - b.sort);
    const ids = [...visibleIds, ...hidden.map((n) => n.id)];
    const pos = new Map(ids.map((id, i) => [id, i]));
    this.notes = this.notes.map((n) => ({ ...n, sort: pos.get(n.id) ?? n.sort }));
    await this.api.reorderNotes(ids);
  }

  /** The board's saved columns for this view at this column count ("view:cols"). */
  boardSteps(key: string): Step[] | null {
    const all = this.settings.board_steps as Record<string, Step[]> | undefined;
    const s = all?.[key];
    return Array.isArray(s) ? s : null;
  }

  /** After a drag: keeps the columns as shown. Other column counts of the same view start
   *  again from the new order (their old arrangement no longer matches). */
  async saveBoardSteps(view: string, cols: number, steps: Step[]) {
    const all = { ...((this.settings.board_steps as Record<string, Step[]> | undefined) ?? {}) };
    for (const k of Object.keys(all)) if (k.startsWith(`${view}:`)) delete all[k];
    steps = steps.filter((s) => !inMemory(s.id));
    all[`${view}:${cols}`] = steps.map((s) => ({ id: s.id, c: s.c }));
    await Promise.all([this.setSetting("board_steps", all), this.reorder(steps.map((s) => s.id))]);
  }

  async setCustom(list: CustomColour[]) {
    await this.setSetting("custom_palette", list);
  }

  /** "Keep the panel open". */
  async setPinned(on: boolean) {
    this.pinned = on;
    await this.api.setPinned(on);
  }

  /** Floating: above every other window, or not (the header button and Settings). */
  async setFloatOnTop(on: boolean) {
    this.win = { ...this.win, float_on_top: on };
    await this.api.setFloatOnTop(on);
  }

  /** The docked panel's width: live while dragging, `save` remembers it for this screen. */
  async resizeDocked(width: number, snap: boolean, save: boolean): Promise<number> {
    const w = (await this.api.resizeDocked(width, snap, save)) ?? this.win.width;
    this.win = { ...this.win, width: w };
    if (save) this.settings = { ...this.settings, docked_width: w };
    return w;
  }

  async refreshWindow() {
    this.win = await this.api.windowInfo();
  }

  /** Jump to a note: shows its space when needed, then it glows. */
  focusNote(id: number) {
    const n = this.notes.find((x) => x.id === id);
    if (!n) return;
    if (this.view !== "all" && this.view !== n.space) void this.setView(n.space);
    // (The Inbox opens to show it.)
    if (this.inboxIds.has(n.id)) this.inboxOpen = true;
    this.glow = { id, at: Date.now() };
  }

  // ---- Pictures ----

  async addPictureBlob(noteId: number, itemId: number | null, blob: Blob) {
    if (inMemory(noteId) || inMemory(itemId)) return null;
    const ext = (blob.type.split("/")[1] || "png").replace("jpeg", "jpg");
    const bytes = new Uint8Array(await blob.arrayBuffer());
    const p = await this.api.savePicture(noteId, itemId, bytes, ext);
    this.pictures = [...this.pictures, p];
    thumbs.soon(this.api, p);
    return p;
  }

  async addPictureFile(noteId: number, itemId: number | null, path: string) {
    if (inMemory(noteId) || inMemory(itemId)) return null;
    const p = await this.api.addPictureFile(noteId, itemId, path);
    this.pictures = [...this.pictures, p];
    thumbs.soon(this.api, p);
    return p;
  }

  async deletePicture(p: Picture) {
    this.pictures = this.pictures.filter((x) => x.id !== p.id);
    await this.api.deletePicture(p.id);
  }

  src(path: string): string {
    return this.api.fileSrc(path);
  }

  /** A picture as a small box shows it (a strip, a row): its thumb, else the picture. */
  thumbSrc(p: Picture): string {
    return thumbs.src(p, (path) => this.api.fileSrc(path));
  }
}

export const store = new Store();

export const IMAGE_EXT = /\.(png|jpe?g|gif|webp|bmp)$/i;
