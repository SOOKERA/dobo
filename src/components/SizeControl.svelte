<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Note size (Small / Normal / Large; stored as compact / comfortable / large), always in reach at the board's corner.
  // Shortcuts: Ctrl+wheel, Ctrl + / Ctrl − / Ctrl 0. After them, "?": a small menu (F1 opens it)
  // with the quick tour first, the keys, the Archive, What's new, Report a problem, and Help &
  // settings (F1 again opens Help). Beside them, while some notes are archived, the way to the
  // Archive: "Archive (2)", or in a narrow window its icon alone with the count on it.
  import { itemDrag } from "../lib/itemdrag.svelte";
  import Icon from "./Icon.svelte";
  import Menu, { type MenuItem } from "./style/parts/Menu.svelte";
  import { askHelp, askTour, helpMenu } from "../lib/tour.svelte";
  import { store, SIZES, type NoteSize } from "../lib/store.svelte";
  import { t, type Key } from "../lib/i18n.svelte";
  import { rise } from "../lib/motion";

  const names: Record<NoteSize, Key> = { compact: "Small notes", comfortable: "Normal notes", large: "Large notes" };
  const keys: Record<NoteSize, string> = { compact: "Ctrl −", comfortable: "Ctrl 0", large: "Ctrl +" };

  let group = $state<HTMLElement>();
  // The menu sits above the whole group, right-aligned with it (measured each time it opens: the
  // panel may have changed width since).
  let at = $state<DOMRect | null>(null);
  $effect(() => {
    at = helpMenu.open && group ? group.getBoundingClientRect() : null;
  });
  // A press on "?" while its menu is open closes it (the menu's own "click away"); the click
  // that follows mustn't open it again.
  let closedAt = 0;
  function toggle() {
    if (helpMenu.open || performance.now() - closedAt < 300) return void (helpMenu.open = false);
    helpMenu.open = true;
  }
  let archived = $derived(store.archived.length);
  // (A narrow window, as the shell's container query sees it: the Archive's tooltip names it, its
  // words being gone.)
  let narrow = $state(false);
  let cornerEl = $state<HTMLElement>();
  $effect(() => {
    const shell = cornerEl?.closest<HTMLElement>(".shell");
    if (!shell || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => (narrow = shell.clientWidth <= 480));
    ro.observe(shell);
    return () => ro.disconnect();
  });
  let items = $derived<MenuItem[]>([
    { icon: "tour", label: t("Take the tour"), hint: t("1 min"), primary: true, run: askTour },
    { icon: "keyboard", label: t("Keyboard shortcuts"), run: () => askHelp("Keyboard") },
    { icon: "archive", label: t("Archive"), hint: archived ? String(archived) : undefined, run: () => store.openArchive("board") },
    { icon: "sparkle", label: t("What’s new"), sep: true, run: () => store.openWhatsNew("board") },
    { icon: "flag", label: t("Report a problem"), run: () => store.openReport("board") },
    { icon: "help", label: t("Help & settings"), run: () => askHelp("Help") },
  ]);
</script>

<div class="corner" class:hushed={!!itemDrag.item} bind:this={cornerEl}>
{#if archived}
  <button class="to-archive" aria-label={t("Archive ({n})", { n: archived })} data-tip={narrow ? t("Archive ({n})", { n: archived }) : t("Open the archive")} onclick={() => store.openArchive("board")} transition:rise={{ y: 6, duration: 240 }}
    ><Icon name="archive" size={14} stroke={1.8} /><span class="words" aria-hidden="true">{t("Archive ({n})", { n: archived })}</span><span class="count" aria-hidden="true">{archived}</span></button
  >
{/if}
<div class="sizes" class:open={helpMenu.open} role="group" aria-label={t("Note size")} bind:this={group}>
  {#each SIZES as s, i (s)}
    <button
      class="size s{i}"
      class:on={store.size === s}
      aria-pressed={store.size === s}
      aria-label={t(names[s])}
      data-tip={t(names[s])} data-tip-keys="{keys[s].replace(" ", "+")}|Ctrl+Wheel"
      onclick={() => store.setSetting("note_size", s)}>A</button
    >
  {/each}
  <span class="sep" aria-hidden="true"></span>
  <button
    class="size help"
    class:on={helpMenu.open}
    data-tour="help"
    aria-label={t("Help and the quick tour")}
    aria-haspopup="menu"
    aria-expanded={helpMenu.open}
    data-tip={t("Help and the quick tour")}
    data-tip-keys="F1"
    onclick={toggle}><Icon name="help" size={17} stroke={1.7} /></button
  >
</div>
</div>
{#if helpMenu.open && at}
  <Menu
    anchor={at}
    {items}
    label={t("Help and the quick tour")}
    onClose={() => {
      helpMenu.open = false;
      closedAt = performance.now();
    }}
  />
{/if}

<style>
  .corner {
    position: absolute;
    right: 22px;
    bottom: 16px;
    z-index: 15;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: opacity 0.18s;
  }
  .sizes,
  .to-archive {
    border: 1px solid var(--line);
    background: var(--card);
    box-shadow: var(--card-shadow);
    opacity: 0.86;
    transition: opacity 0.18s;
  }
  .sizes {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 10px;
  }
  /* As tall as the sizes beside it (28px buttons in 3px of padding and a 1px edge). */
  .to-archive {
    height: 36px;
    padding: 0 12px 0 10px;
    border-radius: 10px;
    color: var(--muted);
    font-size: var(--t-sm);
    display: flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
    transition: opacity 0.18s, color 0.15s;
  }
  .to-archive:hover,
  .to-archive:focus-visible {
    opacity: 1;
    color: var(--ink);
  }
  .count {
    display: none;
  }
  /* A narrow window: its icon alone, a square as tall as the sizes, the count on its corner (the
     tooltip names it). */
  @container shell (max-width: 480px) {
    .to-archive {
      position: relative;
      width: 36px;
      padding: 0;
      justify-content: center;
    }
    /* (A touch down and left of the middle, its corner clear for the count.) */
    .to-archive :global(svg) {
      width: 16px;
      height: 16px;
      translate: -2px 2px;
    }
    .words {
      display: none;
    }
    .count {
      position: absolute;
      top: 2px;
      right: 2px;
      display: block;
      min-width: 14px;
      height: 14px;
      padding: 0 4px;
      border-radius: 7px;
      background: var(--soft);
      color: var(--card);
      font-size: 10px;
      font-weight: 700;
      line-height: 14px;
      text-align: center;
      font-variant-numeric: tabular-nums;
      box-shadow: 0 0 0 2px var(--card);
    }
  }
  .sizes:hover,
  .sizes:focus-within,
  .sizes.open {
    opacity: 1;
  }
  /* While a note's or a task's menu is open, the sizes step aside: the menu may reach down to
     this corner. */
  /* While a line is carried, the "+ New note" card may reach down to this corner (Board.svelte):
     it steps aside, without moving. */
  .corner.hushed,
  :global(.under:has(.menu-open)) .corner {
    opacity: 0;
    pointer-events: none;
  }
  .size {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--soft);
    font-family: var(--serif);
    font-weight: 600;
    line-height: 28px;
    cursor: pointer;
    transition: background-color 0.18s, color 0.18s;
  }
  .s0 {
    font-size: 12px;
  }
  .s1 {
    font-size: 15px;
  }
  .s2 {
    font-size: 19px;
  }
  .size:hover {
    color: var(--ink);
    background: var(--hover);
  }
  .sep {
    width: 1px;
    height: 16px;
    margin: 0 3px;
    background: var(--line);
  }
  .help {
    display: grid;
    place-items: center;
    color: var(--icon);
  }
  .size.on {
    background: var(--chip);
    color: var(--ink);
  }
</style>
