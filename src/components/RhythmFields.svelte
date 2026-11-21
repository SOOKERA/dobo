<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The deadline rhythm's minutes (Reminder rhythm board): used for a custom rhythm on one
  // deadline and for the default in Settings.
  import { untrack } from "svelte";
  import type { Rhythm } from "../lib/types";
  import { t, type Key } from "../lib/i18n.svelte";

  let { value, onSave, compact = false }: { value: Rhythm; onSave: (r: Rhythm) => void; compact?: boolean } = $props();

  let start = $state(untrack(() => value.start));
  let pre = $state(untrack(() => value.pre.join(" · ")));
  let every = $state(untrack(() => value.every));
  let max = $state(untrack(() => value.max));
  let checkin = $state(untrack(() => value.checkin));
  let notyet = $state(untrack(() => value.notyet));

  let parsedPre = $derived(
    pre
      .split(/[^\d]+/)
      .map((s) => Number(s))
      .filter((n) => n > 0)
      .sort((a, b) => b - a),
  );
  let changed = $derived(
    start !== value.start || parsedPre.join() !== value.pre.join() || every !== value.every || max !== value.max || checkin !== value.checkin || notyet !== value.notyet,
  );

  function save() {
    const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Math.round(Number(n) || lo)));
    onSave({
      start: clamp(start, 0, 10080),
      pre: parsedPre,
      every: clamp(every, 1, 1440),
      max: clamp(max, 1, 50),
      checkin: clamp(checkin, 1, 1440),
      notyet: clamp(notyet, 1, 1440),
    });
  }

  const rows: [string, Key, Key][] = [
    ["start", "Start reminding", "min before"],
    ["pre", "Ring again at", "min before"],
    ["every", "After the time, ring every", "min"],
    ["max", "If nobody answers, stop after", "rings"],
    ["checkin", "After “I'm on it”, ask if it's done", "min later"],
    ["notyet", "After “Not yet”, ask again", "min later"],
  ];
</script>

<div class="fields" class:compact>
  {#each rows as [key, label, unit] (key)}
    <label class="row">
      <span class="name">{t(label)}</span>
      {#if key === "pre"}
        <input class="wide" type="text" bind:value={pre} inputmode="numeric" />
      {:else if key === "start"}
        <input type="number" min="0" bind:value={start} />
      {:else if key === "every"}
        <input type="number" min="1" bind:value={every} />
      {:else if key === "max"}
        <input type="number" min="1" bind:value={max} />
      {:else if key === "checkin"}
        <input type="number" min="1" bind:value={checkin} />
      {:else}
        <input type="number" min="1" bind:value={notyet} />
      {/if}
      <span class="unit">{t(unit)}</span>
    </label>
  {/each}
  <div class="foot">
    <button class="save" onclick={save} disabled={!changed}>{t("Save")}</button>
  </div>
</div>

<style>
  .fields {
    display: flex;
    flex-direction: column;
    margin-top: var(--s2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: 38px;
    border-top: 1px solid var(--line);
    font-size: var(--t-base);
  }
  .compact .row {
    font-size: var(--t-sm);
    min-height: 34px;
  }
  .name {
    flex: 1;
    min-width: 0;
    color: var(--soft);
  }
  input {
    width: 64px;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: center;
    color: var(--ink);
  }
  input.wide {
    width: 92px;
  }
  .unit {
    width: 70px;
    color: var(--muted);
    font-size: var(--t-sm);
  }
  .compact .unit {
    width: 58px;
    font-size: var(--t-xs);
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    padding-top: var(--s2);
  }
  .save {
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .save:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
