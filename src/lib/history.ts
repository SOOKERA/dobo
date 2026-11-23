// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Undo and redo (Ctrl+Z / Ctrl+Y): each change records how to take it back and how to do it
// again. A new change after an undo drops what could be redone.

export interface Step {
  /** What the change was; a function when its words are worked out as they're shown (in the
   * language of that moment), a key otherwise. */
  label: string | (() => string);
  undo: () => Promise<void> | void;
  redo: () => Promise<void> | void;
}

export class History {
  private past: Step[] = [];
  private future: Step[] = [];
  private busy = false;
  /** Undo and redo run one after another: a second Ctrl+Z pressed while the first one's card is
   * still leaving waits for it (two at once would each read the board halfway through the other). */
  private queue: Promise<unknown> = Promise.resolve();

  constructor(private limit = 100) {}

  /** Records a change (ignored while undoing or redoing, so those don't record themselves). */
  record(step: Step) {
    if (this.busy) return;
    this.past.push(step);
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
  }

  /** Forgets everything (the notes were replaced, e.g. by restoring a backup: the recorded
   * steps would write old versions over them). */
  clear() {
    this.past = [];
    this.future = [];
  }

  get canUndo() {
    return this.past.length > 0;
  }
  get canRedo() {
    return this.future.length > 0;
  }

  private inTurn<T>(run: () => Promise<T>): Promise<T> {
    const next = this.queue.then(run, run);
    this.queue = next.catch(() => {});
    return next;
  }

  /** Takes back the last change; returns its label, or null when there is nothing to undo. */
  undo(): Promise<Step["label"] | null> {
    return this.inTurn(() => this.undoNow());
  }

  redo(): Promise<Step["label"] | null> {
    return this.inTurn(() => this.redoNow());
  }

  private async undoNow(): Promise<Step["label"] | null> {
    const s = this.past.pop();
    if (!s) return null;
    this.busy = true;
    try {
      await s.undo();
    } finally {
      this.busy = false;
    }
    this.future.push(s);
    return s.label;
  }

  private async redoNow(): Promise<Step["label"] | null> {
    const s = this.future.pop();
    if (!s) return null;
    this.busy = true;
    try {
      await s.redo();
    } finally {
      this.busy = false;
    }
    this.past.push(s);
    return s.label;
  }
}

/** The fields a change made (`from` → `to`), compared by value. */
export function changedKeys<T extends object>(from: T, to: T): (keyof T)[] {
  const keys = new Set([...Object.keys(from), ...Object.keys(to)] as (keyof T)[]);
  return [...keys].filter((k) => JSON.stringify(from[k] ?? null) !== JSON.stringify(to[k] ?? null));
}

/** What an undo (or redo) writes: the thing as it is `now`, with only the fields its change made
 * taken from `want`. Whatever changed meanwhile without a step of its own (a reminder answered
 * "Done" from its notification, a link's preview arriving, a note unfolded by a ringing task)
 * stays as it is now, never written back over. */
export function withFields<T extends object>(now: T, want: T, keys: (keyof T)[]): T {
  const out = { ...now };
  for (const k of keys) out[k] = want[k];
  return out;
}
