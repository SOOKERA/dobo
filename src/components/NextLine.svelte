<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The "Next" line under the entry line: the next three timed tasks. It stays as it is while a
  // task is carried; the hours ruler opens under the top bar as a card of its own (HoursRuler).
  import { store } from "../lib/store.svelte";
  import { formatDue } from "../lib/time";
  import { t } from "../lib/i18n.svelte";
  import { arrive, easeHeight, fadeQuick, glideTo, leave } from "../lib/motion";

  let next = $derived(store.timed.slice(0, 3));
</script>

<div class="wrap" use:easeHeight>
  <div class="stack">
    {#if next.length}
      <div class="then" in:arrive={{ delay: 110, duration: 240, y: -3 }} out:leave={{ duration: 110, y: -3 }}>
        <span class="lead">{t("Next")}</span>
        {#each next as it (it.id)}
          {@const late = (it.due_at ?? 0) < store.now.getTime()}
          <span class="entry" data-tip={it.text} animate:glideTo in:arrive={{ delay: 60, duration: 220, y: 0 }} out:fadeQuick={{ duration: 160 }}>
            <span class="time" class:late>{formatDue(it.due_at ?? 0, store.now)}</span>
            <span class="text">{it.text}</span>
          </span>
        {/each}
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
    overflow: hidden;
    white-space: nowrap;
  }
  .lead {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-lg);
    color: var(--muted);
    flex-shrink: 0;
  }
  .entry {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s2);
    min-width: 0;
    flex-shrink: 1;
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
