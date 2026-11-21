<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import Logo from "./Logo.svelte";
  import type { SpaceView } from "../lib/types";

  import { look } from "../lib/look.svelte";
  import { askTour, styleButton } from "../lib/tour.svelte";
  import Menu, { type MenuItem } from "./style/parts/Menu.svelte";
  import { i18n, t } from "../lib/i18n.svelte";
  import { flushSync, untrack } from "svelte";
  import { blip, easeHeight, fadeQuick, GLIDE, reduced } from "../lib/motion";

  let { onHide, onSettings, onSearch, onTidy, onStyle }: { onHide: () => void; onSettings: () => void; onSearch: () => void; onTidy: () => void; onStyle: () => void } = $props();
  let waiting = $derived(store.inboxWaiting.length);

  let views = $derived<{ key: SpaceView; label: string }[]>([
    { key: "all", label: t("All") },
    { key: "work", label: t("Work") },
    { key: "personal", label: t("Personal") },
  ]);
  let floating = $derived(store.win.mode === "floating");
  // The logo (the Dobo note and the hand-lettered name) always shows whole: the buttons give way.
  // Steps, from the roomiest: 0 one row; 1 one row, the Style button keeps only its brush and
  // Look back goes into the ⋯ menu (one row holds down to ~760 px docked); 2 two rows (All / Work
  // / Personal and the sun/moon go below, Look back comes back out); then, a step at a time,
  // buttons go into the ⋯ menu: 3 Look back, Settings and Always on top, 4 Style, 5 New note.
  // (The ⋯ menu is always there: the Archive and the quick tour live in it.)
  // What fits is measured (the bar's real width, the logo's real width), not guessed from the
  // window's width.
  /** A little air kept after the logo, so it never touches what follows it. */
  const LOGO_AIR = 6;
  let step = $state(0);
  let tight = $derived(step === 1);
  let narrow = $derived(step >= 2);
  let compact = $derived(step >= 3);
  let styleIn = $derived(step >= 4);
  let newIn = $derived(step >= 5);
  // (The quick tour's Style step points at the brush, or at ⋯, and says so.)
  $effect(() => {
    styleButton.shown = styleIn ? "menu" : tight || narrow ? "brush" : "words";
  });
  let moreAt = $state<DOMRect | null>(null);
  // A press on ⋯ while its menu is open closes it (the menu's own "click away"); the click that
  // follows mustn't open it again.
  let moreClosed = 0;
  function toggleMore(e: MouseEvent) {
    if (moreAt || performance.now() - moreClosed < 300) return void (moreAt = null);
    moreAt = (e.currentTarget as HTMLElement).getBoundingClientRect();
  }
  let archived = $derived(store.archived.length);
  // What the narrower steps tuck away, first in the menu.
  // Look back opens and closes from the same button (it shows "on" while its card is there, also
  // when the evening brought it by itself).
  let lookOn = $derived(!!store.lookback);
  function toggleLookBack() {
    if (store.lookback) void store.dismissLookback();
    else store.lookbackOpen = true;
  }
  let lookBack = $derived<MenuItem>({ icon: "sunset", label: lookOn ? t("Close the look back") : t("Look back on today"), run: toggleLookBack });
  let tucked = $derived<MenuItem[]>(
    compact
      ? [
          ...(styleIn ? [{ icon: "brush", label: t("Style"), hint: "Ctrl+Shift+S", run: onStyle }] : []),
          ...(newIn ? [{ icon: "note-plus", label: t("New note"), run: () => (store.templatesOpen = true) }] : []),
          lookBack,
          { icon: "settings", label: t("Settings"), hint: "Ctrl+Shift+P", run: onSettings },
          ...(floating ? [{ icon: "ontop", label: t("Always on top"), hint: store.win.float_on_top ? t("On") : t("Off"), run: () => store.setFloatOnTop(!store.win.float_on_top) }] : []),
        ]
      : tight
        ? [lookBack]
        : [],
  );
  let moreItems = $derived<MenuItem[]>([
    ...tucked,
    { icon: "archive", label: t("Archive"), hint: archived ? String(archived) : undefined, sep: tucked.length > 0, run: () => store.openArchive("board") },
    { icon: "tour", label: t("Take the tour"), run: askTour },
  ]);
  let moreLabel = $derived(t("More: {list}", { list: moreItems.map((i) => i.label).join(", ") }));
  // The header's height (one row or two), for what opens just under it (Style panel, search).
  let barH = $state(60);
  $effect(() => {
    document.documentElement.style.setProperty("--head-h", `${barH}px`);
  });

  let barEl = $state<HTMLElement>();
  let brandEl = $state<HTMLElement>();
  let nameEl = $state<HTMLElement>();
  let toolsEl = $state<HTMLElement>();
  /** Room left beside the logo (negative: it would be cut). In two rows the buttons could wrap
   * under it, so the room is what the row leaves them. */
  function slack(): number {
    if (!barEl || !brandEl || !nameEl || !toolsEl) return 0;
    const need = nameEl.getBoundingClientRect().width + LOGO_AIR;
    if (step < 2) return brandEl.clientWidth - need;
    const cs = getComputedStyle(barEl);
    const inner = barEl.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
    return inner - parseFloat(cs.columnGap) - toolsEl.getBoundingClientRect().width - need;
  }
  // A one-row step was tried at this bar width (or more) and didn't fit: not tried again below
  // it until what sits in the row changes (the language, floating or docked).
  let failsAt = [0, 0];
  let shown = false;

  /** Picks the layout that keeps the logo whole with the fewest buttons tucked away. Runs when
   * the bar's width changes, before that frame is painted; the pieces glide to their new places
   * (the bar's height eases in its box) instead of jumping. */
  function fit() {
    if (!barEl || !toolsEl) return;
    const before = new Map([...barEl.children].map((el) => [el, el.getBoundingClientRect()]));
    const was = step;
    const w = barEl.clientWidth;
    const set = (k: number) => flushSync(() => (step = k));
    // A roomier one-row step, if this width may hold it.
    for (const k of [0, 1]) {
      if (k >= was || w <= failsAt[k]) continue;
      set(k);
      if (slack() >= 0) break;
      failsAt[k] = w;
      set(was);
    }
    // Too tight: the next step, until the logo has its room.
    while (step <= 1 && slack() < 0) {
      failsAt[step] = Math.max(failsAt[step], w);
      set(step + 1);
    }
    if (step >= 2) {
      // Two rows: what each step saves, from the buttons' real sizes.
      const tool = toolsEl.querySelector<HTMLElement>(".tool.find");
      const cost = (tool?.offsetWidth ?? 34) + 2;
      const style = toolsEl.querySelector<HTMLElement>(".style-btn");
      const styleCost = style ? style.offsetWidth + parseFloat(getComputedStyle(style).marginRight) + 2 : cost + 4;
      // (Step 3 tucks Look back and Settings, and Always on top when floating; ⋯ is always there.)
      const first = (floating ? 3 : 2) * cost;
      const saved = [0, first, first + styleCost, first + styleCost + cost];
      const room = slack();
      const at = step - 2;
      let lv = 0;
      while (lv < 3 && room + saved[lv] - saved[at] < 0) lv++;
      if (lv !== at) set(2 + lv);
      // (The sums are exact; this only guards against a rounding pixel.)
      while (step < 5 && slack() < -0.5) set(step + 1);
    }
    if (step === was || !shown || reduced()) return;
    for (const el of barEl.children) {
      const from = before.get(el);
      if (!from) continue;
      const to = el.getBoundingClientRect();
      const dx = from.left - to.left;
      const dy = from.top - to.top;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) continue;
      el.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: 320, easing: GLIDE });
    }
  }
  $effect(() => {
    if (!barEl) return;
    const ro = new ResizeObserver(() => {
      fit();
      shown = true;
    });
    ro.observe(barEl);
    // The buttons' font arriving after the first fit changes their width, not the bar's.
    const fonts = document.fonts;
    const refit = () => {
      failsAt = [0, 0];
      fit();
    };
    let live = true;
    void fonts?.ready.then(() => live && refit());
    fonts?.addEventListener("loadingdone", refit);
    return () => {
      live = false;
      ro.disconnect();
      fonts?.removeEventListener("loadingdone", refit);
    };
  });
  // What sits in the row changed: one row may fit (or not) at another width now.
  $effect(() => {
    void i18n.lang;
    void floating;
    untrack(() => {
      failsAt = [0, 0];
      queueMicrotask(fit);
    });
  });

  // ---- Light / dark, one click (Settings › Appearance has the rest) ----
  let dm = $derived(look.darkMode);
  // "Switch to light (now: dark, like Windows)".
  let appearTitle = $derived.by(() => {
    const now = look.dark ? t("dark") : t("light");
    const state = dm.mode === "windows" ? t("{state}, like Windows", { state: now }) : dm.mode === "hours" ? t("{state}, by the hour", { state: now }) : now;
    return t(look.dark ? "Switch to light (now: {state})" : "Switch to dark (now: {state})", { state });
  });
  /** Shows the opposite of what's showing now and keeps it (Light or Dark). */
  function flip() {
    void look.setDarkMode({ ...dm, mode: look.dark ? "light" : "dark" });
  }
</script>

<!-- (Floating, the header is the window's drag area: lib/windrag.) -->
<div class="room" use:easeHeight>
<header class="bar" class:floating class:tight class:narrow class:compact bind:this={barEl} bind:offsetHeight={barH} data-drag-area={floating || undefined}>
  <div class="brand" bind:this={brandEl}>
    <span class="name" bind:this={nameEl}><Logo size={32} wordSize={22} /></span>
  </div>

  <!-- (The chip is one control: a press between its buttons never moves the window.) -->
  <div class="views" role="group" aria-label={t("Space")} data-no-drag>
    {#each views as v (v.key)}
      <button class="view" class:on={store.view === v.key} aria-pressed={store.view === v.key} onclick={() => store.setView(v.key)}>{v.label}</button>
    {/each}
  </div>

  <button class="tool appear" aria-label={look.dark ? t("Switch to light") : t("Switch to dark")} data-tip={appearTitle} onclick={flip}>
    <span class="ico" class:shown={!look.dark}><Icon name="sun" /></span>
    <span class="ico moon" class:shown={look.dark}><Icon name="moon" size={17} /></span>
  </button>
  <span class="break" aria-hidden="true"></span>

  <div class="tools" bind:this={toolsEl}>
    {#if !styleIn}
    <button class="style-btn" data-tour="style" class:on={look.panelOpen} aria-label={t("Style")} aria-pressed={look.panelOpen} data-tip={t("Style")} data-tip-keys="Ctrl+Shift+S" onclick={onStyle}><Icon name="brush" size={16} /><span>{t("Style")}</span></button>
    {/if}
    {#if !newIn}
    <button class="tool" aria-label={t("New note")} data-tip={t("New note, empty or from a template")} onclick={() => (store.templatesOpen = true)}><Icon name="note-plus" /></button>
    {/if}
    <button class="tool" aria-label={t("Tidy the inbox, {n} waiting", { n: waiting })} data-tip={waiting ? t("Tidy the inbox ({n} waiting)", { n: waiting }) : t("The inbox is clear")} onclick={onTidy}>
      <Icon name="inbox" />
      {#if waiting}<span class="badge" transition:blip>{#key waiting}<span in:fadeQuick={{ duration: 160 }}>{waiting > 99 ? "99+" : waiting}</span>{/key}</span>{/if}
    </button>
    <button class="tool find" aria-label={t("Search")} data-tip={t("Search")} data-tip-keys="{store.shortcuts.search}|/" onclick={onSearch}><Icon name="search" /></button>
    {#if !compact && !tight}
      <button class="tool toggle" class:on={lookOn} aria-label={t("Look back on today")} aria-pressed={lookOn} data-tip={lookOn ? t("Close the look back") : t("Look back on today")} data-tip-keys={store.lookbackOpen ? "Esc" : null} in:fadeQuick={{ delay: 80, duration: 200 }} onclick={toggleLookBack}><Icon name="sunset" /></button>
    {/if}
    {#if !compact}
      <button class="tool" aria-label={t("Settings")} data-tip={t("Settings")} data-tip-keys="Ctrl+Shift+P|Ctrl+," in:fadeQuick={{ delay: 80, duration: 200 }} onclick={onSettings}><Icon name="settings" /></button>
    {/if}
    <button class="tool" data-tour="more" class:active={moreAt} aria-label={moreLabel} aria-haspopup="menu" aria-expanded={!!moreAt} data-tip={moreLabel} onclick={toggleMore}><Icon name="more" /></button>
    <span class="sep" aria-hidden="true"></span>
    {#if floating && !compact}
      <button
        class="tool toggle"
        class:on={store.win.float_on_top}
        aria-label={t("Always on top")}
        aria-pressed={store.win.float_on_top}
        data-tip={store.win.float_on_top ? t("Always on top · on") : t("Always on top · off")}
        in:fadeQuick={{ delay: 80, duration: 200 }}
        onclick={() => store.setFloatOnTop(!store.win.float_on_top)}><Icon name="ontop" /></button
      >
    {/if}
    {#if floating}
      <button class="tool" aria-label={t("Minimise")} data-tip={t("Minimise")} onclick={() => store.backend.minimize()}><Icon name="minimize" /></button>
      <button class="tool" aria-label={t("Close to the tray")} data-tip={t("Close (keeps running in the tray)")} onclick={onHide}><Icon name="close" /></button>
    {:else}
      <button
        class="tool toggle"
        class:on={store.pinned}
        aria-label={t("Keep the panel open")}
        aria-pressed={store.pinned}
        data-tip={store.pinned ? t("Keep the panel open · on") : t("Keep the panel open · off")}
        onclick={() => store.setPinned(!store.pinned)}><Icon name="pin" /></button
      >
      <button class="tool" aria-label={t("Hide the panel")} data-tip={t("Hide")} data-tip-keys="Esc|{store.shortcuts.toggle}" onclick={onHide}><Icon name={store.win.side === "left" ? "hide-left" : "hide-right"} /></button>
    {/if}
  </div>
  {#if moreAt}
    <Menu
      anchor={moreAt}
      items={moreItems}
      label={t("More")}
      onClose={() => {
        moreAt = null;
        moreClosed = performance.now();
      }}
    />
  {/if}
</header>
</div>

<style>
  .room {
    flex-shrink: 0;
  }
  .bar {
    height: 60px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: 0 var(--s3) 0 var(--s6);
  }
  .bar.floating {
    cursor: default;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
  }
  .name {
    display: flex;
  }
  .views {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--chip);
    flex-shrink: 0;
  }
  .view {
    height: 30px;
    padding: 0 var(--s3);
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--soft);
    font-size: var(--t-md);
    cursor: pointer;
    transition: background-color 0.18s, color 0.18s;
  }
  .view:hover {
    color: var(--ink);
  }
  .view.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .tool {
    width: 40px;
    height: 40px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }
  .tool {
    position: relative;
  }
  .badge {
    position: absolute;
    right: 3px;
    top: 4px;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    border-radius: 9px;
    background: var(--ink);
    color: var(--card);
    font-size: 10px;
    font-weight: 700;
    line-height: 17px;
    text-align: center;
    box-shadow: 0 0 0 2px var(--ground);
  }
  .tool:hover:not(:disabled) {
    background: var(--hover);
    color: var(--ink);
  }
  .tool:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .tool.active {
    background: var(--chip);
    color: var(--ink);
  }
  /* A switch that is on (Always on top, Keep open, Look back): filled with the ink, its icon in
     the paper's colour, as the Style button when its panel is open. The fill eases in and out;
     turning on, the icon gives a small press. */
  .tool.toggle {
    transition:
      background-color 0.22s var(--ease-out),
      color 0.22s var(--ease-out);
  }
  .tool.toggle.on {
    background: var(--ink);
    color: var(--card);
  }
  .tool.toggle.on:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ink) 86%, var(--ground));
    color: var(--card);
  }
  .tool.toggle.on > :global(svg) {
    animation: toggle-press 0.32s var(--ease-out);
  }
  @keyframes toggle-press {
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
  @media (prefers-reduced-motion: reduce) {
    .tool.toggle.on > :global(svg) {
      animation: none;
    }
  }
  .style-btn {
    height: 34px;
    margin-right: var(--s1);
    padding: 0 12px 0 10px;
    border: 0;
    border-radius: 9px;
    background: transparent;
    color: var(--icon);
    font-size: var(--t-md);
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 14%, transparent);
    transition: background-color 0.15s, color 0.15s, box-shadow 0.15s;
  }
  .style-btn:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .style-btn.on {
    background: var(--ink);
    color: var(--card);
    box-shadow: none;
  }
  /* Sits with the tools (their 2px gap, not the bar's). The icon turns as it changes: the old
     one leaves before the new one is fully there. */
  .appear {
    margin-right: calc(2px - var(--s4));
    flex-shrink: 0;
  }
  .ico {
    grid-area: 1 / 1;
    display: grid;
    opacity: 0;
    transform: rotate(-70deg) scale(0.6);
    transition:
      opacity 0.16s ease,
      transform 0.16s ease;
  }
  .ico.shown {
    opacity: 1;
    transform: none;
    transition:
      opacity 0.22s ease 0.1s,
      transform 0.34s var(--ease-glide, cubic-bezier(0.2, 0.8, 0.2, 1)) 0.1s;
  }
  .break {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .ico,
    .ico.shown {
      transform: none;
      transition: opacity 0.15s;
    }
  }
  .sep {
    width: 1px;
    height: 20px;
    margin: 0 var(--s1);
    background: var(--line);
  }
  /* Tight: the Style button keeps only its brush (its name is in its tooltip). */
  .bar.tight .style-btn {
    width: 40px;
    padding: 0;
    justify-content: center;
  }
  .bar.tight .style-btn span {
    display: none;
  }
  /* Two rows: the name and the buttons on top, All / Work / Personal (and the sun/moon) across
     below. */
  .bar.narrow {
    height: auto;
    flex-wrap: wrap;
    row-gap: var(--s2);
    padding-top: var(--s3);
    padding-bottom: var(--s1);
  }
  /* Row two: All / Work / Personal, and the sun/moon at its end. */
  .narrow .break {
    display: block;
    order: 2;
    flex-basis: 100%;
    height: 0;
  }
  .narrow .views {
    order: 3;
    flex: 1 1 0;
    min-width: 0;
  }
  .narrow .tool.appear {
    order: 4;
    width: 36px;
    height: 36px;
    margin-right: 0;
    margin-left: calc(var(--s2) - var(--s4));
  }
  .narrow .view {
    flex: 1;
  }
  .narrow .tool {
    width: 34px;
  }
  .narrow .sep {
    margin: 0 2px;
  }
  .narrow .style-btn {
    width: 34px;
    padding: 0;
    justify-content: center;
    box-shadow: none;
  }
  .narrow .style-btn span {
    display: none;
  }
</style>
