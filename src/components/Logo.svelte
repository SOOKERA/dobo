<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Dobo's logo: the mark (the Dobo note: a task with its steps' trace, a ticked one, the
  // ideas band with its folded corner) and the hand-lettered wordmark "dobo". Drawn inline, so it
  // is sharp at every scale and follows the theme: the wordmark is in the ink of the moment; the
  // note stays paper, with an ink edge on light grounds and a paper edge on dark ones (as the
  // icon on a dark taskbar). The name is said once, for screen readers.
  import { APP_NAME } from "../lib/app";

  // `wordSize`: the wordmark's height when it isn't 80 % of the mark's (the header shows the mark
  // at 32 px, its drawing's own grid, so it is crisp at 100 % and 200 %, with the wordmark it had).
  let { mark = true, word = true, size = 28, wordSize }: { mark?: boolean; word?: boolean; size?: number; wordSize?: number } = $props();
  // The wordmark's tall strokes (d, b) reach the note's top; its round letters sit level with
  // the note's middle.
  let wordH = $derived(wordSize ?? Math.round(size * 0.8));
</script>

<span class="logo" role="img" aria-label={APP_NAME} style:--mark="{size}px" style:--word="{wordH}px">
  {#if mark}
    <!-- The 32 px drawing (shapes on whole pixels: design/logo icon-32.svg). -->
    <svg class="mark" viewBox="0 0 32 32" aria-hidden="true">
      <path class="edge" d="M16 1H27A4 4 0 0 1 31 5V27A4 4 0 0 1 27 31H5A4 4 0 0 1 1 27V5A4 4 0 0 1 5 1Z" />
      <path d="M16 3H27A2 2 0 0 1 29 5V27A2 2 0 0 1 27 29H5A2 2 0 0 1 3 27V5A2 2 0 0 1 5 3Z" fill="#FFFCF4" />
      <path d="M3 22H23L29 28V27A2 2 0 0 1 27 29H5A2 2 0 0 1 3 27Z" fill="#D3C9B6" />
      <path d="M23 22L29 28H23Z" fill="#221E1A" />
      <rect x="6" y="25" width="10" height="2" rx="1" fill="#FFFCF4" />
      <path d="M12 5.5H14.5A4 4 0 0 1 18.5 9.5V14.5A4 4 0 0 1 14.5 18.5H9.5A4 4 0 0 1 5.5 14.5V9.5A4 4 0 0 1 9.5 5.5Z" fill="none" stroke="#C2BBAF" stroke-width="1" opacity="0.75" />
      <path d="M12 5.5H14.5A4 4 0 0 1 18.5 9.5V14.5A4 4 0 0 1 14.5 18.5H9.5A4 4 0 0 1 5.5 14.5V9.5A4 4 0 0 1 9.5 5.5Z" fill="none" stroke="#221E1A" stroke-width="1" pathLength="100" stroke-dasharray="66 100" />
      <rect x="9" y="9" width="6" height="6" rx="1.2" fill="none" stroke="#221E1A" stroke-width="2" />
      <polyline points="21.5 11 23.6 13.2 27 8.8" fill="none" stroke="#221E1A" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      <rect x="21" y="17" width="6" height="2" rx="1" fill="#9E978B" />
    </svg>
  {/if}
  {#if word}
    <!-- "dobo", lettered by hand with the mark's pen (design/logo wordmark.svg, its outline
         simplified to within a quarter of a unit). -->
    <svg class="word" viewBox="-3 -110 290 116" aria-hidden="true"><path d="M26.5-65.4L24.6-65.6 19.3-65.1 14.8-63.3 8.9-59.6 5.1-56.1 2-51.7 .2-46.9-1.1-40.9-1.5-31.6-.6-25.2 2.5-16.7 4.7-12.6 8.5-7.7 13-3.9 15.8-2.6 19.5 0 23.7 2.2 28.3 3.4 31.7 3.8 35.4 3.6 37 3.1 40 2 44.3-.3 49.5-4.4 52.7-8.1 55.4-12.3 57.3-17.2 58.9-24.9 59.2-29.8 58.5-34.8 57.3-39.2 54.1-46.1 49.1-53.5 44.3-57.4 36.7-62 29.5-65.1ZM27.8-58.9L32.7-59.2 37.7-57.6 39.9-56.2 43.5-52.5 47.1-44.5 48.7-38.1 49.5-30.8 49.4-27.3 47.6-20.3 43.7-13.3 40.7-9.9 37.9-7.7 30.7-3.6 28.5-3.1 23.6-2.8 18.8-4.5 16.5-6.1 14.8-8.1 11.2-13.2 9.2-17.9 7.9-22.8 7.5-29 9.5-38.6 12.4-46.6 14.3-50.4 16.5-53.1 20-56 23.1-57.7ZM53.7-96.5L57.5-97.6 59.1-98.8 60.2-100.3 60.3-102.3 59.9-103.9 53.9-104.1 52.5-103.7 53.1-99.7 52-93.9 50.7-80 51.3-74.1 50.6-64.1 51.9-52.3 51.9-48.3 50.9-40.4 50.7-34.4 51.4-22.5 51.1-18.5 52.6-8.7 52.3-2.7 52.6-.8 58.5-.1 60.2 .9 59.4 1.9 58.1 1.3 62 2.2 63.9 1.9 65.6 .9 68.4-2 66.1-5.2 62.5-4.9 60.7-4.2 55.1-5 53.3-4.7 51.7-1.1 53.1 2.5 54.8 3.1 60.7 3.5 60.4-6.4 61.1-18.3 59.9-32.2 60.7-40.1 59.6-50 59.6-59.9 60.4-63.8 60.7-69.8 60.1-75.7 60.9-79.6 61.2-87.6 60.5-103.5 60-105.3 57.1-108 51.4-105.9 50.2-100.8 50.7-99.2ZM105.7-65.9L102.3-65.3 96.4-63.4 93.6-61.9 90.7-60.2 85.2-54.4 81.8-49.1 78.8-41.8 78.1-38.6 78.1-30.6 79.2-21.7 81-15.7 85.6-9.1 90.5-5 100.6 1.7 105.3 3.5 108.6 4.1 111.7 4 117 3.7 123.2 1.4 127.8-1.7 130.9-5.7 133.8-11.3 137.3-19.8 138.5-25.8 138.5-33.5 137-42.6 136-45.6 134.7-48.4 132.1-52.4 128.9-55.8 126.5-57.9 117.2-64.1 110.7-66.1ZM108.3-58.3L111.6-58.9 116.7-58.4 119.2-57.9 121.4-56.2 126.4-50.1 128.1-46.7 129.9-40.8 130.7-35.8 130.7-30.7 129.1-23.6 127.4-19.2 125-14.7 122.9-11.9 118-7.7 112.1-4.5 108.3-3.7 104.8-3.2 101.1-3.4 97.2-4.7 93.2-8.2 91.5-10.2 89-14.8 86.9-23 86.1-29.6 87.1-36.7 89.8-43.4 92.3-47.5 97-52.6 101.5-55.8ZM153.8-96.7L157.7-97 159.4-98.1 160.6-99.6 161.1-103.6 161-105.6 159.8-106.3 151.8-105.4 151.4-104 151.8-96.1 151-88.2 151.7-82.2 151.1-78.3 151.2-70.4 150.7-66.4 152.4-36.7 151.3-30.9 152.1-22.9 151.6-19 151.8-17 151.3-7.1 152.2-3.2 153.2-1.5 154.4-.1 156.3 0 158.1-.1 160.3-3.5 161-5.3 161.5-11.3 161.2-15.3 161.5-19.2 160.6-27.1 160.6-33.1 160.1-37 161.1-50.8 160.3-60.7 161-66.6 159.9-72.5 159.2-82.4 160.7-92.3 160.9-100.2 160.6-104.2 160.1-105.5 152.1-106.1 150.5-104.7 150.1-100.8 150.6-99.1ZM181.1-65.9L176.1-65.5 171.2-63.6 168.4-61.8 164.7-58.6 159.2-52.6 156.5-48.4 154.8-43.5 153.7-34.4 153.8-27.2 155.5-19.6 157.6-15.1 161.8-8.7 165-5.3 166.2-4.2 175.9 1.2 183.4 3.1 186.4 3.2 191.5 3 195.1 2.1 201.1-1.6 204.6-4.8 206.6-7.2 210-12.6 211.8-17.2 213.2-25.1 213.6-32.7 213-37.4 211.5-42.1 207.6-48.8 203.3-54.7 200-57.9 197.4-59.7 194.6-61 192-63 186.1-65.2ZM182.9-59.1L186.5-59.4 191.6-58.5 194-57.3 199.8-48.9 202.7-43 204.3-33.5 204-26.2 203.4-23.9 201.3-18.3 196.9-11.1 193.1-7 190.3-5 186.1-3.1 183.4-2.8 179.7-2.6 175.9-3.4 172.2-4.9 170.4-6.7 167.7-11 165-16.8 164.2-20.4 163.7-26.6 163.7-32.9 164.7-41.5 166-45.4 168.2-49.9 169.6-51.8 173.1-55.2 179.2-58.4ZM250.3-65.1L245.5-64.6 240.5-63.2 233.6-59.5 228.5-55.2 225.5-51.2 222.7-44.8 221.3-35.3 221.4-30.5 222.9-22.7 225-17 226.9-13 230.8-8 235.5-4.1 245.1 1.1 250.8 2.5 255.5 3.1 259 2.9 266.7 .3 272.1-3.4 276.6-8 279.4-12.3 283-21.1 284.5-27.2 284.7-33.5 282.8-40.2 280.6-44.5 276.2-50.9 273.2-54.2 269.7-57 261.9-61.8 255-64.4ZM251.7-58.4L257.9-58.2 262.8-57.1 265.1-55.7 268.4-51.7 272.9-43.9 274-40.3 274.6-35.4 273.2-23.4 272-18.8 269.2-13.6 266-9.8 261.6-6 256.1-3.7 252.7-3.2 249-3.3 245.1-4 241.5-5.1 239.4-6.7 235.9-12 232.7-18.8 230.9-24.8 230.4-29.7 230.7-35.9 231.6-40.9 233.4-44.8 236.1-48.7 239.3-52.3 243-55.3 247.2-57.4 250.5-58.4Z" /></svg>
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
    width: var(--mark);
    height: var(--mark);
    flex-shrink: 0;
  }
  .edge {
    fill: #221e1a;
    transition: fill 0.32s ease;
  }
  :global(:root[data-mode="dark"]) .edge {
    fill: #fffcf4;
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
