// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The quick tour's state: open or not, the step showing, which way it last went, and the sample
// line it shows things with ("Call the bank", as if just dumped). The sample lives only in
// memory (never saved, never in Undo, never rings): it goes into the Inbox of the space showing
// (an Inbox note is made in memory too when there is none yet), and goes away when the tour
// closes, however it closes. So does the bigger task it is dropped onto as a step ("Move house"
// and its two steps), which shows only in that step of the tour.
import { store } from "./store.svelte";
import { t } from "./i18n.svelte";
import { STEPS, type StepId } from "./tourPlan";
import { SAMPLE_ITEM, SAMPLE_NOTE, SAMPLE_PARENT, SAMPLE_STEPS, inMemory } from "./sample";
import type { Key } from "./i18n.svelte";
import type { Item, Note } from "./types";

/** The words of each line the tour holds (in the language showing). */
const WORDS: Record<number, Key> = {
  [SAMPLE_ITEM]: "Call the bank",
  [SAMPLE_PARENT]: "Move house",
  [SAMPLE_STEPS[0]]: "Book the van",
  [SAMPLE_STEPS[1]]: "Pack the boxes",
};

/** A plain task held in memory only. */
function line(id: number, noteId: number, sort: number, more: Partial<Item> = {}): Item {
  return {
    id,
    note_id: noteId,
    kind: "task",
    text: t(WORDS[id]),
    due_at: null,
    done_at: null,
    sort,
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
    ...more,
  };
}

class Tour {
  open = $state(false);
  /** The step showing (index in STEPS). */
  i = $state(0);
  /** The way the last move went: 1 forward, -1 back (the bubble's words slide that way). */
  dir = $state<1 | -1>(1);
  /** The sample line and, when the tour made one, its Inbox note. */
  sample = $state<{ item: number; note: number } | null>(null);
  /** What runs the moment the tour closes (the entry line's text back, the keyboard back), not
   * once its fade-out has ended. */
  private leaving = new Set<() => void>();

  get step(): StepId {
    return STEPS[this.i];
  }
  get last(): boolean {
    return this.i === STEPS.length - 1;
  }

  start() {
    if (this.open) return;
    this.i = 0;
    this.dir = 1;
    this.open = true;
  }
  go(i: number) {
    if (!this.open || i < 0 || i >= STEPS.length || i === this.i) return;
    this.dir = i > this.i ? 1 : -1;
    this.i = i;
  }
  next() {
    if (this.last) this.close();
    else this.go(this.i + 1);
  }
  /** Back a step; from the first one, out of the tour (like Back everywhere). */
  back() {
    if (this.i === 0) this.close();
    else this.go(this.i - 1);
  }
  /** Runs `fn` as soon as the tour closes; returns how to take it back. */
  onClose(fn: () => void): () => void {
    this.leaving.add(fn);
    return () => this.leaving.delete(fn);
  }
  close() {
    if (!this.open) return;
    this.open = false;
    const now = [...this.leaving];
    this.leaving.clear();
    for (const fn of now) fn();
    if (store.ruler?.mode === "demo") store.ruler = null;
    this.dropSample();
    if (!store.settings.tour_seen) void store.setSetting("tour_seen", Date.now());
  }

  /** The line the tour shows (in the store), if it is there. */
  get sampleItem(): Item | undefined {
    const s = this.sample;
    return s ? store.items.find((x) => x.id === s.item) : undefined;
  }

  /** Puts the sample line at the top of the Inbox (once). */
  addSample() {
    if (this.sampleItem) return;
    const space = store.space;
    let inbox = store.inboxOf(space);
    let made = 0;
    if (!inbox) {
      const c = store.startColour();
      const first = Math.min(0, ...store.notes.map((n) => n.sort));
      inbox = { id: SAMPLE_NOTE, title: "Inbox", space, palette: c.palette, color: c.color, sort: first - 1, archived: false, created_at: Date.now(), wide: false, folded: false, paper: "plain" } satisfies Note;
      store.notes = [...store.notes, inbox];
      made = SAMPLE_NOTE;
    }
    const top = store.items.filter((x) => x.note_id === inbox.id && x.parent_id == null);
    const it = line(SAMPLE_ITEM, inbox.id, Math.min(0, ...top.map((x) => x.sort)) - 1);
    store.items = [...store.items, it];
    this.sample = { item: SAMPLE_ITEM, note: made };
  }

  /** Keeps the sample where the tour shows it, whatever happens meanwhile: the notes read again
   * (a line dumped from the quick dump bar), another space showing (work hours, a reminder),
   * another language. (The bigger task comes back with it, in its step.) */
  keepSample() {
    const it = this.sampleItem;
    if (!it || !store.visibleNotes.some((n) => n.id === it.note_id)) {
      const nested = this.nested;
      this.dropSample();
      this.addSample();
      if (this.step === "step") {
        this.addParent();
        if (nested) this.nestSample(true);
      }
      return;
    }
    if (this.step === "step" && !store.items.some((x) => x.id === SAMPLE_PARENT)) this.addParent();
    // Put back as it was shown: one of its steps (with less motion, it stays one all along).
    if (this.nested && this.step === "step") this.nestSample(true);
    if (store.items.some((x) => x.id in WORDS && x.text !== t(WORDS[x.id])))
      store.items = store.items.map((x) => (x.id in WORDS && x.text !== t(WORDS[x.id]) ? { ...x, text: t(WORDS[x.id]) } : x));
  }

  /** Where the sample sat in its note before it became a step. */
  private topSort = 0;
  /** The sample is shown as the bigger task's step (kept through a re-read of the notes). */
  private nested = false;

  /** The bigger task the sample is dropped onto, right under it, with two steps of its own (one
   * done): "1/2", then "1/3" once the sample is one of them. It folds in like any new line. */
  addParent() {
    const it = this.sampleItem;
    if (!it || store.items.some((x) => x.id === SAMPLE_PARENT)) return;
    const top = it.parent_id == null ? it.sort : this.topSort;
    store.items = [
      ...store.items,
      line(SAMPLE_PARENT, it.note_id, top + 0.5),
      line(SAMPLE_STEPS[0], it.note_id, 0, { parent_id: SAMPLE_PARENT, done_at: Date.now() }),
      line(SAMPLE_STEPS[1], it.note_id, 1, { parent_id: SAMPLE_PARENT }),
    ];
  }

  /** The sample becomes the bigger task's last step (`on`), or goes back to where it was. */
  nestSample(on: boolean) {
    this.nested = on;
    const it = this.sampleItem;
    if (!it || (it.parent_id === SAMPLE_PARENT) === on) return;
    if (on && !store.items.some((x) => x.id === SAMPLE_PARENT)) return;
    if (on) this.topSort = it.sort;
    store.items = store.items.map((x) => (x.id === SAMPLE_ITEM ? { ...x, parent_id: on ? SAMPLE_PARENT : null, sort: on ? 2 : this.topSort } : x));
  }

  /** The bigger task goes (folding out), and the sample is back on its own. */
  dropParent() {
    this.nestSample(false);
    if (store.items.some((x) => x.id === SAMPLE_PARENT || x.parent_id === SAMPLE_PARENT))
      store.items = store.items.filter((x) => (x.id !== SAMPLE_PARENT && x.parent_id !== SAMPLE_PARENT) || x.id === SAMPLE_ITEM);
  }

  /** Takes everything the tour holds away again (the lines fold out like any other). */
  private dropSample() {
    if (!this.sample && !store.items.some((x) => inMemory(x.id)) && !store.notes.some((n) => inMemory(n.id))) return;
    store.items = store.items.filter((x) => !inMemory(x.id));
    store.notes = store.notes.filter((n) => !inMemory(n.id));
    if (store.focus && (inMemory(store.focus.item) || inMemory(store.focus.note))) store.focus = null;
    if (inMemory(store.target)) store.target = null;
    this.sample = null;
    this.nested = false;
  }
}

export const tour = new Tour();

/** Asks the app to start the tour (it closes what is open first). */
export const askTour = () => window.dispatchEvent(new Event("dobo-tour"));
/** Asks the app to open Settings at Help (or at Keyboard, for the keys). */
export const askHelp = (at: "Help" | "Keyboard" = "Help") => window.dispatchEvent(new CustomEvent("dobo-help", { detail: at }));
/** The "?" menu at the board's corner (F1 opens it too): the tour, the keys, the Archive, Help. */
export const helpMenu = $state({ open: false });
/** How the header shows its Style button now (it gives way as the panel narrows): with its
 * name, as its brush alone, or tucked in the ⋯ menu. The tour's Style step says it that way. */
export const styleButton = $state<{ shown: "words" | "brush" | "menu" }>({ shown: "words" });
