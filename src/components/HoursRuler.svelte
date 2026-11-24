<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Today's hours, floating over the top of the notes just under the Inbox strip (App.svelte puts
  // it right after the strip, on the board): nothing on the board ever moves for them. While a
  // task is carried, a quiet strip waits there, "Drop here for a time"; carried onto it (or near
  // it) a moment, it opens into the hours, a card with its shadow over the notes below, and the
  // board dims; carried on down, they fold back into the strip after a short moment
  // (lib/hoursDock.ts). "Pick on today's hours" in a task's menu opens the same card in the same
  // place. Board scrolled so the Inbox is out of sight: it sticks to the top of the board.
  // The card: now, the hours, the timed tasks as dots, and a Tomorrow spot. Drop (or click) on an
  // hour: snapped to the quarter hour, Shift for the exact minute. Holding over Tomorrow opens
  // tomorrow's hours (DragTime / DragAnim).
  import { untrack } from "svelte";
  import { cubicOut } from "svelte/easing";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { HOURS_LIFT, itemDrag } from "../lib/itemdrag.svelte";
  import { formatDueLong, hhmm } from "../lib/time";
  import { hourLabelStep, rulerRange, spanName, timeAt, type Span } from "../lib/plan";
  import { hourMark, i18n, t } from "../lib/i18n.svelte";
  import { pushLayer } from "../lib/layers";
  import { DOCK, NEAR, SLIP_EDGE, STAY, dockAt, dockLift, dockPlace, dockWake, hourLanding, within, type Box, type Dock } from "../lib/hoursDock";
  import { arrive, blip, crumbIn, crumbOut, easeHeight, fadeQuick, glide, leave, reduced } from "../lib/motion";

  // What the ruler is for: the carried task, or the one whose menu asked for it.
  let carried = $derived(itemDrag.item?.kind === "task" ? itemDrag.item : null);
  let subject = $derived(carried ?? store.ruler?.item ?? null);
  let clickMode = $derived(!carried && !!store.ruler && store.ruler.mode !== "demo");
  // The strip while a task is carried: folded, or open into the hours (lib/hoursDock.ts).
  let dock = $state.raw<Dock>(DOCK);
  // The hours are open: carried onto the strip, or asked for from a task's menu. (The quick tour
  // shows them as a carry would, nothing to click: a slip it flies there lands at a fraction of
  // the hours, and the marker names that time.)
  let open = $derived(!!store.ruler || (!!carried && dock.open));

  let range = $derived.by(() => {
    if (!subject) return null;
    const space = store.notes.find((n) => n.id === subject.note_id)?.space ?? store.space;
    return rulerRange(store.now.getTime(), space, store.day, store.workHours);
  });
  // Showing tomorrow's hours (after holding over the Tomorrow spot).
  let onNext = $state(false);
  let span = $derived<Span | null>(range ? (onNext ? range.next : range.main) : null);

  // What the ruler shows. It stays on screen a moment after the carry ends (while the slip
  // lands on its hour, then while it folds back), so it keeps the last thing it showed.
  type View = { span: Span; range: NonNullable<typeof range>; clickMode: boolean; text: string; id: number; onNext: boolean };
  let live = $derived<View | null>(open && subject && span && range ? { span, range, clickMode, text: subject.text, id: subject.id, onNext } : null);
  let kept = $state<View | null>(null);
  $effect(() => {
    if (live) kept = live;
  });
  let view = $derived(live ?? kept);
  // (A slip landing on a new note's card leaves them folded: only one landing on an hour.)
  let showRuler = $derived(!!live || hourLanding(itemDrag.landings));
  // On screen: the strip while a task is carried, the hours while they show. Going, it keeps the
  // face it had (strip or hours) as it fades.
  let shown = $derived(!!carried || showRuler);
  let lastFace: "strip" | "hours" = "strip";
  let face = $derived.by(() => {
    if (shown) lastFace = showRuler && view ? "hours" : "strip";
    return lastFace;
  });
  // The slip resting on the strip, the hours about to open: the strip takes the accent.
  let near = $derived(!!carried && !dock.open && dock.since != null);
  let place = $derived(dockPlace(store.zoom));
  // Opening where there isn't room under the strip for the whole of the hours (a long Inbox, a
  // short window), they rise just enough to show whole; folding, they go back down with it.
  let lift = $state(0);
  $effect(() => {
    if (face !== "hours" || !wrapEl || !dockEl) {
      lift = 0;
      return;
    }
    const board = wrapEl.closest(".scroll");
    if (!board) return;
    untrack(() => {
      const b = board.getBoundingClientRect();
      lift = dockLift(dockEl!.getBoundingClientRect().top + lift, b);
    });
  });

  let trackEl = $state<HTMLDivElement>();
  let tomorrowEl = $state<HTMLDivElement>();
  let hover = $state<number | null>(null);
  let demoAt = $derived(store.ruler?.mode === "demo" && store.ruler.at != null && span ? timeAt(span, store.ruler.at) : null);
  // Where the keyboard has the marker (sliderKey, below).
  let keyAt = $state<number | null>(null);
  /** Where the marker starts: the task's time when it is in these hours, else the next quarter
   * hour from now, else the first hour. */
  let startAt = $derived.by(() => {
    if (!span) return null;
    const due = subject?.due_at;
    if (due != null && due >= span.start && due <= span.end) return due;
    const q = 15 * 60_000;
    const next = Math.ceil(store.now.getTime() / q) * q;
    return next >= span.start && next <= span.end ? next : span.start;
  });
  // (Always inside the hours shown: a place kept from the other day's hours never is.)
  let at = $derived.by(() => {
    const v = hover ?? keyAt ?? startAt;
    return v == null || !span ? v : Math.min(span.end, Math.max(span.start, v));
  });
  // Today's and tomorrow's hours swapped (the pointer rested on Tomorrow): the keyboard's place,
  // and the pointer's, were in the other day's hours; the marker starts again from this day's.
  $effect(() => {
    void onNext;
    untrack(() => {
      keyAt = null;
      hover = null;
    });
  });
  // (The keyboard's place shows too, while the keyboard is on the hours: keysOn, below.)
  let keysOn = $state(false);
  let marked = $derived(hover ?? (keysOn ? at : null) ?? demoAt);
  let overTomorrow = $state(false);
  /** The marker's time, kept whole inside the track: near an end it slides in from its line. */
  let markLabelW = $state(0);
  let markShift = $derived.by(() => {
    const span = view?.span;
    if (marked == null || !span || !trackW || !markLabelW) return 0;
    const x = ((marked - span.start) / (span.end - span.start)) * trackW;
    const half = markLabelW / 2;
    // (Centred on the line, 1 px left of its place: from there when kept in.)
    const c = x - 1;
    return c < half ? half - c : c > trackW - half ? trackW - half - c : 0;
  });
  let holdTimer: ReturnType<typeof setTimeout> | null = null;

  // Hour labels: as many as fit without crowding (every 1, 2, 3… hours on the clock), none
  // spilling past the track's end toward the Tomorrow spot.
  let trackW = $state(0);
  const END_ROOM = 14;
  let hours = $derived.by(() => {
    const span = view?.span;
    if (!span) return [];
    const step = hourLabelStep((span.end - span.start) / 3_600_000, trackW || 400, i18n.hour12 ? 44 : 32);
    const out: { x: number; label: string; major: boolean; named: boolean }[] = [];
    const len = span.end - span.start;
    const first = new Date(span.start);
    first.setMinutes(first.getMinutes() > 0 ? 60 : 0, 0, 0);
    const halves = len / 3_600_000 <= 9;
    for (let t = first.getTime() - (halves ? 1_800_000 : 0); t <= span.end; t += halves ? 1_800_000 : 3_600_000) {
      if (t < span.start) continue;
      const d = new Date(t);
      const x = (t - span.start) / len;
      const major = d.getMinutes() === 0;
      // (Never cut at either end of the track: a 12-hour "4 PM" is wider than "16h".)
      const room = i18n.hour12 ? 18 : END_ROOM;
      const named = major && d.getHours() % step === 0 && (!trackW || (x * trackW >= room && x * trackW <= trackW - Math.max(END_ROOM, room)));
      out.push({ x, label: hourMark(d.getHours()), major, named });
    }
    return out;
  });
  // The carried task has no dot while carried; once dropped on an hour its dot arrives where
  // the slip lands.
  let dots = $derived.by(() => {
    const span = view?.span;
    if (!span) return [];
    return store.timed
      .filter((i) => i.id !== live?.id && (i.due_at ?? 0) >= span.start && (i.due_at ?? 0) <= span.end)
      .map((i) => ({ id: i.id, text: i.text, x: ((i.due_at ?? 0) - span.start) / (span.end - span.start) }));
  });
  let nowX = $derived.by(() => {
    const span = view?.span;
    return span && span.label === "Today" ? Math.max(0, (store.now.getTime() - span.start) / (span.end - span.start)) : null;
  });
  /** The time being picked has its label over the word for now: the word steps aside meanwhile
   * (« maintenant » is long enough to show half of itself from under it). */
  let nowCovered = $derived.by(() => {
    const span = view?.span;
    if (marked == null || nowX == null || !span || !trackW) return false;
    const d = ((marked - span.start) / (span.end - span.start) - nowX) * trackW;
    return d > -32 && d < (i18n.lang === "fr" ? 62 : 26) + 32;
  });

  function fractionAt(clientX: number): number | null {
    if (!trackEl) return null;
    const r = trackEl.getBoundingClientRect();
    if (r.width <= 0) return null;
    return (clientX - r.left) / r.width;
  }

  function inside(el: HTMLElement | undefined, x: number, y: number, pad = 18): boolean {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return x >= r.left - pad && x <= r.right + pad && y >= r.top - pad && y <= r.bottom + pad;
  }

  /** The carried slip rides right of the pointer over the hours, unless its words don't fit
   * there before Tomorrow (or the window's edge): then on whichever side has more room, the
   * pointer's left most often (with some room to spare before it goes back, so it doesn't flick
   * from side to side at the edge). Where the room is less than its words need (a long line in a
   * narrow panel), it is narrowed to the room (its words cut short), so it never goes past 8 px
   * from either edge of the window nor over Tomorrow. Its move from side to side glides (App). */
  function lean(x: number) {
    const t = tomorrowEl?.getBoundingClientRect();
    const want = itemDrag.slip?.full ?? itemDrag.slip?.w ?? 160;
    // (On the right, 2 px more: its tilt.)
    const edge = Math.min(t ? t.left : Infinity, window.innerWidth) - SLIP_EDGE - 2;
    const roomRight = edge - (x + 14 + HOURS_LIFT.x);
    const roomLeft = x - 14 - SLIP_EDGE;
    const flipped = itemDrag.liftX !== HOURS_LIFT.x;
    const fitsRight = want <= roomRight - (flipped ? 24 : 0);
    // (Neither fits: the roomier side, by a margin before it changes sides.)
    const onLeft = !fitsRight && (want <= roomLeft || roomLeft - roomRight > (flipped ? -24 : 24));
    const w = Math.max(SLIP_MIN, Math.min(want, onLeft ? roomLeft : roomRight));
    const cap = w < want ? Math.floor(w) : null;
    const to = onLeft ? -(w + 28) : HOURS_LIFT.x;
    if (itemDrag.cap !== cap) itemDrag.cap = cap;
    if (itemDrag.liftX !== to) itemDrag.liftX = to;
  }

  /** How narrow the carried slip may get (and SLIP_EDGE: how near the window's edges it may go). */
  const SLIP_MIN = 64;

  function track(x: number, y: number, exact: boolean) {
    if (!span) return;
    if (carried) lean(x);
    if (inside(tomorrowEl, x, y, 6)) {
      hover = null;
      // Dropped right on it: the next day's first hour. (While tomorrow shows, it leads back.)
      itemDrag.time = carried && !onNext ? range!.next.start : null;
      setTimeAt(carried && !onNext && tomorrowEl ? center(tomorrowEl) : null);
      if (!overTomorrow) {
        overTomorrow = true;
        holdTimer = setTimeout(() => {
          onNext = !onNext;
          overTomorrow = false;
          // Still over the spot, which now leads the other way: a drop there means what it says.
          if (carried) {
            itemDrag.time = onNext ? null : range!.next.start;
            itemDrag.timeAt = onNext || !tomorrowEl ? null : center(tomorrowEl);
          }
        }, 500);
      }
      return;
    }
    if (overTomorrow) {
      overTomorrow = false;
      if (holdTimer) clearTimeout(holdTimer);
    }
    const f = fractionAt(x);
    if (f == null || !inside(trackEl?.parentElement ?? undefined, x, y)) {
      hover = null;
      if (carried) {
        itemDrag.time = null;
        itemDrag.timeAt = null;
      }
      return;
    }
    hover = timeAt(span, f, exact);
    if (carried) {
      itemDrag.time = hover;
      const r = trackEl!.getBoundingClientRect();
      setTimeAt({ x: r.left + ((hover - span.start) / (span.end - span.start)) * r.width, y: r.top + r.height / 2 });
    }
  }
  /** (Only a new place is given: the same one as a new object on every move would have what
   * shows it worked out again for nothing.) */
  function setTimeAt(p: { x: number; y: number } | null) {
    const c = itemDrag.timeAt;
    if (c === p || (c && p && c.x === p.x && c.y === p.y)) return;
    itemDrag.timeAt = p;
  }

  function center(el: HTMLElement) {
    const r = el.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  }

  // The strip (folded) or the open hours, as the pointer sees them: the open hours' whole place
  // as soon as they open (the card still growing into it).
  let dockEl = $state<HTMLDivElement>();
  let stackEl = $state<HTMLDivElement>();
  function dockBox(): Box | null {
    if (!dockEl) return null;
    const b = dockEl.getBoundingClientRect();
    if (!dock.open || !stackEl) return b;
    return { left: b.left, top: b.top, right: b.right, bottom: Math.max(b.bottom, stackEl.getBoundingClientRect().bottom) };
  }

  // While carried: the strip opens into the hours when the slip rests on it, they fold when it
  // moves away (each a moment later, also with the pointer still), and over the open hours they
  // follow the pointer.
  $effect(() => {
    if (!carried) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const see = (x: number, y: number) => {
      if (timer) clearTimeout(timer);
      timer = null;
      const was = dock;
      const box = dockBox();
      const inside = within(box, x, y, was.open ? STAY : NEAR);
      itemDrag.dockTop = box ? box.top : null;
      // (On or near them, a drop never makes a new note from the board's empty space: itemdrag.)
      itemDrag.atDock = inside;
      const d = dockAt(was, inside, performance.now());
      if (d !== was) dock = d;
      if (was.open && !d.open) {
        // Folded: nothing of the hours is under the slip any more.
        hover = null;
        itemDrag.time = null;
        itemDrag.timeAt = null;
      }
      const wake = dockWake(d);
      if (wake != null) timer = setTimeout(() => see(itemDrag.x, itemDrag.y), Math.max(0, wake - performance.now()) + 4);
    };
    const move = (e: PointerEvent) => {
      see(e.clientX, e.clientY);
      // (Opened on this move: the hours aren't laid out yet, and only a move over them gives a time.)
      if (dock.open) track(e.clientX, e.clientY, e.shiftKey);
    };
    window.addEventListener("pointermove", move);
    // Where the slip is as the strip shows (the moves that started the carry came before this):
    // picked up off the strip, the strip is ready at once for a slip carried straight onto it.
    const first = requestAnimationFrame(() => see(itemDrag.x, itemDrag.y));
    return () => {
      window.removeEventListener("pointermove", move);
      cancelAnimationFrame(first);
      if (timer) clearTimeout(timer);
      dock = DOCK;
      itemDrag.atDock = false;
      itemDrag.dockTop = null;
    };
  });

  // The hours opening (or folding) cover (or bare) the notes under a pointer that may be still:
  // while they do, what is under the pointer is looked at again each frame. An hour or Tomorrow is
  // only taken once the pointer moves: letting go where the hours just opened, without moving,
  // must not give it a time.
  let wrapEl = $state<HTMLDivElement>();
  $effect(() => {
    if (!carried || !wrapEl) return;
    const el = wrapEl;
    let raf = 0;
    // (Only when the card's height changes is what is under the pointer looked at again, not every
    // frame it fades.)
    let seenH = -1;
    const moving = () =>
      el.getAnimations({ subtree: true }).some((a) => a.playState === "running" && Number.isFinite(Number(a.effect?.getComputedTiming().endTime)));
    const loop = () => {
      raf = 0;
      if (!itemDrag.item) return;
      const h = el.getBoundingClientRect().height;
      if (h !== seenH) {
        seenH = h;
        itemDrag.recheck();
      }
      if (moving()) raf = requestAnimationFrame(loop);
    };
    const kick = () => {
      if (!raf) raf = requestAnimationFrame(loop);
    };
    const ro = new ResizeObserver(kick);
    ro.observe(el);
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  });

  $effect(() => {
    if (!open) {
      untrack(() => {
        onNext = false;
        hover = null;
        overTomorrow = false;
        if (holdTimer) clearTimeout(holdTimer);
        holdTimer = null;
      });
    }
  });

  // "Pick on today's hours" (from a task's menu): Esc closes, a click sets the time.
  $effect(() => {
    if (!clickMode) return;
    return pushLayer(() => (store.ruler = null));
  });

  /** The strip (or the hours asked for from a menu) coming: it comes down a touch from the Inbox
   * above it and fades in; going, it fades back up the same way, a little faster. */
  function dockIn(_node: Element) {
    if (reduced()) return { duration: 120, css: (t: number) => `opacity:${t}` };
    return { duration: 240, easing: glide, css: (t: number, u: number) => `opacity:${t.toFixed(3)};transform:translateY(${(-u * 6).toFixed(2)}px)` };
  }
  function dockOut(_node: Element) {
    if (reduced()) return { duration: 90, css: (t: number) => `opacity:${t}` };
    return { duration: 170, easing: cubicOut, css: (t: number, u: number) => `opacity:${t.toFixed(3)};transform:translateY(${(-u * 4).toFixed(2)}px)` };
  }

  async function choose(e: MouseEvent) {
    if (!clickMode || !store.ruler) return;
    if (inside(tomorrowEl, e.clientX, e.clientY, 4)) return tomorrow();
    if (hover == null) return;
    await setTo(hover);
  }
  async function setTo(at: number) {
    if (!store.ruler) return;
    const it = store.items.find((x) => x.id === store.ruler!.item.id) ?? store.ruler.item;
    store.ruler = null;
    await store.setDue(it, at);
  }
  /** The Tomorrow spot clicked (or pressed): tomorrow's first hour; while tomorrow's hours show,
   * back to today's. */
  async function tomorrow() {
    if (overTomorrow && holdTimer) clearTimeout(holdTimer);
    overTomorrow = false;
    if (onNext) onNext = false;
    else await setTo(range!.next.start);
  }

  // ---- The keyboard ("Pick on today's hours" chosen from the keyboard) ----
  // The hours are a slider: it takes the keyboard as the ruler opens, ← / → move the marker by
  // 15 minutes (Shift: an hour), Home / End go to the first / last hour, Enter sets the time; Tab
  // goes on to Tomorrow and ×. Closing gives the keyboard back to the task's ⋯.
  function sliderKey(e: KeyboardEvent) {
    if (!clickMode || !span || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "Enter") {
      e.preventDefault();
      if (at != null) void setTo(at);
      return;
    }
    const step = (e.shiftKey ? 60 : 15) * 60_000;
    const from = Math.min(span.end, Math.max(span.start, at ?? span.start));
    let to: number;
    if (e.key === "ArrowRight" || e.key === "ArrowUp") to = from + step;
    else if (e.key === "ArrowLeft" || e.key === "ArrowDown") to = from - step;
    else if (e.key === "PageUp") to = from + 60 * 60_000;
    else if (e.key === "PageDown") to = from - 60 * 60_000;
    else if (e.key === "Home") to = span.start;
    else if (e.key === "End") to = span.end;
    else return;
    e.preventDefault();
    // (On the quarter hour, whatever the pointer left it at.)
    const q = 15 * 60_000;
    to = Math.min(span.end, Math.max(span.start, Math.round(to / q) * q));
    keyAt = to;
    hover = to;
  }
  $effect(() => {
    if (!clickMode || !trackEl) return;
    const el = trackEl;
    const id = untrack(() => store.ruler?.item.id);
    el.focus({ preventScroll: true });
    return () => {
      keyAt = null;
      // (Back to the task's ⋯ when the keyboard was here; nowhere if that is gone.)
      const a = document.activeElement;
      if (a && a !== document.body && !el.parentElement?.closest(".ruler")?.contains(a)) return;
      const back = document.querySelector<HTMLElement>(`[data-item-menu="${id}"]`);
      if (back && !back.closest("[inert]")) back.focus({ preventScroll: true });
      else (a as HTMLElement | null)?.blur?.();
    };
  });
</script>

<!-- Where the strip hangs: right after the Inbox strip, taking no room (no height); it sticks to
     the board's top when the Inbox is scrolled away (and, a long Inbox running off the bottom,
     high enough for the hours to open whole). At the notes' size (zoom) undone: the hours keep
     their own size. -->
<div class="place" style:zoom={1 / store.zoom} style:top="{place.stick}px" style:bottom="{place.bottom}px">
  <!-- The board dims a touch while the hours show (only then), so they stand out. -->
  <div class="veil" class:on={showRuler} aria-hidden="true"></div>
  {#if shown}
    <div class="float" data-drop-none style:top="{place.drop}px" style:translate={lift ? `0 ${-lift}px` : null} bind:this={wrapEl} in:dockIn out:dockOut>
      <div class="dock" class:open={face === "hours"} class:near class:click={face === "hours" && !!view?.clickMode} bind:this={dockEl} use:easeHeight={{ grow: 300, shrink: 240 }}>
        <div class="stack" bind:this={stackEl}>
          {#if face === "strip" || !view}
            <div class="strip" in:arrive={{ delay: 70, duration: 200, y: -2 }} out:leave={{ duration: 110, y: -2 }}>
              <Icon name="clock" size={15} stroke={1.9} />
              <span class="lead">{t("Drop here for a time")}</span>
            </div>
          {:else}
            {@const v = view}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div
              class="ruler"
              in:arrive={{ delay: 60, duration: 240, y: -3 }}
              out:leave={{ duration: 120, y: -3 }}
              onpointermove={(e) => clickMode && track(e.clientX, e.clientY, e.shiftKey)}
              onpointerleave={() => clickMode && (hover = null)}
              onclick={choose}
            >
              <div class="top">
                <span class="lead">{v.clickMode ? t("When for “{text}”?", { text: v.text }) : t("Drop on an hour")}</span>
                <span class="hint">
                  {#key v.span.label}
                    <span in:crumbIn out:crumbOut>{v.span.label === "Today" ? "" : `${spanName(v.span.label)} · `}{t("snaps to 15 min · Shift for exact")}</span>
                  {/key}
                </span>
                {#if v.clickMode}
                  <button class="x" aria-label={t("Close the hours")} data-tip={t("Close")} data-tip-keys="Esc" onclick={(e) => ((e.stopPropagation(), (store.ruler = null)))}><Icon name="close" size={14} stroke={2.2} /></button>
                {/if}
              </div>
              <div class="lane">
                <!-- svelte-ignore a11y_no_noninteractive_tabindex (the hours are a slider while they are picked) -->
                <div
                  class="track"
                  bind:this={trackEl}
                  bind:clientWidth={trackW}
                  role={v.clickMode ? "slider" : undefined}
                  tabindex={v.clickMode ? 0 : undefined}
                  aria-label={v.clickMode ? t("When for “{text}”?", { text: v.text }) : undefined}
                  aria-valuemin={v.clickMode ? 0 : undefined}
                  aria-valuemax={v.clickMode ? Math.round((v.span.end - v.span.start) / 60_000) : undefined}
                  aria-valuenow={v.clickMode && at != null ? Math.round((at - v.span.start) / 60_000) : undefined}
                  aria-valuetext={v.clickMode && at != null ? formatDueLong(at, store.now) : undefined}
                  aria-keyshortcuts={v.clickMode ? "Enter" : undefined}
                  onkeydown={sliderKey}
                  onfocus={(e) => (keysOn = (e.currentTarget as HTMLElement).matches(":focus-visible"))}
                  onblur={() => (keysOn = false)}
                >
                  <div class="axis"></div>
                  <!-- Tomorrow's hours (and back): the old hours slide away as the new ones come in. -->
                  {#key v.onNext}
                    <div class="hours" in:arrive={{ delay: 110, duration: 260, y: 0 }} out:leave={{ duration: 130, y: 0 }}>
                      {#if nowX != null}
                        <span class="now" style:left="{nowX * 100}%"><span class="now-label" class:wordy={i18n.lang === "fr"} class:covered={nowCovered}>{t("now")}</span></span>
                      {/if}
                      {#each hours as h (h.x)}
                        <span class="tick" class:major={h.major} style:left="{h.x * 100}%"></span>
                        {#if h.named}<span class="hour" style:left="{h.x * 100}%">{h.label}</span>{/if}
                      {/each}
                      {#each dots as d (d.id)}
                        <span class="dot" style:left="{d.x * 100}%" data-tip={d.text} data-tip-drag in:blip={{ delay: 170 }} out:blip></span>
                      {/each}
                    </div>
                  {/key}
                  {#if marked != null && v.span}
                    <span class="marker" style:left="{((marked - v.span.start) / (v.span.end - v.span.start)) * 100}%" transition:fadeQuick={{ duration: 120 }}><span class="marker-label" bind:offsetWidth={markLabelW} style:margin-left="{markShift}px">{hhmm(new Date(marked))}</span></span>
                  {/if}
                </div>
                <!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_tabindex (a button while the hours are picked) -->
                <div
                  class="tomorrow"
                  class:over={overTomorrow}
                  class:back={v.onNext}
                  bind:this={tomorrowEl}
                  role={v.clickMode ? "button" : undefined}
                  tabindex={v.clickMode ? 0 : undefined}
                  aria-label={v.clickMode ? (v.onNext ? t("Back to today") : formatDueLong(v.range.next.start, store.now)) : undefined}
                  onkeydown={(e) => {
                    if (!v.clickMode || (e.key !== "Enter" && e.key !== " ")) return;
                    e.preventDefault();
                    void tomorrow();
                  }}
                  data-tip-drag
                  data-tip={v.onNext ? t("Back to today") : t("Drop: {when} · hold to see its hours", { when: formatDueLong(v.range.next.start, store.now) })}>
                  {#if overTomorrow}<span class="fill" aria-hidden="true" out:fadeQuick={{ duration: 160 }}></span>{/if}
                  {#key v.onNext}
                    <span class="name" in:crumbIn out:crumbOut>{v.onNext ? t("Today") : spanName(v.range.next.label)}</span>
                  {/key}
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Right after the Inbox strip, on the board: no height, so nothing moves for it; over the notes
     (and the notes' menus). Sticky: with the Inbox scrolled out of sight, it stays at the top of
     the board (its top and bottom are set from lib/hoursDock.ts). */
  .place {
    position: sticky;
    z-index: 26;
    height: 0;
  }
  /* The dim: a spot whose shadow spreads over the whole board (cut at the board's edges, and never
     making the board longer to scroll), under the hours. It fades in only while they show. */
  .veil {
    position: absolute;
    left: 50%;
    top: 0;
    width: 1px;
    height: 1px;
    box-shadow: 0 0 0 150vmax rgb(28 22 14 / 0.07);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.26s var(--ease-glide);
    will-change: opacity;
  }
  :global(:root[data-mode="dark"]) .veil {
    box-shadow: 0 0 0 150vmax rgb(0 0 0 / 0.24);
  }
  .veil.on {
    opacity: 1;
    transition-duration: 0.32s;
  }
  .float {
    position: absolute;
    left: 0;
    right: 0;
    transition: translate 0.3s var(--ease-glide);
  }

  /* ---- The strip, and the hours it opens into ---- */
  /* Folded: a quiet dashed strip, on paper (what it floats over never shows through). Open: the
     hours' card, in the accent, with its shadow over the notes below. Its height eases from one
     to the other (easeHeight). */
  .dock {
    position: relative;
    border: 1.5px solid transparent;
    border-radius: 14px;
    background: var(--card);
    box-shadow: 0 1px 2px rgb(30 24 16 / 0.06), 0 4px 14px rgb(30 24 16 / 0.08);
    user-select: none;
    transition:
      background-color 0.22s var(--ease-glide),
      border-color 0.22s var(--ease-glide),
      box-shadow 0.26s var(--ease-glide);
  }
  /* (The dashes: a border can't fade from dashed to solid, so they are a layer of their own.) */
  .dock::before {
    content: "";
    position: absolute;
    inset: -1.5px;
    border: 1.5px dashed var(--box);
    border-radius: inherit;
    pointer-events: none;
    transition:
      opacity 0.2s var(--ease-glide),
      border-color 0.15s;
  }
  .dock.near {
    background: color-mix(in srgb, var(--accent) 7%, var(--card));
  }
  .dock.near::before {
    border-color: var(--accent);
  }
  .dock.open {
    background: var(--card);
    border-color: var(--accent);
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--accent) 16%, transparent),
      var(--pop-shadow);
  }
  .dock.open::before {
    opacity: 0;
  }
  .dock.click {
    cursor: pointer;
  }
  /* The strip and the hours share one place: the one leaving steps out of the flow (`leave`), so
     the card's height eases straight to the newcomer's. */
  .stack {
    position: relative;
    display: grid;
    /* (Its width is the board's, never its content's: a narrow panel keeps the hours inside.) */
    grid-template-columns: minmax(0, 1fr);
  }
  .stack > * {
    grid-area: 1 / 1;
  }
  .lead {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-lg);
    color: var(--muted);
    flex-shrink: 0;
  }
  .strip {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    height: 26px;
    padding: 0 var(--s4) 0 12px;
    color: var(--icon);
  }
  .strip .lead {
    font-size: 14px;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 0.15s;
  }
  .dock.near .strip {
    color: var(--accent);
  }
  .dock.near .strip .lead {
    color: var(--soft);
  }

  /* ---- The hours ---- */
  .ruler {
    padding: var(--s3) var(--s4) 14px;
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: var(--s3);
  }
  .top .lead {
    flex: 1;
    min-width: 0;
    color: var(--soft);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    display: grid;
    justify-items: end;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
  }
  .hint > span {
    grid-area: 1 / 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .top .hint {
    min-width: 0;
    flex-shrink: 1;
  }
  /* A narrow panel: the ruler keeps its question and its hours; the snapping hint goes. */
  @container shell (max-width: 480px) {
    .top .hint {
      display: none;
    }
  }
  .x {
    align-self: center;
    width: 26px;
    height: 26px;
    margin: -4px -6px -4px 0;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
  }
  .lane {
    display: flex;
    gap: 14px;
    margin-top: 30px;
    height: 44px;
  }
  .track {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  /* (The hours as a slider, the keyboard on them: the ring rounds like the cards'.) */
  .track:focus-visible {
    border-radius: var(--r-md);
  }
  .hours {
    position: absolute;
    inset: 0;
  }
  .axis {
    position: absolute;
    left: 0;
    right: 0;
    top: 20px;
    height: 2px;
    border-radius: 1px;
    background: var(--line);
  }
  .now {
    position: absolute;
    top: 12px;
    width: 2px;
    height: 18px;
    margin-left: -1px;
    border-radius: 1px;
    background: #e9a93a;
  }
  .now-label {
    position: absolute;
    left: -2px;
    top: -30px;
    font-size: var(--t-xs);
    font-weight: 600;
    color: #9a6100;
    transition: opacity 0.12s ease;
  }
  .now-label.covered {
    opacity: 0;
  }
  /* (The same amber as the line, readable on the dark card.) */
  :global(:root[data-mode="dark"]) .now-label {
    color: #e9a93a;
  }
  /* A narrow ruler (a window under 480 px): a long word for now (« maintenant ») gives way to an amber dot where the word sits, over the line
     (still read out: the word is there, at no size). */
  @container shell (max-width: 455px) {
    .now-label.wordy {
      top: -24px;
      left: -2px;
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: #e9a93a;
      font-size: 0;
      overflow: hidden;
    }
  }
  .tick {
    position: absolute;
    top: 16px;
    width: 1px;
    height: 10px;
    background: var(--edge);
  }
  .tick.major {
    top: 14px;
    height: 14px;
    background: var(--box);
  }
  .hour {
    position: absolute;
    top: 32px;
    transform: translateX(-50%);
    font-size: var(--t-xs);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    position: absolute;
    top: 15px;
    width: 12px;
    height: 12px;
    margin-left: -6px;
    border-radius: 50%;
    background: var(--box);
    box-shadow: 0 0 0 2px var(--card);
  }
  .marker {
    position: absolute;
    top: 4px;
    width: 2px;
    /* (Down to the ticks' foot, 28 px: never through the hour names under them, 32 px down.) */
    height: 24px;
    margin-left: -1px;
    border-radius: 1px;
    /* (In the ink of its time chip above it: one marker.) */
    background: var(--ink);
    pointer-events: none;
  }
  .marker-label {
    position: absolute;
    left: 0;
    top: -22px;
    transform: translateX(-50%);
    padding: 2px 7px;
    border-radius: 6px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-sm);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tomorrow {
    position: relative;
    width: 96px;
    flex-shrink: 0;
    overflow: hidden;
    border: 1.5px dashed var(--box);
    border-radius: var(--r-md);
    display: grid;
    place-items: center;
    font-size: var(--t-sm);
    color: var(--soft);
    transition: border-color 0.15s, background-color 0.15s;
  }
  .tomorrow.over {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .tomorrow .name {
    position: relative;
    grid-area: 1 / 1;
  }
  .fill {
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    transform-origin: 0 50%;
    animation: hold 0.5s linear;
  }
  @keyframes hold {
    from {
      transform: scaleX(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      animation: none;
    }
  }
</style>
