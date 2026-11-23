<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // One idea (in a note's tinted part, or in the Inbox strip): its words in italics, its steps'
  // progress, Make it a task and Delete on hover, its link / file and its steps. Double-click (or
  // Enter on the board) edits it; press and drag the words to carry it elsewhere. The tint comes
  // from where it sits (--tint, --idea-ink, --corner).
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import ItemRow from "./ItemRow.svelte";
  import LinkCard from "./LinkCard.svelte";
  import FileCard from "./FileCard.svelte";
  import StepTrace from "./StepTrace.svelte";
  import { store, NO_ITEMS } from "../lib/store.svelte";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { snap } from "../lib/snap";
  import { fold, widen } from "../lib/motion";
  import type { Item } from "../lib/types";
  import { t } from "../lib/i18n.svelte";

  // (`as`: "li" in a list of lines, as in the Inbox strip.)
  let { item, lined = false, gap = 7, as = "div" }: { item: Item; lined?: boolean; gap?: number; as?: "div" | "li" } = $props();

  let editing = $state(false);
  let draft = $state("");
  let steps = $derived(store.stepsOf.get(item.id) ?? NO_ITEMS);
  let stepsDone = $derived(steps.filter((s) => s.done_at != null).length);
  let focused = $derived(store.focus?.item === item.id);
  let dropHere = $derived(itemDrag.target?.kind === "idea" && itemDrag.target.id === item.id);
  let carried = $derived(itemDrag.item?.id === item.id || itemDrag.away.includes(item.id));

  function focusSelect(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
  function start() {
    draft = item.text;
    editing = true;
  }
  async function end() {
    if (!editing) return;
    editing = false;
    if (draft.trim() !== item.text) await store.editItem(item, draft);
  }
</script>

<svelte:element this={as} class="idea-wrap" class:lined transition:fold={{ gap }} class:carried>
  <div class="idea" class:drop={dropHere} class:focused data-drop-item={item.id} data-drop-kind="idea" data-drop-label={t("Drop to make it the first step")} data-line={item.id}>
    {#if editing}
      <input
        class="line-input idea-input"
        bind:value={draft}
        use:focusSelect
        aria-label={t("Edit idea")}
        onkeydown={(e) => {
          // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
          if (composing(e)) return;
          if (e.key === "Enter") end();
          if (e.key === "Escape") {
            e.stopPropagation();
            end();
          }
          // Backspace at the start of the line: it becomes a task.
          const el = e.currentTarget;
          if (e.key === "Backspace" && el.selectionStart === 0 && el.selectionEnd === 0) {
            e.preventDefault();
            const text = draft;
            editing = false;
            store.flipKind({ ...item, text: text.trim() || item.text });
          }
        }}
        onblur={end}
      />
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="idea-text" ondblclick={start} onpointerdown={(e) => itemDrag.press(e, item)}>{item.text}</span>
    {/if}
    <!-- Its steps' progress: an idea has no checkbox, so a small pie beside the count. -->
    {#if steps.length && !editing}
      {@const all = stepsDone === steps.length}
      {@const label = t("{a} of {n} steps done", { a: stepsDone, n: steps.length })}
      <span class="count" class:all transition:widen={{ gap: 10 }} data-tip={label}
        ><StepTrace shape="pie" size={12} stroke={1.25} done={stepsDone} total={steps.length} /><span aria-hidden="true">{stepsDone}/{steps.length}</span><span class="visually-hidden">{label}</span></span
      >
    {/if}
    <span class="acts">
      <button class="make" aria-label={t("Make it a task: “{name}”", { name: item.text })} data-tip={t("Make it a task")} data-tip-keys="Backspace" onclick={() => store.flipKind(item)}>{t("Make it a task")}</button>
      <button class="act" data-tip={t("Delete")} data-tip-keys="Delete" aria-label={t("Delete “{name}”", { name: item.text })} onclick={() => store.deleteItem(item)}><Icon name="trash" size={14} /></button>
    </span>
  </div>
  {#if item.link}<div class="idea-attach" data-menu-line={item.id} transition:fold use:snap={lined}><LinkCard link={item.link} /></div>{/if}
  {#if item.file}<div class="idea-attach" data-menu-line={item.id} transition:fold use:snap={lined}><FileCard path={item.file} /></div>{/if}
  {#if steps.length}
    <ul class="idea-steps" transition:fold>
      {#each steps as s (s.id)}
        <ItemRow item={s} nested tint />
      {/each}
    </ul>
  {/if}
</svelte:element>

<style>
  .idea-wrap {
    transition: opacity 0.15s;
  }
  .idea-wrap.carried {
    opacity: 0.35;
  }
  .idea {
    position: relative;
    display: flex;
    align-items: flex-start;
    min-height: 22px;
    margin: -3px -6px;
    padding: 3px 6px;
    border-radius: var(--r-sm);
    transition: box-shadow 0.15s;
  }
  .idea.drop {
    box-shadow: inset 0 0 0 1.5px var(--idea-ink);
  }
  .idea.drop::after {
    content: attr(data-drop-label);
    position: absolute;
    right: 6px;
    bottom: -10px;
    z-index: 2;
    padding: 1px 7px;
    border-radius: 5px;
    background: var(--ink);
    color: var(--card);
    font-family: var(--sans);
    font-style: normal;
    font-size: var(--t-xs);
    font-weight: 600;
    pointer-events: none;
  }
  /* The steps' count beside an idea, after its small ring. */
  .count {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: 4px 0 0 10px;
    font-size: var(--t-xs);
    font-weight: 600;
    line-height: 14px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    --trace: var(--idea-ink);
    transition: color 0.2s;
  }
  .count.all {
    color: var(--soft);
  }
  .idea-text {
    flex: 1;
    min-width: 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: 16.5px;
    line-height: 1.32;
    color: var(--idea-ink);
    overflow-wrap: anywhere;
    cursor: grab;
    user-select: none;
  }
  .line-input {
    flex: 1;
    min-width: 0;
    height: 24px;
    margin: -2px 0 -2px -6px;
    padding: 0 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-size: var(--t-base);
    color: var(--ink);
    outline: 0;
  }
  .idea-input {
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
    height: 26px;
  }
  .acts {
    position: absolute;
    top: 1px;
    right: 0;
    display: flex;
    gap: 2px;
    padding-left: var(--s4);
    background: linear-gradient(to right, transparent, var(--tint) 14px);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s;
  }
  .idea:hover .acts,
  .acts:focus-within {
    opacity: 1;
    pointer-events: auto;
  }
  .act {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .act:hover {
    background: var(--card);
    color: var(--ink);
  }
  .make {
    height: 26px;
    padding: 0 9px;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--card);
    box-shadow: 0 0 0 1px var(--corner);
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    cursor: pointer;
    white-space: nowrap;
  }
  .make:hover {
    color: var(--ink);
  }
  .idea-attach {
    padding: 6px 0 2px;
  }
  .idea-steps {
    margin: 4px 0 0 4px;
    padding: 0 0 0 6px;
    border-left: 1.5px solid var(--corner);
  }
  .idea.focused {
    background: color-mix(in srgb, var(--focus-ring) 12%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--focus-ring);
  }

  /* On ruled and grid paper every row is one 24 px line. */
  .lined .count {
    margin-top: 5px;
  }
  .lined .idea {
    margin: 0 -6px;
    padding: 0 6px;
    min-height: 24px;
  }
  .lined .idea-text {
    line-height: 24px;
  }
  .lined .idea-steps {
    margin-top: 0;
  }
</style>
