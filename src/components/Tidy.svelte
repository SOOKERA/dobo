<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Tidying the inbox, one card at a time (Tidy board): task or idea, an optional time, which
  // note (one is suggested), and where in it — a new task, a step of a task, or the first step
  // of an idea. Keys: T / I, 1–9, Up/Down, Enter, D (delete), L (keep for later).
  import { composing } from "../lib/compose";
  import { roving } from "../lib/gridkeys";
  import { untrack } from "svelte";
  import ViewHeader from "./ViewHeader.svelte";
  import LinkCard from "./LinkCard.svelte";
  import FileCard from "./FileCard.svelte";
  import { store } from "../lib/store.svelte";
  import { thumbs } from "../lib/thumbs.svelte";
  import { NOT_OURS } from "../lib/ownFiles";
  import Icon from "./Icon.svelte";
  import { normalize, parseCapture } from "../lib/capture";
  import { formatDueLong, hhmm } from "../lib/time";
  import { pushLayer } from "../lib/layers";
  import { digitKey, latinKey } from "../lib/keyboard";
  import { isTopModal, modal } from "../lib/modal";
  import { arrive, crumbIn, crumbOut, fadeQuick, fold, sheet, widen } from "../lib/motion";
  import { swatchOf } from "../lib/palette";
  import { toastAbove } from "../lib/toastroom.svelte";
  import { look } from "../lib/look.svelte";
  import type { Item, Kind, Note } from "../lib/types";
  import { keysText, t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";

  let { onClose }: { onClose: () => void } = $props();

  $effect(() => pushLayer(onClose));
  let tidyEl = $state<HTMLElement>();

  // The queue is taken when tidying starts; "later" sends a card to the end.
  let queue = $state<number[]>(untrack(() => store.inboxWaiting.map((i) => i.id)));
  let done = $state(0);
  let total = untrack(() => queue.length);
  let item = $derived(store.items.find((i) => i.id === queue[0]) as Item | undefined);

  let kind = $state<Kind>("task");
  let text = $state("");
  let when = $state("");
  let noteId = $state<number | null>(null);
  let place = $state(0);
  let naming = $state(false);
  let newName = $state("");
  let flying = $state(false);
  let landed = $state<{ title: string; dot: string; at: number } | null>(null);

  // Words that say little about where a line belongs.
  const STOP = new Set(
    "the and for with from this that what when your have will about into just some more pour dans avec sans plus moins faire tout tous cette mais comme".split(" "),
  );
  const words = (s: string) =>
    s
      .normalize("NFD")
      .replace(/\p{M}/gu, "")
      .toLowerCase()
      .split(/[^\p{L}\p{N}]+/u)
      .filter((w) => w.length >= 4 && !STOP.has(w));
  /** Same word, give or take an ending ("hangar" and "hangars"). */
  const same = (a: string, b: string) => a === b || (Math.min(a.length, b.length) >= 4 && (a.startsWith(b) || b.startsWith(a)) && Math.abs(a.length - b.length) <= 2);

  let notes = $derived(store.notes.filter((n) => !n.archived && !store.inboxIds.has(n.id) && (store.view === "all" || n.space === store.view)));

  // Each text's words worked out once (the same lines come back card after card), and each
  // note's words with how often they come, gathered in one pass over the lines.
  const wordCache = new Map<string, string[]>();
  const wordsOf = (s: string) => {
    let w = wordCache.get(s);
    if (!w) wordCache.set(s, (w = words(s)));
    return w;
  };
  let noteWords = $derived.by(() => {
    const by = new Map<number, Map<string, number>>();
    for (const it of store.items) {
      let m = by.get(it.note_id);
      if (!m) by.set(it.note_id, (m = new Map()));
      for (const w of wordsOf(it.text)) m.set(w, (m.get(w) ?? 0) + 1);
    }
    return by;
  });

  /** The note whose title (counts most) or lines share words with the card. */
  let suggestion = $derived.by(() => {
    if (!item) return null;
    const mine = wordsOf(item.text);
    let best: { note: Note; word: string; score: number } | null = null;
    for (const n of notes) {
      let score = 0;
      let word = "";
      const hit = (w: string, weight: number) => {
        const m = mine.find((x) => same(x, w));
        if (!m) return;
        score += weight;
        if (m.length > word.length) word = m;
      };
      for (const w of wordsOf(n.title)) hit(w, 4);
      // (A word that comes back in several lines counts each time, as before: its count.)
      for (const [w, times] of noteWords.get(n.id) ?? []) hit(w, times);
      if (score > (best?.score ?? 0)) best = { note: n, word, score };
    }
    if (!best) return null;
    const original = item.text.split(/\s+/).find((w) => normalize(w) === best.word || normalize(w).startsWith(best.word)) ?? best.word;
    return { note: best.note, word: original.replace(/[^\p{L}\p{N}-]/gu, "") };
  });

  // Each new card: its own text, suggested note.
  $effect(() => {
    const it = item;
    untrack(() => {
      if (!it) return;
      kind = it.kind;
      text = it.text;
      when = "";
      naming = false;
      noteId = suggestion?.note.id ?? notes[0]?.id ?? null;
      place = 0;
    });
  });

  let chosen = $derived(notes.find((n) => n.id === noteId) ?? null);
  let due = $derived(kind === "task" && when.trim() ? parseCapture(`x ${when}`, "task", store.now) : null);

  interface Place {
    how: string;
    target: string;
    parentId: number | null;
  }
  let places = $derived.by(() => {
    if (!chosen) return [] as Place[];
    const top = store.itemsByNote.get(chosen.id) ?? [];
    const list: Place[] = [{ how: kind === "idea" ? t("A new idea in the note") : t("A new task in the note"), target: noteTitle(chosen), parentId: null }];
    if (kind === "task") {
      for (const x of top.filter((i) => i.kind === "task" && i.done_at == null && i.id !== item?.id).slice(0, 6)) list.push({ how: t("A step of the task"), target: x.text, parentId: x.id });
      for (const x of top.filter((i) => i.kind === "idea").slice(0, 4)) list.push({ how: t("The first step of the idea"), target: x.text, parentId: x.id });
    }
    return list;
  });
  let at = $derived(places[Math.min(place, Math.max(0, places.length - 1))]);

  let ageText = $derived.by(() => {
    if (!item) return "";
    const d = new Date(item.created_at);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const diff = Math.round((today.getTime() - new Date(d).setHours(0, 0, 0, 0)) / 86400000);
    const time = hhmm(d);
    return diff <= 0 ? t("Dumped today at {time}", { time }) : diff === 1 ? t("Dumped yesterday at {time}", { time }) : t("Dumped {n} days ago at {time}", { n: diff, time });
  });

  function next(title?: string, dot?: string) {
    if (title) landed = { title, dot: dot ?? "", at: Date.now() };
    queue = queue.slice(1);
    done += 1;
    flying = false;
  }

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  async function fly() {
    if (reduce.matches) return;
    flying = true;
    await new Promise((r) => setTimeout(r, 300));
  }

  async function putAway() {
    if (!item) return;
    let target = chosen;
    if (naming && newName.trim()) {
      target = await store.newNote(newName.trim());
      naming = false;
      newName = "";
    }
    if (!target) return;
    const p = at && target.id === chosen?.id ? at : { parentId: null };
    const it = item;
    await fly();
    await store.moveItem(it, {
      noteId: target.id,
      parentId: p.parentId,
      kind,
      text,
      dueAt: due?.due ? due.due.getTime() : kind === "task" ? it.due_at : null,
      alarm: due?.deadline && due.due ? "deadline" : it.alarm,
    });
    next(noteTitle(target), swatchOf(target, store.custom).light.dot);
  }

  async function remove() {
    if (!item) return;
    const it = item;
    await store.deleteItem(it);
    next();
  }

  async function later() {
    if (!item) return;
    const it = item;
    await store.later(it);
    queue = [...queue.slice(1), it.id];
  }

  function typing(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    return el.tagName === "INPUT" || el.tagName === "TEXTAREA";
  }

  function onKey(e: KeyboardEvent) {
    // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
    // (Nor while something opened over Tidy has the keyboard: its keys are that view's.)
    // (Nor a key already used: the arrows of a row of choices with the keyboard on it.)
    if (!item || e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || composing(e) || !isTopModal(tidyEl)) return;
    if (e.key === "Enter") {
      // (On a button or a choice, Enter presses that one: Close, Delete, another note.)
      if ((e.target as Element).closest?.("button, a[href], [role=button], [role=radio]")) return;
      e.preventDefault();
      putAway();
      return;
    }
    if (typing(e)) return;
    const k = latinKey(e);
    const digit = digitKey(e);
    if (k === "t") kind = "task";
    else if (k === "i") kind = "idea";
    else if (k === "d") remove();
    else if (k === "l") later();
    else if (e.key === "ArrowDown") {
      e.preventDefault();
      place = Math.min(places.length - 1, place + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      place = Math.max(0, place - 1);
    } else if (digit) {
      const n = notes[digit - 1];
      if (n) {
        noteId = n.id;
        place = 0;
      }
    } else return;
    e.preventDefault();
  }
  // The main button's words follow the choice; they crossfade when it changes.
  let goText = $derived(
    naming && newName.trim()
      ? t("Put it in {name}", { name: newName.trim() })
      : at && at.parentId != null
        ? t("Put it under {name}", { name: at.target })
        : t("Put it in {name}", { name: chosen ? noteTitle(chosen) : "…" }),
  );
  // What the words are about (a new note being named, the line it goes under, the note picked):
  // a change of that crossfades; typing the new note's name just updates the words in place.
  let goKey = $derived(naming && newName.trim() ? "new" : at && at.parentId != null ? `under:${at.parentId}` : `in:${chosen?.id ?? ""}`);
</script>

<svelte:window onkeydown={onKey} />

<!-- (The keyboard comes in, on the view itself: its letter keys work at once, Tab goes to the card.) -->
<div class="tidy" role="dialog" aria-label={t("Tidy the Inbox")} tabindex="-1" bind:this={tidyEl} transition:sheet|global={{ y: 8 }} use:modal={{ initial: "self" }}>
  <div class="top" data-drag-area={store.win.mode === "floating" || undefined}>
    <ViewHeader crumbs={[{ label: t("Tidy the Inbox") }]} big={24} fit onBack={onClose} {onClose}>
      <span class="count">{t("{a} of {b}", { a: Math.min(done + 1, total), b: total })}</span>
      <div class="bar" aria-hidden="true"><div style:width="{total ? (done / total) * 100 : 100}%"></div></div>
    </ViewHeader>
  </div>

  <span class="visually-hidden" aria-live="polite">{landed ? t("Landed in {title}", { title: landed.title }) : ""}</span>
  {#if landed}
    {#key landed.at}
      <div class="landed" aria-hidden="true"><span style:background={landed.dot}></span>{t("Landed in {title}", { title: landed.title })}</div>
    {/key}
  {/if}

  {#if item}
    <div class="scroll">
      <div class="stack">
        {#if queue.length > 2}<div class="back b2" aria-hidden="true" transition:fadeQuick={{ duration: 220 }}></div>{/if}
        {#if queue.length > 1}<div class="back b1" aria-hidden="true" transition:fadeQuick={{ duration: 220 }}></div>{/if}
        {#key item.id}
          <div class="front" class:flying>
            <label class="visually-hidden" for="tidy-text">{t("The line")}</label>
            <textarea id="tidy-text" class:idea={kind === "idea"} bind:value={text} rows="2"></textarea>
            {#if item.link}<LinkCard link={item.link} wide />{/if}
            {#if item.file}<FileCard path={item.file} />{/if}
            {#if store.picturesByItem.get(item.id)?.length}
              <div class="pics">
                {#each store.picturesByItem.get(item.id) ?? [] as p (p.id)}{@const src = store.thumbSrc(p)}{#if src === NOT_OURS}<span class="pic-ph"><Icon name="image" size={20} stroke={1.6} /></span>{:else}<img {src} alt="" loading="lazy" decoding="async" onerror={() => thumbs.failed(p)} />{/if}{/each}
              </div>
            {/if}
            <span class="age">{ageText}</span>
          </div>
        {/key}
      </div>

      <div class="form">
        <div class="field">
          <span class="lab">{t("It’s")}</span>
          <div class="kinds" role="radiogroup" use:roving aria-label={t("Kind")}>
            <button role="radio" aria-checked={kind === "task"} class:on={kind === "task"} onclick={() => (kind = "task")}
              ><span class="box" aria-hidden="true"></span>{t("A task")}<span class="key">T</span></button
            >
            <button role="radio" aria-checked={kind === "idea"} class:on={kind === "idea"} onclick={() => (kind = "idea")}><span class="it">{t("An idea")}</span><span class="key">I</span></button>
          </div>
        </div>

        {#if kind === "task"}
          <div class="field" transition:fold={{ gap: 20 }}>
            <label class="lab" for="tidy-when">{t("When?")} <span class="soft">{t("optional · end with\u00a0! for a deadline")}</span></label>
            <div class="when">
              <input id="tidy-when" bind:value={when} placeholder={t("tomorrow 10am, Friday 3pm…")} autocomplete="off" />
              {#if due?.due}<span class="chip" transition:widen={{ gap: 10 }}>{formatDueLong(due.due.getTime(), store.now)} · {due.deadline ? t("deadline") : t("reminds you")}</span>{/if}
            </div>
          </div>
        {/if}

        <div class="field">
          <div class="lab-row">
            <span class="lab">{t("Into which note?")}</span>
            {#if suggestion}<span class="hint" transition:fadeQuick={{ duration: 180 }}>{t("Suggested: {note}, it mentions {word}", { note: noteTitle(suggestion.note), word: suggestion.word })}</span>{/if}
          </div>
          <div class="notes">
            {#each notes as n, k (n.id)}
              <!-- (The note's own paper; in dark mode its dark one, so the light ink stays readable on it.) -->
              {@const sw = swatchOf(n, store.custom)[look.dark ? "dark" : "light"]}
              <button class="note" class:on={noteId === n.id && !naming} aria-pressed={noteId === n.id && !naming} style:--tint={sw.bg} onclick={() => ((noteId = n.id), (naming = false), (place = 0))}>
                {#if k < 9}<span class="num">{k + 1}</span>{/if}
                <span class="ndot" style:background={sw.dot}></span>{noteTitle(n)}
              </button>
            {/each}
            {#if naming}
              <input class="new-name" in:fadeQuick={{ duration: 180 }} bind:value={newName} placeholder={t("New note’s name")} aria-label={t("New note’s name")} />
            {:else}
              <button class="note add" in:fadeQuick={{ duration: 180 }} onclick={() => (naming = true)}>+ {t("New note")}</button>
            {/if}
          </div>
        </div>

        {#if chosen && !naming && places.length > 1}
          <div class="field" transition:fold={{ gap: 20 }}>
            <span class="lab">{t("Where in {title}?", { title: noteTitle(chosen) })} <span class="soft">{t("Up / Down to choose")}</span></span>
            <div class="places" role="radiogroup" use:roving aria-label={t("Place in the note")}>
              {#each places as p, k (k)}
                <button role="radio" aria-checked={k === place} class:on={k === place} onclick={() => (place = k)}>
                  <span class="radio" aria-hidden="true"></span>
                  <span class="how">{p.how}</span>
                  <span class="target" class:serif={k > 0}>{p.target}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="foot" use:toastAbove>
      <button class="ghost" onclick={remove}>{t("Delete")} <span class="key">D</span></button>
      <button class="ghost" onclick={later} disabled={queue.length < 2}>{t("Keep for later")} <span class="key">L</span></button>
      <span class="grow"></span>
      <button class="go" onclick={putAway} disabled={!chosen && !(naming && newName.trim())}>
        <span class="go-swap">
          {#key goKey}<span class="go-text" in:crumbIn out:crumbOut>{goText}</span>{/key}
        </span>
        <span class="key light">{keysText("Enter")}</span>
      </button>
    </div>
  {:else}
    <div class="clear" in:arrive={{ delay: 120, duration: 320, y: 8 }}>
      <span class="big">{t("Inbox clear.")}</span>
      <span class="soft">{t("Everything has a place. Nothing more to tidy.")}</span>
      <button class="go" onclick={onClose}>{t("Back to the board")}</button>
    </div>
  {/if}
</div>

<style>
  .tidy {
    position: absolute;
    inset: 0;
    z-index: 42;
    display: flex;
    flex-direction: column;
    background: var(--ground);
  }
  /* (The same box as Settings' and the Archive's header: Back, the title and × sit exactly
     where they do there, so nothing jumps from one view to the other.) */
  .top {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: var(--s3) var(--s6);
  }
  .top > :global(.vh) {
    flex: 1;
  }
  .count {
    margin-left: 10px;
    font-size: var(--t-md);
    color: var(--muted);
    white-space: nowrap;
  }
  .bar {
    flex: 1;
    margin: 0 10px;
    height: 4px;
    border-radius: 2px;
    background: var(--edge);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--ink);
    border-radius: 2px;
    transition: width 0.4s var(--ease-out);
  }
  .landed {
    position: absolute;
    right: 40px;
    top: 78px;
    z-index: 3;
    padding: 8px 12px;
    border-radius: var(--r-md);
    background: var(--card);
    border: 1px solid var(--line);
    font-size: var(--t-sm);
    box-shadow: var(--card-shadow);
    display: flex;
    align-items: center;
    gap: 6px;
    animation: landed 1.8s var(--ease-out) forwards;
    pointer-events: none;
  }
  .landed span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  @keyframes landed {
    0% {
      opacity: 0;
      transform: translateY(6px) rotate(0deg);
    }
    15% {
      opacity: 1;
      transform: rotate(6deg);
    }
    75% {
      opacity: 0.8;
    }
    100% {
      opacity: 0;
      transform: rotate(6deg) translateY(-4px);
    }
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 40px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .stack {
    position: relative;
    width: min(600px, 100%);
  }
  .back {
    position: absolute;
    inset: 0;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--card);
  }
  .b2 {
    transform: translate(10px, 12px) rotate(2.2deg);
    opacity: 0.6;
  }
  .b1 {
    transform: translate(-6px, 6px) rotate(-1.4deg);
    opacity: 0.85;
  }
  .front {
    position: relative;
    min-height: 150px;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: 22px 26px;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 14px;
    box-shadow: 0 14px 32px rgba(60, 50, 30, 0.12);
    animation: deal 0.36s var(--ease-out);
    transition: transform 0.3s cubic-bezier(0.5, 0, 0.75, 0), opacity 0.3s;
  }
  @keyframes deal {
    from {
      transform: translate(-6px, 6px) rotate(-1.4deg);
    }
  }
  .front.flying {
    transform: translate(240px, -180px) rotate(9deg) scale(0.55);
    opacity: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .front,
    .tidy,
    .landed {
      animation: none;
      transition: none;
    }
  }
  textarea {
    width: 100%;
    border: 0;
    outline: 0;
    resize: none;
    background: transparent;
    font-family: var(--serif);
    font-size: 26px;
    font-weight: 500;
    line-height: 1.25;
    color: var(--ink);
    padding: 0;
    field-sizing: content;
    min-height: 34px;
  }
  textarea.idea {
    font-style: italic;
    font-weight: 400;
  }
  .pics {
    display: flex;
    gap: 6px;
  }
  .pics img,
  .pics .pic-ph {
    width: 96px;
    height: 64px;
    object-fit: cover;
    border-radius: 6px;
  }
  .age {
    font-size: var(--t-md);
    color: var(--muted);
  }
  .form {
    width: min(600px, 100%);
    margin-top: 28px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .lab {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .lab .soft {
    font-weight: 400;
    color: var(--muted);
  }
  .lab-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--s3);
  }
  .hint {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-base);
    color: var(--muted);
    text-align: right;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .kinds button {
    height: 52px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--card);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    font-size: var(--t-lg);
    color: var(--soft);
    cursor: pointer;
  }
  .kinds button.on {
    border: 2px solid var(--ink);
    color: var(--ink);
    font-weight: 600;
  }
  .kinds .box {
    width: 16px;
    height: 16px;
    border: 1.5px solid currentColor;
    border-radius: 4px;
  }
  .kinds .it {
    font-family: var(--serif);
    font-style: italic;
    font-size: 18px;
  }
  .key {
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--muted);
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    padding: 1px 6px;
  }
  .key.light {
    color: inherit;
    opacity: 0.7;
    border-color: rgba(255, 255, 255, 0.3);
  }
  .when {
    height: 44px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 var(--s2) 0 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
  }
  .when input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-lg);
    color: var(--ink);
  }
  .chip {
    flex-shrink: 0;
    font-size: var(--t-sm);
    font-weight: 500;
    color: #7a4d00;
    background: #fbf1da;
    border-radius: 6px;
    padding: 3px 8px;
  }
  .notes {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
  }
  .note {
    height: 38px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--t-md);
    color: var(--ink);
    cursor: pointer;
  }
  .note.on {
    border: 2px solid var(--ink);
    padding: 0 11px;
    background: var(--tint);
    font-weight: 600;
  }
  .note.add {
    border-style: dashed;
    border-color: var(--box);
    background: transparent;
    color: var(--soft);
  }
  .num {
    font-size: var(--t-xs);
    color: var(--muted);
  }
  .ndot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .new-name {
    height: 38px;
    padding: 0 var(--s3);
    border: 2px solid var(--ink);
    border-radius: 10px;
    background: var(--card);
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
    outline: 0;
  }
  .places {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--card);
  }
  .places button {
    min-height: 40px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    color: var(--ink);
    cursor: pointer;
  }
  .places button.on {
    background: var(--chip);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  .radio {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    border: 1.5px solid var(--box);
    border-radius: 50%;
  }
  .on .radio {
    border: 5px solid var(--ink);
  }
  .how {
    flex-shrink: 0;
    font-size: var(--t-md);
    color: var(--soft);
  }
  .target {
    font-size: var(--t-base);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .target.serif {
    font-family: var(--serif);
    font-style: italic;
    font-size: 15px;
  }
  .foot {
    height: 76px;
    flex-shrink: 0;
    padding: 0 var(--s6);
    border-top: 1px solid var(--line);
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .grow {
    flex: 1;
  }
  .ghost {
    height: 44px;
    padding: 0 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--t-base);
    cursor: pointer;
  }
  .ghost {
    white-space: nowrap;
  }
  .go-swap {
    display: grid;
    grid-template-columns: minmax(0, max-content);
    min-width: 0;
  }
  .go-swap > .go-text {
    grid-area: 1 / 1;
  }
  .go-text {
    /* A long note name ends in "…"; the Enter key stays whole. */
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .go .key {
    flex-shrink: 0;
  }
  .ghost:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .go {
    height: 44px;
    max-width: 60%;
    padding: 0 20px;
    border: 0;
    border-radius: 10px;
    background: var(--ink);
    color: var(--card);
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--t-base);
    font-weight: 600;
    cursor: pointer;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .go:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .clear {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s3);
  }
  .clear .big {
    font-family: var(--serif);
    font-size: 30px;
    font-weight: 500;
  }
  .clear .soft {
    font-family: var(--serif);
    font-style: italic;
    font-size: 17px;
    color: var(--muted);
  }
  @container shell (max-width: 560px) {
    .scroll {
      padding: 14px 20px 20px;
    }
    /* Narrow: Delete and Keep for later share the first row, the main button has the whole
       second one (beside them its words were cut down to "R…"). */
    .foot {
      height: auto;
      flex-wrap: wrap;
      row-gap: var(--s2);
      padding: var(--s3) var(--s4);
    }
    .ghost {
      flex: 1 1 0;
      justify-content: center;
    }
    .ghost .key {
      display: none;
    }
    .grow {
      display: none;
    }
    .go {
      flex: 1 0 100%;
      max-width: none;
      justify-content: center;
    }
  }
  /* (It takes the keyboard itself when it opens: no ring round the whole view.) */
  .tidy:focus {
    outline: none;
  }
</style>
