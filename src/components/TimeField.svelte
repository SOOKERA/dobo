<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A time setting ("HH:MM"): hours and minutes (and AM/PM when Windows uses a 12-hour clock),
  // each a spin button. Up/Down change it, Page Up/Down by 10 minutes or 3 hours, Home/End go to
  // its ends, Left/Right move between them, and digits type it ("9" "3" "0" is 09:30). The
  // browser's own time field followed the region of Windows, not the app (AM/PM in French).
  // Saved once typing stops, or on leaving the field, or with Enter; never while a part still
  // waits for its second digit (a lone "2" isn't 02:00).
  import { tick, untrack } from "svelte";
  import { i18n, t } from "../lib/i18n.svelte";
  import { keyTime, typeDigit, wrapTime, type HM, type Seg } from "../lib/timefield";
  let {
    value,
    label,
    onChange,
    variant = "field",
  }: {
    /** "HH:MM", 24-hour. */
    value: string;
    /** What the field is, for screen readers ("Work from"). */
    label: string;
    onChange: (v: string) => void;
    /** Settings' field (34 px), the one inside a sentence (30 px), or the Style panel's. */
    variant?: "field" | "inline" | "panel";
  } = $props();

  const pad = (n: number) => String(n).padStart(2, "0");
  function parse(v: string): HM {
    const m = /^(\d{1,2}):(\d{2})/.exec(v ?? "");
    return m ? [Math.min(23, Number(m[1])), Math.min(59, Number(m[2]))] : [0, 0];
  }
  const text = ([h, m]: HM) => `${pad(h)}:${pad(m)}`;

  /** Being changed, not yet saved. */
  let draft = $state<HM | null>(null);
  /** Saved, until the setting comes back with it. */
  let sent = $state<string | null>(null);
  $effect(() => {
    void value;
    untrack(() => (sent = null));
  });
  let shown = $derived(draft ?? parse(sent ?? value));
  let h = $derived(shown[0]);
  let m = $derived(shown[1]);
  let twelve = $derived(i18n.hour12);
  let pm = $derived(h >= 12);
  // (6:00 AM, as the app writes it everywhere; the box keeps the width of two digits.)
  let hourText = $derived(twelve ? String(h % 12 || 12) : pad(h));

  let hourEl = $state<HTMLElement>();
  let minuteEl = $state<HTMLElement>();
  let periodEl = $state<HTMLElement>();
  const el = (s: Seg) => (s === "h" ? hourEl : s === "m" ? minuteEl : periodEl);
  const order = $derived<Seg[]>(twelve ? ["h", "m", "p"] : ["h", "m"]);

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  /** The time being changed; saved once nothing else happens (`save`), or not yet. */
  function set([nh, nm]: HM, save = true) {
    draft = wrapTime(nh, nm);
    clearTimeout(saveTimer);
    if (save) saveTimer = setTimeout(commit, 900);
  }
  function commit() {
    clearTimeout(saveTimer);
    if (!draft) return;
    const v = text(draft);
    draft = null;
    if (v === value) return;
    sent = v;
    onChange(v);
  }
  // Digits typed into one part: a second one completes it, or a pause starts again.
  let typed = "";
  let typedFor: Seg | null = null;
  let typedTimer: ReturnType<typeof setTimeout> | undefined;
  function forget() {
    clearTimeout(typedTimer);
    typed = "";
    typedFor = null;
  }
  /** Another key while a part waits for its second digit: what it shows is saved soon. */
  function endWait() {
    if (typedFor === null) return;
    forget();
    if (draft) set(draft);
  }
  // (Leaving Settings while one is being changed: it is kept.)
  $effect(() => () => {
    forget();
    commit();
  });
  // AM/PM gone while it had the focus (Windows' clock turned 24-hour): the minutes have it.
  $effect.pre(() => {
    if (twelve || !periodEl || document.activeElement !== periodEl) return;
    void tick().then(() => minuteEl?.focus());
  });
  function move(from: Seg, by: number) {
    const i = order.indexOf(from) + by;
    if (i >= 0 && i < order.length) el(order[i])?.focus();
  }

  function digit(seg: "h" | "m", d: number) {
    const r = typeDigit(seg, typedFor === seg ? typed : "", d, shown, twelve);
    forget();
    if (r.done) {
      set(r.time);
      move(seg, 1);
    } else {
      // Waiting for the second digit: shown, saved when the wait ends.
      set(r.time, false);
      typed = r.buf;
      typedFor = seg;
      typedTimer = setTimeout(() => {
        forget();
        commit();
      }, 1500);
    }
  }

  function key(e: KeyboardEvent, seg: Seg) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const k = e.key;
    let handled = true;
    if (!/^\d$/.test(k)) endWait();
    if (k === "ArrowLeft" || k === "ArrowRight") move(seg, k === "ArrowRight" ? 1 : -1);
    else if (/^\d$/.test(k) && seg !== "p") digit(seg, Number(k));
    else if (k === "Enter") commit();
    else {
      const next = keyTime(seg, k, shown, twelve);
      if (next) set(next);
      // (Space on the hours or the minutes: nothing, and the sheet doesn't scroll.)
      else if (k !== " ") handled = false;
    }
    if (handled) {
      e.preventDefault();
      e.stopPropagation();
    }
  }

  /** A click beside the numbers goes to the hours; inside a switch's row it never flips the
   * switch. */
  function click(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const seg = (e.target as HTMLElement).closest("[data-seg]");
    if (!seg) hourEl?.focus();
  }

  function leave(e: FocusEvent) {
    const root = e.currentTarget as HTMLElement;
    if (!root.contains(e.relatedTarget as Node | null)) {
      forget();
      commit();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div class="tf {variant}" role="group" aria-label={label} onclick={click} onfocusout={leave}>
  <!-- (The widest the time can be, unseen, in the same place: the box keeps that width, from
       9:00 to 10:00 and AM to PM, and the time sits in its middle. Drawn by CSS, so no words.) -->
  <span class="ghost" aria-hidden="true"><span class="seg" data-g="00"></span><span class="sep" data-g=":"></span><span class="seg" data-g="00"></span>{#if twelve}<span class="gap"></span><span class="seg" data-g="AM"></span>{/if}</span>
  {#if twelve}<span class="ghost" aria-hidden="true"><span class="seg" data-g="00"></span><span class="sep" data-g=":"></span><span class="seg" data-g="00"></span><span class="gap"></span><span class="seg" data-g="PM"></span></span>{/if}
  <span class="time"><span
    bind:this={hourEl}
    data-seg="h"
    class="seg hour"
    role="spinbutton"
    tabindex="0"
    aria-label={t("Hours")}
    aria-valuemin={twelve ? 1 : 0}
    aria-valuemax={twelve ? 12 : 23}
    aria-valuenow={twelve ? h % 12 || 12 : h}
    aria-valuetext={twelve ? String(h % 12 || 12) : pad(h)}
    onkeydown={(e) => key(e, "h")}>{hourText}</span
  ><span class="sep" aria-hidden="true">:</span><span
    bind:this={minuteEl}
    data-seg="m"
    class="seg"
    role="spinbutton"
    tabindex="0"
    aria-label={t("Minutes")}
    aria-valuemin={0}
    aria-valuemax={59}
    aria-valuenow={m}
    aria-valuetext={pad(m)}
    onkeydown={(e) => key(e, "m")}>{pad(m)}</span
  >{#if twelve}<span class="gap" aria-hidden="true"></span><span
      bind:this={periodEl}
      data-seg="p"
      class="seg period"
      role="spinbutton"
      tabindex="0"
      aria-label={t("AM or PM")}
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={pm ? 1 : 0}
      aria-valuetext={pm ? "PM" : "AM"}
      onkeydown={(e) => key(e, "p")}>{pm ? "PM" : "AM"}</span
    >{/if}</span>
</div>

<style>
  /* The same box as the other fields: Settings' 34 px (30 px inside a sentence), the Style
     panel's 30 px. */
  .tf {
    display: inline-grid;
    align-items: center;
    justify-items: center;
    flex: none;
    box-sizing: border-box;
    height: 34px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    font: inherit;
    font-size: var(--t-base);
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: var(--ink);
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }
  .tf.inline {
    height: 30px;
    margin: 0 4px;
    vertical-align: middle;
  }
  .tf.panel {
    height: 30px;
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 600;
  }
  /* The ring on the whole field, as on a text field; the part being changed is lit inside. */
  .tf:focus-within {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .seg {
    padding: 2px 1px;
    border-radius: 3px;
    outline: none;
    transition: background-color 0.12s ease;
  }
  .seg:focus,
  .seg:focus-visible {
    outline: none;
    background: color-mix(in srgb, var(--focus-ring) 24%, transparent);
  }
  /* The time and, unseen under it, the widest it can be ("00:00 PM"), in one place: the box is as
     wide as the widest, so 9:00 AM to 10:00 AM never moves the field, and the time is centred in
     it, with even room on both sides. */
  .time,
  .ghost {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
  }
  .ghost {
    visibility: hidden;
    pointer-events: none;
  }
  .ghost [data-g]::before {
    content: attr(data-g);
  }
  .sep {
    padding: 0 0.5px;
  }
  .gap {
    width: 0.3em;
  }
  @media (prefers-reduced-motion: reduce) {
    .seg {
      transition: none;
    }
  }
</style>
