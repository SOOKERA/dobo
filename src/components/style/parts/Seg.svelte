<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" generics="T extends string">
  // A small segmented choice (Auto · Pins · Tape · None).
  let { value, options, label, onPick, serif = false }: { value: T; options: { key: T; label: string; title?: string; size?: number }[]; label: string; onPick: (v: T) => void; serif?: boolean } = $props();
</script>

<div class="seg" class:serif role="radiogroup" aria-label={label}>
  {#each options as o (o.key)}
    <button role="radio" aria-checked={value === o.key} class:on={value === o.key} data-tip={o.title ?? (o.size ? o.label : null)} style:font-size={o.size ? `${o.size}px` : null} onclick={() => onPick(o.key)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 8px;
    background: var(--chip);
    flex-shrink: 0;
  }
  button {
    height: 26px;
    min-width: 30px;
    padding: 0 9px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--soft);
    font-size: var(--t-sm);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.18s, color 0.18s, box-shadow 0.18s;
  }
  button:hover {
    color: var(--ink);
  }
  button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .serif button {
    font-family: var(--serif);
    width: 36px;
    padding: 0;
  }
</style>
