<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Peeking titles: with "open when the mouse reaches the edge" and "peek first" on, the notes'
  // titles slide out of the screen edge like index cards from a drawer. Click one to open the
  // panel on that note; stay against the edge to open the whole panel; move away to close.
  import { store } from "../lib/store.svelte";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import { t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";
  import { blip } from "../lib/motion";

  let left = $derived(store.win.side === "left");
  // (The Inbox, first, has no colour of its own: a plain tab with a quiet grey dot.)
  // (Whatever its old place among the cards: the board shows it above them now.)
  let notes = $derived([...store.stripNotes, ...store.boardNotes].slice(0, 14));
  let dark = $derived(look.dark);

  function open(id: number) {
    store.peek = false;
    void store.backend.openNote(id);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="peek" class:left onclick={(e) => e.target === e.currentTarget && open(0)}>
  {#each notes as n, i (n.id)}
    {@const inbox = store.inboxIds.has(n.id)}
    {@const tone = inbox ? { bg: "var(--card)", dot: "var(--box)" } : dark ? swatchOf(n, store.custom).dark : swatchOf(n, store.custom).light}
    {@const open_ = (store.itemsByNote.get(n.id) ?? []).filter((x) => x.kind === "task" && x.done_at == null).length}
    <button class="tab" style:--d="{i * 28}ms" style:--tint={tone.bg} onclick={() => open(n.id)} data-tip={t("Open “{title}”", { title: noteTitle(n) })}>
      <span class="dot" style:background={tone.dot}></span>
      <span class="title">{noteTitle(n)}</span>
      {#if open_}<span class="count" transition:blip>{open_}</span>{/if}
    </button>
  {/each}
  <span class="hint">{t("Stay at the edge to open everything")}</span>
</div>

<style>
  .peek {
    position: absolute;
    inset: 12px 12px 12px 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 6px;
  }
  .peek.left {
    align-items: flex-start;
  }
  .tab {
    width: 228px;
    min-height: 40px;
    padding: 0 12px;
    border: 1px solid var(--line);
    border-right: 0;
    border-radius: 10px 0 0 10px;
    background: linear-gradient(to right, var(--card) 70%, var(--tint));
    box-shadow: var(--card-shadow), -6px 4px 14px rgba(20, 16, 10, 0.12);
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    color: var(--ink);
    cursor: pointer;
    transform: translateX(110%);
    animation: slide 0.42s cubic-bezier(0.2, 0.85, 0.25, 1) forwards;
    animation-delay: var(--d);
    transition: transform 0.18s var(--ease-out);
  }
  .left .tab {
    border-right: 1px solid var(--line);
    border-left: 0;
    border-radius: 0 10px 10px 0;
    background: linear-gradient(to left, var(--card) 70%, var(--tint));
    transform: translateX(-110%);
  }
  @keyframes slide {
    to {
      transform: translateX(14px);
    }
  }
  .left .tab {
    animation-name: slide-left;
  }
  @keyframes slide-left {
    to {
      transform: translateX(-14px);
    }
  }
  .tab:hover {
    translate: -10px 0;
  }
  .left .tab:hover {
    translate: 10px 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .tab {
      animation: none;
      transform: none;
    }
  }
  .dot {
    flex-shrink: 0;
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }
  .title {
    flex: 1;
    min-width: 0;
    font-family: var(--serif);
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--muted);
  }
  .hint {
    margin-top: 8px;
    padding: 3px 9px;
    border-radius: 7px;
    background: rgba(20, 22, 26, 0.6);
    color: #f3f1ec;
    font-size: var(--t-xs);
  }
</style>
