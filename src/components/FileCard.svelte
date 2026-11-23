<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A file kept with a line: its name and folder; a click opens it. A file on another computer
  // (a share, a NAS) asks once per computer first: opening it sends the Windows sign-in there
  // (src-tauri/src/files.rs). When it can't open, the card says why.
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { fold } from "../lib/motion";
  import { takeFocus } from "../lib/modal";
  import { i18n, t } from "../lib/i18n.svelte";
  import { untrack } from "svelte";

  let { path }: { path: string } = $props();
  const uid = $props.id();
  let parts = $derived(path.split(/[\\/]/));
  let name = $derived(parts[parts.length - 1] || path);
  let folder = $derived(parts.slice(0, -1).slice(-2).join("\\"));
  let ext = $derived((name.split(".").pop() ?? "").slice(0, 4).toUpperCase());
  /** Why it didn't open (Rust's words, in the language of that moment: another language since,
   * and it goes, the folder back in its place). */
  let error = $state<{ said: string; lang: string } | null>(null);
  let why = $derived(error?.lang === i18n.lang ? error.said : "");
  // (Gone for good: switching back doesn't bring old words back.)
  $effect(() => {
    const lang = i18n.lang;
    untrack(() => {
      if (error && error.lang !== lang) error = null;
    });
  });
  /** The computer to ask about before its files open. */
  let ask = $state<string | null>(null);
  let cardEl = $state<HTMLButtonElement>();
  const ASK_HOST = "ask-host:";

  async function open() {
    try {
      error = null;
      await store.backend.openPath(path);
    } catch (e) {
      const said = String(e);
      if (said.startsWith(ASK_HOST)) ask = said.slice(ASK_HOST.length);
      else error = { said, lang: i18n.lang };
    }
  }
  /** Allow: remembered for that computer (the setting `file_hosts`), then the file opens. */
  async function allow() {
    const host = ask;
    if (!host) return;
    ask = null;
    const hosts = Array.isArray(store.settings.file_hosts) ? (store.settings.file_hosts as string[]) : [];
    if (!hosts.includes(host)) await store.setSetting("file_hosts", [...hosts, host]);
    cardEl?.focus();
    await open();
  }
  function notNow() {
    ask = null;
    cardEl?.focus();
  }
</script>

<button class="file" bind:this={cardEl} onclick={open} data-tip={path} aria-describedby="{uid}-path" aria-expanded={ask ? true : undefined}>
  <span class="badge" aria-hidden="true"><Icon name="file" size={18} /><span class="ext">{ext}</span></span>
  <span class="words">
    <span class="name">{name}</span>
    <span class="folder" class:error={!!why}>{why || folder}</span>
  </span>
</button>
<!-- The whole path (the tooltip shows it), for screen readers too. -->
<span class="visually-hidden" id="{uid}-path">{path}</span>
{#if ask}
  <!-- Once per computer: its files open from the network. (Esc: Not now.) -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="ask" role="group" aria-labelledby="{uid}-ask" transition:fold onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), notNow())}>
    <p id="{uid}-ask">{t("Files on {host} open from the network. Allow?", { host: ask })}</p>
    <span class="choices">
      <button class="small" use:takeFocus onclick={notNow}>{t("Not now")}</button>
      <button class="small strong" onclick={allow}>{t("Allow")}</button>
    </span>
  </div>
{/if}

<style>
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    max-width: 320px;
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--ground);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .file:hover {
    border-color: var(--box);
  }
  .badge {
    position: relative;
    flex-shrink: 0;
    width: 30px;
    height: 34px;
    display: grid;
    place-items: center;
    color: var(--icon);
  }
  .ext {
    position: absolute;
    bottom: 1px;
    left: 50%;
    transform: translateX(-50%);
    padding: 0 3px;
    border-radius: 3px;
    background: var(--ink);
    color: var(--card);
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .words {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-size: var(--t-md);
    font-weight: 600;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .folder {
    font-size: var(--t-xs);
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Why it didn't open: whole, in the late ink. */
  .folder.error {
    white-space: normal;
    overflow-wrap: anywhere;
    color: var(--late-ink);
  }
  .ask {
    max-width: 320px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
  }
  .ask p {
    margin: 0;
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--soft);
    overflow-wrap: anywhere;
  }
  .choices {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 8px;
  }
  .small {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    font-size: var(--t-sm);
    font-weight: 500;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background-color 0.15s;
  }
  /* (As the card's own border, on hover; the inked one lightens a touch, as elsewhere.) */
  .small:not(.strong):hover {
    border-color: var(--box);
  }
  .small.strong {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .small.strong:hover {
    background: color-mix(in srgb, var(--ink) 86%, var(--card));
  }
  .small:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
