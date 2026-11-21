<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The quick tour (eight steps, under a minute): a soft veil over the panel with a lit window on
  // the real thing each step is about, and a bubble beside it with one sentence. The window and
  // the bubble glide from one place to the next (and back the same way); each step's words slide
  // the way you go. Small live demos instead of long text: the quick dump bar types a thought;
  // the entry line writes a time, then a "!" that makes it a deadline; a slip flies from the
  // sample line up onto the real hours ruler, then onto a bigger task, where it becomes one of
  // its steps. Nothing is saved: the sample lines live in memory and the entry line gets back
  // what you had typed.
  // Keys: → or Enter next, ← back, Esc closes; the mouse's back button goes back a step. No other
  // key reaches the board or the app while it shows (with Alt or Ctrl neither).
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { styleButton, tour } from "../lib/tour.svelte";
  import { DIP_OUT, PREFER, STEPS, clip, cornerPlan, cornerWay, fadePlan, fadeWay, laterWords, placeBubble, straightPlan, straightWay, union, type Fade, type Pose, type StepId, type Way } from "../lib/tourPlan";
  import { pushLayer } from "../lib/layers";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { SAMPLE_ITEM, SAMPLE_PARENT } from "../lib/sample";
  import { liveLine } from "../lib/keys";
  import { easeHeight, glide, reduced, rise, stepIn, stepOut, veil, type Box4 } from "../lib/motion";
  import { around, i18n, keysText, t, type Key } from "../lib/i18n.svelte";

  const WORDS: Record<StepId, { title: Key; text: Key }> = {
    dump: { title: "Dump it, from anywhere", text: "Press {keys} in any app, type, Enter: it’s kept, and you’re back to work." },
    inbox: { title: "It waits in the Inbox", text: "What you dump lands in this strip, like anything you type at the top." },
    tidy: { title: "Sort it into notes", text: "Drag a line onto the note it belongs to, or let Tidy sort them one card at a time." },
    time: { title: "Give it a time", text: "Write a time as you type; end with ! for a deadline that rings early, then more often." },
    ruler: { title: "Or drag it onto the hours", text: "Already on the board? Drag a task up onto today’s hours." },
    step: { title: "Or make it a step", text: "Drop it onto a bigger task: it becomes one of its steps." },
    style: { title: "Make it yours", text: "Click Style or press {keys} to change the board’s look: background, colours, notes." },
    more: { title: "Help is always here", text: "The tour again, keyboard shortcuts, the Archive, Help & settings: one click away." },
  };
  // The Style button gives way as the panel narrows: its brush alone, or inside the ⋯ menu.
  const STYLE_WORDS: Record<typeof styleButton.shown, Key> = {
    words: WORDS.style.text,
    brush: "Click the brush or press {keys} to change the board’s look: background, colours, notes.",
    menu: "Style is in here: it changes the board’s look. Or press {keys}.",
  };
  /** The keys a step's words name. */
  const STEP_KEYS: Partial<Record<StepId, () => string>> = { dump: () => store.shortcuts.dump, style: () => "Ctrl+Shift+S" };
  const MARK = "⁣";
  let words = $derived(tour.step === "style" ? { ...WORDS.style, text: STYLE_WORDS[styleButton.shown] } : WORDS[tour.step]);
  let parts = $derived(around(t(words.text, { keys: MARK }), MARK));
  let hasKeys = $derived(words.text.includes("{keys}"));

  // ---- Esc, Back / Forward, arrows ----
  // (Forward on the last step stays there: there is nothing further.)
  $effect(() => pushLayer(() => tour.close(), { back: () => tour.back(), forward: () => !tour.last && tour.next() }));

  let rootEl = $state<HTMLElement>();
  let cutA = $state<SVGRectElement>();
  let cutB = $state<SVGRectElement>();
  let ringA = $state<SVGRectElement>();
  let ringB = $state<SVGRectElement>();
  let glowA = $state<SVGRectElement>();
  let glowB = $state<SVGRectElement>();
  let ringsEl = $state<SVGGElement>();
  // (Where the window was, fading out, with less motion.)
  let cutWasA = $state<SVGRectElement>();
  let cutWasB = $state<SVGRectElement>();
  let ringWasA = $state<SVGRectElement>();
  let ringWasB = $state<SVGRectElement>();
  let glowWasA = $state<SVGRectElement>();
  let glowWasB = $state<SVGRectElement>();
  let ringsWasEl = $state<SVGGElement>();
  // The veil darkens in by a plain colour transition (an opacity fade-in left it unpainted).
  let veilOn = $state(false);
  $effect(() => {
    const id = requestAnimationFrame(() => requestAnimationFrame(() => (veilOn = true)));
    return () => cancelAnimationFrame(id);
  });
  let bubbleEl = $state<HTMLElement>();
  let roomEl = $state<HTMLElement>();
  let stackEl = $state<HTMLElement>();
  let nextEl = $state<HTMLButtonElement>();

  $effect(() => {
    const key = (e: KeyboardEvent) => {
      const a = document.activeElement as HTMLElement | null;
      const onButton = !!a && !!bubbleEl?.contains(a) && a.tagName === "BUTTON";
      const ctrl = e.ctrlKey || e.metaKey;
      // Alt+Left / Alt+Right, Backspace and the Browser Back / Forward keys go through the app's
      // own Back / Forward (one press, one step), which lands on this tour's layer. Only those:
      // any other key with Alt would reach the board (and act on the sample line).
      const plain = !ctrl && !e.shiftKey;
      const nav =
        (plain && e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) ||
        (plain && !e.altKey && e.key === "Backspace") ||
        e.key === "BrowserBack" ||
        e.key === "BrowserForward";
      if (nav) return;
      if (e.key === "Tab" && !ctrl && !e.altKey) return trap(e);
      // Nothing else reaches the board or the app while the tour shows.
      e.stopPropagation();
      // Ctrl keys do nothing here (not the web view's own find, print or paste either).
      if (ctrl) return e.preventDefault();
      if (e.altKey) return;
      if (e.key === "Escape") tour.close();
      else if (e.key === "ArrowRight" && !e.repeat) tour.next();
      else if (e.key === "ArrowLeft" && !e.repeat) tour.back();
      // Enter presses the button that has the keyboard (Next at first), else goes on.
      else if (e.key === "Enter" && !e.repeat) (onButton ? (a as HTMLButtonElement).click() : tour.next());
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  });
  /** Tab stays in the bubble. */
  function trap(e: KeyboardEvent) {
    if (!bubbleEl) return;
    const all = [...bubbleEl.querySelectorAll<HTMLElement>("button:not(:disabled)")];
    if (!all.length) return;
    const a = document.activeElement as HTMLElement | null;
    const i = a ? all.indexOf(a) : -1;
    const to = i < 0 ? all[0] : e.shiftKey ? (i === 0 ? all[all.length - 1] : null) : i === all.length - 1 ? all[0] : null;
    if (to) {
      e.preventDefault();
      to.focus();
    }
  }

  /** `stop`, run once: when the tour closes (at once, not when its fade-out ends) or when what
   * started it ends, whichever comes first. */
  function whenLeft(stop: () => void): () => void {
    let done = false;
    const run = () => {
      if (done) return;
      done = true;
      off();
      stop();
    };
    const off = tour.onClose(run);
    return run;
  }

  // The keyboard comes to the bubble (in this window only; it gives it back on the way out: to
  // where it was, else, when that has gone, as the tour was started from, to the entry line).
  $effect(() => {
    const before = document.activeElement as HTMLElement | null;
    const id = requestAnimationFrame(() => nextEl?.focus({ preventScroll: true }));
    return whenLeft(() => {
      cancelAnimationFrame(id);
      // (Unless something else took the keyboard meanwhile, like a view opening.)
      const a = document.activeElement;
      if (a && a !== document.body && !rootEl?.contains(a)) return;
      const to = before?.isConnected && before !== document.body ? before : document.getElementById("capture");
      to?.focus({ preventScroll: true });
    });
  });

  // ---- What each step points at (in the panel's coordinates) ----
  const scroller = () => document.querySelector<HTMLElement>(".shell .scroll");
  const rectOf = (el: Element | null | undefined): Box4 | null => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return r.width || r.height ? { x: r.left, y: r.top, w: r.width, h: r.height } : null;
  };
  const pad = (b: Box4, p: number): Box4 => ({ x: b.x - p, y: b.y - p, w: b.w + 2 * p, h: b.h + 2 * p });
  /** On the board, only the part that shows under the top of the panel counts. */
  const onBoard = (b: Box4 | null) => {
    const s = rectOf(scroller());
    return b && s ? clip(b, s) : b;
  };
  /** The Inbox strip the sample line waits in. */
  function sampleNoteEl() {
    return tour.sampleItem ? document.querySelector("[data-inbox-strip]") : null;
  }
  function sampleLineEl() {
    const it = tour.sampleItem;
    return it ? document.querySelector(liveLine(it.id)) : null;
  }
  /** The bigger task, with its steps. */
  const parentEl = () => document.querySelector(liveLine(SAMPLE_PARENT))?.closest("li");
  /** One lit window, or two (the hours and the line carried onto them); `on`: what it lights,
   * when that may change within a step (the Style button, or the ⋯ it goes into). */
  type Lit = { a: Box4; b: Box4 | null; r: number; on?: string };
  function target(step: StepId): Lit | null {
    const one = (el: Element | null, p: number, r: number): Lit | null => {
      const b = rectOf(el);
      return b ? { a: pad(b, p), b: null, r } : null;
    };
    if (step === "dump") return null;
    if (step === "inbox") {
      const note = rectOf(sampleNoteEl());
      const line = rectOf(sampleLineEl());
      if (!note) return null;
      // The strip's head and the new line (not the whole of a long Inbox).
      const box = onBoard(line && line.y > note.y ? { ...note, h: line.y + line.h - note.y } : note);
      return box ? { a: pad(box, 6), b: null, r: 18 } : null;
    }
    if (step === "tidy") return one(document.querySelector("[data-tour=tidy]"), 4, 12);
    if (step === "time") {
      // The entry line and, under it, what it reads in what you typed.
      const field = rectOf(document.querySelector("[data-tour=capture]"));
      const hints = rectOf(document.querySelector("[data-tour=capture-hints]"));
      if (!field) return null;
      return { a: pad(hints ? union(field, { ...hints, x: field.x, w: field.w }) : field, 6), b: null, r: 18 };
    }
    if (step === "ruler") {
      const line = onBoard(rectOf(sampleLineEl()));
      const ruler = rectOf(document.querySelector(".shell .ruler"));
      if (!ruler) return line ? { a: pad(line, 4), b: null, r: 12 } : null;
      return { a: pad(ruler, 5), b: line ? pad(line, 4) : null, r: 18 };
    }
    if (step === "step") {
      // The bigger task (with its steps) and, while it is still on its own, the line above it.
      const task = rectOf(parentEl());
      const line = tour.sampleItem?.parent_id == null ? rectOf(sampleLineEl()) : null;
      const box = onBoard(task && line ? union(task, line) : (task ?? line));
      return box ? { a: pad(box, 5), b: null, r: 16 } : null;
    }
    if (step === "style") {
      // The Style button (its words or its brush), else the ⋯ menu it is tucked into.
      const btn = document.querySelector("[data-tour=style]");
      const lit = rectOf(btn) ? one(btn, 4, 13) : one(document.querySelector("[data-tour=more]"), 4, 12);
      return lit && { ...lit, on: rectOf(btn) ? "style" : "more" };
    }
    return one(document.querySelector("[data-tour=help]"), 4, 11);
  }

  // ---- The windows and the bubble glide ----
  // Two windows always exist; with one place to show, the second sits on the first (unseen) and
  // splits off when a step shows two. Between the Style button and the "?" at the board's
  // corner, the way is longer: the window goes along the top bar and down the side, and the
  // bubble keeps out of its way (cornerWay). With less motion, the old place fades out as the new
  // one fades in, and the bubble fades out and back in at its new place.
  let pose: Pose | null = null;
  /** The move under way: from where things were seen, the corner way or not, which way (with
   * less motion, its fade), the bubble's height and how lit the window was when it began. */
  type Leg = { from: Pose; t0: number; corner: boolean; up: boolean; way: Way | "glide" | null; late: Glide | null; fade: Fade | null; bh0: number; lit0: number };
  /** A glide that turned into a dip on the way (see `late` in straightWay): where it went from
   * and to, and when it began. */
  type Glide = { from: Pose; to: Pose; t0: number };
  let leg: Leg | null = null;
  const setOff = (from: Pose, t0: number, more: Partial<Leg> = {}): Leg => ({
    from: { ...from, a: { ...from.a }, b: { ...from.b } },
    t0,
    corner: false,
    up: false,
    way: null,
    late: null,
    fade: null,
    bh0: seenH,
    lit0: seenLit,
    ...more,
  });
  /** The bubble's height and how lit the window was, as last drawn. */
  let seenH = 0;
  let seenStack = 0;
  let seenLit = 1;
  /** What the window lit, as last drawn (see Lit's `on`). */
  let seenOn: string | null = null;
  /** Where things were going, as last drawn. */
  let seenTo: Pose | null = null;
  let size = [0, 0];
  const GO = 480;
  /** (A straight move with the bubble fading out and back in takes a little longer.) */
  const DIP = 640;
  const CORNER = 820;
  const FADE = 260;

  // A new step: start from wherever things are seen now.
  let was: StepId | null = null;
  $effect(() => {
    const step = tour.step;
    untrack(() => {
      if (pose && was && was !== step) {
        const pair = new Set([was, step]);
        const corner = pair.has("style") && pair.has("more");
        leg = setOff(pose, performance.now(), { corner, up: corner && step === "style" });
      }
      was = step;
    });
  });

  const local = (b: Box4, S: DOMRect): Box4 => ({ ...b, x: b.x - S.left, y: b.y - S.top });
  function frame(now: number) {
    // Leaving: everything stays where it is while it fades (what it pointed at goes away).
    if (!rootEl || !bubbleEl || !tour.open) return;
    const S = rootEl.getBoundingClientRect();
    const W = S.width;
    const H = S.height;
    const tg = target(tour.step);
    const a = tg ? local(tg.a, S) : null;
    const b = tg?.b ? local(tg.b, S) : null;
    const around = a && b ? union(a, b) : a;
    const bw = bubbleEl.offsetWidth;
    // (The first frame after the words change, the room hasn't begun to ease yet: it would read
    // the new height at once, though the old one is what is drawn.)
    const stackH = stackEl?.offsetHeight ?? 0;
    const easing = !!roomEl && !reduced() && Math.abs(stackH - seenStack) > 0.5 && !roomEl.getAnimations().length && seenH > 0;
    const bh = easing ? seenH : bubbleEl.offsetHeight;
    seenStack = stackH;
    // (While its words change, the bubble's height eases from one to the other: the side it
    // sits on and the way it goes are chosen for the taller of the two.)
    const tall = Math.max(bh, endHeight());
    const at = placeBubble(around, bw, bh, W, H, around ? PREFER[tour.step] : "center", tall);
    // Nothing to point at: the window closes to a point at the bubble's middle.
    const dot = { x: at.x + bw / 2, y: at.y + bh / 2, w: 0, h: 0 };
    const to: Pose = a
      ? { a, b: b ?? a, r: tg!.r, ring: 1, two: b ? 1 : 0, bx: at.x, by: at.y, show: 1 }
      : { a: dot, b: dot, r: 20, ring: 0, two: 0, bx: at.x, by: at.y, show: 1 };
    const on = tg ? (tg.on ?? tour.step) : null;
    const less = reduced();
    let next = to;
    let lit = 1;
    let fading = 0;
    // Within a step, the bubble may have to change sides at once (the hours opening make what
    // it points at taller), or what the step lights may change (the Style button going into the
    // ⋯ menu as the panel narrows): they glide there too instead of jumping.
    if (!leg && pose && (on !== seenOn || Math.abs(to.bx - pose.bx) > 48 || Math.abs(to.by - pose.by) > 48)) leg = setOff(pose, now);
    seenOn = on;
    if (leg && !less && !leg.corner) {
      leg.way ??= straightPlan(leg.from, to, bw, tall);
      // What the step points at may still change on the way (the bigger task folding in): if
      // the window would now pass under the bubble, the bubble fades out from where it is, and
      // the window goes on without stopping. (Not a window opening from the bubble's middle:
      // that one is meant to.)
      if (leg.way === "glide" && leg.from.ring >= 0.5 && pose && straightPlan(pose, to, bw, tall) === "dip")
        leg = setOff(pose, now, { way: "dip", late: { from: leg.from, to: seenTo ?? to, t0: leg.t0 } });
    }
    const L = leg;
    if (L) {
      const f = L.from;
      const p = Math.min(1, (now - L.t0) / (less ? FADE : L.corner ? CORNER : L.way === "dip" ? DIP : GO));
      // A bubble over its window grows upwards as it fades out there (its words changing), so it
      // never comes nearer the window.
      const top = Math.min(f.a.y, f.two > 0.5 ? f.b.y : Infinity);
      const lift = f.ring >= 0.5 && f.by + L.bh0 <= top + 1 ? Math.max(0, bh - L.bh0) : 0;
      if (less) {
        // The old place fades out as the new one fades in, each while no bubble is over it; the
        // bubble fades out, then in there.
        L.fade ??= fadePlan(f, to, bw, tall);
        const w = fadeWay(f, to, p, L.fade, lift, L.lit0);
        next = w.pose;
        lit = w.lit;
        fading = w.old;
      } else if (L.corner) {
        // (Room beside the window's way, or not.)
        const s = L.up ? to : f;
        const m = L.up ? f : to;
        L.way ??= cornerPlan(s, m, bw, tall);
        next = cornerWay(s, m, L.up ? 1 - p : p, bw, L.way as Way);
      } else {
        next = straightWay(f, to, p, L.way === "dip" ? "dip" : "glide", lift, !!L.late);
        // (Turned into a dip on the way: the bubble fades out going on as it went, keeping ahead
        // of the window behind it, not stopping in its way.)
        const g = L.late;
        if (g && p < DIP_OUT) {
          const e = glide(Math.min(1, (now - g.t0) / GO));
          next = { ...next, bx: g.from.bx + (g.to.bx - g.from.bx) * e, by: g.from.by + (g.to.by - g.from.by) * e - lift };
        }
      }
      if (p >= 1) leg = null;
    }
    const before = pose;
    pose = next;
    seenH = bh;
    seenLit = lit;
    seenTo = to;
    if (!L && before && same(before, next) && W === size[0] && H === size[1]) return;
    size = [W, H];
    put(cutA, next.a, next.r, 0);
    put(cutB, next.b, next.r, 0);
    put(ringA, next.a, next.r, 0);
    put(ringB, next.b, next.r, 0);
    put(glowA, next.a, next.r, 3);
    put(glowB, next.b, next.r, 3);
    cutA?.setAttribute("opacity", lit.toFixed(3));
    cutB?.setAttribute("opacity", lit.toFixed(3));
    ringsEl?.style.setProperty("--ring", (next.ring * lit).toFixed(3));
    ringsEl?.style.setProperty("--two", (next.ring * next.two * lit).toFixed(3));
    // With less motion, the place it was at, fading out.
    const f = L && less ? L.from : null;
    if (f) {
      put(cutWasA, f.a, f.r, 0);
      put(cutWasB, f.b, f.r, 0);
      put(ringWasA, f.a, f.r, 0);
      put(ringWasB, f.b, f.r, 0);
      put(glowWasA, f.a, f.r, 3);
      put(glowWasB, f.b, f.r, 3);
    }
    cutWasA?.setAttribute("opacity", fading.toFixed(3));
    cutWasB?.setAttribute("opacity", fading.toFixed(3));
    ringsWasEl?.style.setProperty("--ring", f ? (f.ring * fading).toFixed(3) : "0");
    ringsWasEl?.style.setProperty("--two", f ? (f.ring * f.two * fading).toFixed(3) : "0");
    bubbleEl.style.transform = `translate(${next.bx.toFixed(1)}px, ${next.by.toFixed(1)}px)`;
    bubbleEl.style.opacity = next.show >= 0.999 ? "" : next.show.toFixed(3);
  }
  /** The bubble's height once its words have settled (its room eases to it meanwhile). */
  function endHeight() {
    if (!bubbleEl || !roomEl || !stackEl) return 0;
    return bubbleEl.offsetHeight - roomEl.offsetHeight + stackEl.offsetHeight;
  }
  /** A rounded box drawn `grow` px outside the window. */
  function put(el: SVGRectElement | undefined, b: Box4, r: number, grow: number) {
    if (!el) return;
    const w = Math.max(0, b.w + 2 * grow);
    const h = Math.max(0, b.h + 2 * grow);
    el.setAttribute("x", (b.x - grow).toFixed(1));
    el.setAttribute("y", (b.y - grow).toFixed(1));
    el.setAttribute("width", w.toFixed(1));
    el.setAttribute("height", h.toFixed(1));
    el.setAttribute("rx", Math.max(0, Math.min(r + grow, w / 2, h / 2)).toFixed(1));
  }
  const near = (u: Box4, v: Box4) => Math.abs(u.x - v.x) < 0.05 && Math.abs(u.y - v.y) < 0.05 && Math.abs(u.w - v.w) < 0.05 && Math.abs(u.h - v.h) < 0.05;
  const same = (u: Pose, v: Pose) =>
    near(u.a, v.a) && near(u.b, v.b) && Math.abs(u.bx - v.bx) < 0.05 && Math.abs(u.by - v.by) < 0.05 && Math.abs(u.ring - v.ring) < 0.001 && Math.abs(u.two - v.two) < 0.001 && Math.abs(u.show - v.show) < 0.001;

  // One look per frame while the tour shows: what it points at may move (the board scrolling or
  // making room, the header changing rows, the window resized). Only one box is read and only
  // the two pieces of the tour are written, so it costs next to nothing.
  $effect(() => {
    let raf = 0;
    const loop = (now: number) => {
      frame(now);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });

  // The sample line stays in view on the board, whatever happens meanwhile (a line dumped from the
  // quick dump bar reads the notes again; work hours or a reminder show another space).
  $effect(() => {
    if (tour.step === "dump") return;
    void [store.items, store.visibleNotes, i18n.lang];
    untrack(() => tour.keepSample());
  });

  // The Inbox strip stays open while the tour shows what waits in it (folded again after, unless
  // it was open already).
  $effect(() => {
    store.inboxTour = tour.step !== "dump";
    return () => (store.inboxTour = false);
  });

  // ---- Each step's own moment ----
  $effect(() => {
    const step = tour.step;
    const stop = untrack(() => enter(step));
    return stop ? whenLeft(stop) : undefined;
  });

  function enter(step: StepId): (() => void) | void {
    if (step !== "dump") tour.addSample();
    if (step === "inbox" || step === "ruler") requestAnimationFrame(() => requestAnimationFrame(reveal));
    if (step === "dump") return dumpDemo();
    if (step === "time") return timeDemo();
    if (step === "ruler") return rulerDemo();
    if (step === "step") return stepDemo();
  }
  /** The sample line in view on the board (it may sit lower down), with the Inbox strip's head
   * (the line is the Inbox's first). */
  function reveal() {
    const note = sampleNoteEl();
    const line = (tour.step === "step" ? parentEl() : null) ?? sampleLineEl() ?? note;
    const sc = scroller();
    if (!line || !sc) return;
    const r = line.getBoundingClientRect();
    const s = sc.getBoundingClientRect();
    const top = Math.min(r.top, note?.getBoundingClientRect().top ?? r.top);
    if (top >= s.top + 8 && r.bottom <= s.bottom - 160) return;
    sc.scrollBy({ top: top - s.top - 12, behavior: reduced() ? "auto" : "smooth" });
  }

  /** Waits, unless the step is left first. */
  function waiter() {
    let live = true;
    const timers = new Set<ReturnType<typeof setTimeout>>();
    const wait = (ms: number) =>
      new Promise<boolean>((res) => {
        if (!live) return res(false);
        const id = setTimeout(() => (timers.delete(id), res(live)), ms);
        timers.add(id);
      });
    const stop = () => {
      live = false;
      timers.forEach(clearTimeout);
    };
    return { wait, stop, alive: () => live };
  }

  // The quick dump bar, in small: a thought typed, Enter, "In the Inbox".
  let typed = $state("");
  let pressed = $state(false);
  let sent = $state(false);
  function dumpDemo() {
    const w = waiter();
    const text = t("Call the bank");
    if (reduced()) {
      typed = text;
      return w.stop;
    }
    void (async () => {
      while (w.alive()) {
        typed = "";
        sent = false;
        if (!(await w.wait(700))) return;
        for (const ch of text) {
          typed += ch;
          if (!(await w.wait(ch === " " ? 90 : 58))) return;
        }
        if (!(await w.wait(450))) return;
        pressed = true;
        if (!(await w.wait(140))) return;
        pressed = false;
        typed = "";
        sent = true;
        if (!(await w.wait(1900))) return;
      }
    })();
    return () => {
      w.stop();
      pressed = false;
    };
  }

  // The real entry line writes a time, then the "!": its own hints show what that means.
  // What you had typed there comes back after.
  function timeDemo() {
    const input = document.getElementById("capture") as HTMLInputElement | null;
    if (!input) return;
    const kept = input.value;
    const set = (v: string) => {
      input.value = v;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    };
    const w = waiter();
    const base = t("Call Lea {when}", { when: laterWords(store.now, i18n.lang) });
    if (reduced()) set(base + "!");
    else
      void (async () => {
        while (w.alive()) {
          set("");
          if (!(await w.wait(500))) return;
          for (let k = 1; k <= base.length; k++) {
            set(base.slice(0, k));
            if (!(await w.wait(base[k - 1] === " " ? 80 : 50))) return;
          }
          if (!(await w.wait(1100))) return;
          set(base + "!");
          if (!(await w.wait(2800))) return;
        }
      })();
    return () => {
      w.stop();
      set(kept);
    };
  }

  // The real ruler opens (as for a carry) and a paper slip flies from the sample line onto an
  // hour, shrinks into it, and the ruler names the time. Again, until the step is left.
  function rulerDemo() {
    const it = tour.sampleItem;
    if (!it) return;
    store.ruler = { item: it, mode: "demo", at: null };
    const w = waiter();
    const AT = 0.56;
    let slip: HTMLElement | null = null;
    const mark = (at: number | null) => {
      if (store.ruler?.mode === "demo") store.ruler = { ...store.ruler, at };
    };
    if (reduced()) mark(AT);
    else
      void (async () => {
        // The ruler opening pushes the board down: the line is brought back into view first.
        if (!(await w.wait(420))) return;
        reveal();
        if (!(await w.wait(380))) return;
        while (w.alive() && rootEl) {
          const S = rootEl.getBoundingClientRect();
          const text = sampleLineEl()?.querySelector(".text")?.getBoundingClientRect();
          const track = document.querySelector(".shell .ruler .track")?.getBoundingClientRect();
          if (!text || !track) {
            if (!(await w.wait(400))) return;
            continue;
          }
          slip = document.createElement("div");
          slip.className = "tour-slip";
          slip.textContent = it.text;
          rootEl.appendChild(slip);
          const sw = slip.offsetWidth;
          const sh = slip.offsetHeight;
          // The slip's text sits 13 px in from its edge, over the line's own text.
          const x0 = text.left - S.left - 13;
          const y0 = text.top - S.top + text.height / 2 - sh / 2;
          const x1 = track.left - S.left + track.width * AT - sw / 2;
          const y1 = track.top - S.top + track.height / 2 - sh / 2;
          const go = slip.animate(
            [
              { transform: `translate(${x0}px, ${y0}px) rotate(0deg) scale(0.97)`, opacity: 0 },
              { transform: `translate(${x0 + 4}px, ${y0 - 6}px) rotate(-2.5deg) scale(1)`, opacity: 1, offset: 0.16 },
              { transform: `translate(${x1}px, ${y1}px) rotate(-1deg) scale(1)`, opacity: 1 },
            ],
            { duration: 900, easing: "cubic-bezier(0.4, 0, 0.3, 1)", fill: "forwards" },
          );
          if (!(await go.finished.then(() => w.alive()).catch(() => false))) return;
          mark(AT);
          const land = slip.animate(
            [
              { transform: `translate(${x1}px, ${y1}px) rotate(-1deg) scale(1)`, opacity: 1 },
              { transform: `translate(${x1}px, ${y1}px) rotate(0deg) scale(0.15)`, opacity: 0 },
            ],
            { duration: 300, easing: "cubic-bezier(.4,0,.6,1)", fill: "forwards" },
          );
          await land.finished.catch(() => {});
          slip.remove();
          slip = null;
          if (!(await w.wait(1500))) return;
          mark(null);
          if (!(await w.wait(700))) return;
        }
      })();
    return () => {
      w.stop();
      slip?.remove();
      if (store.ruler?.mode === "demo") store.ruler = null;
    };
  }

  // The bigger task comes in under the sample line; a slip lifts off the line (its place goes
  // dashed, as when you carry one), flies onto the task ("Drop to make it a step"), and the line
  // becomes its step: the count goes from 1/2 to 1/3 and the ring round its box follows. A
  // moment later it goes back, and again, until the step is left (then the task folds away and
  // the line is on its own again, also when going Back).
  function stepDemo() {
    tour.addParent();
    const w = waiter();
    let slip: HTMLElement | null = null;
    const lift = (on: boolean) => {
      const away = itemDrag.away.filter((id) => id !== SAMPLE_ITEM);
      if (on || away.length !== itemDrag.away.length) itemDrag.away = on ? [...away, SAMPLE_ITEM] : away;
    };
    const over = (on: boolean) => {
      if (on) itemDrag.target = { kind: "task", id: SAMPLE_PARENT };
      else if (itemDrag.target?.id === SAMPLE_PARENT) itemDrag.target = null;
    };
    if (reduced()) {
      tour.nestSample(true);
      requestAnimationFrame(() => requestAnimationFrame(reveal));
    } else
      void (async () => {
        // The task folds in first, and the two are brought into view.
        if (!(await w.wait(320))) return;
        reveal();
        if (!(await w.wait(560))) return;
        while (w.alive() && rootEl) {
          const S = rootEl.getBoundingClientRect();
          const text = sampleLineEl()?.querySelector(".text")?.getBoundingClientRect();
          const ontoEl = document.querySelector(`${liveLine(SAMPLE_PARENT)} .text`);
          const onto = ontoEl?.getBoundingClientRect();
          if (!text || !onto || !ontoEl) {
            if (!(await w.wait(400))) return;
            continue;
          }
          slip = document.createElement("div");
          slip.className = "tour-slip";
          slip.textContent = tour.sampleItem?.text ?? "";
          rootEl.appendChild(slip);
          const sw = slip.offsetWidth;
          const sh = slip.offsetHeight;
          // From over the line's own text (13 px in from the slip's edge) to just after the
          // task's words, as a hand would hold it there, so they stay readable; in a narrow
          // panel, where it can't sit beside them (with the count), it hangs under the row,
          // clear of "Drop to make it a step", over where the step will come in.
          const words = document.createRange();
          words.selectNodeContents(ontoEl);
          const end = Math.min(words.getBoundingClientRect().right, onto.right);
          const x0 = text.left - S.left - 13;
          const y0 = text.top - S.top + text.height / 2 - sh / 2;
          // "Drop to make it a step" shows under the row's right end while it is held there
          // (ItemRow: 8 px in, 9 px below): the slip keeps clear of it, also in French, longer.
          const tag = dropTag(ontoEl.closest<HTMLElement>(".row"));
          const clear = (x: number) => !tag || x + sw <= tag.left - 6;
          const beside = end + 14 + sw <= onto.right - 36 && clear(end + 14);
          const xa = beside ? end + 14 : onto.left + 10;
          let ya = beside ? onto.top + onto.height / 2 - sh / 2 + 3 : onto.bottom + 12;
          if (!beside && tag && !clear(xa)) ya = Math.max(ya, tag.bottom + 4);
          const x1 = xa - S.left;
          const y1 = ya - S.top;
          lift(true);
          const go = slip.animate(
            [
              { transform: `translate(${x0}px, ${y0}px) rotate(0deg) scale(0.97)`, opacity: 0 },
              { transform: `translate(${x0 + 4}px, ${y0 - 6}px) rotate(-2.5deg) scale(1)`, opacity: 1, offset: 0.2 },
              { transform: `translate(${x1}px, ${y1}px) rotate(-1.5deg) scale(1)`, opacity: 1 },
            ],
            { duration: 760, easing: "cubic-bezier(0.4, 0, 0.3, 1)", fill: "forwards" },
          );
          // Held over the task: it lights up and says what a drop does.
          if (!(await w.wait(460))) return;
          over(true);
          if (!(await go.finished.then(() => w.alive()).catch(() => false))) return;
          if (!(await w.wait(520))) return;
          // Dropped: the slip settles into the task and the line becomes its step.
          over(false);
          const drop = slip.animate(
            [
              { transform: `translate(${x1}px, ${y1}px) rotate(-1.5deg) scale(1)`, opacity: 1 },
              { transform: `translate(${x1}px, ${y1 + 4}px) rotate(0deg) scale(0.92)`, opacity: 0 },
            ],
            { duration: 200, easing: "cubic-bezier(.4,0,.6,1)", fill: "forwards" },
          );
          // (Its old place folds away still dashed; the step comes in faint and fills in as the
          // slip melts into it.)
          tour.nestSample(true);
          await drop.finished.catch(() => {});
          // (Left meanwhile: what the step held is already put back.)
          if (!w.alive()) return;
          lift(false);
          slip.remove();
          slip = null;
          if (!(await w.wait(2600))) return;
          // Back on its own, for the next time round.
          tour.nestSample(false);
          if (!(await w.wait(1100))) return;
        }
      })();
    return () => {
      w.stop();
      slip?.remove();
      over(false);
      lift(false);
      // (Once the task has let go of its "drop here" look, so it doesn't fold away with it.)
      queueMicrotask(() => {
        if (tour.step !== "step") tour.dropParent();
      });
    };
  }

  /** Where a row's "Drop to make it a step" shows (its ::after, measured with the same words
   * and type, as it isn't drawn yet). */
  function dropTag(row: HTMLElement | null): { left: number; bottom: number } | null {
    if (!row?.dataset.dropLabel) return null;
    const m = document.createElement("span");
    m.textContent = row.dataset.dropLabel;
    m.style.cssText = "position:absolute;visibility:hidden;white-space:nowrap;padding:1px 7px;font-size:var(--t-xs);font-weight:600";
    row.appendChild(m);
    const b = m.getBoundingClientRect();
    m.remove();
    const R = row.getBoundingClientRect();
    return { left: R.right - 8 * store.zoom - b.width, bottom: R.bottom + 9 * store.zoom };
  }

  // ---- The pointer ----
  // The veil keeps clicks off the panel; the wheel still scrolls the board, and a floating
  // window can still be moved by its top.
  function down(e: PointerEvent) {
    if (bubbleEl?.contains(e.target as Node)) return;
    e.preventDefault();
    if (store.win.mode !== "floating" || e.button !== 0) return;
    const bar = document.querySelector(".shell .bar")?.getBoundingClientRect();
    if (bar && e.clientY <= bar.bottom) store.backend.startDragging();
  }
  function wheel(e: WheelEvent) {
    // Ctrl+wheel (the note size) waits for the end of the tour, like Ctrl + / Ctrl −.
    if (e.ctrlKey) {
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (bubbleEl?.contains(e.target as Node)) return;
    scroller()?.scrollBy({ top: e.deltaY });
  }

  // ---- The bubble's words ----
  const sIn = (n: Element) => stepIn(n, { dir: tour.dir });
  const sOut = (n: Element) => stepOut(n, { dir: tour.dir });
  const sayOut = (_n: Element) => ({ duration: reduced() ? 90 : 120, css: (t: number) => `position:absolute;inset:0 0 auto 0;opacity:${t.toFixed(3)}` });
  const sayIn = (_n: Element) => ({ delay: reduced() ? 60 : 100, duration: reduced() ? 140 : 200, css: (t: number) => `opacity:${t.toFixed(3)}` });
  let moreKeys = $derived([
    [t("Help and the quick tour"), "F1"],
    [t("Settings"), "Ctrl+Shift+P"],
    [t("Show or hide, from any app"), store.shortcuts.toggle],
    [t("Undo"), "Ctrl+Z"],
  ]);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="tour" bind:this={rootEl} onpointerdown={down} onwheel={wheel} oncontextmenu={(e) => e.preventDefault()}>
  <!-- The veil, cut out where the windows are (redrawn as they glide), and their soft rings. -->
  <div class="veil" class:on={veilOn} aria-hidden="true" out:veil|global={{ duration: 260 }}>
    <svg class="shade" width="100%" height="100%">
      <defs>
        <mask id="tour-cut" maskUnits="userSpaceOnUse">
          <rect width="100%" height="100%" fill="white" />
          <rect bind:this={cutA} fill="black" />
          <rect bind:this={cutB} fill="black" />
          <rect bind:this={cutWasA} fill="black" opacity="0" />
          <rect bind:this={cutWasB} fill="black" opacity="0" />
        </mask>
      </defs>
      <rect class="dim" width="100%" height="100%" mask="url(#tour-cut)" />
      <g class="rings" bind:this={ringsEl}>
        <rect class="glow" bind:this={glowA} />
        <rect class="glow b" bind:this={glowB} />
        <rect class="ring" bind:this={ringA} />
        <rect class="ring b" bind:this={ringB} />
      </g>
      <g class="rings" bind:this={ringsWasEl}>
        <rect class="glow" bind:this={glowWasA} />
        <rect class="glow b" bind:this={glowWasB} />
        <rect class="ring" bind:this={ringWasA} />
        <rect class="ring b" bind:this={ringWasB} />
      </g>
    </svg>
  </div>

  <!-- Placed by the glide (its own transform); the bubble inside rises in and sinks out. -->
  <div class="at" bind:this={bubbleEl}>
  <div class="bubble" role="dialog" aria-modal="true" aria-labelledby="tour-title" aria-describedby="tour-text" transition:rise|global={{ duration: 300, delay: 60 }}>
    <button class="x" aria-label={tour.last ? t("Close the tour") : t("Skip the tour")} data-tip={tour.last ? t("Close the tour") : t("Skip the tour")} data-tip-keys="Esc" onclick={() => tour.close()}><Icon name="close" size={14} stroke={2} /></button>
    <div class="room" bind:this={roomEl} use:easeHeight={{ grow: 300, shrink: 260 }}>
      <div class="stack" bind:this={stackEl}>
        {#key tour.i}
          <div class="body" in:sIn out:sOut>
            <h2 class="title" id="tour-title">{t(words.title)}</h2>
            <p class="text" id="tour-text">
              <!-- (Style's words follow the header as it narrows: the old ones fade, the new come.) -->
              {#key tour.step === "style" ? styleButton.shown : ""}
                <span class="say" in:sayIn out:sayOut>{parts[0]}{#if hasKeys}<kbd>{keysText(STEP_KEYS[tour.step]?.() ?? "")}</kbd>{parts[1]}{/if}</span>
              {/key}
            </p>
            {#if tour.step === "dump"}
              <div class="demo" aria-hidden="true">
                <div class="mini-bar">
                  <Icon name="inbox" size={15} />
                  <span class="typed" class:empty={!typed}>{typed || t("Dump it here")}{#if typed}<span class="caret"></span>{/if}</span>
                  <span class="key" class:pressed>{keysText("Enter")}</span>
                </div>
                <div class="under">
                  <span class="ok" class:on={sent}><span class="tick"><Icon name="check" size={9} stroke={3.4} /></span>{t("Into the Inbox")}</span>
                </div>
              </div>
            {:else if tour.step === "more"}
              <div class="keys">
                {#each moreKeys as [what, keys] (what)}
                  <span class="what">{what}</span><kbd>{keysText(keys)}</kbd>
                {/each}
              </div>
            {/if}
          </div>
        {/key}
      </div>
    </div>
    <div class="foot">
      <div class="dots" aria-hidden="true">
        {#each STEPS as s (s)}<span class="dot"></span>{/each}
        <span class="now" style:transform="translateX({tour.i * 12}px)"></span>
      </div>
      <span class="visually-hidden" aria-live="polite">{t("Step {a} of {b}", { a: tour.i + 1, b: STEPS.length })}</span>
      <div class="btns">
        <button class="btn" onclick={() => tour.back()} data-tip={tour.i ? t("Back") : t("Skip the tour")} data-tip-keys={tour.i ? "Left|Mouse back" : "Esc"}>
          <span class="swap"><span class:on={tour.i === 0}>{t("Skip")}</span><span class:on={tour.i > 0}>{t("Back")}</span></span>
        </button>
        <button class="btn strong" bind:this={nextEl} onclick={() => tour.next()} data-tip={tour.last ? t("Close the tour") : t("Next step")} data-tip-keys="Right|Enter">
          <span class="swap"><span class:on={!tour.last}>{t("Next (tour)")}</span><span class:on={tour.last}>{t("Done (tour)")}</span></span>
        </button>
      </div>
    </div>
  </div>
  </div>
</div>

<style>
  .tour {
    position: absolute;
    inset: 0;
    z-index: 90;
    overflow: hidden;
    border-radius: inherit;
    --tour-veil: rgb(30 24 16 / 0.34);
  }
  :global(:root[data-mode="dark"]) .tour {
    --tour-veil: rgb(0 0 0 / 0.5);
  }
  .veil,
  .shade {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  /* The veil darkens in. */
  .shade {
    overflow: visible;
  }
  .dim {
    fill: var(--tour-veil);
    opacity: 0;
    transition: opacity 0.26s ease-out;
  }
  .veil.on .dim {
    opacity: 1;
  }
  .rings {
    --ring: 0;
    --two: 0;
    fill: none;
  }
  .ring {
    stroke: var(--focus-ring);
    stroke-width: 1.5;
    opacity: calc(var(--ring) * 0.75);
  }
  .glow {
    stroke: var(--focus-ring);
    stroke-width: 4.5;
    opacity: calc(var(--ring) * 0.14);
  }
  .ring.b {
    opacity: calc(var(--two) * 0.75);
  }
  .glow.b {
    opacity: calc(var(--two) * 0.14);
  }
  .at {
    position: absolute;
    left: 0;
    top: 0;
    width: min(340px, calc(100% - 32px));
  }
  .bubble {
    position: relative;
    padding: var(--s4) var(--s4) var(--s3);
    border-radius: var(--r-xl);
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--pop-shadow);
    border: 1px solid var(--line);
  }
  .x {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }
  .x:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .stack {
    position: relative;
    display: grid;
  }
  .stack > .body {
    grid-area: 1 / 1;
  }
  .title {
    margin: 0 32px 6px 0;
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.25;
  }
  .text {
    position: relative;
    margin: 0;
    font-size: var(--t-base);
    line-height: 1.5;
    color: var(--soft);
  }
  kbd {
    display: inline-block;
    padding: 0 6px;
    margin: 0 1px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    background: var(--ground);
    font: inherit;
    font-size: var(--t-sm);
    font-weight: 500;
    line-height: 19px;
    color: var(--soft);
    white-space: nowrap;
  }

  /* The quick dump bar, in small. */
  .demo {
    margin-top: var(--s3);
  }
  .mini-bar {
    height: 44px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 0 12px;
    border-radius: var(--r-lg);
    background: var(--card);
    border: 1px solid var(--line);
    box-shadow: var(--card-shadow), 0 6px 16px rgb(30 24 16 / 0.08);
    color: var(--icon);
  }
  .typed {
    flex: 1;
    min-width: 0;
    font-size: var(--t-lg);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
  }
  .typed.empty {
    color: var(--muted);
  }
  .caret {
    display: inline-block;
    width: 1.5px;
    height: 1.05em;
    margin-left: 1px;
    vertical-align: -0.15em;
    background: var(--ink);
    animation: caret 1s steps(1) infinite;
  }
  @keyframes caret {
    50% {
      opacity: 0;
    }
  }
  .key {
    flex-shrink: 0;
    padding: 2px 7px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    transition: transform 0.12s var(--ease-out), background-color 0.12s;
  }
  .key.pressed {
    transform: translateY(1px);
    border-bottom-width: 1px;
    background: var(--chip);
  }
  .under {
    height: 30px;
    display: flex;
    align-items: center;
    padding: 0 2px;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--t-sm);
    color: var(--soft);
    opacity: 0;
    transform: translateY(-3px);
    transition: opacity 0.16s, transform 0.16s;
  }
  .ok.on {
    opacity: 1;
    transform: none;
    transition: opacity 0.24s 0.06s, transform 0.3s var(--ease-glide) 0.06s;
  }
  .tick {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #6f9c67;
    color: #fff;
    display: grid;
    place-items: center;
  }

  /* Four keys to keep. */
  .keys {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px 12px;
    align-items: center;
    margin-top: var(--s3);
    font-size: var(--t-md);
    color: var(--soft);
  }
  .keys kbd {
    justify-self: end;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    margin-top: var(--s4);
  }
  /* The steps; the one you're on is a longer mark that travels from dot to dot. */
  .dots {
    position: relative;
    display: flex;
    gap: 6px;
    padding-left: 1px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--edge);
  }
  .now {
    position: absolute;
    left: 1px;
    top: 0;
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--ink);
    transition: transform 0.36s var(--ease-glide);
  }
  .btns {
    display: flex;
    gap: 6px;
  }
  .btn {
    height: 30px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    color: var(--ink);
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s;
  }
  .btn:hover {
    background: var(--hover);
  }
  .btn.strong {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .btn.strong:hover {
    background: color-mix(in srgb, var(--ink) 88%, var(--card));
  }
  .btn:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  /* Both words in one place (the button keeps the wider one's width): the old fades before the
     new one comes. */
  .swap {
    display: grid;
  }
  .swap > span {
    grid-area: 1 / 1;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .swap > span.on {
    opacity: 1;
    transition: opacity 0.2s 0.1s;
  }

  /* The slip flown onto the hours (like a carried line). */
  .tour :global(.tour-slip) {
    position: absolute;
    left: 0;
    top: 0;
    max-width: 240px;
    padding: 7px 12px;
    border-radius: 8px;
    background: var(--card);
    border: 1px solid var(--line);
    box-shadow: var(--lift-shadow);
    font-size: var(--t-base);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
    transform-origin: 50% 50%;
    opacity: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .now,
    .ok,
    .ok.on,
    .key {
      transition: none;
    }
    .caret {
      animation: none;
    }
  }
</style>
