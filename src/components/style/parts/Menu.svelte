<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script module lang="ts">
  export interface MenuItem {
    /** What it does, for the card that shows the menu ("rename" gets F2 and double-click). */
    id?: string;
    icon: string;
    label: string;
    run: () => void;
    danger?: boolean;
    /** Shown muted at the right: a shortcut, a count, or "On" / "Off". */
    hint?: string;
    /** The one to start with: shown a touch stronger (the "?" menu's "Take the tour"). */
    primary?: boolean;
    /** A thin line above it: the start of another group. */
    sep?: boolean;
  }
</script>

<script lang="ts">
  import { pop } from "../../../lib/motion";
  // The small ⋯ menu of a card (Use, Edit, Rename, Duplicate, Export, Delete…), of the header, and
  // the "?" at the board's corner. Delete asks once. Under its anchor, or above it when there is
  // no room below; right-aligned with it. Arrows, Home / End move; Esc (or Back) closes and the
  // keyboard goes back where it was.
  import Icon from "../../Icon.svelte";
  import { pushLayer } from "../../../lib/layers";
  import { takeFocus } from "../../../lib/modal";
  import { t } from "../../../lib/i18n.svelte";
  import { tick } from "svelte";
  type Item = MenuItem;
  let { items, anchor, onClose, label }: { items: Item[]; anchor: DOMRect; onClose: () => void; label?: string } = $props();
  let el: HTMLDivElement;
  let confirming = $state<Item | null>(null);

  // Closing (by Esc, Back, a press elsewhere or a choice): while it fades out it takes no more
  // keys or clicks.
  let done = false;
  let back = false;
  let was: HTMLElement | null = null;
  function close(keyboardBack: boolean) {
    if (done) return;
    done = true;
    back = keyboardBack;
    onClose();
  }
  $effect(() => {
    const away = (e: PointerEvent) => {
      if (!el.contains(e.target as Node)) close(false);
    };
    // Closed by Esc or Back, or by a choice that left the keyboard nowhere (an On / Off item): the
    // keyboard goes back to where it was (the button that opened it).
    was = document.activeElement as HTMLElement | null;
    const pop = pushLayer(() => close(true));
    setTimeout(() => window.addEventListener("pointerdown", away, true));
    el.querySelector<HTMLElement>("button")?.focus();
    return () => {
      pop();
      window.removeEventListener("pointerdown", away, true);
      const a = document.activeElement;
      if (back && was?.isConnected && (!a || a === document.body || el.contains(a))) was.focus({ preventScroll: true });
    };
  });

  // Placed in the window's own frame (the panel's shell moves, so "fixed" is relative to it).
  // Right-aligned with its anchor (or at the left edge when the anchor is too far left), as wide
  // as its longest line needs (200 px at least). Under the anchor, or above it when there is no
  // room below: then it hangs from its bottom edge, so it stays on the anchor whatever its height
  // (Delete's question is shorter than the list).
  const host = document.querySelector(".shell")?.getBoundingClientRect();
  const frame = host ? { left: host.left, top: host.top, width: host.width, height: host.height } : { left: 0, top: 0, width: window.innerWidth, height: window.innerHeight };
  let fromLeft = $derived(anchor.right - frame.left < 208);
  let right = $derived(Math.max(8, frame.width - (anchor.right - frame.left)));
  // Its height, known before it shows: 6 px padding, 34 px a line, 9 px a separator.
  let height = $derived(12 + items.length * 34 + items.filter((it, i) => i > 0 && (it.sep || it.danger)).length * 9);
  let below = $derived(frame.height - (anchor.bottom - frame.top) - 6);
  let above = $derived(anchor.top - frame.top - 6);
  let up = $derived(below < height + 8 && above > below);
  let top = $derived(anchor.bottom - frame.top + 6);
  let bottom = $derived(frame.height - (anchor.top - frame.top) + 6);

  async function keep() {
    if (done || !confirming) return;
    const asked = confirming.label;
    confirming = null;
    await tick();
    [...el.querySelectorAll<HTMLElement>("[role=menuitem]")].find((b) => b.querySelector(".label")?.textContent === asked)?.focus();
  }

  function keys(e: KeyboardEvent) {
    if (done) return e.preventDefault();
    // Tab leaves the menu, as from the button that opened it: the menu closes and Tab goes on
    // to what follows that button.
    if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (was?.isConnected && was !== document.body) was.focus({ preventScroll: true });
      close(false);
      return;
    }
    const list = [...el.querySelectorAll<HTMLElement>("button")];
    const i = list.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") list[(i + 1) % list.length]?.focus();
    else if (e.key === "ArrowUp") list[(i - 1 + list.length) % list.length]?.focus();
    else if (e.key === "Home") list[0]?.focus();
    else if (e.key === "End") list[list.length - 1]?.focus();
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="menu"
  class:up
  class:from-left={fromLeft}
  role="menu"
  aria-label={label}
  transition:pop|global={{ duration: 160, y: up ? 4 : -4 }}
  tabindex="-1"
  bind:this={el}
  style:left={fromLeft ? "8px" : null}
  style:right={fromLeft ? null : `${right}px`}
  style:top={up ? null : `${top}px`}
  style:bottom={up ? `${bottom}px` : null}
  style:max-height="{Math.max(up ? above : below, 120) - 8}px"
  onkeydown={keys}
>
  {#if confirming}
    <p class="ask">{t("{action}? Undo can bring it back.", { action: confirming.label.replace("…", "") })}</p>
    <div class="row">
      <!-- (The keyboard comes to the safe choice; Keep gives it back to the line that asked.) -->
      <button class="plain" use:takeFocus onclick={keep}>{t("Keep")}</button>
      <button
        class="del"
        onclick={() => {
          if (done) return;
          confirming?.run();
          close(true);
        }}>{t("Delete")}</button
      >
    </div>
  {:else}
    {#each items as it, i (it.label)}
      {#if (it.danger || it.sep) && i > 0}<hr />{/if}
      <button
        role="menuitem"
        class:danger={it.danger}
        class:primary={it.primary}
        onclick={() => {
          if (done) return;
          if (it.danger) confirming = it;
          else {
            close(true);
            it.run();
          }
        }}><Icon name={it.icon} size={15} /><span class="label">{it.label}</span>{#if it.hint}<span class="hint">{it.hint}</span>{/if}</button
      >
    {/each}
  {/if}
</div>

<style>
  .menu {
    transform-origin: 100% 0;
    position: fixed;
    z-index: 90;
    width: max-content;
    min-width: 200px;
    max-width: min(300px, calc(100% - 16px));
    overflow-y: auto;
    padding: 6px;
    border-radius: var(--r-lg);
    background: var(--card);
    box-shadow: var(--pop-shadow), 0 0 0 1px color-mix(in srgb, var(--ink) 6%, transparent);
    display: flex;
    flex-direction: column;
  }
  [role="menuitem"] {
    flex-shrink: 0;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--ink);
    font-size: var(--t-md);
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    cursor: pointer;
  }
  [role="menuitem"]:hover,
  [role="menuitem"]:focus-visible {
    background: var(--hover);
    outline: none;
  }
  /* (The keyboard's place: the ring inside the row, as on the board.) */
  [role="menuitem"]:focus-visible {
    box-shadow: inset 0 0 0 2px var(--focus-ring);
  }
  .menu.up {
    transform-origin: 100% 100%;
  }
  .menu.from-left {
    transform-origin: 0 0;
  }
  .menu.from-left.up {
    transform-origin: 0 100%;
  }
  [role="menuitem"] :global(svg) {
    flex-shrink: 0;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The one to start with: a soft tint of the accent, its icon in the accent. */
  .primary {
    font-weight: 600;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .primary :global(svg) {
    color: var(--accent);
  }
  .primary .hint {
    color: var(--soft);
  }
  [role="menuitem"].primary:hover,
  [role="menuitem"].primary:focus-visible {
    background: color-mix(in srgb, var(--accent) 17%, transparent);
  }
  .hint {
    font-weight: 400;
    margin-left: auto;
    padding-left: 6px;
    flex-shrink: 0;
    white-space: nowrap;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .danger {
    color: var(--late-ink);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 4px 6px;
  }
  .ask {
    margin: 6px 8px 10px;
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--soft);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    padding: 0 2px 2px;
  }
  .row button {
    height: 30px;
    padding: 0 12px;
    border-radius: 8px;
    border: 0;
    font-size: var(--t-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .plain {
    background: var(--chip);
    color: var(--ink);
  }
  .del {
    background: var(--danger-fill);
    color: #fff;
  }
  @media (prefers-reduced-motion: reduce) {
    .menu {
      animation: none;
    }
  }
</style>
