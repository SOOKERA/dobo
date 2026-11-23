// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Carrying one line (task, idea, step, Inbox entry) to another place:
// - onto a task: it becomes one of its steps;
// - onto an idea: it becomes the idea's first step;
// - onto a note: it goes into that note (top level);
// - onto the Inbox strip: back into the Inbox (of the space it says).
// A small paper slip follows the pointer (lift with a tilt, settles when dropped).
import { store } from "./store.svelte";
import type { Item, Space } from "./types";

/** A slip landing on its hour (a short animation after the drop). Several can be in flight. */
export interface Landing {
  key: number;
  text: string;
  idea: boolean;
  from: { x: number; y: number };
  to: { x: number; y: number };
  tilt: number;
  /** The slip's size as it was carried (the same words: the landing one is as big). */
  size?: { w: number; h: number } | null;
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
}

export interface DropTarget {
  kind: "note" | "task" | "idea" | "inbox";
  /** The note or the line (0 for the Inbox: it is found, or made, on the drop). */
  id: number;
  /** The Inbox's space. */
  space?: Space;
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
    }
    return null;
  }

  /** A new target only when it is another one: every line and note on the board looks at it, so
   * the same target given again as a new object would have all of them looked at again. */
  private setTarget(t: DropTarget | null) {
    const c = this.target;
    if (c === t || (c && t && c.kind === t.kind && c.id === t.id && c.space === t.space)) return;
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
    const move = (ev: PointerEvent) => {
      let first = false;
      if (!started) {
        if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < 6) return;
        started = true;
        first = true;
        this.item = it;
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
      this.backs = [...this.backs, { key: ++landed, id: it.id, text: it.text, idea: it.kind === "idea", from: over ? { x: this.x + this.liftX, y: this.y - HOURS_LIFT.y } : { x: this.x, y: this.y }, tilt: this.tilt, cap: over ? this.cap : null }];
      this.item = null;
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
      const from = time != null ? { x: this.x + this.liftX, y: this.y - HOURS_LIFT.y } : { x: this.x, y: this.y };
      this.item = null;
      this.target = null;
      this.time = null;
      this.timeAt = null;
      if (time != null) {
        // Dropped on an hour: the slip shrinks into its dot, the line gets the time.
        if (to) this.landings = [...this.landings, { key: ++landed, text: it.text, idea: it.kind === "idea", from, to, tilt: this.tilt, size: this.slip }];
        await store.setDue(it, time);
        return;
      }
      if (!t) return;
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

export const itemDrag = new ItemDrag();
