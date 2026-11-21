<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The terms of use (TERMS.md), before anything goes online, as Pako: the first thing a portable
  // Dobo shows (Dobo Setup's "By installing, you accept the terms of use" counts for an installed
  // one), and once for people who used Dobo before it had terms. Only its own two buttons answer
  // it (no Esc, no click beside it). Until Agree and continue, Dobo makes no update check and
  // fetches no link preview (src-tauri/src/consent.rs); the notes work as always.
  import termsMd from "../../TERMS.md?raw";
  import Logo from "./Logo.svelte";
  import { store } from "../lib/store.svelte";
  import { APP_NAME } from "../lib/app";
  import { sheet } from "../lib/motion";
  import { parseTerms } from "../lib/terms";
  import { t } from "../lib/i18n.svelte";

  let { onDone }: { onDone: () => void } = $props();

  /** Used Dobo before it had terms (its first-start setup was done then). */
  const existing = !!store.settings.onboarded;
  const blocks = parseTerms(termsMd);

  let busy = $state(false);
  let error = $state("");
  async function agree() {
    if (busy) return;
    busy = true;
    try {
      // (The date saved is the terms' own "Last updated" date, as Dobo Setup saves it.)
      const date = await store.backend.acceptTerms();
      store.settings = { ...store.settings, terms_accepted: date };
      onDone();
    } catch (e) {
      error = t("{app} couldn’t save that you agreed: {error}", { app: APP_NAME, error: String(e) });
    }
    busy = false;
  }
  function open(url: string) {
    void store.backend.openUrl(url);
  }
</script>

<div class="terms" role="dialog" aria-modal="true" aria-label={t("Terms of use")} transition:sheet|global>
  <div class="move" data-drag-area aria-hidden="true"></div>
  <div class="page">
    <span class="kicker"><Logo word={false} size={22} />{existing ? t("{app} now has terms of use", { app: APP_NAME }) : t("Welcome to {app}", { app: APP_NAME })}</span>
    <h1>{t("Terms of use")}</h1>
    <p class="lead">
      {existing
        ? t("Please read them: {app} carries on as soon as you agree. Your notes stay as they are.", { app: APP_NAME })
        : t("Before you start, please read {app}’s terms of use. {app} starts as soon as you agree.", { app: APP_NAME })}
    </p>
    <!-- Focusable so the keyboard can scroll it. (The terms are in English, as published.) -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="text" role="document" lang="en" tabindex="0">
      {#each blocks as b, i (i)}
        <!-- (The "# Terms of use" heading: the page's own title says it.) -->
        {#if b.kind === "h2"}
          <h2>{b.text}</h2>
        {:else if b.kind === "p" || b.kind === "li"}
          <svelte:element this={b.kind} class:dated={b.kind === "p" && b.parts.length === 1 && b.parts[0].text.startsWith("Last updated")}>
            {#each b.parts as p, j (j)}{#if p.url}<button class="link" onclick={() => open(p.url!)}>{p.text}</button>{:else if p.bold}<b>{p.text}</b>{:else}{p.text}{/if}{/each}
          </svelte:element>
        {/if}
      {/each}
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="foot">
      <button class="back" onclick={() => void store.backend.quitApp()}>{t("Quit {app}", { app: APP_NAME })}</button>
      <button class="go" disabled={busy} onclick={agree}>{t("Agree and continue")}</button>
    </div>
  </div>
</div>

<style>
  .terms {
    position: absolute;
    inset: 0;
    /* Over the first-start setup (70). */
    z-index: 80;
    overflow-y: auto;
    background: var(--ground);
    display: grid;
    container: terms / inline-size;
  }
  .move {
    grid-area: 1 / 1;
    align-self: start;
    position: sticky;
    top: 0;
    z-index: 1;
    height: 32px;
    background: linear-gradient(var(--ground) 75%, transparent);
  }
  .page {
    grid-area: 1 / 1;
    min-height: 100%;
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
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
  .text {
    margin-top: 22px;
    padding: 6px 20px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--card);
    font-size: var(--t-md);
    line-height: 1.55;
    color: var(--ink);
  }
  .text:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  .text h2 {
    margin: 18px 0 4px;
    font-family: var(--serif);
    font-size: 17px;
    font-weight: 600;
  }
  .text p {
    margin: 8px 0 0;
  }
  .text p.dated {
    color: var(--muted);
  }
  .text li {
    margin: 4px 0 0 18px;
    list-style: disc;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: var(--line);
    text-underline-offset: 3px;
    transition: text-decoration-color 0.16s ease;
  }
  .link:hover {
    text-decoration-color: var(--ink);
  }
  .error {
    margin: 12px 0 0;
    color: var(--danger, #b3261e);
    font-size: var(--t-md);
  }
  .foot {
    margin-top: auto;
    padding-top: 24px;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--s4);
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
  .go:disabled {
    opacity: 0.6;
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
  @container terms (max-width: 520px) {
    .page {
      padding: 28px 20px 20px;
    }
    h1 {
      font-size: 26px;
    }
    .text {
      padding: 4px 14px 12px;
    }
  }
</style>
