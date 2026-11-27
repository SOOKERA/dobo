<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Style panel's footer: "Changed from Cork board · colour Honey" + Reset, a name and
  // Save as my style (or, for one of yours, Update "name" and Save as new).
  import { composing } from "../../../lib/compose";
  import { look } from "../../../lib/look.svelte";
  import { background, describeChange, sameLook } from "../../../lib/looks";
  import { around, cap, t, tMaybe } from "../../../lib/i18n.svelte";
  import { arrive, easeHeight, fadeQuick, leave } from "../../../lib/motion";

  let { mixMode = false }: { mixMode?: boolean } = $props();

  let from = $derived(look.activeFrom);
  let base = $derived(look.baseOf(from));
  let baseName = $derived(look.nameOf(from));
  let changed = $derived(!base || !sameLook(base, look.active));
  let change = $derived(base && changed ? describeChange(base, look.active) : "");
  let yours = $derived(from?.kind === "yours" && !!base);

  let changedFrom = $derived(around(t("Changed from {name}"), "{name}"));
  // (The whole line, in a tooltip when it is cut short at a narrow width.)
  let whole = $derived(baseName ? t("Changed from {name}", { name: baseName }) + (change ? ` · ${change}` : "") : t("Your own mix, not saved yet"));
  let name = $state("");
  let typed = $state(false);
  // A suggested name until you type one: "My cork", or the mix's.
  $effect(() => {
    const suggestion = cap(mixMode ? (look.wc.name ? t("My {name}", { name: tMaybe(look.wc.name).toLowerCase() }) : t("My watercolour")) : look.active.bg === "picture" ? t("My picture") : t("My {name}", { name: tMaybe(background(look.active.bg).name).toLowerCase() }));
    if (!typed) name = suggestion;
  });

  async function save() {
    if (mixMode) await look.saveMix(name);
    else await look.saveStyle(name);
    typed = false;
  }
</script>

<!-- Changed ⇄ not changed: the footer eases to its new height and the two crossfade. -->
<div class="foot">
  <div class="room" use:easeHeight>
  <div class="stk">
  {#if changed}
    <div class="state" in:arrive={{ delay: 90, duration: 240, y: 4 }} out:leave={{ duration: 120, y: 0 }}>
    <div class="ch">
      <span class="what" data-tip={whole} data-tip-cut>
        {#if baseName}{changedFrom[0]}<b>{baseName}</b>{changedFrom[1]}{#if change}{" · "}{change}{/if}{:else}{t("Your own mix, not saved yet")}{/if}
      </span>
      {#if base}<button class="link" transition:fadeQuick onclick={() => look.reset()}>{t("Reset")}</button>{/if}
    </div>
    <div class="save">
      {#if yours && !mixMode}
        <button class="btn grow" onclick={() => from && look.updateStyle(from.id)}><span class="cut">{t("Update “{name}”", { name: baseName ?? "" })}</span></button>
        <button class="btn2" onclick={() => look.saveStyle(baseName ?? t("My style"))}>{t("Save as new")}</button>
      {:else}
        <label class="visually-hidden" for="style-name">{mixMode ? t("Mix name") : t("Style name")}</label>
        <input id="style-name" class="name-in" bind:value={name} oninput={() => (typed = true)} maxlength="60" onkeydown={(e) => e.key === "Enter" && !composing(e) && name.trim() && save()} />
        <button class="btn" disabled={!name.trim()} onclick={save}>{mixMode ? t("Save as my mix") : t("Save as my style")}</button>
      {/if}
    </div>
    </div>
  {:else}
    <div class="state" in:arrive={{ delay: 90, duration: 240, y: 0 }} out:leave={{ duration: 120, y: 0 }}>
    <div class="ch">
      <span class="what wrap"><b>{baseName}</b>{" · "}{yours ? t("your style") : from?.kind === "theme" ? t("from the theme") : t("one of ours")}<span class="muted">{t(". Change anything to make it yours.")}</span></span>
    </div>
    </div>
  {/if}
  </div>
  </div>
</div>

<style>
  .stk {
    position: relative;
    display: grid;
  }
  .state {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .ch {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
    font-size: 12px;
    min-height: 18px;
  }
  .what {
    color: var(--soft);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* the resting line has no button beside it: let it wrap instead of cutting off */
  .what.wrap {
    white-space: normal;
    line-height: 1.45;
    text-wrap: pretty;
  }
  .what b {
    font-weight: 600;
    color: var(--ink);
  }
  .save {
    display: flex;
    gap: 8px;
  }
  .save .btn.grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* (The panel's .btn is a flex box that doesn't shrink: these win over it, and its words need
     their own box to be cut short.) */
  .save .btn.grow .cut {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
