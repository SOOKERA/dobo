<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // One task (or a step under it): checkbox, text, time (hourglass for a deadline), its menu,
  // its link / file / pictures, and its steps. Press and drag the text to carry it elsewhere.
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import ItemMenu from "./ItemMenu.svelte";
  import ItemRow from "./ItemRow.svelte";
  import LinkCard from "./LinkCard.svelte";
  import FileCard from "./FileCard.svelte";
  import StepTrace from "./StepTrace.svelte";
  import { store, NO_ITEMS, NO_PICTURES } from "../lib/store.svelte";
  import { thumbs } from "../lib/thumbs.svelte";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { openViewer } from "../lib/viewer.svelte";
  import { formatDue, formatDueLong } from "../lib/time";
  import { findDate } from "../lib/capture";
  import { repeatLabel } from "../lib/repeat";
  import { snap } from "../lib/snap";
  import { blip, fadeQuick, fold, widen } from "../lib/motion";
  import type { Item } from "../lib/types";
  import { t } from "../lib/i18n.svelte";
  import { rightClick, takeMenu, type Point } from "../lib/rightclick.svelte";
  import { untrack } from "svelte";

  let { item, nested = false, tint = false }: { item: Item; nested?: boolean; tint?: boolean } = $props();

  let editing = $state(false);
  let draft = $state("");
  let menuOpen = $state(false);
  /** Where its menu was right-clicked open (null: under its ⋯). */
  let menuAt = $state.raw<Point | null>(null);
  let rowEl = $state<HTMLElement>();
  let addingStep = $state(false);
  let stepDraft = $state("");
  let inputEl = $state<HTMLInputElement>();
  let mirrorEl = $state<HTMLDivElement>();

  // (The Inbox strip has no paper of its own, whatever its note once had.)
  let lined = $derived.by(() => {
    if (store.inboxIds.has(item.note_id)) return false;
    const p = store.notes.find((n) => n.id === item.note_id)?.paper;
    return p === "ruled" || p === "grid";
  });
  let focused = $derived(store.focus?.item === item.id);

  // Smart dates: a date or time written in the line is underlined and offered as a reminder.
  let ignored = $state<string | null>(null);
  let found = $derived.by(() => {
    if (!editing || item.kind !== "task" || store.smartDates === "off") return null;
    const f = findDate(draft, store.now);
    if (!f) return null;
    const phrase = draft.slice(f.index, f.index + f.length);
    if (phrase === ignored) return null;
    if (item.due_at != null && Math.abs(item.due_at - f.due.getTime()) < 60_000) return null;
    return { ...f, phrase };
  });
  let auto = $derived(store.smartDates === "auto" && !!found?.certain);

  // (The same steps give the same list: a line whose steps didn't change isn't redrawn.)
  let stepsWere: Item[] | null = null;
  let stepsList: Item[] = NO_ITEMS;
  let steps = $derived.by(() => {
    const s = nested ? NO_ITEMS : (store.stepsOf.get(item.id) ?? NO_ITEMS);
    if (s === stepsWere) return stepsList;
    stepsWere = s;
    return (stepsList = s.length ? [...s.filter((x) => x.done_at == null), ...s.filter((x) => x.done_at != null)] : NO_ITEMS);
  });
  let stepsDone = $derived(steps.filter((s) => s.done_at != null).length);
  // Every step ticked: the task itself is still yours to tick (nothing ticks it for you).
  let allSteps = $derived(steps.length > 0 && stepsDone === steps.length);
  let stepsLabel = $derived(t("{a} of {n} steps done", { a: stepsDone, n: steps.length }));
  let pics = $derived(store.picturesByItem.get(item.id) ?? NO_PICTURES);
  let late = $derived(item.due_at != null && item.done_at == null && item.due_at < store.now.getTime());
  let ringing = $derived(item.done_at == null && (item.ring_state === "ringing" || item.ring_state === "check" || item.ring_state === "early"));
  let dropHere = $derived(itemDrag.target?.kind === "task" && itemDrag.target.id === item.id);
  let carried = $derived(itemDrag.item?.id === item.id || itemDrag.away.includes(item.id));

  function focusSelect(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
  // The add-step field takes the keys when it opens, also when it is opened again while still
  // folding away (the same field comes back, so nothing new is mounted).
  let stepInput = $state<HTMLInputElement>();
  $effect(() => {
    if (addingStep && stepInput && document.activeElement !== stepInput) stepInput.focus();
  });

  function start() {
    draft = item.text;
    ignored = null;
    editing = true;
  }
  /** Saves the line; `withTime` sets the date found in it (the chip was accepted). */
  async function end(save: boolean, withTime = auto) {
    if (!editing) return;
    const f = found;
    editing = false;
    if (!save) return;
    if (withTime && f) await store.editItem(item, draft, f.due.getTime());
    else if (draft.trim() !== item.text) await store.editItem(item, draft);
  }

  function editKey(e: KeyboardEvent) {
    // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
    if (composing(e)) return;
    const t = e.currentTarget as HTMLInputElement;
    if (e.key === "Enter") {
      e.preventDefault();
      // Enter takes the offered time too.
      end(true, !!found);
    } else if (e.key === "Escape") {
      e.stopPropagation();
      // First Esc turns down the offered time; the next one ends the edit (keeping the text).
      if (found && !auto) ignored = found.phrase;
      else end(true, false);
    } else if (e.key === "Backspace" && t.selectionStart === 0 && t.selectionEnd === 0) {
      // Backspace at the start of the line: it becomes an idea.
      e.preventDefault();
      const text = draft.trim() || item.text;
      editing = false;
      store.flipKind({ ...item, text });
    }
  }

  function syncMirror() {
    if (mirrorEl && inputEl) mirrorEl.scrollLeft = inputEl.scrollLeft;
  }

  async function addStep(keepOpen: boolean) {
    const t = stepDraft.trim();
    stepDraft = "";
    if (t) await store.addStep(item, t);
    if (!keepOpen || !t) addingStep = false;
  }

  // Its menu asked for by a right-click, Shift+F10 or the Menu key (App): it opens, at the
  // pointer or under its ⋯. (Not the copy of a line on its way out to another note.)
  $effect(() => {
    if (rightClick.ask?.kind !== "line" || rightClick.ask.id !== item.id) return;
    untrack(() => {
      if (!rowEl || rowEl.closest("[inert]:not([data-cover])")) return;
      const got = takeMenu("line", item.id);
      if (!got) return;
      menuAt = got.at;
      menuOpen = true;
    });
  });

  function press(e: PointerEvent) {
    if (editing) return;
    itemDrag.press(e, item);
  }
</script>

<li class="wrap" class:nested class:carried transition:fold>
  <div
    class="row"
    class:done={item.done_at != null}
    class:drop={dropHere}
    class:menu-open={menuOpen}
    class:focused
    class:tint
    data-drop-item={item.id}
    data-drop-kind="task"
    data-drop-label={t("Drop to make it a step")}
    data-line={item.id}
    bind:this={rowEl}
  >
    <button class="box" class:small={nested} class:all={allSteps && item.done_at == null} aria-label={item.done_at == null ? t("Mark “{name}” done", { name: item.text }) : t("Mark “{name}” not done", { name: item.text })} data-tip={item.done_at == null ? t("Done") : t("Not done")} data-tip-keys="Space" onclick={() => store.toggleDone(item)}>
      {#if item.done_at != null}<Icon name="check" size={nested ? 9 : 10} stroke={3.4} />{/if}
      <!-- Its steps' progress: a thin ring round the box (gone once the task is ticked). -->
      {#if steps.length && item.done_at == null}<span class="trace" class:lined transition:fadeQuick={{ duration: 200 }}><StepTrace done={stepsDone} total={steps.length} size={lined ? 22 : 24} radius={lined ? 7 : 8} stroke={1.25} /></span>{/if}
    </button>
    {#if editing}
      <div class="edit">
        {#if found}
          <div class="mirror" bind:this={mirrorEl} aria-hidden="true">{draft.slice(0, found.index)}<mark>{found.phrase}</mark>{draft.slice(found.index + found.length)}</div>
        {/if}
        <input
          class="line-input"
          bind:this={inputEl}
          bind:value={draft}
          use:focusSelect
          aria-label={t("Edit")}
          onkeydown={editKey}
          oninput={syncMirror}
          onscroll={syncMirror}
          onkeyup={syncMirror}
          onblur={() => end(true)}
        />
      </div>
      {#if found}
        <span class="break"></span>
        <button
          class="offer"
          transition:fold={{ y: -3, duration: 240 }}
          class:auto
          data-tip={auto ? t("Will remind you (Settings › Notes › Dates)") : t("Add the reminder (Esc leaves it)")} data-tip-keys={auto ? null : "Enter"}
          onpointerdown={(e) => e.preventDefault()}
          onclick={() => end(true, true)}
        >
          <Icon name="clock" size={12} stroke={2.2} />{formatDueLong(found.due.getTime(), store.now)} · {auto ? t("reminds") : t("Remind?")}
        </button>
      {/if}
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="text" ondblclick={start} onpointerdown={press}>{item.text}</span>
    {/if}
    {#if steps.length}
      <span class="count" class:all={allSteps} transition:widen={{ gap: 10 }} data-tip={stepsLabel}
        >{#if allSteps}<span class="all-tick" transition:widen={{ gap: 0, duration: 220 }}><Icon name="check" size={10} stroke={3} /></span>{/if}<span aria-hidden="true">{stepsDone}/{steps.length}</span><span class="visually-hidden">{stepsLabel}</span></span
      >
    {/if}
    {#if item.due_at != null && item.done_at == null && !editing}
      {@const pillTip = [item.alarm === "deadline" ? t("Deadline") : t("Reminder"), item.repeat ? repeatLabel(item.repeat) : ""].filter(Boolean).join(" · ")}
      <span class="pill" transition:widen={{ gap: 10 }} class:late class:ringing data-tip={pillTip}>
        <span class="visually-hidden">{pillTip}: </span>
        {#if item.alarm === "deadline"}<Icon name="hourglass" size={11} stroke={2.2} />{/if}
        {formatDue(item.due_at, store.now)}
        {#if item.repeat}<Icon name="repeat" size={11} stroke={2.2} />{/if}
      </span>
    {/if}
    <span class="acts">
      <button class="act" data-item-menu={item.id} aria-haspopup="dialog" data-tip={t("Time, repeat, reminder, steps…")} aria-label={t("Menu for “{name}”", { name: item.text })} aria-expanded={menuOpen} onclick={() => ((menuAt = null), (menuOpen = !menuOpen))}
        ><Icon name="more" size={15} /></button
      >
    </span>
    {#if menuOpen}
      <ItemMenu {item} at={menuAt} onClose={() => (menuOpen = false)} onAddStep={() => (addingStep = true)} />
    {/if}
  </div>

  {#if item.link || item.file || pics.length}
    <div class="attach" data-menu-line={item.id} transition:fold use:snap={lined}>
      {#if item.link}<LinkCard link={item.link} />{/if}
      {#if item.file}<FileCard path={item.file} />{/if}
      {#if pics.length}
        <div class="thumbs">
          {#each pics as p, i (p.id)}
            <button class="thumb" in:blip aria-label={t("Open picture")} ondblclick={() => openViewer(pics, i)} onclick={() => openViewer(pics, i)}>
              <img src={store.thumbSrc(p)} alt="" loading="lazy" decoding="async" draggable="false" onerror={() => thumbs.failed(p)} />
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if steps.length || addingStep}
    <ul class="steps" transition:fold>
      {#each steps as s (s.id)}
        <ItemRow item={s} nested {tint} />
      {/each}
      {#if addingStep}
        <li class="add-step" transition:fold={{ duration: 240 }}>
          <Icon name="step" size={13} />
          <input
            bind:value={stepDraft}
            bind:this={stepInput}
            placeholder={t("A step, then Enter")}
            aria-label={t("New step")}
            onkeydown={(e) => {
              if (composing(e)) return;
              if (e.key === "Enter") {
                e.preventDefault();
                addStep(true);
              }
              if (e.key === "Escape") {
                e.stopPropagation();
                addStep(false);
              }
            }}
            onblur={() => addStep(false)}
          />
        </li>
      {/if}
    </ul>
  {/if}
</li>

<style>
  .wrap {
    list-style: none;
    transition: opacity 0.15s;
  }
  /* While carried, its place stays as a dashed gap (it fades in, and back out on the drop). */
  .row {
    outline: 1.5px dashed transparent;
    outline-offset: -1.5px;
  }
  .wrap.carried > .row {
    outline-color: var(--edge);
  }
  .row > *,
  .wrap > .attach,
  .wrap > .steps {
    transition: opacity 0.18s var(--ease-out);
  }
  .wrap.carried > .row > *,
  .wrap.carried > .attach,
  .wrap.carried > .steps {
    opacity: 0.25;
  }
  .row.focused {
    background: color-mix(in srgb, var(--focus-ring) 12%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--focus-ring);
  }
  .edit {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .mirror {
    position: absolute;
    left: -6px;
    right: 0;
    top: -2px;
    height: 24px;
    padding: 0 7px;
    overflow: hidden;
    white-space: pre;
    font: inherit;
    font-size: var(--t-base);
    line-height: 24px;
    color: transparent;
    pointer-events: none;
  }
  .mirror mark {
    background: transparent;
    color: transparent;
    text-decoration: underline dotted var(--late-ink);
    text-decoration-thickness: 2px;
    text-underline-offset: 4px;
  }
  .edit .line-input {
    background: transparent;
  }
  /* The offer sits on its own line under the text, so the field keeps its width. */
  .row:has(> .offer) {
    flex-wrap: wrap;
    row-gap: 4px;
  }
  .break {
    order: 9;
    flex-basis: 100%;
    height: 0;
  }
  .offer {
    order: 10;
    flex-shrink: 0;
    height: 22px;
    margin-left: 26px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-xs);
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    cursor: pointer;
    animation: offer-in 0.2s var(--ease-out);
  }
  .offer.auto {
    background: var(--chip);
    color: var(--soft);
  }
  @keyframes offer-in {
    from {
      opacity: 0;
      transform: translateX(-4px);
    }
  }
  .row {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 6px var(--s2);
    border-radius: var(--r-sm);
    transition: background-color 0.15s, box-shadow 0.15s, outline-color 0.18s var(--ease-out);
  }
  .row:hover,
  .row.menu-open {
    background: var(--hover);
  }
  .row.drop {
    background: var(--hover);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  /* "Drop to make it a step": fades up at the row's right end while a line is held over it, set
     on the row's top edge like a legend: over neither its own words and time nor the next
     row's. */
  .row::after {
    content: attr(data-drop-label);
    position: absolute;
    right: 10px;
    top: 0;
    translate: 0 -50%;
    z-index: 2;
    padding: 1px 7px;
    border-radius: 5px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-xs);
    font-weight: 600;
    pointer-events: none;
    display: none;
    opacity: 0;
    transform: translateY(3px);
    transition: opacity 0.14s var(--ease-out), transform 0.18s var(--ease-out), display 0.18s allow-discrete;
  }
  @media (prefers-reduced-motion: reduce) {
    .row::after {
      transition: opacity 0.14s var(--ease-out), display 0.14s allow-discrete;
    }
  }
  .row.drop::after {
    display: block;
    opacity: 1;
    transform: none;
    @starting-style {
      opacity: 0;
      transform: translateY(3px);
    }
  }
  .nested .row {
    padding: 4px var(--s2) 4px 6px;
  }
  .box {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin-top: 2px;
    padding: 0;
    border: 1.5px solid var(--box);
    border-radius: 4px;
    background: transparent;
    color: var(--card);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.18s, border-color 0.18s, transform 0.18s var(--ease-out);
  }
  /* The steps' ring sits 4 px out from the box, on its middle (measured from the middle, so a
     border drawn thinner on some screens doesn't pull it off centre; a whole number of pixels
     out, so its line falls on the screen's pixels). */
  .box {
    position: relative;
  }
  .trace {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 24px;
    height: 24px;
    margin: -12px;
  }
  /* On ruled and grid paper a row is one 24 px line: the ring comes in to 3 px, so it stays
     clear of the row's edges (and of its frame when focused or dropped onto). */
  .trace.lined {
    width: 22px;
    height: 22px;
    margin: -11px;
  }
  /* The keyboard's ring goes round the steps' ring, not over it. */
  .box:has(> .trace):focus-visible {
    outline-offset: 6px;
  }
  /* Every step done: the ring has closed round the box, in full ink, and so has the box's own
     border: ready to be ticked (nothing ticks it for you). */
  .box.all {
    border-color: var(--ink);
  }
  .box.small {
    width: 14px;
    height: 14px;
    margin-top: 2px;
  }
  .box:hover {
    border-color: var(--ink);
  }
  .box:active {
    transform: scale(0.88);
  }
  .done .box {
    background: var(--done-box);
    border-color: var(--done-box);
  }
  .text {
    flex: 1;
    min-width: 0;
    font-size: var(--t-base);
    line-height: 1.4;
    overflow-wrap: anywhere;
    cursor: grab;
    user-select: none;
    transition: color 0.2s;
  }
  .nested .text {
    font-size: var(--t-md);
  }
  .done .text {
    color: var(--done-ink);
    text-decoration: line-through;
    text-decoration-color: var(--done-box);
  }
  .count {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    margin-top: 2px;
    font-size: var(--t-xs);
    font-weight: 600;
    line-height: 16px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    transition: color 0.2s;
  }
  .count.all {
    color: var(--soft);
  }
  .all-tick {
    display: inline-flex;
    padding-right: 3px;
  }
  .pill {
    flex-shrink: 0;
    margin-top: 1px;
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--chip);
    color: var(--soft);
    font-size: var(--t-sm);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .pill.late,
  .pill.ringing {
    background: var(--danger-fill);
    color: #fff;
    font-weight: 600;
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
  .acts {
    position: absolute;
    right: 4px;
    top: 3px;
    display: flex;
    gap: 2px;
    padding-left: var(--s4);
    background: linear-gradient(to right, transparent, var(--card) 14px);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s;
  }
  .tint .acts {
    background: linear-gradient(to right, transparent, var(--tint) 14px);
  }
  .nested .acts {
    top: 1px;
  }
  .row:hover .acts,
  .row.menu-open .acts,
  .acts:focus-within {
    opacity: 1;
    pointer-events: auto;
  }
  /* (A line held over it: its time stays in sight, not its ⋯.) */
  .row.drop .acts {
    opacity: 0;
  }
  .act {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--card);
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .act:hover {
    background: var(--chip);
    color: var(--ink);
  }
  .attach {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 var(--s2) 6px 34px;
  }
  .nested .attach {
    padding-left: 28px;
  }
  .thumbs {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .thumb {
    width: 64px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    overflow: hidden;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: zoom-in;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .steps {
    margin: -2px 0 2px 26px;
    padding: 0 0 0 6px;
    border-left: 1.5px solid var(--line);
  }
  .add-step {
    list-style: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 4px 6px;
    color: var(--muted);
  }
  .add-step input {
    flex: 1;
    min-width: 0;
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
    outline: 0;
  }
</style>
