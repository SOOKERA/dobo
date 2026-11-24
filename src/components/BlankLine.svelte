<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A blank line waiting in a note made from a template (Today, Ideas, Plan): an empty task (its
  // box) or an empty idea, on a faint dotted rule, ready to type. Enter (or a click away) makes
  // it a real line of the note, after its lines of that kind; this blank goes, the others stay as
  // they are. Enter goes on to the next blank (else on down the note); Backspace in an empty one
  // takes it away and goes to the one before. Blanks are never saved (store.blanks): they don't
  // count as "to do", and a note shows "All done" only once none wait.
  import { tick } from "svelte";
  import { store } from "../lib/store.svelte";
  import { nextBlank } from "../lib/newNote";
  import { composing } from "../lib/compose";
  import { fold } from "../lib/motion";
  import { t } from "../lib/i18n.svelte";
  import type { Kind } from "../lib/types";

  let {
    noteId,
    id,
    kind,
    first = false,
    lined = false,
    name,
    onEnd,
  }: {
    noteId: number;
    /** This blank's own id (store.blanks). */
    id: number;
    kind: Kind;
    /** The first blank of its kind: it says what goes there. */
    first?: boolean;
    lined?: boolean;
    /** The note's name as it shows (for screen readers). */
    name: string;
    /** No blank of its kind waits any more after Enter (or Backspace): the keyboard goes on
     * (NoteCard). */
    onEnd: (kind: Kind) => void;
  } = $props();

  let draft = $state("");
  let busy = $state(false);

  let inputEl = $state<HTMLInputElement>();

  /** The keyboard to another blank of this note (by its id), else on down the note. (The note is
   * found first: this blank may be gone by then.) */
  async function goTo(to: number | null, card = inputEl?.closest("[data-drop-note]")) {
    await tick();
    const el = to == null ? null : card?.querySelector<HTMLInputElement>(`[data-blank="${to}"]`);
    if (el) el.focus({ preventScroll: true });
    else onEnd(kind);
  }

  async function fill(byEnter: boolean) {
    const raw = draft;
    if (!raw.trim() || busy) return;
    busy = true;
    draft = "";
    // (Where the keyboard goes next is known before this blank goes.)
    const next = nextBlank(store.blanks[noteId]?.[kind] ?? [], id, "enter");
    const card = inputEl?.closest("[data-drop-note]");
    try {
      const made = await store.fillBlank(noteId, kind, id, raw);
      if (!made && !draft) draft = raw;
      else if (made && byEnter) void goTo(next, card);
    } catch (e) {
      if (!draft) draft = raw;
      throw e;
    } finally {
      busy = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (composing(e)) return;
    const el = e.currentTarget as HTMLInputElement;
    if (e.key === "Enter") {
      e.preventDefault();
      void fill(true);
    } else if (e.key === "Escape") {
      // (Esc leaves the line, not what is under it; what was typed is kept, on the blur.)
      e.stopPropagation();
      e.preventDefault();
      el.blur();
    } else if (e.key === "Backspace" && !draft && !busy) {
      e.preventDefault();
      const next = nextBlank(store.blanks[noteId]?.[kind] ?? [], id, "backspace");
      const card = inputEl?.closest("[data-drop-note]");
      store.dropBlank(noteId, kind, id);
      void goTo(next, card);
    }
  }
</script>

{#if kind === "task"}
  <li class="wrap" transition:fold>
    <div class="row blank">
      <span class="box" aria-hidden="true"></span>
      <input class="text" bind:this={inputEl} data-blank={id} bind:value={draft} type="text" autocomplete="off" spellcheck="true" placeholder={first ? t("A task") : ""} aria-label={t("Add a line to {name}", { name })} onkeydown={onKey} onblur={() => void fill(false)} />
    </div>
  </li>
{:else}
  <div class="idea-blank" class:lined transition:fold={{ gap: lined ? 0 : 7 }}>
    <input class="idea-text" bind:this={inputEl} data-blank={id} bind:value={draft} type="text" autocomplete="off" spellcheck="true" placeholder={first ? t("An idea") : ""} aria-label={t("Add a line to {name}", { name })} onkeydown={onKey} onblur={() => void fill(false)} />
  </div>
{/if}

<style>
  .wrap {
    list-style: none;
  }
  /* As a task's row: its box where the boxes are, its words where theirs are. */
  .row {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 6px var(--s2);
    border-radius: var(--r-sm);
  }
  .box {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin-top: 2px;
    border: 1.5px dashed var(--box);
    border-radius: 4px;
    transition: border-color 0.18s;
  }
  .row:focus-within .box {
    border-style: solid;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 1lh;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    outline: 0;
    font: inherit;
    color: var(--ink);
    cursor: text;
    /* A faint dotted rule to write on, firmer while writing. */
    --rule-ink: color-mix(in srgb, var(--box) 70%, transparent);
    background: repeating-linear-gradient(to right, var(--rule-ink) 0 2px, transparent 2px 5px) left bottom / 100% 1px no-repeat;
    transition: background-size 0.2s;
  }
  input:focus {
    --rule-ink: var(--box);
  }
  input::placeholder {
    color: var(--muted);
    opacity: 1;
  }
  .text {
    font-size: var(--t-base);
    line-height: 1.4;
  }
  /* As an idea: its italic words on the ideas' band. */
  .idea-blank {
    display: flex;
    min-height: 22px;
    margin: -3px -6px;
    padding: 3px 6px;
  }
  .idea-text {
    font-family: var(--serif);
    font-style: italic;
    font-size: 16.5px;
    line-height: 1.32;
    color: var(--idea-ink);
  }
  .idea-text::placeholder {
    color: color-mix(in srgb, var(--idea-ink) 55%, transparent);
  }
  /* On ruled and grid paper: one 24 px line, written on the paper's own rule. */
  .idea-blank.lined {
    margin: 0 -6px;
    padding: 0 6px;
    min-height: 24px;
  }
  .idea-blank.lined .idea-text {
    line-height: 24px;
  }
  :global(.lined) input {
    background: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .box,
    input {
      transition: none;
    }
  }
</style>
