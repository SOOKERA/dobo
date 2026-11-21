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
  import { store } from "../lib/store.svelte";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import type { Note } from "../lib/types";
  import { arrange, moveTo, sameSeq, type Step } from "../lib/arrange";
  import { untrack } from "svelte";
  import { t } from "../lib/i18n.svelte";
  import { crumbIn, crumbOut, fadeQuick } from "../lib/motion";

  let { notes, scroller }: { notes: Note[]; scroller?: HTMLElement } = $props();

  const GAP = 18;
  const MIN_COL = 248;
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  let isDark = $derived(look.dark);

  let width = $state(0);
  let heights = $state<Record<number, number>>({});
  let order = $state<number[]>([]);
  let menuFor = $state<number | null>(null);

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
  // new column count, a note made wide or a new order packs again.
  let wideSig = $derived(notes.map((n) => (n.wide ? n.id : "")).join(","));
  let holdSig = $derived(`${savedKey}|${order.join(",")}|${wideSig}`);
  let hold = $state<{ sig: string; seq: Step[] } | null>(null);
  let held = $derived(hold && hold.sig === holdSig ? hold.seq : null);

  let layout = $derived.by(() => {
    const a = arrange({
      ids: order.filter((id) => byId.has(id)),
      wide: (id) => !!byId.get(id)?.wide,
      height: (id) => heights[id] ?? 140,
      cols,
      colW,
      gap: GAP,
      saved: working ?? saved ?? held,
    });
    return { pos: a.pos, seq: a.seq, height: a.height, cols, width };
  });

  // A note made wide or normal: the others move with the graceful reflow, not a plain glide.
  let prevWideSig = "";
  $effect.pre(() => {
    if (prevWideSig && wideSig !== prevWideSig) reflowUntil = performance.now() + 500;
    prevWideSig = wideSig;
  });

  // ---- Widening a note by its edge grip ----
  let widen = $state<{ id: number; w: number; want: boolean } | null>(null);
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
      window.removeEventListener("keydown", key, true);
      document.body.style.cursor = "";
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
  function running(node: HTMLElement): Animation[] {
    return node.getAnimations({ subtree: true }).filter((a) => {
      const end = a.effect?.getComputedTiming().endTime;
      return a.playState === "running" && typeof end === "number" && Number.isFinite(end);
    });
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
    hold = { sig: holdSig, seq: layout.seq };
  }
  function measure(node: HTMLElement, id: number) {
    const ro = new ResizeObserver(() => {
      const anims = heights[id] == null || reduce.matches ? [] : running(node);
      if (anims.length) {
        const e = easing.get(id);
        // Still the same easing: its end is already known.
        if (e && anims.every((a) => e.anims.has(a))) return;
        const { h, total, at, start } = settled(node, anims);
        easing.set(id, { anims: new Set(anims), h });
        if (heights[id] !== h) {
          if (!inStep || total - at > inStep.total - inStep.at) inStep = { total, at, start };
          holdColumns();
          heights[id] = h;
        }
        return;
      }
      easing.delete(id);
      const h = node.offsetHeight;
      if (heights[id] === h) return;
      if (heights[id] != null) holdColumns();
      heights[id] = h;
    });
    ro.observe(node);
    return { destroy: () => (ro.disconnect(), easing.delete(id)) };
  }

  // ---- FLIP: animate from where each note is seen now to its new place ----
  const slots = new Map<number, HTMLElement>();
  function slot(node: HTMLElement, id: number) {
    slots.set(id, node);
    return { destroy: () => slots.delete(id) };
  }

  let boardEl: HTMLDivElement;
  let seen = new Map<number, { x: number; y: number }>();
  let prevCols = 0;
  let prevWidth = 0;
  let prevHeight = 0;
  let first = true;
  let justDropped: number | null = null;
  // After the columns change, the notes are measured again at their new width (same frame):
  // that second pass must keep the reflow style instead of replacing it with a plain glide.
  let reflowUntil = 0;

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
    const colsChanged = prevCols !== 0 && l.cols !== prevCols;
    const onlyWidth = !colsChanged && prevWidth !== 0 && Math.abs(l.width - prevWidth) > 0.5;
    // Until every note has been measured once, notes just take their place (no gliding).
    const measured = order.every((id) => heights[id] != null);
    const wasFirst = first || !measured;
    prevCols = l.cols;
    prevWidth = l.width;
    first = !measured;
    if (colsChanged) reflowUntil = performance.now() + 160;
    const reflow = colsChanged || performance.now() < reflowUntil;
    // In step with a note easing its height (its own ease-out, for the time it has left).
    const step = inStep && inStep.total - inStep.at > 40 ? inStep : null;
    inStep = null;
    const wasHeight = prevHeight;
    prevHeight = l.height;
    if (reduce.matches) return;

    // The board taken straight to its new, shorter height would pull the page up at once under a
    // window scrolled to the end (the note is still folding): it eases down with the fold.
    // (A new placing round takes over from one still easing.)
    if (Math.abs(l.height - wasHeight) > 0.5) for (const a of boardEl.getAnimations()) a.cancel();
    if (step && !wasFirst && l.height < wasHeight - 0.5) {
      const a = boardEl.animate([{ height: `${Math.max(wasHeight, 0)}px` }, { height: `${Math.max(l.height, 0)}px` }], { duration: step.total, easing: FOLD_EASE });
      if (step.start != null) a.startTime = step.start;
      else a.currentTime = step.at;
    }

    let k = 0;
    for (const [id, el] of slots) {
      if (id === carried) continue;
      const to = l.pos.get(id);
      if (!to) continue;
      const from = seen.get(id);
      if (!from) {
        // A note put back from the archive while the Archive page covers the board: it waits,
        // unseen, and settles in once the board shows again (below).
        if (untrack(() => store.over && store.arriving.includes(id))) {
          el.style.opacity = "0";
          continue;
        }
        // A new note: it settles in.
        if (!wasFirst) {
          el.animate(
            [
              { opacity: 0, transform: `translate(${to.x}px, ${to.y + 10}px) scale(0.97)` },
              { opacity: 1, transform: `translate(${to.x}px, ${to.y}px)` },
            ],
            { duration: 320, easing: "cubic-bezier(.2,.8,.2,1)" },
          );
        }
        continue;
      }
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      if (wasFirst || (onlyWidth && !reflow)) continue;
      for (const a of el.getAnimations()) a.cancel();
      const fromT = `translate(${from.x}px, ${from.y}px)`;
      const toT = `translate(${to.x}px, ${to.y}px)`;
      if (reflow) {
        const tilt = (dx >= 0 ? 1 : -1) * 1.5;
        el.animate(
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
          const a = el.animate([{ transform: fromT }, { transform: toT }], { duration: step.total, easing: FOLD_EASE });
          if (step.start != null) a.startTime = step.start;
          else a.currentTime = step.at;
          continue;
        }
        el.animate([{ transform: fromT }, { transform: toT }], {
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
        for (const a of el.getAnimations()) a.cancel();
        const at = `translate(${p.x}px, ${p.y}px)`;
        const a = reduce.matches
          ? el.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 120, fill: "forwards" })
          : el.animate(
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
          el.animate(
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
        const el = slots.get(id);
        el?.getAnimations().forEach((a) => a.cancel());
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
      window.removeEventListener("pointercancel", up);
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
      stop();
      const tilt = drag.tilt;
      const card = slots.get(id)?.querySelector(".card") as HTMLElement | null;
      drag = null;
      working = null;
      if (card && !reduce.matches)
        card.animate([{ transform: `rotate(${tilt}deg) scale(1.04)` }, { transform: "rotate(0deg) scale(1)" }], { duration: 300, easing: "cubic-bezier(.3,.8,.25,1)" });
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    window.addEventListener("keydown", key, true);
  }
</script>

<div class="board" bind:this={boardEl} bind:clientWidth={width} style:height="{Math.max(layout.height, 0)}px">
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
        style:--tilt="{dragging && drag ? drag.tilt.toFixed(2) : -2.5}deg"
      >
        <Holder look={look.active} {id} {index} />
        <div use:measure={id}>
          <NoteCard
            {note}
            items={store.itemsByNote.get(id) ?? []}
            tone={isDark ? swatchOf(note, store.custom).dark : swatchOf(note, store.custom).light}
            menuOpen={menuFor === id}
            lifted={dragging}
            onMenu={(open) => (menuFor = open ? id : null)}
            onGrab={(e) => grab(e, id)}
            onWiden={cols >= 2 ? (e) => grip(e, id) : null}
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
