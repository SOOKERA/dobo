<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A date ("19 Nov 2026", « 19 nov. 2026 »: the app's own way of writing it, in its language):
  // the day, the month and the year, each a spin button, as in the time field (TimeField). Up/Down
  // change it, Page Up/Down by a week, a quarter or 5 years, Home/End go to its ends, Left/Right
  // move between them, and digits type it ("1" "9" is the 19th, "3" is March, "2027"). The
  // browser's own date field followed the region of Windows, not the app. Saved once typing
  // stops, or on leaving the field, or with Enter; never while a part still waits for its next
  // digit. A day the month hasn't (the 31st typed in November) stays as typed until then: the
  // month or the year may come next; it is saved as the month's last day.
  import { untrack } from "svelte";
  import { dateFmt, i18n, t } from "../lib/i18n.svelte";
  import { daysIn, fitDay, keyDate, pausedYear, typeDateDigit, type DSeg, type Years, type YMD } from "../lib/datefield";
  let {
    value,
    label,
    onChange,
  }: {
    /** The day, as a time in it (ms). */
    value: number;
    /** What the field is, for screen readers ("Last day"). */
    label: string;
    /** The day chosen, at noon (ms). */
    onChange: (v: number) => void;
  } = $props();

  const ymd = (ms: number): YMD => {
    const d = new Date(ms);
    return [d.getFullYear(), d.getMonth(), d.getDate()];
  };
  const noon = ([y, mo, d]: YMD) => new Date(y, mo, d, 12).getTime();
  const same = (a: YMD, b: YMD) => a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
  // From this year (or the date's, if earlier) to 50 years on.
  const years: Years = untrack(() => [Math.min(new Date().getFullYear(), ymd(value)[0]), new Date().getFullYear() + 50]);

  /** Being changed, not yet saved. */
  let draft = $state<YMD | null>(null);
  /** Saved, until the setting comes back with it. */
  let sent = $state<number | null>(null);
  $effect(() => {
    void value;
    untrack(() => (sent = null));
  });
  let shown = $derived(draft ?? ymd(sent ?? value));
  let y = $derived(shown[0]);
  let mo = $derived(shown[1]);
  let d = $derived(shown[2]);
  const monthName = (m: number, long = false) => dateFmt({ month: long ? "long" : "short" }).format(new Date(2026, m, 15));
  // (Every month's name in one place, unseen: the month part is as wide as the widest, so the
  // field never changes width as the month goes round.)
  let months = $derived.by(() => {
    void i18n.lang;
    return Array.from({ length: 12 }, (_, m) => monthName(m));
  });
  /** A year being typed shows its digits so far. */
  let yearText = $state<string | null>(null);

  let dayEl = $state<HTMLElement>();
  let monthEl = $state<HTMLElement>();
  let yearEl = $state<HTMLElement>();
  const el = (s: DSeg) => (s === "d" ? dayEl : s === "mo" ? monthEl : yearEl);
  const order: DSeg[] = ["d", "mo", "y"];

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  /** The date being changed; saved once nothing else happens (`save`), or not yet. */
  function set(next: YMD, save = true) {
    draft = next;
    clearTimeout(saveTimer);
    if (save) saveTimer = setTimeout(commit, 900);
  }
  function commit() {
    clearTimeout(saveTimer);
    if (!draft) return;
    const v = fitDay(draft);
    draft = null;
    if (same(v, ymd(value))) return;
    sent = noon(v);
    onChange(sent);
  }

  // Digits typed into one part: the next ones complete it, or a pause starts again.
  let typed = "";
  let typedFor: DSeg | null = null;
  let typedTimer: ReturnType<typeof setTimeout> | undefined;
  /** The wait for a part's next digit ends: a year typed in part ("27") is taken as it is
   * (saved when `save`, else just shown until the next save). */
  function forget(save = true) {
    clearTimeout(typedTimer);
    const year = typedFor === "y" ? pausedYear(typed, years) : null;
    const waited = typedFor !== null;
    typed = "";
    typedFor = null;
    yearText = null;
    if (year != null) set(keyDate("y", "Home", shown, [year, year])!, save);
    else if (waited && save && draft) set(draft);
  }
  // (Leaving the menu while one is being changed: it is kept, at once; nothing waits after.)
  $effect(() => () => {
    forget(false);
    commit();
  });
  function move(from: DSeg, by: number) {
    const i = order.indexOf(from) + by;
    if (i >= 0 && i < order.length) el(order[i])?.focus();
  }

  function digit(seg: DSeg, n: number) {
    // (Another part's digits waited: they are kept as they are, saved with this one.)
    if (typedFor !== seg) forget(false);
    clearTimeout(typedTimer);
    const r = typeDateDigit(seg, typed, n, shown, years);
    const changed = !same(r.date, shown);
    if (r.done) {
      typed = "";
      typedFor = null;
      yearText = null;
      if (changed || draft) set(changed ? r.date : draft!);
      move(seg, 1);
    } else {
      // Waiting for the next digit: shown, saved when the wait ends.
      if (changed) set(r.date, false);
      else clearTimeout(saveTimer);
      typed = r.buf;
      typedFor = seg;
      yearText = seg === "y" ? r.buf : null;
      typedTimer = setTimeout(() => {
        forget(false);
        commit();
      }, 1500);
    }
  }

  function key(e: KeyboardEvent, seg: DSeg) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const k = e.key;
    let handled = true;
    if (!/^\d$/.test(k)) forget();
    if (k === "ArrowLeft" || k === "ArrowRight") move(seg, k === "ArrowRight" ? 1 : -1);
    else if (/^\d$/.test(k)) digit(seg, Number(k));
    else if (k === "Enter") commit();
    else {
      const next = keyDate(seg, k, shown, years);
      if (next) set(next);
      // (Space: nothing, and the menu doesn't scroll.)
      else if (k !== " ") handled = false;
    }
    if (handled) {
      e.preventDefault();
      e.stopPropagation();
    }
  }

  /** A click beside the numbers goes to the day. */
  function click(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const seg = (e.target as HTMLElement).closest("[data-seg]");
    if (!seg) dayEl?.focus();
  }

  function leave(e: FocusEvent) {
    const root = e.currentTarget as HTMLElement;
    if (!root.contains(e.relatedTarget as Node | null)) {
      forget(false);
      commit();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div class="df" role="group" aria-label={label} onclick={click} onfocusout={leave}>
  <span
    bind:this={dayEl}
    data-seg="d"
    class="seg day"
    role="spinbutton"
    tabindex="0"
    aria-label={t("Day")}
    aria-valuemin={1}
    aria-valuemax={Math.max(d, daysIn(y, mo))}
    aria-valuenow={d}
    onkeydown={(e) => key(e, "d")}>{d}</span
  ><span class="gap" aria-hidden="true"></span><span
    bind:this={monthEl}
    data-seg="mo"
    class="seg month"
    role="spinbutton"
    tabindex="0"
    aria-label={t("Month")}
    aria-valuemin={1}
    aria-valuemax={12}
    aria-valuenow={mo + 1}
    aria-valuetext={monthName(mo, true)}
    onkeydown={(e) => key(e, "mo")}
    ><span class="now">{months[mo]}</span>{#each months as name (name)}<span class="size" aria-hidden="true">{name}</span>{/each}</span
  ><span class="gap" aria-hidden="true"></span><span
    bind:this={yearEl}
    data-seg="y"
    class="seg year"
    role="spinbutton"
    tabindex="0"
    aria-label={t("Year")}
    aria-valuemin={years[0]}
    aria-valuemax={years[1]}
    aria-valuenow={y}
    aria-valuetext={yearText ?? String(y)}
    onkeydown={(e) => key(e, "y")}>{yearText ?? y}</span
  >
</div>

<style>
  /* The repeat menu's fields: 28 px, their corners and words. */
  .df {
    display: inline-flex;
    align-items: center;
    flex: none;
    box-sizing: border-box;
    height: 28px;
    padding: 0 7px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--card);
    font: inherit;
    font-size: var(--t-md);
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: var(--ink);
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }
  /* The ring on the whole field, as on a text field; the part being changed is lit inside. */
  .df:focus-within {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .seg {
    padding: 2px 2px;
    border-radius: 3px;
    outline: none;
    transition: background-color 0.12s ease;
  }
  .seg:focus,
  .seg:focus-visible {
    outline: none;
    background: color-mix(in srgb, var(--focus-ring) 24%, transparent);
  }
  /* (The day as wide as two digits: the 9th to the 10th moves nothing.) */
  .day {
    min-width: 2ch;
    text-align: center;
  }
  .month {
    display: inline-grid;
    justify-items: center;
  }
  .month > * {
    grid-area: 1 / 1;
  }
  .month .size {
    visibility: hidden;
  }
  /* A year being typed keeps the width of four digits. */
  .year {
    min-width: 4ch;
    text-align: center;
  }
  .gap {
    width: 0.25em;
  }
  @media (prefers-reduced-motion: reduce) {
    .seg {
      transition: none;
    }
  }
</style>
