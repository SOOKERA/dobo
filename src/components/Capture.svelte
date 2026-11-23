<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { completeTag, findNoteForTag, parseCapture, suggestNotes, tagAt, tagOf, titleFromTag } from "../lib/capture";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import { clockDate, formatDueLong } from "../lib/time";
  import type { Kind, Note } from "../lib/types";
  import { i18n, keysText, t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";
  import { arrive, crumbIn, crumbOut, easeHeight, leave, pop, widen } from "../lib/motion";
  import { composing } from "../lib/compose";

  let { input = $bindable() }: { input?: HTMLInputElement } = $props();

  let text = $state("");
  let busy = $state(false);

  let preview = $derived.by(() => {
    if (!text.trim()) return null;
    const p = parseCapture(text, "task", store.now);
    const tagged = p.tag ? findNoteForTag(p.tag, store.notes, store.space) : null;
    const into = p.tag
      ? tagged
        ? noteTitle(tagged)
        : t("New note “{name}”", { name: titleFromTag(p.tag) })
      : noteTitle(store.notes.find((n) => n.id === store.target && !n.archived) ?? null);
    return { due: p.due, into, deadline: p.deadline && !!p.due };
  });

  let targetNote = $derived(store.notes.find((n) => n.id === store.target && !n.archived) ?? null);
  // What the line under the field shows: the reading of what's typed, the note it goes into,
  // or the hint. Each one crossfades into the next.
  let mode = $derived(preview ? "preview" : targetNote ? "target" : "hint");
  let previewHint = $derived(preview?.deadline ? "deadline" : preview?.due ? "due" : "");
  /** The time's hint, whole when it fits beside the reading, else its short form ("Deadline: end
   * with !"); when even that doesn't fit, it goes whole onto the line that never shows (`.set`).
   * Tried whole first each time the room changes (the window, the reading's chips, the language). */
  function fitHint(node: HTMLElement, _lang: string) {
    const set = node.closest<HTMLElement>(".set");
    const check = () => {
      if (!set || !node.querySelector(".due-full")) return;
      node.classList.remove("brief");
      if (node.offsetTop - set.offsetTop >= set.clientHeight - 1) node.classList.add("brief");
    };
    const room = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(check);
    const chips = node.closest(".chips");
    const pair = set?.querySelector(".pair");
    if (chips) room?.observe(chips);
    if (pair) room?.observe(pair);
    return {
      update: check,
      destroy: () => room?.disconnect(),
    };
  }
  let clock = $derived(clockDate(store.now));
  // For screen readers, what the line will do (its time, its note), once typing pauses and only
  // when that changes: not every key, never the clock.
  let reading = $derived(preview ? [preview.due ? formatDueLong(preview.due.getTime(), store.now) : "", preview.into].filter(Boolean).join(" · ") : "");
  let spoken = $state("");
  $effect(() => {
    const r = reading;
    const id = setTimeout(() => (spoken = r), 700);
    return () => clearTimeout(id);
  });

  // ---- # suggestions ----
  // As soon as a "#" is typed, the notes it may mean show under the field, narrowing as you type
  // (the same matching as filing: whole name, starts with, contains; the space new lines go to
  // first). The best one is lit; Up / Down move; Space completes the lit note's tag (Tab and Enter
  // too once Up / Down have been used); Esc closes the list and keeps the text. Ignored, nothing
  // changes: Enter files the line as it always did (a new note when nothing matches).
  let caret = $state(0);
  let focused = $state(false);
  /** The #word the list was closed on (Esc): it stays closed until another one is typed. */
  let closedAt = $state<number | null>(null);
  let sel = $state(0);
  /** Up / Down were used: the list has the keys (Tab and Enter pick from it too). */
  let engaged = $state(false);
  // (While a character is being composed, a dead key's "^" before its "ê", the list holds still
  // on the #word as it was, rather than closing on the "^" and opening again.)
  let composingNow = $state(false);
  let heldAt: ReturnType<typeof tagAt> = null;
  let at = $derived.by(() => {
    if (!focused) return (heldAt = null);
    if (composingNow && heldAt) return heldAt;
    return (heldAt = tagAt(text, caret));
  });
  let hits = $derived(at ? suggestNotes(at.query, store.notes, store.space) : []);
  let listOpen = $derived(!!at && hits.length > 0 && closedAt !== at.start);
  let lit = $derived(Math.min(sel, Math.max(0, hits.length - 1)));
  /** Space completes the lit note once something follows the "#" (or Up / Down chose one). */
  let spacePicks = $derived(!!at?.query || engaged);
  // A new #word, or other notes in the list: the best match is lit again.
  let sig = $derived(at ? `${at.start}|${hits.map((n) => n.id).join(",")}` : "");
  $effect(() => {
    void sig;
    sel = 0;
    engaged = false;
  });
  $effect(() => {
    if (!at || (closedAt != null && closedAt !== at.start)) closedAt = null;
  });

  function track() {
    if (input) caret = input.selectionStart ?? text.length;
  }
  function pick(i: number) {
    const n = hits[i];
    if (!at || !n || !input) return;
    const done = completeTag(text, at, n);
    text = done.text;
    caret = done.caret;
    // (The field shows the new text first, then the caret goes after the tag.)
    queueMicrotask(() => {
      input?.setSelectionRange(done.caret, done.caret);
      track();
    });
  }
  /** The keys the list takes while it shows; true when one was used. */
  function listKey(e: KeyboardEvent): boolean {
    // (Not while a character is being composed: a dead key's accent, an IME's word.)
    if (!listOpen || !at || composing(e)) return false;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const d = e.key === "ArrowDown" ? 1 : -1;
      sel = (lit + d + hits.length) % hits.length;
      engaged = true;
    } else if (e.key === " " && !e.ctrlKey && !e.altKey && !e.metaKey && spacePicks) pick(lit);
    // (A bare "#" then Space, as in "# 5": just a space, unless a note was chosen with Up / Down.)
    else if ((e.key === "Tab" || e.key === "Enter") && engaged && !e.shiftKey) pick(lit);
    else if (e.key === "Escape") {
      // (The list closes, not the panel; the text stays.)
      e.stopPropagation();
      closedAt = at.start;
    } else return false;
    e.preventDefault();
    return true;
  }
  const toneOf = (n: { id: number; palette: Note["palette"]; color: string }) =>
    store.inboxIds.has(n.id) ? "var(--box)" : swatchOf(n, store.custom)[look.dark ? "dark" : "light"].dot;
  const SPACE_NAME = { work: "Work", personal: "Personal" } as const;

  async function submit(kind: Kind) {
    if (!text.trim() || busy) return;
    busy = true;
    try {
      const made = await store.capture(text, kind);
      if (made) text = "";
    } finally {
      busy = false;
      input?.focus();
    }
  }

  function onKey(e: KeyboardEvent) {
    if (listKey(e)) return;
    if (e.key === "Enter" && !composing(e)) {
      e.preventDefault();
      submit(e.shiftKey ? "idea" : "task");
    }
  }
</script>

<div class="capture">
  <div class="field" data-tour="capture">
    <Icon name="plus" size={16} stroke={2} />
    <label for="capture" class="visually-hidden">{t("New task or idea")}</label>
    <input
      id="capture"
      bind:this={input}
      bind:value={text}
      onkeydown={onKey}
      oninput={track}
      onkeyup={track}
      onclick={track}
      onselect={track}
      onfocus={() => ((focused = true), track())}
      onblur={() => ((focused = false), (composingNow = false))}
      oncompositionstart={() => (composingNow = true)}
      oncompositionend={() => ((composingNow = false), track())}
      type="text"
      autocomplete="off"
      spellcheck="true"
      placeholder={t("Jot something down")}
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={listOpen}
      aria-controls="tag-list"
      aria-activedescendant={listOpen && hits[lit] ? `tag-${hits[lit].id}` : undefined}
    />
    <div class="kinds">
      <button class="kind task" onclick={() => submit("task")} disabled={!text.trim()}>{t("Task")} <span class="key">{keysText("Enter")}</span></button>
      <button class="kind idea" onclick={() => submit("idea")} disabled={!text.trim()}>{t("Idea")} <span class="key">{keysText("Shift+Enter")}</span></button>
    </div>
  </div>
  {#if listOpen}
    <!-- Under the field (never over what is typed), over the hint line. -->
    <div class="tags" transition:pop={{ duration: 160 }}>
      <div class="room" use:easeHeight={{ grow: 200, shrink: 180 }}>
        <div>
          <ul id="tag-list" role="listbox" aria-label={t("Notes for #{tag}", { tag: at?.query ?? "" })}>
            <span class="lit" aria-hidden="true" style:transform="translateY({lit * 34}px)"></span>
            {#each hits as n, i (n.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events (the field keeps the keys: Up / Down, Space) -->
              <li id="tag-{n.id}" role="option" aria-selected={i === lit} class:on={i === lit} onpointerdown={(e) => (e.preventDefault(), pick(i))} onpointermove={() => (sel = i)}>
                <span class="ndot" style:background={toneOf(n)}></span>
                <span class="nname">{noteTitle(n)}</span>
                {#if store.view === "all" || n.space !== store.view}<span class="nspace">{t(SPACE_NAME[n.space])}</span>{/if}
                <span class="ntag">#{tagOf(n)}</span>
              </li>
            {/each}
          </ul>
          <!-- (After a bare "#", Space is just a space until a note is chosen with Up / Down.) -->
          <p class="keys" aria-hidden="true">
            {#key spacePicks}<span class="keys-words" in:crumbIn out:crumbOut
                >{spacePicks
                  ? t("{space} picks · {arrows} move · {esc} closes", { space: keysText("Space"), arrows: "↑↓", esc: keysText("Esc") })
                  : t("{arrows} to choose · {esc} closes", { arrows: "↑↓", esc: keysText("Esc") })}</span
              >{/key}
          </p>
        </div>
      </div>
      <span class="visually-hidden" aria-live="polite">{t("{n} notes match", { n: hits.length })}</span>
    </div>
  {/if}
  <span class="visually-hidden" aria-live="polite">{spoken}</span>
  <div class="chips" data-tour="capture-hints">
    <span class="what">
      {#key mode}
        <span class="set" in:arrive={{ delay: 70, duration: 200, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
          {#if mode === "preview" && preview}
            <span class="pair">
              {#if preview.due}
                <span class="chip" transition:widen><Icon name={preview.deadline ? "hourglass" : "clock"} size={12} stroke={2.2} /><span class="label">{formatDueLong(preview.due.getTime(), store.now)}</span></span>
              {/if}
              <span class="chip into"><span class="label">{preview.into}</span></span>
            </span>
            <span class="hints">
              {#key previewHint}
                <span class="hint" in:crumbIn out:crumbOut use:fitHint={i18n.lang}
                  >{#if previewHint === "due"}<span class="due-full">{t("Reminds you at the time · end with\u00a0! for a deadline")}</span><span class="due-brief"
                      >{t("Deadline: end with\u00a0!")}</span
                    >{:else if previewHint === "deadline"}{t("Deadline: rings early, then more often")}{/if}</span
                >
              {/key}
            </span>
          {:else if mode === "target" && targetNote}
            <span class="chip into"><span class="label">{t("Into {title}", { title: noteTitle(targetNote) })}</span><button class="unchip" aria-label={t("Back to the Inbox")} onclick={() => (store.target = null)}><Icon name="close" size={12} stroke={2.2} /></button></span>
          {:else}
            <!-- The whole hint, or at narrow widths the same three examples alone (never cut, never wrapped). -->
            <span class="hint"><span class="full">{t("Add a time like “3pm” or “tomorrow 9am”, and #note to pick where it goes")}</span><span class="brief">{t("“3pm”, “tomorrow 9am”, #note")}</span></span>
          {/if}
        </span>
      {/key}
    </span>
    <span class="clock" aria-label={t("Now")}>
      {#key clock}
        <span class="tick" in:crumbIn out:crumbOut><span class="long">{clock}</span><span class="short">{clockDate(store.now, true)}</span></span>
      {/key}
    </span>
  </div>
</div>

<style>
  .capture {
    position: relative;
    z-index: 5;
    padding: 0 var(--s6);
    flex-shrink: 0;
  }

  /* ---- # suggestions: a small list under the field ---- */
  .tags {
    position: absolute;
    top: 56px;
    left: calc(var(--s6) + 30px);
    z-index: 50;
    width: min(400px, calc(100% - 2 * var(--s6) - 30px));
    padding: 6px;
    border-radius: var(--r-lg);
    background: var(--card);
    box-shadow: var(--pop-shadow);
    transform-origin: 24px 0;
  }
  .tags ul {
    position: relative;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  /* The lit row: it travels from one to the next. */
  .lit {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 34px;
    border-radius: var(--r-md);
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: transform 0.14s var(--ease-out);
    pointer-events: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .lit {
      transition: none;
    }
  }
  .tags li {
    position: relative;
    height: 34px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-radius: var(--r-md);
    font-size: var(--t-base);
    color: var(--soft);
    cursor: pointer;
  }
  .tags li.on {
    color: var(--ink);
  }
  .ndot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  /* The name keeps its room; the tag gives way first. */
  .nname {
    flex: 0 1 auto;
    min-width: 3em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nspace {
    flex-shrink: 0;
    font-size: var(--t-xs);
    color: var(--muted);
  }
  .ntag {
    flex: 1 1 0;
    min-width: 0;
    text-align: right;
    padding-left: var(--s2);
    font-size: var(--t-sm);
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .keys {
    display: grid;
    margin: 4px 10px 2px;
    font-size: var(--t-xs);
    color: var(--muted);
  }
  .keys-words {
    grid-area: 1 / 1;
  }
  .field {
    height: 52px;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 0 6px 0 var(--s4);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--card-shadow);
    color: var(--muted);
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .field:focus-within {
    border-color: var(--box);
    box-shadow: var(--card-shadow), 0 0 0 3px rgba(233, 196, 106, 0.35);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-lg);
    color: var(--ink);
  }
  input::placeholder {
    color: var(--muted);
  }
  .kinds {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--chip);
    flex-shrink: 0;
  }
  .kind {
    height: 34px;
    padding: 0 var(--s3);
    border: 0;
    border-radius: 7px;
    display: flex;
    align-items: center;
    gap: var(--s2);
    cursor: pointer;
    transition: opacity 0.15s, background-color 0.15s;
  }
  .kind:disabled {
    cursor: default;
  }
  .kind.task {
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-md);
    font-weight: 500;
  }
  .kind.task:disabled {
    opacity: 0.55;
  }
  .kind.idea {
    background: transparent;
    color: var(--soft);
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
  }
  .kind.idea:not(:disabled):hover {
    background: var(--hover);
  }
  .key {
    font-family: var(--sans);
    font-style: normal;
    font-size: var(--t-sm);
    opacity: 0.7;
  }
  .chips {
    min-height: 24px;
    margin-top: var(--s2);
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--t-sm);
    color: var(--soft);
    overflow: hidden;
    white-space: nowrap;
  }
  /* The reading, the target or the hint: one place; the one leaving steps out of the flow. */
  .what {
    position: relative;
    display: flex;
    min-width: 0;
    flex: 0 1 auto;
  }
  /* In a narrow line the reading's hint goes whole rather than cut short: it wraps onto a second
     line that never shows. The time and the note stay; only a long note name shortens. */
  .set {
    height: 24px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: var(--s2);
    min-width: 0;
    overflow: hidden;
  }
  .pair {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
  }
  .hints {
    display: grid;
  }
  .hints > .hint {
    grid-area: 1 / 1;
  }
  .chip {
    height: 24px;
    padding: 0 var(--s2);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: var(--r-sm);
    background: var(--card);
    border: 1px solid var(--line);
    max-width: 260px;
    flex-shrink: 0;
  }
  .chip.into {
    flex-shrink: 1;
    min-width: 3.5em;
  }
  .chip .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .unchip {
    width: 18px;
    height: 18px;
    margin-right: -4px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .unchip:hover {
    background: var(--hover);
  }
  /* The time's hint whole, or its short form when that doesn't fit (`fitHint`). */
  .hint .due-brief,
  .hint:global(.brief) .due-full {
    display: none;
  }
  .hint:global(.brief) .due-brief {
    display: inline;
  }
  .hint {
    min-width: 0;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The date and time, quiet at the end of the line; the hint gives way before it does. */
  .clock {
    display: grid;
    justify-items: end;
    flex-shrink: 0;
    margin-left: auto;
    padding-left: var(--s2);
    font-family: var(--serif);
    font-style: italic;
    font-size: 14px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .clock > .tick {
    grid-area: 1 / 1;
  }
  .clock .short,
  .hint .brief {
    display: none;
  }
  /* The whole hint needs about 620 px beside the date and a 12-hour time, with room to spare: below that, its three examples alone. */
  @container shell (max-width: 640px) {
    .hint .full {
      display: none;
    }
    .hint .brief {
      display: inline;
    }
  }
  @container shell (max-width: 520px) {
    .key {
      display: none;
    }
    .clock .long {
      display: none;
    }
    .clock .short {
      display: inline;
    }
  }
</style>
