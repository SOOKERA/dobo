<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { untrack } from "svelte";
  import { roving } from "../lib/gridkeys";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { swatchesFor, defaultCustom } from "../lib/palette";
  import { pushLayer } from "../lib/layers";
  import { closeOnLeave, keepInView, modal, takeFocus } from "../lib/modal";
  import { look } from "../lib/look.svelte";
  import { hexToRgb, rgbStrToHex } from "../lib/looks";
  import { saveNotePicture } from "../lib/snapshot";
  import type { Note, PaletteName, Paper } from "../lib/types";
  import { fold, pop } from "../lib/motion";
  import { t, tMaybe } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";
  import { atPointer, type Point } from "../lib/rightclick.svelte";
  import { noteAsText } from "../lib/astext";

  const papers: [Paper, "Plain" | "Ruled" | "Grid" | "Antique"][] = [
    ["plain", "Plain"],
    ["ruled", "Ruled"],
    ["grid", "Grid"],
    ["antique", "Antique"],
  ];
  let saving = $state(false);
  async function picture() {
    const card = menuEl.closest(".card") as HTMLElement | null;
    if (!card || saving) return;
    saving = true;
    onClose();
    try {
      await new Promise((r) => requestAnimationFrame(() => r(null)));
      await saveNotePicture(card, noteTitle(note));
    } catch (e) {
      store.showToast(() => t("Couldn’t save the picture: {error}", { error: String(e) }));
    } finally {
      saving = false;
    }
  }

  // (`at`: opened by a right-click, at the pointer; else under its ⋯.)
  let { note, onClose, at = null }: { note: Note; onClose: () => void; at?: Point | null } = $props();
  /** How it comes in: from above, or from below when it opens above the pointer. */
  const motion = { y: -4 };

  // Opens on the note's own palette tab.
  let tab = $state<PaletteName>(untrack(() => note.palette));
  let confirmDelete = $state(false);
  /** "Keep" answered from the keyboard: it goes back to "Delete note…". */
  let kept = $state(false);
  let editing = $state<number | null>(null);
  let draftName = $state("");
  let draftDot = $state("#c8643c");
  let menuEl: HTMLDivElement;

  let isDark = $derived(look.dark);

  let swatches = $derived(swatchesFor(tab, store.custom));
  let current = $derived(note.palette === tab ? note.color : null);
  let pickedName = $derived(swatchesFor(note.palette, store.custom).find((s) => s.key === note.color)?.name ?? "");

  $effect(() => {
    const away = (e: PointerEvent) => {
      const t = e.target as HTMLElement;
      if (menuEl.contains(t)) return;
      if (t.closest(".more, .dot") && t.closest(".card") === menuEl.closest(".card")) return;
      onClose();
    };
    const pop = pushLayer(onClose);
    window.addEventListener("pointerdown", away, true);
    return () => {
      pop();
      window.removeEventListener("pointerdown", away, true);
    };
  });

  function startEdit(i: number) {
    editing = i;
    draftName = store.custom[i].name;
    draftDot = store.custom[i].dot;
  }
  async function saveEdit() {
    if (editing == null) return;
    const list = store.custom.map((c, i) => (i === editing ? { name: draftName.trim() || c.name, dot: draftDot } : c));
    await store.setCustom(list);
    await store.setColour(note, "custom", String(editing));
    // Kept with your colours (the same ones the watercolour Customise page shows).
    void look.addColour(hexToRgb(draftDot).join(","));
    editing = null;
  }
</script>

<!-- (The keyboard comes in, on the note's colour, and goes back to ⋯ when it closes.) -->
<div
  class="menu"
  use:atPointer={{ at, motion }}
  transition:pop|global={motion}
  bind:this={menuEl}
  role="dialog"
  aria-label={t("Note options")}
  use:modal={{ initial: () => menuEl?.querySelector<HTMLElement>(".swatch.on") ?? menuEl?.querySelector<HTMLElement>(".swatch"), trap: false }}
  use:closeOnLeave={onClose}
  use:keepInView={confirmDelete}
>
  <div class="row-head">
    <span class="label">{t("Colour")}</span>
    <span class="picked">{tMaybe(pickedName)}</span>
  </div>
  <div class="tabs" role="tablist" aria-label={t("Palette")} use:roving>
    {#each [["soft", "Soft"], ["vivid", "Vivid"], ["custom", "Custom"]] as const as [key, label] (key)}
      <button role="tab" id="palette-{key}" aria-controls="palette-swatches" class="tab" class:on={tab === key} aria-selected={tab === key} onclick={() => ((tab = key as PaletteName), (editing = null))}>{t(label)}</button>
    {/each}
  </div>
  <div class="swatches" id="palette-swatches" role="tabpanel" aria-labelledby="palette-{tab}">
    {#each swatches as s, i (s.key)}
      {@const tone = isDark ? s.dark : s.light}
      <button
        class="swatch"
        class:on={current === s.key}
        aria-label={current === s.key ? t("{name}, selected", { name: tMaybe(s.name) }) : tMaybe(s.name)}
        data-tip={tMaybe(s.name)}
        onclick={() => store.setColour(note, tab, s.key)}
        oncontextmenu={(e) => {
          // (Selected words right-clicked here: the Copy menu took it.)
          if (tab !== "custom" || e.defaultPrevented) return;
          e.preventDefault();
          startEdit(i);
        }}
      >
        <span class="disc" style:background={tone.bg}><span style:background={s.light.dot}></span></span>
      </button>
    {/each}
  </div>
  {#if tab === "custom"}
    <div transition:fold>
    {#if editing == null}
      <div class="custom-foot" transition:fold>
        <span>{t("Right-click a colour to change it")}</span>
        <button class="small" onclick={() => startEdit(Math.max(0, Number(current ?? 0)))}>{t("Edit")}</button>
      </div>
    {:else}
      <div transition:fold>
      <div class="editor">
        <label class="color-field">
          <span class="visually-hidden">{t("Colour")}</span>
          <input type="color" bind:value={draftDot} />
        </label>
        <label class="name-field">
          <span class="visually-hidden">{t("Name")}</span>
          <input type="text" bind:value={draftName} maxlength="18" />
        </label>
        <button class="small strong" onclick={saveEdit}>{t("Save")}</button>
      </div>
      {#if look.colours.length}
        <div class="yours" role="group" aria-label={t("Your colours")}>
          {#each look.colours.slice(0, 12) as c (c.rgb)}
            {@const hex = rgbStrToHex(c.rgb)}
            <button class="ydot" class:on={draftDot.toUpperCase() === hex} style:background={hex} data-tip={t("Your colour {hex}", { hex })} aria-label={t("Your colour {hex}", { hex })} onclick={() => (draftDot = hex.toLowerCase())}></button>
          {/each}
        </div>
      {/if}
      <button class="link" onclick={() => ((draftDot = defaultCustom[editing ?? 0].dot), (draftName = tMaybe(defaultCustom[editing ?? 0].name)))}>{t("Back to {name} (Soft)", { name: tMaybe(defaultCustom[editing ?? 0].name) })}</button>
      </div>
    {/if}
    </div>
  {/if}

  <div class="divider"></div>

  <div class="row-head">
    <span class="label">{t("Space")}</span>
    <div class="seg" role="group" aria-label={t("Space")}>
      <button class:on={note.space === "work"} aria-pressed={note.space === "work"} onclick={() => store.setNoteSpace(note, "work")}>{t("Work")}</button>
      <button class:on={note.space === "personal"} aria-pressed={note.space === "personal"} onclick={() => store.setNoteSpace(note, "personal")}>{t("Personal")}</button>
    </div>
  </div>

  <div class="row-head">
    <span class="label">{t("Paper")}</span>
    <div class="papers" role="radiogroup" use:roving aria-label={t("Paper")}>
      {#each papers as [key, label] (key)}
        <button role="radio" aria-checked={(note.paper ?? "plain") === key} class="paper p-{key}" class:on={(note.paper ?? "plain") === key} data-tip={t(label)} aria-label={t(label)} onclick={() => store.setPaper(note, key)}
          ><span></span></button
        >
      {/each}
    </div>
  </div>

  <div class="divider"></div>

  <button
    class="item"
    onclick={() => {
      store.target = note.id;
      onClose();
      document.getElementById("capture")?.focus();
    }}><span>{t("Write into this note")}</span><Icon name="plus" size={14} /></button
  >
  <button
    class="item"
    onclick={() => {
      store.setWide(note, !note.wide);
      onClose();
    }}><span>{note.wide ? t("Make normal width") : t("Make wide")}</span><span class="keys">{t("or drag its edge")}</span></button
  >
  <button
    class="item"
    onclick={() => {
      store.setFolded(note, !note.folded);
      onClose();
    }}><span>{note.folded ? t("Unfold") : t("Fold to its title")}</span><span class="keys">F</span></button
  >
  <button
    class="item"
    onclick={() => {
      store.setDoneLast(note, !note.done_last);
      onClose();
    }}><span>{note.done_last ? t("Keep done tasks in place") : t("Move done tasks to the end")}</span></button
  >
  <button
    class="item"
    onclick={() => {
      store.saveTemplate(note);
      onClose();
    }}><span>{t("Save as a template")}</span><Icon name="template" size={14} /></button
  >
  <button class="item" onclick={picture} disabled={saving}><span>{t("Save as a picture")}</span><span class="keys">{t("copies it too")}</span></button>
  <button
    class="item"
    onclick={() => {
      void store.copy(noteAsText(noteTitle(note), store.itemsByNote.get(note.id) ?? [], (id) => store.stepsOf.get(id) ?? [], note.done_last));
      onClose();
    }}><span>{t("Copy as text")}</span></button
  >
  <button
    class="item"
    onclick={() => {
      store.archive(note);
      onClose();
    }}><span>{t("Archive note")}</span></button
  >
  {#if !confirmDelete}
    <button class="item danger" use:takeFocus={kept} onfocus={() => (kept = false)} transition:fold={{ duration: 220 }} onclick={() => (confirmDelete = true)}><span>{t("Delete note…")}</span><Icon name="trash" size={14} /></button>
  {:else}
    <div class="confirm" transition:fold={{ duration: 260 }}>
      <span>{t("Delete “{name}” and its lines?", { name: noteTitle(note) })}</span>
      <div class="confirm-actions">
        <!-- (The keyboard comes to the safe choice.) -->
        <button class="small" use:takeFocus onclick={() => ((kept = true), (confirmDelete = false))}>{t("Keep")}</button>
        <button class="small danger-fill" onclick={() => store.deleteNote(note)}>{t("Delete")}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .menu {
    position: absolute;
    top: 44px;
    right: -6px;
    width: 272px;
    z-index: 30;
    padding: var(--s3);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--pop-shadow);
    transform-origin: 90% 0;
    cursor: default;
  }
  .row-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 2px var(--s2);
    min-height: 30px;
  }
  .label {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .picked {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-base);
    color: var(--muted);
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 2px;
    padding: 3px;
    margin-bottom: var(--s3);
    border-radius: var(--r-md);
    background: var(--chip);
  }
  .tab {
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--soft);
    font-size: var(--t-sm);
    cursor: pointer;
  }
  .tab.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--s2);
    justify-items: center;
  }
  .swatch {
    width: 38px;
    height: 38px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    background: transparent;
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s var(--ease-out);
  }
  .swatch:hover {
    transform: scale(1.06);
  }
  .swatch.on {
    border-color: var(--ink);
  }
  .disc {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
    display: grid;
    place-items: center;
  }
  .disc span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .custom-foot {
    margin-top: var(--s3);
    padding-top: 10px;
    border-top: 1px solid var(--line);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .editor {
    margin-top: var(--s3);
    padding-top: 10px;
    border-top: 1px solid var(--line);
    display: flex;
    gap: var(--s2);
    align-items: center;
  }
  .color-field input {
    width: 36px;
    height: 32px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    cursor: pointer;
  }
  .name-field {
    flex: 1;
    min-width: 0;
  }
  .name-field input {
    width: 100%;
    height: 32px;
    padding: 0 var(--s2);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
  }
  .yours {
    margin-top: var(--s2);
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .ydot {
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.1);
    transition: transform 0.15s var(--ease-out);
  }
  .ydot:hover {
    transform: scale(1.12);
  }
  .ydot.on {
    box-shadow: 0 0 0 2px var(--card), 0 0 0 3.5px var(--ink);
  }
  .link {
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .small {
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--card);
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--ink);
    cursor: pointer;
  }
  .small.strong {
    height: 32px;
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .divider {
    height: 1px;
    margin: var(--s3) calc(-1 * var(--s3)) 6px;
    background: var(--line);
  }
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
  }
  .seg button {
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .item {
    width: 100%;
    height: 36px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: 7px;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--t-base);
    color: var(--ink);
    cursor: pointer;
  }
  .item:hover {
    background: var(--hover);
  }
  .item.danger {
    color: var(--late-ink);
  }
  .item:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .keys {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .papers {
    display: flex;
    gap: 4px;
  }
  .paper {
    width: 34px;
    height: 30px;
    padding: 3px;
    border: 1.5px solid transparent;
    border-radius: 7px;
    background: transparent;
    cursor: pointer;
  }
  .paper.on {
    border-color: var(--ink);
  }
  .paper span {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 4px;
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .p-ruled span {
    background-image: repeating-linear-gradient(to bottom, transparent 0 4px, var(--line) 4px 5px);
  }
  .p-grid span {
    background-image: repeating-linear-gradient(to bottom, transparent 0 4px, var(--line) 4px 5px), repeating-linear-gradient(to right, transparent 0 4px, var(--line) 4px 5px);
  }
  .p-antique span {
    background: var(--antique);
  }
  .confirm {
    padding: var(--s2);
    border-radius: var(--r-md);
    background: var(--hover);
    font-size: var(--t-md);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .danger-fill {
    background: var(--danger-fill);
    border-color: var(--danger-fill);
    color: #fff;
  }
</style>
