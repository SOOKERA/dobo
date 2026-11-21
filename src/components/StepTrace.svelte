<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // How far a line's steps have got, drawn like a clock hand going round: from the top middle,
  // clockwise. On a task, a thin ring round its checkbox ("box": a faint track, and over it a
  // stroke that lengthens as steps are ticked and shortens back). An idea has no checkbox, so it
  // gets a small pie beside its count ("pie"). Both only move when the numbers change (a CSS
  // transition, nothing between): cheap on a big board. When every step is done the ring closes
  // and the pie fills, with a tick.
  let {
    done,
    total,
    shape = "box",
    size = 16,
    radius = 4,
    stroke = 1.5,
  }: { done: number; total: number; shape?: "box" | "pie"; size?: number; radius?: number; stroke?: number } = $props();

  // (Out of 100: the paths are measured that way.)
  let part = $derived(total > 0 ? Math.min(100, (done / total) * 100) : 0);
  let all = $derived(total > 0 && done >= total);
  const m = $derived(size / 2);
  /** A rounded square inside `size`, its stroke's middle `c` in from the edge, from the top middle. */
  let square = $derived.by(() => {
    const c = stroke / 2;
    const r = Math.max(0, radius - c);
    const e = size - c;
    return `M ${m} ${c} H ${e - r} A ${r} ${r} 0 0 1 ${e} ${c + r} V ${e - r} A ${r} ${r} 0 0 1 ${e - r} ${e} H ${c + r} A ${r} ${r} 0 0 1 ${c} ${e - r} V ${c + r} A ${r} ${r} 0 0 1 ${c + r} ${c} Z`;
  });
  /** A circle of radius `r` round the middle, from the top. */
  const circle = (r: number) => `M ${m} ${m - r} A ${r} ${r} 0 1 1 ${m} ${m + r} A ${r} ${r} 0 1 1 ${m} ${m - r}`;
  // The pie: an outline, and a wedge drawn as a stroke as wide as the inner circle's radius.
  let outer = $derived(m - stroke / 2);
  let inner = $derived((m - stroke - 0.9) / 2);
</script>

<svg class="trace" class:ring={shape === "box"} class:pie={shape === "pie"} class:all width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
  {#if shape === "box"}
    <path class="track" d={square} stroke-width={stroke} />
    <path class="part" d={square} stroke-width={stroke} pathLength="100" style:stroke-dashoffset={100 - part} />
  {:else}
    <path class="track" d={circle(outer)} stroke-width={stroke} />
    <path class="part" d={circle(inner)} stroke-width={inner * 2} pathLength="100" style:stroke-dashoffset={100 - part} />
    <!-- Every step done: the pie becomes a full disc with a tick on it. -->
    <circle class="disc" cx={m} cy={m} r={m} />
    <path class="tick" d="M {size * 0.29} {size * 0.51} L {size * 0.44} {size * 0.66} L {size * 0.72} {size * 0.37}" />
  {/if}
</svg>

<style>
  /* (Its classes are its own: a page's ".box" rule, like a lined note's, never reaches it.) */
  .trace {
    display: block;
    overflow: visible;
    pointer-events: none;
  }
  path {
    fill: none;
  }
  .track {
    stroke: var(--trace-base, color-mix(in srgb, var(--box) 34%, transparent));
    transition: stroke 0.14s;
  }
  /* Closed, the track under it takes the ink too, once the stroke has got all the way round:
     where its two ends meet at the top, no seam shows. */
  .ring.all .track {
    stroke: var(--trace-all, var(--ink));
    transition: stroke 0.12s 0.46s;
  }
  .pie .track {
    stroke: var(--trace, var(--soft));
    opacity: 0.45;
  }
  .part {
    stroke: var(--trace, var(--soft));
    stroke-dasharray: 100 100;
    transition:
      stroke-dashoffset 0.52s var(--ease-glide),
      stroke 0.2s;
  }
  /* The ring, closed, takes the full ink. */
  .ring.all .part {
    stroke: var(--trace-all, var(--ink));
  }
  .disc {
    fill: var(--trace, var(--soft));
    opacity: 0;
    transition: opacity 0.14s;
  }
  .all .disc {
    opacity: 1;
    transition: opacity 0.2s 0.3s;
  }
  .tick {
    stroke: var(--card);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0;
    transition: opacity 0.14s;
  }
  .all .tick {
    opacity: 1;
    /* (Once the wedge has gone all the way round.) */
    transition: opacity 0.2s 0.36s;
  }
  @media (prefers-reduced-motion: reduce) {
    .track,
    .ring.all .track,
    .part,
    .disc,
    .all .disc,
    .tick,
    .all .tick {
      transition: none;
    }
  }
</style>
