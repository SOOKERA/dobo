<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Jump to / search (Find board): notes, tasks and ideas, in all spaces or this one.
  // Up/Down move, Enter jumps to the note and makes it glow, Esc closes.
  import { composing } from "../lib/compose";
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import ViewHeader from "./ViewHeader.svelte";
  import { fadeQuick, pop, reduced, veil } from "../lib/motion";
  import { store } from "../lib/store.svelte";
  import { normalize } from "../lib/capture";
  import { pushLayer } from "../lib/layers";
  import { modal } from "../lib/modal";
  import { isWindowDrag } from "../lib/windrag";
  import { swatchOf } from "../lib/palette";
  import type { Item, Note } from "../lib/types";
  import { keysText, t } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";

  let { onClose }: { onClose: () => void } = $props();

  let q = $state("");
  let scope = $state<"all" | "this">("all");
  let sel = $state(0);
  let input: HTMLInputElement;

  $effect(() => pushLayer(onClose));

  interface Hit {
    key: string;
    kind: "note" | "task" | "idea";
    text: string;
    where: string;
    dot: string;
    noteId: number;
    done: boolean;
  }

  const names = { work: "Work", personal: "Personal" } as const;

  function inScope(n: Note | undefined): n is Note {
    if (!n || n.archived) return false;
    return scope === "all" || store.view === "all" || n.space === store.view;
  }

  let groups = $derived.by(() => {
    const w = normalize(q);
    if (!w) return [] as { label: string; hits: Hit[] }[];
    // (The Inbox has no colour of its own: a quiet grey.)
    const dotOf = (n: Note) => (store.inboxIds.has(n.id) ? "var(--box)" : swatchOf(n, store.custom).light.dot);
    const notes: Hit[] = store.notes
      .filter((n) => inScope(n) && (normalize(n.title).includes(w) || normalize(noteTitle(n)).includes(w)))
      .slice(0, 6)
      .map((n) => {
        const its = store.itemsByNote.get(n.id) ?? [];
        const open = its.filter((i) => i.kind === "task" && i.done_at == null).length;
        const ideas = its.filter((i) => i.kind === "idea").length;
        return { key: `n${n.id}`, kind: "note", text: noteTitle(n), where: `${t(names[n.space])} · ${t("{n} to do", { n: open })} · ${t("{n} ideas", { n: ideas })}`, dot: dotOf(n), noteId: n.id, done: false };
      });
    const lines = (kind: "task" | "idea") =>
      store.items
        .filter((i: Item) => i.kind === kind && normalize(i.text).includes(w))
        .map((i) => ({ i, n: store.notes.find((n) => n.id === i.note_id) }))
        .filter((x): x is { i: Item; n: Note } => inScope(x.n))
        .sort((a, b) => Number(a.i.done_at != null) - Number(b.i.done_at != null))
        .slice(0, 8)
        .map(({ i, n }) => ({
          key: `i${i.id}`,
          kind,
          text: i.text,
          where: store.view === "all" || n.space !== store.view ? `${t(names[n.space])} · ${noteTitle(n)}` : noteTitle(n),
          dot: dotOf(n),
          noteId: n.id,
          done: i.done_at != null,
        }));
    return [
      { label: t("Jump to note").toLocaleUpperCase(), hits: notes },
      { label: t("Tasks").toLocaleUpperCase(), hits: lines("task") },
      { label: t("Ideas").toLocaleUpperCase(), hits: lines("idea") },
    ].filter((g) => g.hits.length);
  });
  let flat = $derived(groups.flatMap((g) => g.hits));

  $effect(() => {
    void q;
    void scope;
    sel = 0;
  });

  /** Text with the match marked (accents and case ignored when possible). */
  function parts(text: string): [string, string, string] {
    const lower = text.toLowerCase();
    const needle = q.trim().toLowerCase();
    let i = needle ? lower.indexOf(needle) : -1;
    if (i < 0 && needle) {
      const plain = text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
      const n2 = needle.normalize("NFD").replace(/\p{M}/gu, "");
      i = plain.indexOf(n2);
      if (i >= 0 && plain.length === text.length) return [text.slice(0, i), text.slice(i, i + n2.length), text.slice(i + n2.length)];
      return [text, "", ""];
    }
    return i < 0 ? [text, "", ""] : [text.slice(0, i), text.slice(i, i + needle.length), text.slice(i + needle.length)];
  }

  /** Jumped: the keyboard goes onto the board, on what was found (its Up / Down go on from there),
   * not back to what had it before. */
  let jumped = false;
  function jump(h: Hit | undefined) {
    if (!h) return;
    jumped = true;
    onClose();
    store.focusNote(h.noteId);
    const line = h.kind === "note" ? null : Number(h.key.slice(1));
    // (A line in a folded note: the note opens to show it.)
    const n = store.notes.find((x) => x.id === h.noteId);
    if (line != null && n?.folded && !store.inboxIds.has(n.id)) void store.setFolded(n, false);
    store.focus = { note: h.noteId, item: line };
    (document.activeElement as HTMLElement | null)?.blur();
  }

  // For screen readers: how many were found, once typing pauses.
  let count = $state("");
  $effect(() => {
    const n = flat.length;
    const said = q.trim() ? t("{n} results", { n }) : "";
    const id = setTimeout(() => (count = said), 500);
    return () => clearTimeout(id);
  });

  function onKey(e: KeyboardEvent) {
    // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
    if (composing(e)) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(flat.length - 1, sel + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(0, sel - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      jump(flat[sel]);
    }
  }

  // The selection is one soft block that glides from line to line (fast), instead of jumping.
  let resultsEl = $state<HTMLDivElement>();
  let hl = $state<{ y: number; h: number; glide: boolean } | null>(null);
  $effect(() => {
    void sel;
    void flat;
    const el = resultsEl?.querySelector<HTMLElement>(".hit.sel");
    if (!el) return void (hl = null);
    const had = untrack(() => !!hl);
    hl = { y: el.offsetTop, h: el.offsetHeight, glide: had && !reduced() };
    // The window resized, a line wrapped differently: the block follows at once (no glide).
    const ro = new ResizeObserver(() => {
      if (!el.isConnected) return;
      const y = el.offsetTop;
      const h = el.offsetHeight;
      const now = untrack(() => hl);
      if (now && (now.y !== y || now.h !== h)) hl = { y, h, glide: false };
    });
    ro.observe(el);
    if (resultsEl) ro.observe(resultsEl);
    return () => ro.disconnect();
  });
</script>

<!-- (Floating, a press on the veil over the top bar moves the window and keeps the search open.) -->
<div class="veil" aria-hidden="true" data-drag-through onpointerdown={(e) => !isWindowDrag(e) && onClose()} transition:veil|global></div>
<div class="find" role="dialog" aria-label={t("Search")} transition:pop|global={{ y: -8, scale: 0.99, duration: 260 }} use:modal={{ initial: "#find", restore: () => !jumped }}>
  <div class="head">
    <ViewHeader crumbs={[{ label: t("Search") }]} big={20} onBack={onClose} {onClose}>
      <div class="seg" role="group" aria-label={t("Search in")}>
        <button class:on={scope === "all"} aria-pressed={scope === "all"} onclick={() => (scope = "all")}>{t("All spaces")}</button>
        <button class:on={scope === "this"} aria-pressed={scope === "this"} onclick={() => (scope = "this")} disabled={store.view === "all"}>{t("This space")}</button>
      </div>
    </ViewHeader>
  </div>
  <div class="top">
    <Icon name="search" />
    <label for="find" class="visually-hidden">{t("Jump to a note or find a line")}</label>
    <input
      id="find"
      bind:this={input}
      bind:value={q}
      onkeydown={onKey}
      placeholder={t("Find a note, a task, an idea")}
      autocomplete="off"
      spellcheck="false"
      role="combobox"
      aria-expanded={flat.length > 0}
      aria-controls="find-results"
      aria-autocomplete="list"
      aria-activedescendant={flat[sel] ? `hit-${flat[sel].key}` : undefined}
    />
    <span class="visually-hidden" aria-live="polite">{count}</span>
  </div>

  <div class="results" id="find-results" role="listbox" aria-label={t("Results")} bind:this={resultsEl}>
    {#if hl}<span class="hl" class:glide={hl.glide} style:transform="translateY({hl.y}px)" style:height="{hl.h}px" aria-hidden="true" transition:fadeQuick={{ duration: 120 }}></span>{/if}
    {#if q.trim() && !flat.length}
      <p class="none" in:fadeQuick={{ delay: 60, duration: 180 }}>{t("Nothing matches “{q}”.", { q: q.trim() })}</p>
    {/if}
    {#each groups as g (g.label)}
      <div class="label" role="presentation" in:fadeQuick={{ duration: 140 }}>{g.label}</div>
      {#each g.hits as h (h.key)}
        {@const k = flat.indexOf(h)}
        {@const [a, b, c] = parts(h.text)}
        <!-- (Options of the search field: Up / Down move through them, the keyboard stays in the field.) -->
        <button
          class="hit {h.kind}"
          id="hit-{h.key}"
          role="option"
          aria-selected={k === sel}
          tabindex="-1"
          in:fadeQuick={{ duration: 140 }}
          class:sel={k === sel}
          class:done={h.done}
          onclick={() => jump(h)}
          onpointermove={() => (sel = k)}
        >
          {#if h.kind === "note"}
            <span class="ndot" style:background={h.dot}></span>
          {:else if h.kind === "task"}
            <span class="box"></span>
          {:else}
            <span class="dash"></span>
          {/if}
          <span class="text">{a}<mark>{b}</mark>{c}</span>
          <span class="where">
            {#if h.kind !== "note"}<span class="wdot" style:background={h.dot}></span>{/if}{h.where}
          </span>
          {#if k === sel}<span class="enter" in:fadeQuick={{ duration: 120 }}>{keysText("Enter")}</span>{/if}
        </button>
      {/each}
    {/each}
  </div>

  <div class="foot">
    <span>{t("Up / Down to move")}</span>
    <span>{t("Enter jumps to it and makes it glow")}</span>
    <span>{t("Esc closes")}</span>
    <span class="push">{t("Open with")} <b>{keysText(store.shortcuts.search)}</b> {t("or")} <b>/</b> {t("on the board")}</span>
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    z-index: 44;
    background: color-mix(in srgb, var(--ground) 55%, transparent);
  }
  .find {
    position: absolute;
    left: var(--s6);
    right: var(--s6);
    top: var(--head-h, 60px);
    z-index: 45;
    max-height: calc(100% - 84px);
    display: flex;
    flex-direction: column;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(40, 32, 20, 0.18), 0 4px 12px rgba(40, 32, 20, 0.08);
    overflow: hidden;
    transform-origin: 50% 0;
  }
  .head {
    flex-shrink: 0;
    padding: var(--s2) var(--s3) 0 var(--s5);
    color: var(--ink);
  }
  .top {
    height: 52px;
    flex-shrink: 0;
    padding: 0 var(--s4) 0 var(--s5);
    display: flex;
    align-items: center;
    gap: var(--s3);
    border-bottom: 1px solid var(--line);
    color: var(--ink);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: 18px;
    color: var(--ink);
  }
  input::placeholder {
    color: var(--muted);
  }
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
    flex-shrink: 0;
  }
  .seg button {
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .seg button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .results {
    position: relative;
    overflow-y: auto;
    padding: var(--s2) var(--s2) var(--s1);
  }
  .none {
    margin: var(--s4) var(--s3);
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
    color: var(--muted);
  }
  .label {
    padding: 10px var(--s3) 4px;
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--muted);
    letter-spacing: 0.03em;
  }
  .hit {
    width: 100%;
    min-height: 40px;
    padding: var(--s2) var(--s3);
    border: 0;
    border-radius: 9px;
    background: transparent;
    display: flex;
    align-items: center;
    gap: var(--s3);
    text-align: left;
    color: var(--ink);
    cursor: pointer;
  }
  .hl {
    position: absolute;
    left: var(--s2);
    right: var(--s2);
    top: 0;
    border-radius: 9px;
    background: var(--chip);
    pointer-events: none;
  }
  .hl.glide {
    transition: transform 0.14s var(--ease-out), height 0.14s var(--ease-out);
  }
  .hit {
    position: relative;
  }
  .hit.note {
    min-height: 44px;
  }
  .ndot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .box {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    border: 1.5px solid var(--box);
    border-radius: 4px;
  }
  .dash {
    width: 14px;
    height: 1.5px;
    flex-shrink: 0;
    background: var(--box);
  }
  .text {
    flex: 1;
    min-width: 0;
    font-size: var(--t-base);
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .note .text {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
  }
  .idea .text {
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
  }
  .done .text {
    color: var(--done-ink);
    text-decoration: line-through;
  }
  mark {
    background: rgba(233, 196, 106, 0.55);
    color: inherit;
    border-radius: 3px;
  }
  /* (A softer amber on the dark card, and a done line's match in readable ink.) */
  :global(:root[data-mode="dark"]) mark {
    background: rgba(233, 196, 106, 0.28);
  }
  .done mark {
    color: var(--soft);
  }
  .where {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wdot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .enter {
    flex-shrink: 0;
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    padding: 1px 6px;
  }
  .foot {
    height: 44px;
    flex-shrink: 0;
    padding: 0 var(--s5);
    border-top: 1px solid var(--line);
    display: flex;
    align-items: center;
    gap: 18px;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .push {
    margin-left: auto;
  }
  .foot b {
    color: var(--soft);
    font-weight: 600;
  }
  @container shell (max-width: 640px) {
    .foot span:not(.push) {
      display: none;
    }
  }
</style>
