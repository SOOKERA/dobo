<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Edit theme: a big diagonal preview (light above, dark below, a little note on each), Light /
  // Dark tabs, the chosen half's style picked from ours and yours. Changes show on the board
  // at once; the footer saves the pair as your theme (or updates the one it came from).
  import { composing } from "../../../lib/compose";
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import StyleCard from "../parts/StyleCard.svelte";
  import { look } from "../../../lib/look.svelte";
  import { OUR_STYLES, OUR_THEMES, background, sameLook, type Look, type Ref } from "../../../lib/looks";
  import { arrive, crumbIn, crumbOut, easeHeight, fadeQuick, fold, glideRing, leave } from "../../../lib/motion";
  import { around, t, tMaybe } from "../../../lib/i18n.svelte";

  let half = $derived(look.editHalf);
  let b = $derived(look.board);
  let cur = $derived(half === "light" ? b.light : b.dark);
  let curFrom = $derived(half === "light" ? b.lightFrom : b.darkFrom);
  const isPicked = (ref: Ref, l: Look) => !!cur && curFrom?.kind === ref.kind && curFrom.id === ref.id && sameLook(l, cur);
  const nameOf = (l: Look | null, from: Ref | null) => {
    if (!l) return t("Same as light");
    const base = look.baseOf(from);
    if (base && sameLook(base, l)) return look.nameOf(from)!;
    const b = tMaybe(background(l.bg).name);
    return l.bg === "watercolour" ? `${b} · ${l.wc?.name ? tMaybe(l.wc.name) : t("your colours")}` : l.bg === "picture" ? t("Your picture") : `${b} · ${l.colour === "Custom" ? t("custom") : tMaybe(l.colour)}`;
  };

  // Footer: changed from the theme it came from?
  let theme = $derived(b.theme);
  let themeName = $derived(look.themeNameOf(theme));
  let pair = $derived(look.themePair(theme));
  let lightChanged = $derived(!pair || !sameLook(pair.light, b.light));
  let darkChanged = $derived(!pair || (pair.dark && b.dark ? !sameLook(pair.dark, b.dark) : !!pair.dark !== !!b.dark));
  let changed = $derived(lightChanged || darkChanged);
  let yours = $derived(theme?.kind === "yours" && !!pair);
  let changedFrom = $derived(around(t("Changed from {name}"), "{name}"));
  let name = $state("");
  let typed = $state(false);
  $effect(() => {
    const s = themeName && !OUR_THEMES.some((x) => tMaybe(x.name) === themeName) ? themeName : themeName ? t("{name} (mine)", { name: themeName }) : t("My theme");
    if (!typed) name = s;
  });
  async function save() {
    await look.saveTheme(name);
    typed = false;
  }
</script>

<div class="body">
  <div class="big">
    <span class="hl"><LookSwatch look={b.light} w={332} h={150} /></span>
    <span class="hl d"><LookSwatch look={b.dark ?? b.light} w={332} h={150} /></span>
    <div class="mn light"><b>Logitech</b>{t("Migrate to GitHub")}</div>
    <div class="mn dark"><b>Hub</b>{t("Call général")}</div>
    <span class="tag tl" class:editing={half === "light"}><Icon name="sun" size={12} stroke={2.2} />{t("Light")}{half === "light" ? ` · ${t("editing")}` : ""}</span>
    <span class="tag td" class:editing={half === "dark"}><Icon name="moon" size={11} />{t("Dark")}{half === "dark" ? ` · ${t("editing")}` : ""}</span>
  </div>

  <div class="tabs" role="tablist" aria-label={t("Which half")}>
    <button role="tab" aria-selected={half === "light"} class:on={half === "light"} onclick={() => (look.editHalf = "light")}><Icon name="sun" size={14} stroke={2} /><span>{t("Light")} · {nameOf(b.light, b.lightFrom)}</span></button>
    <button role="tab" aria-selected={half === "dark"} class:on={half === "dark"} onclick={() => (look.editHalf = "dark")}><Icon name="moon" size={13} /><span>{t("Dark")} · {nameOf(b.dark, b.darkFrom)}</span></button>
  </div>

  {#if half === "dark"}
    <button class="same" transition:fold={{ duration: 240 }} class:on={!b.dark} aria-pressed={!b.dark} onclick={() => look.setHalf("dark", null, null)}>
      <span class="ring"></span>
      <span><b>{t("Same as light")}</b><span class="muted">{` · ${t("the board never changes with dark mode")}`}</span></span>
    </button>
  {/if}

  <section class="sec">
    <div class="row-h">
      <span class="lbl-swap">{#key half}<span class="lbl" in:crumbIn out:crumbOut>{half === "light" ? t("Light style · ours") : t("Dark style · ours")}</span>{/key}</span>
      <span class="muted">{t("pick any style")}</span>
    </div>
    <div class="grid3" use:glideRing={`${half}${curFrom?.id}${cur ? JSON.stringify(cur) : ""}`}>
      {#each OUR_STYLES as s (s.id)}
        {@const ref = { kind: "ours", id: s.id } as Ref}
        <StyleCard name={tMaybe(s.name)} look={s.look} h={28} selected={isPicked(ref, s.look)} onPick={() => look.setHalf(half, s.look, ref)} />
      {/each}
    </div>
  </section>
  {#if look.styles.length}
    <section class="sec" transition:fold>
      <span class="lbl-swap">{#key half}<span class="lbl" in:crumbIn out:crumbOut>{half === "light" ? t("Light style · yours") : t("Dark style · yours")}</span>{/key}</span>
      <div class="grid3" use:glideRing={`${half}${curFrom?.id}`}>
        {#each look.styles as s (s.id)}
          {@const ref = { kind: "yours", id: s.id } as Ref}
          <StyleCard yours name={s.name} look={s.look} h={28} selected={isPicked(ref, s.look)} onPick={() => look.setHalf(half, s.look, ref)} />
        {/each}
      </div>
    </section>
  {/if}
</div>

<div class="foot">
  <div class="room" use:easeHeight>
  <div class="stk">
  {#if changed}
    <div class="state" in:arrive={{ delay: 90, duration: 240, y: 4 }} out:leave={{ duration: 120, y: 0 }}>
    <div class="ch">
      <span class="what">
        {#if themeName}{changedFrom[0]}<b>{themeName}</b>{changedFrom[1]} · {lightChanged && darkChanged ? t("both halves") : lightChanged ? t("Light") : t("Dark")}{:else}{t("A new pair, not saved yet")}{/if}
      </span>
      {#if pair}<button class="link" transition:fadeQuick onclick={() => look.resetTheme()}>{t("Reset")}</button>{/if}
    </div>
    <div class="save">
      {#if yours}
        <button class="btn grow" onclick={() => theme && look.updateTheme(theme.id)}>{t("Update “{name}”", { name: themeName ?? "" })}</button>
        <button class="btn2" onclick={() => look.saveTheme(themeName ?? t("My theme"))}>{t("Save as new")}</button>
      {:else}
        <label class="visually-hidden" for="theme-name">{t("Theme name")}</label>
        <input id="theme-name" class="name-in" bind:value={name} oninput={() => (typed = true)} maxlength="60" onkeydown={(e) => e.key === "Enter" && !composing(e) && name.trim() && save()} />
        <button class="btn" disabled={!name.trim()} onclick={save}>{t("Save as my theme")}</button>
      {/if}
    </div>
    </div>
  {:else}
    <div class="state" in:arrive={{ delay: 90, duration: 240, y: 0 }} out:leave={{ duration: 120, y: 0 }}>
    <div class="ch">
      <span class="what"><b>{themeName}</b> · {yours ? t("your theme") : t("one of ours")}</span>
      <span class="muted">{t("Pick a style to change a half")}</span>
    </div>
    </div>
  {/if}
  </div>
  </div>
</div>

<style>
  .stk,
  .lbl-swap {
    position: relative;
    display: grid;
  }
  .lbl-swap > * {
    grid-area: 1 / 1;
  }
  .state {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .big {
    position: relative;
    flex: none;
    height: 150px;
    border-radius: 12px;
    overflow: hidden;
  }
  .big::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    pointer-events: none;
  }
  .hl {
    position: absolute;
    inset: 0;
  }
  .hl.d {
    clip-path: polygon(100% 0, 100% 100%, 0 100%);
  }
  .mn {
    position: absolute;
    width: 86px;
    border-radius: 6px;
    padding: 6px 8px;
    font-size: 9.5px;
    line-height: 1.35;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }
  .mn b {
    display: block;
    font-family: var(--serif);
    font-size: 11px;
  }
  .mn.light {
    left: 18px;
    top: 40px;
    background: #fffefb;
    color: #1f1d1a;
  }
  .mn.dark {
    right: 26px;
    bottom: 34px;
    background: #2a292f;
    color: #ece8e0;
  }
  .tag {
    position: absolute;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .tl {
    left: 10px;
    top: 10px;
    background: rgba(255, 254, 251, 0.92);
    color: #1f1d1a;
  }
  .td {
    right: 10px;
    bottom: 10px;
    background: rgba(36, 37, 42, 0.92);
    color: #ece8e0;
  }
  .tag.editing {
    box-shadow: 0 0 0 2px #e9a23b;
  }
  .tabs {
    flex: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
    padding: 3px;
    border-radius: 10px;
    background: var(--chip);
  }
  .tabs button {
    min-width: 0;
    height: 34px;
    padding: 0 8px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--soft);
    font-size: 12.5px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    cursor: pointer;
  }
  .tabs button span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tabs button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .same {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    color: var(--ink);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .same b {
    font-weight: 600;
  }
  .ring {
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.6px solid var(--box);
  }
  .same.on {
    border-color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--ink);
  }
  .same.on .ring {
    border: 5px solid var(--ink);
  }
  .ch {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
    font-size: 12px;
  }
  .what {
    color: var(--soft);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .what b {
    font-weight: 600;
    color: var(--ink);
  }
  .save {
    display: flex;
    gap: 8px;
  }
  .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
