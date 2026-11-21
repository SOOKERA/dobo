<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // First launch (FirstRun board): how Dobo sits on the screen, shown with small moving
  // pictures; then the shortcuts, starting with the computer, and the Sticky Notes import.
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import { store } from "../lib/store.svelte";
  import { inTauri } from "../lib/backend";
  import { APP_NAME } from "../lib/app";
  import { fadeQuick, side } from "../lib/motion";
  import { around, i18n, keysText, readChoice, switchLanguage, t, type LangChoice, words, type Words } from "../lib/i18n.svelte";

  let { onDone }: { onDone: () => void } = $props();

  let step = $state<1 | 2>(1);
  /** Going back to step 1: the pages slide the other way. */
  let back = $state(false);
  let mode = $state<"docked" | "floating">(store.win.docked_possible ? "docked" : "floating");
  // On by default, as in Dobo Setup (its "Start Dobo with Windows" is on): an installed copy and
  // a portable one start the same way. Installed, it shows what was chosen in Setup
  // (`installer_start`, saved by --install-choices), so finishing here never undoes it.
  let autostart = $state(store.settings.installer_start !== false);
  let importMsg = $state<Words>("");
  let imported = $derived(Number(store.settings.sticky_imported ?? 0));
  const isWindows = navigator.userAgent.includes("Windows");
  let osName = $derived(isWindows ? "Windows" : navigator.userAgent.includes("Mac") ? t("your Mac") : t("your computer"));
  // The language, right here too (it starts as Windows speaks).
  let langChoice = $derived(readChoice(store.settings.language));
  let firstEl = $state<HTMLElement>();
  function setLanguage(c: LangChoice) {
    if (c !== langChoice) void switchLanguage(c, () => store.setSetting("language", c), firstEl);
  }
  let [stickyBefore, stickyAfter] = $derived(around(t("Your {n} sticky notes are already on the board.", { n: imported }), String(imported)));

  async function next() {
    await store.setSetting("mode", mode);
    await store.refreshWindow();
    await store.backend.applyWindow();
    back = false;
    step = 2;
  }

  async function bringIn() {
    importMsg = () => t("Reading Sticky Notes…");
    try {
      const n = await store.backend.importStickyNotes();
      await store.reload();
      importMsg = () => (n ? t("Brought in {n} notes.", { n }) : t("No sticky notes found on this computer."));
    } catch (e) {
      importMsg = () => t("Couldn't read Sticky Notes: {error}", { error: String(e) });
    }
  }

  async function finish() {
    try {
      await store.backend.setAutostart(autostart);
    } catch {
      /* not possible here (preview) */
    }
    await store.setSetting("onboarded", Date.now());
    onDone();
  }
</script>

<div class="first" bind:this={firstEl} role="dialog" aria-label={t("Welcome to {app}", { app: APP_NAME })}>
  <!-- The window has no title bar: its top strip moves it (lib/windrag), as it stays in view. -->
  <div class="move" data-drag-area aria-hidden="true"></div>
  <div class="lang" role="radiogroup" aria-label={t("Language")}>
    {#each [["en", "English"], ["fr", "Français"]] as const as [k, name] (k)}
      <button role="radio" lang={k} aria-checked={i18n.lang === k} class:on={i18n.lang === k} onclick={() => setLanguage(k)}>{name}</button>
    {/each}
  </div>
  {#if step === 1}
    <div class="page" data-step="1" in:side={{ x: back ? -24 : 24, duration: 400 }} out:side={{ x: back ? 24 : -24, duration: 400 }}>
      <span class="kicker"><Logo word={false} size={22} />{t("Welcome to {app}", { app: APP_NAME })}</span>
      <h1>{t("How should it sit on your screen?")}</h1>
      <p class="lead">{t("You can change this any time in Settings.")}</p>

      <div class="choices" role="radiogroup" aria-label={t("Window mode")}>
        <button role="radio" aria-checked={mode === "docked"} class:on={mode === "docked"} disabled={!store.win.docked_possible} onclick={() => (mode = "docked")}>
          <div class="pic" aria-hidden="true">
            <div class="win"></div>
            <div class="dock">
              <span class="t"></span>
              <span class="n" style:--tint="#FBF4DF" style:height="34px"></span>
              <span class="n" style:--tint="#ECF2F9" style:height="44px"></span>
              <span class="n" style:--tint="#F2EFFA" style:height="30px"></span>
            </div>
            <span class="keys">{keysText(store.shortcuts.toggle)}</span>
            <div class="task"></div>
          </div>
          <span class="name"><span class="radio"></span>{t("Docked to the edge")}</span>
          <span class="text"
            >{store.win.docked_possible
              ? t("Slides in with a shortcut on the screen your mouse is on, hides when you click away. Always one key away, never in the way.")
              : t("Not possible on this desktop (Wayland): use the floating window.")}</span
          >
        </button>

        <button role="radio" aria-checked={mode === "floating"} class:on={mode === "floating"} onclick={() => (mode = "floating")}>
          <div class="pic" aria-hidden="true">
            <div class="win"></div>
            <div class="float">
              <div class="fbar"><span class="on"></span><span></span><span></span></div>
              <div class="fgrid">
                <span style:--tint="#EDF3EA" style:height="34px"></span>
                <span style:--tint="#FAEEEE" style:height="44px"></span>
                <span style:--tint="#E8F2F3" style:height="40px"></span>
              </div>
            </div>
            <span class="keys left">{t("Always on top, if you like")}</span>
            <div class="task"></div>
          </div>
          <span class="name"><span class="radio"></span>{t("A floating window")}</span>
          <span class="text">{t("Put it wherever you like, any size, on any screen, and keep it above other windows. Like a sticky note that stays in sight.")}</span>
        </button>
      </div>

      <div class="foot">
        <span class="soft">{t("Next: your shortcuts, and your notes from Sticky Notes")}</span>
        <button class="go" onclick={next}>{t("Continue")}</button>
      </div>
    </div>
  {:else}
    <div class="page" data-step="2" in:side={{ x: back ? -24 : 24, duration: 400 }} out:side={{ x: back ? 24 : -24, duration: 400 }}>
      <span class="kicker">{t("Almost there")}</span>
      <h1>{t("A few keys to remember")}</h1>
      <p class="lead">{t("They work from anywhere. Change them any time in Settings.")}</p>

      <div class="keys-list">
        <div class="row"><span>{t("Open or hide {app}", { app: APP_NAME })}</span><span class="k">{keysText(store.shortcuts.toggle)}</span></div>
        <div class="row"><span>{t("Quick dump into the Inbox")}</span><span class="k">{keysText(store.shortcuts.dump)}</span></div>
        <div class="row"><span>{t("Search, when {app} is open", { app: APP_NAME })}</span><span class="k">{keysText(store.shortcuts.search)}</span></div>
        <div class="row"><span>{t("A task · an idea · a deadline")}</span><span class="k">{t("Enter · Shift+Enter · end with !")}</span></div>
      </div>

      <label class="toggle">
        <span>
          <b>{t("Start with {os}", { os: osName })}</b>
          <small>{t("Reminders only ring while {app} runs; it waits quietly in the tray.", { app: APP_NAME })}</small>
        </span>
        <input type="checkbox" bind:checked={autostart} />
        <span class="sw" aria-hidden="true"></span>
      </label>

      {#if isWindows || !inTauri}
        <div class="sticky">
          <Icon name="inbox" size={20} />
          {#if imported > 0}
            <span in:fadeQuick={{ duration: 220 }}>{stickyBefore}<b>{imported}</b>{stickyAfter}</span>
          {:else}
            <span>{words(importMsg) || t("Bring in the notes you already have in Windows Sticky Notes.")}</span>
            {#if !importMsg}<button class="small" out:fadeQuick onclick={bringIn}>{t("Bring them in")}</button>{/if}
          {/if}
        </div>
      {/if}

      <div class="foot">
        <button class="back" onclick={() => ((back = true), (step = 1))}>{t("Back")}</button>
        <button class="go" onclick={finish}>{t("Start (button)")}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  /* The language, in the corner (it starts as Windows speaks). */
  .lang {
    position: absolute;
    top: 16px;
    right: 20px;
    z-index: 2;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--ink) 5%, transparent);
  }
  .lang button {
    height: 24px;
    padding: 0 11px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--t-sm);
    cursor: pointer;
    transition:
      background-color 0.18s,
      color 0.18s,
      box-shadow 0.18s;
  }
  .lang button:hover {
    color: var(--ink);
  }
  .lang button.on {
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .first {
    position: absolute;
    inset: 0;
    z-index: 70;
    overflow-y: auto;
    background: var(--ground);
    display: grid;
  }
  /* Over the pages' top padding (under the language), held at the top while they scroll. It is
   * no taller than that padding, and what scrolls under it goes out of sight (the paper's own
   * colour, softening at its lower edge): nothing that shows is ever under it, unclickable. */
  .move {
    grid-area: 1 / 1;
    align-self: start;
    position: sticky;
    top: 0;
    z-index: 1;
    height: 32px;
    background: linear-gradient(var(--ground) 75%, transparent);
  }
  /* Both steps share the place while one slides out and the other in. */
  .page {
    grid-area: 1 / 1;
    min-height: 100%;
    padding: 36px 40px 28px;
    display: flex;
    flex-direction: column;
  }
  .kicker {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
    color: var(--muted);
  }
  h1 {
    margin: 6px 0 0;
    font-family: var(--serif);
    font-size: 32px;
    font-weight: 500;
    letter-spacing: -0.01em;
    line-height: 1.15;
  }
  .lead {
    margin: var(--s2) 0 0;
    font-size: var(--t-base);
    color: var(--soft);
  }
  .choices {
    margin-top: 28px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 20px;
  }
  .choices button {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 13px 13px 19px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: var(--card);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.2s;
  }
  .choices button.on {
    border: 2px solid var(--ink);
    padding: 12px 12px 18px;
    box-shadow: 0 8px 24px rgba(60, 50, 30, 0.1);
  }
  .choices button:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .pic {
    position: relative;
    height: 200px;
    border-radius: 10px;
    overflow: hidden;
    background: linear-gradient(160deg, #6a7b86 0%, #3f4c57 70%, #2e3740 100%);
  }
  .win {
    position: absolute;
    left: 16px;
    top: 16px;
    width: 58%;
    height: 130px;
    border-radius: 5px;
    background: #f7f7f8;
    opacity: 0.92;
  }
  .task {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 18px;
    background: rgba(28, 30, 34, 0.9);
  }
  .dock {
    position: absolute;
    right: 8px;
    top: 8px;
    bottom: 26px;
    width: 108px;
    padding: 8px;
    border-radius: 8px;
    background: #f2f0eb;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3);
    display: flex;
    flex-direction: column;
    gap: 6px;
    animation: slide 6s cubic-bezier(0.3, 0.8, 0.25, 1) infinite;
  }
  .dock .t {
    height: 8px;
    width: 50%;
    border-radius: 4px;
    background: #1f1d1a;
  }
  .dock .n,
  .fgrid span {
    border-radius: 5px;
    background: #fffefb;
    box-shadow: inset 0 -12px 0 var(--tint);
  }
  .keys {
    position: absolute;
    left: 56px;
    top: 156px;
    font-size: 11px;
    font-weight: 600;
    color: #fff;
    background: rgba(20, 22, 26, 0.7);
    border-radius: 6px;
    padding: 3px 8px;
    animation: key 6s ease infinite;
  }
  .keys.left {
    left: 16px;
    animation: none;
  }
  .float {
    position: absolute;
    left: 52%;
    top: 34px;
    width: 130px;
    height: 112px;
    border-radius: 8px;
    background: #f2f0eb;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.35);
    overflow: hidden;
    animation: drift 7s cubic-bezier(0.45, 0, 0.2, 1) infinite;
  }
  .fbar {
    height: 16px;
    background: #eae6de;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 5px;
    padding: 0 6px;
  }
  .fbar span {
    width: 6px;
    height: 6px;
    border-radius: 2px;
    background: #b5b0a5;
  }
  .fbar span.on {
    background: #1f1d1a;
  }
  .fgrid {
    padding: 6px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 5px;
  }
  @keyframes slide {
    0%,
    12% {
      transform: translateX(120px);
    }
    26%,
    74% {
      transform: translateX(0);
    }
    88%,
    100% {
      transform: translateX(120px);
    }
  }
  @keyframes key {
    0%,
    6% {
      opacity: 0;
    }
    10%,
    30% {
      opacity: 1;
    }
    36%,
    100% {
      opacity: 0;
    }
  }
  @keyframes drift {
    0%,
    15% {
      transform: translate(0, 0);
    }
    45%,
    60% {
      transform: translate(-96px, 26px);
    }
    90%,
    100% {
      transform: translate(0, 0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dock,
    .keys,
    .float,
    .page {
      animation: none;
    }
  }
  .name {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 6px;
    font-family: var(--serif);
    font-size: 20px;
    font-weight: 600;
    color: var(--ink);
  }
  .radio {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border: 1.5px solid var(--box);
    border-radius: 50%;
  }
  .on .radio {
    border: 5px solid var(--ink);
  }
  .text {
    padding: 0 6px;
    font-size: 13.5px;
    line-height: 1.5;
    color: var(--soft);
  }
  .foot {
    margin-top: auto;
    padding-top: 28px;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--s4);
  }
  .soft {
    font-size: var(--t-md);
    color: var(--muted);
  }
  .go {
    height: 44px;
    padding: 0 22px;
    border: 0;
    border-radius: 10px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-base);
    font-weight: 600;
    cursor: pointer;
  }
  .back {
    height: 44px;
    padding: 0 18px;
    margin-right: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    font-size: var(--t-base);
    cursor: pointer;
  }
  .keys-list {
    margin-top: 24px;
    max-width: 620px;
    border-top: 1px solid var(--line);
  }
  .row {
    min-height: 48px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    border-bottom: 1px solid var(--line);
    font-size: var(--t-base);
  }
  .k {
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    padding: 3px 8px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    background: var(--card);
    white-space: nowrap;
  }
  .toggle {
    position: relative;
    margin-top: 22px;
    max-width: 620px;
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: 14px var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--card);
    cursor: pointer;
  }
  .toggle > span:first-child {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .toggle b {
    font-size: var(--t-base);
    font-weight: 600;
  }
  .toggle small {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .sw {
    width: 46px;
    height: 28px;
    flex-shrink: 0;
    border-radius: 14px;
    background: var(--edge);
    position: relative;
    transition: background-color 0.2s;
  }
  .sw::after {
    content: "";
    position: absolute;
    left: 3px;
    top: 3px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--card);
    transition: transform 0.25s var(--ease-out);
  }
  .toggle input:checked + .sw {
    background: var(--ink);
  }
  .toggle input:checked + .sw::after {
    transform: translateX(18px);
  }
  .toggle input:focus-visible + .sw {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .sticky {
    margin-top: 14px;
    max-width: 620px;
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 14px var(--s4);
    border-radius: var(--r-lg);
    background: #fbf4df;
    color: #4a3c14;
    font-size: var(--t-base);
  }
  .sticky span {
    flex: 1;
  }
  .small {
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid #eadba9;
    border-radius: var(--r-md);
    background: #fffefb;
    color: #4a3c14;
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
  }
  :global(:root[data-mode="dark"]) .sticky {
    background: rgba(217, 169, 40, 0.12);
    color: #ead9a0;
  }
  :global(:root[data-mode="dark"]) .small {
    background: transparent;
    color: #ead9a0;
    border-color: #5e4b19;
  }
  @container shell (max-width: 560px) {
    .page {
      padding: 24px 20px 20px;
    }
    .move {
      height: 24px;
    }
    h1 {
      font-size: 26px;
    }
  }
  /* The narrowest window: the welcome line (the note and the words) would touch the language
     switch beside it, so it starts under it. */
  @container shell (max-width: 380px) {
    .page {
      padding-top: 56px;
    }
  }
</style>
