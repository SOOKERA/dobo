<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The evening look back (Review board): what got done, what is still open from today (move it
  // to tomorrow, pick a day, or leave it without a time), and today's new ideas. No scores.
  // It comes once when work hours (or the day) end, and leaves by itself the next morning.
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { inMemory } from "../lib/sample";
  import { parseCapture } from "../lib/capture";
  import { formatDueLong } from "../lib/time";
  import { dayBounds, lookbackAt, lookbackSpan, lookbackTomorrow, spanDay } from "../lib/plan";
  import { doneBetween } from "../lib/doneLog";
  import { pushLayer } from "../lib/layers";
  import { modal, takeFocus } from "../lib/modal";
  import type { Item } from "../lib/types";
  import { arrive, card, fold, leave, widen } from "../lib/motion";
  import { dayName, t } from "../lib/i18n.svelte";
  import { untrack } from "svelte";

  let { day, scroller }: { day: Date; scroller?: HTMLElement } = $props();

  // (Not the quick tour's sample lines: "Book the van" is shown done while the tour is on.)
  let real = $derived(store.items.filter((i) => !inMemory(i.id)));
  // (Calendar days: 23 or 25 hours on the nights the clocks change; a work night that crosses
  // midnight counts for the day it started, up to its end the next morning. Its notification
  // counts these three the same way, reminders.rs `lookback_count`: keep them in step.)
  let dayStart = $derived(dayBounds(day).start);
  let start = $derived(lookbackSpan(day, store.workHours).start);
  let end = $derived(lookbackSpan(day, store.workHours).end);
  let dayBefore = $derived(dayBounds(new Date(dayStart - 1)).start);
  // (A repeating task done that day is in its history once it is open again: each day it was
  // done has it, never twice.)
  let done = $derived(doneBetween(real, store.doneLog, start, end));
  let open = $derived(real.filter((i) => i.kind === "task" && i.done_at == null && i.due_at != null && i.due_at < end && i.due_at >= dayBefore));
  let ideas = $derived(real.filter((i) => i.kind === "idea" && i.created_at >= start && i.created_at < end));
  /** The day whose span holds now: in the night part of work hours that cross midnight, still
   * the day that night began. */
  let isToday = $derived(dayBounds(spanDay(store.now.getTime(), store.workHours)).start === dayStart);
  /** Opened before the day is over (its look back comes at the end of work hours or of the day):
   * "Today so far.", and no "See you tomorrow" yet. */
  let soFar = $derived(isToday && store.now.getTime() < lookbackAt(dayStart, store.day, store.workHours));
  /** …and after midnight, in the night's work hours: "Tonight so far." */
  let tonight = $derived(soFar && store.now.getTime() >= dayBounds(day).end);

  let lookEl = $state<HTMLElement>();
  let picking = $state<number | null>(null);
  let pickText = $state("");
  /** The row whose day picking Esc called off: the keyboard goes back to its "Pick a day". */
  let pickedBack = $state<number | null>(null);
  let picked = $derived(pickText.trim() ? parseCapture(`x ${pickText}`, "task", store.now).due : null);

  // Opened from the menu, Esc closes it. (When it came by itself, Esc still hides the panel.)
  $effect(() => {
    if (store.lookbackOpen) return pushLayer(() => store.hideLookback(), undefined, { beside: true });
  });

  // Opened by hand, it opens at the top of the board: a board scrolled down goes up to it, as the
  // card unfolds. (The evening's, coming by itself, never moves the board under someone working:
  // it makes its room above what they are looking at.)
  $effect(() => {
    if (!scroller || scroller.scrollTop < 1 || !untrack(() => store.lookbackOpen)) return;
    scroller.scrollTo({ top: 0, behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
  });

  // (After midnight the look back still shows the day before: its "Tomorrow" is today.)
  function tomorrow(it: Item) {
    store.setDue(it, lookbackTomorrow(it.due_at ?? store.now.getTime(), day, Date.now()));
  }
</script>

<!-- Opened by hand, the keyboard comes in (and goes back when it closes); come by itself in the
     evening, it never takes the keyboard from someone working. -->
<section
  class="look"
  aria-label={t("Look back on the day")}
  tabindex="-1"
  bind:this={lookEl}
  transition:card|global={{ duration: 520, room: 0.9, show: 0.3 }}
  use:modal={{ initial: untrack(() => store.lookbackOpen) ? "self" : "none", trap: false, restore: () => !!lookEl?.contains(document.activeElement) }}
>
  <div class="head">
    <div>
      <h2>{tonight ? t("Tonight so far.") : soFar ? t("Today so far.") : t("That was {day}.", { day: dayName(day.getDay(), true) })}</h2>
      <p>{t("Here is what happened, and what moves on.")}</p>
    </div>
    <!-- (Esc closes only one opened by hand: the evening's, come by itself, leaves Esc to hide the panel.) -->
    <button class="x" aria-label={t("Close the look back")} data-tip={t("Close")} data-tip-keys={store.lookbackOpen ? "Esc" : null} onclick={() => store.dismissLookback()}><Icon name="close" size={16} /></button>
  </div>

  <div class="cols">
    <div class="col">
      <span class="label">{isToday ? t("Done today · {n}", { n: done.length }) : t("Done · {n}", { n: done.length })}</span>
      {#if done.length}
        {#each done.slice(0, 5) as d (d.key)}
          <div class="done" transition:fold={{ gap: 6 }}><span class="box"><Icon name="check" size={10} stroke={3.4} /></span><span class="t">{d.text}</span></div>
        {/each}
        {#if done.length > 5}<span class="more" transition:fold={{ gap: 6 }}>{t("and {n} more", { n: done.length - 5 })}</span>{/if}
      {:else}
        <span class="quiet" transition:fold={{ gap: 6 }}>{t("Nothing ticked. Some days are like that.")}</span>
      {/if}
    </div>

    <div class="col">
      <span class="label">{isToday ? t("Still open from today · {n}", { n: open.length }) : t("Still open from that day · {n}", { n: open.length })}</span>
      {#each open as o (o.id)}
        <div class="open" transition:fold={{ gap: 6 }}>
          <span class="t" data-tip={o.text}>{o.text}</span>
          <span class="swap">
          {#if picking === o.id}
            <span class="pick" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
              <input
                use:takeFocus
                bind:value={pickText}
                placeholder={t("Monday 9am, Friday…")}
                aria-label={t("New day for {text}", { text: o.text })}
                autocomplete="off"
                onkeydown={(e) => {
                  if (composing(e)) return;
                  if (e.key === "Enter" && picked) {
                    store.setDue(o, picked.getTime());
                    picking = null;
                    pickText = "";
                  } else if (e.key === "Escape") {
                    // Esc goes back to the three choices (not out of the look back).
                    e.stopPropagation();
                    e.preventDefault();
                    pickedBack = o.id;
                    picking = null;
                    pickText = "";
                  }
                }}
              />
              {#if picked}<button class="b strong" transition:widen={{ gap: 4 }} onclick={() => (picked && store.setDue(o, picked.getTime()), (picking = null), (pickText = ""))}>{formatDueLong(picked.getTime(), store.now)}</button>{/if}
            </span>
          {:else}
            <span class="btns" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
              <button class="b strong" onclick={() => tomorrow(o)}>{t("Tomorrow")}</button>
              <button class="b" use:takeFocus={pickedBack === o.id} onfocus={() => (pickedBack = null)} onclick={() => ((picking = o.id), (pickText = ""))}>{t("Pick a day")}</button>
              <button class="b plain" onclick={() => store.setDue(o, null)}>{t("No time")}</button>
            </span>
          {/if}
          </span>
        </div>
      {:else}
        <span class="quiet" transition:fold={{ gap: 6 }}>{isToday ? t("Nothing left from today.") : t("Nothing left from that day.")}</span>
      {/each}
    </div>
  </div>

  {#if ideas.length}
    <div class="ideas" transition:fold>
      <span class="label">{t("New ideas · {n}", { n: ideas.length })}</span>
      <span class="idea">{ideas[ideas.length - 1].text}</span>
    </div>
  {/if}

  <div class="foot">
    {#if !soFar}<span class="bye">{t("See you tomorrow.")}</span>{/if}
    <button class="close" onclick={() => store.dismissLookback()}>{t("Close")}</button>
  </div>
</section>

<style>
  /* Whole (never its own scrollbar): the board under it makes room and the page scrolls. */
  .look {
    position: relative;
    z-index: 2;
    margin: 0 0 var(--s5);
    padding: var(--s5) var(--s6);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 16px;
    box-shadow: 0 14px 32px rgba(60, 50, 30, 0.1);
    display: flex;
    flex-direction: column;
    gap: var(--s4);
  }
  @media (prefers-reduced-motion: reduce) {
    .look {
      animation: none;
    }
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--s3);
  }
  h2 {
    margin: 0;
    font-family: var(--serif);
    font-size: 26px;
    font-weight: 500;
  }
  p {
    margin: 4px 0 0;
    font-size: var(--t-base);
    color: var(--soft);
  }
  .x {
    width: 32px;
    height: 32px;
    margin: -4px -8px 0 0;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
  }
  /* One column, as on the board: done first, then what is still open (its titles need the width). */
  .cols {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .label {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .done {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--t-base);
    color: var(--muted);
  }
  .box {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--done-box);
    color: var(--card);
    display: grid;
    place-items: center;
  }
  .t {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more,
  .quiet {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .more {
    padding-left: 26px;
  }
  .quiet {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-base);
  }
  /* The line, then its three answers at the right; a narrow card puts the answers under it. */
  .open {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: var(--s3);
    row-gap: 4px;
    min-height: 32px;
    font-size: var(--t-base);
  }
  .open .t {
    flex: 1 1 160px;
  }
  /* The three answers and the day field share one place. */
  .swap {
    position: relative;
    display: grid;
    justify-items: end;
    flex-shrink: 0;
    margin-left: auto;
  }
  .swap > * {
    grid-area: 1 / 1;
    justify-content: flex-end;
  }
  .btns {
    display: flex;
    gap: 4px;
  }
  .pick {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .pick input {
    width: 130px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--ground);
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
  }
  .b {
    height: 28px;
    padding: 0 9px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--card);
    font-size: var(--t-sm);
    color: var(--soft);
    cursor: pointer;
    white-space: nowrap;
  }
  .b.strong {
    color: var(--ink);
    font-weight: 600;
  }
  .b.plain {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
  }
  .b:hover {
    background: var(--hover);
  }
  .ideas {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 14px;
    border-radius: 10px;
    background: #fbf4df;
  }
  .ideas .label {
    color: #5e4b19;
  }
  .idea {
    font-family: var(--serif);
    font-style: italic;
    font-size: 15px;
    color: #4a3c14;
  }
  :global(:root[data-mode="dark"]) .ideas {
    background: rgba(217, 169, 40, 0.12);
  }
  :global(:root[data-mode="dark"]) .ideas .label,
  :global(:root[data-mode="dark"]) .idea {
    color: #ead9a0;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .bye {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-lg);
    color: var(--muted);
  }
  .close {
    margin-left: auto;
    height: 40px;
    padding: 0 18px;
    border: 0;
    border-radius: 10px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-base);
    font-weight: 600;
    cursor: pointer;
  }
  .look:focus {
    outline: none;
  }
</style>
