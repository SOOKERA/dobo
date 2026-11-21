<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Archive: notes put away with "Archive note". Each shows as a small card in its own look
  // (colour dot, paper, its first lines), with when it went in. Restore puts it back on the
  // board at its place (its card settles in there once this page has gone); Delete takes it
  // away for good, but Undo (the toast, Ctrl+Z) brings it back until Dobo closes. Empty archive
  // asks first, and is one step Undo takes back whole.
  // Opened from the board's right-click menu, the "See archive" of an Archived toast, or
  // Settings › Archive (then Back goes to Settings).
  import Icon from "./Icon.svelte";
  import StepTrace from "./StepTrace.svelte";
  import ViewHeader, { type Crumb } from "./ViewHeader.svelte";
  import { store } from "../lib/store.svelte";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import { pushLayer } from "../lib/layers";
  import { noteTitle } from "../lib/names";
  import { APP_NAME } from "../lib/app";
  import { arrive, easeHeight, fadeQuick, fold, glide, glideTo, imgFade, leave, reduced, sheet } from "../lib/motion";
  import { dayMonth, dayMonthYear, i18n, t } from "../lib/i18n.svelte";
  import type { Item, Note } from "../lib/types";

  let { from, onBack, onClose }: { from: "board" | "settings"; onBack: () => void; onClose: () => void } = $props();

  // Esc: straight to the board; Back (mouse back, Alt+Left, Backspace): where it was opened from.
  $effect(() => pushLayer(onClose, { back: () => onBack() }));

  let crumbs = $derived<Crumb[]>(from === "settings" ? [{ label: t("Settings"), go: onBack }, { label: t("Archive") }] : [{ label: t("Archive") }]);
  let notes = $derived(store.archived);
  let dark = $derived(look.dark);
  let confirmEmpty = $state(false);
  // (Nothing left to empty: the question goes.)
  $effect(() => {
    if (!notes.length) confirmEmpty = false;
  });
  // While it asks, Esc (or Back) answers "Keep them" first.
  $effect(() => {
    if (!confirmEmpty) return;
    return pushLayer(() => (confirmEmpty = false));
  });

  /** The first lines, as the note shows them: open tasks, then done ones, then ideas. */
  function preview(n: Note): { tasks: Item[]; ideas: Item[]; more: number } {
    const lines = store.itemsByNote.get(n.id) ?? [];
    const open = lines.filter((i) => i.kind === "task" && i.done_at == null);
    const done = lines.filter((i) => i.kind === "task" && i.done_at != null);
    const ideas = lines.filter((i) => i.kind === "idea");
    const tasks = [...open, ...done].slice(0, 3);
    const shownIdeas = ideas.slice(0, Math.max(0, 4 - tasks.length));
    return { tasks, ideas: shownIdeas, more: lines.length - tasks.length - shownIdeas.length };
  }

  function meta(n: Note): string {
    const lines = store.itemsByNote.get(n.id) ?? [];
    const tasks = lines.filter((i) => i.kind === "task");
    const ideas = lines.filter((i) => i.kind === "idea").length;
    const pics = store.picturesByNote.get(n.id)?.length ?? 0;
    const parts = [
      tasks.length ? t("{n} to do", { n: tasks.filter((i) => i.done_at == null).length }) : "",
      ideas ? t("{n} ideas", { n: ideas }) : "",
      pics ? t("{n} pictures", { n: pics }) : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : t("Empty");
  }

  /** "Archived today", "Archived 3 days ago", "Archived on 12 Oct" (with the year when it isn't
   * this one). Archived before the time was kept: "Archived a while ago" (they sort last). */
  function when(n: Note): string {
    if (!n.archived_at) return t("Archived a while ago");
    const d = new Date(n.archived_at);
    const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
    const days = Math.round((start(store.now) - start(d)) / 86_400_000);
    if (days >= 0 && days < 7) {
      const rel = new Intl.RelativeTimeFormat(i18n.locale, { numeric: "auto" }).format(-days, "day");
      return t("Archived {when}", { when: rel });
    }
    return t("Archived on {date}", { date: d.getFullYear() === store.now.getFullYear() ? dayMonth(d) : dayMonthYear(d) });
  }

  // A card leaving: restored, it rises a little toward the board; deleted, it sinks. Out of the
  // flow at once (Svelte holds it in place), so the others glide into its room.
  let leaving = new Map<number, "restore" | "delete">();
  function out(_node: Element, { id }: { id: number }) {
    const how = leaving.get(id) ?? "delete";
    leaving.delete(id);
    if (reduced()) return { duration: 120, css: (t: number) => `opacity:${t}` };
    const dy = how === "restore" ? -14 : 8;
    // (Under the cards gliding into its room, and mostly gone before they get there.)
    return {
      duration: 220,
      easing: (x: number) => x,
      css: (t: number, u: number) => {
        const e = glide(u);
        return `z-index:0;opacity:${Math.pow(t, 1.6).toFixed(3)};transform:translateY(${(e * dy).toFixed(2)}px) scale(${(1 - e * 0.05).toFixed(4)})`;
      },
    };
  }
  function cardIn(_node: Element, { i = 0 }: { i?: number } = {}) {
    if (reduced()) return { duration: 140, css: (t: number) => `opacity:${t}` };
    return {
      delay: 60 + Math.min(i, 8) * 40,
      duration: 320,
      easing: glide,
      css: (t: number, u: number) => `opacity:${t.toFixed(3)};transform:translateY(${(u * 10).toFixed(2)}px) scale(${(0.97 + 0.03 * t).toFixed(4)})`,
    };
  }

  async function restore(n: Note) {
    leaving.set(n.id, "restore");
    await store.unarchive(n);
  }
  async function remove(n: Note) {
    leaving.set(n.id, "delete");
    await store.deleteNote(n);
  }
  async function emptyAll() {
    confirmEmpty = false;
    for (const n of notes) leaving.set(n.id, "delete");
    await store.emptyArchive();
  }

  let sheetEl = $state<HTMLElement>();
  $effect(() => {
    const a = document.activeElement as HTMLElement | null;
    if (!a || a === document.body || !sheetEl?.contains(a)) sheetEl?.focus({ preventScroll: true });
  });

  // Opening: the list fades in a card at a time; a card that comes back (Undo) just arrives.
  let opened = $state(false);
  $effect(() => {
    const id = setTimeout(() => (opened = true), 600);
    return () => clearTimeout(id);
  });
  const lined = (n: Note) => n.paper === "ruled" || n.paper === "grid";
</script>

<!-- (Focused when it opens, so the keyboard is in it: Backspace / Alt+Left go back from here.) -->
<div class="sheet" role="dialog" aria-label={t("Archive")} tabindex="-1" bind:this={sheetEl} transition:sheet|global>
  <div class="top" data-drag-area={store.win.mode === "floating" || undefined}>
    <ViewHeader {crumbs} big={24} sub={notes.length ? t("{n} notes", { n: notes.length }) : ""} {onBack} {onClose} />
  </div>

  <!-- (The row eases to its new height when the question takes the button's place.) -->
  <div class="intro-room" use:easeHeight>
  <div class="intro-row">
    <p class="intro">{t("Notes you put away, with their lines and pictures. Their reminders stay quiet until they come back.")}</p>
    {#if notes.length}
      <span class="empty-swap" transition:fadeQuick>
        {#if confirmEmpty}
          <span class="confirm" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
            <span class="ask">{t("Delete all {n} archived notes?", { n: notes.length })}</span>
            <span class="btns">
              <button class="small" onclick={() => (confirmEmpty = false)}>{t("Keep them")}</button>
              <button class="small danger" onclick={emptyAll}>{t("Delete all")}</button>
            </span>
          </span>
        {:else}
          <span class="confirm" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
            <button class="small" onclick={() => (confirmEmpty = true)}><Icon name="trash" size={14} />{t("Empty archive…")}</button>
          </span>
        {/if}
      </span>
    {/if}
  </div>
  </div>
  {#if confirmEmpty}
    <p class="undo-note" transition:fold={{ duration: 240 }}>{t("Undo can bring them back until {app} closes.", { app: APP_NAME })}</p>
  {/if}

  <div class="list" data-grid>
    {#each notes as n, i (n.id)}
      {@const tone = dark ? swatchOf(n, store.custom).dark : swatchOf(n, store.custom).light}
      {@const p = preview(n)}
      {@const pics = store.picturesByNote.get(n.id) ?? []}
      <article
        class="card paper-{n.paper ?? 'plain'}"
        class:lined={lined(n)}
        style:--rule={tone.corner}
        aria-label={noteTitle(n)}
        in:cardIn={{ i: opened ? 0 : i }}
        out:out={{ id: n.id }}
        animate:glideTo={{ duration: 300 }}
      >
        <div class="head">
          <span class="dot" style:background={tone.dot}></span>
          <h3 class="title">{noteTitle(n)}</h3>
        </div>
        <div class="meta">
          <span>{meta(n)}</span>
          <span class="space">{n.space === "work" ? t("Work") : t("Personal")}</span>
        </div>
        {#if p.tasks.length || p.ideas.length}
          <div class="lines" aria-hidden="true">
            {#each p.tasks as it (it.id)}
              {@const steps = store.stepsOf.get(it.id) ?? []}
              {@const stepsDone = steps.filter((x) => x.done_at != null).length}
              <div class="task" class:done={it.done_at != null}
                ><span class="box" class:all={it.done_at == null && steps.length > 0 && stepsDone === steps.length}
                  >{#if it.done_at != null}<Icon name="check" size={10} stroke={2.6} />{/if}{#if steps.length && it.done_at == null}<span class="trace"><StepTrace done={stepsDone} total={steps.length} size={18} radius={6} stroke={1.2} /></span>{/if}</span
                ><span class="txt">{it.text}</span>{#if steps.length}<span class="count">{stepsDone}/{steps.length}</span>{/if}</div
              >
            {/each}
            {#if p.ideas.length}
              <div class="ideas" style:--tint={tone.bg} style:--idea-ink={tone.ink}>
                {#each p.ideas as it (it.id)}<span class="txt">{it.text}</span>{/each}
              </div>
            {/if}
            {#if p.more > 0}<span class="more-lines">{t("and {n} more lines", { n: p.more })}</span>{/if}
          </div>
        {/if}
        {#if pics.length}
          <div class="pics" aria-hidden="true">
            {#each pics.slice(0, 4) as pic (pic.id)}<img src={store.src(pic.path)} alt="" loading="lazy" draggable="false" use:imgFade />{/each}
            {#if pics.length > 4}<span class="pics-more">+{pics.length - 4}</span>{/if}
          </div>
        {/if}
        <div class="foot">
          <span class="when">{when(n)}</span>
          <span class="acts">
            <button class="act del" aria-label={t("Delete “{name}”", { name: noteTitle(n) })} data-tip={t("Delete · Undo can bring it back")} onclick={() => remove(n)}><Icon name="trash" size={15} /></button>
            <button class="small strong" data-tip={t("Back on the board, at its place")} onclick={() => restore(n)}>{t("Restore")}</button>
          </span>
        </div>
      </article>
    {/each}
  </div>

  {#if !notes.length}
    <div class="nothing" in:arrive={{ delay: opened ? 260 : 120, duration: 320, y: 8 }} out:fadeQuick={{ duration: 120 }}>
      <div class="tray" aria-hidden="true"><span class="c1"></span><span class="c2"></span><span class="c3"></span></div>
      <p class="big">{t("Nothing in the archive")}</p>
      <p class="soft">{t("Archive a note from its ⋯ menu to put it away without deleting it. It waits here, quiet, until you need it again.")}</p>
      <button class="small" onclick={onClose}>{t("Back to the board")}</button>
    </div>
  {/if}
</div>

<style>
  .sheet {
    position: absolute;
    inset: 0;
    z-index: 41;
    outline: none;
    overflow-y: auto;
    padding: 0 var(--s6) var(--s6);
    background: var(--ground);
  }
  .top {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 calc(-1 * var(--s6));
    padding: var(--s3) var(--s6);
    background: var(--ground);
  }
  .intro-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2) var(--s4);
    padding: var(--s1) 0 var(--s3);
    border-bottom: 1px solid var(--line);
  }
  .intro {
    flex: 1 1 260px;
    margin: 0;
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--muted);
  }
  /* (Positioned: what leaves steps out of the flow in place, not at the top of the page.) */
  .empty-swap {
    position: relative;
    display: grid;
    justify-items: end;
    margin-left: auto;
  }
  .empty-swap > * {
    grid-area: 1 / 1;
  }
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: var(--s2) var(--s3);
  }
  .ask {
    font-size: var(--t-md);
    font-weight: 600;
  }
  .btns {
    display: flex;
    gap: var(--s2);
  }
  .undo-note {
    margin: var(--s2) 0 0;
    text-align: right;
    text-wrap: balance;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .small {
    height: 30px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--t-md);
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s;
  }
  .small:hover {
    background: var(--hover);
  }
  .small.strong {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .small.strong:hover {
    background: color-mix(in srgb, var(--ink) 86%, var(--card));
  }
  .small.danger {
    background: var(--late-ink);
    border-color: var(--late-ink);
    color: #fff;
  }
  .small.danger:hover {
    background: color-mix(in srgb, var(--late-ink) 88%, #000);
  }

  /* The cards: a grid that fills the width (one column in a narrow panel). */
  .list {
    position: relative;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(232px, 1fr));
    gap: var(--s4);
    margin-top: var(--s4);
    align-items: start;
  }
  .card {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 10px 12px 10px;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--card-shadow);
  }
  .card.paper-antique {
    background: var(--antique);
  }
  .card.lined {
    background-image: repeating-linear-gradient(to bottom, transparent 0 19px, color-mix(in srgb, var(--rule) 70%, transparent) 19px 20px, transparent 20px 24px);
    background-position: 0 12px;
  }
  .card.paper-grid {
    background-image: repeating-linear-gradient(to bottom, transparent 0 19px, color-mix(in srgb, var(--rule) 65%, transparent) 19px 20px, transparent 20px 24px),
      repeating-linear-gradient(to right, transparent 0 23px, color-mix(in srgb, var(--rule) 40%, transparent) 23px 24px);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    margin-left: 2px;
    border-radius: 50%;
  }
  .title {
    min-width: 0;
    margin: 0;
    font-family: var(--serif);
    font-size: 17px;
    font-weight: 600;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: var(--s2);
    margin: 3px 0 0 21px;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .meta > span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .space {
    flex: none;
  }
  .lines {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 8px 0 0 21px;
    font-size: var(--t-md);
    color: var(--ink);
  }
  .task {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    line-height: 21px;
  }
  .box {
    flex: none;
    width: 12px;
    height: 12px;
    border: 1.5px solid var(--box);
    border-radius: 3px;
    display: grid;
    place-items: center;
    color: var(--card);
  }
  /* A task's steps: their progress in a thin ring round its box, as on the board. */
  .box {
    position: relative;
  }
  .trace {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 18px;
    height: 18px;
    margin: -9px;
  }
  .box.all {
    border-color: var(--ink);
  }
  /* At the line's right end, as on the board (the counts line up down the card). */
  .count {
    flex: none;
    margin-left: auto;
    font-size: var(--t-xs);
    font-weight: 600;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .task.done .box {
    background: var(--box);
  }
  .task.done .txt {
    color: var(--muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--muted) 60%, transparent);
  }
  .txt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ideas {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 2px 0 0 -6px;
    padding: 5px 8px;
    border-radius: 8px;
    background: var(--tint);
    color: var(--idea-ink);
    font-family: var(--serif);
    font-style: italic;
    font-size: 15px;
    line-height: 1.35;
  }
  .pics {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 0 0 21px;
  }
  .pics img {
    width: 44px;
    height: 34px;
    border-radius: 6px;
    object-fit: cover;
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    border: 1px solid var(--line);
    opacity: 0;
    transition: opacity 0.2s;
  }
  .pics img:global(.loaded) {
    opacity: 1;
  }
  .pics img:global(.instant) {
    transition: none;
  }
  .pics-more {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .more-lines {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2);
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid color-mix(in srgb, var(--line) 80%, transparent);
  }
  .when {
    min-width: 0;
    font-size: var(--t-sm);
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acts {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .act {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }
  .act.del:hover,
  .act.del:focus-visible {
    background: color-mix(in srgb, var(--late-ink) 12%, transparent);
    color: var(--late-ink);
  }

  /* Nothing archived: a quiet tray of three blank cards. */
  .nothing {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s2);
    max-width: 360px;
    margin: 56px auto 0;
    text-align: center;
  }
  .tray {
    position: relative;
    width: 108px;
    height: 74px;
    margin-bottom: var(--s3);
  }
  .tray span {
    position: absolute;
    left: 50%;
    top: 0;
    width: 68px;
    height: 54px;
    margin-left: -34px;
    border: 1.5px dashed color-mix(in srgb, var(--ink) 22%, transparent);
    border-radius: 10px;
    background: var(--card);
  }
  .tray .c1 {
    transform: translate(-16px, 10px) rotate(-8deg);
  }
  .tray .c2 {
    transform: translate(16px, 12px) rotate(7deg);
  }
  .tray .c3 {
    transform: translateY(18px);
    border-style: solid;
    border-color: var(--line);
    box-shadow: var(--card-shadow);
  }
  .big {
    margin: 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: 19px;
    color: var(--ink);
  }
  .soft {
    margin: 0 0 var(--s3);
    font-size: var(--t-md);
    line-height: 1.5;
    color: var(--muted);
  }
</style>
