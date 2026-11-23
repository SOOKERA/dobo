<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A style card: the real background with a little note on it, and its name. Ours fit one
  // line; yours get up to two lines (the full name on hover) and a ⋯ menu.
  import { composing } from "../../../lib/compose";
  import { focusSelect } from "../../../lib/modal";
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import Menu, { type MenuItem } from "./Menu.svelte";
  import type { Look } from "../../../lib/looks";
  import { t } from "../../../lib/i18n.svelte";

  let {
    name,
    look: l,
    selected = false,
    yours = false,
    w = 100,
    h = 34,
    menu = null,
    onPick,
    onRename = null,
  }: {
    name: string;
    look: Look;
    selected?: boolean;
    yours?: boolean;
    w?: number;
    h?: number;
    menu?: MenuItem[] | null;
    onPick: () => void;
    onRename?: ((name: string) => void) | null;
  } = $props();

  let menuAt = $state<DOMRect | null>(null);
  let renaming = $state(false);
  let draft = $state("");

  function done(save: boolean) {
    if (!renaming) return;
    if (save && draft.trim() && draft.trim() !== name) onRename?.(draft.trim());
    renaming = false;
  }
  let items = $derived(menu?.map((m) => (m.id === "rename" ? { ...m, hint: "F2", run: rename } : m)) ?? null);
  let canRename = $derived(!!menu?.some((m) => m.id === "rename"));
  function rename() {
    draft = name;
    renaming = true;
  }
</script>

<div class="card" class:yours data-sel={selected}>
  <button
    class="pick"
    aria-pressed={selected}
    data-tip={canRename ? t("{name} · rename", { name }) : name} data-tip-keys={canRename ? "F2|Double-click" : null}
    onclick={onPick}
    ondblclick={() => canRename && rename()}
    onkeydown={(e) => e.key === "F2" && canRename && (e.preventDefault(), rename())}
  >
    <span class="art"><LookSwatch look={l} {w} {h} card={{ left: 9, top: 7, w: 26, h: 20 }} /></span>
    <span class="name" class:on={selected} class:hide={renaming}>{name}</span>
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
        const card = (e.currentTarget as HTMLElement).parentElement?.querySelector<HTMLElement>("button.pick");
        if (e.key === "Enter") done(true);
        if (e.key === "Escape") {
          e.stopPropagation();
          done(false);
        }
        if (e.key === "Enter" || e.key === "Escape") queueMicrotask(() => card?.focus());
      }}
      onblur={() => done(true)}
    />
  {/if}
  {#if items}
    <button class="more" class:open={menuAt} aria-label={t("More for {name}", { name })} data-tip={t("Rename, duplicate, delete")} onclick={(e) => (menuAt = (e.currentTarget as HTMLElement).getBoundingClientRect())}><Icon name="more" size={14} /></button>
  {/if}
  {#if menuAt && items}
    <Menu {items} anchor={menuAt} onClose={() => (menuAt = null)} />
  {/if}
</div>

<style>
  .card {
    position: relative;
    min-width: 0;
  }
  .pick {
    width: 100%;
    padding: 0;
    border: 0;
    border-radius: 9px;
    background: var(--card);
    color: var(--ink);
    display: flex;
    flex-direction: column;
    text-align: left;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 9%, transparent);
    overflow: hidden;
    transition: transform 0.15s var(--ease-out);
  }
  .pick:hover {
    transform: translateY(-1px);
  }
  .art {
    display: block;
    border-radius: 9px 9px 0 0;
    overflow: hidden;
  }
  .name {
    padding: 5px 8px 6px;
    font-size: 11.5px;
    line-height: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .yours .name {
    white-space: normal;
    min-height: 30px;
    box-sizing: content-box;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    padding-right: 24px;
  }
  .name.on {
    font-weight: 600;
  }
  .name.hide {
    visibility: hidden;
  }
  .rename {
    position: absolute;
    left: 3px;
    right: 3px;
    bottom: 4px;
    height: 28px;
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
    right: 3px;
    bottom: 5px;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background-color 0.15s;
  }
  .card:hover .more,
  .card:focus-within .more,
  .more.open,
  [data-sel="true"] .more {
    opacity: 1;
  }
  .more:hover,
  .more.open {
    background: var(--hover);
    color: var(--ink);
  }
</style>
