<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Customise the watercolour: the pattern (each chip the real painting in your colours), 1 to 3
  // colours (pick a slot, then a paint), the 17 watercolour paints, your own colours (softened
  // toward a pigment), white or dark paper. Everything paints live on the board; Undo works.
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import Seg from "../parts/Seg.svelte";
  import Paging from "../parts/Paging.svelte";
  import StyleFooter from "../parts/StyleFooter.svelte";
  import { look } from "../../../lib/look.svelte";
  import { PATTERNS, WC_PAINTS, paintName, rgbStrToHex, soften, type Look } from "../../../lib/looks";
  import { blip, deal, fadeQuick, glideRing } from "../../../lib/motion";
  import { t, tMaybe } from "../../../lib/i18n.svelte";

  let wc = $derived(look.wc);
  let slot = $state(0);
  $effect.pre(() => {
    if (slot >= wc.colours.length) slot = wc.colours.length - 1;
  });
  let current = $derived(wc.colours[slot]);

  const wash = (rgb: string, seed: number): Look => ({ bg: "watercolour", colour: "", held: "auto", wc: { colours: [rgb], pattern: "Scattered", seed, paper: wc.paper } });

  // Your colours: one row of 8, newest first.
  let page = $state(0);
  let pages = $derived(Math.max(1, Math.ceil(look.colours.length / 8)));
  $effect.pre(() => {
    if (page >= pages) page = pages - 1;
  });
  let mine = $derived(look.colours.slice(page * 8, page * 8 + 8));
  let picker = $state<HTMLInputElement>();

  async function custom(hex: string) {
    const rgb = soften(hex).join(",");
    await look.addColour(rgb);
    page = 0;
    await look.setSlot(slot, rgb);
  }
</script>

<div class="body">
  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Pattern")}</span>
      <button class="txt" onclick={() => look.repaint()}><Icon name="refresh" size={13} stroke={2} />{t("Repaint")}</button>
    </div>
    <div class="pats" use:glideRing={wc.pattern}>
      {#each PATTERNS as p (p)}
        <button class="pat" data-sel={wc.pattern === p} aria-pressed={wc.pattern === p} onclick={() => look.setPattern(p)}>
          <span class="th" data-ring><LookSwatch look={{ bg: "watercolour", colour: "", held: "auto", wc: { ...wc, pattern: p } }} w={76} h={42} /></span>
          <span class:on={wc.pattern === p}>{t(p)}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Colours")}</span>
      <span class="muted">{t("1 to 3 · overlaps blend like paint")}</span>
    </div>
    <div class="slots" use:glideRing={`${slot}/${wc.colours.length}`}>
      {#each wc.colours as c, i (i + c)}
        <div class="slot" data-sel={i === slot}>
          <button class="sl" aria-pressed={i === slot} data-tip={t("Pick this colour, then a paint")} onclick={() => (slot = i)} in:deal={{ i }}>
            <span class="swb" data-ring><LookSwatch look={wash(c, 30 + i * 11)} w={104} h={46} layout="own" /></span>
            <span class="sn" class:on={i === slot}>{paintName(c) ? tMaybe(paintName(c)!) : rgbStrToHex(c)}</span>
          </button>
          {#if i === slot && wc.colours.length > 1}
            <button class="x" transition:blip={{ duration: 180 }} aria-label={t("Remove this colour")} data-tip={t("Remove this colour")} onclick={() => look.removeSlot(i)}><Icon name="close" size={10} stroke={2.6} /></button>
          {/if}
        </div>
      {/each}
      {#if wc.colours.length < 3}
        <button class="add" transition:fadeQuick={{ duration: 200 }} onclick={async () => (await look.addSlot(), (slot = wc.colours.length - 1))}>
          <span class="addb"><Icon name="plus" size={14} stroke={2.2} />{t("Add")}</span>
          <span class="sn">&nbsp;</span>
        </button>
      {/if}
    </div>
  </section>

  <section class="sec">
    <span class="lbl">{t("Watercolour paints")}</span>
    <div class="paints">
      {#each WC_PAINTS as [name, rgb], i (name)}
        <button class="paint" class:on={current === rgb} aria-pressed={current === rgb} data-tip={tMaybe(name)} aria-label={tMaybe(name)} onclick={() => look.setSlot(slot, rgb)}>
          <LookSwatch look={wash(rgb, 60 + i * 3)} w={28} h={28} layout="own" />
        </button>
      {/each}
    </div>
    <div class="row-h">
      <span class="lbl soft">{t("Your colours")}</span>
      <span class="mine-nav">
        {#if pages > 1}
          <span class="dots" aria-hidden="true" transition:fadeQuick={{ duration: 200 }}>{#each Array(pages) as _, k (k)}<i class:on={k === page} in:blip></i>{/each}</span>
        {/if}
        <Paging {page} {pages} label={t("of your colours")} onPage={(p) => (page = p)} />
      </span>
    </div>
    <div class="paints mine">
      {#each mine as c, i (c.rgb)}
        <span class="mc">
          <button class="paint" class:on={current === c.rgb} aria-pressed={current === c.rgb} data-tip={rgbStrToHex(c.rgb)} aria-label={t("Your colour {hex}", { hex: rgbStrToHex(c.rgb) })} onclick={() => look.setSlot(slot, c.rgb)} in:deal={{ i }}>
            <LookSwatch look={wash(c.rgb, 70 + i * 5)} w={28} h={28} layout="own" />
          </button>
          <button class="del" aria-label={t("Remove {hex} from your colours", { hex: rgbStrToHex(c.rgb) })} data-tip={t("Remove (Undo brings it back)")} data-tip-keys="Ctrl+Z" onclick={() => look.removeColour(c.rgb)}><Icon name="close" size={8} stroke={3.4} /></button>
        </span>
      {/each}
      {#if !mine.length}<span class="muted none" in:fadeQuick={{ delay: 120, duration: 200 }}>{t("Colours you make land here.")}</span>{/if}
    </div>
    <div class="row-h">
      <span class="muted">{t("Neon or very dark picks are eased toward a pigment.")}</span>
      <button class="addc" onclick={() => picker?.click()}><Icon name="plus" size={12} stroke={2.2} />{t("Custom colour")}</button>
      <input class="visually-hidden" type="color" tabindex="-1" aria-label={t("Custom colour")} bind:this={picker} value={rgbStrToHex(current).toLowerCase()} onchange={(e) => custom(e.currentTarget.value)} />
    </div>
  </section>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Paper")}</span>
      <Seg label={t("Paper")} value={wc.paper} options={[{ key: "white", label: t("White") }, { key: "dark", label: t("Dark") }]} onPick={(p) => look.setPaper(p)} />
    </div>
    <div class="tip">{t("Pick a colour, then a paint.")} <b>×</b> {t("removes one,")} <b>+ {t("Add")}</b> {t("brings it back. One colour = a solid wash.")}</div>
  </section>
</div>

<StyleFooter mixMode />

<style>
  .txt {
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }
  .pats {
    position: relative;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 9px;
  }
  .pat {
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    cursor: pointer;
  }
  .pat .on,
  .sn.on {
    font-weight: 600;
  }
  .th,
  .swb {
    display: block;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .swb {
    border-radius: 9px;
  }
  .slots {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 9px;
  }
  .slot {
    position: relative;
    min-width: 0;
  }
  .sl,
  .add {
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    cursor: pointer;
  }
  .x {
    position: absolute;
    right: 4px;
    top: 4px;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 254, 251, 0.94);
    color: #1f1d1a;
    display: grid;
    place-items: center;
    cursor: pointer;
    z-index: 3;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .addb {
    width: 100%;
    height: 46px;
    border-radius: 9px;
    border: 1.5px dashed color-mix(in srgb, var(--ink) 22%, transparent);
    color: var(--ink);
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .paints {
    display: grid;
    grid-template-columns: repeat(9, 28px);
    justify-content: space-between;
    row-gap: 9px;
  }
  .paints.mine {
    grid-template-columns: repeat(8, 28px);
    justify-content: start;
    column-gap: 12px;
    min-height: 28px;
  }
  .paint {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    overflow: hidden;
    cursor: pointer;
    background: none;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    transition: transform 0.15s var(--ease-out), box-shadow 0.2s;
  }
  .paint:hover {
    transform: scale(1.08);
  }
  .paint.on {
    box-shadow: 0 0 0 2px var(--card), 0 0 0 4px var(--ink);
  }
  .mc {
    position: relative;
    width: 28px;
    height: 28px;
  }
  .del {
    position: absolute;
    right: -5px;
    top: -5px;
    width: 15px;
    height: 15px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--ink);
    color: var(--card);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .mc:hover .del,
  .mc:focus-within .del,
  .mc:has(.on) .del {
    opacity: 1;
  }
  .none {
    grid-column: 1 / -1;
    align-self: center;
  }
  .lbl.soft {
    font-weight: 500;
  }
  .mine-nav {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dots {
    display: flex;
    gap: 4px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--ink) 22%, transparent);
  }
  .dots i.on {
    background: var(--ink);
  }
  .addc {
    height: 30px;
    padding: 0 11px;
    border-radius: 15px;
    border: 1px dashed color-mix(in srgb, var(--ink) 30%, transparent);
    background: none;
    color: var(--ink);
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    flex: none;
  }
  .tip b {
    color: var(--ink);
    font-weight: 600;
  }
</style>
