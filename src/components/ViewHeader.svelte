<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" module>
  export interface Crumb {
    label: string;
    /** Jumps straight to that level (the earlier crumbs; the last one is where you are). */
    go?: () => void;
  }
</script>

<script lang="ts">
  // The header row every view over the board shares: ‹ Back (one level up), where you are as a
  // path ("Style › Themes › Edit theme": earlier levels quieter and clickable, the current one
  // strongest), anything the view adds, and × (Close · Esc: straight back to the board).
  // Long paths fold their middle levels into "…" when the row is too narrow; the current page
  // always stays whole.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import { crumbIn, crumbOut, fadeQuick } from "../lib/motion";
  import { t } from "../lib/i18n.svelte";

  let {
    crumbs,
    onBack,
    onClose,
    sub = "",
    big = 22,
    fit = false,
    children,
  }: { crumbs: Crumb[]; onBack: () => void; onClose: () => void; sub?: string; big?: number; fit?: boolean; children?: Snippet } = $props();

  let box = $state<HTMLElement>();
  let measure = $state<HTMLElement>();
  let fold = $state(false);

  // Fold the middle levels when the whole path doesn't fit (measured on an invisible copy).
  $effect(() => {
    if (!box || !measure) return;
    const b = box;
    const m = measure;
    const check = () => (fold = crumbs.length > 2 && m.scrollWidth > b.clientWidth + 1);
    check();
    const ro = new ResizeObserver(check);
    ro.observe(b);
    ro.observe(m);
    return () => ro.disconnect();
  });

  let shown = $derived.by<(Crumb & { more?: string })[]>(() => {
    if (!fold || crumbs.length <= 2) return crumbs;
    const mid = crumbs.slice(1, -1);
    // "…" jumps one level up (to the parent of where you are), like Back.
    return [crumbs[0], { label: "…", go: mid[mid.length - 1].go, more: mid.map((c) => c.label).join(" › ") }, crumbs[crumbs.length - 1]];
  });
</script>

<div class="vh" style:--big="{big}px">
  <button class="icon back" aria-label={t("Back")} data-tip={t("Back")} data-tip-keys="Alt+Left|Backspace|Mouse back" onclick={onBack}><Icon name="chev-left" size={18} stroke={2} /></button>
  <div class="where" class:fit>
    <nav class="path" aria-label={t("You are here")} bind:this={box}>
      {#each shown as c, i (i)}
        {@const last = i === shown.length - 1}
        {#if i}<span class="sep" aria-hidden="true">›</span>{/if}
        <span class="cell" class:last class:dots={c.label === "…"} in:crumbIn>
          <!-- (The key sits inside the if, so a new label's words really crossfade.) -->
          {#if last}
            {#key c.label}
              <span class="crumb now" aria-current="page" in:crumbIn out:crumbOut>{c.label}</span>
            {/key}
          {:else}
            {#key c.label}
              <button class="crumb" data-tip={c.more ?? t("Go to {label}", { label: c.label })} aria-label={c.more} onclick={() => c.go?.()} in:crumbIn out:crumbOut>{c.label}</button>
            {/key}
          {/if}
        </span>
      {/each}
    </nav>
    <!-- The whole path, unseen, to know whether it fits. -->
    <span class="path measure" aria-hidden="true" bind:this={measure}>
      {#each crumbs as c, i (i)}
        {#if i}<span class="sep">›</span>{/if}<span class="crumb" class:now={i === crumbs.length - 1}>{c.label}</span>
      {/each}
    </span>
    {#if sub}<span class="sub-swap" transition:fadeQuick>{#key sub}<small class="sub" in:crumbIn out:crumbOut>{sub}</small>{/key}</span>{/if}
  </div>
  {#if children}{@render children()}{/if}
  <button class="icon close" aria-label={t("Close")} data-tip={t("Close")} data-tip-keys="Esc" onclick={onClose}><Icon name="close" size={16} stroke={2} /></button>
</div>

<style>
  .vh {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    min-width: 0;
  }
  .icon {
    flex: none;
    width: 34px;
    height: 34px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon, currentColor);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .icon:hover {
    background: var(--hover, color-mix(in srgb, currentColor 10%, transparent));
    color: var(--ink, currentColor);
  }
  .back {
    margin-left: -8px;
  }
  .close {
    margin-right: -6px;
  }
  .where {
    flex: 1;
    min-width: 0;
    position: relative;
  }
  /* The view puts its own things after the path (Tidy's progress): the path takes only its room. */
  .where.fit {
    flex: 0 1 auto;
  }
  .path {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }
  .measure {
    position: absolute;
    left: 0;
    top: 0;
    visibility: hidden;
    pointer-events: none;
    width: max-content;
  }
  .cell {
    display: grid;
    grid-template-columns: minmax(0, auto);
    min-width: 0;
    /* The earlier levels give up their room long before the current page does. */
    flex: 0 100 auto;
  }
  .cell > :global(*) {
    grid-area: 1 / 1;
  }
  /* The current page keeps its whole name; only when even that can't fit does it shorten. */
  .cell.last {
    flex: 0 1 auto;
  }
  .cell.dots {
    flex: none;
  }
  .cell:first-child:not(.last) {
    min-width: 2.6em;
  }
  .crumb {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-family: var(--sans);
    font-size: 13px;
    color: color-mix(in srgb, currentColor 58%, transparent);
    cursor: pointer;
    text-align: left;
  }
  button.crumb:hover {
    color: inherit;
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-thickness: 1px;
  }
  button.crumb:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 3px;
  }
  .crumb.now {
    font-family: var(--serif);
    font-size: var(--big);
    font-weight: 500;
    line-height: 1.15;
    color: inherit;
    cursor: default;
  }
  .sep {
    flex: none;
    font-size: 13px;
    color: color-mix(in srgb, currentColor 40%, transparent);
  }
  .sub-swap {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .sub-swap > .sub {
    grid-area: 1 / 1;
  }
  .sub {
    display: block;
    margin-top: 1px;
    font-family: var(--sans);
    font-size: 11.5px;
    color: color-mix(in srgb, currentColor 58%, transparent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
