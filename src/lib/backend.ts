// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The app talks to Rust through this. In a plain browser (`npm run dev` without Tauri) a mock
// keeps everything in localStorage, so the UI can be previewed and tested there
// (add ?w=dump to the address to preview the quick dump bar).
//
// The order of calls: most commands run one after another on Rust's main thread, in the order
// sent. A few run off it (`#[tauri::command(async)]` in lib.rs: restore, dumpItem, savePicture,
// addPictureFile, saveNoteImage, the backup commands, the Sticky Notes count and import, openPath,
// reveal…), and one of those can finish AFTER a command sent later. So always await an async
// command before the next call that reads or writes the same rows (undo awaits restore; the dump
// bar awaits dumpItem before setting the link): a "send, then edit" without awaiting may land on
// a row that isn't written yet.
import { nextAfterDone } from "./repeat";
import { dockWidth, widthForScreen } from "./dock";
import type {
  Everything,
  Item,
  LinkPreview,
  MonitorInfo,
  NewItem,
  Note,
  Picture,
  Rhythm,
  Thumb,
  RingAction,
  Space,
  WindowInfo,
} from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type EventName =
  | "nav"
  | "panel-shown"
  | "panel-hide-request"
  | "ring"
  | "item-updated"
  | "picture-added"
  | "reload"
  | "focus-item"
  | "open-tidy"
  | "tidy-ring"
  | "dump-shown"
  | "dump-source"
  | "focus-note"
  | "later-item"
  | "open-lookback"
  /** Global shortcuts another app holds at start (or after a restore): [{ which, keys }]. */
  | "shortcut-taken"
  | "lookback-ready"
  | "peek-shown"
  | "dark-mode"
  | "language"
  /** Windows switched between a 12- and a 24-hour clock (true: 12-hour). */
  | "clock-format"
  | "update-status"
  /** The automatic backup didn't work (its error; said once until one works again). */
  | "backup-failed";

/** Dobo's own update, as Settings › Updates shows it (src-tauri/src/updates.rs, View). */
export interface UpdateView {
  current: string;
  /** A newer version that exists. */
  latest: string | null;
  notes: string | null;
  /** Downloaded and checked: used at the next start, or with Restart now. */
  ready: boolean;
  busy: boolean;
  error: string | null;
  /** A check was made this session. */
  checked: boolean;
  /** When the last check that worked was made (ms). */
  last_checked: number | null;
}

export interface DropEvent {
  kind: "over" | "drop" | "leave";
  paths: string[];
  /** Logical pixels in the window. */
  x: number;
  y: number;
}

/** The dark-mode switch as the clock sees it (dark: null = follow Windows). */
export interface DarkNow {
  mode: string;
  from: string;
  until: string;
  dark: boolean | null;
}

export interface BackupInfo {
  name: string;
  path: string;
  at: number;
  bytes: number;
  /** The kind: "daily", "manual", "before-restore" or "before-update". */
  reason: string;
  /** Before an update: the version it was made for. */
  version: string | null;
  /** In the data folder's own backups folder, not the one chosen in Settings: a backup before an
   * update made there instead (the chosen one couldn't be used then). */
  kept_in_data: boolean;
}

/** Whether the last automatic backup worked (src-tauri/src/backup.rs, Status). */
export interface BackupStatus {
  /** Why it didn't (null: it worked, or none was made yet). */
  error: string | null;
  failed_at: number | null;
  /** A word of its own (the backup made before a restore misses what was typed during it; the
   * backup before a new version went elsewhere, or couldn't be made), said with `noticeText`. */
  notice: BackupNotice | null;
  notice_at: number | null;
}

/** A word of its own for Settings › Backups as Rust keeps it (backup.rs `Notice`): what it is
 * and its parts, said in the language of the moment; or, kept by an older version, the finished
 * words (said as they are). */
export type BackupNotice =
  | string
  | { code: "restore_missed"; args: { error: string; path: string } }
  | { code: "update_backup_elsewhere"; args: { app: string; version: string; path: string; chosen: string; why?: BackupMissed | null } }
  | { code: "update_backup_failed"; args: { app: string; version: string; error: string; why?: BackupMissed | null } };

/** Why a backup before an update didn't go where it should (backup.rs `Missed`; none: kept by an
 * earlier 1.1 build). */
export type BackupMissed = "no_answer" | "not_writable" | "full" | "failed";

/** Settings › About: how this copy runs and where its notes are. */
export interface DataInfo {
  kind: "installed" | "portable" | "dev";
  dir: string;
  /** Portable, using the notes this computer already had (the usual folder). */
  kept_existing: boolean;
  /** Portable, but the folder next to Dobo.exe can't be written (the usual folder). */
  fallback: boolean;
  version: string;
}

/** The notes didn't open at start (src-tauri/src/recovery.rs, Problem): the recovery screen. */
export interface RecoveryProblem {
  /** damaged: repair, try again, restore or start empty · locked: another program holds the file ·
   * readonly: it can't be written · failed: it won't open for another reason · portable: a
   * portable copy can't write in its own folder (asked before the usual one is used). */
  kind: "damaged" | "locked" | "readonly" | "failed" | "portable";
  /** SQLite's or Windows' own words. */
  error: string;
  /** The data folder, and the notes file in it. */
  data: string;
  file: string;
  /** The file an earlier start put aside (broken-<date>.sqlite, nothing in its place), in the
   * data folder. */
  kept_as: string | null;
  /** The newest backup whose notes are whole, and what it holds (null: it couldn't be counted). */
  backup: { path: string; at: number; counts: RecoveryCounts | null } | null;
  /** Damaged: a repaired copy is ready ("Use the repaired notes"). */
  repaired: boolean;
  /** What the repaired copy holds (a repair may miss notes it couldn't read). */
  repaired_counts: RecoveryCounts | null;
  /** The file put aside (kept_as) reads whole now: it can be put back. */
  whole_again: boolean;
  /** Portable: the folder next to Dobo that can't be written, and whether notes are in it. */
  portable: string | null;
  portable_notes: boolean;
}
export type RecoveryChoice = "repair" | "putback" | "restore" | "empty" | "retry" | "usual";
/** What notes hold, on a recovery choice's card: every note (archived ones too) and every line. */
export interface RecoveryCounts {
  notes: number;
  lines: number;
}

export interface Backend {
  /** "main" or "dump". */
  label: string;
  loadAll(): Promise<Everything>;
  createNote(title: string, space: Space, palette: string, color: string): Promise<Note>;
  updateNote(note: Note): Promise<Note>;
  deleteNote(id: number): Promise<void>;
  reorderNotes(ids: number[]): Promise<void>;
  createItem(n: NewItem): Promise<Item>;
  updateItem(item: Item): Promise<Item>;
  deleteItem(id: number): Promise<void>;
  setSetting(key: string, value: unknown): Promise<void>;
  inboxFor(space: Space): Promise<number>;
  /** An Inbox made only to stand in for one an undo put back in the archive: removed while it is
   * still empty (the space had none before). Whether it went. */
  dropEmptyInbox(id: number): Promise<boolean>;
  /** How many Windows Sticky Notes an import would bring in (0: none, or all already here).
   * Only reads; nothing comes in until `importStickyNotes`. */
  countStickyNotes(): Promise<number>;
  /** Brings in the Windows Sticky Notes not already here. How many came in. */
  importStickyNotes(): Promise<number>;
  windowInfo(): Promise<WindowInfo>;
  hidePanel(): Promise<void>;
  applyWindow(): Promise<void>;
  setFloatOnTop(on: boolean): Promise<void>;
  /** The docked panel’s width (live; save remembers it for this screen). Returns the width used. */
  resizeDocked(width: number, snap: boolean, save: boolean): Promise<number | null>;
  /** "Keep the panel open" (a pinned docked panel also steps aside for full-screen apps). */
  setPinned(on: boolean): Promise<void>;
  minimize(): Promise<void>;
  startDragging(): Promise<void>;
  startResize(direction: string): Promise<void>;
  listMonitors(): Promise<MonitorInfo[]>;
  answerRing(itemId: number, action: RingAction): Promise<Item>;
  /** The board listens: what notifications asked of it before (Dobo started by a click on one). */
  pageReady(): Promise<[EventName, unknown][]>;
  tidyAnswer(action: string): Promise<void>;
  rhythmPresets(): Promise<Record<"gentle" | "normal" | "insistent", Rhythm>>;
  showDump(): Promise<void>;
  hideDump(): Promise<void>;
  dumpItem(text: string, link: LinkPreview | null, file: string | null): Promise<[Item, number]>;
  fetchPreview(url: string): Promise<LinkPreview>;
  /** Only the line's link (a preview that arrived later); the rest of the line is kept as it is now. */
  setItemLink(id: number, link: LinkPreview | null): Promise<Item>;
  openPath(path: string): Promise<void>;
  openUrl(url: string): Promise<void>;
  savePicture(noteId: number, itemId: number | null, bytes: Uint8Array, ext: string): Promise<Picture>;
  addPictureFile(noteId: number, itemId: number | null, path: string): Promise<Picture>;
  deletePicture(id: number): Promise<void>;
  /** A picture's board-size copy, made now unless it was already; null when it is small enough
   * as it is (thumbs.rs). */
  makeThumb(path: string): Promise<Thumb | null>;
  releasePicture(id: number, noteId: number): Promise<void>;
  setShortcut(which: "toggle" | "dump" | "search", accel: string): Promise<void>;
  /** The global shortcuts another app holds ("toggle", "dump"): they do nothing until changed. */
  shortcutsTaken(): Promise<string[]>;
  getAutostart(): Promise<boolean>;
  setAutostart(on: boolean): Promise<void>;
  /** Undo after a delete: puts them back exactly as they were. */
  restore(notes: Note[], items: Item[], pictures: Picture[]): Promise<void>;
  /** From the peeking strip: the whole panel, on this note. */
  openNote(noteId: number): Promise<void>;
  endPeek(): Promise<void>;
  /** Saves a PNG into Pictures › Dobo; returns the file. */
  saveNoteImage(bytes: Uint8Array, name: string): Promise<string>;
  reveal(path: string): Promise<void>;
  backupNow(): Promise<BackupInfo>;
  backupList(): Promise<BackupInfo[]>;
  backupStatus(): Promise<BackupStatus>;
  backupRestore(path: string): Promise<void>;
  backupFolder(): Promise<string>;
  dataInfo(): Promise<DataInfo>;
  /** "Agree and continue" on the terms of use: saved, and Dobo may go online from now on. The
   * date saved (the terms' "Last updated" date, "2026-11-21"). */
  acceptTerms(): Promise<string>;
  /** "Quit Dobo" (on the terms). */
  quitApp(): Promise<void>;
  updateStatus(): Promise<UpdateView>;
  /** "Check for updates": a newer Dobo is downloaded and checked, ready for the next start. */
  updateCheck(): Promise<UpdateView>;
  /** "Restart now": Dobo quits and starts the new version. */
  updateRestart(): Promise<void>;
  /** Opens the data folder in Explorer. */
  openDataFolder(): Promise<void>;
  /** At start: what the recovery screen shows (null: the notes opened, the board shows). */
  recoveryState(): Promise<RecoveryProblem | null>;
  /** A choice on the recovery screen: null once the notes are open, else what still stands in
   * the way. Fails when it couldn't be done (nothing changed). */
  recoveryChoose(choice: RecoveryChoice): Promise<RecoveryProblem | null>;
  /** The notes are open: the board starts (then the board's page is shown). Gives the name the
   * damaged file was put aside as, if it was. */
  recoveryContinue(): Promise<string | null>;
  /** A folder picked in the system dialog (null: cancelled). */
  pickFolder(): Promise<string | null>;
  /** A file's address for <img src>. */
  fileSrc(path: string): string;
  darkModeNow(): Promise<DarkNow>;
  /** The language setting ("system", "en", "fr") and the language Windows shows its own words in
   * ("fr", else "en"). */
  /** The language setting, the one Windows speaks, and whether Windows uses a 12-hour clock. */
  languageNow(): Promise<{ choice: string; system: "en" | "fr"; hour12: boolean }>;
  /** Saves a .ztheme where the person chooses; null when cancelled. */
  exportLook(content: string, name: string): Promise<string | null>;
  /** Reads a .ztheme (the given file, or one picked); [file name, checked contents] or null. */
  importLook(path: string | null): Promise<[string, string] | null>;
  /** A picture for the board, copied next to the notes; its path, or null when cancelled. */
  pickBackgroundPicture(): Promise<string | null>;
  onEvent(name: EventName, cb: (payload: unknown) => void): void;
  onFocusChanged(cb: (focused: boolean) => void): void;
  onFileDrop(cb: (e: DropEvent) => void): void;
}

async function tauriBackend(): Promise<Backend> {
  const { invoke, convertFileSrc } = await import("@tauri-apps/api/core");

  const { listen } = await import("@tauri-apps/api/event");
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  const win = getCurrentWindow();
  return {
    label: win.label,
    loadAll: () => invoke("load_all"),
    createNote: (title, space, palette, color) => invoke("create_note", { title, space, palette, color }),
    updateNote: (note) => invoke("update_note", { note }),
    deleteNote: (id) => invoke("delete_note", { id }),
    reorderNotes: (ids) => invoke("reorder_notes", { ids }),
    createItem: (n) =>
      invoke("create_item", {
        noteId: n.noteId,
        kind: n.kind,
        text: n.text,
        dueAt: n.dueAt ?? null,
        parentId: n.parentId ?? null,
        alarm: n.alarm ?? "reminder",
        link: n.link ?? null,
        file: n.file ?? null,
      }),
    updateItem: (item) => invoke("update_item", { item }),
    deleteItem: (id) => invoke("delete_item", { id }),
    setSetting: (key, value) => invoke("set_setting", { key, value }),
    inboxFor: (space) => invoke("inbox_for", { space }),
    dropEmptyInbox: (id) => invoke("drop_empty_inbox", { id }),
    countStickyNotes: () => invoke("count_sticky_notes"),
    importStickyNotes: () => invoke("import_sticky_notes"),
    windowInfo: () => invoke("window_info"),
    hidePanel: () => invoke("hide_panel"),
    applyWindow: () => invoke("apply_window"),
    setFloatOnTop: (on) => invoke("set_float_on_top", { on }),
    resizeDocked: (width, snap, save) => invoke("resize_docked", { width, snap, save }),
    setPinned: (on) => invoke("set_pinned", { on }),
    minimize: () => win.minimize(),
    startDragging: () => win.startDragging(),
    startResize: (direction) => win.startResizeDragging(direction as never),
    listMonitors: () => invoke("list_monitors"),
    answerRing: (itemId, action) => invoke("answer_ring", { itemId, action }),
    pageReady: () => invoke("page_ready"),
    tidyAnswer: (action) => invoke("tidy_answer", { action }),
    rhythmPresets: () => invoke("rhythm_presets"),
    showDump: () => invoke("show_dump"),
    hideDump: () => invoke("hide_dump"),
    dumpItem: (text, link, file) => invoke("dump_item", { text, link, file }),
    fetchPreview: (url) => invoke("fetch_preview", { url }),
    setItemLink: (id, link) => invoke("set_item_link", { id, link }),
    openPath: (path) => invoke("open_path", { path }),
    openUrl: (url) => invoke("open_url", { url }),
    savePicture: (noteId, itemId, bytes, ext) =>
      invoke("save_picture", bytes, { headers: { "note-id": String(noteId), "item-id": itemId == null ? "" : String(itemId), ext } }),
    addPictureFile: (noteId, itemId, path) => invoke("add_picture_file", { noteId, itemId, path }),
    deletePicture: (id) => invoke("delete_picture", { id }),
    makeThumb: (path) => invoke("make_thumb", { path }),
    releasePicture: (id, noteId) => invoke("release_picture", { id, noteId }),
    setShortcut: (which, accel) => invoke("set_shortcut", { which, accel }),
    shortcutsTaken: () => invoke("shortcuts_taken"),
    getAutostart: () => invoke("get_autostart"),
    setAutostart: (on) => invoke("set_autostart", { on }),
    restore: (notes, items, pictures) => invoke("restore", { notes, items, pictures }),
    openNote: (noteId) => invoke("open_note", { noteId }),
    endPeek: () => invoke("end_peek"),
    saveNoteImage: (bytes, name) => invoke("save_note_image", bytes, { headers: { name: encodeURIComponent(name) } }),
    reveal: (path) => invoke("reveal", { path }),
    backupNow: () => invoke("backup_now"),
    backupList: () => invoke("backup_list"),
    backupStatus: () => invoke("backup_status"),
    backupRestore: (path) => invoke("backup_restore", { path }),
    backupFolder: () => invoke("backup_folder"),
    dataInfo: () => invoke("data_info"),
    acceptTerms: () => invoke("accept_terms"),
    quitApp: () => invoke("quit_app"),
    updateStatus: () => invoke("update_status"),
    updateCheck: () => invoke("update_check"),
    updateRestart: () => invoke("update_restart"),
    openDataFolder: () => invoke("open_data_folder"),
    recoveryState: () => invoke("recovery_state"),
    recoveryChoose: (choice) => invoke("recovery_choose", { choice }),
    recoveryContinue: () => invoke("recovery_continue"),
    pickFolder: async () => {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { t } = await import("./i18n.svelte");
      const r = await open({ directory: true, multiple: false, title: t("Folder for the backups") });
      return typeof r === "string" ? r : null;
    },
    fileSrc: (path) => convertFileSrc(path),
    darkModeNow: () => invoke("dark_mode_now"),
    languageNow: () => invoke("language_now"),
    exportLook: (content, name) => invoke("export_look", { content, name }),
    importLook: (path) => invoke("import_look", { path }),
    pickBackgroundPicture: () => invoke("pick_background_picture"),
    onEvent: (name, cb) => void listen(name, (e) => cb(e.payload)),
    onFocusChanged: (cb) => void win.onFocusChanged((e) => cb(e.payload)),
    onFileDrop: (cb) =>
      void getCurrentWebview().onDragDropEvent((e) => {
        const p = e.payload as { type: string; paths?: string[]; position?: { x: number; y: number } };
        const ratio = window.devicePixelRatio || 1;
        const x = (p.position?.x ?? 0) / ratio;
        const y = (p.position?.y ?? 0) / ratio;
        if (p.type === "over" || p.type === "enter") cb({ kind: "over", paths: p.paths ?? [], x, y });
        else if (p.type === "drop") cb({ kind: "drop", paths: p.paths ?? [], x, y });
        else cb({ kind: "leave", paths: [], x, y });
      }),
  };
}

// ---- Mock (browser preview) ----

// (The preview's storage keys keep the app's working name, ZToDo: a preview board survives.)
const KEY = "ztodo-mock-v3";
const MIN = 60_000;

export function blankItem(over: Partial<Item>): Item {
  return {
    id: 0,
    note_id: 0,
    kind: "task",
    text: "",
    due_at: null,
    done_at: null,
    sort: 0,
    created_at: Date.now(),
    parent_id: null,
    alarm: "reminder",
    rhythm: null,
    link: null,
    file: null,
    repeat: null,
    ring_state: "",
    rings: 0,
    last_ring_at: null,
    snooze_until: null,
    on_it_at: null,
    check_at: null,
    ...over,
  };
}

/** A drawn placeholder picture for the preview (a perspective floor, like the boards). */
function samplePicture(hue: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="300" viewBox="0 0 240 150"><rect width="240" height="150" fill="hsl(${hue},18%,88%)"/><path d="M0 150 L120 58 L240 150 Z" fill="hsl(${hue},16%,82%)"/><path d="M120 58 L40 150 M120 58 L80 150 M120 58 L160 150 M120 58 L200 150 M30 128 H210 M58 107 H182 M82 88 H158 M100 74 H140" stroke="hsl(${hue},14%,70%)" stroke-width="1"/><path d="M70 138 L104 94 L150 103 L176 140 Z" fill="hsl(${hue},45%,62%)" fill-opacity="0.45" stroke="hsl(${hue},45%,50%)" stroke-width="1.4"/></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function seed(): Everything {
  const now = Date.now();
  const nextMonday = (h: number) => {
    const d = new Date();
    d.setHours(h, 0, 0, 0);
    do d.setDate(d.getDate() + 1);
    while (d.getDay() !== 1);
    return d.getTime();
  };
  const at = (h: number, m: number, plusDays = 0) => {
    const d = new Date();
    d.setDate(d.getDate() + plusDays);
    d.setHours(h, m, 0, 0);
    return d.getTime();
  };
  let nid = 0;
  let iid = 0;
  const notes: Note[] = [];
  const items: Item[] = [];
  type Line = [Item["kind"], string, Partial<Item>?];
  const note = (title: string, space: Space, color: string, lines: Line[]) => {
    const id = ++nid;
    notes.push({ id, title, space, palette: "soft", color, sort: id, archived: false, created_at: now, wide: false, folded: false, paper: "plain" });
    lines.forEach(([kind, text, over], i) => items.push(blankItem({ id: ++iid, note_id: id, kind, text, sort: i, created_at: now - (lines.length - i) * 3600_000, ...over })));
    return id;
  };
  note("Inbox", "work", "Butter", [
    ["task", "Ask Mélanie for the Hangar Y files"],
    ["task", "Book of Concord"],
    [
      "task",
      "Shader breakdown to read",
      { link: { url: "https://www.example.com/shaders/wet-surfaces", title: "Wet surfaces in real time: a practical breakdown", site: "example.com", image: samplePicture(200) } },
    ],
    ["idea", "Off days : feux d’artifice, F1 qui volent en passant"],
  ]);
  const logi = note("Logitech", "work", "Sky", [
    ["task", "Reply to Logitech about the assets", { due_at: now - 12 * MIN, ring_state: "ringing", rings: 3, last_ring_at: now - 2 * MIN }],
    ["task", "Migrate the Logitech stuff to GitHub"],
    ["task", "EDR et MX Master 4 repo", { due_at: at(9, 0, 1) }],
    ["idea", "Fake un making-of de Logi"],
    ["idea", "Walkthrough Hangars Y"],
  ]);
  const migrate = items.find((i) => i.note_id === logi && i.text.startsWith("Migrate"))!;
  items.push(blankItem({ id: ++iid, note_id: logi, kind: "task", text: "List the repos to move", parent_id: migrate.id, sort: 10 }));
  items.push(blankItem({ id: ++iid, note_id: logi, kind: "task", text: "Ask IT for the org access", parent_id: migrate.id, sort: 11, done_at: now }));
  note("Hub", "work", "Sage", [
    ["task", "Call général, début et fin de projet", { due_at: at(15, 0) }],
    ["task", "Weekly", { due_at: nextMonday(10), repeat: { freq: "week", interval: 1, days: [1], monthday: null, ends: { kind: "never" }, done: 0 } }],
    ["idea", "Mini projet pour le hub"],
    ["idea", "Quick share pour le hub"],
  ]);
  const chanel = note("Chanel VR", "work", "Rose", [
    ["task", "Send navmesh build to Chanel", { due_at: at(17, 30), alarm: "deadline", rhythm: { preset: "normal" } }],
    ["task", "Fix glass on VR Quest"],
    ["task", "Gizmo mesh separate"],
    ["task", "Get some product, 1/2 bag", { done_at: now }],
  ]);
  note("Runway · Grand Palais", "work", "Lilac", [
    ["task", "Better Grand Palais"],
    ["task", "Fashion show models"],
    ["task", "Spline, défilé"],
    ["idea", "Loin mat, proche glossy"],
    ["idea", "Lumière et caméra en un seul pass"],
  ]);
  note("Tour du monde", "personal", "Ocean", [
    ["idea", "Naviguer dans l’océan, super chaotique"],
    ["idea", "6 mois dessus : plus une émotion qu’une expérience"],
    ["idea", "Moins de prestige, plus de tech et d’écologie"],
  ]);
  note("Material rework", "work", "Stone", [
    ["task", "Consolidate basic master"],
    ["task", "AO", { done_at: now }],
    ["task", "Wetness, raindrops"],
    ["task", "Layers"],
    ["task", "Blend / VT"],
    ["task", "Landscape"],
  ]);
  note("Philanthropie", "personal", "Peach", [["task", "Agrandir la height du perso"], ["idea", "Chirurgien charpentier"]]);
  const pictures: Picture[] = [0, 1, 2].map((k) => ({ id: k + 1, note_id: chanel, item_id: null, path: samplePicture([350, 20, 300][k]), sort: k, created_at: now }));
  return { notes, items, pictures, settings: {} };
}

const PRESETS: Record<"gentle" | "normal" | "insistent", Rhythm> = {
  gentle: { start: 30, pre: [15], every: 10, max: 3, checkin: 45, notyet: 20 },
  normal: { start: 60, pre: [30, 15, 5], every: 5, max: 6, checkin: 30, notyet: 15 },
  insistent: { start: 120, pre: [60, 30, 15, 10, 5], every: 3, max: 10, checkin: 20, notyet: 10 },
};

function mockBackend(): Backend {
  let data: Everything;
  try {
    data = JSON.parse(localStorage.getItem(KEY) || "null") || seed();
  } catch {
    data = seed();
  }
  data.pictures ??= [];
  const save = () => {
    try {
      localStorage.setItem(KEY, JSON.stringify(data));
    } catch {
      /* preview only */
    }
  };
  // A new id is never one given before, as in the app (AUTOINCREMENT): an undo that puts a
  // deleted note back never lands on one made meanwhile.
  const SEQ = "ztodo-mock-seq";
  const nextId = (table: "notes" | "items" | "pictures") => {
    let seq: Record<string, number> = {};
    try {
      seq = JSON.parse(localStorage.getItem(SEQ) || "{}") ?? {};
    } catch {
      /* preview only */
    }
    const id = Math.max(seq[table] ?? 0, ...data[table].map((x) => x.id), 0) + 1;
    seq[table] = id;
    try {
      localStorage.setItem(SEQ, JSON.stringify(seq));
    } catch {
      /* preview only */
    }
    return id;
  };
  const params = new URLSearchParams(location.search);
  const listeners: Record<string, ((p: unknown) => void)[]> = {};
  const emit = (name: string, p: unknown) => listeners[name]?.forEach((cb) => cb(p));
  const ok = <T,>(v: T) => Promise.resolve(structuredClone(v));
  const recover = params.get("recover");
  const previewKept = `broken-${new Date().toISOString().slice(0, 10)}.sqlite`;
  let previewChosen: RecoveryChoice | null = null;
  let previewContinueFailed = false;
  let previewProblem: RecoveryProblem | null =
    recover === "damaged" || recover === "locked" || recover === "readonly" || recover === "failed" || recover === "portable"
      ? {
          kind: recover,
          error: recover === "damaged" ? (params.has("whole") ? "" : "database disk image is malformed") : recover === "locked" ? "The process cannot access the file because it is being used by another process. (os error 32)" : recover === "readonly" ? "Access is denied. (os error 5)" : recover === "portable" ? "" : "disk I/O error",
          data: "C:\\Users\\you\\AppData\\Roaming\\com.sookera.ztodo",
          file: "C:\\Users\\you\\AppData\\Roaming\\com.sookera.ztodo\\ztodo.sqlite",
          kept_as: recover === "damaged" && params.has("aside") ? previewKept : null,
          // (…&counts=0: none could be counted; …&one: one note and one line.)
          backup: recover === "damaged" || recover === "failed" ? (params.get("backup") === "0" ? null : { path: "preview/backup", at: Date.now() - 26 * 3600_000, counts: params.get("counts") === "0" ? null : params.has("one") ? { notes: 1, lines: 1 } : { notes: 24, lines: 187 } }) : null,
          repaired: recover === "damaged" && params.has("repaired"),
          repaired_counts: recover === "damaged" && params.has("repaired") && params.get("counts") !== "0" ? (params.has("one") ? { notes: 1, lines: 1 } : { notes: 23, lines: 171 }) : null,
          whole_again: recover === "damaged" && params.has("aside") && params.has("whole"),
          portable: recover === "portable" ? "E:\\Dobo\\data" : null,
          portable_notes: recover === "portable" && params.has("notes"),
        }
      : null;
  let previewUpdate: UpdateView = { current: "1.1.0", latest: null, notes: null, ready: false, busy: false, error: null, checked: false, last_checked: null };
  const findItem = (id: number) => data.items.find((i) => i.id === id)!;
  const spawnNext = (it: Item) => {
    if (!it.repeat || it.due_at == null || it.kind !== "task") return;
    const found = nextAfterDone(it.repeat, it.due_at);
    if (found == null) return;
    const n = blankItem({ ...it, id: nextId("items"), due_at: found.at, done_at: null, created_at: Date.now(), sort: it.sort + 0.001, repeat: { ...it.repeat, done: found.done }, ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null });
    data.items = data.items.map((x) => (x.id === it.id ? { ...x, repeat: null } : x));
    data.items.push(n);
    save();
    setTimeout(() => {
      emit("item-updated", structuredClone({ ...it, repeat: null }));
      emit("item-updated", structuredClone(n));
    });
  };
  /** A space's Inbox, made when missing (the first one, as the app's: a second note called
   * "Inbox" is a plain note). */
  const inboxOf = (space: Space): number => {
    const found = data.notes.filter((n) => n.title === "Inbox" && n.space === space && !n.archived).sort((a, b) => a.id - b.id)[0];
    if (found) return found.id;
    const min = data.notes.reduce((m, n) => Math.min(m, n.sort), 0);
    const n: Note = { id: nextId("notes"), title: "Inbox", space, palette: "soft", color: "Butter", sort: min - 1, archived: false, created_at: Date.now(), wide: false, folded: false, paper: "plain" };
    data.notes.push(n);
    save();
    return n.id;
  };
  /** After a change to the notes, as in the app (`keep_inboxes`): a space whose Inbox just went
   * has one again at once, the next note called "Inbox" or a new one. */
  const inboxSpaces = () => new Set(data.notes.filter((n) => n.title === "Inbox" && !n.archived).map((n) => n.space));
  const keepInboxes = (had: Set<Space>) => {
    const now = inboxSpaces();
    for (const space of had) if (!now.has(space)) inboxOf(space);
  };
  const STICKY_COLORS = ["Butter", "Sage", "Rose", "Sky", "Lilac", "Peach", "Ocean", "Sand", "Stone"];
  /** The preview's sample sticky notes: [id, first line, more lines…]. */
  const previewSticky = (): [string, string, ...string[]][] =>
    params.get("sticky") === "0"
      ? []
      : [
          ["p1", "Call the plumber"],
          ["p2", "Gift ideas", "A good pen for Sam", "Tickets for the concert"],
          ["p3", "Read later", "The long article on sleep"],
        ];
  const stickyIds = (): string[] => (Array.isArray(data.settings.sticky_ids) ? [...(data.settings.sticky_ids as string[])] : []);
  const BACKUPS = "ztodo-mock-backups";
  /** The preview's data folder (?copy=…, see dataInfo). */
  const previewDataDir = () => {
    const c = params.get("copy");
    return c === "installed" || c === "kept" || c === "fallback" ? "C:\\Users\\you\\AppData\\Roaming\\com.sookera.ztodo" : c === "dev" ? "D:\\Projects\\Dobo\\data" : "D:\\Apps\\Dobo\\data";
  };
  const backups = (): (BackupInfo & { data: Everything })[] => {
    try {
      return JSON.parse(localStorage.getItem(BACKUPS) || "[]");
    } catch {
      return [];
    }
  };
  return {
    label: params.get("w") === "dump" ? "dump" : "main",
    loadAll: () => ok(data),
    createNote: (title, space, palette, color) => {
      const sort = data.notes.reduce((m, n) => Math.max(m, n.sort), 0) + 1;
      const n: Note = { id: nextId("notes"), title, space, palette: palette as Note["palette"], color, sort, archived: false, created_at: Date.now(), wide: false, folded: false, paper: "plain" };
      data.notes.push(n);
      save();
      return ok(n);
    },
    updateNote: (note) => {
      // The archive time follows `archived`, as in the real backend.
      const old = data.notes.find((n) => n.id === note.id);
      const at = !note.archived ? null : old?.archived ? (old.archived_at ?? note.archived_at ?? Date.now()) : (note.archived_at ?? Date.now());
      const saved: Note = { ...note, archived_at: at };
      const had = inboxSpaces();
      data.notes = data.notes.map((n) => (n.id === note.id ? saved : n));
      keepInboxes(had);
      save();
      return ok(saved);
    },
    deleteNote: (id) => {
      const had = inboxSpaces();
      data.notes = data.notes.filter((n) => n.id !== id);
      keepInboxes(had);
      data.items = data.items.filter((i) => i.note_id !== id);
      data.pictures = data.pictures.filter((p) => p.note_id !== id);
      save();
      return Promise.resolve();
    },
    reorderNotes: (ids) => {
      ids.forEach((id, i) => {
        const n = data.notes.find((x) => x.id === id);
        if (n) n.sort = i;
      });
      save();
      return Promise.resolve();
    },
    createItem: (n) => {
      const sort = data.items.filter((i) => i.note_id === n.noteId).reduce((m, i) => Math.max(m, i.sort), -1) + 1;
      const it = blankItem({
        id: nextId("items"),
        note_id: n.noteId,
        kind: n.kind,
        text: n.text,
        due_at: n.dueAt ?? null,
        parent_id: n.parentId ?? null,
        alarm: n.alarm ?? "reminder",
        link: n.link ?? null,
        file: n.file ?? null,
        sort,
      });
      data.items.push(it);
      save();
      return ok(it);
    },
    updateItem: (item) => {
      const old = findItem(item.id);
      let next: Item = { ...old, ...item, ring_state: old.ring_state, rings: old.rings, last_ring_at: old.last_ring_at, snooze_until: old.snooze_until, on_it_at: old.on_it_at, check_at: old.check_at };
      if (old.due_at !== item.due_at || old.done_at !== item.done_at || old.alarm !== item.alarm || JSON.stringify(old.rhythm) !== JSON.stringify(item.rhythm))
        next = { ...next, ring_state: "", rings: 0, last_ring_at: null, snooze_until: null, on_it_at: null, check_at: null };
      data.items = data.items.map((i) => (i.id === item.id ? next : i.parent_id === item.id ? { ...i, note_id: next.note_id } : i));
      save();
      // A repeating task ticked: its next occurrence (the Rust side does the same).
      if (old.done_at == null && next.done_at != null) spawnNext(next);
      return ok(next);
    },
    deleteItem: (id) => {
      // The line and everything under it (steps, steps of steps), as the Rust side does.
      const gone = new Set([id]);
      for (let grew = true; grew; ) {
        grew = false;
        for (const i of data.items)
          if (i.parent_id != null && gone.has(i.parent_id) && !gone.has(i.id)) {
            gone.add(i.id);
            grew = true;
          }
      }
      data.items = data.items.filter((i) => !gone.has(i.id));
      data.pictures = data.pictures.filter((p) => p.item_id == null || !gone.has(p.item_id));
      save();
      return Promise.resolve();
    },
    setSetting: (key, value) => {
      data.settings[key] = value;
      save();
      return Promise.resolve();
    },
    inboxFor: async (space) => inboxOf(space),
    dropEmptyInbox: async (id) => {
      const n = data.notes.find((x) => x.id === id);
      const empty = !!n && n.title === "Inbox" && !n.archived && !data.items.some((i) => i.note_id === id) && !data.pictures.some((p) => p.note_id === id);
      if (empty) {
        data.notes = data.notes.filter((x) => x.id !== id);
        save();
      }
      return empty;
    },
    // The preview has a few sample sticky notes, as a Windows PC with Sticky Notes would
    // (none with ?sticky=0). Brought in as the app does: a note each, a one-line one into the
    // Inbox, never twice.
    countStickyNotes: async () => previewSticky().filter(([id]) => !stickyIds().includes(id)).length,
    importStickyNotes: async () => {
      const ids = stickyIds();
      const found = previewSticky().filter(([id]) => !ids.includes(id));
      found.forEach(([id, title, ...lines], i) => {
        const noteId = lines.length ? nextId("notes") : inboxOf("work");
        if (lines.length) {
          const sort = data.notes.reduce((m, n) => Math.max(m, n.sort), 0) + 1;
          data.notes.push({ id: noteId, title, space: "work", palette: "soft", color: STICKY_COLORS[i % STICKY_COLORS.length], sort, archived: false, created_at: Date.now(), wide: false, folded: false, paper: "plain" });
        }
        const base = data.items.filter((x) => x.note_id === noteId).reduce((m, x) => Math.max(m, x.sort), -1) + 1;
        (lines.length ? lines : [title]).forEach((text, k) => data.items.push(blankItem({ id: nextId("items"), note_id: noteId, kind: "task", text, sort: base + k, created_at: Date.now() })));
        ids.push(id);
      });
      data.settings.sticky_ids = ids;
      if (found.length) data.settings.sticky_imported = found.length;
      save();
      return found.length;
    },
    windowInfo: () => {
      const chosen = ((params.get("mode") as WindowInfo["mode"]) || (data.settings.mode as WindowInfo["mode"]) || "docked") as WindowInfo["mode"];
      return ok({ mode: chosen, chosen, docked_possible: true, side: (data.settings.side as "left" | "right") || "right", float_on_top: !!data.settings.float_on_top, width: dockWidth(widthForScreen(data.settings.docked_widths, "preview", data.settings.docked_width), window.innerWidth, false) });
    },
    hidePanel: () => {
      // In the preview the panel comes back after a moment, as if the shortcut was pressed.
      setTimeout(() => emit("panel-shown", null), 900);
      return Promise.resolve();
    },
    applyWindow: () => Promise.resolve(),
    setFloatOnTop: (on) => {
      data.settings.float_on_top = on;
      save();
      return Promise.resolve();
    },
    // The preview is one "screen" (the browser window).
    resizeDocked: (width, snap, keep) => {
      const w = dockWidth(width, window.innerWidth, snap);
      if (keep) {
        data.settings.docked_widths = { ...((data.settings.docked_widths as object) ?? {}), preview: w };
        data.settings.docked_width = w;
        save();
      }
      return Promise.resolve(w);
    },
    setPinned: () => Promise.resolve(),
    minimize: () => Promise.resolve(),
    startDragging: () => Promise.resolve(),
    startResize: () => Promise.resolve(),
    listMonitors: () =>
      ok([
        { key: "DISPLAY1@0,0", name: "\\\\.\\DISPLAY1", x: 0, y: 0, width: 2560, height: 1440, scale: 1, primary: true },
        { key: "DISPLAY2@2560,180", name: "\\\\.\\DISPLAY2", x: 2560, y: 180, width: 1920, height: 1080, scale: 1, primary: false },
      ]),
    pageReady: async () => [],
    answerRing: (itemId, action) => {
      const it = findItem(itemId);
      const now = Date.now();
      const r = it.alarm === "deadline" ? PRESETS[(it.rhythm?.preset as "normal") ?? "normal"] ?? PRESETS.normal : PRESETS.normal;
      if (action === "done") {
        const wasOpen = it.done_at == null;
        Object.assign(it, { done_at: now, ring_state: "", check_at: null });
        if (wasOpen) spawnNext(it);
      } else if (action.startsWith("until:")) Object.assign(it, { snooze_until: Math.max(now + MIN, Number(action.slice(6))), ring_state: "snoozed", rings: 0 });
      else if (action === "onit") Object.assign(it, { on_it_at: now, check_at: now + r.checkin * MIN, ring_state: "on_it", rings: 0 });
      else if (action === "notyet") Object.assign(it, { check_at: now + r.notyet * MIN, ring_state: "on_it" });
      else if (action.startsWith("snooze:")) Object.assign(it, { snooze_until: now + Number(action.slice(7)) * MIN, ring_state: "snoozed", rings: 0 });
      it.last_ring_at = now;
      save();
      return ok(it);
    },
    tidyAnswer: () => Promise.resolve(),
    rhythmPresets: () => ok(PRESETS),
    showDump: () => Promise.resolve(),
    hideDump: () => {
      emit("dump-shown", null);
      return Promise.resolve();
    },
    dumpItem: async (text, link, file) => {
      const space: Space = data.settings.view === "personal" ? "personal" : "work";
      const inbox = inboxOf(space);
      const sort = data.items.filter((i) => i.note_id === inbox).length;
      const it = blankItem({ id: nextId("items"), note_id: inbox, text, link, file, sort });
      data.items.push(it);
      save();
      const waiting = data.items.filter((i) => i.note_id === inbox && i.done_at == null && i.parent_id == null).length;
      return [structuredClone(it), waiting];
    },
    fetchPreview: (url) => {
      const host = url.replace(/^https?:\/\//, "").split("/")[0].replace(/^www\./, "");
      return new Promise((res) => setTimeout(() => res({ url, title: `A page on ${host}`, site: host, image: samplePicture(160) }), 500));
    },
    setItemLink: (id, link) => {
      const it = { ...findItem(id), link };
      data.items = data.items.map((i) => (i.id === id ? it : i));
      save();
      emit("item-updated", structuredClone(it));
      return ok(it);
    },
    // A file on a share asks about its computer first, as the app (preview: \\nas\…); a path
    // with "missing" in it is gone.
    openPath: (path) => {
      const host = /^[\\/]{2}([^\\/]+)/.exec(path)?.[1]?.toLowerCase();
      const hosts = Array.isArray(data.settings.file_hosts) ? (data.settings.file_hosts as string[]) : [];
      if (host && !hosts.includes(host)) return Promise.reject(`ask-host:${host}`);
      if (path.includes("missing")) return Promise.reject("Couldn’t find that file: moved or deleted?");
      return Promise.resolve();
    },
    openUrl: (url) => {
      window.open(url, "_blank", "noopener");
      return Promise.resolve();
    },
    savePicture: (noteId, itemId, bytes, ext) => {
      const b64 = btoa(String.fromCharCode(...bytes));
      const p: Picture = { id: nextId("pictures"), note_id: noteId, item_id: itemId, path: `data:image/${ext};base64,${b64}`, sort: data.pictures.length, created_at: Date.now() };
      data.pictures.push(p);
      save();
      return ok(p);
    },
    addPictureFile: () => Promise.reject("Dropping files works in the app"),
    deletePicture: (id) => {
      data.pictures = data.pictures.filter((p) => p.id !== id);
      save();
      return Promise.resolve();
    },
    // (The browser preview keeps its pictures as they are.)
    makeThumb: () => Promise.resolve(null),
    releasePicture: (id, noteId) => {
      data.pictures = data.pictures.map((p) => (p.id === id ? { ...p, item_id: null, note_id: noteId } : p));
      save();
      return Promise.resolve();
    },
    setShortcut: (which, accel) => {
      data.settings[`shortcut_${which}`] = accel;
      save();
      return Promise.resolve();
    },
    shortcutsTaken: () => Promise.resolve([]),
    getAutostart: () => Promise.resolve(!!data.settings.autostart_mock),
    setAutostart: (on) => {
      data.settings.autostart_mock = on;
      save();
      return Promise.resolve();
    },
    restore: (notes, items, pictures) => {
      const put = <T extends { id: number }>(list: T[], add: T[]) => [...list.filter((x) => !add.some((a) => a.id === x.id)), ...structuredClone(add)];
      const had = inboxSpaces();
      data.notes = put(data.notes, notes);
      keepInboxes(had);
      data.items = put(data.items, items);
      data.pictures = put(data.pictures, pictures);
      save();
      return Promise.resolve();
    },
    openNote: (noteId) => {
      emit("panel-shown", null);
      setTimeout(() => emit("focus-note", noteId), 50);
      return Promise.resolve();
    },
    endPeek: () => Promise.resolve(),
    saveNoteImage: (bytes, name) => {
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
      a.download = `${name}.png`;
      a.click();
      return Promise.resolve(`Downloads/${name}.png`);
    },
    reveal: () => Promise.resolve(),
    backupNow: () => {
      const at = Date.now();
      const info: BackupInfo = { name: `dobo-${new Date(at).toISOString().slice(0, 16).replace("T", "_")}-manual`, path: `preview/${at}`, at, bytes: JSON.stringify(data).length, reason: "manual", version: null, kept_in_data: false };
      // (Each kind keeps its own last few, as the app.)
      const kept = [{ ...info, data: structuredClone(data) }, ...backups()].filter((b, i, all) => all.filter((x) => x.reason === b.reason).indexOf(b) < (b.reason === "manual" ? 10 : 14));
      localStorage.setItem(BACKUPS, JSON.stringify(kept));
      delete data.settings.backup_error;
      save();
      return ok(info);
    },
    // (?backups=sample: one or two of each kind; one made before an update in the data folder,
    // the chosen folder not answering then.)
    backupList: () =>
      ok(
        params.get("backups") === "sample"
          ? (
              [
                ["daily", 2, null, false],
                ["daily", 26, null, false],
                ["daily", 50, null, false],
                ["manual", 5, null, false],
                ["before-restore", 30, null, false],
                ["before-update", 74, "1.1.0", true],
                ["before-update", 900, "1.0.1", false],
              ] as const
            ).map(([reason, h, version, kept_in_data], i) => ({ name: `sample-${i}`, path: `preview/sample-${i}`, at: Date.now() - h * 3600_000, bytes: 2_400_000 + i * 31_000, reason, version, kept_in_data }))
          : backups().map((b) => ({ name: b.name, path: b.path, at: b.at, bytes: b.bytes, reason: b.reason, version: b.version ?? null, kept_in_data: false })),
      ),
    // (A preview shows a failed one with backup_error set in its settings.)
    backupStatus: () => {
      const error = typeof data.settings.backup_error === "string" ? data.settings.backup_error : null;
      // (?notice=elsewhere | elsewhere-full | elsewhere-writable | elsewhere-failed | failed |
      // failed-full | failed-writable: a word about the backup before an update.)
      const [what, why] = (params.get("notice") ?? "").split("-");
      const missed = why === "full" ? "full" : why === "writable" ? "not_writable" : why === "failed" ? "failed" : null;
      const sample: BackupNotice | null =
        what === "elsewhere"
          ? { code: "update_backup_elsewhere", args: { app: "Dobo", version: "1.1.0", path: `${previewDataDir()}\\backups\\dobo-2026-11-23_090000-before-update-1.1.0`, chosen: "E:\\Backups\\Dobo", why: missed ?? "no_answer" } }
          : what === "failed"
            ? { code: "update_backup_failed", args: { app: "Dobo", version: "1.1.0", error: "disk I/O error", why: missed ?? "failed" } }
            : null;
      const n = sample ?? data.settings.backup_notice;
      const notice = typeof n === "string" || (n && typeof n === "object" && "code" in n) ? (n as BackupNotice) : null;
      return ok({ error, failed_at: error ? Number(data.settings.backup_failed_at) || Date.now() : null, notice, notice_at: notice ? Date.now() : null } satisfies BackupStatus);
    },
    backupRestore: (path) => {
      const b = backups().find((x) => x.path === path);
      if (b) {
        data = structuredClone(b.data);
        save();
        setTimeout(() => emit("reload", null));
      }
      return Promise.resolve();
    },
    // (By default in the data folder, as the app's.)
    backupFolder: () => Promise.resolve(String(data.settings.backup_dir || `${previewDataDir()}\\backups`)),
    // ?copy=installed | kept | fallback | dev shows the other cases (default: a new portable copy).
    dataInfo: () => {
      const c = params.get("copy");
      const kind = c === "installed" ? "installed" : c === "dev" ? "dev" : "portable";
      return ok({ kind, dir: previewDataDir(), kept_existing: c === "kept", fallback: c === "fallback", version: "1.1.0" } satisfies DataInfo);
    },
    acceptTerms: () => {
      data.settings.terms_accepted = "2026-11-21";
      save();
      return Promise.resolve("2026-11-21");
    },
    quitApp: () => Promise.resolve(),
    // ?update=ready | error shows those cases (default: up to date after a check).
    updateStatus: () => ok(previewUpdate),
    updateCheck: () =>
      new Promise((done) => {
        previewUpdate = { ...previewUpdate, busy: true, error: null };
        emit("update-status", structuredClone(previewUpdate));
        setTimeout(() => {
          const u = params.get("update");
          previewUpdate = {
            ...previewUpdate,
            busy: false,
            checked: true,
            // (As updates.rs: only a check that worked is the last one.)
            last_checked: u === "error" ? previewUpdate.last_checked : Date.now(),
            latest: u === "ready" ? "1.0.1" : null,
            ready: u === "ready",
            error: u === "error" ? "Couldn't reach Dobo's update page. Check your connection, then try again." : null,
          };
          emit("update-status", structuredClone(previewUpdate));
          done(structuredClone(previewUpdate));
        }, 900);
      }),
    updateRestart: () => {
      location.reload();
      return Promise.resolve();
    },
    openDataFolder: () => Promise.resolve(),
    // ?recover=damaged | locked | readonly | failed | portable shows the recovery screen (…&backup=0:
    // none to put back; …&stuck: Try again finds it still held; …&broken: putting back fails;
    // damaged…&repaired: a repaired copy is ready; damaged…&aside: an earlier start put it aside,
    // …&aside&whole: and it reads whole now; portable…&notes: notes are in the folder that can't
    // be written; …&nocontinue: the board fails to come in once after a choice; …&counts=0: what
    // the repair and the backup hold couldn't be counted; …&one: one note and one line).
    recoveryState: () => ok(previewProblem),
    recoveryChoose: (choice) =>
      new Promise((done, fail) =>
        setTimeout(() => {
          if (!previewProblem) return done(null);
          if (choice === "retry" && params.has("stuck")) return done(structuredClone(previewProblem));
          if (choice === "restore" && params.has("broken")) return fail("That backup couldn't be put back (disk I/O error).");
          previewChosen = choice;
          previewProblem = null;
          done(null);
        }, choice === "retry" ? 500 : 900),
      ),
    recoveryContinue: () =>
      params.has("nocontinue") && !previewContinueFailed
        ? ((previewContinueFailed = true), Promise.reject("The notes couldn’t be opened (disk I/O error)."))
        : Promise.resolve(previewChosen === "restore" || previewChosen === "empty" || previewChosen === "repair" ? (recover === "damaged" || recover === "failed" ? previewKept : null) : null),
    pickFolder: () => Promise.resolve(window.prompt("Folder for the backups", String(data.settings.backup_dir || "D:\\Backups\\Dobo"))),
    fileSrc: (path) => path,
    languageNow: () =>
      Promise.resolve({
        choice: String(data.settings.language ?? "system"),
        system: params.get("lang") === "fr" || (!params.get("lang") && /^fr\b/i.test(navigator.language)) ? "fr" : "en",
        // ?clock=12 previews Windows set to a 12-hour clock.
        hour12: params.get("clock") === "12",
      }),
    darkModeNow: () => {
      const m = (data.settings.dark_mode ?? {}) as { mode?: string; from?: string; until?: string };
      const mode = m.mode ?? "windows";
      const from = m.from ?? "19:00";
      const until = m.until ?? "07:00";
      const d = new Date();
      const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
      const inside = from === until ? false : from < until ? hm >= from && hm < until : hm >= from || hm < until;
      return Promise.resolve({ mode, from, until, dark: mode === "light" ? false : mode === "dark" ? true : mode === "hours" ? inside : null });
    },
    exportLook: (content, name) => {
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([content], { type: "application/json" }));
      a.download = name;
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 1000);
      return Promise.resolve(name);
    },
    importLook: () =>
      new Promise((resolve) => {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = ".ztheme";
        input.onchange = async () => {
          const f = input.files?.[0];
          if (!f) return resolve(null);
          if (f.size > 512 * 1024) return resolve([f.name, "{}".padEnd(600000)]);
          resolve([f.name, await f.text()]);
        };
        input.click();
      }),
    pickBackgroundPicture: () =>
      new Promise((resolve) => {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = "image/*";
        input.onchange = () => {
          const f = input.files?.[0];
          if (!f) return resolve(null);
          const r = new FileReader();
          r.onload = () => resolve(String(r.result));
          r.readAsDataURL(f);
        };
        input.click();
      }),
    onEvent: (name, cb) => {
      (listeners[name] ||= []).push(cb);
    },
    onFocusChanged: () => {},
    onFileDrop: () => {},
  };
}

/** Told when a change to the notes couldn't be saved (the disk is full or read-only, the notes
 * file can't be written): the board says so and shows what is really saved, rather than a
 * change that will be gone at the next start. */
let saveFailed: ((error: unknown) => void) | null = null;
export function onSaveFailed(cb: (error: unknown) => void) {
  saveFailed = cb;
}

/** The calls that change the notes or the settings. */
const WRITES = [
  "createNote",
  "updateNote",
  "deleteNote",
  "reorderNotes",
  "createItem",
  "updateItem",
  "deleteItem",
  "setSetting",
  "restore",
  "setItemLink",
  "deletePicture",
  "releasePicture",
] as const satisfies readonly (keyof Backend)[];

/** Each change that fails (refused, or thrown at once) is told to `onSaveFailed`, then fails as
 * before for its caller. */
export function guardWrites(b: Backend): Backend {
  for (const name of WRITES) {
    const run = b[name] as (...args: unknown[]) => Promise<unknown>;
    const told = (e: unknown) => {
      saveFailed?.(e);
      return Promise.reject(e);
    };
    (b as unknown as Record<string, unknown>)[name] = (...args: unknown[]) => {
      try {
        return run.apply(b, args).catch(told);
      } catch (e) {
        return told(e);
      }
    };
  }
  return b;
}

export const backendPromise: Promise<Backend> = (inTauri ? tauriBackend() : Promise.resolve(mockBackend())).then(guardWrites);
