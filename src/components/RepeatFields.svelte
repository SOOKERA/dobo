<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A task's repeat (Repeat board): never / every day / weekday / week (on days) / month (on a
  // date), every N, and when it ends. Saves as you choose.
  import { store } from "../lib/store.svelte";
  import { roving } from "../lib/gridkeys";
  import { firstOccurrence, repeatLabel } from "../lib/repeat";
  import { dayName, t } from "../lib/i18n.svelte";
  import type { Item, Repeat } from "../lib/types";
  import { fold, widen } from "../lib/motion";
  import DateField from "./DateField.svelte";

  let { item }: { item: Item } = $props();

  type Freq = Repeat["freq"] | "never";
  let rule = $derived(item.repeat);
  let freq = $derived<Freq>(rule?.freq ?? "never");
  const order = [1, 2, 3, 4, 5, 6, 0];

  function base(f: Repeat["freq"]): Repeat {
    const due = item.due_at != null ? new Date(item.due_at) : null;
    return {
      freq: f,
      interval: rule?.freq === f ? rule.interval : 1,
      days: f === "week" ? (rule?.days?.length ? rule.days : [due?.getDay() ?? 1]) : [],
      monthday: f === "month" ? (rule?.monthday ?? due?.getDate() ?? 1) : null,
      ends: rule?.ends ?? { kind: "never" },
      done: rule?.done ?? 0,
      ...(rule?.at ? { at: rule.at } : {}),
    };
  }

  /** The time a repeat gave the task here, when it had none: it goes again with the repeat
   * (back to Never), unless the time was changed meanwhile. */
  let added: number | null = null;

  async function save(r: Repeat | null) {
    if (!r) {
      const mine = added != null && item.due_at === added;
      added = null;
      return mine ? store.setRepeat(item, null, null) : store.setRepeat(item, null);
    }
    // No time yet: the first occurrence, at 09:00 (or the time it already has).
    let due: number | undefined;
    if (item.due_at == null) due = added = firstOccurrence(r, 9, 0, store.now).getTime();
    await store.setRepeat(item, r, due);
  }

  function pickFreq(f: Freq) {
    save(f === "never" ? null : base(f));
  }
  function toggleDay(d: number) {
    if (!rule) return;
    const days = rule.days?.includes(d) ? rule.days.filter((x) => x !== d) : [...(rule.days ?? []), d];
    if (!days.length) return;
    save({ ...rule, days: days.sort() });
  }
  function setInterval(v: string) {
    if (!rule) return;
    save({ ...rule, interval: Math.max(1, Math.min(52, Number(v) || 1)) });
  }
  function setMonthday(v: string) {
    if (!rule) return;
    save({ ...rule, monthday: Math.max(1, Math.min(31, Number(v) || 1)) });
  }
  function setEnds(kind: "never" | "after" | "on") {
    if (!rule) return;
    if (kind === "never") save({ ...rule, ends: { kind } });
    if (kind === "after") save({ ...rule, ends: { kind, count: rule.ends.kind === "after" ? rule.ends.count : 10 } });
    if (kind === "on") {
      const d = new Date(store.now);
      d.setMonth(d.getMonth() + 3);
      save({ ...rule, ends: { kind, date: rule.ends.kind === "on" ? rule.ends.date : d.getTime() } });
    }
  }
  const unit = (f: Freq, n: number) => t(f === "day" ? "day(s)" : f === "week" ? "week(s)" : "month(s)", { n });
</script>

<div class="rep">
  <div class="seg" role="radiogroup" use:roving aria-label={t("Repeat")}>
    {#each [["never", "Never"], ["day", "Day"], ["weekday", "Weekday"], ["week", "Week"], ["month", "Month"]] as const as [key, label] (key)}
      <button role="radio" aria-checked={freq === key} class:on={freq === key} onclick={() => pickFreq(key as Freq)} data-label={t(label)}>{t(label)}</button>
    {/each}
  </div>

  {#if rule}
    <div class="more" transition:fold={{ gap: 8 }}>
    {#if rule.freq === "week"}
      <div class="days" transition:fold={{ gap: 8 }} role="group" aria-label={t("Days")}>
        {#each order as d (d)}
          {@const on = rule.days?.includes(d)}
          <button class:on aria-pressed={on} onclick={() => toggleDay(d)}>{dayName(d)}</button>
        {/each}
      </div>
    {/if}
    {#if rule.freq !== "weekday"}
      <div class="line" transition:fold={{ gap: 8 }}>
        <span>{rule.freq === "week" ? t("Every (weeks)") : t("Every")}</span>
        <input type="number" min="1" max="52" value={rule.interval} aria-label={t("Every how many")} onchange={(e) => setInterval(e.currentTarget.value)} />
        <span>{unit(rule.freq, rule.interval)}</span>
        {#if rule.freq === "month"}
          <span transition:widen={{ gap: 6 }}>{t("on the")}</span>
          <input transition:widen={{ gap: 6 }} type="number" min="1" max="31" value={rule.monthday ?? 1} aria-label={t("Day of the month")} onchange={(e) => setMonthday(e.currentTarget.value)} />
        {/if}
      </div>
    {/if}
    <div class="line ends">
      <span>{t("Ends")}</span>
      <div class="seg small" role="radiogroup" use:roving aria-label={t("Ends")}>
        <button role="radio" aria-checked={rule.ends.kind === "never"} class:on={rule.ends.kind === "never"} onclick={() => setEnds("never")} data-label={t("Never")}>{t("Never")}</button>
        <button role="radio" aria-checked={rule.ends.kind === "after"} class:on={rule.ends.kind === "after"} onclick={() => setEnds("after")} data-label={t("After")}>{t("After")}</button>
        <button role="radio" aria-checked={rule.ends.kind === "on"} class:on={rule.ends.kind === "on"} onclick={() => setEnds("on")} data-label={t("On a date")}>{t("On a date")}</button>
      </div>
      {#if rule.ends.kind === "after"}
        <input transition:widen={{ gap: 6 }} type="number" min="1" max="999" value={rule.ends.count} aria-label={t("How many times")} onchange={(e) => rule && save({ ...rule, ends: { kind: "after", count: Math.max(1, Number(e.currentTarget.value) || 1) } })} />
        <span transition:widen={{ gap: 6 }}>{t("times")}</span>
      {/if}
      {#if rule.ends.kind === "on"}
        <span class="date" transition:widen={{ gap: 6 }}>
          <DateField value={rule.ends.date} label={t("Last day")} onChange={(date) => rule && save({ ...rule, ends: { kind: "on", date } })} />
        </span>
      {/if}
    </div>
    <p class="sum">{t("{label} · ticked, it comes back on its next day", { label: repeatLabel(rule) })}</p>
    </div>
  {/if}
</div>

<style>
  .more {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .rep {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  /* Each choice as wide as its word, the room left shared out evenly (five equal cells ran
     "Weekday" into "Week" in a narrow menu). */
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
  }
  .seg button {
    flex: 1 1 auto;
    height: 26px;
    padding: 0 5px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    cursor: pointer;
    white-space: nowrap;
  }
  /* Room for the word in bold, so picking one doesn't nudge its neighbours. */
  .seg button {
    display: grid;
    place-items: center;
  }
  .seg button::after {
    content: attr(data-label);
    height: 0;
    font-weight: 600;
    visibility: hidden;
    overflow: hidden;
  }
  .seg button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .days {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 3px;
  }
  .days button {
    height: 28px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font-size: var(--t-xs);
    color: var(--soft);
    cursor: pointer;
  }
  .days button.on {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
    font-weight: 600;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    font-size: var(--t-md);
    color: var(--soft);
  }
  .line input {
    width: 48px;
    height: 28px;
    padding: 0 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
  }
  .date {
    display: inline-flex;
  }
  .sum {
    margin: 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: var(--t-base);
    color: var(--muted);
    text-wrap: pretty;
  }
</style>
