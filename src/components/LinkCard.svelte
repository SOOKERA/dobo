<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A web page kept with a line, shown like a link shared in a message: picture, title, site.
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import type { LinkPreview } from "../lib/types";
  import { imgFade } from "../lib/motion";

  let { link, wide = false }: { link: LinkPreview; wide?: boolean } = $props();
  let broken = $state(false);
</script>

<button class="link" class:wide class:has-image={link.image && !broken} onclick={() => store.backend.openUrl(link.url)} data-tip={link.url}>
  {#if link.image && !broken}
    <img src={store.src(link.image)} alt="" loading="lazy" draggable="false" use:imgFade onerror={() => (broken = true)} />
  {/if}
  <span class="words">
    <span class="title">{link.title || link.url}</span>
    <span class="site"><Icon name="link" size={11} stroke={2.2} />{link.site}</span>
  </span>
</button>

<style>
  .link {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 320px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    overflow: hidden;
    background: var(--ground);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s var(--ease-out);
  }
  .link:hover {
    border-color: var(--box);
  }
  .link:active {
    transform: scale(0.99);
  }
  .link.wide {
    max-width: none;
  }
  img {
    width: 100%;
    aspect-ratio: 1.91 / 1;
    object-fit: cover;
    display: block;
    background: var(--chip);
    opacity: 0;
    transition: opacity 0.3s var(--ease-out);
  }
  img:global(.loaded) {
    opacity: 1;
  }
  img:global(.instant) {
    transition: none;
  }
  .words {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px 9px;
  }
  .title {
    font-size: var(--t-md);
    font-weight: 600;
    line-height: 1.3;
    color: var(--ink);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .site {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--t-xs);
    color: var(--muted);
  }
</style>
