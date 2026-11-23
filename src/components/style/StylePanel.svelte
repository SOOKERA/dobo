<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Style panel: slides in at the side of the board, every change shows on the board at
  // once (Undo works). More mixes, Customise, Themes, Edit theme and Export & import slide in
  // over it inside the same panel. ‹ Back goes up one level (the page reverses its entrance);
  // × and Esc close the whole panel the way it opened. The header says where you are
  // ("Style › Themes › Edit theme"); an earlier level jumps straight back to it.
  import { tick, untrack } from "svelte";
  import ViewHeader, { type Crumb } from "../ViewHeader.svelte";
  import MainPage from "./pages/MainPage.svelte";
  import MoreMixes from "./pages/MoreMixes.svelte";
  import Customise from "./pages/Customise.svelte";
  import Themes from "./pages/Themes.svelte";
  import EditTheme from "./pages/EditTheme.svelte";
  import Share from "./pages/Share.svelte";
  import { look, type PanelPage } from "../../lib/look.svelte";
  import { pushLayer } from "../../lib/layers";
  import { modal, tabbables } from "../../lib/modal";
  import { isWindowDrag } from "../../lib/windrag";
  import { pageIn, pageOut, side } from "../../lib/motion";
  import { follow, forward, goTo, pagesAt, upTo, type Pages } from "../../lib/nav";
  import { t, type Key } from "../../lib/i18n.svelte";

  let { onClose }: { onClose: () => void } = $props();

  const PARENT: Record<PanelPage, PanelPage | null> = { main: null, more: "main", customise: "main", themes: "main", "edit-theme": "themes", share: "themes" };
  /** The way up from a page: its parents (main first). */
  function lineage(p: PanelPage): PanelPage[] {
    const up: PanelPage[] = [];
    for (let q = PARENT[p]; q; q = PARENT[q]) up.unshift(q);
    return up;
  }

  let back = $state(false);
  let panelEl = $state<HTMLElement>();
  /** Where you are, the way you came, and the pages just backed out of (for Forward). */
  let nav = $state<Pages<PanelPage>>(pagesAt(look.page, lineage(look.page)));
  let moving = false;
  $effect.pre(() => {
    const p = look.page;
    untrack(() => {
      if (moving || p === nav.here) return void (moving = false);
      // Changed from elsewhere (an import): up if the page was above us, else down.
      const f = follow(nav, p, lineage);
      back = f.up;
      nav = f.pages;
    });
  });
  function show(n: Pages<PanelPage>, up: boolean) {
    // The keyboard follows (when it is in the panel): down to the new page's first control; back
    // up to the control that opened the page left.
    const a = document.activeElement as HTMLElement | null;
    const keys = !!a && (a === document.body || !!panelEl?.contains(a));
    if (!up && pagesEl && a) {
      const i = tabbables(pagesEl).indexOf(a);
      if (i >= 0) opener.set(nav.here, i);
    }
    moving = n.here !== look.page;
    back = up;
    nav = n;
    look.page = n.here;
    if (keys) void tick().then(() => land(up ? (opener.get(n.here) ?? 0) : 0, true));
  }
  /** Each page's control that opened the page below it (its place among the page's controls). */
  const opener = new Map<PanelPage, number>();
  let pagesEl = $state<HTMLElement>();
  function land(i: number, again: boolean) {
    // (The page leaving is inert: only the new one's controls count.)
    const list = pagesEl ? tabbables(pagesEl) : [];
    if (!list.length && again) return void requestAnimationFrame(() => land(i, false));
    const to = list[Math.min(i, list.length - 1)] ?? panelEl?.querySelector<HTMLElement>(".head .back");
    to?.focus({ preventScroll: true });
  }

  const go = (p: PanelPage) => show(goTo(nav, p), false);
  /** Up to the level at index i of the path (one level = the last one); above the home page,
   * the panel closes. */
  function upToLevel(i: number) {
    const n = upTo(nav, i);
    if (n) show(n, true);
    else onClose();
  }
  const goBack = () => upToLevel(nav.trail.length - 1);
  function goForward() {
    const n = forward(nav);
    if (n) show(n, false);
  }

  // Esc closes the whole panel (a menu open inside it closes first: it is its own layer); the
  // mouse's Back / Forward, Alt+Left / Alt+Right and Backspace step through the pages.
  $effect(() => pushLayer(onClose, { back: goBack, forward: goForward }));

  // A click on the board, away from the panel, closes it (what you changed stays: it's live,
  // and Undo still works). Not a drag, not a right-click, not a click in something of the
  // panel's (its menus) or in what opened it (the Style button, "Change background…"), not the
  // Undo toast, not the window's own edges, not a press that moves the window (its top, floating).
  // The header's sun/moon keeps the panel open: flip light/dark to compare the style in both.
  const KEEP = ".panel, .style-btn, .appear,.ctx, .ctx-veil, .toast, .menu, [role='menu'], [role='dialog'], .dock-grip, .grip";
  $effect(() => {
    let start: { x: number; y: number; t: number } | null = null;
    const away = (t: EventTarget | null) => t instanceof Element && t.isConnected && !t.closest(KEEP);
    const down = (e: PointerEvent) => {
      start = e.button === 0 && away(e.target) && !isWindowDrag(e) ? { x: e.clientX, y: e.clientY, t: e.timeStamp } : null;
    };
    const up = (e: PointerEvent) => {
      const s = start;
      start = null;
      // A press held long (press-and-hold to set a time) or moved (a drag) isn't a click away.
      if (!s || e.button !== 0 || e.timeStamp - s.t > 450 || Math.hypot(e.clientX - s.x, e.clientY - s.y) > 6) return;
      if (away(e.target)) onClose();
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointerup", up, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointerup", up, true);
    };
  });

  const NAMES: Record<PanelPage, Key> = { main: "Style", more: "More mixes", customise: "Customise", themes: "Themes", "edit-theme": "Edit theme", share: "Export & import" };
  const SUBS: Partial<Record<PanelPage, Key>> = { more: "Watercolour", customise: "Watercolour", themes: "A Light style and a Dark style, kept together" };
  let crumbs = $derived<Crumb[]>([...nav.trail.map((p, i) => ({ label: t(NAMES[p]), go: () => upToLevel(i) })), { label: t(NAMES[look.page]) }]);
  let sub = $derived.by(() => {
    if (look.page === "edit-theme") return look.themeNameOf(look.board.theme) ?? t("This board");
    const s = SUBS[look.page];
    return s ? t(s) : "";
  });
</script>

<!-- (Beside the board, which stays as it is: the keyboard comes in, and Tab may go on out.) -->
<aside class="panel" aria-label={t("Style")} tabindex="-1" bind:this={panelEl} transition:side|global use:modal={{ initial: "self", trap: false }}>
  <div class="head">
    <ViewHeader {crumbs} {sub} onBack={goBack} {onClose} />
  </div>
  <div class="pages" bind:this={pagesEl}>
    {#key look.page}
      <div class="page" in:pageIn={{ back }} out:pageOut={{ back }}>
        {#if look.page === "main"}
          <MainPage {go} />
        {:else if look.page === "more"}
          <MoreMixes {go} />
        {:else if look.page === "customise"}
          <Customise />
        {:else if look.page === "themes"}
          <Themes {go} />
        {:else if look.page === "edit-theme"}
          <EditTheme />
        {:else}
          <Share />
        {/if}
      </div>
    {/key}
  </div>
</aside>

<style>
  .panel {
    position: absolute;
    right: 12px;
    top: calc(var(--head-h, 60px) + 8px);
    bottom: 12px;
    z-index: 46;
    width: min(372px, calc(100% - 24px));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 16px;
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 24px 60px rgba(30, 20, 10, 0.3), 0 0 0 1px color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .head {
    flex: none;
    padding: 12px 12px 8px 20px;
    min-height: 62px;
  }
  .pages {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
  }
  /* Shared by the pages. */
  .panel :global(.body) {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 2px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    scrollbar-gutter: stable;
  }
  .panel :global(.sec) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .panel :global(.row-h) {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    min-height: 22px;
  }
  .panel :global(.lbl) {
    font-size: 12px;
    font-weight: 600;
    color: var(--soft);
  }
  .panel :global(.muted) {
    font-size: 12px;
    color: var(--muted);
  }
  .panel :global(.grid3) {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    align-items: start;
  }
  .panel :global(.glide-ring) {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--card), 0 0 0 4px var(--ink);
    transition: transform 0.3s var(--ease-glide), opacity 0.2s;
    z-index: 2;
  }
  .panel :global(.foot) {
    flex: none;
    padding: 12px 16px 14px;
    border-top: 1px solid var(--line);
    background: color-mix(in srgb, var(--card) 70%, var(--ground));
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .panel :global(.btn) {
    height: 36px;
    padding: 0 14px;
    border: 0;
    border-radius: 9px;
    background: var(--ink);
    color: var(--card);
    font-size: 13px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    cursor: pointer;
    white-space: nowrap;
    flex: none;
  }
  .panel :global(.btn:disabled) {
    opacity: 0.4;
    cursor: default;
  }
  .panel :global(.btn2) {
    height: 34px;
    padding: 0 13px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--card);
    color: var(--ink);
    font-size: 12.5px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
    flex: none;
    transition: background-color 0.15s;
  }
  .panel :global(.btn2:hover) {
    background: var(--hover);
  }
  .panel :global(.link) {
    padding: 0;
    border: 0;
    background: none;
    font-size: 12px;
    color: var(--soft);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .panel :global(.link:hover) {
    color: var(--ink);
  }
  .panel :global(.name-in) {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: 13px;
  }
  .panel :global(.name-in:focus) {
    outline: none;
    border-color: var(--ink);
  }
  .panel :global(.go) {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    border: 0;
    border-radius: 12px;
    background: color-mix(in srgb, var(--chip) 70%, var(--card));
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .panel :global(.go:hover) {
    background: var(--chip);
  }
  .panel :global(.go b) {
    display: block;
    font-size: 13.5px;
    font-weight: 600;
  }
  .panel :global(.go .sub) {
    font-size: 12px;
    color: var(--muted);
  }
  .panel :global(.tip) {
    font-size: 12px;
    line-height: 1.45;
    color: var(--muted);
    background: color-mix(in srgb, var(--chip) 55%, var(--card));
    border-radius: 10px;
    padding: 8px 12px;
  }
  /* (It takes the keyboard itself when it opens: no ring round the whole view.) */
  .panel:focus {
    outline: none;
  }
</style>
