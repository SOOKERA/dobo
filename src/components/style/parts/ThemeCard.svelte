<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A theme card, split on the diagonal: the Light style above, the Dark style below, a little
  // light and dark note on each. Tap: use it. Tap a half of the one in use: change that half.
  import { composing } from "../../../lib/compose";
  import { focusSelect } from "../../../lib/modal";
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import Menu, { type MenuItem } from "./Menu.svelte";
  import type { Look } from "../../../lib/looks";
  import { t } from "../../../lib/i18n.svelte";

  let {
    name,
    light,
    dark,
    selected = false,
    w = 104,
    h = 58,
    menu,
    onUse,
    onHalf,
    onRename = null,
  }: {
    name: string;
    light: Look;
    dark: Look | null;
    selected?: boolean;
    w?: number;
    h?: number;
    menu: MenuItem[];
    onUse: () => void;
    onHalf: (half: "light" | "dark") => void;
    onRename?: ((name: string) => void) | null;
  } = $props();

  let menuAt = $state<DOMRect | null>(null);
  let renaming = $state(false);
  let draft = $state("");
  let items = $derived(menu.map((m) => (m.id === "rename" ? { ...m, hint: "F2", run: rename } : m)));
  let canRename = $derived(!!onRename && menu.some((m) => m.id === "rename"));
  function rename() {
    draft = name;
    renaming = true;
  }

  function tap(e: MouseEvent) {
    if (!selected) return onUse();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // Below the diagonal: the dark half.
    const x = (e.clientX - r.left) / r.width;
    const y = (e.clientY - r.top) / r.height;
    onHalf(y > 1 - x ? "dark" : "light");
  }
  function done(save: boolean) {
    if (!renaming) return;
    if (save && draft.trim() && draft.trim() !== name) onRename?.(draft.trim());
    renaming = false;
  }
</script>

<div class="th" data-sel={selected}>
  <button class="thumb" aria-label={name} aria-pressed={selected} aria-keyshortcuts={canRename ? "F2" : undefined} data-tip={selected ? t("{name} · tap a half to change it", { name }) : t("Use {name}", { name })} onclick={tap} onkeydown={(e) => e.key === "F2" && canRename && (e.preventDefault(), rename())} data-ring>
    <span class="half"><LookSwatch look={light} {w} {h} /></span>
    <span class="half d"><LookSwatch look={dark ?? light} {w} {h} /></span>
    <span class="mini l"></span>
    <span class="mini dk"></span>
    <span class="ico sun"><Icon name="sun" size={11} stroke={2.2} /></span>
    <span class="ico moon"><Icon name="moon" size={11} stroke={2} /></span>
  </button>
  {#if renaming}
    <input
      class="rename"
      bind:value={draft}
      maxlength="60"
      aria-label={t("New name")}
      use:focusSelect
      onkeydown={(e) => {
        if (composing(e)) return;
        // (The keyboard goes back to the card.)
        const card = (e.currentTarget as HTMLElement).parentElement?.querySelector<HTMLElement>("button.thumb");
        if (e.key === "Enter") done(true);
        if (e.key === "Escape") {
          e.stopPropagation();
          done(false);
        }
        if (e.key === "Enter" || e.key === "Escape") queueMicrotask(() => card?.focus());
      }}
      onblur={() => done(true)}
    />
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span class="name" class:on={selected} data-tip={canRename ? t("{name} · rename", { name }) : name} data-tip-keys={canRename ? "F2|Double-click" : null} ondblclick={() => canRename && rename()}>{name}</span>
  {/if}
  <button class="more" class:open={menuAt} aria-label={t("More for {name}", { name })} data-tip={t("Use, edit, export…")} onclick={(e) => (menuAt = (e.currentTarget as HTMLElement).getBoundingClientRect())}><Icon name="more" size={14} /></button>
  {#if menuAt}
    <Menu {items} anchor={menuAt} onClose={() => (menuAt = null)} />
  {/if}
</div>

<style>
  .th {
    position: relative;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .thumb {
    position: relative;
    display: block;
    height: 58px;
    padding: 0;
    border: 0;
    border-radius: 9px;
    overflow: hidden;
    cursor: pointer;
    background: none;
  }
  .thumb::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 9%, transparent);
    pointer-events: none;
  }
  .half {
    position: absolute;
    inset: 0;
  }
  .half.d {
    clip-path: polygon(100% 0, 100% 100%, 0 100%);
  }
  .mini {
    position: absolute;
    width: 20px;
    height: 14px;
    border-radius: 3px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
  }
  .mini.l {
    left: 26px;
    top: 10px;
    background: #fffefb;
  }
  .mini.dk {
    right: 26px;
    bottom: 10px;
    background: #2a292f;
  }
  .ico {
    position: absolute;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: grid;
    place-items: center;
  }
  .sun {
    left: 4px;
    top: 4px;
    background: rgba(255, 254, 251, 0.92);
    color: #1f1d1a;
  }
  .moon {
    right: 4px;
    bottom: 4px;
    background: rgba(36, 37, 42, 0.92);
    color: #ece8e0;
  }
  .name {
    font-size: 11.5px;
    line-height: 15px;
    color: var(--soft);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .name.on {
    font-weight: 600;
    color: var(--ink);
  }
  .rename {
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--ink);
    border-radius: 7px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: 12px;
    min-width: 0;
  }
  .more {
    position: absolute;
    right: 4px;
    top: 4px;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 254, 251, 0.94);
    color: #1f1d1a;
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    opacity: 0;
    transition: opacity 0.15s;
  }
  .th:hover .more,
  .th:focus-within .more,
  .more.open {
    opacity: 1;
  }
</style>
