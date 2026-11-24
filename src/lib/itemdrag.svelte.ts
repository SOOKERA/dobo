// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Carrying one line (task, idea, step, Inbox entry) to another place:
// - onto a task: it becomes one of its steps;
// - onto an idea: it becomes the idea's first step;
// - onto a note: it goes into that note (top level);
// - onto the Inbox strip: back into the Inbox (of the space it says);
// - onto the dashed "+ New note" card at the end of the board (it shows while a line is carried):
//   a new note there, titled with its words, the line inside it (lineNote.ts, store.noteFromLine);
//   onto the board's empty space: the same, where it was dropped (as the board's right-click
//   New note: it appears there, then glides up its column under the note above).
// A small paper slip follows the pointer (lift with a tilt, settles when dropped).
import { store } from "./store.svelte";
import { liveLine, liveNote } from "./keys";
import { spaceForLine } from "./lineNote";
import { reduced } from "./motion";
import { FOOT, edgeStep, slipOnLeft } from "./hoursDock";
import type { Item, Space } from "./types";

/** A slip landing on its hour (a short animation after the drop). Several can be in flight. */
export interface Landing {
  key: number;
  /** Onto an hour (the hours show while it lands), or onto a new note's card (they don't). */
  kind: "hour" | "new";
  text: string;
  idea: boolean;
  from: { x: number; y: number };
  to: { x: number; y: number };
  tilt: number;
  /** The slip's size as it was carried (the same words: the landing one is as big). */
  size?: { w: number; h: number } | null;
  /** How big it is as it ends (0.15: into an hour's dot; near 1: onto a new note's card). */
  scale?: number;
  /** Where it ends, looked at each frame (the new note, wherever the board puts it as the line
   * leaves its place; null until it is there: `to`). */
  follow?: () => { x: number; y: number } | null;
}
let landed = 0;

/** Over the hours, the slip rides higher than elsewhere and a little further right, so the hour
 * labels under the pointer and the time it would get above it stay in sight (App.svelte). A slip
 * landing or going back starts from there too. Near the hours' end it moves to the pointer's
 * left instead (`liftX`), so it never covers Tomorrow as the pointer heads there. */
export const HOURS_LIFT = { x: 12, y: 34 };

/** A slip going back to its line (the carry was called off with Esc). */
export interface Back {
  key: number;
  /** The line it goes back to. */
  id: number;
  text: string;
  idea: boolean;
  from: { x: number; y: number };
  tilt: number;
  /** Its width, when it was narrowed over the hours (`ItemDrag.cap`). */
  cap?: number | null;
  /** `from` is where the line's words start (their left edge, their middle), not the pointer:
   * the slip takes the line's place exactly (an undo lifting it off a note). */
  onLine?: boolean;
  /** How long it waits there, lifting a touch, before it glides back (ms). */
  wait?: number;
}

export interface DropTarget {
  kind: "note" | "task" | "idea" | "inbox" | "new";
  /** The note or the line (0 for the Inbox: it is found, or made, on the drop). */
  id: number;
  /** The Inbox's space. */
  space?: Space;
  /** A new note at the spot dropped on (the board's empty space), not at the card. */
  spot?: boolean;
}

class ItemDrag {
  item = $state<Item | null>(null);
  x = $state(0);
  y = $state(0);
  tilt = $state(-2);
  target = $state<DropTarget | null>(null);
  /** Over the time ruler: the time it would get (set by the ruler). */
  time = $state<number | null>(null);
  /** Where on screen that time's dot is (the slip shrinks into it when dropped). */
  timeAt = $state<{ x: number; y: number } | null>(null);
  /** Over the hours, how far right of its usual place the slip rides: HOURS_LIFT.x, or (near
   * Tomorrow) less than nothing, to the pointer's left (set by the ruler). */
  liftX = $state(HOURS_LIFT.x);
  /** Over the hours, the slip's width when the room beside the pointer is less than its words
   * need (set by the ruler; null: as wide as they need, up to 260 px). */
  cap = $state<number | null>(null);
  /** The carried slip's size, as laid out, and the width its words need (App.svelte). */
  slip: { w: number; h: number; full?: number } | null = null;
  /** Slips landing on the ruler. */
  landings = $state<Landing[]>([]);
  /** Slips going back to their lines (a carry called off with Esc). Several can be in flight. */
  backs = $state<Back[]>([]);
  /** The lines slips are going back to: each stays dimmed until its slip is on it. */
  away = $state<number[]>([]);
  /** The slip is on today's strip or the open hours, or near them (set by the ruler): no new
   * note from the board's empty space there, and the board doesn't scroll under it. */
  atDock = false;
  /** The strip's (or the open hours') top on screen while a task is carried (set by the ruler):
   * above it, the board's top edge still scrolls. */
  dockTop: number | null = null;
  /** Away from the hours, the slip rides on the pointer's left: on its right, its words would
   * cross the window's edge (a long line over the board's right column). */
  leftSide = $state(false);

  /** Where the slip's own place starts for the pointer at `x`, away from the hours (the landing
   * and going-back slips start from there). */
  private slipX(x: number): number {
    return this.leftSide ? x - (this.slip?.w ?? 0) - 28 : x;
  }

  /** "Make a new note of it" (a line's menu): what a drop on "+ New note" does, the line lifting
   * off as a slip and landing on the new note as it grows in at the end of the board. */
  async toNewNote(it: Item) {
    const words = document.querySelector(`${liveLine(it.id)} ${it.kind === "idea" ? ".idea-text" : ".text"}`)?.getBoundingClientRect();
    const made = { id: 0 };
    if (words?.width) {
      // (The slip's words where the line's are: 13 px in from its edge.)
      const h = 34;
      const w = Math.min(260, words.width + 28);
      const from = { x: words.left - 13 - 14, y: words.top + words.height / 2 - h / 2 - 10 };
      const follow = () => {
        const r = made.id ? document.querySelector(liveNote(made.id))?.getBoundingClientRect() : null;
        return r?.width ? { x: r.left + 12 + Math.min(w, r.width - 24) / 2, y: r.top + 10 + h / 2 } : null;
      };
      this.landings = [...this.landings, { key: ++landed, kind: "new", text: it.text, idea: it.kind === "idea", from, to: { x: from.x + 14 + w / 2, y: from.y + 10 + h / 2 }, tilt: 0, size: { w, h }, scale: 0.94, follow }];
    }
    const n = await store.noteFromLine(it, spaceForLine(store.view, store.noteOf(it)?.space, store.space));
    if (!n) return;
    made.id = n.id;
    requestAnimationFrame(() => requestAnimationFrame(() => bringIntoView(n.id)));
  }

  /** A line lifts off its note as a slip (its words where the line's were), waits a moment, then
   * glides back to wherever the line is by then (store.lineLifts: an undo). */
  liftBack(it: Item, at: { x: number; y: number }, wait: number) {
    this.away = [...this.away, it.id];
    this.backs = [...this.backs, { key: ++landed, id: it.id, text: it.text, idea: it.kind === "idea", from: at, tilt: 0, onLine: true, wait }];
  }

  private findTarget(x: number, y: number): DropTarget | null {
    const it = this.item;
    if (!it) return null;
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!el) return null;
    const hasSteps = (store.stepsOf.get(it.id)?.length ?? 0) > 0;
    const row = el.closest<HTMLElement>("[data-drop-item]");
    if (row && !hasSteps) {
      const id = Number(row.dataset.dropItem);
      const kind = row.dataset.dropKind as "task" | "idea";
      if (id !== it.id && id !== it.parent_id) return { kind, id };
    }
    const tray = el.closest<HTMLElement>("[data-drop-inbox]");
    if (tray) {
      const space = tray.dataset.dropInbox as Space;
      // (Already one of that Inbox's own lines: nothing to do.)
      const home = store.notes.find((n) => n.id === it.note_id);
      if (it.parent_id == null && home && store.inboxIds.has(home.id) && home.space === space) return null;
      return { kind: "inbox", id: 0, space };
    }
    const card = el.closest<HTMLElement>("[data-drop-note]");
    if (card) {
      const id = Number(card.dataset.dropNote);
      if (id !== it.note_id || it.parent_id != null) return { kind: "note", id };
      return null;
    }
    if (el.closest("[data-drop-new]")) return NEW;
    // The board's own empty space (not a note, the Inbox, the hours, a button on it), clear of
    // them by a little: a drop that just misses a note never makes a new one.
    if (el.dataset.dropEmpty != null && !this.atDock && this.clear(x, y)) return NEW_HERE;
    return null;
  }

  /** Nothing to drop on within REACH of the point (four looks around it). */
  private clear(x: number, y: number): boolean {
    for (const [dx, dy] of [[REACH, 0], [-REACH, 0], [0, REACH], [0, -REACH]]) {
      const el = document.elementFromPoint(x + dx, y + dy);
      if (el?.closest("[data-drop-note], [data-drop-inbox], [data-drop-none]")) return false;
    }
    return true;
  }

  /** A new target only when it is another one: every line and note on the board looks at it, so
   * the same target given again as a new object would have all of them looked at again. */
  private setTarget(t: DropTarget | null) {
    const c = this.target;
    if (c === t || (c && t && c.kind === t.kind && c.id === t.id && c.space === t.space && c.spot === t.spot)) return;
    this.target = t;
  }

  /** The page moved under a still pointer (the ruler opening or folding): look again at what
   * is under it. */
  recheck() {
    if (this.item && this.time == null) this.setTarget(this.findTarget(this.x, this.y));
  }

  // What is under the pointer is looked at once a frame, however many moves came in it.
  private aimAt: { x: number; y: number } | null = null;
  private aimFrame = 0;
  private aim(x: number, y: number) {
    this.aimAt = { x, y };
    if (!this.aimFrame) this.aimFrame = requestAnimationFrame(() => this.hit());
  }
  /** Looks under the pointer now (a frame's look still to come is done at once). */
  private hit() {
    if (this.aimFrame) cancelAnimationFrame(this.aimFrame);
    this.aimFrame = 0;
    const at = this.aimAt;
    this.aimAt = null;
    if (!at || !this.item) return;
    // (Over the ruler, the ruler decides: it set `time` on the same move.)
    this.setTarget(this.time != null ? null : this.findTarget(at.x, at.y));
  }
  private unaim() {
    if (this.aimFrame) cancelAnimationFrame(this.aimFrame);
    this.aimFrame = 0;
    this.aimAt = null;
  }

  /** Called on pointerdown on a line's text; the carry starts after a few pixels. */
  press(e: PointerEvent, it: Item) {
    if (e.button !== 0) return;
    const sx = e.clientX;
    const sy = e.clientY;
    let started = false;
    let lastX = sx;
    let lastT = performance.now();
    // The very first move skips the hit test (see below); a carry that ends right there (a
    // quick flick, all of it in one frame) works it out on the drop instead.
    let skipped = false;
    // Near the board's top or bottom edge, it scrolls (to a note far down, to the "+ New note"
    // card at the end), except on or near the hours, which stay put under the slip. (Only once
    // the slip has been away from both edges: a line picked up near one doesn't set it going.)
    const board = (e.target as HTMLElement | null)?.closest<HTMLElement>("[data-carry-scroll]") ?? null;
    let scrollTimer: ReturnType<typeof setInterval> | null = null;
    let rightEdge = window.innerWidth;
    let armed = false;
    const edgeScroll = () => {
      if (!board) return;
      const dy = edgeStep(this.y, board.getBoundingClientRect(), EDGE, { overHour: this.time != null, atDock: this.atDock, dockTop: this.dockTop });
      if (!dy) armed = true;
      if (!dy || !armed) return;
      const was = board.scrollTop;
      board.scrollTop = was + Math.max(-12, Math.min(12, dy));
      if (board.scrollTop !== was) this.aim(this.x, this.y);
    };
    const move = (ev: PointerEvent) => {
      let first = false;
      if (!started) {
        if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < 6) return;
        started = true;
        first = true;
        this.item = it;
        // (The panel's own right edge: past it, the slip would hang over the window's margin.)
        rightEdge = document.querySelector(".shell")?.getBoundingClientRect().right ?? window.innerWidth;
        if (board) scrollTimer = setInterval(edgeScroll, 16);
        // The grabbing hand and the cleared selection come a frame later: both make the browser
        // restyle the whole page, which would land in the same frame as the ruler opening.
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            if (this.item !== it) return;
            document.body.style.cursor = "grabbing";
            window.getSelection()?.removeAllRanges();
          }),
        );
      }
      const now = performance.now();
      const vx = (ev.clientX - lastX) / Math.max(1, now - lastT);
      lastX = ev.clientX;
      lastT = now;
      this.tilt += (-2 + Math.max(-4, Math.min(4, vx * 6)) - this.tilt) * 0.3;
      this.x = ev.clientX;
      this.y = ev.clientY;
      const left = slipOnLeft(ev.clientX, this.slip?.w ?? 0, rightEdge, this.leftSide);
      if (left !== this.leftSide) this.leftSide = left;
      // Over the ruler, the ruler decides (it sets `time`); elsewhere, notes and lines. (Not on
      // the very first move: still on its own line, and asking would force the whole page's
      // style to be worked out twice in the frame the ruler opens.)
      if (this.time != null || first) {
        this.unaim();
        this.setTarget(null);
      } else this.aim(ev.clientX, ev.clientY);
      skipped = first && this.time == null;
    };
    const stop = () => {
      if (scrollTimer) clearInterval(scrollTimer);
      scrollTimer = null;
      this.atDock = false;
      this.dockTop = null;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      document.body.style.cursor = "";
    };
    // Esc calls the carry off: nothing changes, the ruler folds and the slip goes back to its
    // line (which shows again as it lands).
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape" || !started) return;
      ev.preventDefault();
      ev.stopImmediatePropagation();
      callOff();
    };
    // The system took the pointer (a pen or touch gesture, Windows taking it mid-carry): called
    // off the same way, never dropped on whatever was last under it.
    const cancel = () => {
      if (started) callOff();
      else stop();
    };
    const callOff = () => {
      stop();
      this.unaim();
      this.away = [...this.away, it.id];
      const over = this.time != null;
      this.backs = [...this.backs, { key: ++landed, id: it.id, text: it.text, idea: it.kind === "idea", from: over ? { x: this.x + this.liftX, y: this.y - HOURS_LIFT.y } : { x: this.slipX(this.x), y: this.y }, tilt: this.tilt, cap: over ? this.cap : null }];
      this.item = null;
      this.leftSide = false;
      this.target = null;
      this.time = null;
      this.timeAt = null;
    };
    const up = async () => {
      stop();
      if (!started) return;
      this.hit();
      if (skipped && this.time == null) this.target = this.findTarget(this.x, this.y);
      const t = this.target;
      const time = this.time;
      const to = this.timeAt;
      const from = time != null ? { x: this.x + this.liftX, y: this.y - HOURS_LIFT.y } : { x: this.slipX(this.x), y: this.y };
      // (The dashed card, read before it goes: the slip lands on it, when it is in sight on the
      // board.)
      const spot = t?.kind === "new" && !t.spot ? document.querySelector("[data-drop-new]")?.getBoundingClientRect() : null;
      // (Dropped on the board's empty space: the new note goes there, the board's own point.)
      const here = t?.kind === "new" && t.spot ? (store.board?.point({ x: this.x, y: this.y }) ?? null) : null;
      const area = board?.getBoundingClientRect() ?? { top: 0, bottom: window.innerHeight };
      const seen = !!spot && spot.width > 0 && spot.top + 24 > area.top && spot.top + 24 < area.bottom;
      this.item = null;
      this.leftSide = false;
      this.target = null;
      this.time = null;
      this.timeAt = null;
      if (time != null) {
        // Dropped on an hour: the slip shrinks into its dot, the line gets the time.
        if (to) this.landings = [...this.landings, { key: ++landed, kind: "hour", text: it.text, idea: it.kind === "idea", from, to, tilt: this.tilt, size: this.slip }];
        await store.setDue(it, time);
        return;
      }
      if (!t) return;
      if (t.kind === "new") {
        // The slip settles onto the dashed card's top, where the new note's words will be, as
        // the note grows in there; and once the note is there, onto the note itself, wherever the
        // board puts it (the line leaving its place can move the notes: an unfolded Inbox
        // getting shorter, the note above it in its column).
        const made = { id: 0 };
        if (here) {
          // Onto the new note's title, where it appears under the pointer (then wherever it goes).
          const h = this.slip?.h ?? 34;
          const follow = () => {
            const r = made.id ? document.querySelector(liveNote(made.id))?.getBoundingClientRect() : null;
            return r?.width ? { x: r.left + 12 + Math.min(this.slip?.w ?? 160, r.width - 24) / 2, y: r.top + 10 + h / 2 } : null;
          };
          this.landings = [...this.landings, { key: ++landed, kind: "new", text: it.text, idea: it.kind === "idea", from, to: { x: from.x + 14 + (this.slip?.w ?? 160) / 2, y: from.y + 10 + h / 2 }, tilt: this.tilt, size: this.slip, scale: 0.94, follow }];
        } else if (spot && seen) {
          const w = Math.min(this.slip?.w ?? 160, spot.width - 24);
          const h = this.slip?.h ?? 34;
          const to = { x: spot.left + 12 + w / 2, y: spot.top + 10 + h / 2 };
          const follow = () => {
            const r = made.id ? document.querySelector(liveNote(made.id))?.getBoundingClientRect() : null;
            return r?.width ? { x: r.left + 12 + Math.min(this.slip?.w ?? w, r.width - 24) / 2, y: r.top + 10 + h / 2 } : null;
          };
          this.landings = [...this.landings, { key: ++landed, kind: "new", text: it.text, idea: it.kind === "idea", from, to, tilt: this.tilt, size: this.slip, scale: 0.94, follow }];
        }
        const space = spaceForLine(store.view, store.noteOf(it)?.space, store.space);
        const n = await store.noteFromLine(it, space, here);
        if (n) made.id = n.id;
        // (At the spot dropped on, it is in sight: the board keeps it so as it glides up.)
        if (here) return;
        // Out of sight, or partly (dropped on empty space higher up a long board, a card at the
        // window's foot): it comes into view, as little as it takes, clear of the note sizes and
        // the toast at the foot.
        if (n) requestAnimationFrame(() => requestAnimationFrame(() => bringIntoView(n.id)));
        return;
      }
      if (t.kind === "note") await store.moveItem(it, { noteId: t.id, parentId: null });
      else if (t.kind === "inbox") await store.moveItem(it, { noteId: await store.inboxId(t.space ?? store.space), parentId: null });
      else {
        const parent = store.items.find((x) => x.id === t.id);
        if (parent) await store.moveItem(it, { noteId: parent.note_id, parentId: parent.id, kind: "task" });
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
  }
}

/** A new note made from a line comes into view on the board, as little as it takes: clear of
 * the board's top, and of the note sizes (FOOT) and the toast at its foot. */
function bringIntoView(id: number) {
  const el = document.querySelector<HTMLElement>(liveNote(id));
  const board = el?.closest<HTMLElement>("[data-carry-scroll]");
  if (!el || !board) return;
  const r = el.getBoundingClientRect();
  const b = board.getBoundingClientRect();
  const toast = document.querySelector(".toast")?.getBoundingClientRect();
  const foot = Math.min(b.bottom - FOOT, toast?.height ? toast.top - 10 : Infinity);
  const top = b.top + 8;
  let dy = 0;
  if (r.top < top) dy = r.top - top;
  else if (r.bottom > foot) dy = Math.min(r.bottom - foot, r.top - top);
  if (Math.abs(dy) >= 1) board.scrollBy({ top: dy, behavior: reduced() ? "auto" : "smooth" });
}

/** A new note: on the dashed card (at the end of the board), or on the board's empty space (there). */
const NEW: DropTarget = { kind: "new", id: 0 };
const NEW_HERE: DropTarget = { kind: "new", id: 0, spot: true };
/** How far round a drop on the board's empty space must be clear of notes, the Inbox and the hours. */
const REACH = 14;
/** How near the board's top or bottom edge a carried line scrolls it. */
const EDGE = 48;

export const itemDrag = new ItemDrag();
