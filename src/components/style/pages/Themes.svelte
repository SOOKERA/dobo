<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Themes: a Light style and a Dark style kept together. Ours (they never change: Use, Edit a
  // copy, Export) and yours (newest first). Tap a card to use it; tap a half of the one in use
  // to change that half. Export / Import moves your look to another PC.
  import Icon from "../../Icon.svelte";
  import ThemeCard from "../parts/ThemeCard.svelte";
  import Paging from "../parts/Paging.svelte";
  import { look, type PanelPage } from "../../../lib/look.svelte";
  import { OUR_THEMES, background, sameLook, themeShownName, type Look, type Ref, type ThemeRef } from "../../../lib/looks";
  import { glideRing } from "../../../lib/motion";
  import { t, tMaybe } from "../../../lib/i18n.svelte";
  import { shownHHMM } from "../../../lib/time";

  let { go }: { go: (p: PanelPage) => void } = $props();

  let th0 = $derived(look.board.theme);
  let pairChanged = $derived.by(() => {
    const p = look.themePair(th0);
    if (!p) return true;
    const sameDark = p.dark && look.board.dark ? sameLook(p.dark, look.board.dark) : !p.dark && !look.board.dark;
    return !sameLook(p.light, look.board.light) || !sameDark;
  });
  let name = $derived(look.themeNameOf(th0) ? (pairChanged ? t("{name}, changed", { name: look.themeNameOf(th0)! }) : look.themeNameOf(th0)!) : t("Your pair"));
  const styleName = (l: Look, from: Ref | null) => {
    const b = look.baseOf(from);
    return b && sameLook(b, l) ? look.nameOf(from)! : tMaybe(background(l.bg).name);
  };
  let status = $derived.by(() => {
    const dm = look.darkMode;
    const lightName = styleName(look.board.light, look.board.lightFrom);
    const darkName = look.board.dark ? styleName(look.board.dark, look.board.darkFrom) : lightName;
    if (dm.mode === "hours") return look.dark ? t("dark until {time}, then {name}", { time: shownHHMM(dm.until), name: lightName }) : t("light until {time}, then {name}", { time: shownHHMM(dm.from), name: darkName });
    if (dm.mode === "windows") return look.dark ? t("follows Windows · dark now, {name}", { name: darkName }) : t("follows Windows · light now, {name}", { name: lightName });
    return dm.mode === "dark" ? t("always dark · {name}", { name: darkName }) : t("always light · {name}", { name: lightName });
  });

  const isOn = (ref: ThemeRef) => th0?.kind === ref.kind && th0.id === ref.id;
  let page = $state(0);
  let cells = $derived([...look.themes.map((x) => x as (typeof look.themes)[number] | null), null]);
  let pages = $derived(Math.max(1, Math.ceil(cells.length / 6)));
  let shown = $derived(cells.slice(page * 6, page * 6 + 6));

  function editHalf(half: "light" | "dark") {
    look.editHalf = half;
    go("edit-theme");
  }
  async function editCopy(pair: { light: Look; dark: Look | null }, n: string) {
    const copy = await look.duplicateTheme(pair, t("{name} copy", { name: n }));
    await look.useTheme(copy, { kind: "yours", id: copy.id }, copy.name);
    editHalf(look.dark ? "dark" : "light");
  }
</script>

<div class="body">
  <div class="now">
    <Icon name={look.dark ? "moon" : "sun"} size={15} />
    <span><b>{name}</b> · {status}</span>
  </div>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Our themes")}</span>
      <span class="muted hint">{t("tap to use · tap a half to change it")}</span>
    </div>
    <div class="grid3" use:glideRing={th0 ? th0.kind + th0.id : null}>
      {#each OUR_THEMES as th (th.id)}
        {@const ref = { kind: "ours", id: th.id } as ThemeRef}
        <ThemeCard
          name={tMaybe(th.name)}
          light={th.light}
          dark={th.dark}
          selected={isOn(ref)}
          onUse={() => look.useTheme(th, ref, tMaybe(th.name))}
          onHalf={editHalf}
          menu={[
            { icon: "check", label: t("Use this theme"), run: () => look.useTheme(th, ref, tMaybe(th.name)) },
            { icon: "rename", label: t("Edit a copy"), run: () => editCopy(th, tMaybe(th.name)) },
            { icon: "download", label: t("Export this theme"), run: () => look.exportLook("theme", th) },
          ]}
        />
      {/each}
    </div>
  </section>

  <section class="sec">
    <div class="row-h">
      <span class="lbl">{t("Your themes")}</span>
      <Paging {page} {pages} label={t("of your themes")} onPage={(p) => (page = p)} />
    </div>
    <div class="grid3" use:glideRing={(th0 ? th0.kind + th0.id : "") + page}>
      {#each shown as th (th?.id ?? "new")}
        {#if th}
          {@const ref = { kind: "yours", id: th.id } as ThemeRef}
          <ThemeCard
            name={themeShownName(th)}
            light={th.light}
            dark={th.dark}
            selected={isOn(ref)}
            onUse={() => look.useTheme(th, ref, themeShownName(th))}
            onHalf={editHalf}
            onRename={(n) => look.renameTheme(th.id, n)}
            menu={[
              { icon: "check", label: t("Use this theme"), run: () => look.useTheme(th, ref, themeShownName(th)) },
              { icon: "rename", label: t("Edit"), run: async () => (await look.useTheme(th, ref, th.name), editHalf(look.dark ? "dark" : "light")) },
              { id: "rename", icon: "lines", label: t("Rename"), run: () => {} },
              { icon: "copy", label: t("Duplicate"), run: () => look.duplicateTheme(th, th.name) },
              { icon: "download", label: t("Export this theme"), run: () => look.exportLook("theme", th) },
              { icon: "trash", label: t("Delete…"), danger: true, run: () => look.deleteTheme(th.id) },
            ]}
          />
        {:else}
          <button class="new" onclick={() => editCopy(look.board, t("My theme"))}>
            <span class="newb"><Icon name="plus" size={14} stroke={2.2} /></span>
            <span class="muted">{t("New theme")}</span>
          </button>
        {/if}
      {/each}
    </div>
  </section>
</div>

<div class="foot">
  <div class="tf">
    <span class="muted">{t("Move your look to another PC")}</span>
    <span class="links">
      <button onclick={() => go("share")}><Icon name="download" size={12} stroke={2} />{t("Export")}</button>
      <button onclick={() => (look.importing = null, look.importError = null, go("share"))}><Icon name="import" size={12} stroke={2} />{t("Import")}</button>
    </span>
  </div>
</div>

<style>
  .now {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--chip) 55%, var(--card));
    font-size: 12px;
    color: var(--soft);
  }
  .now b {
    color: var(--ink);
    font-weight: 600;
  }
  .hint {
    font-size: 11.5px;
  }
  .new {
    padding: 0;
    border: 0;
    background: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
    text-align: left;
    cursor: pointer;
    font-size: 11.5px;
  }
  .newb {
    height: 58px;
    border-radius: 9px;
    border: 1.5px dashed color-mix(in srgb, var(--ink) 22%, transparent);
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .tf {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .links {
    display: flex;
    gap: 14px;
  }
  .links button {
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
</style>
