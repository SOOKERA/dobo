// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
export type Space = "work" | "personal";
export type SpaceView = Space | "all";
export type Kind = "task" | "idea";
export type PaletteName = "soft" | "vivid" | "custom";
export type Alarm = "reminder" | "deadline" | "off";
export type RingState = "" | "early" | "ringing" | "check" | "on_it" | "snoozed" | "passed";
export type PresetName = "gentle" | "normal" | "insistent" | "custom";

export interface Note {
  id: number;
  title: string;
  space: Space;
  palette: PaletteName;
  /** The colour's name in its palette, or the slot number ("0"–"9") for custom. */
  color: string;
  sort: number;
  archived: boolean;
  created_at: number;
  /** Covers two columns of the board. */
  wide: boolean;
  /** Folded down to its title. */
  folded: boolean;
  paper: Paper;
  /** When it went into the archive (ms; set by the backend, null when not archived or archived
   * before the archive kept the time). */
  archived_at?: number | null;
  /** Its done tasks show after the open ones (else where they are: the default). */
  done_last?: boolean;
}

export type Paper = "plain" | "ruled" | "grid" | "antique";

export type Ends = { kind: "never" } | { kind: "after"; count: number } | { kind: "on"; date: number };

export interface Repeat {
  freq: "day" | "weekday" | "week" | "month";
  /** Every N days / weeks / months. */
  interval: number;
  /** For "week": 0 = Sunday … 6 = Saturday. */
  days?: number[];
  /** For "month": the day of the month. */
  monthday?: number | null;
  ends: Ends;
  /** Occurrences already done. */
  done?: number;
  /** Its own time of day ("02:30"), set by the backend from the time it is given: every
   * occurrence is at that time, also after the clocks change. */
  at?: string;
  /** Set by the backend while the task is ticked and already at its next occurrence (it is a
   * to-do again on that day): what unticking gives back, the occurrence ticked and the count
   * before. */
  ticked?: { due: number; done: number };
}

/** One time a repeating task was done (its history; src-tauri/src/db.rs `Done`). */
export interface Done {
  id: number;
  item_id: number;
  /** The occurrence that was done (null: a repeat without a time). */
  due_at: number | null;
  done_at: number;
}

export interface Rhythm {
  /** First ring, minutes before the time (0: none). */
  start: number;
  /** More rings, minutes before. */
  pre: number[];
  /** After the time, minutes between rings. */
  every: number;
  /** Stop after this many unanswered rings. */
  max: number;
  /** After "I'm on it", minutes until "Is it really done?". */
  checkin: number;
  /** After "Not yet", minutes until asking again. */
  notyet: number;
}

export type RhythmChoice = { preset: Exclude<PresetName, "custom"> } | ({ preset: "custom" } & Rhythm);

export interface LinkPreview {
  url: string;
  title: string;
  site: string;
  /** Local file of the page's picture. */
  image: string | null;
}

export interface Item {
  id: number;
  note_id: number;
  kind: Kind;
  text: string;
  /** Milliseconds since 1970. */
  due_at: number | null;
  done_at: number | null;
  sort: number;
  created_at: number;
  /** A step under a task (or the first step of an idea). */
  parent_id: number | null;
  alarm: Alarm;
  rhythm: RhythmChoice | null;
  link: LinkPreview | null;
  /** A file on this computer. */
  file: string | null;
  repeat: Repeat | null;
  // Written by the reminder clock.
  ring_state: RingState;
  rings: number;
  last_ring_at: number | null;
  snooze_until: number | null;
  on_it_at: number | null;
  check_at: number | null;
}

export interface Picture {
  id: number;
  note_id: number;
  /** Set when it belongs to one line (dumped with it). */
  item_id: number | null;
  path: string;
  sort: number;
  created_at: number;
  /** Its board-size copy, when it has one (thumbs.svelte.ts). */
  thumb?: Thumb;
}

/** A picture's board-size copy (thumbs.rs): its file, its size in pixels. */
export interface Thumb {
  path: string;
  w: number;
  h: number;
}

export interface Everything {
  notes: Note[];
  items: Item[];
  pictures: Picture[];
  settings: Record<string, unknown>;
  /** Repeating tasks' history (older previews' saved boards have none). */
  done_log?: Done[];
}

export interface WindowInfo {
  mode: "docked" | "floating";
  chosen: "docked" | "floating";
  docked_possible: boolean;
  side: "right" | "left";
  float_on_top: boolean;
  /** The docked width on the screen the panel uses. */
  width: number;
}

export interface MonitorInfo {
  key: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}

export interface Source {
  url: string | null;
  title: string;
  app: string;
}

export interface RingEvent {
  item: Item;
  space: Space;
  note_title: string;
}

export type RingAction = "done" | "onit" | "notyet" | `snooze:${number}` | `until:${number}`;

export interface NewItem {
  noteId: number;
  kind: Kind;
  text: string;
  dueAt?: number | null;
  parentId?: number | null;
  alarm?: Alarm;
  link?: LinkPreview | null;
  file?: string | null;
}
