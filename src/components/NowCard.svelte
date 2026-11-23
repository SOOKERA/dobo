<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Now card: what is ringing, with its answers (Fold v2 / Reminder rhythm boards).
  // Reminders ring in every space, so this shows whatever space is on screen.
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import { store } from "../lib/store.svelte";
  import { dayWord, formatDueLong, hhmm } from "../lib/time";
  import { laterChoices } from "../lib/plan";
  import { parseCapture } from "../lib/capture";
  import { pushLayer } from "../lib/layers";
  import { closeOnLeave, modal } from "../lib/modal";
  import type { Item, RingAction } from "../lib/types";
  import { arrive, card, easeHeight, fadeQuick, glideTo, leave, pop, widen } from "../lib/motion";
  import { around, duration, i18n, ordinal, t, type Key } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";

  let index = $state(0);
  let list = $derived(store.ringing);
  // For screen readers: said once when a reminder starts ringing (not the card's minutes going by).
  // Emptied first, then the words a frame later: the same reminder ringing again (after
  // "Later…", a repeat) is said again too.
  let rang = $state("");
  let heard = new Set<number>();
  let sayFrame = 0;
  $effect(() => {
    const now = new Set(list.map((i) => i.id));
    const fresh = list.find((i) => !heard.has(i.id));
    heard = now;
    if (!fresh) return;
    const text = t("Reminder: {text}", { text: fresh.text });
    rang = "";
    cancelAnimationFrame(sayFrame);
    sayFrame = requestAnimationFrame(() => (rang = text));
  });
  $effect(() => () => cancelAnimationFrame(sayFrame));
  let it = $derived<Item | undefined>(list[Math.min(index, Math.max(0, list.length - 1))]);
  let note = $derived(it ? store.noteOf(it) : undefined);

  // "Later…": 1 hour, this evening, tomorrow morning, next Monday, or a time you type.
  let laterOpen = $derived(!!it && store.laterFor === it.id);
  let pick = $state("");
  let picked = $derived(pick.trim() ? parseCapture(`x ${pick}`, "task", store.now).due : null);
  let choices = $derived(laterChoices(store.now.getTime(), store.day));
  $effect(() => {
    // A "Later…" from a notification: show that item and its choices.
    const want = store.laterFor;
    if (want == null) return;
    const i = list.findIndex((x) => x.id === want);
    if (i >= 0) index = i;
  });
  $effect(() => {
    if (!laterOpen) return;
    return pushLayer(() => (store.laterFor = null), undefined, { beside: true });
  });
  function later(at: number) {
    if (it) store.answer(it, `until:${Math.round(at)}`);
    pick = "";
  }

  /** The label in capitals, but not what fills it: in French "3ᵉ" and "1 h 30" keep their
   * small letters ("MAINTENANT · 3ᵉ RAPPEL", "BIENTÔT · DANS 1 h 30"). */
  interface Label {
    pre: string;
    val: string;
    sup: string;
    post: string;
  }
  function upper(key: Key, fill?: { mark: string; val: string; nth?: number }): Label {
    const up = (s: string) => s.toLocaleUpperCase(i18n.locale);
    if (!fill) return { pre: up(t(key)), val: "", sup: "", post: "" };
    const [pre, post] = around(t(key), `{${fill.mark}}`);
    if (i18n.lang !== "fr") return { pre: up(pre), val: up(fill.val), sup: "", post: up(post) };
    if (fill.nth != null) return { pre: up(pre), val: String(fill.nth), sup: fill.nth === 1 ? "er" : "e", post: up(post) };
    return { pre: up(pre), val: fill.val, sup: "", post: up(post) };
  }

  function left(due: number, now: number): string {
    return duration(Math.max(1, Math.round((due - now) / 60000)));
  }

  let view = $derived.by(() => {
    if (!it) return null;
    const now = store.now.getTime();
    const due = it.due_at ?? now;
    const where = note ? noteTitle(note) : "";
    const deadline = it.alarm === "deadline";
    if (it.ring_state === "check") {
      return {
        tone: "check",
        label: upper("Checking in"),
        title: t("Is “{text}” really done?", { text: it.text }),
        meta: it.on_it_at ? t("You said you were on it at {time} · {where}", { time: hhmm(new Date(it.on_it_at)), where }) : t("You said you were on it · {where}", { where }),
        buttons: [
          [t("Yes, done"), "done", true],
          [t("Not yet"), "notyet", false],
        ] as [string, RingAction, boolean][],
      };
    }
    const early = it.ring_state === "early" || due > now;
    const label = early
      ? upper("Soon · in {left}", { mark: "left", val: left(due, now) })
      : it.rings > 1
        ? upper("Now · {nth} reminder", { mark: "nth", val: ordinal(it.rings), nth: it.rings })
        : upper("Now");
    const time = hhmm(new Date(due));
    // (Another day than today says which: a reminder missed overnight rings "Was due yesterday".)
    const day = dayWord(due, store.now);
    const meta = early
      ? day
        ? t("Due {day} at {time} · {where}", { day, time, where })
        : t("Due at {time} · {where}", { time, where })
      : now - due > 60000
        ? day
          ? t("Was due {day} at {time} · {where}", { day, time, where })
          : t("Was due {time} · {where}", { time, where })
        : t("Due now · {where}", { where });
    const buttons: [string, RingAction, boolean][] = deadline
      ? [
          [t("I’m on it"), "onit", true],
          [t("Done"), "done", false],
          [duration(10), "snooze:10", false],
        ]
      : [
          [t("Done"), "done", true],
          [duration(10), "snooze:10", false],
          [duration(60), "snooze:60", false],
        ];
    return { tone: early ? "early" : "now", label, title: it.text, meta, buttons, deadline };
  });

  async function act(a: RingAction) {
    if (!it) return;
    await store.answer(it, a);
  }
</script>

<span class="visually-hidden" aria-live="polite">{rang}</span>
{#if it && view}
  <!-- (The card's box also eases when the window's width moves its buttons under the words.) -->
  <div class="now-room" use:easeHeight>
  <section class="now {view.tone}" aria-label={t("Ringing now")} transition:card|global={{ duration: 400 }}>
    <!-- Another reminder in its place (the next one, or after an answer): the words crossfade
         and the card eases to its new height. -->
    <div class="bodies" use:easeHeight>
      <div class="stack">
        {#key it.id}
          <div class="body" in:arrive={{ delay: 100, duration: 260, y: 4 }} out:leave={{ duration: 120, y: -4 }}>
            <div class="label">
              <span class="beat" aria-hidden="true"></span>
              <span>{view.label.pre}{view.label.val}{#if view.label.sup}<sup class="nth">{view.label.sup}</sup>{/if}{view.label.post}</span>
              {#if "deadline" in view && view.deadline}<Icon name="hourglass" size={13} stroke={2} />{/if}
            </div>
            <div class="title">{view.title}</div>
            <div class="meta">{view.meta}</div>
          </div>
        {/key}
      </div>
    </div>
    <div class="acts">
      {#each view.buttons as [label, action, primary] (action)}
        <button class:primary animate:glideTo in:fadeQuick={{ delay: 100, duration: 200 }} out:fadeQuick={{ duration: 140 }} onclick={() => act(action)}>
          {#if primary && action === "done"}<Icon name="check" size={14} stroke={3} />{/if}
          {label}
        </button>
      {/each}
      {#if view.tone !== "check"}
        <button class:on={laterOpen} aria-expanded={laterOpen} transition:fadeQuick={{ duration: 180 }} onclick={() => (store.laterFor = laterOpen ? null : (it?.id ?? null))}>{t("Later…")}</button>
      {/if}
    </div>
    {#if list.length > 1}
      <button class="more" transition:fadeQuick={{ duration: 180 }} onclick={() => (index = (index + 1) % list.length)} data-tip={t("Show the next one")}>{t("{n} more", { n: list.length - 1 })}</button>
    {/if}
    {#if laterOpen}
      <!-- Opened by its button, the keyboard comes in (to the first choice) and goes back to it;
           opened from a notification, it stays where it is. -->
      <div
        class="later"
        role="dialog"
        aria-label={t("Remind me later")}
        transition:pop|global
        use:modal={{ initial: document.activeElement?.closest(".now") ? ".choice" : "none", trap: false, restore: () => !!document.activeElement?.closest(".later") }}
        use:closeOnLeave={() => (store.laterFor = null)}
      >
        {#each choices as c (c.label)}
          <button class="choice" onclick={() => later(c.at)}><span>{t(c.label)}</span><span class="when">{formatDueLong(c.at, store.now)}</span></button>
        {/each}
        <div class="pick">
          <label class="visually-hidden" for="later-pick">{t("Pick a time")}</label>
          <input
            id="later-pick"
            bind:value={pick}
            placeholder={t("Pick a time… (tomorrow 2pm, Friday 9am)")}
            autocomplete="off"
            onkeydown={(e) => {
              if (e.key === "Enter" && picked && !composing(e)) later(picked.getTime());
            }}
          />
          {#if picked}<button class="set" transition:widen={{ gap: 6 }} onclick={() => picked && later(picked.getTime())}>{formatDueLong(picked.getTime(), store.now)}</button>{/if}
        </div>
      </div>
    {/if}
  </section>
  </div>
{/if}

<style>
  .now-room {
    flex-shrink: 0;
  }
  .now {
    position: relative;
    margin: var(--s4) var(--s6) 0;
    padding: var(--s4) 18px;
    border-radius: 14px;
    background: var(--now-bg);
    color: var(--now-ink);
    display: flex;
    align-items: center;
    gap: 18px;
    flex-shrink: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .now {
      animation: none;
    }
    .beat {
      animation: none !important;
    }
  }
  .bodies {
    flex: 1;
    min-width: 0;
  }
  .stack {
    position: relative;
    display: grid;
  }
  .stack > .body {
    grid-area: 1 / 1;
  }
  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--t-sm);
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--now-accent);
  }
  .nth {
    /* "3ᵉ": a small raised e that doesn't push the line height. */
    font-size: 0.72em;
    line-height: 0;
    vertical-align: 0.42em;
    letter-spacing: 0;
  }
  .check .label {
    color: var(--now-check);
  }
  .beat {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 0 4px var(--now-halo);
    animation: beat 1.6s ease-in-out infinite;
  }
  .early .beat {
    animation: none;
    opacity: 0.8;
  }
  @keyframes beat {
    50% {
      transform: scale(0.75);
      opacity: 0.7;
    }
  }
  .title {
    font-family: var(--serif);
    font-size: 21px;
    font-weight: 500;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  .meta {
    font-size: var(--t-md);
    color: var(--now-muted);
  }
  .acts {
    display: flex;
    gap: var(--s2);
    flex-shrink: 0;
  }
  .acts button {
    height: 40px;
    padding: 0 14px;
    border: 1px solid var(--now-line);
    border-radius: 9px;
    background: transparent;
    color: var(--now-ink);
    font-size: var(--t-base);
    display: flex;
    align-items: center;
    gap: var(--s2);
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 0.15s, transform 0.12s;
  }
  .acts button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .acts button:active {
    transform: scale(0.97);
  }
  .acts button.primary {
    border-color: transparent;
    background: var(--now-btn);
    color: var(--now-btn-ink);
    font-weight: 600;
  }
  .more {
    position: absolute;
    right: 14px;
    top: 10px;
    height: 22px;
    padding: 0 8px;
    border: 0;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--now-muted);
    font-size: var(--t-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .acts button.on {
    background: rgba(255, 255, 255, 0.12);
  }
  .later {
    position: absolute;
    right: 14px;
    top: calc(100% - 6px);
    z-index: 30;
    width: 290px;
    padding: 6px;
    border-radius: var(--r-lg);
    background: var(--card);
    color: var(--ink);
    border: 1px solid var(--line);
    box-shadow: var(--pop-shadow);
  }
  .choice {
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
  .choice:hover {
    background: var(--hover);
  }
  .when {
    font-size: var(--t-sm);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 4px 2px 2px;
    padding: 0 4px 0 10px;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--ground);
  }
  .pick input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-md);
    color: var(--ink);
  }
  .pick input::placeholder {
    color: var(--muted);
  }
  .set {
    flex-shrink: 0;
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-sm);
    font-weight: 600;
    cursor: pointer;
  }
  @container shell (max-width: 600px) {
    .now {
      flex-direction: column;
      align-items: stretch;
    }
    /* Longer words (French) wrap onto a second row of equal buttons rather than spill out. */
    .acts {
      flex-wrap: wrap;
    }
    .acts button {
      flex: 1 1 auto;
      justify-content: center;
    }
  }
</style>
