<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A repeating task's history (its menu's "History…"), with a short line in the rule's own
  // units. A daily task (weekdays, every few days) shows a month's calendar, a weekly one a year
  // of weeks, a monthly one a year of months, all three alike: filled when done (a week in part
  // when only some of its days were), a deeper chip when missed, a ring when still to come, a
  // dot between two of an every-2 rule (in the years); today, this week or month outlined. Its
  // rule changed while it is open (an Undo), it shows the new one's view. Opens where its menu
  // was (above the task when a short board has more room there than under it, the board scrolling
  // a little so all of it shows); Back goes back to the menu, Esc (or ×) closes it. ‹ › (or Page Up / Page Down) change
  // the month or the year, from its first one to this one. A year's squares are one Tab stop;
  // the arrows, Home and End go between them.
  import { tick, untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { repeatLabel } from "../lib/repeat";
  import { doneDays, dueDays, markWords, monthCells, monthGrid, seriesStart, summary, viewOf, weekCells, weekNames, weekYear, type Cell, type Unit } from "../lib/doneLog";
  import { dayKey } from "../lib/plan";
  import { pushLayer } from "../lib/layers";
  import { closeOnLeave, modal } from "../lib/modal";
  import { crumbIn, crumbOut, popSwap, reduced, stepIn, stepOut } from "../lib/motion";
  import { cap, dateFmt, dayMonth, dayMonthLong, dayName, i18n, monthShort, ordinal, t, type Key } from "../lib/i18n.svelte";
  import { atPointer, type Point } from "../lib/rightclick.svelte";
  import type { Item } from "../lib/types";

  // (`swap`: which way it and the menu are taking each other's place, for its motion.)
  let { item, at = null, swap, onBack, onClose }: { item: Item; at?: Point | null; swap?: () => number; onBack: () => void; onClose: () => void } = $props();
  const motion = { y: -4 };

  let cardEl: HTMLDivElement;
  let prevEl: HTMLButtonElement;
  let viewsEl: HTMLDivElement;
  let nextEl: HTMLButtonElement;
  /** The first day its history knows (the calendar goes back to its month, or its year). */
  const start = untrack(() => seriesStart(item, store.logOf(item.id)));
  /** A month of days (daily, weekdays, every few days), a year of weeks (weekly), a year of
   * months (monthly): its rule's, as it is now. */
  let view = $derived(viewOf(item.repeat));
  /** The month a day is in, as months since year 0; or its year (a week's: its Thursday's). */
  const indexOf = (d: Date) => (view === "days" ? d.getFullYear() * 12 + d.getMonth() : view === "weeks" ? weekYear(d) : d.getFullYear());
  /** The month or year shown (from its first one to this one: open past midnight at a month's
   * end, the new one comes). */
  let last = $derived(indexOf(store.now));
  let first = $derived(Math.min(indexOf(start), last));
  let shown = $state(untrack(() => Math.max(first, last)));
  /** Which way the month moved (the grid slides that way). */
  let dir = $state(1);
  // (Showing this month as midnight brings the next one: the new one comes, as a move on, the
  // keyboard still on its square. Its rule changed to another view while it is open, an Undo of
  // a rule edit: the new view comes, at this month or year. Before the page is drawn, so the
  // view or the month arriving is drawn once, as it is, the way it moves.)
  let lastSeen = untrack(() => last);
  let viewSeen = untrack(() => view);
  $effect.pre(() => {
    const now = last;
    const v = view;
    untrack(() => {
      const here = document.activeElement as HTMLElement | null;
      const on = here && cardEl?.contains(here) && here.dataset.i != null;
      if (v !== viewSeen) {
        viewSeen = v;
        lastSeen = now;
        dir = 1;
        cur = -1;
        shown = now;
        // (Its squares go: the card keeps the keyboard, never the page.)
        if (on) cardEl.focus({ preventScroll: true });
        return;
      }
      if (now === lastSeen) return;
      if (shown === lastSeen) {
        dir = now > lastSeen ? 1 : -1;
        shown = now;
        if (on) void tick().then(() => focusCell(cur));
      }
      lastSeen = now;
    });
  });
  let year = $derived(view === "days" ? Math.floor(shown / 12) : shown);
  let month = $derived(view === "days" ? shown % 12 : 0);
  let title = $derived(view === "days" ? cap(dateFmt({ month: "long", year: "numeric" }).format(new Date(year, month, 15))) : dateFmt({ year: "numeric" }).format(new Date(year, 6, 1)));
  let weeks = $derived(view === "days" ? monthGrid(year, month) : []);
  let log = $derived(store.logOf(item.id));
  let done = $derived(doneDays(log, item.id));
  let due = $derived(view === "days" ? dueDays(item, new Date(year, month, 1), new Date(year, month + 1, 0), start, log, store.now) : new Set<string>());
  let todayKey = $derived(dayKey(store.now));
  let sum = $derived(summary(item, log, store.now.getTime()));
  // Monday first, in both languages (as the repeat's days).
  const order = [1, 2, 3, 4, 5, 6, 0];

  /** A year's weeks or months, each with how it went. */
  let cells = $derived(view === "weeks" ? weekCells(item, log, year, start, store.now) : view === "months" ? monthCells(item, log, year, start, store.now) : []);
  /** Its rows: a quarter of weeks each (13; a year's 53rd week ends the last one), or half a
   * year of months. */
  let per = $derived(view === "weeks" ? 13 : 6);
  let rows = $derived.by(() => {
    const n = view === "weeks" ? 4 : view === "months" ? 2 : 0;
    return Array.from({ length: n }, (_, r) => ({ at: r * per, cells: cells.slice(r * per, r === n - 1 ? cells.length : (r + 1) * per) }));
  });
  let cols = $derived(view === "weeks" ? Math.max(13, cells.length - 3 * 13) : 6);
  let offs = $derived(cells.some((c) => c.state === "off"));

  /** `at`: the square the keyboard goes to in the year coming (else the one it was on, if it was
   * on one; Infinity: the last). */
  async function go(by: number, at?: number) {
    const to = Math.min(last, Math.max(first, shown + by));
    if (to === shown) return;
    const here = document.activeElement as HTMLElement | null;
    const on = here && cardEl.contains(here) && here.dataset.i != null ? Number(here.dataset.i) : null;
    // (The arrow that goes out at the end keeps no keyboard: the card takes it, never the page.)
    const out = (to === first && here === prevEl) || (to === last && here === nextEl);
    if (out) cardEl.focus({ preventScroll: true });
    dir = by > 0 ? 1 : -1;
    shown = to;
    const want = at ?? on;
    if (want == null) return;
    await tick();
    focusCell(want);
  }

  // ---- A year's squares: one Tab stop, the arrows between them ----

  /** The square the keyboard was last on (-1: none yet; then this week or month, else the first). */
  let cur = $state(-1);
  let roving = $derived.by(() => {
    if (!cells.length) return -1;
    if (cur >= 0) return Math.min(cur, cells.length - 1);
    return Math.max(0, cells.findIndex((c) => c.current));
  });
  function focusCell(i: number) {
    // (Only the year arriving: the one leaving is still on the page for a moment.)
    const all = cardEl.querySelectorAll<HTMLElement>(`.year[data-year="${shown}"] [data-i]`);
    if (!all.length) return;
    const j = Math.max(0, Math.min(i, all.length - 1));
    cur = j;
    all[j].focus({ preventScroll: true });
    if (squeezed) all[j].scrollIntoView({ block: "nearest" });
  }
  function cellKeys(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (el.dataset.i == null || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    const i = Number(el.dataset.i);
    const r = Math.min(Math.floor(i / per), rows.length - 1);
    const c = i - rows[r].at;
    const end = (k: number) => rows[k].at + rows[k].cells.length - 1;
    let to: number | null = null;
    switch (e.key) {
      case "ArrowLeft":
        if (i > 0) to = i - 1;
        else if (shown > first) void go(-1, Infinity);
        break;
      case "ArrowRight":
        if (i < cells.length - 1) to = i + 1;
        else if (shown < last) void go(1, 0);
        break;
      case "ArrowUp":
        if (r > 0) to = Math.min(rows[r - 1].at + c, end(r - 1));
        break;
      case "ArrowDown":
        if (r < rows.length - 1) to = Math.min(rows[r + 1].at + c, end(r + 1));
        break;
      case "Home":
        to = rows[r].at;
        break;
      case "End":
        to = end(r);
        break;
      default:
        return;
    }
    e.preventDefault();
    if (to != null) focusCell(to);
  }

  // What each square says (its tooltip, and to a screen reader): "17–23 Nov · Done Monday,
  // missed Thursday", "March · Done on the 15th".
  /** A day of the month in a sentence: "the 15th", « le 15 » (« le 1er »). */
  const nth = (n: number) => (i18n.lang === "fr" ? (n === 1 ? "1er" : String(n)) : ordinal(n));
  function weekWords(c: Cell): string {
    const span = c.from.getMonth() === c.to.getMonth() ? `${c.from.getDate()}–${dayMonth(c.to)}` : `${dayMonth(c.from)}\u00a0– ${dayMonth(c.to)}`;
    const what = c.state === "off" ? t("Not this week") : markWords(c, "week", (d) => dayName(d.getDay(), true));
    return what ? `${span} · ${what}` : span;
  }
  function monthWords(c: Cell): string {
    const name = cap(dateFmt({ month: "long" }).format(c.from));
    const what = c.state === "off" ? t("Not this month") : markWords(c, "month", (d) => nth(d.getDate()));
    return what ? `${name} · ${what}` : name;
  }

  /** How tight it is laid out (0: at rest; 1: the month or year closer; 2: also its name and rule
   * on one line, without the key; 3: also its days or squares a little smaller) to fit the
   * board's whole height (`atPointer`'s cap), its line at the bottom in view. Chosen as it opens
   * (and for a new view), room kept for the tallest month (six weeks; a year's rows are always as
   * many): it never jumps from one month to the next. */
  let fit = $state(0);
  /** Even at its tightest, taller than the board (a very short window): only its days, weeks or
   * months scroll inside it, their edges fading; its name, month and line stay in sight. */
  let squeezed = $state(false);
  /** Opened under its ⋯ where the board has too little room under the task for the whole card
   * but enough above it: it opens above the task (else under it, the board scrolling a little).
   * Chosen once, as it opens. */
  let up = $state(false);
  let placed = false;
  // (Laid out before it is first drawn: it comes in where it stays, as tight as it stays.)
  $effect(() => {
    void view;
    untrack(() => {
      const cap = parseFloat(cardEl.style.maxHeight);
      if (!(cap > 0)) return;
      delete cardEl.dataset.squeezed;
      let level = 0;
      for (level of [0, 1, 2, 3]) {
        cardEl.dataset.fit = String(level);
        // (Its foot's padding is tighter past 0: its fade there, app.css `[data-capped]`, with it.)
        cardEl.style.setProperty("--cap-pad-bottom", getComputedStyle(cardEl).paddingBottom);
        if (cardEl.scrollHeight <= cap + 0.5) break;
      }
      fit = level;
      squeezed = cardEl.scrollHeight > cap + 0.5;
      if (squeezed) cardEl.dataset.squeezed = "";
      if (!placed) {
        placed = true;
        up = opensUp();
        if (up) {
          cardEl.dataset.up = "";
          motion.y = 4;
        }
      }
      reveal();
      void tick().then(() => {
        // (Squeezed: its days scrolled to show today, this week or this month.)
        const now = squeezed && viewsEl?.querySelector<HTMLElement>(".day.today, .sq.current");
        if (now) {
          const v = viewsEl.getBoundingClientRect();
          const r = now.getBoundingClientRect();
          if (r.bottom > v.bottom - 6) viewsEl.scrollTop += r.bottom - v.bottom + 6;
        }
        edges();
      });
    });
  });
  /** Under its ⋯ (not at the pointer): above the task instead, when all of it fits there and not
   * under it. */
  function opensUp(): boolean {
    if (at) return false;
    const sc = cardEl.closest<HTMLElement>(".scroll");
    const host = cardEl.offsetParent as HTMLElement | null;
    if (!sc || !host) return false;
    const box = sc.getBoundingClientRect();
    const r = host.getBoundingClientRect();
    const z = host.offsetHeight ? r.height / host.offsetHeight : 1;
    // (Under: from 30 px down the line, as its CSS puts it; above: up from 2 px into it.)
    const below = box.bottom - 8 - (r.top + 30 * z);
    const above = r.top + 2 * z - (box.top + 8);
    const h = cardEl.offsetHeight * z;
    return h > below && h <= above;
  }
  /** The board scrolls just enough to show all of it, 8 px clear of its edges (its top first). */
  function reveal() {
    const sc = cardEl.closest<HTMLElement>(".scroll");
    const host = cardEl.offsetParent as HTMLElement | null;
    if (!sc || !host || !cardEl.isConnected) return;
    const box = sc.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    const z = host.offsetHeight ? h.height / host.offsetHeight : 1;
    const top = h.top + cardEl.offsetTop * z;
    const bottom = top + cardEl.offsetHeight * z;
    const by = top < box.top + 8 ? top - box.top - 8 : bottom > box.bottom - 8 ? Math.min(bottom - (box.bottom - 8), top - box.top - 8) : 0;
    if (Math.abs(by) >= 1) sc.scrollBy({ top: by, behavior: reduced() ? "auto" : "smooth" });
  }
  /** Squeezed: which edges of its days have more past them, for their fades (app.css). */
  function edges() {
    if (!viewsEl) return;
    viewsEl.toggleAttribute("data-more-up", squeezed && viewsEl.scrollTop > 0.5);
    viewsEl.toggleAttribute("data-more-down", squeezed && viewsEl.scrollTop + viewsEl.clientHeight < viewsEl.scrollHeight - 0.5);
  }

  function keys(e: KeyboardEvent) {
    if (e.key === "PageUp" || e.key === "PageDown") {
      e.preventDefault();
      void go(e.key === "PageUp" ? -1 : 1);
    }
  }

  $effect(() => {
    const away = (e: PointerEvent) => {
      const el = e.target as HTMLElement;
      if (cardEl.contains(el) || el.closest(`[data-item-menu="${item.id}"]`)) return;
      onClose();
    };
    const popLayer = pushLayer(onClose, { back: onBack });
    window.addEventListener("pointerdown", away, true);
    return () => {
      popLayer();
      window.removeEventListener("pointerdown", away, true);
    };
  });

  // The line in the rule's own units: "Done 18 of the last 30 days · 5 days in a row".
  const OF: Record<Unit, Key> = {
    days: "Done {n} of the last {of} days",
    weekdays: "Done {n} of the last {of} weekdays",
    weeks: "Done {n} of the last {of} weeks",
    months: "Done {n} of the last {of} months",
    times: "Done {n} of the last {of} times",
  };
  const ROW: Record<Unit, Key> = {
    days: "{n} days in a row",
    weekdays: "{n} weekdays in a row",
    weeks: "{n} weeks in a row",
    months: "{n} months in a row",
    times: "{n} times in a row",
  };
  // (Its parts never break inside: a line too long for one goes on at the "·".)
  let line = $derived.by(() => {
    if (!sum || sum.ever === 0) return null;
    if (sum.of < 2) return [t("Done {n} times so far", { n: sum.ever })];
    const head = t(OF[sum.unit], { n: sum.done, of: sum.of });
    return sum.streak >= 2 ? [head, t(ROW[sum.unit], { n: sum.streak })] : [head];
  });
  const dayLabel = (d: Date) => cap(`${dayName(d.getDay(), true)} ${dayMonthLong(d)}`);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="card"
  role="dialog"
  tabindex="-1"
  aria-label={t("History of “{name}”", { name: item.text })}
  use:atPointer={{ at, motion, margin: 16, fit }}
  transition:popSwap|global={{ motion, swap }}
  bind:this={cardEl}
  use:modal={{ initial: "self", trap: false }}
  use:closeOnLeave={onClose}
  data-fit={fit}
  data-squeezed={squeezed ? "" : null}
  data-up={up ? "" : null}
  onkeydown={keys}
>
  <div class="top">
    <button class="icon back" aria-label={t("Back to the task’s menu")} data-tip={t("Back")} onclick={onBack}><Icon name="chev-left" size={15} stroke={2.2} /></button>
    <span class="label">{t("History")}</span>
    <button class="icon" aria-label={t("Close the history")} data-tip={t("Close")} data-tip-keys="Esc" onclick={onClose}><Icon name="close" size={14} /></button>
  </div>
  <p class="what"><span class="name">{item.text}</span>{#if item.repeat}<span class="rule">{repeatLabel(item.repeat)}</span>{/if}</p>

  <div class="month">
    <button class="icon" aria-label={t(view === "days" ? "Previous month" : "Previous year")} data-tip={t(view === "days" ? "Previous month" : "Previous year")} data-tip-keys="Page Up" disabled={shown <= first} bind:this={prevEl} onclick={() => go(-1)}><Icon name="chev-left" size={14} stroke={2.2} /></button>
    <span class="title" aria-live="polite">
      {#key shown}<span class="t" in:crumbIn out:crumbOut>{title}</span>{/key}
    </span>
    <button class="icon" aria-label={t(view === "days" ? "Next month" : "Next year")} data-tip={t(view === "days" ? "Next month" : "Next year")} data-tip-keys="Page Down" disabled={shown >= last} bind:this={nextEl} onclick={() => go(1)}><Icon name="chev-right" size={14} stroke={2.2} /></button>
  </div>

  <div class="views" bind:this={viewsEl} data-capped={squeezed ? "" : null} onscroll={edges}>
    {#key view}
      <div class="view" in:stepIn={{ dir: 1 }} out:stepOut={{ dir: 1 }}>
        {#if view === "days"}
          <div class="days" aria-hidden="true">
            {#each order as d (d)}<span>{dayName(d)}</span>{/each}
          </div>
          <div class="stage">
            {#key shown}
              <table class="grid" in:stepIn={{ dir }} out:stepOut={{ dir }}>
                <caption class="visually-hidden">{title}</caption>
                <tbody>
                  {#each weeks as week, w (w)}
                    <tr>
                      {#each week as day, i (i)}
                        {#if day}
                          {@const k = dayKey(day)}
                          {@const isDone = done.has(k)}
                          {@const isDue = !isDone && due.has(k)}
                          {@const missed = isDue && k < todayKey}
                          {@const coming = isDue && k >= todayKey}
                          <td>
                            <span class="day" class:done={isDone} class:missed class:coming class:today={k === todayKey} class:later={k > todayKey}>
                              <span aria-hidden="true">{day.getDate()}</span>
                              <span class="visually-hidden">{dayLabel(day)}{isDone ? `, ${t("Done (history)")}` : missed ? `, ${t("Missed (history)")}` : coming ? `, ${t("Coming (history)")}` : ""}{k === todayKey ? `, ${t("Today (history)")}` : ""}</span>
                            </span>
                          </td>
                        {:else}
                          <td></td>
                        {/if}
                      {/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/key}
          </div>
        {:else}
          <div class="stage">
            {#key shown}
              <div
                class="year"
                class:of-months={view === "months"}
                data-year={shown}
                role="grid"
                tabindex="-1"
                aria-label={title}
                style:--cols={cols}
                in:stepIn={{ dir }}
                out:stepOut={{ dir }}
                onkeydown={cellKeys}
                onfocusin={(e) => {
                  const i = (e.target as HTMLElement).dataset.i;
                  if (i != null) cur = Number(i);
                }}
              >
                {#each rows as row, r (r)}
                  {#if view === "weeks"}
                    <div class="names" aria-hidden="true">
                      {#each weekNames(row.cells.map((c) => c.from)) as n (n.col)}<span style:grid-column={n.col + 1}>{monthShort(n.month)}</span>{/each}
                    </div>
                  {/if}
                  <div class="row" role="row">
                    {#each row.cells as c, j (j)}
                      {@const i = row.at + j}
                      {@const words = view === "weeks" ? weekWords(c) : monthWords(c)}
                      <span
                        role="gridcell"
                        class="sq {c.state}"
                        class:current={c.current}
                        class:rest-missed={c.state === "partial" && c.days.some((d) => d.mark === "missed")}
                        style:--part={c.state === "partial" ? `${Math.round(c.part * 100)}%` : null}
                        tabindex={i === roving ? 0 : -1}
                        data-i={i}
                        data-tip={words}
                        aria-label={c.current ? `${words} · ${t(view === "weeks" ? "This week (history)" : "This month (history)")}` : words}
                      >
                        {#if view === "months"}<span class="m" aria-hidden="true">{monthShort(c.from)}</span>{/if}
                      </span>
                    {/each}
                  </div>
                {/each}
              </div>
            {/key}
          </div>
        {/if}

        <div class="legend" class:squares={view !== "days"} aria-hidden="true">
          <span><i class="dot done"></i>{t("Done (history)")}</span>
          <span><i class="dot missed"></i>{t("Missed (history)")}</span>
          <span><i class="dot coming"></i>{t("Coming (history)")}</span>
          {#if offs}<span><i class="dot off"></i>{t("Not due (history)")}</span>{/if}
        </div>
      </div>
    {/key}
  </div>
  <p class="sum">
    <!-- (The "·" ends its part's line: a long summary goes on after it, never with it.) -->
    {#if line}{#each line as part, i (i)}{#if i}{" "}{/if}<span class="part">{part}{#if i < line.length - 1}&nbsp;·{/if}</span>{/each}{:else}{t("Not done yet. Each time you tick it, it shows here.")}{/if}
  </p>
</div>

<style>
  .card {
    /* A missed day or week's chip, and the ring of one due. */
    --missed: color-mix(in srgb, var(--ink) 7%, var(--chip));
    --due-ring: inset 0 0 0 1px color-mix(in srgb, var(--box) 45%, transparent);
    position: absolute;
    top: 30px;
    right: 0;
    width: 300px;
    max-width: calc(100cqw - 8px);
    z-index: 40;
    padding: var(--s3);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--pop-shadow);
    transform-origin: 90% 0;
    cursor: default;
    text-align: left;
    outline: none;
  }
  /* Above the task (a short board with more room there): its foot 2 px into the line, by its ⋯,
     growing from there. */
  .card[data-up] {
    top: auto;
    bottom: calc(100% - 2px);
    transform-origin: 90% 100%;
  }
  /* Taller than the board even at its tightest: its days alone scroll, between its name and
     month above and its line below. (Their own padding keeps a focused square's ring whole.) */
  .card[data-squeezed] {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .card[data-squeezed] > * {
    flex: none;
  }
  /* (It never scrolls itself then: no fades of its own, nothing in its column but its parts.) */
  .card[data-squeezed]::before,
  .card[data-squeezed]::after {
    display: none;
  }
  .card[data-squeezed] > .views {
    flex: 0 1 auto;
    min-height: 0;
    margin: -3px;
    padding: 3px;
    --cap-pad-top: 3px;
    --cap-pad-bottom: 3px;
  }
  .top {
    display: grid;
    grid-template-columns: 28px 1fr 28px;
    align-items: center;
    gap: var(--s1);
    margin: -2px -2px 0;
  }
  .label {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
    text-align: center;
  }
  .icon {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.12s ease, opacity 0.12s ease;
  }
  .icon:hover:not(:disabled) {
    background: var(--hover);
  }
  .icon:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .what {
    margin: 6px 2px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    text-align: center;
  }
  .name {
    max-width: 100%;
    min-width: 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-lg);
    line-height: 1.3;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rule {
    max-width: 100%;
    font-size: var(--t-sm);
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .month {
    display: grid;
    grid-template-columns: 28px 1fr 28px;
    align-items: center;
    margin: var(--s3) -2px 6px;
  }
  .title {
    display: grid;
    justify-items: center;
    font-size: var(--t-md);
    font-weight: 600;
    color: var(--ink);
  }
  .title > .t {
    grid-area: 1 / 1;
    white-space: nowrap;
  }
  .days,
  .grid tr {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
  .days span {
    text-align: center;
    font-size: var(--t-xs);
    font-weight: 500;
    color: var(--muted);
    line-height: 20px;
  }
  /* (One cell: the month leaving is laid over the one arriving, never below it; and the view
     leaving, its rule changed while open, over the one arriving.) */
  .stage,
  .views {
    position: relative;
  }
  /* (Room kept for six weeks: a month of five is as tall, and ‹ › never makes the card jump.) */
  .days ~ .stage {
    min-height: 190px;
  }
  .grid {
    display: block;
    width: 100%;
    border-collapse: collapse;
  }
  .grid tbody {
    display: grid;
    gap: 2px;
  }
  .grid td {
    display: grid;
    place-items: center;
    padding: 0;
    height: 30px;
  }
  /* (Its ring, and today's outline over it: the same states as a year's squares.) */
  .day {
    position: relative;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: var(--t-sm);
    font-variant-numeric: tabular-nums;
    color: var(--soft);
    box-shadow: var(--ring, 0 0 #0000), var(--now, 0 0 #0000);
  }
  .day.later {
    color: var(--muted);
  }
  /* (Missed: a touch deeper than the chip. Still to come, today's too: only its ring.) */
  .day.missed {
    background: var(--missed);
    --ring: var(--due-ring);
  }
  .day.coming {
    --ring: var(--due-ring);
  }
  .day.done {
    background: var(--accent);
    color: var(--card);
    font-weight: 600;
  }
  .day.today {
    --now: 0 0 0 1.5px var(--ink);
  }
  .day.today.done {
    --now: 0 0 0 1.5px var(--card), 0 0 0 3px var(--ink);
  }

  /* ---- A year of weeks (a row a quarter, each month's name over its first week) or of months
     (a row half a year) ---- */
  /* (Its squares sized as fourteen columns would be, 52 weeks or 53, in a narrow card too: the
     card is as tall in every year.) */
  .year {
    --gap: 4px;
    display: grid;
    width: calc((min(100%, 14 * 20px - var(--gap)) - 13 * var(--gap)) * var(--cols) / 14 + (var(--cols) - 1) * var(--gap));
    margin: 2px auto 0;
    outline: none;
  }
  .names,
  .row {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
    column-gap: var(--gap);
  }
  .names {
    height: 14px;
    margin-bottom: 3px;
  }
  .row + .names {
    margin-top: 8px;
  }
  /* (A name is wider than its week: it runs on over the next ones, the room its month has.) */
  .names span {
    grid-row: 1;
    font-size: var(--t-xs);
    line-height: 14px;
    color: var(--muted);
    white-space: nowrap;
  }
  .sq {
    position: relative;
    aspect-ratio: 1;
    border-radius: 4px;
    box-shadow: var(--ring, 0 0 #0000), var(--now, 0 0 #0000);
    outline-offset: 1px;
  }
  /* (Before its first one, after its last: barely there, never "missed".) */
  .sq.none {
    background: color-mix(in srgb, var(--chip) 40%, transparent);
  }
  /* (Between two of an every-2 rule: a dot, so the rhythm shows.) */
  .sq.off::after,
  .dot.off::after {
    content: "";
    position: absolute;
    left: 50%;
    top: 50%;
    width: 4px;
    height: 4px;
    margin: -2px 0 0 -2px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--box) 75%, transparent);
  }
  /* (Missed: a touch deeper than the chip, so it reads apart from those still to come in the
     dark look too.) */
  .sq.missed {
    background: var(--missed);
    --ring: var(--due-ring);
  }
  /* (Still to come: only its ring.) */
  .sq.later {
    --ring: var(--due-ring);
  }
  .sq.done {
    background: var(--accent);
  }
  /* (Some of its days done: that part filled, the rest missed or still to come.) */
  .sq.partial {
    background: linear-gradient(to right, var(--accent) var(--part), transparent var(--part));
    --ring: inset 0 0 0 1px color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .sq.partial.rest-missed {
    background: linear-gradient(to right, var(--accent) var(--part), var(--missed) var(--part));
  }
  .sq.current {
    --now: 0 0 0 1.5px var(--ink);
  }
  .sq.current:is(.done, .partial) {
    --now: 0 0 0 1.5px var(--card), 0 0 0 3px var(--ink);
  }
  /* A year of months: twelve squares, their names in them. */
  .year.of-months {
    width: min(100%, 280px);
  }
  .of-months .row {
    column-gap: 8px;
  }
  .of-months .row + .row {
    margin-top: 8px;
  }
  .of-months .sq {
    display: grid;
    place-items: center;
    border-radius: var(--r-md);
    outline-offset: 2px;
  }
  .of-months .sq.off::after {
    top: auto;
    bottom: 5px;
    margin-top: 0;
  }
  .m {
    font-size: var(--t-xs);
    font-weight: 500;
    color: var(--soft);
  }
  .of-months .sq:is(.none, .off, .later) .m {
    color: var(--muted);
  }
  .of-months .sq.none .m {
    opacity: 0.6;
  }
  .of-months .sq.done .m {
    color: var(--card);
    font-weight: 600;
  }

  .legend {
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    column-gap: var(--s3);
    row-gap: 2px;
    margin-top: var(--s2);
    font-size: var(--t-xs);
    color: var(--muted);
  }
  .legend.squares {
    margin-top: var(--s3);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    position: relative;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .legend.squares .dot {
    border-radius: 3px;
  }
  .dot.done {
    background: var(--accent);
  }
  .dot.missed {
    background: var(--missed);
    box-shadow: var(--due-ring);
  }
  .dot.coming {
    box-shadow: var(--due-ring);
  }
  /* (Its room below is the card's own padding: a negative margin there would be lost against
     the fade at the card's foot, app.css `[data-capped]::after`, and leave twice the room.) */
  .sum {
    margin: var(--s3) calc(-1 * var(--s3)) 0;
    padding: var(--s3) var(--s3) 0;
    border-top: 1px solid var(--line);
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--soft);
    text-align: center;
    text-wrap: balance;
  }
  .part {
    white-space: nowrap;
  }

  /* Tighter, in a short board (`fit` 1 and up): the month closer, its line still in view. */
  .card:not([data-fit="0"]) .what {
    margin-top: 2px;
  }
  .card:not([data-fit="0"]) .month {
    margin: var(--s2) -2px 2px;
  }
  .card:not([data-fit="0"]) .grid tbody {
    gap: 1px;
  }
  .card:not([data-fit="0"]) .grid td {
    height: 25px;
  }
  .card:not([data-fit="0"]) .days ~ .stage {
    min-height: 155px;
  }
  .card:not([data-fit="0"]) .day {
    width: 24px;
    height: 24px;
  }
  .card:not([data-fit="0"]) .names {
    height: 12px;
    margin-bottom: 2px;
  }
  .card:not([data-fit="0"]) .names span {
    line-height: 12px;
  }
  .card:not([data-fit="0"]) .row + .names {
    margin-top: 5px;
  }
  .card:not([data-fit="0"]) .of-months .row + .row {
    margin-top: 6px;
  }
  .card:not([data-fit="0"]) .legend.squares {
    margin-top: var(--s2);
  }
  /* (Its line centred between its rule and the card's foot, as at rest.) */
  .card:not([data-fit="0"]) {
    padding-bottom: var(--s2);
  }
  .card:not([data-fit="0"]) .sum {
    margin-top: var(--s2);
    padding-top: var(--s2);
  }
  /* (2 and up) Its name and rule on one line, and no key: each day says what it is to a screen
     reader, and the line below counts them. */
  .card:is([data-fit="2"], [data-fit="3"]) .what {
    flex-direction: row;
    justify-content: center;
    align-items: baseline;
    gap: 6px;
  }
  /* (Both in sight however long: its name up to 60 % of the line, its rule the rest, each cut
     short with "…" past it.) */
  .card:is([data-fit="2"], [data-fit="3"]) .name {
    flex: none;
    max-width: 60%;
  }
  .card:is([data-fit="2"], [data-fit="3"]) .rule {
    flex: 0 1 auto;
    min-width: 0;
  }
  .card:is([data-fit="2"], [data-fit="3"]) .rule::before {
    content: "· ";
  }
  .card:is([data-fit="2"], [data-fit="3"]) .legend {
    display: none;
  }
  /* (3) Its month a touch closer, its days, or its squares, a little smaller. */
  .card[data-fit="3"] .month {
    margin-top: var(--s1);
  }
  .card[data-fit="3"] .days span {
    line-height: 16px;
  }
  .card[data-fit="3"] .grid td {
    height: 23px;
  }
  .card[data-fit="3"] .days ~ .stage {
    min-height: 143px;
  }
  .card[data-fit="3"] .day {
    width: 22px;
    height: 22px;
  }
  .card[data-fit="3"] .year {
    --gap: 3px;
    width: calc((min(100%, 14 * 17px - var(--gap)) - 13 * var(--gap)) * var(--cols) / 14 + (var(--cols) - 1) * var(--gap));
  }
  .card[data-fit="3"] .year.of-months {
    width: min(100%, 234px);
  }
  .card[data-fit="3"] .of-months .row {
    column-gap: 6px;
  }
</style>
