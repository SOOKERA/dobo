<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The board: notes in columns (masonry from their saved order until a note is dragged, then the
  // columns as the user arranged them: lib/arrange.ts). A dragged note moves one slot at a time
  // within its column (or into the column under it); only that column's notes make room, the
  // others stay put. When the width changes the
  // number of columns, notes glide one after another to their new place with a slight tilt
  // (Reflow demo). Dragging a note: lift (peel, tilt, shadow), carry (sway), the others make
  // room, press (tiny squash) — Motion demo. Only transform and opacity are animated.
  import NoteCard from "./NoteCard.svelte";
  import Holder from "./style/Holder.svelte";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import type { Item, Note } from "../lib/types";
  import { arrange, moveTo, sameSeq, type Step } from "../lib/arrange";
  import { nextNoteSlot } from "../lib/lineNote";
  import { spotPlace } from "../lib/newNote";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { untrack } from "svelte";
  import { t } from "../lib/i18n.svelte";
  import { crumbIn, crumbOut, fadeQuick } from "../lib/motion";
  import { rightClick, takeMenu } from "../lib/rightclick.svelte";
  import type { Point } from "../lib/contextmenu";

  let { notes, scroller }: { notes: Note[]; scroller?: HTMLElement } = $props();

  const GAP = 18;
  // (One empty list for every note without lines: a new one each time would redraw them all.)
  const NO_LINES: Item[] = [];
  const MIN_COL = 248;
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  let isDark = $derived(look.dark);

  let width = $state(0);
  // (Replaced whole, once per round of measuring: one placing round for all the notes that
  // changed size together, not one per note.)
  let heights = $state.raw<Record<number, number>>({});
  let order = $state<number[]>([]);
  let menuFor = $state<number | null>(null);
  /** Where its menu was right-clicked open (null: under its ⋯). */
  let menuAt = $state.raw<Point | null>(null);

  interface Drag {
    id: number;
    ox: number;
    oy: number;
    x: number;
    y: number;
    tilt: number;
  }
  let drag = $state<Drag | null>(null);
  // Changes only when a carry starts or ends (not on every pointer move).
  let dragId = $derived(drag?.id ?? null);

  // Keep the display order in step with the store, except while a note is being carried.
  $effect(() => {
    const ids = notes.map((n) => n.id);
    if (!drag) order = ids;
  });

  let byId = $derived(new Map(notes.map((n) => [n.id, n])));
  let cols = $derived(Math.max(1, Math.floor((width + GAP) / (MIN_COL + GAP))));
  let colW = $derived(width > 0 ? (width - GAP * (cols - 1)) / cols : 0);
  // Browsers report positions after the note-size zoom; the layout works before it.
  let z = $derived(store.zoom);

  // The board keeps the columns it shows once a note has been dragged (see arrange.ts); while
  // carrying, `working` holds the arrangement being tried.
  let working = $state<Step[] | null>(null);
  let savedKey = $derived(`${store.view}:${cols}`);
  let saved = $derived(store.boardSteps(savedKey));

  // Without saved columns the board is packed from the notes' order (masonry). Once a note
  // changes height (folds, gets a line), the columns as shown are held for the session, so only
  // that note's column moves: re-packing would send notes all over the board for one fold. A
  // new column count, a note made wide or a new order packs again; a new note added at the end
  // (made from a carried line, say) just goes where a new note goes, the others staying put.
  const sigOf = (ids: number[]) => `${savedKey}|${ids.join(",")}|${ids.map((id) => (byId.get(id)?.wide ? id : "")).join(",")}`;
  let holdSig = $derived(sigOf(order));
  let hold = $state<{ sig: string; n: number; seq: Step[] } | null>(null);
  let held = $derived.by(() => {
    if (!hold) return null;
    if (hold.sig === holdSig) return hold.seq;
    const n = hold.n;
    if (order.length > n && order.slice(n).every((id) => byId.has(id) && !byId.get(id)?.wide) && sigOf(order.slice(0, n)) === hold.sig) return hold.seq;
    return null;
  });

  let placing = $derived({
    ids: order.filter((id) => byId.has(id)),
    wide: (id: number) => !!byId.get(id)?.wide,
    height: (id: number) => heights[id] ?? 140,
    cols,
    colW,
    gap: GAP,
    saved: working ?? saved ?? held,
  });
  let layout = $derived.by(() => {
    const a = arrange(placing);
    return { pos: a.pos, seq: a.seq, height: a.height, cols, width };
  });

  // ---- "+ New note", while a line is carried ----
  // A dashed card waits where a new note would go (lib/lineNote.ts): a line dropped on it (or on
  // the board's empty space) becomes a new note there, which grows in as the card fades.
  const NEW_H = 92;
  /** Room kept under it, so the board can scroll it clear of the note sizes at the bottom. */
  const NEW_ROOM = 64;
  let newSpot = $derived(itemDrag.item && width > 0 && !drag ? nextNoteSlot(placing, NEW_H) : null);
  // (Lit only for a drop on it: on the board's empty space, the note goes where it is dropped.)
  let newHere = $derived(itemDrag.target?.kind === "new" && !itemDrag.target.spot);
  // Where the card was as the line was dropped on it (or on empty space): the new note grows in
  // from there to its place, which can be a little higher (the line left a note above it).
  let lastSpot: { x: number; y: number } | null = null;
  let bornAt: { x: number; y: number; at: number } | null = null;
  $effect(() => {
    if (newSpot) lastSpot = newHere ? { x: newSpot.x, y: newSpot.y } : null;
    else if (lastSpot) {
      bornAt = { ...lastSpot, at: performance.now() };
      lastSpot = null;
    }
  });

  // ---- "New note" from the board's right-click menu ----
  // The new note goes in the column clicked, under the note above the spot (lib/newNote.ts):
  // it grows in where it was clicked, then glides up its column to its place as the notes below
  // make room.
  /** How far above the pointer the new card's top grows in (the pointer on its title). */
  const BORN_LIFT = 18;
  let spotBorn: { id: number; x: number; y: number; at: number } | null = null;
  $effect(() => {
    store.board = {
      point: (p) => {
        if (!boardEl || width <= 0) return null;
        const b = boardEl.getBoundingClientRect();
        return { x: Math.max(0, (p.x - b.left) / z), y: Math.max(0, (p.y - b.top) / z) };
      },
      around: (at) => {
        const s = spotPlace(layout.seq, layout.pos, -1, at, cols, colW, GAP);
        return { above: s.above, below: s.below };
      },
      place: (id, at) => {
        if (width <= 0) return;
        const s = spotPlace(layout.seq, layout.pos, id, at, cols, colW, GAP);
        spotBorn = { id, x: s.clicked * (colW + GAP), y: Math.max(0, at.y - BORN_LIFT), at: performance.now() };
        return store.saveBoardSteps(store.view, cols, s.seq);
      },
    };
    return () => (store.board = null);
  });
  /** A new note growing in: left to finish while its place stays the same (its own first
   * measuring, or the notes below making room, would otherwise cut it short with a plain glide). */
  let birth: { id: number; x: number; y: number; anim: Animation } | null = null;

  // A note made wide or normal: the others move with the graceful reflow, not a plain glide.
  // (Only a note that was there and changed: one coming or going has its own way in and out, and
  // the others make room for it with the plain glide.)
  let wideOf = $derived(new Map(notes.map((n) => [n.id, n.wide])));
  let prevWide: Map<number, boolean> | null = null;
  $effect.pre(() => {
    const was = prevWide;
    if (was && [...wideOf].some(([id, on]) => was.has(id) && was.get(id) !== on)) reflowUntil = performance.now() + 500;
    prevWide = wideOf;
  });

  // ---- Widening a note by its edge grip ----
  let widen = $state<{ id: number; w: number; want: boolean } | null>(null);
  // A note carried or widened: no right-click menu opens meanwhile (App).
  let carrying = $derived(dragId != null || widen != null);
  $effect(() => {
    rightClick.carrying = carrying;
  });
  // A note's menu asked for by a right-click, Shift+F10 or the Menu key (App): it opens, at the
  // pointer or under its ⋯.
  $effect(() => {
    if (rightClick.ask?.kind !== "note") return;
    const id = rightClick.ask.id;
    untrack(() => {
      if (!byId.has(id) || carrying) return;
      const got = takeMenu("note", id);
      if (!got) return;
      menuAt = got.at;
      menuFor = id;
    });
  });
  function grip(e: PointerEvent, id: number) {
    if (e.button !== 0 || cols < 2) return;
    e.preventDefault();
    e.stopPropagation();
    const note = byId.get(id);
    const p = layout.pos.get(id);
    if (!note || !p) return;
    const sx = e.clientX;
    const base = p.w;
    const move = (ev: PointerEvent) => {
      const dx = (ev.clientX - sx) / z;
      const w = Math.max(colW * 0.8, Math.min(colW * 2 + GAP, base + dx));
      const want = note.wide ? w > colW * 1.6 : w > colW * 1.35;
      widen = { id, w, want };
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      document.body.style.cursor = "";
    };
    // The system took the pointer: the width stays as it was, as with Esc.
    const cancel = () => {
      stop();
      widen = null;
    };
    const up = () => {
      stop();
      const w = widen;
      widen = null;
      if (w && w.want !== note.wide) store.setWide(note, w.want);
    };
    // Esc: the width stays as it was (the ghost fades).
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      ev.stopImmediatePropagation();
      stop();
      widen = null;
    };
    document.body.style.cursor = "ew-resize";
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
  }

  // ---- Measuring each note ----
  // A note whose content eases its height (folding, a line folding in or out) changes size every
  // frame; placing the whole board again each frame (columns re-packed, every note touched) made
  // big boards stutter. Instead the note is taken at the height it is going to (its running
  // animations looked at at their end, once), so the others glide there once, in step with it.
  type Easing = { anims: Set<Animation>; h: number };
  const easing = new Map<number, Easing>();
  // The next placing round glides in step with an easing note: its length and how far it is.
  let inStep: { total: number; at: number; start: number | null } | null = null;
  // The app's fold ease (cubicOut, as the notes' own transitions use it), for WAAPI.
  const FOLD_EASE = `linear(${Array.from({ length: 25 }, (_, i) => (1 - Math.pow(1 - i / 24, 3)).toFixed(4)).join(", ")})`;
  /** The running animations inside each measured note, from one look at the page's animations
   * (not one look per note: each look makes the browser bring its styles up to date). */
  function runningIn(): Map<Element, Animation[]> {
    const by = new Map<Element, Animation[]>();
    for (const a of document.getAnimations()) {
      if (a.playState !== "running") continue;
      const end = a.effect?.getComputedTiming().endTime;
      if (typeof end !== "number" || !Number.isFinite(end)) continue;
      const target = (a.effect as KeyframeEffect | null)?.target;
      const box = target?.closest("[data-measure]");
      if (!box) continue;
      const list = by.get(box);
      if (list) list.push(a);
      else by.set(box, [a]);
    }
    return by;
  }
  /** The height once the running animations are over, and the longest one's length and time. */
  function settled(node: HTMLElement, anims: Animation[]): { h: number; total: number; at: number; start: number | null } {
    const was = anims.map((a) => a.currentTime);
    let total = 0;
    let at = 0;
    let start: number | null = null;
    for (const a of anims) {
      const end = Number(a.effect!.getComputedTiming().endTime);
      if (end > total) {
        total = end;
        at = Number(a.currentTime ?? 0);
        start = a.startTime == null || a.playbackRate !== 1 ? null : Number(a.startTime);
      }
      a.currentTime = Math.max(0, end - 0.01);
    }
    const h = node.offsetHeight;
    anims.forEach((a, i) => (a.currentTime = was[i]));
    return { h, total, at, start };
  }
  /** A note's height is about to change: keep the columns as shown (see `hold`). */
  function holdColumns() {
    // (Not while the fonts are still coming in: those first heights aren't the notes' own, and
    // the packing they'd give would be held for the whole session.)
    if (saved || working || held || document.fonts?.status === "loading" || !order.every((i) => heights[i] != null)) return;
    hold = { sig: holdSig, n: order.length, seq: layout.seq };
  }
  // One observer for all the notes: the notes that change size in the same frame (a new width,
  // the note size, the language, the fonts coming in) are taken together, in one round.
  const measuredId = new WeakMap<Element, number>();
  const sizes = new ResizeObserver((entries) => {
    let next: Record<number, number> | null = null;
    const now = () => next ?? heights;
    const set = (id: number, h: number) => {
      next ??= { ...heights };
      next[id] = h;
    };
    // (Only notes measured once before can be easing: the first measure just takes the size.)
    let anims: Map<Element, Animation[]> | null = null;
    for (const entry of entries) {
      const node = entry.target as HTMLElement;
      const id = measuredId.get(node);
      if (id == null || !node.isConnected) continue;
      if (now()[id] != null && !reduce.matches) anims ??= runningIn();
      const own = now()[id] == null || reduce.matches ? undefined : anims?.get(node);
      if (own?.length) {
        const e = easing.get(id);
        // Still the same easing: its end is already known.
        if (e && own.every((a) => e.anims.has(a))) continue;
        const { h, total, at, start } = settled(node, own);
        easing.set(id, { anims: new Set(own), h });
        if (now()[id] !== h) {
          if (!inStep || total - at > inStep.total - inStep.at) inStep = { total, at, start };
          holdColumns();
          set(id, h);
        }
        continue;
      }
      easing.delete(id);
      const h = node.offsetHeight;
      if (now()[id] === h) continue;
      if (now()[id] != null) holdColumns();
      set(id, h);
    }
    if (next) heights = next;
  });
  function measure(node: HTMLElement, id: number) {
    measuredId.set(node, id);
    sizes.observe(node);
    return { destroy: () => (sizes.unobserve(node), measuredId.delete(node), easing.delete(id)) };
  }
  $effect(() => () => sizes.disconnect());

  /** A new note gliding up its column to its place (`y`, on the board) over `ms`: when that place
   * is above the board's part in sight (a short column, the board scrolled down), the board
   * scrolls with it, so its title, which has the keyboard, stays in sight. */
  function keepInSight(id: number, y: number, ms: number) {
    if (!scroller || !boardEl) return;
    const s = scroller.getBoundingClientRect();
    const top = boardEl.getBoundingClientRect().top + y * z;
    const bottom = top + (heights[id] ?? NEW_H) * z;
    const room = 12;
    let dy = 0;
    if (top < s.top + room) dy = top - (s.top + room);
    else if (bottom > s.bottom - room && top > s.top + room) dy = Math.min(bottom - (s.bottom - room), top - (s.top + room));
    if (Math.abs(dy) < 1) return;
    const from = scroller.scrollTop;
    const el = scroller;
    const t0 = performance.now();
    const step = (now: number) => {
      const p = Math.min(1, (now - t0) / ms);
      el.scrollTop = from + dy * (p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2);
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  // ---- FLIP: animate from where each note is seen now to its new place ----
  const slots = new Map<number, HTMLElement>();
  function slot(node: HTMLElement, id: number) {
    slots.set(id, node);
    // A note just added waits unseen until it settles in (the next placing round, at once).
    if (comingIn.has(id) && !reduce.matches) {
      node.style.opacity = "0";
      // (Shown anyway after a while, should no placing round come; never one put back from the
      // archive, waiting unseen under the Archive page until the board shows again.)
      setTimeout(() => {
        if (node.style.opacity === "0" && !(store.over && store.arriving.includes(id))) node.style.opacity = "";
      }, 600);
    }
    return { destroy: () => (slots.delete(id), slotAnims.delete(id)) };
  }
  // The animations this board started on each note's slot (the only ones a slot ever has), kept
  // here so stopping them needs no look at the page: asking an element for its animations right
  // after another one was started makes the browser restyle the page, once per note.
  const slotAnims = new Map<number, Set<Animation>>();
  function play(id: number, el: HTMLElement, frames: Keyframe[], opts: KeyframeAnimationOptions): Animation {
    const a = el.animate(frames, opts);
    let set = slotAnims.get(id);
    if (!set) slotAnims.set(id, (set = new Set()));
    set.add(a);
    const done = () => slotAnims.get(id)?.delete(a);
    // (One that holds its end, a note faded away, stays known until it is let go.)
    if (opts.fill !== "forwards" && opts.fill !== "both") a.addEventListener("finish", done);
    a.addEventListener("cancel", done);
    return a;
  }
  /** Stops a slot's animations (`moves` only: not a note fading away where it is). */
  function stopSlot(id: number, moves = false) {
    const set = slotAnims.get(id);
    if (!set?.size) return;
    for (const a of [...set]) if (!moves || a.effect?.getTiming().fill !== "forwards") a.cancel();
  }
  // The board's own height easing down with a folding note.
  let boardAnim: Animation | null = null;

  let boardEl: HTMLDivElement;
  let seen = new Map<number, { x: number; y: number }>();
  let prevCols = 0;
  let prevWidth = 0;
  let prevHeight = 0;
  let first = true;
  /** The notes on the board at the last placing round (a note new since then is one just added). */
  let prevIds = new Set<number>();
  /** Notes just added, to settle in once their card is on the page. */
  const comingIn = new Set<number>();
  let justDropped: number | null = null;
  // After the columns change, the notes are measured again at their new width (same frame):
  // that second pass must keep the reflow style instead of replacing it with a plain glide.
  let reflowUntil = 0;
  // The width changing continuously (the panel's edge dragged, the window resized): the notes
  // take their places as it goes, and the reflow glide is kept for a change of width on its own.
  const LIVE_GAP = 150;
  let widthAt = -Infinity;
  let live = false;

  $effect.pre(() => {
    void layout;
    void dragId;
    if (!boardEl) return;
    const b = boardEl.getBoundingClientRect();
    const snap = new Map<number, { x: number; y: number }>();
    for (const [id, el] of slots) {
      const r = el.getBoundingClientRect();
      snap.set(id, { x: (r.left - b.left) / z, y: (r.top - b.top) / z });
    }
    seen = snap;
  });

  $effect(() => {
    const l = layout;
    const carried = dragId;
    if (!boardEl || l.width <= 0) return;
    const widthChanged = prevWidth !== 0 && Math.abs(l.width - prevWidth) > 0.5;
    const t = performance.now();
    if (widthChanged) {
      live = t - widthAt < LIVE_GAP;
      widthAt = t;
    } else if (t - widthAt >= LIVE_GAP) live = false;
    const colsChanged = !live && prevCols !== 0 && l.cols !== prevCols;
    const onlyWidth = !colsChanged && widthChanged;
    // Until every note has been measured once, notes just take their place (no gliding). One
    // note just added to a board measured whole (made, put back) isn't that: it settles in, and
    // the others make room for it, before it is measured too.
    const unmeasured = order.filter((id) => heights[id] == null);
    const measured = !unmeasured.length;
    if (measured) store.markBoardSettled();
    const added = !first && unmeasured.length === 1 && !prevIds.has(unmeasured[0]);
    // (Its card may only be on the page by the next round: it settles in then.)
    for (const id of comingIn) if (!order.includes(id)) comingIn.delete(id);
    if (added && !reduce.matches) comingIn.add(unmeasured[0]);
    const wasFirst = (first || !measured) && !added;
    prevCols = l.cols;
    prevWidth = l.width;
    prevIds = new Set(order);
    first = !measured && !added;
    if (colsChanged) reflowUntil = t + 160;
    const reflow = !live && (colsChanged || t < reflowUntil);
    // In step with a note easing its height (its own ease-out, for the time it has left).
    const step = inStep && inStep.total - inStep.at > 40 ? inStep : null;
    inStep = null;
    const wasHeight = prevHeight;
    prevHeight = l.height;
    if (reduce.matches) return;

    // The board taken straight to its new, shorter height would pull the page up at once under a
    // window scrolled to the end (the note is still folding): it eases down with the fold.
    // (A new placing round takes over from one still easing.)
    if (Math.abs(l.height - wasHeight) > 0.5 && boardAnim) {
      boardAnim.cancel();
      boardAnim = null;
    }
    if (step && !wasFirst && l.height < wasHeight - 0.5) {
      const a = boardEl.animate([{ height: `${Math.max(wasHeight, 0)}px` }, { height: `${Math.max(l.height, 0)}px` }], { duration: step.total, easing: FOLD_EASE });
      if (step.start != null) a.startTime = step.start;
      else a.currentTime = step.at;
      boardAnim = a;
      a.addEventListener("finish", () => boardAnim === a && (boardAnim = null));
    }

    let k = 0;
    for (const [id, el] of slots) {
      if (id === carried) continue;
      const to = l.pos.get(id);
      if (!to) continue;
      const coming = comingIn.delete(id);
      const from = coming ? undefined : seen.get(id);
      if (!from) {
        // A note put back from the archive while the Archive page covers the board: it waits,
        // unseen, and settles in once the board shows again (below).
        if (untrack(() => store.over && store.arriving.includes(id))) {
          el.style.opacity = "0";
          continue;
        }
        // A new note: it settles in (made from a carried line: from where the card was; from the
        // right-click menu: it grows in where it was clicked, then glides up to its place).
        el.style.opacity = "";
        if (!wasFirst) {
          const born = bornAt && t - bornAt.at < 1500 ? bornAt : null;
          bornAt = null;
          const spot = spotBorn?.id === id && t - spotBorn.at < 1500 ? spotBorn : null;
          if (spotBorn?.id === id) spotBorn = null;
          // Its fading in is an animation of its own, which a new place for it (the notes above
          // making room, its own first measuring) never cuts short: only the move is retargeted.
          let a: Animation;
          let fade: number;
          if (spot) {
            // (It only ever glides up: under the note above, never left lower than the spot; and
            // across to the first empty column, when the one clicked was empty with one on its left.)
            const by = Math.max(spot.y, to.y);
            const rise = Math.hypot(by - to.y, spot.x - to.x);
            const grow = 210;
            const glide = rise < 4 ? 0 : Math.round(Math.min(480, 260 + rise * 0.45));
            const total = grow + glide;
            const at = `translate(${spot.x}px, ${by}px)`;
            fade = glide ? grow : 260;
            a = play(
              id,
              el,
              glide
                ? [
                    { transform: `${at} scale(0.94)`, easing: "cubic-bezier(.2,.8,.2,1)" },
                    { transform: `${at} scale(1)`, offset: grow / total, easing: "cubic-bezier(.45,0,.2,1)" },
                    { transform: `translate(${to.x}px, ${to.y}px) scale(1)` },
                  ]
                : [{ transform: `${at} scale(0.94)` }, { transform: `translate(${to.x}px, ${to.y}px) scale(1)` }],
              { duration: glide ? total : 260, easing: glide ? "linear" : "cubic-bezier(.2,.8,.2,1)" },
            );
            if (glide) keepInSight(id, to.y, total);
          } else {
            fade = 320;
            a = play(
              id,
              el,
              [{ transform: born ? `translate(${born.x}px, ${born.y}px) scale(0.97)` : `translate(${to.x}px, ${to.y + 10}px) scale(0.97)` }, { transform: `translate(${to.x}px, ${to.y}px)` }],
              { duration: 320, easing: "cubic-bezier(.2,.8,.2,1)" },
            );
          }
          el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: fade, easing: "cubic-bezier(.2,.8,.2,1)" });
          birth = { id, x: to.x, y: to.y, anim: a };
        }
        continue;
      }
      // (Still growing in, and its place the same: it carries on.)
      if (birth?.id === id && birth.anim.playState === "running" && Math.abs(birth.x - to.x) < 0.5 && Math.abs(birth.y - to.y) < 0.5) continue;
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      if (wasFirst) continue;
      // (While the width is dragged, a glide from a step before would end on a place that is no
      // longer the note's: the note just takes its place, as it does at every step of the drag.)
      if (live) {
        stopSlot(id, true);
        continue;
      }
      if (onlyWidth && !reflow) continue;
      stopSlot(id);
      const fromT = `translate(${from.x}px, ${from.y}px)`;
      const toT = `translate(${to.x}px, ${to.y}px)`;
      if (reflow) {
        const tilt = (dx >= 0 ? 1 : -1) * 1.5;
        play(
          id,
          el,
          [
            { transform: `${fromT} rotate(0deg)` },
            { transform: `translate(${(from.x + to.x) / 2}px, ${(from.y + to.y) / 2 - 8}px) rotate(${tilt}deg) scale(1.02)`, offset: 0.5 },
            { transform: `${toT} rotate(0deg)` },
          ],
          { duration: 620, delay: Math.min(k, 8) * 55, easing: "cubic-bezier(.3,.8,.25,1)", fill: "backwards" },
        );
        k++;
      } else {
        const dropped = id === justDropped;
        if (step && !dropped) {
          // On the easing note's own clock (same start), so both move as one, frame by frame.
          const a = play(id, el, [{ transform: fromT }, { transform: toT }], { duration: step.total, easing: FOLD_EASE });
          if (step.start != null) a.startTime = step.start;
          else a.currentTime = step.at;
          continue;
        }
        play(id, el, [{ transform: fromT }, { transform: toT }], {
          duration: dropped ? 260 : 300,
          easing: dropped ? "cubic-bezier(.2,.9,.3,1)" : "cubic-bezier(.3,.8,.25,1)",
        });
      }
    }
    justDropped = null;
  });

  // ---- Notes leaving and coming back ----
  // A note going away (archived, deleted): its card lifts a touch and fades where it is, then
  // the others glide into its room (the placing above). The store waits for it.
  $effect(() => {
    store.noteLeaves = async (ids) => {
      // (Under a view, as from the Archive page, nothing is seen: no waiting.)
      if (store.over) return;
      const anims: Animation[] = [];
      for (const id of ids) {
        const el = slots.get(id);
        const p = layout.pos.get(id);
        if (!el || !p) continue;
        stopSlot(id);
        const at = `translate(${p.x}px, ${p.y}px)`;
        const a = reduce.matches
          ? play(id, el, [{ opacity: 1 }, { opacity: 0 }], { duration: 120, fill: "forwards" })
          : play(
              id,
              el,
              [
                { opacity: 1, transform: `${at} scale(1)` },
                { opacity: 0, transform: `translate(${p.x}px, ${p.y - 12}px) scale(0.95)` },
              ],
              { duration: 240, easing: "cubic-bezier(0.4, 0, 0.7, 1)", fill: "forwards" },
            );
        anims.push(a);
        // If the note stays after all (the change failed), it shows again.
        setTimeout(() => el.isConnected && a.cancel(), 2000);
      }
      await Promise.all(anims.map((a) => a.finished.catch(() => {})));
    };
    return () => (store.noteLeaves = null);
  });

  // Notes put back from the archive: once the board shows again, their cards settle in at
  // their place, one after another (the first one scrolled into view when it's out of sight).
  $effect(() => {
    const ids = store.arriving;
    if (store.over || !ids.length) return;
    return untrack(() => {
      const timer = setTimeout(() => {
        let k = 0;
        let first: HTMLElement | null = null;
        for (const id of ids) {
          const el = slots.get(id);
          const to = layout.pos.get(id);
          if (!el || !to) continue;
          el.style.opacity = "";
          first ??= el;
          if (reduce.matches) continue;
          play(
            id,
            el,
            [
              { opacity: 0, transform: `translate(${to.x}px, ${to.y + 14}px) scale(0.96) rotate(-1.2deg)` },
              { opacity: 1, transform: `translate(${to.x}px, ${to.y}px) scale(1) rotate(0deg)` },
            ],
            { duration: 420, delay: k * 70, easing: "cubic-bezier(.2,.8,.2,1)", fill: "backwards" },
          );
          k++;
        }
        if (first && scroller) {
          const r = first.getBoundingClientRect();
          const s = scroller.getBoundingClientRect();
          // (Seen whole when it fits: its top near the top of the board.)
          if (r.top < s.top || r.top > s.top + s.height * 0.55)
            scroller.scrollBy({ top: r.top - s.top - 24, behavior: reduce.matches ? "auto" : "smooth" });
        }
        store.arriving = [];
      }, 140);
      return () => clearTimeout(timer);
    });
  });

  // ---- Drag ----
  function grab(e: PointerEvent, id: number) {
    const sx = e.clientX;
    const sy = e.clientY;
    let started = false;
    let lastX = sx;
    let lastT = performance.now();
    let lockUntil = 0;
    let lastClientY = sy;
    let scrollTimer: ReturnType<typeof setInterval> | null = null;

    const boardPoint = (cx: number, cy: number) => {
      const b = boardEl.getBoundingClientRect();
      return { x: (cx - b.left) / z, y: (cy - b.top) / z };
    };

    // Where the carried card would go: the column under its middle, between the notes of that
    // column above and below its middle. Only that column (and the one it left) move.
    let startSeq: Step[] = [];
    let startCols = 0;
    const retarget = () => {
      if (!drag || performance.now() < lockUntil) return;
      const box = layout.pos.get(id);
      if (!box) return;
      const wide = cols >= 2 && box.w > colW * 1.5;
      const step = colW + GAP;
      const c = wide
        ? Math.max(0, Math.min(cols - 2, Math.round(drag.x / step)))
        : Math.max(0, Math.min(cols - 1, Math.floor((drag.x + colW / 2 + GAP / 2) / step)));
      const next = moveTo(layout.seq, layout.pos, id, c, drag.y + box.h / 2, colW, GAP, wide);
      if (!sameSeq(next, layout.seq)) {
        working = next;
        lockUntil = performance.now() + 90;
      }
    };

    const move = (ev: PointerEvent) => {
      if (!started) {
        if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < 5) return;
        const pos = layout.pos.get(id);
        if (!pos) return;
        started = true;
        menuFor = null;
        stopSlot(id);
        const p = boardPoint(sx, sy);
        drag = { id, ox: p.x - pos.x, oy: p.y - pos.y, x: pos.x, y: pos.y, tilt: -2.5 };
        startSeq = layout.seq;
        startCols = cols;
        working = layout.seq;
        document.body.style.cursor = "grabbing";
        scrollTimer = setInterval(() => {
          if (!scroller) return;
          const r = scroller.getBoundingClientRect();
          const edge = 56;
          let dy = 0;
          if (lastClientY < r.top + edge) dy = -Math.ceil((r.top + edge - lastClientY) / 4);
          else if (lastClientY > r.bottom - edge) dy = Math.ceil((lastClientY - (r.bottom - edge)) / 4);
          // Never past the board's last note: the carried card itself makes the page taller,
          // which would otherwise keep the scrolling going for ever.
          if (dy > 0) {
            const b = boardEl.getBoundingClientRect();
            const end = b.top - r.top + scroller.scrollTop + (layout.height + 80) * z - scroller.clientHeight;
            dy = Math.max(0, Math.min(dy, Math.ceil(end - scroller.scrollTop)));
          }
          if (dy) {
            scroller.scrollTop += dy;
            if (drag) {
              const p = boardPoint(lastX, lastClientY);
              drag = { ...drag, x: p.x - drag.ox, y: p.y - drag.oy };
            }
          }
          // Catches up after the short lock even when the pointer rests.
          retarget();
        }, 16);
      }
      if (!drag) return;
      const now = performance.now();
      const dt = Math.max(1, now - lastT);
      const vx = (ev.clientX - lastX) / dt;
      lastX = ev.clientX;
      lastT = now;
      lastClientY = ev.clientY;
      const target = -2.5 + Math.max(-3.5, Math.min(3.5, vx * 6));
      const p = boardPoint(ev.clientX, ev.clientY);
      drag = { ...drag, x: p.x - drag.ox, y: p.y - drag.oy, tilt: drag.tilt + (target - drag.tilt) * 0.3 };
      retarget();
    };

    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      if (scrollTimer) clearInterval(scrollTimer);
      document.body.style.cursor = "";
    };
    const up = () => {
      stop();
      if (!started || !drag) return;
      const tilt = drag.tilt;
      const card = slots.get(id)?.querySelector(".card") as HTMLElement | null;
      justDropped = id;
      drag = null;
      if (card && !reduce.matches) {
        card.animate(
          [
            { transform: `rotate(${tilt}deg) scale(1.04)` },
            { transform: "rotate(0.6deg) scale(0.985)", offset: 0.6 },
            { transform: "rotate(0deg) scale(1)" },
          ],
          { duration: 380, easing: "cubic-bezier(.3,.7,.3,1)" },
        );
      }
      // What the board shows now, at the column count it has now (the window may have changed
      // it during the carry: the steps tried were for the old count, clamped since).
      const done = working ? layout.seq : null;
      if (done && (cols !== startCols || !sameSeq(done, startSeq))) void store.saveBoardSteps(store.view, cols, done);
      // The saved setting is in place (same tick), so the board doesn't flick back.
      working = null;
    };
    // Esc calls the carry off: the notes that made room go back, and the card glides home and
    // settles flat (no press: it isn't put down anywhere new).
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape" || !started || !drag) return;
      ev.preventDefault();
      ev.stopImmediatePropagation();
      callOff();
    };
    // The system took the pointer (a pen or touch gesture, Windows taking it mid-carry): called
    // off the same way, never put down wherever it last was.
    const cancel = () => {
      if (started && drag) callOff();
      else stop();
    };
    const callOff = () => {
      stop();
      if (!drag) return;
      const tilt = drag.tilt;
      const card = slots.get(id)?.querySelector(".card") as HTMLElement | null;
      drag = null;
      working = null;
      if (card && !reduce.matches)
        card.animate([{ transform: `rotate(${tilt}deg) scale(1.04)` }, { transform: "rotate(0deg) scale(1)" }], { duration: 300, easing: "cubic-bezier(.3,.8,.25,1)" });
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
  }
</script>

<div class="board" bind:this={boardEl} bind:clientWidth={width} style:height="{Math.max(layout.height, 0)}px" data-drop-empty>
  {#if newSpot}
    {@const s = newSpot}
    <div class="new-slot" style:width="{s.w}px" style:height="{s.h + NEW_ROOM}px" style:transform="translate({s.x}px, {s.y}px)" out:fadeQuick={{ duration: 200 }} aria-hidden="true">
      <div class="new-card" class:on={newHere} style:height="{s.h}px" data-drop-new>
        <Icon name="plus" size={15} stroke={1.9} />
        <span class="new-words">{t("New note")}</span>
      </div>
    </div>
  {/if}
  {#if drag}
    {@const ph = layout.pos.get(drag.id)}
    {#if ph}
      <div class="placeholder" out:fadeQuick|global={{ duration: 180 }} style:width="{ph.w}px" style:height="{ph.h}px" style:transform="translate({ph.x}px, {ph.y}px)"></div>
    {/if}
  {/if}
  {#if widen}
    {@const wp = layout.pos.get(widen.id)}
    {#if wp}
      <div
        class="widen"
        transition:fadeQuick|global={{ duration: 160 }}
        class:want={widen.want}
        style:width="{widen.want ? colW * 2 + GAP : colW}px"
        style:height="{wp.h}px"
        style:transform="translate({wp.x}px, {wp.y}px)"
        aria-hidden="true"
      >
        <span class="wswap">{#key widen.want}<span in:crumbIn out:crumbOut>{widen.want ? t("Wide note") : t("Normal width")}</span>{/key}</span>
      </div>
    {/if}
  {/if}
  {#each order as id, index (id)}
    {@const note = byId.get(id)}
    {@const p = layout.pos.get(id)}
    {#if note && p}
      {@const dragging = drag?.id === id}
      <div
        class="slot"
        class:dragging
        class:menu-open={menuFor === id}
        use:slot={id}
        style:width="{p.w}px"
        style:transform={dragging && drag ? `translate(${drag.x}px, ${drag.y}px)` : `translate(${p.x}px, ${p.y}px)`}
      >
        <Holder look={look.active} {id} {index} />
        <div data-measure use:measure={id}>
          <NoteCard
            {note}
            items={store.itemsByNote.get(id) ?? NO_LINES}
            tone={isDark ? swatchOf(note, store.custom).dark : swatchOf(note, store.custom).light}
            menuOpen={menuFor === id}
            menuAt={menuFor === id ? menuAt : null}
            lifted={dragging}
            tilt={dragging && drag ? drag.tilt : -2.5}
            onMenu={(open) => ((menuAt = null), (menuFor = open ? id : null))}
            onGrab={(e) => grab(e, id)}
            onWiden={cols >= 2 ? (e) => grip(e, id) : null}
            width={p.w}
          />
        </div>
      </div>
    {/if}
  {/each}
</div>

<style>
  .board {
    position: relative;
    width: 100%;
    min-height: 40px;
  }
  .slot {
    position: absolute;
    left: 0;
    top: 0;
    will-change: transform;
  }
  .slot.dragging {
    z-index: 20;
  }
  .slot.menu-open,
  .slot:has(:global([role="dialog"])) {
    z-index: 25;
  }
  /* "+ New note": dashed like the hours' strip, on paper (a textured board never shows through
     its words); in the accent while a drop there would make the note. */
  .new-slot {
    position: absolute;
    left: 0;
    top: 0;
    /* (Its room underneath is the board's empty space.) */
    pointer-events: none;
  }
  .new-card {
    position: relative;
    pointer-events: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s2);
    padding: 0 var(--s4);
    border-radius: var(--r-lg);
    background: color-mix(in srgb, var(--card) 78%, transparent);
    color: var(--icon);
    user-select: none;
    animation: new-in 0.26s var(--ease-glide) 0.04s backwards;
    transition:
      background-color 0.18s var(--ease-glide),
      color 0.15s;
  }
  .new-card::before {
    content: "";
    position: absolute;
    inset: 0;
    border: 1.5px dashed var(--box);
    border-radius: inherit;
    pointer-events: none;
    transition: border-color 0.15s;
  }
  .new-words {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--serif);
    font-style: italic;
    font-size: 15px;
    color: var(--muted);
    transition: color 0.15s;
  }
  .new-card.on {
    background: color-mix(in srgb, var(--accent) 7%, var(--card));
    color: var(--accent);
  }
  .new-card.on::before {
    border-color: var(--accent);
  }
  .new-card.on .new-words {
    color: var(--soft);
  }
  @keyframes new-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.97);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .new-card {
      animation: new-fade 0.12s ease backwards;
    }
  }
  @keyframes new-fade {
    from {
      opacity: 0;
    }
  }
  .placeholder {
    position: absolute;
    left: 0;
    top: 0;
    border: 1.5px dashed var(--edge);
    border-radius: var(--r-lg);
    animation: ph-in 0.2s ease;
    /* It moves with the notes making room (same glide). */
    transition: transform 0.3s var(--ease-glide), height 0.3s var(--ease-glide);
  }
  @keyframes ph-in {
    from {
      opacity: 0;
    }
  }
  .widen {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 24;
    border: 1.5px dashed var(--box);
    border-radius: var(--r-lg);
    pointer-events: none;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 8px;
  }
  .widen {
    transition: width 0.22s var(--ease-glide), border-color 0.15s;
  }
  /* Reduced motion: the notes snap to their places, so the gap and the ghost do too. */
  @media (prefers-reduced-motion: reduce) {
    .placeholder,
    .widen {
      transition: none;
    }
  }
  .widen.want {
    border-color: var(--ink);
  }
  .wswap {
    display: grid;
  }
  .wswap > span {
    grid-area: 1 / 1;
  }
  .wswap > span {
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-xs);
    font-weight: 600;
  }
</style>
