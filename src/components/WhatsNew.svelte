<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // What's new (as Pako's): once on the first open after an update, every version since the one
  // you had, newest first ("What's new since Dobo 1.1.0", each with its heading), or one version's
  // own ("What's new in Dobo 1.2.0"); again any time from the "?" menu (Back: the board) or
  // Settings › Updates (Back: there). The words are docs/help/whats-new.md, read as text
  // (whatsnew.ts): a guide's link opens it on dobo.sookera.com, a web link in the browser.
  import ViewHeader, { type Crumb } from "./ViewHeader.svelte";
  import { store } from "../lib/store.svelte";
  import { pushLayer } from "../lib/layers";
  import { modal } from "../lib/modal";
  import { sheet } from "../lib/motion";
  import { APP_NAME } from "../lib/app";
  import { linkTarget, linkTip, titleOf, whatsNewPage, type Inline, type Shown } from "../lib/whatsnew";
  import { i18n, t } from "../lib/i18n.svelte";

  let { from, shown, onBack, onClose }: { from: "board" | "settings"; shown: Shown; onBack: () => void; onClose: () => void } = $props();

  // Esc: straight to the board; Back (mouse back, Alt+Left, Backspace): where it was opened from.
  $effect(() => pushLayer(() => onClose(), { back: () => onBack() }));

  let title = $derived(titleOf(shown, t));
  let crumbs = $derived<Crumb[]>(from === "settings" ? [{ label: t("Settings"), go: onBack }, { label: title }] : [{ label: title }]);
  let several = $derived(shown.entries.length > 1);
  const target = (href: string) => linkTarget(href, i18n.lang);
  const open = (url: string) => void store.backend.openUrl(url);
</script>

<!-- One line on purpose: a line break between the parts would show as a space. -->
{#snippet line(parts: Inline[])}{#each parts as p, i (i)}{#if p.kind === "bold"}<strong>{p.text}</strong>{:else if p.kind === "code"}<code>{p.text}</code>{:else if p.kind === "link"}{@const url = target(p.href)}{#if url}<button class="link" data-tip={linkTip(url)} onclick={() => open(url)}>{p.text}</button>{:else}{p.text}{/if}{:else}{p.text}{/if}{/each}{/snippet}

<div class="sheet" role="dialog" aria-label={title} tabindex="-1" transition:sheet|global use:modal={{ initial: "self" }}>
  <div class="top">
    <!-- (A step smaller under "Settings ›", so the version still fits at the narrowest panel.) -->
    <ViewHeader {crumbs} {onBack} {onClose} big={from === "settings" ? 18 : 20} />
  </div>

  <div class="page" lang={i18n.lang}>
    {#each shown.entries as e, i (e.version)}
      <section>
        {#if several}<h2>{APP_NAME} {e.version}</h2>{/if}
        {#each e.blocks as b, j (j)}
          {#if b.kind === "heading"}
            <h3>{b.text}</h3>
          {:else if b.kind === "para"}
            <p class:lead={j === 0}>{@render line(b.inline)}</p>
          {:else}
            <ul>
              {#each b.items as it, k (k)}<li>{@render line(it)}</li>{/each}
            </ul>
          {/if}
        {/each}
      </section>
      {#if i < shown.entries.length - 1}<hr />{/if}
    {/each}

    <div class="foot">
      <button class="small strong" onclick={onBack}>{t("Got it")}</button>
      <button class="link quiet" data-tip={whatsNewPage(i18n.lang).replace(/^https:\/\//, "")} onclick={() => open(whatsNewPage(i18n.lang))}>{t("Every version on dobo.sookera.com")}</button>
    </div>
  </div>
</div>

<style>
  .sheet {
    position: absolute;
    inset: 0;
    z-index: 41;
    outline: none;
    overflow-y: auto;
    padding: 0 var(--s6) var(--s6);
    background: var(--ground);
  }
  .top {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 calc(-1 * var(--s6));
    padding: var(--s3) var(--s6);
    background: var(--ground);
  }
  .page {
    max-width: 640px;
    padding-top: var(--s1);
  }
  h2 {
    margin: 0 0 var(--s2);
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.25;
    color: var(--ink);
  }
  /* A version's own parts ("Highlights", "Also"): quiet small headings. */
  h3 {
    margin: var(--s5) 0 var(--s2);
    font-size: var(--t-md);
    font-weight: 600;
    color: var(--soft);
  }
  p,
  li {
    font-size: var(--t-base);
    line-height: 1.6;
    color: var(--ink);
  }
  p {
    margin: 0 0 var(--s3);
  }
  /* The version's one-sentence intro. */
  p.lead {
    font-size: var(--t-lg);
    line-height: 1.55;
    color: var(--soft);
  }
  ul {
    margin: 0 0 var(--s3);
    padding-left: 18px;
  }
  li {
    margin: 0 0 var(--s2);
    padding-left: 4px;
  }
  li:last-child {
    margin-bottom: 0;
  }
  li::marker {
    color: var(--muted);
  }
  strong {
    font-weight: 600;
  }
  code {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.9em;
    padding: 1px 4px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--ink) 7%, transparent);
  }
  hr {
    margin: var(--s6) 0 var(--s5);
    border: 0;
    border-top: 1px solid var(--line);
  }
  .link {
    padding: 0;
    border: 0;
    background: transparent;
    font: inherit;
    color: inherit;
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, currentColor 40%, transparent);
    text-underline-offset: 3px;
    text-decoration-thickness: 1px;
    cursor: pointer;
    border-radius: 3px;
    transition: text-decoration-color 0.15s;
  }
  .link:hover {
    text-decoration-color: currentColor;
  }
  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s3) var(--s4);
    margin: var(--s6) 0 0;
    padding: var(--s4) 0 0;
    border-top: 1px solid var(--line);
  }
  .link.quiet {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .small {
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid var(--ink);
    border-radius: var(--r-md);
    background: var(--ink);
    color: var(--card);
    display: inline-flex;
    align-items: center;
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .small:hover {
    background: color-mix(in srgb, var(--ink) 88%, var(--card));
  }
  .small:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
