<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // More mixes: 14 watercolour mixes by mood, one row of three each (‹ › when a mood has more).
  // Each card is the real painting at your board's shape. Tap: your board paints it.
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import Paging from "../parts/Paging.svelte";
  import { look, type PanelPage } from "../../../lib/look.svelte";
  import { WC_MOODS, WC_OURS } from "../../../lib/looks";
  import { deal, glideRing } from "../../../lib/motion";
  import { t, tMaybe } from "../../../lib/i18n.svelte";

  let { go }: { go: (p: PanelPage) => void } = $props();

  let pages = $state<number[]>(WC_MOODS.map(() => 0));
  let wc = $derived(look.wc);
  const same = (c: string[]) => c.join("|") === wc.colours.join("|");
  const ours = (name: string) => WC_OURS.some((m) => m.name === name);
</script>

<div class="body">
  {#each WC_MOODS as mood, k (mood.mood)}
    {@const count = Math.ceil(mood.mixes.length / 3)}
    {@const shown = mood.mixes.slice(pages[k] * 3, pages[k] * 3 + 3)}
    <section class="sec">
      <div class="row-h">
        <b class="mood">{tMaybe(mood.mood)}</b>
        {#if count > 1}
          <Paging page={pages[k]} pages={count} label={tMaybe(mood.mood)} onPage={(p) => (pages[k] = p)} />
        {:else}
          <span class="muted small">{t("{n} mixes", { n: mood.mixes.length })}</span>
        {/if}
      </div>
      <div class="grid3" use:glideRing={wc.colours.join("|") + pages[k]}>
        {#key pages[k]}
          {#each shown as m, i (m.name)}
            <button class="mix" data-sel={same(m.colours)} aria-pressed={same(m.colours)} data-tip={tMaybe(m.name)} onclick={() => look.pickMix(m.colours, m.name, "Scattered")} in:deal={{ i }}>
              <span class="art" data-ring><LookSwatch look={{ bg: "watercolour", colour: "", held: "auto", wc: { ...wc, colours: m.colours, pattern: "Scattered", name: m.name } }} w={104} h={56} /></span>
              <span class="n" class:on={same(m.colours)}>{tMaybe(m.name)}{#if ours(m.name)}<span class="tag">{` · ${t("ours")}`}</span>{/if}</span>
            </button>
          {/each}
        {/key}
      </div>
    </section>
  {/each}
</div>

<div class="foot">
  <div class="mf">
    <span class="muted">{t("Want your own colours?")}</span>
    <button class="to" onclick={() => go("customise")}>{t("Customise")} <Icon name="chev-right" size={13} stroke={2.2} /></button>
  </div>
</div>

<style>
  .mood {
    font-family: var(--serif);
    font-size: 15.5px;
    font-weight: 600;
  }
  .small {
    font-size: 11px;
  }
  .mix {
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
    display: flex;
    flex-direction: column;
    gap: 5px;
    text-align: left;
    cursor: pointer;
    font-size: 11.5px;
  }
  .art {
    display: block;
    width: 100%;
    border-radius: 9px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    transition: transform 0.15s var(--ease-out);
  }
  .mix:hover .art {
    transform: translateY(-1px);
  }
  .n {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .n.on {
    font-weight: 600;
    color: var(--ink);
  }
  .tag {
    font-size: 10px;
    font-weight: 400;
    color: var(--muted);
  }
  .mf {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .to {
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    cursor: pointer;
  }
</style>
