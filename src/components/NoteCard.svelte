<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import NoteMenu from "./NoteMenu.svelte";
  import ItemRow from "./ItemRow.svelte";
  import IdeaRow from "./IdeaRow.svelte";
  import AddLine from "./AddLine.svelte";
  import BlankLine from "./BlankLine.svelte";
  import NotePictures from "./NotePictures.svelte";
  import { store, NO_PICTURES } from "../lib/store.svelte";
  import { NO_BLANK_IDS, freshEnd, pressedIn } from "../lib/newNote";
  import { tick, untrack } from "svelte";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { fileDrop } from "../lib/filedrop.svelte";
  import { snap } from "../lib/snap";
  import { fold, fadeQuick, drawMark, crumbIn, crumbOut, veil } from "../lib/motion";
  import { allDone } from "../lib/allDone";
  import type { Tone } from "../lib/palette";
  import type { Item, Kind, Note } from "../lib/types";
  import { around, t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";
  import { fromControl } from "../lib/dblclick";
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
  // Blank lines waiting to be filled in (a note made from a template): never counted.
  let blankIds = $derived(store.blanks[note.id] ?? NO_BLANK_IDS);
  let blanks = $derived({ task: blankIds.task.length, idea: blankIds.idea.length });
  // Every task done: a big faint check, the card softened, "All done" (ideas still counted).
  // (Not while blank tasks still wait to be written.)
  let done = $derived(tasks.length > 0 && !blanks.task && allDone(tasks, store.inboxIds.has(note.id)));
  let meta = $derived(
    [done ? t("All done") : tasks.length ? t("{n} to do", { n: open }) : "", ideas.length ? t("{n} ideas", { n: ideas.length }) : ""].filter(Boolean).join(" · "),
  );
  // All done: "All done" always whole; what follows it (the ideas, "folded") goes first when the
  // line is short of room (a long title), never "All do…".
  let after = $derived([ideas.length ? t("{n} ideas", { n: ideas.length }) : "", note.folded ? t("folded") : ""].filter(Boolean).join(" · "));

  // The big check's size and place come from the card as it is open, so folding it fades the
  // check where it is (never shrinking and sliding with the card).
  let cardH = $state(0);
  let openH = $state<number | null>(null);
  let folding = false;
  let foldTimer: ReturnType<typeof setTimeout> | null = null;
  let wasFolded = untrack(() => note.folded);
  $effect(() => {
    const f = note.folded;
    if (f === wasFolded) return;
    wasFolded = f;
    folding = true;
    if (foldTimer) clearTimeout(foldTimer);
    foldTimer = setTimeout(() => {
      folding = false;
      if (!note.folded && cardEl) openH = cardEl.offsetHeight;
    }, 420);
    return () => foldTimer && clearTimeout(foldTimer);
  });
  $effect(() => {
    const h = cardH;
    if (h > 0 && !note.folded && !folding) untrack(() => (openH = h));
  });
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
    // (A new note's title, where it was just clicked: in sight already.)
    el.focus({ preventScroll: fresh });
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

  // ---- A new note from the board's right-click menu: its title waits to be typed ----
  /** Its title field is the new note's (empty, "Title of the note" in it). */
  let fresh = $state(false);
  /** Taken away again: fading off the board. */
  let leaving = $state(false);
  /** When the note was last pressed (anywhere in it). */
  let pressedAt: number | null = null;
  $effect(() => {
    if (store.fresh !== note.id) return;
    untrack(() => {
      if (editingTitle && fresh) return;
      titleDraft = "";
      fresh = true;
      editingTitle = true;
    });
  });
  /** Enter: named (or left "New note" with nothing typed), and the keyboard goes on into it.
   * Esc, or a click away, with nothing typed: the note goes again. */
  async function endFresh(how: "enter" | "esc" | "blur", e?: FocusEvent) {
    if (!editingTitle || !fresh) return;
    // (In the note: the keyboard went to something in it, or it was pressed anywhere in it.)
    const inside = (e?.relatedTarget instanceof Node && !!cardEl?.contains(e.relatedTarget)) || (how === "blur" && pressedIn(pressedAt, performance.now()));
    const end = freshEnd(how, titleDraft, { inside, windowFocused: document.hasFocus() });
    if (end === "wait") return;
    fresh = false;
    if (end === "drop") {
      // (Its empty field stays as the card fades away: no "New note" showing for a moment.)
      leaving = true;
      if (document.activeElement instanceof HTMLElement && cardEl?.contains(document.activeElement)) document.activeElement.blur();
      if (await store.dropNew(note.id)) return;
      leaving = false;
      editingTitle = false;
      return;
    }
    editingTitle = false;
    if (end === "name") await store.nameNew(note, titleDraft);
    else store.fresh = null;
    if (how === "enter") void goOn("title");
  }
  /** The keyboard goes on down the note: to its first blank line (of a kind after `from`), else
   * its "+ Add" field. */
  async function goOn(from: "title" | Kind) {
    await tick();
    const order = from === "title" ? [".tasks .blank input", ".idea-blank input"] : from === "task" ? [".idea-blank input"] : [];
    for (const q of order) {
      const el = cardEl?.querySelector<HTMLInputElement>(q);
      if (el) return el.focus({ preventScroll: true });
    }
    adding = true;
  }

  // "+ Add" at the end of the note; a click in the note's empty space opens it too.
  let adding = $state(false);
  let last = $derived<"tasks" | "ideas" | "pictures" | "empty">(ideas.length || blanks.idea ? "ideas" : pictures.length ? "pictures" : tasks.length || blanks.task ? "tasks" : "empty");
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
  class:done
  aria-label={noteTitle(note)}
  data-drop-note={note.id}
  bind:this={cardEl}
  bind:clientHeight={cardH}
  onpointerdowncapture={() => (pressedAt = performance.now())}
  style:--rule={tone.corner}
  style:--open-h={openH ? `${openH}px` : null}
  style:--lift-tilt={lifted ? `${tilt.toFixed(2)}deg` : null}
>
  <div class="lift-shadow" aria-hidden="true"></div>
  <div class="peel" style:--peel={tone.corner} aria-hidden="true"></div>
  {#if onWiden && !note.folded}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="grip" transition:fadeQuick data-tip={note.wide ? t("Drag left to make it normal width") : t("Drag right to make it wide")} onpointerdown={onWiden} aria-hidden="true"><span></span></div>
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="head" class:editing={editingTitle} onpointerdown={grab} ondblclick={(e) => !fromControl(e, e.currentTarget) && startTitle()}>
    <button class="dot" aria-label={t("Colour and options for “{name}”", { name: noteTitle(note) })} data-tip={t("Note colour and options")} onclick={() => onMenu(!menuOpen)}><span style:background={tone.dot}></span></button>
    {#if editingTitle}
      <input
        class="title-input"
        bind:value={titleDraft}
        use:focusSelect
        aria-label={t("Note title")}
        placeholder={fresh || leaving ? t("Title of the note") : null}
        onkeydown={(e) => {
          if (composing(e)) return;
          if (e.key === "Enter") {
            if (fresh) endFresh("enter");
            else endTitle(true);
          }
          if (e.key === "Escape") {
            e.stopPropagation();
            if (fresh) endFresh("esc");
            else endTitle(true);
          }
        }}
        onblur={(e) => (leaving ? undefined : fresh ? endFresh("blur", e) : endTitle(true))}
      />
    {:else}
      <h3 class="title">{noteTitle(note)}</h3>
    {/if}
    <span class="meta" class:whole={done}
      >{#key done}<span class="words" class:parts={done} in:crumbIn out:crumbOut
          >{#if done}<span class="part"
              >{#if note.folded}<span class="tick" transition:fadeQuick={{ duration: 180 }}><Icon name="check" size={10} stroke={3} /></span>{/if}{t("All done")}</span
            >{#if after}<span class="part rest">&nbsp;· {after}</span>{/if}{:else}{meta}{note.folded ? ` · ${t("folded")}` : ""}{/if}</span
        >{/key}</span
    >
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
  {#if tasks.length || blanks.task}
    <ul class="tasks" transition:fold>
      {#each tasks as it (it.id)}
        <ItemRow item={it} />
      {/each}
      {#each blankIds.task as id, i (id)}
        <BlankLine noteId={note.id} {id} kind="task" first={i === 0} {lined} name={noteTitle(note)} onEnd={goOn} />
      {/each}
    </ul>
  {/if}

  <div use:snap={lined}><NotePictures {pictures} {width} /></div>

  {#if ideas.length || blanks.idea}
    <div class="ideas" transition:fold class:alone={!tasks.length && !blanks.task && !pictures.length} style:--tint={tone.bg} style:--idea-ink={tone.ink} style:--corner={tone.corner}>
      <span class="fold" aria-hidden="true"></span>
      {#each ideas as it (it.id)}
        <IdeaRow item={it} {lined} gap={lined ? 0 : 7} />
      {/each}
      {#each blankIds.idea as id, i (id)}
        <BlankLine noteId={note.id} {id} kind="idea" first={i === 0} {lined} name={noteTitle(note)} onEnd={goOn} />
      {/each}
    </div>
  {/if}

  {#if !tasks.length && !ideas.length && !pictures.length && !blanks.task && !blanks.idea}
    {@const [before, after] = around(t("Empty. Add a line here, or type {tag} above."), "{tag}")}
    <p class="empty" transition:fold>{before}<b>#{tagFor(fresh && titleDraft.trim() ? { title: titleDraft } : note)}</b>{after}</p>
  {/if}
  <AddLine noteId={note.id} name={noteTitle(note)} {lined} tint={ideas.length || blanks.idea ? tone.bg : null} after={last} bind:active={adding} />
  </div>
  {/if}

  <!-- All done: a big faint check across the lower right, over the words but blended into the
       paper (so it reads as behind them), never in the way of a click. It goes with the body
       when the note folds. -->
  {#if !note.folded}
    <div class="done-layer" aria-hidden="true" transition:veil={{ duration: 220 }}>
      <svg class="done-mark" viewBox="0 0 120 96" fill="none">
        {#if done}<path d="M15 53 C24 61 34 71 44 80 C61 56 81 34 105 15" pathLength="1" transition:drawMark={{ delay: 120 }} />{/if}
      </svg>
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
    /* (All done: the count's column keeps "All done" whole, the title wraps first.) */
    grid-template-columns: auto minmax(0, max-content) minmax(var(--meta-min, 0px), 1fr) auto;
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
      text-align: left;
    }
    .head .meta .words.parts {
      justify-content: flex-start;
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
  /* (Its words in one cell: "1 to do" and "All done" cross there, never side by side.) */
  .meta {
    display: grid;
    justify-self: end;
    min-width: 0;
    max-width: 100%;
    font-size: var(--t-sm);
    color: var(--muted);
    text-align: right;
  }
  .meta .words {
    grid-area: 1 / 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* All done: "All done" never cut (the title gives way first), and what follows it wraps onto a
     line that never shows when it doesn't fit. */
  .meta.whole {
    min-width: min-content;
  }
  .card.done {
    --meta-min: min-content;
  }
  .meta .words.parts {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    min-width: min-content;
    height: 1lh;
  }
  .meta .part {
    flex-shrink: 0;
  }
  /* Folded and all done: a small check before the words (the big one folds away with the body). */
  .meta .tick {
    display: inline-flex;
    margin-right: 4px;
    color: var(--done-mark);
    vertical-align: -1px;
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

  /* ---- All done ---- */
  /* The check sits over the words but is blended into what is under it (multiplied on light
     paper, screened on dark), so dark words stay dark, light ones light: it reads as drawn on
     the paper behind them. It stays whole over a picture too (a check cut by one would read as
     a fault). It never takes a pointer. */
  .done-layer {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    overflow: hidden;
    pointer-events: none;
    mix-blend-mode: var(--done-mark-blend);
  }
  .done-mark {
    position: absolute;
    right: 7%;
    /* Big but kept clear of the title: at most the card's height under its head (the card as it
       is open: folding, it stays where it is and fades). */
    --mark-h: min(42cqw, 170px, calc(var(--open-h, 100%) - 52px));
    top: calc(var(--open-h, 100%) - 12px - var(--mark-h));
    height: var(--mark-h);
    aspect-ratio: 120 / 96;
    overflow: visible;
    opacity: var(--done-mark-strength);
  }
  .done-mark path {
    stroke: var(--done-mark);
    stroke-width: 9;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  /* The words dim a little (still readable), and back the same way. A filter, so it adds to
     the opacity a carried line already takes. */
  .title,
  .card :global(.tasks .text),
  .card :global(.ideas .idea-text),
  .card :global(.ideas .row .text) {
    /* (Its own: the colour of a tick, the opacity of a carry.) */
    transition:
      filter 0.42s var(--ease-out),
      opacity 0.18s var(--ease-out),
      color 0.2s;
  }
  .card.done .title {
    filter: opacity(0.62);
  }
  .card.done :global(.tasks .text),
  .card.done :global(.ideas .idea-text),
  .card.done :global(.ideas .row .text) {
    filter: opacity(0.78);
  }
  @media (prefers-reduced-motion: reduce) {
    .title,
    .card :global(.tasks .text),
    .card :global(.ideas .idea-text),
    .card :global(.ideas .row .text) {
      transition:
        filter 0.14s,
        opacity 0.18s var(--ease-out),
        color 0.2s;
    }
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
