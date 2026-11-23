<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Export & import: a small .ztheme file with your look (never your notes). Importing shows
  // what's inside first; nothing you have is replaced, a name already used gets "(2)".
  import Icon from "../../Icon.svelte";
  import { roving } from "../../../lib/gridkeys";
  import { look } from "../../../lib/look.svelte";
  import { OUR_THEMES } from "../../../lib/looks";
  import type { ExportScope } from "../../../lib/ztheme";
  import { t } from "../../../lib/i18n.svelte";
  import { fadeQuick, fold } from "../../../lib/motion";

  let theme = $derived(look.board.theme);
  let themeName = $derived(look.themeNameOf(theme) ?? t("This board’s pair"));
  let scope = $state<ExportScope>("theme");
  let busy = $state(false);

  async function save() {
    busy = true;
    try {
      // Our theme's name as it is underneath, so the file reads the same in either language.
      if (scope === "theme") await look.exportLook("theme", { name: look.themePair(theme)?.name ?? t("This board’s pair"), light: look.board.light, dark: look.board.dark });
      else await look.exportLook(scope);
    } finally {
      busy = false;
    }
  }
  let f = $derived(look.importing?.file ?? null);
  const names = (list: { name: string }[]) => list.map((x) => x.name).join(", ");
  void OUR_THEMES;
</script>

<div class="body">
  <section class="sec">
    <span class="lbl">{t("Export")}</span>
    <div class="opts" role="radiogroup" use:roving aria-label={t("What to export")}>
      <button role="radio" aria-checked={scope === "theme"} class="opt" class:on={scope === "theme"} onclick={() => (scope = "theme")}>
        <i></i><span><b>{t("This theme")}</b> · {themeName}<small>{t("its two styles and any custom colours they use")}</small></span>
      </button>
      <button role="radio" aria-checked={scope === "themes"} class="opt" class:on={scope === "themes"} disabled={!look.themes.length} onclick={() => (scope = "themes")}>
        <i></i><span><b>{t("All my themes")}</b> · {look.themes.length}<small>{t("with their styles and colours")}</small></span>
      </button>
      <button role="radio" aria-checked={scope === "everything"} class="opt" class:on={scope === "everything"} onclick={() => (scope = "everything")}>
        <i></i><span><b>{t("Everything")}</b><small>{t("themes, styles, watercolour mixes, your colours, dark-mode hours")}</small></span>
      </button>
    </div>
    <div class="row-h">
      <span class="muted">{t("Notes are never in it, only the look.")}</span>
      <button class="btn" disabled={busy} onclick={save}><Icon name="download" size={13} stroke={2} />{t("Save file…")}</button>
    </div>
  </section>

  <div class="rule"></div>

  <section class="sec">
    <div class="row-h">
      <span class="imp">{t("Import")}</span>
      <span class="muted">{t("or drop a .ztheme on the board")}</span>
    </div>
    <div class="drop">
      <span class="file">.ztheme</span>
      {#if look.importing}
        <span class="fn" in:fadeQuick={{ delay: 60, duration: 200 }}><b>{look.importing.name}</b><br /><span class="muted">{t("checked, safe to add")}</span></span>
        <button class="btn2" in:fadeQuick={{ delay: 60, duration: 200 }} onclick={() => look.readImport(null)}>{t("Change file")}</button>
      {:else}
        <span class="fn" in:fadeQuick={{ delay: 60, duration: 200 }}><span class="muted">{look.importError ?? t("A look someone shared, or your own from another PC.")}</span></span>
        <button class="btn2" in:fadeQuick={{ delay: 60, duration: 200 }} onclick={() => look.readImport(null)}>{t("Choose file…")}</button>
      {/if}
    </div>
    {#if f}
      <div class="import-more" transition:fold={{ gap: 8 }}>
      <div class="inside">
        {#if f.themes.length}<div><span>{t("Themes")}</span><span class="muted">{names(f.themes)}</span></div>{/if}
        {#if f.styles.length}<div><span>{t("Styles")}</span><span class="muted">{f.styles.length}</span></div>{/if}
        {#if f.mixes.length}<div><span>{t("Watercolour mixes")}</span><span class="muted">{f.mixes.length}</span></div>{/if}
        {#if f.colours.length}<div><span>{t("Your colours")}</span><span class="muted">{t("{n} new", { n: f.colours.filter((c) => !look.colours.some((x) => x.rgb === c.rgb)).length })}</span></div>{/if}
        {#if f.dark_mode}<div><span>{t("Dark-mode hours")}</span><span class="muted">{t("yours stay as they are")}</span></div>{/if}
      </div>
      <div class="tip">{t("Nothing you have is replaced. A name you already use gets")} <span class="nw">{t("“(2)”")}</span>.</div>
      <div class="acts">
        <button class="btn2" onclick={() => (look.importing = null)}>{t("Cancel")}</button>
        <button class="btn" onclick={() => look.addImport()}>{t("Add to my themes")}</button>
      </div>
      </div>
    {/if}
  </section>
</div>

<style>
  .import-more {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .opts {
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .opt {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--card);
    color: var(--ink);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .opt:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .opt b {
    font-weight: 600;
  }
  .opt.on {
    border-color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--ink);
  }
  .opt i {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 1px;
    border-radius: 50%;
    border: 1.6px solid var(--box);
  }
  .opt.on i {
    border: 5px solid var(--ink);
  }
  .opt small {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .rule {
    border-top: 1px solid var(--line);
  }
  .imp {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 500;
  }
  /* The file, its words beside it, and the button under the words (beside them, in a panel this
     narrow, the words went down to a word or two a line). */
  .drop {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: 12px;
    row-gap: 10px;
    align-items: center;
    padding: 12px;
    border-radius: 12px;
    border: 1.5px dashed color-mix(in srgb, var(--ink) 22%, transparent);
    background: color-mix(in srgb, var(--chip) 35%, var(--card));
  }
  .file {
    grid-row: 1 / 3;
    width: 38px;
    height: 46px;
    border-radius: 6px;
    background: var(--chip);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 6px;
    font-size: 9px;
    font-weight: 700;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  .drop > .btn2 {
    grid-column: 2;
    justify-self: start;
  }
  .fn {
    min-width: 0;
    font-size: 12.5px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .fn b {
    font-weight: 600;
  }
  .inside {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
  }
  .inside div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .inside .muted {
    text-align: right;
    overflow-wrap: anywhere;
  }
  .nw {
    white-space: nowrap;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
