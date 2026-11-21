<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Note size (Small / Normal / Large; stored as compact / comfortable / large), always in reach at the board's corner.
  // Shortcuts: Ctrl+wheel, Ctrl + / Ctrl − / Ctrl 0. After them, "?": a small menu (F1 opens it)
  // with the quick tour first, the keys, the Archive, and Help & settings (F1 again opens Help).
  import Icon from "./Icon.svelte";
  import Menu, { type MenuItem } from "./style/parts/Menu.svelte";
  import { askHelp, askTour, helpMenu } from "../lib/tour.svelte";
  import { store, SIZES, type NoteSize } from "../lib/store.svelte";
  import { t, type Key } from "../lib/i18n.svelte";

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
  let items = $derived<MenuItem[]>([
    { icon: "tour", label: t("Take the tour"), hint: t("1 min"), primary: true, run: askTour },
    { icon: "keyboard", label: t("Keyboard shortcuts"), run: () => askHelp("Keyboard") },
    { icon: "archive", label: t("Archive"), hint: archived ? String(archived) : undefined, run: () => store.openArchive("board") },
    { icon: "help", label: t("Help & settings"), sep: true, run: () => askHelp("Help") },
  ]);
</script>

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
  .sizes {
    position: absolute;
    right: 22px;
    bottom: 16px;
    z-index: 15;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 10px;
    background: var(--card);
    border: 1px solid var(--line);
    box-shadow: var(--card-shadow);
    opacity: 0.86;
    transition: opacity 0.18s;
  }
  .sizes:hover,
  .sizes:focus-within,
  .sizes.open {
    opacity: 1;
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
