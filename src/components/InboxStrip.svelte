<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Inbox: a strip across the top of the board, not a note. Folded, it says how many lines
  // wait ("Inbox · 3 waiting"), with Tidy, its pin and its chevron; a click (or Enter) unfolds it
  // downwards and the notes glide down to make room. Its lines show in two columns when there is
  // room. They are dragged out onto the note they belong to (or the hours, or a task); lines can
  // be dropped into it too. The pin keeps it open (remembered); unpinned, it folds only when
  // asked (a click, Esc). It can't be moved, archived, deleted or renamed, and has no colour or
  // paper of its own. Each space has its own Inbox: with All, both show, Work first.
  import Icon from "./Icon.svelte";
  import ItemRow from "./ItemRow.svelte";
  import IdeaRow from "./IdeaRow.svelte";
  import NotePictures from "./NotePictures.svelte";
  import { store } from "../lib/store.svelte";
  import { itemDrag } from "../lib/itemdrag.svelte";
  import { inboxGroups } from "../lib/inbox";
  import { isStrip } from "../lib/keys";
  import { untrack } from "svelte";
  import { pushLayer } from "../lib/layers";
  import { swatchOf } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import { crumbIn, crumbOut, fold, gridGlide, reduced, widen } from "../lib/motion";
  import { keysText, t } from "../lib/i18n.svelte";

  let shown = $derived(store.inboxShown);
  let ids = $derived(new Set(store.stripNotes.map((n) => n.id)));
  let groups = $derived(inboxGroups());
  // With All, each space's lines go under its name: both, or the one there is when it isn't the
  // space new lines go to (Personal's lines alone, while new lines go to Work).
  let labelled = $derived(store.view === "all" && (groups.length > 1 || (groups.length === 1 && groups[0].space !== store.space)));
  // (As the header's badge: the tour's sample counts, as if just dumped.)
  let waiting = $derived(store.inboxWaiting.length);
  let focused = $derived(!!store.focus && store.focus.item == null && isStrip(store.focus.note));
  let dropHere = $derived(itemDrag.target?.kind === "inbox");
  // Ideas sit on a slip of butter paper, as they would in a note.
  let tone = $derived(swatchOf({ palette: "soft", color: "Butter" }, store.custom)[look.dark ? "dark" : "light"]);
  let pinned = $derived(store.inboxPinned);

  // Opened by hand (not pinned): Esc folds it, as it closes anything open beside the board.
  $effect(() => {
    if (store.inboxOpen && !store.inboxPinned && !store.inboxTour) return pushLayer(() => (store.inboxOpen = false), undefined, { beside: true });
  });

  // Folded while the keyboard was on one of its lines: the keyboard stays on the strip's head
  // (not on a line no longer shown).
  $effect(() => {
    if (shown) return;
    const f = store.focus;
    if (f && f.item != null && isStrip(f.note))
      untrack(() => {
        store.focus = { note: f.note, item: null };
      });
  });

  function toggle() {
    store.toggleInbox();
  }
  /** A click on the strip's head, outside its buttons, opens or folds it too. */
  function headClick(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest("button")) toggle();
  }

  // Jumped to (search, a notification): it shows, and glows a moment.
  let el: HTMLElement;
  $effect(() => {
    const g = store.glow;
    if (!g || !ids.has(g.id) || !el) return;
    el.scrollIntoView({ block: "nearest", behavior: reduced() ? "auto" : "smooth" });
    el.animate(
      [
        { boxShadow: "0 0 0 0 rgba(233,196,106,0)" },
        { boxShadow: "0 0 0 4px rgba(233,196,106,0.75)", offset: 0.25 },
        { boxShadow: "0 0 0 4px rgba(233,196,106,0.6)", offset: 0.7 },
        { boxShadow: "0 0 0 0 rgba(233,196,106,0)" },
      ],
      { duration: 1600, easing: "ease-out" },
    );
  });

  let count = $derived(waiting ? t("{n} waiting", { n: waiting }) : t("empty"));
  let tip = $derived(shown ? t("Fold the Inbox") : t("Open the Inbox"));
</script>

<section
  class="inbox"
  class:open={shown}
  class:focused
  class:drop={dropHere}
  bind:this={el}
  aria-label={t("Inbox")}
  data-inbox-strip
  data-drop-inbox={store.space}
  style:--tint={tone.bg}
  style:--idea-ink={tone.ink}
  style:--corner={tone.corner}
>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (the strip's button below takes the keys) -->
  <div class="head" onclick={headClick}>
    <button class="opener" aria-expanded={shown} aria-controls="inbox-lines" data-tip={tip} onclick={toggle}>
      <span class="tray"><Icon name="inbox" size={17} /></span>
      <span class="words">
        <span class="name">{t("Inbox")}</span>
        <span class="count" aria-live="polite">
          {#key count}<span in:crumbIn out:crumbOut>· {count}</span>{/key}
        </span>
      </span>
    </button>
    {#if !waiting && !shown}
      <span class="hint" transition:widen={{ gap: 0 }}>{t("Type above, or {keys} from anywhere", { keys: keysText(store.shortcuts.dump) })}</span>
    {/if}
    <span class="end">
      {#if waiting}
        <button class="tidy" data-tour="tidy" transition:widen={{ gap: 8 }} data-tip={t("Tidy the Inbox, one card at a time")} onclick={() => window.dispatchEvent(new CustomEvent("dobo-tidy"))}>{t("Tidy")}</button>
      {/if}
      <button
        class="small pin"
        class:on={pinned}
        aria-label={t("Keep the Inbox open")}
        aria-pressed={pinned}
        data-tip={pinned ? t("Keep the Inbox open · on") : t("Keep the Inbox open · off")}
        onclick={() => store.setInboxPinned(!pinned)}><Icon name="pin" size={15} /></button
      >
      <button class="small chev" class:up={shown} aria-label={tip} aria-expanded={shown} aria-controls="inbox-lines" data-tip={tip} data-tip-keys={shown && !pinned ? "Esc" : null} onclick={toggle}
        ><Icon name="chev-down" size={15} stroke={2.2} /></button
      >
    </span>
  </div>

  {#if shown}
    <div class="body" id="inbox-lines" transition:fold={{ y: -6, duration: 340, room: 1, show: 0.15 }}>
      {#each groups as g (g.space)}
        <!-- (A line dropped over a space's lines goes to that space's Inbox.) -->
        <div class="group" transition:fold data-drop-inbox={g.space}>
          {#if labelled}<span class="space" transition:fold>{g.space === "work" ? t("Work") : t("Personal")}</span>{/if}
          <ul class="lines" use:gridGlide>
            {#each g.tasks as it (it.id)}
              <ItemRow item={it} />
            {/each}
            {#each g.ideas as it (it.id)}
              <IdeaRow item={it} as="li" gap={0} />
            {/each}
          </ul>
          {#if g.pictures.length}<div class="pics"><NotePictures pictures={g.pictures} /></div>{/if}
        </div>
      {:else}
        <p class="empty" transition:fold>{t("Nothing waiting. Lines typed without a #note land here, and so do quick dumps ({keys}).", { keys: keysText(store.shortcuts.dump) })}</p>
      {/each}
    </div>
  {/if}
</section>

<style>
  .inbox {
    position: relative;
    z-index: 3;
    margin-bottom: 18px;
    container: inbox / inline-size;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--card-shadow);
    transition:
      border-color 0.15s,
      box-shadow 0.15s;
  }
  /* A line's menu opens over the notes under the strip. */
  .inbox:has(:global(.menu-open)) {
    z-index: 30;
  }
  .inbox.drop {
    border-color: var(--ink);
    box-shadow: var(--card-shadow), 0 0 0 1px var(--ink);
  }
  .inbox.focused {
    box-shadow: var(--card-shadow), 0 0 0 3px var(--ground), 0 0 0 5px var(--focus-ring);
  }

  /* ---- The head: tray, name, count; then Tidy, the pin and the chevron ---- */
  .head {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: 46px;
    padding: 0 var(--s2) 0 10px;
    cursor: pointer;
    user-select: none;
  }
  .opener {
    flex: 0 1 auto;
    min-width: 0;
    height: 34px;
    margin-left: -4px;
    padding: 0 6px 0 4px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--ink);
    display: flex;
    align-items: center;
    gap: var(--s2);
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .opener:hover {
    background: var(--hover);
  }
  .tray {
    flex-shrink: 0;
    width: 24px;
    display: grid;
    place-items: center;
    color: var(--icon);
  }
  /* The name and its count on one baseline. */
  .words {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
    min-width: 0;
  }
  .name {
    flex-shrink: 0;
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.25;
    letter-spacing: -0.005em;
  }
  .count {
    display: grid;
    min-width: 0;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
  }
  .count > span {
    grid-area: 1 / 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint {
    flex: 1 1 0;
    min-width: 0;
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .end {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .tidy {
    flex-shrink: 0;
    height: 24px;
    margin-right: 6px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-sm);
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .tidy:hover {
    opacity: 0.88;
  }
  .small {
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
    transition:
      background-color 0.2s var(--ease-out),
      color 0.2s var(--ease-out);
  }
  .small:hover {
    background: var(--chip);
    color: var(--ink);
  }
  /* The pin, on: filled with the ink, as the header's switches. */
  .pin.on {
    background: var(--ink);
    color: var(--card);
  }
  .pin.on:hover {
    background: color-mix(in srgb, var(--ink) 86%, var(--ground));
    color: var(--card);
  }
  .pin.on > :global(svg) {
    animation: pin-press 0.32s var(--ease-out);
  }
  @keyframes pin-press {
    0% {
      transform: scale(0.82);
    }
    55% {
      transform: scale(1.06);
    }
    100% {
      transform: none;
    }
  }
  .chev > :global(svg) {
    transition: transform 0.34s var(--ease-glide);
  }
  .chev.up > :global(svg) {
    transform: rotate(180deg);
  }
  @media (prefers-reduced-motion: reduce) {
    .pin.on > :global(svg) {
      animation: none;
    }
    .chev > :global(svg) {
      transition: none;
    }
  }
  /* A narrow strip keeps the name, the count and its buttons; the hint gives way. */
  @container inbox (max-width: 460px) {
    .hint {
      display: none;
    }
  }

  /* ---- Its lines: two columns when there is room ---- */
  .body {
    padding: 0 var(--s2) 10px;
  }
  .group + .group {
    margin-top: var(--s2);
  }
  .space {
    display: block;
    padding: 4px var(--s2) 2px;
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .lines {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 2px 0 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    column-gap: var(--s4);
    align-items: start;
  }
  @container inbox (min-width: 514px) {
    .lines {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  /* An idea: a slip of butter paper among the tasks. */
  .lines > :global(li.idea-wrap) {
    margin: 3px 0;
    padding: 6px 12px 7px;
    border-radius: var(--r-md);
    background: var(--tint);
  }
  .pics {
    max-width: 360px;
    padding: var(--s2) var(--s2) 0;
  }
  .empty {
    margin: 0;
    padding: 2px var(--s2) 6px 34px;
    font-size: var(--t-sm);
    color: var(--muted);
  }
</style>
