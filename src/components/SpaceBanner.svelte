<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Switched to Work for this reminder · Back to Personal" (Work hours board).
  import { store } from "../lib/store.svelte";
  import { card } from "../lib/motion";
  import { around, t } from "../lib/i18n.svelte";

  const names = { work: "Work", personal: "Personal", all: "All" } as const;
  let [before, after] = $derived(around(t("Switched to {space} for this reminder"), "{space}"));
</script>

{#if store.banner}
  <div class="banner" role="status" transition:card|global>
    <span class="text">{before}<b>{t(names[store.banner.space])}</b>{after}</span>
    <button onclick={() => store.banner && store.setView(store.banner.back)}>{t("Back to {space}", { space: t(names[store.banner.back]) })}</button>
  </div>
{/if}

<style>
  .banner {
    margin: var(--s3) var(--s6) 0;
    padding: 10px var(--s3);
    display: flex;
    align-items: center;
    gap: 10px;
    border-radius: 10px;
    background: var(--card);
    border: 1px solid var(--line);
    font-size: var(--t-md);
    flex-shrink: 0;
  }
  .text {
    flex: 1;
    color: var(--soft);
  }
  b {
    color: var(--ink);
    font-weight: 600;
  }
  button {
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--card);
    font-size: var(--t-sm);
    font-weight: 500;
    cursor: pointer;
  }
  button:hover {
    background: var(--hover);
  }
</style>
