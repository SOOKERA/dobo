<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A note's pictures (Pictures board): one large, arrows and "2 / 3" on mouse-over, dots
  // under it. Double-click opens the viewer.
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { openViewer } from "../lib/viewer.svelte";
  import type { Picture } from "../lib/types";
  import { t } from "../lib/i18n.svelte";
  import { blip, fold } from "../lib/motion";

  let { pictures }: { pictures: Picture[] } = $props();
  let i = $state(0);
  let index = $derived(Math.min(i, pictures.length - 1));
  let dir = $state(1);

  function go(d: number) {
    dir = d;
    i = (index + d + pictures.length) % pictures.length;
  }
</script>

{#if pictures.length}
  <div class="pics" transition:fold={{ duration: 320 }}>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="stage"
      role="group"
      aria-label={t("Pictures, {a} of {b}", { a: index + 1, b: pictures.length })}
      tabindex="0"
      ondblclick={() => openViewer(pictures, index)}
      onkeydown={(e) => {
        if (e.key === "ArrowRight") go(1);
        if (e.key === "ArrowLeft") go(-1);
        if (e.key === "Enter") openViewer(pictures, index);
      }}
    >
      {#key pictures[index].id}
        <img src={store.src(pictures[index].path)} alt="" draggable="false" style:--from="{dir * 18}px" />
      {/key}
      {#if pictures.length > 1}
        <button class="nav prev" aria-label={t("Previous picture")} onclick={() => go(-1)}><Icon name="chev-left" size={16} stroke={2.2} /></button>
        <button class="nav next" aria-label={t("Next picture")} onclick={() => go(1)}><Icon name="chev-right" size={16} stroke={2.2} /></button>
        <span class="count">{index + 1} / {pictures.length}</span>
      {/if}
    </div>
    {#if pictures.length > 1}
      <div class="dots" aria-hidden="true" transition:fold={{ y: 0, duration: 240 }}>
        {#each pictures as p, k (p.id)}
          <span class:on={k === index} in:blip out:blip></span>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .pics {
    padding: 0 var(--s3) var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .stage {
    position: relative;
    height: 150px;
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--chip);
    cursor: zoom-in;
    outline-offset: 2px;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: slide 0.32s var(--ease-out);
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateX(var(--from));
    }
  }
  @media (prefers-reduced-motion: reduce) {
    img {
      animation: none;
    }
  }
  .nav {
    position: absolute;
    top: 50%;
    width: 36px;
    height: 36px;
    margin-top: -18px;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 254, 251, 0.92);
    color: #1f1d1a;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
    transform: scale(0.9);
    transition: opacity 0.15s, transform 0.15s;
  }
  .prev {
    left: 8px;
  }
  .next {
    right: 8px;
  }
  .stage:hover .nav,
  .stage:focus-within .nav {
    opacity: 1;
    transform: none;
  }
  .count {
    position: absolute;
    right: 8px;
    top: 8px;
    padding: 3px 8px;
    border-radius: 10px;
    background: rgba(20, 18, 16, 0.62);
    color: #fff;
    font-size: var(--t-xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .stage:hover .count {
    opacity: 1;
  }
  .dots {
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .dots span {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--edge);
    transition: width 0.25s var(--ease-out), background-color 0.25s;
  }
  .dots span.on {
    width: 16px;
    background: var(--ink);
  }
</style>
