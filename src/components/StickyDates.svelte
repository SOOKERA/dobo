<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Once, after the Sticky Notes import: lines that mention a day or a time ("demain 9h",
  // "lundi 14h") can get a reminder in one go. The text stays as it was written.
  import { store } from "../lib/store.svelte";
  import { findDate } from "../lib/capture";
  import { formatDueLong } from "../lib/time";
  import { card } from "../lib/motion";
  import { t } from "../lib/i18n.svelte";

  let found = $derived(
    store.items
      .filter((i) => i.kind === "task" && i.due_at == null && i.done_at == null)
      .map((i) => ({ it: i, f: findDate(i.text, store.now) }))
      .filter((x) => x.f && x.f.due.getTime() > store.now.getTime())
      .slice(0, 8),
  );
  let picked = $state<Record<number, boolean>>({});
  let on = (id: number, certain: boolean) => picked[id] ?? certain;

  async function add() {
    for (const { it, f } of found) if (f && on(it.id, f.certain)) await store.setDue(it, f.due.getTime());
    await store.setSetting("sticky_dates_offered", true);
  }
</script>

{#if found.length}
  <section class="offer" aria-label={t("Times found in your sticky notes")} transition:card|global>
    <div class="head">
      <span class="title">{t("Times found in your sticky notes")}</span>
      <span class="sub">{t("Add a reminder to these lines? Their words stay as you wrote them.")}</span>
    </div>
    {#each found as { it, f } (it.id)}
      {#if f}
        <label class="row">
          <input type="checkbox" checked={on(it.id, f.certain)} onchange={(e) => (picked = { ...picked, [it.id]: e.currentTarget.checked })} />
          <span class="text">{it.text}</span>
          <span class="when">{formatDueLong(f.due.getTime(), store.now)}</span>
        </label>
      {/if}
    {/each}
    <div class="foot">
      <button class="plain" onclick={() => store.setSetting("sticky_dates_offered", true)}>{t("No thanks")}</button>
      <button class="strong" onclick={add}>{t("Add these reminders")}</button>
    </div>
  </section>
{/if}

<style>
  .offer {
    margin: var(--s4) var(--s6) 0;
    padding: var(--s4) var(--s5);
    flex-shrink: 0;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 14px;
    box-shadow: var(--card-shadow);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 4px;
  }
  .title {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
  }
  .sub {
    font-size: var(--t-md);
    color: var(--soft);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    font-size: var(--t-base);
    cursor: pointer;
  }
  .row input {
    width: 16px;
    height: 16px;
    accent-color: var(--ink);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
    background: var(--chip);
    border-radius: 5px;
    padding: 2px 6px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    margin-top: 6px;
  }
  .foot button {
    height: 36px;
    padding: 0 14px;
    border-radius: 9px;
    font-size: var(--t-base);
    cursor: pointer;
  }
  .plain {
    border: 1px solid var(--line);
    background: var(--card);
    color: var(--soft);
  }
  .strong {
    border: 0;
    background: var(--ink);
    color: var(--card);
    font-weight: 600;
  }
</style>
