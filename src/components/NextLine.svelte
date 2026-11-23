<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The "Next" line under the entry line: the next three timed tasks. It stays as it is while a
  // task is carried; the hours ruler opens under the top bar as a card of its own (HoursRuler).
  import { store } from "../lib/store.svelte";
  import { formatDue } from "../lib/time";
  import { t } from "../lib/i18n.svelte";
  import { arrive, easeHeight, fadeQuick, glideTo, leave } from "../lib/motion";

  // (The same three as before stay the same list: the entries are measured for their glide only
  // when what shows changes, not each time any line changes.)
  let shown: typeof store.timed = [];
  let next = $derived.by(() => {
    const n = store.timed.slice(0, 3);
    if (n.length === shown.length && n.every((x, i) => x === shown[i])) return shown;
    return (shown = n);
  });
</script>

<div class="wrap" use:easeHeight>
  <div class="stack">
    {#if next.length}
      <div class="then" in:arrive={{ delay: 110, duration: 240, y: -3 }} out:leave={{ duration: 110, y: -3 }}>
        <span class="lead">{t("Next")}</span>
        <span class="list">
        {#each next as it (it.id)}
          {@const late = (it.due_at ?? 0) < store.now.getTime()}
          <span class="entry" data-tip={it.text} animate:glideTo in:arrive={{ delay: 60, duration: 220, y: 0 }} out:fadeQuick={{ duration: 160 }}>
            <span class="time" class:late>{formatDue(it.due_at ?? 0, store.now)}</span>
            <span class="text">{it.text}</span>
          </span>
        {/each}
        </span>
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap {
    flex-shrink: 0;
    position: relative;
  }
  .stack {
    position: relative;
    display: grid;
    /* (Its width is the panel's, never its content's.) */
    grid-template-columns: minmax(0, 1fr);
  }
  .stack > * {
    grid-area: 1 / 1;
  }
  .then {
    display: flex;
    align-items: baseline;
    gap: var(--s5);
    padding: var(--s3) var(--s6) 0 calc(var(--s6) + var(--s1));
    font-size: var(--t-md);
    white-space: nowrap;
  }
  /* Whole entries only: one that can't show its time and a few words goes onto a second line,
     which never shows (never a time cut in half). */
  .list {
    flex: 1;
    min-width: 0;
    height: 20px;
    line-height: 20px;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    column-gap: var(--s5);
    overflow: hidden;
  }
  .lead {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-lg);
    color: var(--muted);
    flex-shrink: 0;
  }
  /* At least its time and a few words (a narrower panel shows fewer entries); wider, up to its
     whole text. */
  .entry {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s2);
    flex: 1 0 auto;
    width: 10em;
    max-width: max-content;
    min-width: 0;
    overflow: hidden;
  }
  .time {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .time.late {
    color: var(--late-ink);
  }
  .text {
    color: var(--soft);
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
