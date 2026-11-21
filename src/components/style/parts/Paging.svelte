<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // ‹ › for a row with more than fits (newest first).
  import Icon from "../../Icon.svelte";
  import { t } from "../../../lib/i18n.svelte";
  let { page, pages, label, onPage }: { page: number; pages: number; label: string; onPage: (p: number) => void } = $props();
</script>

{#if pages > 1}
  <span class="arrows">
    <button class="arrow" disabled={page <= 0} aria-label={t("Previous: {label}", { label })} onclick={() => onPage(page - 1)}><Icon name="chev-left" size={11} stroke={2.6} /></button>
    <button class="arrow" disabled={page >= pages - 1} aria-label={t("More: {label}", { label })} onclick={() => onPage(page + 1)}><Icon name="chev-right" size={11} stroke={2.6} /></button>
  </span>
{/if}

<style>
  .arrows {
    display: flex;
    gap: 6px;
  }
  .arrow {
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: var(--card);
    color: var(--ink);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.15s, opacity 0.15s;
  }
  .arrow:hover:not(:disabled) {
    background: var(--hover);
  }
  .arrow:disabled {
    opacity: 0.35;
    cursor: default;
  }
</style>
