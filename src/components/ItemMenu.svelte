<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A task's menu: its time, Reminder or Deadline (and the deadline's rhythm), add a step,
  // make it an idea, delete.
  import { composing } from "../lib/compose";
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import RhythmFields from "./RhythmFields.svelte";
  import RepeatFields from "./RepeatFields.svelte";
  import { store } from "../lib/store.svelte";
  import { parseCapture } from "../lib/capture";
  import { formatDueLong } from "../lib/time";
  import { presetOf, rhythmPresets } from "../lib/rhythm";
  import { repeatLabel } from "../lib/repeat";
  import { pushLayer } from "../lib/layers";
  import type { Item, PresetName, Rhythm } from "../lib/types";
  import { fadeQuick, fold, pop, widen } from "../lib/motion";
  import { keysText, t } from "../lib/i18n.svelte";

  const PRESET_NAME = { gentle: "Gentle", normal: "Normal", insistent: "Insistent", custom: "Custom" } as const;
  let { item, onClose, onAddStep }: { item: Item; onClose: () => void; onAddStep: () => void } = $props();

  let menuEl: HTMLDivElement;
  let when = $state("");
  let whenError = $state(false);
  let preset = $state<PresetName>(untrack(() => presetOf(item, store.settings)));
  let custom = $state<Rhythm | null>(null);
  let presets = $state<Record<string, Rhythm> | null>(null);
  let repeatOpen = $state(untrack(() => !!item.repeat));

  $effect(() => {
    rhythmPresets().then((p) => {
      presets = p;
      const own = item.rhythm?.preset === "custom" ? (item.rhythm as Rhythm & { preset: "custom" }) : null;
      custom = own ? { start: own.start, pre: own.pre, every: own.every, max: own.max, checkin: own.checkin, notyet: own.notyet } : { ...((store.settings.deadline_custom as Rhythm) ?? p.normal) };
    });
  });

  $effect(() => {
    const away = (e: PointerEvent) => {
      const t = e.target as HTMLElement;
      if (menuEl.contains(t) || t.closest(`[data-item-menu="${item.id}"]`)) return;
      onClose();
    };
    const pop = pushLayer(onClose);
    window.addEventListener("pointerdown", away, true);
    return () => {
      pop();
      window.removeEventListener("pointerdown", away, true);
    };
  });

  let parsed = $derived(when.trim() ? parseCapture(`x ${when}`, "task", store.now).due : null);

  async function setWhen() {
    if (!parsed) {
      whenError = !!when.trim();
      return;
    }
    await store.setDue(item, parsed.getTime());
    when = "";
    whenError = false;
  }

  async function pick(p: PresetName) {
    preset = p;
    if (p !== "custom") await store.setAlarm(item, "deadline", { preset: p });
  }

  async function saveCustom(r: Rhythm) {
    custom = r;
    await store.setAlarm(item, "deadline", { preset: "custom", ...r });
  }
</script>

<div class="menu" transition:pop|global bind:this={menuEl} role="dialog" aria-label={t("Task options")}>
  <div class="head">
    <span class="label">{t("When")}</span>
    <span class="current">{item.due_at != null ? formatDueLong(item.due_at, store.now) : t("No time")}</span>
  </div>
  <div class="when">
    <label class="visually-hidden" for="when-{item.id}">{t("New time")}</label>
    <input
      id="when-{item.id}"
      bind:value={when}
      class:bad={whenError}
      placeholder={t("demain 10h, 3pm, friday…")}
      autocomplete="off"
      onkeydown={(e) => {
        if (e.key === "Enter" && !composing(e)) {
          e.preventDefault();
          setWhen();
        }
      }}
    />
    {#if parsed}<span class="chip" transition:widen={{ gap: 6 }}>{formatDueLong(parsed.getTime(), store.now)}</span>{/if}
    <button class="small strong" onclick={setWhen} disabled={!when.trim()}>{t("Set")}</button>
  </div>
  <div class="links">
    <button
      class="link"
      onclick={() => {
        onClose();
        store.ruler = { item, mode: "click" };
      }}>{t("Pick on today’s hours")}</button
    >
    {#if item.due_at != null}
      <button class="link" transition:fadeQuick onclick={() => store.setDue(item, null)}>{t("Remove the time")}</button>
    {/if}
  </div>

  <div class="divider"></div>
  <button class="head head-btn" aria-expanded={repeatOpen} onclick={() => (repeatOpen = !repeatOpen)}>
    <span class="label">{t("Repeat")}</span>
    <span class="current">{item.repeat ? repeatLabel(item.repeat) : t("Never")}<Icon name={repeatOpen ? "chev-up" : "chev-down"} size={13} stroke={2.2} /></span>
  </button>
  {#if repeatOpen}<div transition:fold><RepeatFields {item} /></div>{/if}

  {#if item.due_at != null}
    <div class="span" transition:fold>
    <div class="divider"></div>
    <div class="head">
      <span class="label">{t("Reminder")}</span>
      <span class="keys">{t("end a line with ! for a deadline")}</span>
    </div>
    <div class="kinds" role="radiogroup" aria-label={t("Kind of reminder")}>
      <button role="radio" aria-checked={item.alarm !== "deadline"} class:on={item.alarm !== "deadline"} onclick={() => store.setAlarm(item, "reminder")}>
        <Icon name="bell" size={15} />
        <span><b>{t("Reminder")}</b><small>{t("Rings at the time")}</small></span>
      </button>
      <button
        role="radio"
        aria-checked={item.alarm === "deadline"}
        class:on={item.alarm === "deadline"}
        onclick={() => store.setAlarm(item, "deadline", item.rhythm ?? (preset === "custom" && custom ? { preset: "custom", ...custom } : { preset: preset as "normal" }))}
      >
        <Icon name="hourglass" size={15} />
        <span><b>{t("Deadline")}</b><small>{t("The rhythm")}</small></span>
      </button>
    </div>
    {#if item.alarm === "deadline"}
      <div transition:fold>
      <div class="tabs" role="tablist" aria-label={t("Rhythm")}>
        {#each ["gentle", "normal", "insistent", "custom"] as const as p (p)}
          <button role="tab" class:on={preset === p} aria-selected={preset === p} onclick={() => pick(p as PresetName)}>{t(PRESET_NAME[p])}</button>
        {/each}
      </div>
      {#if preset === "custom" && custom}
        <div transition:fold><RhythmFields value={custom} onSave={saveCustom} compact /></div>
      {:else if presets}
        {@const r = presets[preset]}
        <p class="summary" transition:fold>
          {t("Starts {start} min before, again at {pre} min before, then every {every} min (at most {max}). “Really done?” {checkin} min after “I'm on it”.", { start: r.start, pre: r.pre.join(" · "), every: r.every, max: r.max, checkin: r.checkin })}
        </p>
      {/if}
      </div>
    {/if}
    </div>
  {/if}

  <div class="divider"></div>
  {#if item.parent_id == null}
    <button
      class="item"
      onclick={() => {
        onClose();
        onAddStep();
      }}><span>{t("Add a step")}</span><span class="keys">{t("or drop a line on it")}</span></button
    >
  {/if}
  <button
    class="item"
    onclick={() => {
      store.flipKind(item);
      onClose();
    }}><span>{t("Make it an idea")}</span><span class="keys">{keysText("Backspace")}</span></button
  >
  <button
    class="item danger"
    onclick={() => {
      store.deleteItem(item);
      onClose();
    }}><span>{t("Delete")}</span><span class="keys">{keysText("Del")}</span></button
  >
</div>

<style>
  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    width: 300px;
    max-width: calc(100cqw - 8px);
    z-index: 40;
    padding: var(--s3);
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--pop-shadow);
    transform-origin: 90% 0;
    cursor: default;
    text-align: left;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 0 2px var(--s2);
  }
  .label {
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .current {
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-base);
    color: var(--muted);
  }
  .when {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 4px 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--ground);
  }
  .when input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-base);
    color: var(--ink);
  }
  .when input::placeholder {
    color: var(--muted);
  }
  .when input.bad {
    color: var(--late-ink);
  }
  .chip {
    flex-shrink: 0;
    font-size: var(--t-xs);
    font-weight: 600;
    color: var(--soft);
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--chip);
  }
  .small {
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font-size: var(--t-sm);
    font-weight: 500;
    cursor: pointer;
  }
  .small.strong {
    background: var(--ink);
    color: var(--card);
    border-color: var(--ink);
  }
  .small:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .link {
    margin-top: 6px;
    padding: 2px;
    border: 0;
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
  }
  .kinds button {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: 48px;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    text-align: left;
    color: var(--soft);
    cursor: pointer;
  }
  .kinds button.on {
    border-color: var(--ink);
    box-shadow: 0 0 0 1px var(--ink);
    color: var(--ink);
  }
  .kinds span {
    display: flex;
    flex-direction: column;
  }
  .kinds b {
    font-size: var(--t-md);
    font-weight: 600;
  }
  .kinds small {
    font-size: var(--t-xs);
    color: var(--muted);
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 2px;
    margin-top: var(--s2);
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
  }
  .tabs button {
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    cursor: pointer;
  }
  .tabs button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .summary {
    margin: var(--s2) 2px 0;
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--muted);
  }
  /* A section that folds in and out: as wide as the menu, so its divider isn't cut while it moves. */
  .span {
    margin: 0 calc(-1 * var(--s3));
    padding: 0 var(--s3);
  }
  .divider {
    height: 1px;
    margin: var(--s3) calc(-1 * var(--s3)) 6px;
    background: var(--line);
  }
  .item {
    width: 100%;
    height: 34px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: var(--r-sm);
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
  .item > span:first-child {
    text-align: left;
  }
  .keys {
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
  }
  .item.danger .keys {
    color: inherit;
    opacity: 0.75;
  }
  .links {
    display: flex;
    gap: var(--s3);
  }
  .head-btn {
    width: 100%;
    border: 0;
    background: transparent;
    cursor: pointer;
    text-align: left;
    padding: 2px 2px var(--s2);
  }
  .head-btn .current {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
