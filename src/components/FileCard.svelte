<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A file on this computer kept with a line: its name and folder; a click opens it.
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { t, words, type Words } from "../lib/i18n.svelte";

  let { path }: { path: string } = $props();
  const uid = $props.id();
  let parts = $derived(path.split(/[\\/]/));
  let name = $derived(parts[parts.length - 1] || path);
  let folder = $derived(parts.slice(0, -1).slice(-2).join("\\"));
  let ext = $derived((name.split(".").pop() ?? "").slice(0, 4).toUpperCase());
  let error = $state<Words>("");

  async function open() {
    try {
      error = "";
      await store.backend.openPath(path);
    } catch (e) {
      error = () => t("Couldn't open it (moved or deleted?)");
      void e;
    }
  }
</script>

<button class="file" onclick={open} data-tip={path} aria-describedby="{uid}-path">
  <span class="badge" aria-hidden="true"><Icon name="file" size={18} /><span class="ext">{ext}</span></span>
  <span class="words">
    <span class="name">{name}</span>
    <span class="folder">{words(error) || folder}</span>
  </span>
</button>
<!-- The whole path (the tooltip shows it), for screen readers too. -->
<span class="visually-hidden" id="{uid}-path">{path}</span>

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
</style>
