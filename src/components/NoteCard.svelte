<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import NoteMenu from "./NoteMenu.svelte";
  import ItemRow from "./ItemRow.svelte";
  import IdeaRow from "./IdeaRow.svelte";
  import AddLine from "./AddLine.svelte";
  import NotePictures from "./NotePictures.svelte";
  import { store, NO_PICTURES } from "../lib/store.svelte";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { fileDrop } from "../lib/filedrop.svelte";
  import { snap } from "../lib/snap";
  import { fold, fadeQuick } from "../lib/motion";
  import type { Tone } from "../lib/palette";
  import type { Item, Note } from "../lib/types";
  import { around, t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";
  import type { Point } from "../lib/contextmenu";

  let {
    note,
    items,
    tone,
    menuOpen,
    menuAt = null,
    lifted,
    tilt = -2.5,
    onMenu,
    onGrab,
    onWiden = null,
    width = null,
  }: {
    note: Note;
    items: Item[];
    tone: Tone;
    menuOpen: boolean;
    /** Its menu opened by a right-click: where (else under its ⋯). */
    menuAt?: Point | null;
    lifted: boolean;
    /** While lifted: its tilt (deg), swaying with the carry. */
    tilt?: number;
    onMenu: (open: boolean) => void;
    onGrab: (e: PointerEvent) => void;
    onWiden?: ((e: PointerEvent) => void) | null;
    /** Its width on the board (its pictures show from their thumbs when sharp enough). */
    width?: number | null;
  } = $props();

  let lined = $derived(note.paper === "ruled" || note.paper === "grid");
  let focused = $derived(store.focus?.note === note.id && store.focus.item == null);

  function toggleFold() {
    store.setFolded(note, !note.folded);
  }

  let tasks = $derived([...items.filter((i) => i.kind === "task" && i.done_at == null), ...items.filter((i) => i.kind === "task" && i.done_at != null)]);
  let ideas = $derived(items.filter((i) => i.kind === "idea"));
  // (The quick tour's lines live in the Inbox strip, never in a note.)
  let open = $derived(tasks.filter((t) => t.done_at == null).length);
  let pictures = $derived(store.picturesByNote.get(note.id) ?? NO_PICTURES);
  let meta = $derived(
    [tasks.length ? t("{n} to do", { n: open }) : "", ideas.length ? t("{n} ideas", { n: ideas.length }) : ""].filter(Boolean).join(" · "),
  );
  let dropHere = $derived(
    (itemDrag.target?.kind === "note" && itemDrag.target.id === note.id) || fileDrop.note === note.id,
  );

  // Glow when jumped to (search, a notification).
  let cardEl: HTMLElement;
  $effect(() => {
    const g = store.glow;
    if (!g || g.id !== note.id || !cardEl) return;
    cardEl.scrollIntoView({ block: "center", behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
    cardEl.animate(
      [
        { boxShadow: "0 0 0 0 rgba(233,196,106,0)" },
        { boxShadow: "0 0 0 4px rgba(233,196,106,0.75)", offset: 0.25 },
        { boxShadow: "0 0 0 4px rgba(233,196,106,0.6)", offset: 0.7 },
        { boxShadow: "0 0 0 0 rgba(233,196,106,0)" },
      ],
      { duration: 1600, easing: "ease-out" },
    );
  });

  let editingTitle = $state(false);
  let titleDraft = $state("");

  function focusSelect(el: HTMLInputElement) {
    el.focus();
    el.select();
  }

  /** The #tag that finds this note: its first word ("#hub"). */
  function tagFor(n: { title: string }): string {
    return noteTitle(n).toLowerCase().split(/\s/)[0];
  }

  function startTitle() {
    titleDraft = noteTitle(note);
    editingTitle = true;
  }
  async function endTitle(save: boolean) {
    if (!editingTitle) return;
    editingTitle = false;
    if (save) await store.rename(note, titleDraft);
  }

  // "+ Add" at the end of the note; a click in the note's empty space opens it too.
  let adding = $state(false);
  let last = $derived<"tasks" | "ideas" | "pictures" | "empty">(ideas.length ? "ideas" : pictures.length ? "pictures" : tasks.length ? "tasks" : "empty");
  const NOT_EMPTY = "button, input, textarea, a, [data-line], .add-row, .pics, .attach, .idea-attach, [role='dialog'], [role='menu'], [role='group']";
  /** Where a press in the note's empty space began (a click that ends a line's carry, its press
   * on the line and its release over the empty space, isn't one). */
  let pressedEmpty: { x: number; y: number } | null = null;
  function emptyPress(e: PointerEvent) {
    pressedEmpty = e.button === 0 && !(e.target as HTMLElement).closest(NOT_EMPTY) ? { x: e.clientX, y: e.clientY } : null;
  }
  function emptyClick(e: MouseEvent) {
    const p = pressedEmpty;
    pressedEmpty = null;
    if (!p || Math.hypot(e.clientX - p.x, e.clientY - p.y) > 6) return;
    if ((e.target as HTMLElement).closest(NOT_EMPTY)) return;
    if (window.getSelection()?.toString()) return;
    adding = true;
  }

  function grab(e: PointerEvent) {
    if (e.button !== 0 || editingTitle) return;
    if ((e.target as HTMLElement).closest("button, input")) return;
    onGrab(e);
  }
</script>

<article
  class="card paper-{note.paper ?? 'plain'}"
  class:lined
  class:lifted
  class:folded={note.folded}
  class:focused
  class:drop={dropHere}
  aria-label={noteTitle(note)}
  data-drop-note={note.id}
  bind:this={cardEl}
  style:--rule={tone.corner}
  style:--lift-tilt={lifted ? `${tilt.toFixed(2)}deg` : null}
>
  <div class="lift-shadow" aria-hidden="true"></div>
  <div class="peel" style:--peel={tone.corner} aria-hidden="true"></div>
  {#if onWiden && !note.folded}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="grip" transition:fadeQuick data-tip={note.wide ? t("Drag left to make it normal width") : t("Drag right to make it wide")} onpointerdown={onWiden} aria-hidden="true"><span></span></div>
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="head" class:editing={editingTitle} onpointerdown={grab} ondblclick={startTitle}>
    <button class="dot" aria-label={t("Colour and options for “{name}”", { name: noteTitle(note) })} data-tip={t("Note colour and options")} onclick={() => onMenu(!menuOpen)}><span style:background={tone.dot}></span></button>
    {#if editingTitle}
      <input
        class="title-input"
        bind:value={titleDraft}
        use:focusSelect
        aria-label={t("Note title")}
        onkeydown={(e) => {
          if (composing(e)) return;
          if (e.key === "Enter") endTitle(true);
          if (e.key === "Escape") {
            e.stopPropagation();
            endTitle(true);
          }
        }}
        onblur={() => endTitle(true)}
      />
    {:else}
      <h3 class="title">{noteTitle(note)}</h3>
    {/if}
    <span class="meta">{meta}{note.folded ? ` · ${t("folded")}` : ""}</span>
    <span class="end">
    <button class="more" class:shown={menuOpen} aria-haspopup="dialog" aria-label={t("Menu for “{name}”", { name: noteTitle(note) })} data-tip={t("Note menu")} aria-expanded={menuOpen} onclick={() => onMenu(!menuOpen)}><Icon name="more" size={16} /></button>
    <button class="chev" class:shown={note.folded} aria-label={note.folded ? t("Unfold “{name}”", { name: noteTitle(note) }) : t("Fold “{name}” to its title", { name: noteTitle(note) })} aria-expanded={!note.folded} data-tip={note.folded ? t("Unfold") : t("Fold to its title")} data-tip-keys="F" onclick={toggleFold}
      ><Icon name={note.folded ? "chev-down" : "chev-up"} size={15} stroke={2.2} /></button
    >
    </span>
  </div>

  {#if !note.folded}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (its "+ Add" button is the way in by keyboard) -->
  <div class="body" transition:fold={{ y: -6, duration: 340, room: 1, show: 0.15 }} onpointerdown={emptyPress} onclick={emptyClick}>
  {#if tasks.length}
    <ul class="tasks" transition:fold>
      {#each tasks as it (it.id)}
        <ItemRow item={it} />
      {/each}
    </ul>
  {/if}

  <div use:snap={lined}><NotePictures {pictures} {width} /></div>

  {#if ideas.length}
    <div class="ideas" transition:fold class:alone={!tasks.length && !pictures.length} style:--tint={tone.bg} style:--idea-ink={tone.ink} style:--corner={tone.corner}>
      <span class="fold" aria-hidden="true"></span>
      {#each ideas as it (it.id)}
        <IdeaRow item={it} {lined} gap={lined ? 0 : 7} />
      {/each}
    </div>
  {/if}

  {#if !tasks.length && !ideas.length && !pictures.length}
    {@const [before, after] = around(t("Empty. Add a line here, or type {tag} above."), "{tag}")}
    <p class="empty" transition:fold>{before}<b>#{tagFor(note)}</b>{after}</p>
  {/if}
  <AddLine noteId={note.id} name={noteTitle(note)} {lined} tint={ideas.length ? tone.bg : null} after={last} bind:active={adding} />
  </div>
  {/if}

  {#if menuOpen}
    <NoteMenu {note} at={menuAt} onClose={() => onMenu(false)} />
  {/if}
</article>

<style>
  .card {
    position: relative;
    container-type: inline-size;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--card-shadow);
    transform-origin: 50% 30%;
    transition: transform 0.18s var(--ease-out), border-color 0.15s;
  }
  /* (The tilt is set on the card alone, a property its contents don't inherit: a carry restyles
     the card on each move, not everything in it. Its own layer while lifted: it is drawn once,
     then only moved and turned.) */
  .card.lifted {
    transform: rotate(var(--lift-tilt)) scale(1.04);
    transition: transform 0.12s linear;
    will-change: transform;
  }
  .card.drop {
    border-color: var(--ink);
    box-shadow: var(--card-shadow), 0 0 0 1px var(--ink);
  }
  .lift-shadow {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: var(--lift-shadow);
    opacity: 0;
    transition: opacity 0.2s var(--ease-out);
    pointer-events: none;
  }
  .lifted .lift-shadow {
    opacity: 1;
  }
  .peel {
    position: absolute;
    left: -1px;
    top: -1px;
    width: 0;
    height: 0;
    border-style: solid;
    border-width: 18px 18px 0 0;
    border-color: var(--peel) transparent transparent transparent;
    border-top-left-radius: var(--r-lg);
    opacity: 0;
    transition: opacity 0.16s;
    pointer-events: none;
    z-index: 1;
  }
  .lifted .peel {
    opacity: 1;
  }

  /* The dot, the title, the count, then the buttons. When the head is short of room the count
     gives way first (down to an ellipsis: its column only gets what the title leaves), and only
     then does the title wrap, at its words. */
  .head {
    display: grid;
    grid-template-columns: auto minmax(0, max-content) minmax(0, 1fr) auto;
    align-items: center;
    column-gap: var(--s2);
    padding: 11px var(--s2) var(--s1) 10px;
    cursor: grab;
    user-select: none;
    /* Folding gives the title a little more air below; it eases with the fold. */
    transition: padding-bottom 0.34s var(--ease-glide);
  }
  @media (prefers-reduced-motion: reduce) {
    .head {
      transition: none;
    }
  }
  .lifted .head {
    cursor: grabbing;
  }
  .dot {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .dot span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    transition: transform 0.15s;
  }
  .dot:hover span {
    transform: scale(1.25);
  }
  .title {
    min-width: 0;
    margin: 0;
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.25;
    letter-spacing: -0.005em;
    overflow-wrap: break-word;
  }
  /* Renaming: the field takes the room. */
  .head.editing {
    grid-template-columns: auto minmax(0, 1fr) auto auto;
  }
  /* Narrow cards: the count goes under the title so the title keeps its words whole. */
  @container (max-width: 320px) {
    .head {
      grid-template-columns: auto minmax(0, 1fr) auto;
      row-gap: 0;
    }
    /* (Renaming too: the count is below, so no empty fourth column with its gap.) */
    .head.editing {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }
    .head .meta {
      grid-row: 2;
      grid-column: 2 / -1;
      justify-self: start;
      margin-top: -2px;
    }
  }
  .end {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .title-input {
    width: calc(100% + 6px);
    min-width: 0;
    height: 28px;
    padding: 0 6px;
    margin-left: -6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--ground);
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--ink);
    outline: 0;
  }
  .meta {
    justify-self: end;
    min-width: 0;
    max-width: 100%;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .more {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background-color 0.15s;
  }
  .card:hover .more,
  .more.shown,
  .more:focus-visible {
    opacity: 1;
  }
  .more:hover,
  .more.shown {
    background: var(--chip);
  }

  .tasks {
    list-style: none;
    margin: 0;
    padding: 2px var(--s2) 10px;
  }

  /* (The "+ Add" row under them carries their tint on to the card's bottom edge.) */
  .ideas {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: var(--s3) var(--s4) 6px;
    background: var(--tint);
  }
  .ideas.alone {
    margin-top: var(--s1);
  }
  .fold {
    position: absolute;
    top: 0;
    right: 0;
    width: 0;
    height: 0;
    border-style: solid;
    border-width: 0 16px 16px 0;
    border-color: transparent var(--card) var(--corner) transparent;
  }
  .empty {
    margin: 0;
    padding: 2px var(--s4) 14px 42px;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .empty b {
    font-weight: 600;
    color: var(--soft);
  }

  /* ---- Fold, tidy, widen, keyboard focus ---- */
  .chev {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    margin-left: -2px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background-color 0.15s;
  }
  .card:hover .chev,
  .chev:focus-visible,
  .chev.shown {
    opacity: 1;
  }
  .chev:hover,
  .chev.shown {
    background: var(--chip);
    color: var(--ink);
  }
  .card.folded .head {
    padding-bottom: 11px;
  }
  .grip {
    position: absolute;
    right: -7px;
    top: 50%;
    z-index: 4;
    width: 14px;
    height: 48px;
    margin-top: -24px;
    display: grid;
    place-items: center;
    cursor: ew-resize;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .grip span {
    width: 5px;
    height: 30px;
    border-radius: 3px;
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--box);
  }
  .card:hover .grip,
  .grip:hover {
    opacity: 1;
  }
  .grip:hover span {
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  .lifted .grip {
    opacity: 0;
  }
  .card.focused {
    box-shadow: var(--card-shadow), 0 0 0 3px var(--ground), 0 0 0 5px var(--focus-ring);
  }

  /* ---- Paper ---- */
  .card.paper-antique {
    background: var(--antique);
  }
  /* The lines start 12 px down (the header's top margin); each row is one 24 px line with its
     text sitting on the rule. */
  .card.lined {
    --lines: repeating-linear-gradient(to bottom, transparent 0 19px, color-mix(in srgb, var(--rule) 75%, transparent) 19px 20px, transparent 20px 24px);
    background-image: var(--lines);
    background-position: 0 12px;
  }
  .card.paper-grid {
    --lines: repeating-linear-gradient(to bottom, transparent 0 19px, color-mix(in srgb, var(--rule) 70%, transparent) 19px 20px, transparent 20px 24px),
      repeating-linear-gradient(to right, transparent 0 23px, color-mix(in srgb, var(--rule) 45%, transparent) 23px 24px);
  }
  /* Every row is one line high (24 px) and starts on a line. */
  .lined .head {
    min-height: 36px;
    padding-top: 12px;
    padding-bottom: 0;
    align-items: start;
  }
  .lined .end {
    align-items: flex-start;
  }
  .lined.folded .head {
    padding-bottom: 12px;
  }
  .lined .title,
  .lined .title-input {
    line-height: 24px;
  }
  .lined .meta {
    line-height: 24px;
    margin-top: 0;
  }
  .lined .title-input {
    height: 24px;
  }
  .lined .dot,
  .lined .more,
  .lined .chev {
    width: 24px;
    height: 24px;
  }
  .lined .tasks {
    padding: 0 var(--s2);
  }
  /* Half a line clear of the last task, and the tint starts half a line above the first idea:
     one line in all, so the 24 px rhythm holds (the band's own rules start with its ideas). */
  .lined .ideas {
    gap: 0;
    margin-top: 12px;
    padding: 12px var(--s4) 0;
    background-image: var(--lines);
    background-position: 0 12px;
  }
  .lined .ideas.alone {
    margin-top: 0;
    padding-top: 0;
    background-position: 0 0;
  }
  .lined .empty {
    padding-top: 0;
    padding-bottom: 0;
    line-height: 24px;
  }
  .lined :global(.tasks .row) {
    padding-top: 0;
    padding-bottom: 0;
    min-height: 24px;
  }
  .lined :global(.tasks .row .text) {
    line-height: 24px;
  }
  .lined :global(.tasks .row .box) {
    margin-top: 4px;
  }
  /* (A row is exactly one line: its frame, when focused or dropped onto, is drawn just outside
     it, so it doesn't run over a task's steps ring.) */
  .lined :global(.tasks .row.focused) {
    box-shadow: 0 0 0 1.5px var(--focus-ring);
  }
  .lined :global(.tasks .row.drop) {
    box-shadow: 0 0 0 1.5px var(--ink);
  }
  .lined :global(.tasks .row .pill),
  .lined :global(.tasks .row .count) {
    margin-top: 3px;
  }
  .lined :global(.tasks .row .acts) {
    top: 0;
  }
  .lined :global(.tasks .row .act) {
    height: 24px;
  }
  .lined :global(.tasks .steps) {
    margin-top: 0;
    margin-bottom: 0;
  }
  .lined :global(.add-step) {
    padding-top: 0;
    padding-bottom: 0;
    height: 24px;
  }
  .lined :global(.add-step input) {
    height: 22px;
  }
</style>
