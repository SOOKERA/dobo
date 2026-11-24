<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // New note, empty or from a template (Review board): Dobo's own first (Today, Ideas, Plan:
  // never hidden nor deleted, with blank lines to fill in), then the person's. Any note can be
  // saved as a template from its menu. Opened from the board's right-click menu, its note goes
  // where the board was clicked (store.templateAt).
  import { composing } from "../lib/compose";
  import { roving } from "../lib/gridkeys";
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { store, type Template } from "../lib/store.svelte";
  import { swatchesFor } from "../lib/palette";
  import { look } from "../lib/look.svelte";
  import { pushLayer } from "../lib/layers";
  import { modal } from "../lib/modal";
  import { isWindowDrag } from "../lib/windrag";
  import { arrive, easeHeight, fold, leave, rise, veil } from "../lib/motion";
  import { around, t } from "../lib/i18n.svelte";
  import { builtInTemplates, isBlank } from "../lib/newNote";
  import { toastAbove } from "../lib/toastroom.svelte";

  let { onClose }: { onClose: () => void } = $props();
  $effect(() => pushLayer(onClose));

  let chosen = $state<number | null>(null);
  let title = $state("");
  let busy = $state(false);
  // (In the language of the moment: a note made from one is named in it.)
  let builtIns = $derived(builtInTemplates());
  let tpl = $derived<Template | null>([...builtIns, ...store.templates].find((x) => x.id === chosen) ?? null);
  let dark = $derived(look.dark);
  /** "Make your own: Save as a template, in any note’s ⋯ menu.", the menu's words in bold. */
  let hint = $derived(around(t("Make your own: {action}, in any note’s ⋯ menu."), "{action}"));

  function dotOf(tp: Template | null): string {
    if (!tp) return "#b5b0a5";
    const s = swatchesFor(tp.palette, store.custom).find((x) => x.key === tp.color);
    return s?.light.dot ?? "#b5b0a5";
  }
  function tintOf(tp: Template): string {
    const s = swatchesFor(tp.palette, store.custom).find((x) => x.key === tp.color);
    return (dark ? s?.dark.bg : s?.light.bg) ?? "transparent";
  }

  /** A template deleted: the keyboard goes to the one now at its place (else the one before, else
   * Blank note), never nowhere. */
  async function deleteTemplate(e: MouseEvent, id: number) {
    const list = (e.currentTarget as HTMLElement).closest(".list");
    const i = store.templates.findIndex((x) => x.id === id);
    await store.deleteTemplate(id);
    if (chosen === id) chosen = null;
    await tick();
    const opts = [...(list?.querySelectorAll<HTMLElement>(".opt") ?? [])].filter((o) => !o.closest("[inert]"));
    // (Blank note is the first, then Dobo's own.)
    opts[Math.min(i + 1 + builtIns.length, opts.length - 1)]?.focus({ preventScroll: true });
  }

  async function create() {
    if (busy) return;
    busy = true;
    try {
      await store.fromTemplate(tpl, title, { at: store.templateAt });
      onClose();
    } finally {
      busy = false;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<!-- (Floating, a press on the veil over the top bar moves the window and keeps this open.) -->
<div class="veil" data-drag-through onclick={(e) => !isWindowDrag(e) && onClose()} transition:veil|global></div>
<div class="dialog" role="dialog" aria-label={t("New note")} transition:rise|global={{ y: -6 }} use:modal={{ initial: "input" }}>
  <div class="top">
    <h2>{t("New note")}</h2>
    <button class="x" aria-label={t("Close")} data-tip={t("Close")} data-tip-keys="Esc" onclick={onClose}><Icon name="close" size={16} /></button>
  </div>

  <label class="name">
    <span class="visually-hidden">{t("Title")}</span>
    <!-- (The keyboard comes here when it opens: use:modal.) -->
    <input
      bind:value={title}
      placeholder={tpl ? tpl.name : t("Title of the note")}
      autocomplete="off"
      onkeydown={(e) => {
        if (e.key === "Enter" && !composing(e)) create();
      }}
    />
  </label>

  <div class="list" role="radiogroup" use:roving aria-label={t("Start from")}>
    <button role="radio" aria-checked={chosen == null} class="opt" class:on={chosen == null} onclick={() => (chosen = null)}>
      <span class="dot" style:background={dotOf(null)}></span>
      <span class="what"><b>{t("Blank note")}</b><small>{t("Nothing inside")}</small></span>
    </button>
    {#each builtIns as tp (tp.id)}
      <button role="radio" aria-checked={chosen === tp.id} class="opt" class:on={chosen === tp.id} onclick={() => (chosen = tp.id)}>
        <span class="dot" style:background={dotOf(tp)}></span>
        <span class="what"><b>{tp.name}</b><small>{tp.about}</small></span>
      </button>
    {/each}
    {#each store.templates as tp (tp.id)}
      <div class="opt-row" transition:fold={{ gap: 2 }}>
        <button role="radio" aria-checked={chosen === tp.id} class="opt" class:on={chosen === tp.id} onclick={() => (chosen = tp.id)}>
          <span class="dot" style:background={dotOf(tp)}></span>
          <span class="what"
            ><b>{tp.name}</b><small
              >{t("{n} tasks", { n: tp.lines.filter((l) => l.kind === "task").length })}{tp.lines.some((l) => l.kind === "idea") ? `, ${t("{n} ideas", { n: tp.lines.filter((l) => l.kind === "idea").length })}` : ""}</small
            ></span
          >
        </button>
        <button class="del" aria-label={t("Delete the template {name}", { name: tp.name })} data-tip={t("Delete this template")} onclick={(e) => deleteTemplate(e, tp.id)}
          ><Icon name="trash" size={14} /></button
        >
      </div>
    {/each}
    <p class="hint">{hint[0]}<b>{t("Save as a template")}</b>{hint[1]}</p>
  </div>

  {#if tpl}
    <div class="pv" transition:fold={{ gap: 12 }}>
    <span class="label">{t("Preview")}</span>
    <!-- Another template: its preview crossfades in and the box eases to its height. -->
    <div class="preview" use:easeHeight>
      <div class="pv-stack">
      {#key tpl.id}
      <div class="pv-face" in:arrive={{ delay: 90, duration: 240, y: 0 }} out:leave={{ duration: 120, y: 0 }}>
      <div class="p-head"><span class="dot" style:background={dotOf(tpl)}></span><span class="p-title">{title.trim() || tpl.name}</span></div>
      {#each tpl.lines.filter((l) => l.kind === "task") as l, i (i)}
        <!-- (A blank line: its box and a faint rule, to write on.) -->
        <div class="p-task" class:blank={isBlank(l)}><span class="box"></span>{#if isBlank(l)}<span class="rule"></span>{:else}{l.text}{l.deadline ? " !" : ""}{/if}</div>
        {#each l.steps ?? [] as s, k (k)}<div class="p-task step"><span class="box"></span>{s}</div>{/each}
      {/each}
      {#if tpl.lines.some((l) => l.kind === "idea")}
        <div class="p-ideas" style:background={tintOf(tpl)}>
          {#each tpl.lines.filter((l) => l.kind === "idea") as l, i (i)}{#if isBlank(l)}<span class="rule idea"></span>{:else}<span>{l.text}</span>{/if}{/each}
        </div>
      {/if}
      </div>
      {/key}
      </div>
    </div>
    </div>
  {/if}

  <div class="foot" use:toastAbove>
    <button class="create" onclick={create} disabled={busy}>{t("Create note")}</button>
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    z-index: 44;
    background: rgba(20, 16, 10, 0.18);
  }
  .dialog {
    position: absolute;
    left: 0;
    right: 0;
    margin: 0 auto;
    /* Under the top bar (one row or two), which stays free to move a floating window by. */
    top: calc(var(--head-h, 60px) + 12px);
    z-index: 45;
    width: min(440px, calc(100% - 32px));
    max-height: calc(100% - var(--head-h, 60px) - 36px);
    overflow-y: auto;
    padding: var(--s4) var(--s5) var(--s5);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 16px;
    box-shadow: var(--pop-shadow);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  @media (prefers-reduced-motion: reduce) {
    .veil,
    .dialog {
      animation: none;
    }
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h2 {
    margin: 0;
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 500;
  }
  .x {
    width: 32px;
    height: 32px;
    margin-right: -8px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
  }
  .name input {
    width: 100%;
    height: 44px;
    padding: 0 14px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--ground);
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--ink);
    outline: 0;
  }
  .name input:focus {
    border-color: var(--box);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .opt-row {
    position: relative;
    display: flex;
  }
  .opt {
    flex: 1;
    min-height: 48px;
    padding: 6px 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    color: var(--ink);
    cursor: pointer;
  }
  .opt:hover {
    background: var(--hover);
  }
  .opt.on {
    background: var(--chip);
  }
  .dot {
    flex-shrink: 0;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .what {
    display: flex;
    flex-direction: column;
  }
  .what b {
    font-size: var(--t-base);
    font-weight: 600;
  }
  .what small {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .del {
    position: absolute;
    right: 6px;
    top: 50%;
    width: 30px;
    height: 30px;
    margin-top: -15px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
  }
  .opt-row:hover .del,
  .del:focus-visible {
    opacity: 1;
  }
  .del:hover {
    background: var(--card);
    color: var(--late-ink);
  }
  /* In line with the options' names (12 px in, the dot, its gap). */
  .hint {
    margin: 4px 12px 4px 32px;
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .hint b {
    font-weight: 600;
    color: var(--soft);
  }
  .label {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .pv {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .pv-stack {
    position: relative;
    display: grid;
  }
  .pv-face {
    grid-area: 1 / 1;
    min-width: 0;
  }
  .preview {
    border: 1px solid var(--line);
    border-radius: 12px;
    overflow: hidden;
  }
  .p-head {
    padding: 10px 14px 4px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .p-title {
    font-family: var(--serif);
    font-size: 16px;
    font-weight: 600;
    color: var(--muted);
  }
  .p-task {
    display: flex;
    gap: 10px;
    padding: 4px 14px;
    font-size: var(--t-md);
  }
  .p-task.step {
    padding-left: 38px;
  }
  .p-task .box {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    margin-top: 2px;
    border: 1.5px solid var(--box);
    border-radius: 4px;
  }
  /* A blank line: a faint dotted rule where its words would be. */
  .rule {
    flex: 1;
    max-width: 62%;
    height: 1px;
    margin-top: 13px;
    background: repeating-linear-gradient(to right, color-mix(in srgb, var(--box) 70%, transparent) 0 2px, transparent 2px 5px);
  }
  .p-task.blank .box {
    border-style: dashed;
  }
  .p-task.blank {
    min-height: 26px;
  }
  .rule.idea {
    flex: none;
    width: 55%;
    margin: 10px 0 8px;
    background: repeating-linear-gradient(to right, color-mix(in srgb, currentColor 35%, transparent) 0 2px, transparent 2px 5px);
  }
  .p-ideas {
    margin-top: 6px;
    padding: 8px 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-family: var(--serif);
    font-style: italic;
    font-size: 15px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
  }
  .create {
    height: 40px;
    padding: 0 18px;
    border: 0;
    border-radius: 10px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-base);
    font-weight: 600;
    cursor: pointer;
  }
  .create:disabled {
    opacity: 0.6;
  }
</style>
