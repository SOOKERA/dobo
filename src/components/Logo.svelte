<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Dobo's logo: the mark (the true logo, the hand-drawn Dobo note: a task with its steps' trace,
  // a ticked one, the ideas band with its folded corner) and the hand-lettered wordmark "Dobo".
  // The wordmark is drawn inline, in the ink of the moment. The note is a picture (its drawing,
  // art/logo/mark.svg, is 900 KB): made by art/icons.mjs at each size used here, for screens at
  // 100 % to 300 % (public/logo). An ink edge on light grounds, a paper edge on dark ones
  // (mark-dark.svg), as the icon on a dark taskbar. The name is said once, for screen readers.
  import { APP_NAME } from "../lib/app";

  // `size`: the mark's height (the note is 30 of it, as tall as the wordmark's tall strokes);
  // `wordSize`: the wordmark's height when it isn't 80 % of the mark's.
  let { mark = true, word = true, size = 28, wordSize }: { mark?: boolean; word?: boolean; size?: number; wordSize?: number } = $props();
  // The round letters sit level with the note's middle.
  let wordH = $derived(wordSize ?? Math.round(size * 0.8));

  // The sizes art/icons.mjs draws (MARK_SIZES there), each with its box's width: the note's,
  // and a hair. Another size takes the next bigger drawing, scaled.
  const DRAWN: [number, number][] = [
    [20, 18],
    [22, 20],
    [32, 28],
  ];
  let drawn = $derived(DRAWN.find(([s]) => s >= size) ?? DRAWN[DRAWN.length - 1]);
  let markW = $derived((size * drawn[1]) / drawn[0]);
  const srcset = (look: string, s: number) => [1, 1.5, 2, 3].map((k) => `/logo/mark-${look}-${s}@${k}x.webp ${k}x`).join(", ");
</script>

<span class="logo" role="img" aria-label={APP_NAME} style:--mark="{size}px" style:--mark-w="{markW}px" style:--word="{wordH}px">
  {#if mark}
    <!-- Both looks, the one of the moment on top (a cross-fade when the look changes). -->
    <span class="mark" aria-hidden="true">
      <img class="light" src="/logo/mark-light-{drawn[0]}@1x.webp" srcset={srcset("light", drawn[0])} alt="" draggable="false" />
      <img class="dark" src="/logo/mark-dark-{drawn[0]}@1x.webp" srcset={srcset("dark", drawn[0])} alt="" draggable="false" />
    </span>
  {/if}
  {#if word}
    <!-- "Dobo", lettered by hand with the mark's pen (design/logo wordmark-header.svg: the
         wordmark's outline within a tenth of a unit). The D in the weight of the "d" it took
         over from (a touch heavier pen, its stem steady), its top level with the b's. -->
    <svg class="word" viewBox="-3 -110 290 116" aria-hidden="true"><path d="M.7-94.7l-1.4-1.4-.4-.8 0-.9.9-2.9 1.1-2.1 4.1-2.7.8-.3 4.2 4.3.2 3.9.6 4.9-.1 4-.5 3.9.3 4-.1 4.9-.6 3-.2 2.9-.7 4 .1 2 1.2 7.8-.3 3 .3 4-.1 8.9.4 3.9-.9 8-.1 2 .2 1.9-.2 4 .7 3.9-.1 9 .7 2.9-.4 1.9-.5.8-1.3 1.5-.8.5-.9.1-3.1-.1-1.7-.6-.7-.6-1.2-1.7-.3-.9.5-3.9-.5-3.9 0-4 .6-5-.2-2.9.4-5-.3-3.9-.9-3.9 0-1 .2-2-.2-9 .1-4.9-.3-2 0-2 .5-1.9.7-6-.6-3.9.2-4-.2-2.9 0-6 .3-3-.6-3.9 0-6 .4-3.9-.1-2 .5-.3.7-.1 2.8 1.1 1 .2 3.9-.7.3.5 0 1.7-.1 1-.4.9-1.1 1.6-.8.7-.9.4-2.8.9zm2.1-2.9l0-2.5.6-.5.8 0 2.2-.6 4.6-1.6 1.4-.2 2.9 0 4.3.4 8.6 0 4.6.6 5.1 1.5 3.4 1.6 2.6.8 4 2 6.2 4.6 4.4 4 3.7 4 3.1 4.2 3.2 7.1 1.1 3 .9 4.9 0 2.6.6 3.7-.1 6.4.5 4.4.1 2.7-.5 6.2-.6 3.1-1.7 6.1-1.1 6.2-.9 2.2-2.6 4.9-1.3 2.7-3.1 4.7-5 4.9-3.6 2.5-5.9 3.1-4.5 1.7-7.5 1.4-14 0-3.6-.6-2.3-.7-5.6-2.3-1.3-.4-.8 0-.5-.4 0-1.1.6-2.4.6-.2.8.2 5 .4 5.8-1.1 2.7-.2 5.5 0 4-.5 4.9-.9 2.3-.6 1.1-.5 2.9-.7 3-1.6 1.9-2.2 5.3-4.4 2.7-3 2.6-3.6 2.5-6.6.6-1.9.2-1.4.3-5.5.4-3.5-.3-5.4.5-4 0-2.9-.4-4-.1-3.1-1-10.8-.7-1.7-1-1.9-1.9-4.9-.2-1.2-2.6-4.2-3.1-3.4-.9-.8-1.8-1.1-.8-.8-1.1-.2-2.5-1.1-1.7-.4-2.7-.9-6.9-1.1-1.1.2-4 0-2.8-.3-4 .2-3.3.6-4.8.5-1.5-.2-5.7-1.5-.7.2-.7-.5zm104.5 31.4l3.4.1 5.8 1.7 2.2 1.1 2.6 2.1 4.6 2.8 2.4 2 3.8 4 2.6 4 1.3 2.8 1 3 1.2 6.8.3 2.3-.1 1.3.3 2.6-.2 3.8-.8 4.5-.9 3.1-1.2 2.9-1.8 4-2.9 5.6-3.1 4-3 2.2-2.3 1.3-2.4 1-3.1.9-1.8.3-3.5 0-1.4.2-1.7-.1-3.3-.6-3.2-1.1-1.5-.7-3.9-2.4-6.2-4.3-3.7-2.9-1.2-1.2-2.3-3-2.3-3.6-1.8-6-.7-4.5-.6-7.5.1-3.9.4-2.7 1.7-5.3 2.1-4.2 2-3.3 3.6-4.4 2.9-2.7 5.7-3.2 5.9-1.9 2.5-.6.9 0zm-4.2 9.6l-2 1.1-4.1 2.9-1.8 1.7-2.6 3-2 3-2 4.3-1.7 4.3-.6 3.6-.2 3.1.3 2.1 0 1.8.4 1.3.1 1.4.5 2.3.5 1.3.4 2.4.7 2.2 2.5 4.6 2.7 2.9 3 2.6 1.3.3 2.6 1 1.2.2 2.5 0 3-.4 1.6-.5 2.1-.3 1.1-.3 1.2-.8 2.3-1 2.4-1.5 1.9-1.5 1.1-1.2 1.1-.9 1.2-1.4 2.2-3.2.7-1.8.9-1.7 1.5-3.9 1-4.1.2-2 .4-1-.1-6.3-1.3-6.2-1.2-3.5-1.7-3.4-3.3-4.2-1.7-1.9-2.2-1.7-.6-.2-6.3-.8-4 .6zm48.2-41.9l-.7-.6-.4-.7-.1-2 .5-3.3 1.5-1 8 .6.4.5.1.8.3 4-.3 5.9.1 2-1.5 9.9.7 9.9 1.1 5.9-.1 1.9-.6 4 .2 4 .6 4.9-.3 2.9-.1 4-.4 1.9.1 3-.3 3 .6 4.9-.2 3 .2 3 .8 6.9-.3 3.9.3 4-.3 5-.5 1.9-.4.9-2.2 3.4-.8.2-2.9-.2-.7-.6-1-1.6-.9-1.8-.5-3 .1-4.9.4-5-.2-2 .5-3.9-.8-8 .2-1.9.7-2.9.2-2 0-3.9-.4-3-.1-5-.4-2.9-.3-9-.5-4.9.5-3-.2-2.9.1-6 .6-3.9-.1-2-.6-4 .2-3.9.6-4-.4-7.9.1-.8.3-.6 7-.8 1-.1.7.2.5.5.1 1-.3 4-.2 1-.6.8-1.4 1.4-.9.4-3.9.3zm31.6 32.7l3.2.6 5.2 1.8 3.9 2.8 2.2.9 2.6 1.8 1.2 1.1 2.1 2.1 4.3 5.9 3.5 5.9 1.7 4.6.2.9 0 1.2.5 2.6 0 3.2-.3 5.3-1 6.2-1.2 4-1 2.3-1.6 2.9-3.8 4.9-4.2 3.8-1.6 1.1-3.7 2-1.8.6-2.6.4-2.6.2-4.7-.2-7.5-1.9-2.2-1.2-3.2-2-4.3-2.2-3.4-3.2-2.3-3.1-2.9-4.6-2.1-4.5-1.6-6.6-.2-8.2.6-6.2 1-4.8 1.2-3 1.5-2.4 3.3-4.4 5.7-5.5 2.1-1.6 2.9-1.7 4.1-1.5 5-.4zm-3.7 7.4l-1.9 1-1.2.4-3 1.8-3.5 3.4-1.1 1.5-2.7 5.3-1.1 3.5-1 8.6.2 1.5 0 3.9-.2.9.7 7.3.6 2.5 2.7 5.8 1.5 2.1 1.2 2.2 1.8 1.8 3.1 1.3 1.8.3 2.6.7 2.6 0 2.3-.5 2.1-.2 3.1-1.4 3.7-2.6 3.4-3.8 4.4-7.2 2.1-5.6.6-2.3.1-1.2.2-6.1-.5-2.7.1-1-1.2-5.8-1.6-3.6-4.5-7.2-2.6-3.5-1.8-1-1.2-.3-1.8-.1-2.7-.7-3.6.3zm72.8-6.4l1.4 0 1.6.4 6.9 2.6 1.4.8 2.4 1.8 3.4 1.8 4.1 3.2 3 3.3 4 5.7 2.6 5 1.3 4 .6 2.7-.2 6.3-.5 2.6-.7 2-.3 1.5-1.9 5.2-1.7 3.6-2.8 4.3-2.2 2.5-2.3 2.1-3.8 2.8-1.6.9-3.2 1.3-3.7 1.1-2.6.5-3.4-.2-3-.5-5.7-1.4-4.2-2.1-3.2-2.1-2.2-1-2.5-1.8-2.2-2.1-3.9-5-3.1-6.9-1.7-5.9-.6-3.8-.3-4.1.1-1.6.5-4.6.9-4.9 1.6-4.1.8-1.6 1.3-2.1 1.6-2 1.2-1.3 4.4-3.6 3.3-2 3.6-1.7 1.6-.6 3.4-.8 2.9-.5 1.9 0zm-1.5 6.4l-.8.5-3.1.7-1.4.6-.9.7-1.8.9-3.2 2.7-3.2 3.6-2.9 4.3-1.6 3.5-.9 5 0 1.5-.3 1.7 0 3 .5 4.9.9 3.6.9 2.4.9 1.8.6 1.7 1.7 3.3 3.5 5.3 2.1 1.6 3.6 1.1 1.8.2 2.1.5 4.3.1 1.8-.2 1-.3 4-1.5 2-1.1 3.9-3.5 3.2-3.8 1.5-2.3 1.3-2.9 1.2-4.6.2-2 .5-3 .4-6.1.3-.9-.5-4.3-1.5-4.8-1.8-3.4-1.5-2.2-.9-1.6-3.3-4-2.3-1.4-2.4-.4-2.5-.7-2.5-.3-3.7.1z" /></svg>
  {/if}
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: center;
    /* (From the wordmark: the header's 32 px mark keeps the gap it had at 28.) */
    gap: calc(var(--word) * 0.38);
    flex-shrink: 0;
    line-height: 0;
  }
  .mark {
    position: relative;
    isolation: isolate;
    width: var(--mark-w);
    height: var(--mark);
    flex-shrink: 0;
  }
  .mark img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    user-select: none;
    -webkit-user-drag: none;
  }
  /* The look coming in fades in on top; the one going out leaves once it is covered. */
  .light,
  :global(:root[data-mode="dark"]) .dark {
    opacity: 1;
    z-index: 1;
    transition: opacity 0.32s ease;
  }
  .dark,
  :global(:root[data-mode="dark"]) .light {
    opacity: 0;
    z-index: 0;
    transition: opacity 0s linear 0.32s;
  }
  .word {
    height: var(--word);
    width: calc(var(--word) * 290 / 116);
    flex-shrink: 0;
    fill: var(--ink);
    transition: fill 0.32s ease;
    /* Its letters' round part sits level with the note's middle (optically: the box holds the
       tall strokes above them). */
    translate: 0 calc(var(--word) * -0.12);
  }
</style>
