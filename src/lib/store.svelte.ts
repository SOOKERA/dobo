// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { backendPromise, type Backend } from "./backend";
import { findNoteForTag, parseCapture, titleFromTag, type Parsed } from "./capture";
import { defaultCustom, rotation, type CustomColour } from "./palette";
import { History } from "./history";
import { dropFromSteps, dropNotesFromSteps, putBackInSteps, type SavedSteps, type Step } from "./arrange";
import { dayKey, defaultDay, lookbackDay, type DaySettings } from "./plan";
import { backWords } from "./undoWords";
import { SAMPLE_ITEM, inMemory } from "./sample";
import { i18n, keysText, readChoice, t as tr, tMaybe, type Key, type Words } from "./i18n.svelte";
import { noteTitle } from "./names";
import { APP_NAME } from "./app";
import { arrivedSince, firstInboxes, inboxesLost, isInbox, savedInboxes } from "./inboxes";
import type {
  Alarm,
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
  lines: TemplateLine[];
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

class Store {
  notes = $state<Note[]>([]);
  items = $state<Item[]>([]);
  pictures = $state<Picture[]>([]);
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
  templatesOpen = $state(false);
  /** The Archive page, and where it was opened from (Back goes there). */
  archiveOpen = $state<"board" | "settings" | null>(null);
  /** A full view covers the board (Settings, the Archive, Search, Tidy…; App.svelte keeps it). */
  over = $state(false);
  /** Notes put back from the archive while a view covers the board: their cards settle in once
   * the board shows again (Board.svelte). */
  arriving = $state<number[]>([]);
  /** The board plays a note's way out (archived, deleted) before it goes; set by Board.svelte. */
  noteLeaves: ((ids: number[]) => Promise<void>) | null = null;
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

  /** The day whose look back shows now (or today, when opened from the menu). */
  get lookback(): Date | null {
    if (this.lookbackOpen) return lookbackDay(this.now.getTime(), this.day, this.workHours) ?? this.now;
    const d = lookbackDay(this.now.getTime(), this.day, this.workHours);
    if (!d || this.settings.lookback_dismissed === dayKey(d)) return null;
    return d;
  }

  showToast(text: Words, action?: Toast["action"], ms = 5200, more?: Toast["more"]) {
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toast = { id: Date.now(), text, action, more };
    this.toastTimer = setTimeout(() => (this.toast = null), ms);
  }

  /** A shortcut set long ago on a key Dobo now uses inside (Ctrl+Shift+S, Ctrl+Shift+P, Ctrl+,)
   * went back to its default at start (Rust): said once, then forgotten. */
  private sayMovedShortcuts() {
    const moved = this.settings.shortcut_moved as { from: string; to: string }[] | null | undefined;
    if (!Array.isArray(moved) || !moved.length) return;
    // Both may have moved (Open or hide and Quick dump): each is said.
    this.showToast(() => moved.map((m) => tr("{old} is used inside {app} now: your shortcut moved to {keys}", { old: keysText(m.from), keys: keysText(m.to), app: APP_NAME })).join(" · "), undefined, 10_000);
    void this.setSetting("shortcut_moved", null);
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
    this.replaceItem(next);
    const saved = await this.api.updateItem(next);
    this.replaceItem(saved);
    if (label && prev) {
      this.history.record({
        label,
        undo: async () => void (await this.putItem(prev, null)),
        redo: async () => void (await this.putItem(next, null)),
      });
    }
    return saved;
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
      this.history.record({
        label,
        undo: async () => void (await this.putNote(prev, null)),
        redo: async () => void (await this.putNote(next, null)),
      });
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

  /** Top-level lines of each note (steps are under their task). */
  itemsByNote = $derived.by(() => {
    const map = new Map<number, Item[]>();
    for (const it of [...this.items].sort((a, b) => a.sort - b.sort || a.id - b.id)) {
      if (it.parent_id != null) continue;
      const list = map.get(it.note_id);
      if (list) list.push(it);
      else map.set(it.note_id, [it]);
    }
    return map;
  });

  stepsOf = $derived.by(() => {
    const map = new Map<number, Item[]>();
    for (const it of [...this.items].sort((a, b) => a.sort - b.sort || a.id - b.id)) {
      if (it.parent_id == null) continue;
      const list = map.get(it.parent_id);
      if (list) list.push(it);
      else map.set(it.parent_id, [it]);
    }
    return map;
  });

  picturesByNote = $derived.by(() => {
    const map = new Map<number, Picture[]>();
    for (const p of [...this.pictures].sort((a, b) => a.sort - b.sort || a.id - b.id)) {
      if (p.item_id != null) continue;
      const list = map.get(p.note_id);
      if (list) list.push(p);
      else map.set(p.note_id, [p]);
    }
    return map;
  });

  picturesByItem = $derived.by(() => {
    const map = new Map<number, Picture[]>();
    for (const p of this.pictures) {
      if (p.item_id == null) continue;
      const list = map.get(p.item_id);
      if (list) list.push(p);
      else map.set(p.item_id, [p]);
    }
    return map;
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

  /** Open lines waiting in the Inbox (of the space showing; both when All). The tour's sample
   * line counts, as if just dumped, also while "Or make it a step" shows it as one; the bigger
   * task it is dropped onto there doesn't (the badge stays still under the tour's veil). */
  inboxWaiting = $derived(
    this.items
      .filter((i) => (i.id === SAMPLE_ITEM || (i.parent_id == null && !inMemory(i.id))) && i.done_at == null && this.inboxIds.has(i.note_id))
      .filter((i) => {
        if (this.view === "all") return true;
        return this.notes.find((n) => n.id === i.note_id)?.space === this.view;
      })
      .sort((a, b) => a.sort - b.sort || a.id - b.id),
  );

  async init() {
    try {
      // (The language was asked before anything was drawn: main.ts.)
      this.api = await backendPromise;
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
    this.api.onEvent("ring", (p) => this.onRing(p as RingEvent));
    this.api.onEvent("reload", () => void this.reload());
    setInterval(() => {
      this.now = new Date();
      this.applyClock(false);
    }, 20_000);
  }

  async reload() {
    const all = await this.api.loadAll();
    this.notes = all.notes;
    this.items = all.items;
    this.pictures = all.pictures ?? [];
    this.settings = all.settings;
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
    const w = this.workHours;
    const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    return w.days.includes(d.getDay()) && hm >= w.from && hm < w.until ? "work" : "personal";
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
  async setRepeat(it: Item, repeat: Repeat | null, due?: number) {
    await this.putItem({ ...it, repeat, kind: "task", due_at: due ?? it.due_at }, repeat ? "Repeat set" : "Repeat removed");
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

  async toggleDone(it: Item) {
    if (inMemory(it.id)) return;
    const prev = $state.snapshot(this.items.find((x) => x.id === it.id) ?? it) as Item;
    const next = { ...prev, done_at: prev.done_at == null ? Date.now() : null };
    await this.putItem(next, null);
    this.history.record({
      label: next.done_at != null ? "Done" : "Not done",
      undo: async () => {
        // A repeating task: its next occurrence goes away again.
        if (prev.done_at == null && prev.repeat) {
          const spawned = this.items.find((x) => x.id !== prev.id && x.note_id === prev.note_id && x.text === prev.text && x.repeat && x.done_at == null && x.created_at >= (next.done_at ?? 0) - 1000);
          if (spawned) {
            this.removeLocal([spawned.id]);
            await this.api.deleteItem(spawned.id);
          }
        }
        await this.putItem(prev, null);
      },
      redo: async () => void (await this.putItem({ ...prev, done_at: next.done_at }, null)),
    });
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

  /** Ctrl+Up / Ctrl+Down: swaps a line with its neighbour (same note, same level, same kind). */
  async moveLine(it: Item, dir: -1 | 1) {
    if (inMemory(it.id)) return;
    const sibs = this.items
      .filter((x) => x.note_id === it.note_id && (x.parent_id ?? null) === (it.parent_id ?? null) && x.kind === it.kind && (x.done_at == null) === (it.done_at == null))
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
      undo: async () => void (await Promise.all([this.putItem(prevA, null), this.putItem(prevB, null)])),
      redo: async () => void (await Promise.all([this.putItem(a, null), this.putItem(b, null)])),
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
    // Everything under it follows into the note (the backend moves steps of steps too).
    const follow = (noteId: number) => {
      const under = this.subtreeIds(it.id);
      this.items = this.items.map((x) => (x.id !== it.id && under.has(x.id) ? { ...x, note_id: noteId } : x));
      this.pictures = this.pictures.map((p) => (p.item_id != null && under.has(p.item_id) ? { ...p, note_id: noteId } : p));
    };
    follow(to.noteId);
    const before = this.items.find((x) => x.id === it.id) ?? it;
    await this.putItem(next, null);
    const back = { ...before, sort: it.sort };
    this.history.record({
      label: "Moved",
      undo: async () => {
        follow(back.note_id);
        await this.putItem(back, null);
      },
      redo: async () => {
        follow(next.note_id);
        await this.putItem(next, null);
      },
    });
    // A picture dumped with the line becomes the note's own once it is put away.
    if (!this.inboxIds.has(to.noteId)) {
      for (const p of this.pictures.filter((p) => p.item_id === it.id)) {
        await this.api.releasePicture(p.id, to.noteId);
        this.pictures = this.pictures.map((x) => (x.id === p.id ? { ...x, item_id: null, note_id: to.noteId } : x));
      }
    }
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
      await this.putNote(next, null);
    };
    await put();
    this.history.record({ label: "Archived", undo: async () => void (await this.putNote(prev, null)), redo: put });
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
      await this.putNote(next, null);
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
      await this.putNote(prev, null);
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
    await drop();
    this.history.record({ label, undo: back, redo: drop });
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
      .filter((i) => i.done_at == null && !inMemory(i.id))
      .map((i) => ({
        kind: i.kind,
        text: i.text,
        deadline: i.alarm === "deadline" || undefined,
        steps: (this.stepsOf.get(i.id) ?? []).map((s) => s.text),
      }));
    const t: Template = { id: Date.now(), name: n.title, palette: n.palette, color: n.color, paper: n.paper ?? "plain", lines };
    await this.setSetting("templates", [...this.templates.filter((x) => x.name !== t.name), t]);
    this.showToast(() => tr("Saved “{name}” as a template", { name: noteTitle(n) }));
  }

  async deleteTemplate(id: number) {
    await this.setSetting(
      "templates",
      this.templates.filter((t) => t.id !== id),
    );
  }

  /** A new note, empty or from a template. */
  async fromTemplate(t: Template | null, title: string): Promise<Note> {
    const name = title.trim() || t?.name || tr("New note");
    const c = this.startColour();
    let n = await this.api.createNote(name, this.space, t?.palette ?? c.palette, t?.color ?? c.color);
    if (t && t.paper !== "plain") n = await this.api.updateNote({ ...n, paper: t.paper });
    this.notes = [...this.notes, n];
    for (const line of t?.lines ?? []) {
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
    let kept: { notes: Note[]; items: Item[]; pictures: Picture[] } | null = null;
    this.history.record({
      label: "Note added",
      undo: async () => {
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
      },
      redo: async () => {
        if (!kept) return;
        await this.restoreAll(kept.notes, kept.items, kept.pictures);
        this.focusNote(id);
      },
    });
    this.focusNote(n.id);
    return n;
  }

  // ---- The evening look back ----

  /** Closes the look back. The evening's own card is said done for that day; one opened by hand
   * earlier in the day just closes (the evening's still comes). */
  async dismissLookback() {
    const d = lookbackDay(this.now.getTime(), this.day, this.workHours);
    this.lookbackOpen = false;
    if (d) await this.setSetting("lookback_dismissed", dayKey(d));
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
    return p;
  }

  async addPictureFile(noteId: number, itemId: number | null, path: string) {
    if (inMemory(noteId) || inMemory(itemId)) return null;
    const p = await this.api.addPictureFile(noteId, itemId, path);
    this.pictures = [...this.pictures, p];
    return p;
  }

  async deletePicture(p: Picture) {
    this.pictures = this.pictures.filter((x) => x.id !== p.id);
    await this.api.deletePicture(p.id);
  }

  src(path: string): string {
    return this.api.fileSrc(path);
  }
}

export const store = new Store();

export const IMAGE_EXT = /\.(png|jpe?g|gif|webp|bmp)$/i;
