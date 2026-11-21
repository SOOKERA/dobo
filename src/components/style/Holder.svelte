<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // What holds a note on the board: a pushpin (cork, felt) or a strip of tape (linen, plaster,
  // kraft, terrazzo, blueprint, your picture). Pins lift out and drop in; tape presses on.
  import { heldBy, type Look } from "../../lib/looks";

  let { look: l, id, index = 0 }: { look: Look; id: number; index?: number } = $props();

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  const PINS = ["#C2410C", "#3D7FE0", "#6F9C67", "#D9A928"];
  const WASHI = [
    "repeating-linear-gradient(45deg, rgba(233,196,106,0.62) 0 6px, rgba(255,250,235,0.45) 6px 12px)",
    "repeating-linear-gradient(-45deg, rgba(201,114,122,0.5) 0 5px, rgba(255,245,245,0.45) 5px 10px)",
  ];

  let kind = $derived(heldBy(l));
  let pin = $derived(l.bg === "felt" ? "#D9A928" : PINS[id % PINS.length]);
  let tape = $derived(
    l.bg === "blueprint"
      ? "rgba(250,250,245,0.72)"
      : l.bg === "terrazzo"
        ? "repeating-linear-gradient(45deg, rgba(201,120,90,0.55) 0 6px, rgba(255,248,240,0.5) 6px 12px)"
        : WASHI[id % 2],
  );
  let tilt = $derived(id % 2 ? 3 : -4);

  // Pins drop in from just above and settle; tape is pressed on (a little big, then flat).
  function hold(node: HTMLElement, { pin, out }: { pin: boolean; out?: boolean }) {
    const delay = reduce.matches ? 0 : Math.min(index, 12) * 22;
    if (reduce.matches) return { duration: 140, css: (t: number) => `opacity:${t}` };
    return {
      delay: out ? 0 : delay,
      duration: out ? 180 : 320,
      css: (t: number) => {
        const u = 1 - t;
        if (pin) return `opacity:${Math.min(1, t * 1.6)};transform:translate(-50%, ${(-12 * u * u).toFixed(2)}px) scale(${1 + 0.18 * u})`;
        return `opacity:${Math.min(1, t * 1.8)};transform:translateX(-50%) rotate(${tilt}deg) scale(${1 + 0.12 * u * u})`;
      },
    };
  }
</script>

{#key kind}
  {#if kind === "pins"}
    <span class="pin" in:hold={{ pin: true }} out:hold={{ pin: true, out: true }} aria-hidden="true">
      <svg width="24" height="30" viewBox="0 0 24 30"
        ><ellipse cx="15" cy="27" rx="5" ry="1.8" fill="rgba(0,0,0,0.28)" /><path d="M12 14 L14.5 26" stroke="#7D7A75" stroke-width="1.5" stroke-linecap="round" /><circle cx="12" cy="10" r="8" fill={pin} /><circle cx="9.2" cy="7.2" r="2.6" fill="rgba(255,255,255,0.55)" /></svg
      >
    </span>
  {:else if kind === "tape"}
    <span class="tape" style:background={tape} style:--tilt="{tilt}deg" in:hold={{ pin: false }} out:hold={{ pin: false, out: true }} aria-hidden="true"></span>
  {/if}
{/key}

<style>
  .pin,
  .tape {
    position: absolute;
    left: 50%;
    z-index: 3;
    pointer-events: none;
  }
  .pin {
    top: -14px;
    width: 24px;
    height: 30px;
    transform: translateX(-50%);
  }
  .pin svg {
    display: block;
  }
  .tape {
    top: -11px;
    width: 74px;
    height: 22px;
    transform: translateX(-50%) rotate(var(--tilt));
    clip-path: polygon(3% 0, 97% 0, 100% 22%, 97% 45%, 100% 72%, 97% 100%, 3% 100%, 0 78%, 3% 52%, 0 26%);
  }
</style>
