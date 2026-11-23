<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // One of your watercolour mixes (the "Watercolour · yours" row): the real painting, its name on
  // up to two lines, a ⋯ menu to use, rename or delete it.
  import { composing } from "../../../lib/compose";
  import { focusSelect } from "../../../lib/modal";
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import Menu from "./Menu.svelte";
  import { look } from "../../../lib/look.svelte";
  import type { SavedMix } from "../../../lib/looks";
  import { deal } from "../../../lib/motion";
  import { t } from "../../../lib/i18n.svelte";

  let { mix: m, selected, i }: { mix: SavedMix; selected: boolean; i: number } = $props();

  let menuAt = $state<DOMRect | null>(null);
  let renaming = $state(false);
  let draft = $state("");
  const use = () => look.pickMix(m.colours, m.name, m.pattern, m.paper);
  function rename() {
    draft = m.name;
    renaming = true;
  }
  function done(save: boolean) {
    if (!renaming) return;
    if (save && draft.trim() && draft.trim() !== m.name) void look.renameMix(m.id, draft.trim());
    renaming = false;
  }
</script>

<div class="cell" data-sel={selected} in:deal={{ i }}>
  <button
    class="pick"
    aria-pressed={selected}
    data-tip={t("{name} · rename", { name: m.name })} data-tip-keys="F2|Double-click"
    onclick={use}
    ondblclick={rename}
    onkeydown={(e) => e.key === "F2" && (e.preventDefault(), rename())}
  >
    <span class="art" data-ring><LookSwatch look={{ bg: "watercolour", colour: "", held: "auto", wc: { ...look.wc, colours: m.colours, pattern: m.pattern, paper: m.paper } }} w={58} h={46} /></span>
    {#if !renaming}<span class="n" class:on={selected}>{m.name}</span>{/if}
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
  <button class="more" class:open={menuAt} aria-label={t("More for {name}", { name: m.name })} data-tip={t("Rename or delete")} onclick={(e) => (menuAt = (e.currentTarget as HTMLElement).getBoundingClientRect())}><Icon name="more" size={13} /></button>
  {#if menuAt}
    <Menu
      anchor={menuAt}
      onClose={() => (menuAt = null)}
      items={[
        { icon: "check", label: t("Use this mix"), run: use },
        { id: "rename", icon: "lines", label: t("Rename"), hint: "F2", run: rename },
        { icon: "trash", label: t("Delete…"), danger: true, run: () => look.deleteMix(m.id) },
      ]}
    />
  {/if}
</div>

<style>
  .cell {
    position: relative;
    min-width: 0;
  }
  .pick {
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    line-height: 1.25;
    text-align: center;
    cursor: pointer;
  }
  .art {
    display: block;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .n {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .n.on {
    font-weight: 600;
    color: var(--ink);
  }
  .rename {
    width: 100%;
    height: 24px;
    margin-top: 4px;
    padding: 0 4px;
    border: 1px solid var(--ink);
    border-radius: 6px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: 11px;
  }
  .more {
    position: absolute;
    right: 3px;
    top: 3px;
    width: 20px;
    height: 20px;
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
  .cell:hover .more,
  .cell:focus-within .more,
  .more.open {
    opacity: 1;
  }
</style>
