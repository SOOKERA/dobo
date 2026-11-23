<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // First launch (FirstRun board): how Dobo sits on the screen, shown with small moving
  // pictures; then the shortcuts, starting with the computer, and, when Windows Sticky Notes has
  // notes, the offer to bring them in (asked, never done by itself).
  import Icon from "./Icon.svelte";
  import { roving } from "../lib/gridkeys";
  import Logo from "./Logo.svelte";
  import { store } from "../lib/store.svelte";
  import { inTauri } from "../lib/backend";
  import { APP_NAME } from "../lib/app";
  import { arrive, easeHeight, fadeQuick, fold, leave, side } from "../lib/motion";
  import { modal, tabbables } from "../lib/modal";
  import { onMount, tick } from "svelte";
  import { askedLanguage, i18n, keysText, readChoice, switchLanguage, t, type LangChoice, words, type Words } from "../lib/i18n.svelte";

  let { onDone }: { onDone: () => void } = $props();

  /** How long bringing in Sticky Notes may take before it is said to have failed. */
  const STICKY_LIMIT_MS = 20_000;
  class StickyTimeout extends Error {}

  let step = $state<1 | 2>(1);
  /** Going back to step 1: the pages slide the other way. */
  let back = $state(false);
  let mode = $state<"docked" | "floating">(store.win.docked_possible ? "docked" : "floating");
  // On by default, as in Dobo Setup (its "Start Dobo with Windows" is on): an installed copy and
  // a portable one start the same way. Installed, it shows what was chosen in Setup
  // (`installer_start`, saved by --install-choices), so finishing here never undoes it.
  let autostart = $state(store.settings.installer_start !== false);
  const isWindows = navigator.userAgent.includes("Windows");
  let osName = $derived(isWindows ? "Windows" : navigator.userAgent.includes("Mac") ? t("your Mac") : t("your computer"));
  // The language, right here too: it starts from the setting, the one picked in Dobo Setup
  // (--install-choices --lang, install.rs) or else as Windows speaks.
  let langChoice = $derived(readChoice(store.settings.language));
  let firstEl = $state<HTMLElement>();
  function setLanguage(c: LangChoice) {
    if (c !== (askedLanguage() ?? langChoice)) void switchLanguage(c, () => store.setSetting("language", c), firstEl);
  }

  // Windows Sticky Notes: how many an import would bring in (null while that is read; 0 when
  // there are none, or "Not now" was said at an earlier start: then nothing is shown). Nothing
  // comes in unless Import is pressed; Settings › Start can bring them in any time.
  let stickyCount = $state<number | null>(null);
  /** The offer: asked, being brought in, done, put off, or failed (then asked again). */
  let sticky = $state<"ask" | "busy" | "done" | "later" | "failed">("ask");
  let stickyMsg = $state<Words>("");
  /** Said to screen readers when the offer's answer changes. */
  let stickySaid = $state("");
  let stickyEl = $state<HTMLElement>();
  // The count can come in once this page shows: the offer then unfolds above Back and Start,
  // moving them down. Never while the pointer is on them (a click aimed at Start would land on
  // the offer): it unfolds as soon as the pointer leaves them (or they go: Back). Start pressed
  // before that shows the offer, once, instead of starting (`finish`).
  let offerShown = $state(false);
  let onFoot = $state(false);
  $effect(() => {
    if (stickyCount && !offerShown && !(step === 2 && onFoot)) offerShown = true;
  });
  /** The pointer on Back and Start: kept for as long as they are there (cleared as they go, the
   * pointer then never leaving them). */
  function footWatch(node: HTMLElement) {
    const enter = () => (onFoot = true);
    const leave = () => (onFoot = false);
    node.addEventListener("pointerenter", enter);
    node.addEventListener("pointerleave", leave);
    return {
      destroy() {
        node.removeEventListener("pointerenter", enter);
        node.removeEventListener("pointerleave", leave);
        onFoot = false;
      },
    };
  }
  /** Start was pressed while the offer waited unseen: it was shown instead, once. */
  let startHeld = false;
  onMount(() => {
    if (store.settings.sticky_declined || !(isWindows || !inTauri)) return void (stickyCount = 0);
    store.backend.countStickyNotes().then(
      (n) => (stickyCount = n),
      () => (stickyCount = 0),
    );
  });

  async function next() {
    await store.setSetting("mode", mode);
    await store.refreshWindow();
    await store.backend.applyWindow();
    back = false;
    step = 2;
    await follow(2, null);
  }
  /** The keyboard follows from step to step (when it was in the window): to the new step's first
   * choice, or back to Continue. */
  async function follow(to: 1 | 2, sel: string | null) {
    const a = document.activeElement;
    const keys = !a || a === document.body || !!firstEl?.contains(a);
    await tick();
    if (!keys) return;
    const page = firstEl?.querySelector<HTMLElement>(`.page[data-step="${to}"]:not([inert])`);
    const el = page && (sel ? page.querySelector<HTMLElement>(sel) : tabbables(page)[0]);
    el?.focus({ preventScroll: true });
  }
  function toStep1() {
    // (Back and Start go, under the pointer: it is on them no more.)
    onFoot = false;
    back = true;
    step = 1;
    void follow(1, ".go");
  }

  async function bringIn() {
    if (sticky === "busy") return;
    sticky = "busy";
    stickyMsg = () => t("Reading Sticky Notes…");
    stickySaid = words(stickyMsg);
    // (Never waited for forever: a Sticky Notes file that can't be read, on a stuck drive for
    // one. If it does come in after all, it is said then.)
    const importing = store.backend.importStickyNotes();
    let limit = 0;
    const late = new Promise<never>((_, no) => (limit = window.setTimeout(() => no(new StickyTimeout()), STICKY_LIMIT_MS)));
    try {
      const n = await Promise.race([importing, late]);
      await brought(n);
    } catch (e) {
      // Asked again, with what went wrong under the question (Import tries again).
      const slow = e instanceof StickyTimeout;
      stickyMsg = () => t("Couldn’t read Sticky Notes: {error}", { error: slow ? t("it took too long") : String(e) });
      sticky = "failed";
      stickySaid = words(stickyMsg);
      if (slow) {
        importing.then(
          (n) => {
            if (sticky === "failed") void brought(n);
          },
          () => {},
        );
      }
    } finally {
      clearTimeout(limit);
    }
  }
  async function brought(n: number) {
    await store.reload();
    stickyMsg = () => (n ? t("Your {n} sticky notes are on the board now.", { n }) : t("Nothing new to bring in from Sticky Notes."));
    await answered("done");
  }
  /** Not now: remembered (a later start doesn't ask again); Settings › Start still can. */
  async function notNow() {
    if (sticky === "busy") return;
    stickyMsg = () => t("They stay in Sticky Notes. Settings › Start brings them in any time.");
    await answered("later");
    await store.setSetting("sticky_declined", Date.now());
  }
  /** The offer's answer takes the question's place; the keyboard, if it was on its buttons
   * (now gone), goes on to Start. */
  async function answered(how: "done" | "later") {
    const had = !!stickyEl?.contains(document.activeElement);
    sticky = how;
    stickySaid = words(stickyMsg);
    await tick();
    if (had) firstEl?.querySelector<HTMLElement>('.page[data-step="2"]:not([inert]) .go')?.focus({ preventScroll: true });
  }

  async function finish() {
    // (Not while the notes are coming in: the window would close before saying how it went.)
    if (sticky === "busy") return;
    // (An offer that came in under the pointer, not shown yet: shown now, once, rather than
    // started past without a word.)
    if (stickyCount && !offerShown && !startHeld) {
      startHeld = true;
      offerShown = true;
      return;
    }
    try {
      await store.backend.setAutostart(autostart);
    } catch {
      /* not possible here (preview) */
    }
    await store.setSetting("onboarded", Date.now());
    onDone();
  }
</script>

<!-- (On Start it fades away over the board that is already there, rather than cutting to it.) -->
<div class="first" bind:this={firstEl} role="dialog" aria-label={t("Welcome to {app}", { app: APP_NAME })} tabindex="-1" use:modal={{ initial: "self" }} out:fadeQuick={{ duration: 200 }}>
  <!-- The window has no title bar: its top strip moves it (lib/windrag), as it stays in view. -->
  <div class="move" data-drag-area aria-hidden="true"></div>
  <div class="lang" role="radiogroup" use:roving aria-label={t("Language")}>
    {#each [["en", "English"], ["fr", "Français"]] as const as [k, name] (k)}
      <button role="radio" lang={k} aria-checked={i18n.lang === k} class:on={i18n.lang === k} onclick={() => setLanguage(k)}>{name}</button>
    {/each}
  </div>
  {#if step === 1}
    <div class="page" data-step="1" in:side={{ x: back ? -24 : 24, duration: 400 }} out:side={{ x: back ? 24 : -24, duration: 400 }}>
      <span class="kicker"><Logo word={false} size={22} />{t("Welcome to {app}", { app: APP_NAME })}</span>
      <h1>{t("How should it sit on your screen?")}</h1>
      <p class="lead">{t("You can change this any time in Settings.")}</p>

      <div class="choices" role="radiogroup" use:roving aria-label={t("Window mode")}>
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
        <!-- (Once it is known whether Sticky Notes has notes to bring in.) -->
        {#if stickyCount !== null}<span class="soft" in:fadeQuick={{ duration: 220 }}>{stickyCount ? t("Next: your shortcuts, and your notes from Sticky Notes") : t("Next: your shortcuts")}</span>{/if}
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
        <div class="row"><span>{t("A task · an idea · a deadline")}</span><span class="k">{t("Enter · Shift+Enter · end with\u00a0!")}</span></div>
      </div>

      <label class="toggle">
        <span>
          <b>{t("Start {app} with {os}", { app: APP_NAME, os: osName })}</b>
          <small>{t("Reminders only ring while {app} runs; it waits quietly in the tray.", { app: APP_NAME })}</small>
        </span>
        <input type="checkbox" bind:checked={autostart} />
        <span class="sw" aria-hidden="true"></span>
      </label>

      <!-- Windows Sticky Notes, when it has notes to bring in: a question, answered here. It
           unfolds if the count comes in after this page; its answer then takes the question's
           place, the strip easing to its new height. -->
      {#if stickyCount && offerShown}
        <section class="sticky" aria-labelledby="sticky-q" aria-busy={sticky === "busy"} bind:this={stickyEl} transition:fold={{ duration: 320 }} use:easeHeight>
          <div class="stack">
            {#if sticky === "ask" || sticky === "busy" || sticky === "failed"}
              <div class="offer" in:arrive out:leave>
                <span class="ic"><Icon name="inbox" size={20} /></span>
                <span class="words">
                  <span class="q" id="sticky-q">{t("Bring in your {n} Windows Sticky Notes?", { n: stickyCount })}</span>
                  <small id="sticky-more" class:warn={sticky === "failed"}>{sticky === "ask" ? t("A copy comes onto your board; Sticky Notes keeps its own.") : words(stickyMsg)}</small>
                </span>
                <span class="acts">
                  <button class="plain" aria-disabled={sticky === "busy"} onclick={notNow}>{t("Not now")}</button>
                  <button class="small" aria-disabled={sticky === "busy"} aria-describedby="sticky-more" onclick={bringIn}>{t("Import")}</button>
                </span>
              </div>
            {:else}
              <div class="offer" in:arrive out:leave>
                <span class="ic" class:done={sticky === "done"}><Icon name={sticky === "done" ? "check" : "inbox"} size={20} /></span>
                <span class="words"><span class="q said" id="sticky-q">{words(stickyMsg)}</span></span>
              </div>
            {/if}
          </div>
          <span class="visually-hidden" aria-live="polite">{stickySaid}</span>
        </section>
      {/if}

      <div class="foot" use:footWatch>
        <button class="back" onclick={toStep1}>{t("Back")}</button>
        <button class="go" aria-disabled={sticky === "busy"} onclick={finish}>{t("Start (button)")}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  /* The language, in the corner (it starts as the setting says: Setup's choice, or Windows). */
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
    color: var(--soft);
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
    transition: opacity 0.2s var(--ease-glide);
  }
  /* (While the notes come in: Start waits.) */
  .go[aria-disabled="true"] {
    opacity: 0.55;
    cursor: default;
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
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  /* The Sticky Notes offer: a sticky note of its own, under the switch above (same width,
     padding and corners). The icon sits level with the question's line; the buttons to the
     right, centred on the words (under them, right-aligned, when it is narrow). */
  .sticky {
    margin-top: 14px;
    max-width: 620px;
    padding: 14px var(--s4);
    border-radius: var(--r-lg);
    background: #fbf4df;
    color: #4a3c14;
    font-size: var(--t-base);
  }
  .stack {
    position: relative;
    display: grid;
  }
  .offer {
    grid-area: 1 / 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: var(--s3);
    row-gap: 10px;
  }
  .ic {
    flex-shrink: 0;
    align-self: flex-start;
    display: grid;
    place-items: center;
    width: 20px;
    /* The question's line box: the icon is centred on it. */
    height: 20px;
  }
  .ic.done {
    color: #6d8f4e;
  }
  .words {
    flex: 1 1 220px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .q {
    line-height: 20px;
    font-weight: 600;
  }
  /* The answer: said, not asked. */
  .q.said {
    font-weight: 500;
  }
  .words small {
    font-size: var(--t-sm);
    line-height: 17px;
    color: #7a6a3e;
  }
  .words small.warn {
    color: #9a3b26;
  }
  .acts {
    flex-shrink: 0;
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .small,
  .plain {
    height: 32px;
    padding: 0 var(--s3);
    border-radius: var(--r-md);
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.18s,
      border-color 0.18s,
      color 0.18s,
      opacity 0.18s;
  }
  .small {
    border: 1px solid #eadba9;
    background: #fffefb;
    color: #4a3c14;
  }
  .small:hover {
    border-color: #d9c68a;
  }
  .plain {
    border: 1px solid transparent;
    background: transparent;
    color: #7a6a3e;
  }
  .plain:hover {
    background: rgba(74, 60, 20, 0.07);
    color: #4a3c14;
  }
  .small[aria-disabled="true"],
  .plain[aria-disabled="true"] {
    opacity: 0.55;
    cursor: default;
  }
  :global(:root[data-mode="dark"]) .sticky {
    background: rgba(217, 169, 40, 0.12);
    color: #ead9a0;
  }
  :global(:root[data-mode="dark"]) .words small {
    color: #bfae78;
  }
  :global(:root[data-mode="dark"]) .words small.warn {
    color: #f0a48f;
  }
  :global(:root[data-mode="dark"]) .ic.done {
    color: #a9c98a;
  }
  :global(:root[data-mode="dark"]) .small {
    background: transparent;
    color: #ead9a0;
    border-color: #5e4b19;
  }
  :global(:root[data-mode="dark"]) .small:hover {
    border-color: #7c6526;
  }
  :global(:root[data-mode="dark"]) .plain {
    color: #bfae78;
  }
  :global(:root[data-mode="dark"]) .plain:hover {
    background: rgba(234, 217, 160, 0.08);
    color: #ead9a0;
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
  /* Narrow: each key under what it does (beside it, a long one squeezed the words to a column),
     as in Settings › Keyboard. */
  @container shell (max-width: 420px) {
    .row {
      flex-direction: column;
      align-items: flex-start;
      justify-content: center;
      gap: 6px;
      padding: 10px 0;
    }
  }
  /* (It takes the keyboard itself when it opens: no ring round the whole view.) */
  .first:focus {
    outline: none;
  }
</style>
