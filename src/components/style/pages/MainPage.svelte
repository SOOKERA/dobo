<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Style panel's main page: the theme (Light + Dark style), the dark-mode switch, our
  // styles and yours, the background and its own colours (watercolour: our five, yours, More
  // mixes, Customise), what holds the notes, note size and the new notes' colour.
  import Icon from "../../Icon.svelte";
  import LookSwatch from "../LookSwatch.svelte";
  import StyleCard from "../parts/StyleCard.svelte";
  import Seg from "../parts/Seg.svelte";
  import Paging from "../parts/Paging.svelte";
  import StyleFooter from "../parts/StyleFooter.svelte";
  import MixCard from "../parts/MixCard.svelte";
  import { look, type PanelPage } from "../../../lib/look.svelte";
  import { store, SIZES, type NoteSize } from "../../../lib/store.svelte";
  import { BACKGROUNDS, background, OUR_STYLES, WC_OURS, WC_MOODS, sameLook, type BgKind, type Held, type Look, type SavedMix } from "../../../lib/looks";
  import { arrive, deal, easeFollowers, easeHeight, fadeQuick, fold, glideRing, leave, sink, titleIn, titleOut } from "../../../lib/motion";
  import { swatchesFor } from "../../../lib/palette";
  import type { PaletteName } from "../../../lib/types";
  import { t, tMaybe } from "../../../lib/i18n.svelte";

  let { go }: { go: (p: PanelPage) => void } = $props();

  let a = $derived(look.active);
  let bg = $derived(background(a.bg));
  let fromRef = $derived(look.activeFrom);

  // ---- Theme line ----
  let themeName = $derived(look.themeNameOf(look.board.theme));
  let pairChanged = $derived.by(() => {
    const p = look.themePair(look.board.theme);
    if (!p) return true;
    const sameDark = p.dark && look.board.dark ? sameLook(p.dark, look.board.dark) : !p.dark && !look.board.dark;
    return !sameLook(p.light, look.board.light) || !sameDark;
  });
  const nameFor = (l: Look | null, from: typeof fromRef) => {
    if (!l) return t("Same as light");
    const n = look.nameOf(from);
    const base = look.baseOf(from);
    if (n && base && sameLook(base, l)) return n;
    // Changed: its background and colour ("Paper · Night").
    const b = tMaybe(background(l.bg).name);
    return l.bg === "watercolour" ? `${b} · ${l.wc?.name ? tMaybe(l.wc.name) : t("your colours")}` : l.bg === "picture" ? t("Your picture") : `${b} · ${l.colour === "Custom" ? t("custom") : tMaybe(l.colour)}`;
  };

  // ---- Background chips: each shows its own texture (in the colour you have, when it's yours) ----
  function chipLook(k: BgKind): Look {
    if (k === a.bg) return a;
    if (k === "watercolour") return { bg: k, colour: "", held: "auto", wc: look.wc };
    if (k === "picture") return { bg: k, colour: "", held: "auto", picture: findPicture() };
    return { bg: k, colour: background(k).palette[0].name, held: "auto" };
  }
  function findPicture(): string | undefined {
    const all = [look.board.light, look.board.dark, ...look.styles.map((s) => s.look)];
    return all.find((l) => l?.bg === "picture" && l.picture)?.picture;
  }

  // ---- Colours of the background ----
  let customInput = $state<HTMLInputElement>();
  let swatchCols = $derived(bg.palette.length + (a.colour === "Custom" ? 1 : 0));

  // ---- Watercolour ----
  let wc = $derived(look.wc);
  const sameMix = (c: string[]) => c.join("|") === wc.colours.join("|");
  let yoursPage = $state(0);
  let yoursCells = $derived<(SavedMix | null)[]>([...look.mixes, null]);
  let yoursPages = $derived(Math.max(1, Math.ceil(yoursCells.length / 5)));
  let yoursShown = $derived(yoursCells.slice(yoursPage * 5, yoursPage * 5 + 5));
  let nowOn = $derived.by(() => {
    const mine0 = wc.name ? look.mixes.find((m) => m.name === wc.name && sameMix(m.colours)) : null;
    if (mine0) return { name: mine0.name, from: t("one of yours") };
    if (WC_OURS.some((m) => sameMix(m.colours))) return { name: tMaybe(WC_OURS.find((m) => sameMix(m.colours))!.name), from: t("one of ours") };
    const mood = WC_MOODS.flatMap((m) => m.mixes).find((m) => sameMix(m.colours));
    if (mood) return { name: tMaybe(mood.name), from: t("from More mixes") };
    const mine = look.mixes.find((m) => sameMix(m.colours));
    if (mine) return { name: mine.name, from: t("one of yours") };
    return { name: t("Your own colours"), from: t("from Customise") };
  });

  // ---- New notes ----
  let nn = $derived(look.newNoteColour);
  let nnTab = $state<PaletteName | "turns">("turns");
  $effect.pre(() => {
    nnTab = nn?.palette ?? "turns";
  });
  let nnSwatches = $derived(nnTab === "turns" ? [] : swatchesFor(nnTab, store.custom));

  // Two lines for "By hours".
  let dm = $derived(look.darkMode);
  let sizes = $derived<{ key: NoteSize; label: string; size: number; title: string }[]>([
    { key: "compact", label: "A", size: 13, title: t("Small notes") },
    { key: "comfortable", label: "A", size: 16, title: t("Normal notes") },
    { key: "large", label: "A", size: 19, title: t("Large notes") },
  ]);
  void SIZES;
</script>

<div class="body">
  <!-- Theme: the Light style and the Dark style. -->
  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Theme · {name}", { name: themeName ? (pairChanged ? t("{name}, changed", { name: themeName }) : themeName) : t("your own pair") })}</span>
      <button class="to" onclick={() => go("themes")}>{t("Themes")} <Icon name="chev-right" size={13} stroke={2.2} /></button>
    </div>
    <div class="themeline">
      <button class="half" class:now={!look.editingDark} data-tip={t("Change the Light style")} onclick={() => ((look.editHalf = "light"), go("edit-theme"))}>
        <span class="hsw"><LookSwatch look={look.board.light} w={30} h={22} /></span>
        <span class="ht"><small>{t("Light")}</small><b>{nameFor(look.board.light, look.board.lightFrom)}</b></span>
      </button>
      <button class="half" class:now={look.editingDark} data-tip={t("Change the Dark style")} onclick={() => ((look.editHalf = "dark"), go("edit-theme"))}>
        <span class="hsw"><LookSwatch look={look.board.dark ?? look.board.light} w={30} h={22} /></span>
        <span class="ht"><small>{t("Dark")}</small><b>{nameFor(look.board.dark, look.board.darkFrom)}</b></span>
      </button>
    </div>
  </section>

  <!-- Appearance (light / dark): changes the app and the notes; the board only via the theme's
       Dark style. The same setting as Settings › Appearance and the header's sun/moon. -->
  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Appearance")}</span>
      <Seg
        label={t("Appearance")}
        value={dm.mode}
        options={[
          { key: "windows", label: "Windows", title: t("Follow Windows") },
          { key: "hours", label: t("By hours"), title: t("Dark between two times") },
          { key: "light", label: t("Light"), title: t("Always light") },
          { key: "dark", label: t("Dark"), title: t("Always dark") },
        ]}
        onPick={(m) => look.setDarkMode({ ...dm, mode: m })}
      />
    </div>
    {#if dm.mode === "hours"}
      <div class="hours" transition:fold={{ gap: 8, duration: 260 }}>
        <span>{t("Dark from")}</span>
        <input type="time" aria-label={t("Dark from")} value={dm.from} onchange={(e) => e.currentTarget.value && look.setDarkMode({ ...dm, from: e.currentTarget.value })} />
        <span>{t("to")}</span>
        <input type="time" aria-label={t("Dark until")} value={dm.until} onchange={(e) => e.currentTarget.value && look.setDarkMode({ ...dm, until: e.currentTarget.value })} />
      </div>
    {/if}
  </section>

  <!-- Our styles, then yours. -->
  <section class="sec">
    <span class="lbl">{t("Our styles")}</span>
    <div class="grid3" use:glideRing={fromRef?.kind === "ours" ? fromRef.id : null}>
      {#each OUR_STYLES as s (s.id)}
        <StyleCard name={tMaybe(s.name)} look={s.look} selected={fromRef?.kind === "ours" && fromRef.id === s.id} onPick={() => look.pickStyle({ kind: "ours", id: s.id })} />
      {/each}
    </div>
    {#if look.styles.length}
      <div class="yours" transition:fold={{ gap: 8 }}>
      <span class="lbl sub">{t("Your styles")}</span>
      <div class="grid3" use:glideRing={fromRef?.kind === "yours" ? fromRef.id : null}>
        {#each look.styles as s (s.id)}
          <StyleCard
            yours
            name={s.name}
            look={s.look}
            selected={fromRef?.kind === "yours" && fromRef.id === s.id}
            onPick={() => look.pickStyle({ kind: "yours", id: s.id })}
            onRename={(n) => look.renameStyle(s.id, n)}
            menu={[
              { icon: "check", label: t("Use this style"), run: () => look.pickStyle({ kind: "yours", id: s.id }) },
              { id: "rename", icon: "lines", label: t("Rename"), run: () => {} },
              { icon: "copy", label: t("Duplicate"), run: () => look.duplicateStyle(s.id) },
              { icon: "trash", label: t("Delete…"), danger: true, run: () => look.deleteStyle(s.id) },
            ]}
          />
        {/each}
      </div>
      </div>
    {/if}
  </section>

  <!-- Background, then its own colours. -->
  <section class="sec">
    <span class="lbl">{t("Background")}</span>
    <div class="bgs" use:glideRing={a.bg}>
      {#each BACKGROUNDS as b (b.key)}
        <button class="bgc" data-sel={a.bg === b.key} aria-pressed={a.bg === b.key} data-tip={tMaybe(b.name)} onclick={() => look.setBg(b.key)}>
          <span class="bgsw" data-ring><LookSwatch look={chipLook(b.key)} w={76} h={30} /></span>
          <span class="bgn" class:on={a.bg === b.key}>{tMaybe(b.name)}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="sec colours" use:easeFollowers>
    <div class="row-h">
      <span class="stack">
        {#key bg.key}
          <span class="lbl" in:titleIn out:titleOut>{tMaybe(bg.colourLabel)}</span>
        {/key}
      </span>
      {#if bg.custom}
        <span class="muted which" transition:fadeQuick={{ duration: 180 }}>
          <button class:on={a.colour !== "Custom"} onclick={() => a.colour === "Custom" && look.setColour(bg.palette[0].name)}>{t("Ours")}</button>
          ·
          <button class:on={a.colour === "Custom"} onclick={() => customInput?.click()}>{t("Custom")}</button>
          <input class="visually-hidden" type="color" bind:this={customInput} value={a.colour === "Custom" ? (a.custom ?? "#888888").toLowerCase() : bg.palette[0].hex.toLowerCase()} onchange={(e) => look.setCustom(e.currentTarget.value)} tabindex="-1" aria-label={t("Custom colour")} />
        </span>
      {/if}
    </div>
    <div class="stack">
      {#key bg.key}
        <div class="rowbox" out:sink>
          {#if bg.key === "watercolour"}
            <div class="wc">
              <div class="defs" use:glideRing={wc.colours.join("|")}>
                {#each WC_OURS as m, i (m.name)}
                  <button class="def" data-sel={sameMix(m.colours)} aria-pressed={sameMix(m.colours)} data-tip={tMaybe(m.name)} onclick={() => look.pickMix(m.colours, m.name, "Scattered")} in:deal={{ i }}>
                    <span class="dsw" data-ring><LookSwatch look={{ bg: "watercolour", colour: "", held: "auto", wc: { ...wc, colours: m.colours, pattern: "Scattered" } }} w={58} h={46} /></span>
                    <span class="dn">{tMaybe(m.name)}</span>
                  </button>
                {/each}
              </div>
              <div class="row-h">
                <span class="lbl">{t("Watercolour · yours")}</span>
                <Paging page={yoursPage} pages={yoursPages} label={t("of yours")} onPage={(p) => (yoursPage = p)} />
              </div>
              <div class="defs" use:glideRing={wc.colours.join("|") + yoursPage}>
                {#each yoursShown as m, i (m?.id ?? "make")}
                  {#if m}
                    <MixCard mix={m} selected={sameMix(m.colours)} {i} />
                  {:else}
                    <button class="def" data-tip={t("Make your own in Customise")} onclick={() => go("customise")} in:deal={{ i }}>
                      <span class="newmix"><Icon name="plus" size={14} stroke={2.2} /></span>
                      <span class="dn">{t("Make one")}</span>
                    </button>
                  {/if}
                {/each}
              </div>
              <div class="nowon">
                <LookSwatch look={a} w={46} h={28} />
                <span><b>{nowOn.name}</b> <span class="muted">{t("is on your board · {from}", { from: nowOn.from })}</span></span>
              </div>
              <div class="repaint">
                <button class="btn2" onclick={() => look.repaint()}><Icon name="brush" size={14} stroke={1.9} />{t("Repaint")}</button>
                <span class="muted">{t("Same colours, a fresh painting.")}</span>
              </div>
              <button class="go" onclick={() => go("more")}>
                <Icon name="grid" size={20} />
                <span style="flex: 1"><b>{t("More mixes")}</b><span class="sub">{t("14 mixes, grouped by mood")}</span></span>
                <Icon name="chev-right" size={16} stroke={2} />
              </button>
              <button class="go" onclick={() => go("customise")}>
                <Icon name="settings" size={20} />
                <span style="flex: 1"><b>{t("Customise")}</b><span class="sub">{t("Another pattern, your own colours")}</span></span>
                <Icon name="chev-right" size={16} stroke={2} />
              </button>
            </div>
          {:else if bg.key === "picture"}
            <div class="pic">
              <span class="picsw"><LookSwatch look={a} w={120} h={68} /></span>
              <div class="pict">
                <button class="btn2" onclick={() => look.pickPicture()}><Icon name="image" size={14} />{a.picture ? t("Another picture…") : t("Choose a picture…")}</button>
                <span class="muted">{t("Softened and dimmed so your notes stay easy to read.")}</span>
              </div>
            </div>
          {:else}
            <div class="sws" style:grid-template-columns="repeat({swatchCols}, minmax(0, 1fr))" use:glideRing={a.colour + (a.custom ?? "")}>
              {#each bg.palette as c, i (c.name)}
                {@const l = { bg: bg.key, colour: c.name, held: "auto" } as Look}
                <button class="sw" data-sel={a.colour === c.name} aria-pressed={a.colour === c.name} data-tip={tMaybe(c.name)} aria-label={tMaybe(c.name)} onclick={() => look.setColour(c.name)} in:deal={{ i }} data-ring>
                  <LookSwatch look={l} w={70} h={30} />
                </button>
              {/each}
              {#if a.colour === "Custom"}
                <button class="sw" data-sel="true" aria-pressed="true" data-tip={t("Your colour · click to change it")} aria-label={t("Custom colour")} onclick={() => customInput?.click()} in:deal={{ i: bg.palette.length }} data-ring>
                  <LookSwatch look={a} w={70} h={30} />
                </button>
              {/if}
            </div>
            <span class="cname muted">{a.colour === "Custom" ? t("Custom · {hex}", { hex: a.custom ?? "" }) : tMaybe(a.colour)}</span>
          {/if}
        </div>
      {/key}
    </div>
  </section>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Notes held by")}</span>
      <Seg
        label={t("Notes held by")}
        value={a.held}
        options={[
          { key: "auto", label: t("Auto"), title: t("As the background likes ({how})", { how: background(a.bg).auto === "none" ? t("nothing") : background(a.bg).auto === "pins" ? t("pins") : t("tape") }) },
          { key: "pins", label: t("Pins") },
          { key: "tape", label: t("Tape") },
          { key: "none", label: t("None") },
        ] as { key: Held; label: string; title?: string }[]}
        onPick={(h) => look.setHeld(h)}
      />
    </div>
    <div class="row-h">
      <span class="lbl">{t("Note size")}</span>
      <Seg serif label={t("Note size")} value={store.size} options={sizes} onPick={(s) => store.setSetting("note_size", s)} />
    </div>
  </section>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("New notes start in")}</span>
      <span class="muted which">
        {#each [["turns", "Turns", "Each new note takes the next colour"], ["soft", "Soft", "Soft colours"], ["vivid", "Vivid", "Vivid colours"], ["custom", "Custom", "Custom colours"]] as const as [k, label, tip], i (k)}
          {#if i}·{/if}
          <button class:on={nnTab === k} data-tip={t(tip)} onclick={() => (k === "turns" ? ((nnTab = "turns"), look.setNewNoteColour(null)) : (nnTab = k as PaletteName))}>{t(label)}</button>
        {/each}
      </span>
    </div>
    <!-- Turns ⇄ a palette: the old row fades out of the way, the new one comes in, the room eases. -->
    <div use:easeHeight>
    <div class="nn-stack">
    {#key nnTab}
    <div class="nn-face" in:arrive={{ delay: 90, duration: 240, y: 4 }} out:leave={{ duration: 120, y: 0 }}>
    {#if nnTab === "turns"}
      <span class="muted">{t("Each new note takes the next colour in turn.")}</span>
    {:else}
      <div class="nn">
        {#each nnSwatches as s (s.key)}
          {@const on = nn?.palette === nnTab && nn.color === s.key}
          <button class="nsw" class:on aria-pressed={on} data-tip={tMaybe(s.name)} aria-label={tMaybe(s.name)} onclick={() => look.setNewNoteColour({ palette: nnTab as PaletteName, color: s.key })}>
            <span style:background={look.dark ? s.dark.bg : s.light.bg}><i style:background={s.light.dot}></i></span>
          </button>
        {/each}
      </div>
    {/if}
    </div>
    {/key}
    </div>
    </div>
  </section>
</div>

<StyleFooter />

<style>
  .lbl.sub {
    margin-top: 4px;
  }
  .yours {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .nn-stack {
    position: relative;
    display: grid;
  }
  .nn-face {
    grid-area: 1 / 1;
    min-width: 0;
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
  .themeline {
    display: flex;
    gap: 8px;
    padding: 6px;
    border-radius: 12px;
    background: var(--chip);
  }
  .half {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 0;
    border-radius: 9px;
    background: var(--card);
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
    transition: box-shadow 0.2s;
  }
  .half.now {
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06), 0 0 0 1.5px var(--ink);
  }
  .hsw {
    flex: none;
    border-radius: 5px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .ht {
    min-width: 0;
    font-size: 12px;
    line-height: 1.25;
  }
  .ht small {
    display: block;
    font-size: 10.5px;
    color: var(--muted);
  }
  .ht b {
    display: block;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hours {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    font-size: 12.5px;
    color: var(--soft);
  }
  .hours input {
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
  }
  .bgs {
    position: relative;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px 8px;
  }
  .bgc {
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }
  .bgsw {
    display: block;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    transition: transform 0.15s var(--ease-out);
  }
  .bgc:hover .bgsw {
    transform: translateY(-1px);
  }
  .bgn {
    font-size: 11px;
  }
  .bgn.on {
    font-weight: 600;
  }
  .stack {
    display: grid;
  }
  .stack > :global(*) {
    grid-area: 1 / 1;
  }
  .which {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .which button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    text-underline-offset: 2px;
  }
  .which button:hover {
    color: var(--ink);
  }
  .which button.on {
    color: var(--ink);
    font-weight: 600;
  }
  .rowbox {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .sws {
    position: relative;
    display: grid;
    gap: 8px;
  }
  .sw {
    height: 30px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    overflow: hidden;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    background: none;
  }
  .sw :global(.sw) {
    width: 100% !important;
  }
  .cname {
    font-size: 11.5px;
  }
  .wc {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .defs {
    position: relative;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 8px;
    align-items: start;
  }
  .def {
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    cursor: pointer;
    font-size: 11px;
    line-height: 1.25;
    text-align: center;
  }
  .dsw {
    display: block;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .newmix {
    width: 100%;
    height: 46px;
    border-radius: 8px;
    border: 1.5px dashed color-mix(in srgb, var(--ink) 22%, transparent);
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .nowon {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--chip) 55%, var(--card));
    font-size: 12.5px;
  }
  .nowon > :global(.sw) {
    border-radius: 6px;
    box-shadow: 0 0 0 2px var(--card), 0 0 0 3.5px var(--ink);
  }
  .nowon b {
    font-weight: 600;
  }
  .repaint {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pic {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .picsw {
    border-radius: 9px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .pict {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .nn {
    display: grid;
    grid-template-columns: repeat(10, minmax(0, 1fr));
    gap: 5px;
  }
  .nsw {
    aspect-ratio: 1;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    background: none;
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s var(--ease-out);
  }
  .nsw:hover {
    transform: scale(1.06);
  }
  .nsw.on {
    border-color: var(--ink);
  }
  .nsw span {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    display: grid;
    place-items: center;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .nsw i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
</style>
