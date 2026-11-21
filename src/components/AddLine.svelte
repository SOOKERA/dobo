<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "+ Add" at the end of a note: a quiet row that becomes a field. Enter adds a task, Shift+Enter
  // an idea (as in the entry line); a time, a repeat and a "!" deadline are read as there. It stays
  // open for the next line; Esc or a click away closes it, and what was typed is kept (added),
  // never lost. A click in the note's empty space opens it too (`active`).
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { parseCapture } from "../lib/capture";
  import { formatDueLong } from "../lib/time";
  import { arrive, leave, widen } from "../lib/motion";
  import { keysText, t } from "../lib/i18n.svelte";
  import { composing } from "../lib/compose";
  import type { Kind } from "../lib/types";

  let {
    noteId,
    name,
    lined = false,
    tint = null,
    after = "tasks",
    active = $bindable(false),
  }: {
    noteId: number;
    /** The note's name as it shows (for screen readers). */
    name: string;
    lined?: boolean;
    /** The ideas' tint when the row follows them (it carries on to the card's bottom edge). */
    tint?: string | null;
    /** What the row follows, for its spacing. */
    after?: "tasks" | "ideas" | "pictures" | "empty";
    active?: boolean;
  } = $props();

  let draft = $state("");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();
  let parsed = $derived(draft.trim() ? parseCapture(draft, "task", store.now, { tags: false }) : null);
  let due = $derived(parsed?.due ? { at: parsed.due.getTime(), deadline: parsed.deadline } : null);

  // The field takes the keys as it opens (also when opened again while still leaving).
  $effect(() => {
    if (active && input && document.activeElement !== input) input.focus();
  });

  async function add(kind: Kind) {
    const raw = draft;
    if (!raw.trim() || busy) return;
    busy = true;
    draft = "";
    try {
      const made = await store.addLine(noteId, raw, kind);
      // (Nothing made: the words stay to be fixed.)
      if (!made && !draft) draft = raw;
    } catch (e) {
      // (Not saved: the words come back rather than being lost.)
      if (!draft) draft = raw;
      throw e;
    } finally {
      busy = false;
    }
  }
  /** Closes the field; what was typed is added first (as a task), never thrown away. */
  async function close() {
    if (!active) return;
    active = false;
    if (draft.trim()) await add("task");
  }
  function onKey(e: KeyboardEvent) {
    // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
    if (composing(e)) return;
    if (e.key === "Enter") {
      e.preventDefault();
      void add(e.shiftKey ? "idea" : "task");
    } else if (e.key === "Escape") {
      // (Esc closes the field, not what is under it.)
      e.stopPropagation();
      e.preventDefault();
      void close();
    }
  }
</script>

<div class="add-row after-{after}" class:lined class:tinted={!!tint} style:--add-tint={tint}>
  <span class="swap">
    {#if active}
      <span class="field" in:arrive={{ delay: 40, duration: 200, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
        <span class="mark"><Icon name="plus" size={14} stroke={2} /></span>
        <input
          bind:this={input}
          bind:value={draft}
          type="text"
          autocomplete="off"
          spellcheck="true"
          placeholder={t("A task, or {keys} for an idea", { keys: keysText("Shift+Enter") })}
          aria-label={t("Add a line to {name}", { name })}
          onkeydown={onKey}
          onblur={close}
        />
        {#if due}
          <span class="chip" class:deadline={due.deadline} transition:widen={{ gap: 6 }}
            ><Icon name={due.deadline ? "hourglass" : "clock"} size={11} stroke={2.2} /><span>{formatDueLong(due.at, store.now)}</span></span
          >
        {/if}
      </span>
    {:else}
      <button class="add" in:arrive={{ delay: 40, duration: 200, y: 0 }} out:leave={{ duration: 110, y: 0 }} aria-label={t("Add a line to {name}", { name })} data-tip={t("Add a task or an idea here")} onclick={() => (active = true)}
        ><span class="mark"><Icon name="plus" size={14} stroke={2} /></span><span>{t("Add")}</span></button
      >
    {/if}
  </span>
</div>

<style>
  /* One row, like a task's: the "+" where the checkboxes are, the words where their text is. */
  .add-row {
    padding: 0 var(--s2) 6px;
    border-radius: 0 0 11px 11px;
    background-color: transparent;
    transition: background-color 0.24s var(--ease-out);
  }
  /* After the ideas, the row carries their tint to the card's bottom edge. */
  .add-row.tinted {
    background-color: var(--add-tint);
  }
  .after-tasks,
  .after-empty {
    margin-top: -8px;
  }
  .after-pictures {
    margin-top: -4px;
  }
  .swap {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .swap > * {
    grid-area: 1 / 1;
  }
  .add,
  .field {
    height: 30px;
    padding: 0 var(--s2);
    display: flex;
    align-items: center;
    gap: 10px;
    border-radius: var(--r-sm);
  }
  .mark {
    flex-shrink: 0;
    width: 16px;
    display: grid;
    place-items: center;
  }
  .add {
    justify-self: start;
    padding-right: 10px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--t-md);
    cursor: text;
    opacity: 0.75;
    transition:
      opacity 0.15s,
      color 0.15s,
      background-color 0.15s;
  }
  .add:hover,
  .add:focus-visible {
    opacity: 1;
    color: var(--ink);
    background: var(--hover);
  }
  .field {
    color: var(--muted);
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .tinted .field {
    background: color-mix(in srgb, var(--card) 70%, var(--add-tint));
  }
  .field:focus-within {
    box-shadow: inset 0 0 0 1px var(--box);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-base);
    color: var(--ink);
  }
  input::placeholder {
    color: var(--muted);
  }
  .chip {
    flex-shrink: 0;
    height: 20px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: 5px;
    background: var(--chip);
    color: var(--soft);
    font-size: var(--t-xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  /* On ruled and grid paper: one 24 px line, its words on the rule. */
  .lined {
    padding-bottom: 12px;
  }
  .lined.tinted {
    background-image: var(--lines);
  }
  .lined.after-tasks,
  .lined.after-empty,
  .lined.after-pictures {
    margin-top: 0;
  }
  .lined .add,
  .lined .field {
    height: 24px;
  }
</style>
