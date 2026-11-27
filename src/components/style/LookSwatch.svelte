<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A look, small: every chip, swatch and card of the Style panel shows the real thing — the
  // texture's own tile, the watercolour painted at the board's shape (or at its own size for
  // paints), your picture softened. Never a flat stand-in colour.
  import { look as lookState } from "../../lib/look.svelte";
  import { store } from "../../lib/store.svelte";
  import { swatchPaint, textureCss } from "../../lib/paint.svelte";
  import { baseHex, type Look } from "../../lib/looks";
  import Icon from "../Icon.svelte";

  let {
    look: l,
    w,
    h,
    layout = "board",
    card = null,
  }: {
    look: Look;
    /** Its size on screen (CSS px). */
    w: number;
    h: number;
    /** A painting laid out at the board's shape (shrunk), or at the swatch's own size. */
    layout?: "board" | "own";
    /** A little note card on it (the style cards). */
    card?: { left: number; top: number; w: number; h: number } | null;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let ready = $state(false);
  let shownKey = "";

  // The board's paper for the mini card: dark cards on dark boards are the dark mode's own.
  let cardBg = $derived(lookState.dark ? "#2A292F" : "#FFFEFB");

  $effect(() => {
    if (l.bg !== "watercolour" || !l.wc || !canvas) return;
    const wc = l.wc;
    // Drawn at the board's shape (or its own), covering the swatch.
    const sp = swatchPaint(wc, w, h, layout === "own", lookState.boardW, lookState.boardH);
    const { key, outW, outH } = sp;
    const c = canvas;
    // The same painting again (the panel redrew): nothing to do.
    if (key === shownKey) return;
    const p = sp.paint(5);
    let alive = true;
    void p.done.then((ok) => {
      if (!alive || !ok) return;
      c.width = outW;
      c.height = outH;
      c.getContext("2d")!.drawImage(p.canvas, 0, 0);
      shownKey = key;
      ready = true;
    });
    return () => {
      alive = false;
    };
  });
</script>

<span class="sw" style:width="{w}px" style:height="{h}px" style:background={l.bg === "watercolour" ? baseHex(l) : textureCss(l)} aria-hidden="true">
  {#if l.bg === "watercolour"}
    <canvas bind:this={canvas} class:ready></canvas>
  {:else if l.bg === "picture" && lookState.usable(l.picture)}
    {@const path = l.picture}
    <img src={store.src(path)} alt="" onerror={() => lookState.pictureFailed(path)} />
  {:else if l.bg === "picture" && l.picture}
    <!-- (Outside Dobo's folders, or gone: its place, never a broken picture.) -->
    <span class="pic-ph ph"><Icon name="image" size={Math.round(Math.min(18, Math.max(10, h * 0.42)))} stroke={1.8} /></span>
  {:else if l.bg === "picture"}
    <span class="pic-empty"></span>
  {/if}
  {#if card}
    <span class="mini" style:left="{card.left}px" style:top="{card.top}px" style:width="{card.w}px" style:height="{card.h}px" style:background={cardBg}></span>
  {/if}
</span>

<style>
  .sw {
    position: relative;
    display: block;
    overflow: hidden;
    flex: none;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.25s ease;
  }
  canvas.ready {
    opacity: 1;
  }
  img {
    position: absolute;
    inset: -4px;
    width: calc(100% + 8px);
    height: calc(100% + 8px);
    object-fit: cover;
    filter: blur(1.5px) saturate(0.85);
    opacity: 0.85;
  }
  .ph {
    position: absolute;
    inset: 0;
  }
  .pic-empty {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, #8fb0c9, #ead8c2 60%, #86a08b);
    filter: blur(2px);
  }
  .mini {
    position: absolute;
    border-radius: 3px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.22);
  }
</style>
