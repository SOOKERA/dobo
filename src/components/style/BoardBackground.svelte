<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The board's background: fixed behind the notes (they scroll over it). Textures are small
  // ready-made tiles; a watercolour is painted once per board size into a picture (off-screen,
  // kept, and kept across launches too); your picture is softened and dimmed once. A new look
  // crossfades in (~300 ms) once it is complete.
  import { untrack } from "svelte";
  import { look } from "../../lib/look.svelte";
  import { store } from "../../lib/store.svelte";
  import { paint, textureCss, tileReady, type Painting } from "../../lib/paint.svelte";
  import { baseHex, type Look } from "../../lib/looks";

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");

  interface Layer {
    id: number;
    key: string;
    look: Look;
    css: string;
    canvas: HTMLCanvasElement | null;
    shown: boolean;
  }
  let layers = $state<Layer[]>([]);
  let el: HTMLDivElement;
  let n = 0;
  let w = $state(0);
  let h = $state(0);
  /** The size paintings are made for (settles after a resize stops). */
  let size = $state({ w: 0, h: 0 });
  let painting: Painting | null = null;

  $effect(() => {
    const ro = new ResizeObserver(([e]) => {
      w = Math.round(e.contentRect.width);
      h = Math.round(e.contentRect.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });
  // Repaint only once the window has stopped changing size.
  $effect(() => {
    const W = w;
    const H = h;
    if (!W || !H) return;
    // (The size is read without depending on it: setting it below must not start this again,
    // or a new timer would be armed every 300 ms for ever, the window resting.)
    const was = untrack(() => size);
    if (was.w === W && was.h === H) return;
    const first = was.w === 0;
    const t = setTimeout(
      () => {
        size = { w: W, h: H };
        look.boardW = W;
        look.boardH = H;
      },
      first ? 0 : 300,
    );
    return () => clearTimeout(t);
  });

  function keyOf(l: Look): string {
    const veil = l.bg === "picture" ? (look.dark ? "d" : "l") : "";
    const sz = l.bg === "watercolour" || l.bg === "picture" ? `${size.w}x${size.h}` : "";
    return JSON.stringify(l) + veil + sz;
  }

  /** Your picture: covered, softened and dimmed once into a still picture. */
  async function softPicture(path: string, W: number, H: number, dark: boolean): Promise<HTMLCanvasElement> {
    const img = new Image();
    img.src = store.src(path);
    await img.decode();
    const c = document.createElement("canvas");
    c.width = W;
    c.height = H;
    const g = c.getContext("2d")!;
    const s = Math.max((W + 40) / img.naturalWidth, (H + 40) / img.naturalHeight);
    const iw = img.naturalWidth * s;
    const ih = img.naturalHeight * s;
    g.filter = "blur(7px) saturate(0.85)";
    g.drawImage(img, (W - iw) / 2, (H - ih) / 2, iw, ih);
    g.filter = "none";
    g.fillStyle = dark ? "rgba(22,21,25,0.5)" : "rgba(250,248,243,0.38)";
    g.fillRect(0, 0, W, H);
    return c;
  }

  function show(layer: Layer) {
    layer.shown = true;
    const id = layer.id;
    // Older layers go once the new one has faded in.
    setTimeout(() => (layers = layers.filter((x) => x.id >= id)), reduce.matches ? 160 : 340);
  }

  $effect(() => {
    // (A picture background whose picture can't be shown: a plain board, look.svelte.ts `shown`.)
    const l = look.shown;
    const W = size.w;
    const H = size.h;
    if (!W || !H) return;
    const key = keyOf(l);
    untrack(() => place(l, key, W, H));
  });

  function place(l: Look, key: string, W: number, H: number) {
    if (layers.length && layers[layers.length - 1].key === key) return;
    painting?.cancel();
    painting = null;
    const layer: Layer = { id: ++n, key, look: l, css: baseHex(l), canvas: null, shown: false };

    if (l.bg === "watercolour" && l.wc) {
      // Painted out of sight (or read back from the paintings kept from before), then the whole
      // painting crossfades in at once, like any other look. The board keeps the old look until
      // then: never half a painting, never patches popping in one by one.
      const p = paint(l.wc, W, H, W, H, { priority: 10 });
      painting = p;
      layer.canvas = p.canvas;
      layers = [...layers, layer];
      void p.done.then((ok) => {
        const found = layers.find((x) => x.id === layer.id);
        if (ok && found) requestAnimationFrame(() => show(found));
      });
      return;
    }
    if (l.bg === "picture") {
      layers = [...layers, layer];
      if (!l.picture) return requestAnimationFrame(() => show(layers.find((x) => x.id === layer.id)!));
      void softPicture(l.picture, W, H, look.dark)
        .then((c) => {
          const found = layers.find((x) => x.id === layer.id);
          if (!found) return;
          found.canvas = c;
          requestAnimationFrame(() => show(found));
        })
        .catch(() => {
          const found = layers.find((x) => x.id === layer.id);
          if (found) show(found);
          // (Gone or damaged: the board turns plain, the picture tile imports again.)
          if (l.picture) look.pictureFailed(l.picture);
        });
      return;
    }
    // A texture: wait for its tile, then fade it in.
    layers = [...layers, layer];
    void tileReady(l).then(() => {
      const found = layers.find((x) => x.id === layer.id);
      if (!found) return;
      found.css = textureCss(l);
      requestAnimationFrame(() => show(found));
    });
  }

  function mount(node: HTMLElement, c: HTMLCanvasElement | null) {
    const put = (c: HTMLCanvasElement | null) => {
      node.replaceChildren();
      if (c) node.appendChild(c);
    };
    put(c);
    return { update: put };
  }
</script>

<div class="bg" bind:this={el} aria-hidden="true">
  {#each layers as layer, i (layer.id)}
    <div class="layer" class:shown={layer.shown || i === 0} style:background={layer.css} use:mount={layer.canvas}></div>
  {/each}
</div>

<style>
  .bg {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    border-radius: inherit;
    pointer-events: none;
    contain: strict;
  }
  .layer {
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity 0.3s ease;
    will-change: opacity;
  }
  .layer.shown {
    opacity: 1;
  }
  .layer :global(canvas) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }
  @media (prefers-reduced-motion: reduce) {
    .layer {
      transition-duration: 0.14s;
    }
  }
</style>
