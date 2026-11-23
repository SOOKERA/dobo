<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The notes didn't open at start (src-tauri/src/recovery.rs): what stands in the way, said
  // calmly, and what can be done about it, instead of a Dobo that never shows. Shown before the
  // board (nothing of it is loaded); only its own buttons answer it (no Esc, no click beside it).
  // - damaged: nothing renamed yet: use the repaired notes (when a repair worked), Try again, put
  //   back the newest backup, or start with an empty board (each puts the file aside first,
  //   broken-<date>.sqlite); one an earlier start put aside that reads whole now: put it back;
  // - locked (another program holds it), readonly: Try again;
  // - failed (won't open for another reason): Try again, put back, or start empty;
  // - portable (a portable copy can't write in its own folder): use the usual folder.
  // Up / Down move between the choices; Quit is always there.
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import { APP_NAME } from "../lib/app";
  import { arrive, fold, rise } from "../lib/motion";
  import { modal } from "../lib/modal";
  import { installWindowDrag } from "../lib/windrag";
  import { dateAndTime, t } from "../lib/i18n.svelte";
  import type { Backend, RecoveryChoice, RecoveryCounts, RecoveryProblem } from "../lib/backend";

  let {
    backend,
    problem: first,
    error = "",
    stuck: stuckAtFirst = false,
    onDone,
  }: {
    backend: Backend;
    problem: RecoveryProblem;
    /** Why the board couldn't come in after a choice that worked (the screen is back). */
    error?: string;
    /** …and nothing is left to choose (the notes aren't waiting on a choice any more): the reason,
     * Try again (the board coming in again) and Quit, never the choices of before. */
    stuck?: boolean;
    onDone: (choice: RecoveryChoice, problem: RecoveryProblem) => void;
  } = $props();

  // (The problem can change: Try again may find another one.)
  // svelte-ignore state_referenced_locally
  let problem = $state(first);
  // svelte-ignore state_referenced_locally
  let stuck = $state(stuckAtFirst);
  let busy = $state<RecoveryChoice | null>(null);
  /** What the last choice said (it couldn't be done, or the file is still held). */
  let said = $state("");
  // (Why the board couldn't come in, said once the screen is there: a live region announces what
  // changes in it, never what it held from the start.)
  onMount(() => {
    if (!error) return;
    void tick().then(() => requestAnimationFrame(() => (said ||= error)));
  });
  let listEl = $state<HTMLElement>();

  // The window moves by its top strip, as the first-launch window.
  $effect(() => installWindowDrag(() => backend.startDragging()));

  interface Option {
    choice: RecoveryChoice;
    icon: string;
    title: string;
    detail: string;
    /** A folder, on its own line above the detail (a full stop after it would read as its own). */
    path?: string;
    /** It can't be done (no backup to put back). */
    off?: boolean;
  }

  const when = (ms: number) => dateAndTime(new Date(ms), true);
  let fileName = $derived(problem.file.split(/[\\/]/).pop() ?? problem.file);

  let problemTitle = $derived(
    problem.kind === "damaged" && problem.whole_again
      ? t("Your notes can be put back")
      : problem.kind === "damaged"
        ? t("Your notes file is damaged")
        : problem.kind === "locked"
          ? t("Another program is using your notes")
          : problem.kind === "readonly"
            ? t("Your notes can’t be saved")
            : problem.kind === "portable"
              ? t("{app} can’t keep its notes next to itself", { app: APP_NAME })
              : t("Your notes didn’t open"),
  );
  let title = $derived(stuck ? t("The board couldn’t start") : problemTitle);
  /** The file's name stands out in the words, whole (never cut at a hyphen). */
  const MARK = "\u0001";
  let problemLead = $derived(
    (problem.kind === "damaged"
      ? problem.whole_again
        ? t("An earlier start put your notes aside as {name}, in the data folder, because they couldn’t be read. They read whole now: put them back to carry on where you left off.", { name: MARK })
        : problem.kept_as
          ? t("{app} couldn’t read it, so it put it aside as it was, as {name} in its data folder. Nothing was deleted.", { app: APP_NAME, name: MARK })
          : t("{app} couldn’t read {name} whole. Nothing has been changed: whatever you choose, the file is kept as it is in the data folder, under a new name.", { app: APP_NAME, name: MARK })
      : problem.kind === "locked"
        ? t("Something else has {name} open (often an antivirus, a sync or a backup tool), so {app} can’t open it now. Your notes are fine: wait a moment, then try again.", { app: APP_NAME, name: MARK })
        : problem.kind === "readonly"
          ? t("Windows lets {app} read {name} but not change it: the file or its folder is read-only, or this account can’t write there. Your notes are fine. Untick Read-only in the file’s Properties (or ask whoever looks after this computer), then try again.", { app: APP_NAME, name: MARK })
          : problem.kind === "portable"
            ? problem.portable_notes
              ? t("This copy of {app} keeps its notes in {name}, next to it, but Windows doesn’t let it write there (a read-only drive or folder). So that your notes aren’t split, {app} asks first. To keep them there, quit, then move {app} to a folder it can write in, such as Documents.", { app: APP_NAME, name: MARK })
              : t("This copy of {app} would keep its notes in {name}, next to it, but Windows doesn’t let it write there (a read-only drive or folder, or Program Files). To keep them there, quit, then move {app} to a folder it can write in, such as Documents.", { app: APP_NAME, name: MARK })
            : t("{app} couldn’t open {name}. Try again first. If it still won’t open, put back the newest backup or start with an empty board: the file is put aside either way, never deleted.", { app: APP_NAME, name: MARK })
    ).split(MARK),
  );
  let lead = $derived(stuck ? [t("Try again. If it still doesn’t start, quit {app} and open it again. Your notes are kept as they are.", { app: APP_NAME })] : problemLead);
  let leadName = $derived(problem.kind === "damaged" && problem.kept_as ? problem.kept_as : problem.kind === "portable" ? (problem.portable ?? "") : fileName);

  let options = $derived.by((): Option[] => {
    // (Try again: Rust has nothing to choose, and the board is asked to come in again.)
    if (stuck) return [{ choice: "retry", icon: "refresh", title: t("Try again"), detail: t("{app} tries to start the board again.", { app: APP_NAME }) }];
    const retry: Option = {
      choice: "retry",
      icon: "refresh",
      title: t("Try again"),
      // (Each its own: what trying again does for that problem.)
      detail:
        problem.kind === "damaged"
          ? t("{app} reads the file again: a drive that answered badly may read it whole now.", { app: APP_NAME })
          : problem.kind === "locked"
            ? t("{app} opens your notes as soon as it can.", { app: APP_NAME })
            : problem.kind === "readonly"
              ? t("{app} checks the file again, once Read-only is unticked.", { app: APP_NAME })
              : t("{app} tries to open the file again now.", { app: APP_NAME }),
    };
    // (What the repair and the backup hold, so that they can be weighed: a repair may miss notes.)
    const held = (c: RecoveryCounts) => ({ notes: t("{n} notes", { n: c.notes }), lines: t("{n} lines", { n: c.lines }) });
    const repair: Option = {
      choice: "repair",
      icon: "check",
      title: t("Use the repaired notes"),
      detail: problem.repaired_counts
        ? t("Everything {app} could still read, copied into a new file: {notes} and {lines}. The damaged one is kept in the data folder.", { app: APP_NAME, ...held(problem.repaired_counts) })
        : t("Everything {app} could still read, copied into a new file. The damaged one is kept in the data folder.", { app: APP_NAME }),
    };
    const putBack: Option = { choice: "putback", icon: "file", title: t("Put my notes back"), detail: t("As they were when they were put aside.") };
    const restore: Option = problem.backup
      ? {
          choice: "restore",
          icon: "undo",
          title: t("Put back the newest backup"),
          detail: problem.backup.counts
            ? t("From {when}: {notes} and {lines}, with their pictures, as they were then.", { when: when(problem.backup.at), ...held(problem.backup.counts) })
            : t("From {when}: your notes and pictures as they were then.", { when: when(problem.backup.at) }),
        }
      : { choice: "restore", icon: "undo", title: t("Put back the newest backup"), detail: t("No backup was found."), off: true };
    const empty: Option = {
      choice: "empty",
      icon: "note-plus",
      title: t("Start with an empty board"),
      detail: t("Your settings come along when they can be read. The old file stays in the data folder."),
    };
    if (problem.kind === "portable")
      return [
        {
          choice: "usual",
          icon: "file",
          title: t("Use the usual folder"),
          path: problem.data,
          detail: problem.portable_notes
            ? t("Your notes stay there from now on, apart from the notes next to {app}.", { app: APP_NAME })
            : t("Your notes stay there from now on, even once {app} can write next to itself.", { app: APP_NAME }),
        },
      ];
    if (problem.kind === "damaged" && problem.whole_again) return [putBack, restore, empty];
    if (problem.kind === "damaged") return problem.repaired ? [repair, retry, restore, empty] : [retry, restore, empty];
    if (problem.kind === "failed") return [retry, restore, empty];
    return [retry];
  });

  const BUSY: Record<RecoveryChoice, () => string> = {
    repair: () => t("Opening the repaired notes…"),
    putback: () => t("Putting your notes back…"),
    restore: () => t("Putting the backup back…"),
    empty: () => t("Starting an empty board…"),
    retry: () => t("Looking again…"),
    usual: () => t("Opening the notes…"),
  };

  async function pick(o: Option) {
    if (busy || o.off) return;
    busy = o.choice;
    said = "";
    try {
      const next = await backend.recoveryChoose(o.choice);
      if (!next) {
        onDone(o.choice, problem);
        return;
      }
      stuck = false;
      if (next.kind === problem.kind && o.choice === "retry")
        said = problem.kind === "locked" ? t("It’s still in use. Try again in a moment.") : problem.kind === "readonly" ? t("It still can’t be changed.") : t("It still won’t open.");
      problem = next;
    } catch (e) {
      said = String(e);
    }
    busy = null;
    // The keyboard back on the choice (its button was off while it ran), or on the first one when
    // the screen changed.
    await tick();
    const all = [...(listEl?.querySelectorAll<HTMLButtonElement>("button.choice:not(:disabled)") ?? [])];
    (all.find((b) => b.dataset.choice === o.choice) ?? all[0])?.focus();
  }

  /** Up / Down (and Home / End) between the choices; from the screen itself (as it opens), Down
   * comes to the first. */
  function keys(e: KeyboardEvent) {
    if (!listEl || !["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    const all = [...listEl.querySelectorAll<HTMLButtonElement>("button.choice:not(:disabled)")];
    if (!all.length) return;
    e.preventDefault();
    const at = all.indexOf(document.activeElement as HTMLButtonElement);
    const down = e.key === "ArrowDown";
    const next = e.key === "Home" || (at < 0 && down) ? 0 : e.key === "End" || at < 0 ? all.length - 1 : (at + (down ? 1 : -1) + all.length) % all.length;
    all[next].focus();
  }
</script>

<div class="recovery" role="dialog" aria-modal="true" aria-labelledby="recovery-title" aria-describedby="recovery-lead" transition:rise|global={{ y: 10, duration: 320 }} tabindex="-1" onkeydown={keys} use:modal={{ initial: "self" }}>
  <div class="move" data-drag-area aria-hidden="true"></div>
  <div class="page">
    <span class="kicker"><Logo size={22} /></span>
    {#key problem.kind}
      <div class="body" in:arrive={{ delay: 60, duration: 260, y: 6 }}>
        <h1 id="recovery-title">{title}</h1>
        <p class="lead" id="recovery-lead">{#each lead as part, i (i)}{#if i}<b class="file" class:path={problem.kind === "portable"}>{leadName}</b>{/if}{part}{/each}</p>
        {#if options.length}<div class="choices" bind:this={listEl} role="group" aria-label={t("What to do")}>
          {#each options as o, i (o.choice)}
            <button class="choice" class:first={i === 0} data-choice={o.choice} disabled={!!busy || o.off} aria-busy={busy === o.choice} aria-describedby="choice-{o.choice}" onclick={() => pick(o)}>
              <span class="c-icon" aria-hidden="true"><Icon name={busy === o.choice ? "refresh" : o.icon} size={18} /></span>
              <span class="c-words">
                <span class="c-title">{o.title}</span>
                <span class="c-detail" id="choice-{o.choice}">
                  <!-- (Both in one place, crossfading: the card keeps its height while it runs.) -->
                  <span class="c-text" class:off={busy === o.choice} aria-hidden={busy === o.choice}>{#if o.path}<span class="c-path">{o.path}</span>{/if}{o.detail}</span>
                  <span class="c-text" class:off={busy !== o.choice} aria-hidden={busy !== o.choice}>{BUSY[o.choice]()}</span>
                </span>
              </span>
              {#if !o.off}<span class="c-go" aria-hidden="true"><Icon name="chev-right" size={16} /></span>{/if}
            </button>
          {/each}
        </div>{/if}
        <div class="said" aria-live="polite">
          {#if said}<p transition:fold>{said}</p>{/if}
        </div>
        {#if problem.error && !stuck}<p class="tech">{t("What Windows or SQLite said: {error}", { error: problem.error })}</p>{/if}
      </div>
    {/key}
    <div class="foot">
      <button class="back" disabled={!!busy} onclick={() => void backend.quitApp()}>{t("Quit {app}", { app: APP_NAME })}</button>
      <button class="link" onclick={() => void backend.openDataFolder().catch(() => {})}>{t("Open the data folder")}</button>
    </div>
  </div>
</div>

<style>
  /* The card, as the board's own (App's .shell): the window round it is see-through. */
  .recovery {
    position: absolute;
    inset: 12px;
    display: grid;
    overflow-y: auto;
    border-radius: var(--r-xl);
    background: var(--ground);
    box-shadow: var(--shell-shadow);
    container: recovery / inline-size;
  }
  .recovery:focus {
    outline: none;
  }
  .move {
    grid-area: 1 / 1;
    align-self: start;
    position: sticky;
    top: 0;
    z-index: 1;
    height: 32px;
    background: linear-gradient(var(--ground) 75%, transparent);
  }
  .page {
    grid-area: 1 / 1;
    min-height: 100%;
    max-width: 600px;
    width: 100%;
    margin: 0 auto;
    padding: 36px 40px 28px;
    display: flex;
    flex-direction: column;
  }
  /* Calm: the words, the choices and Quit together, in the middle of the window (auto margins:
     when they don't fit, it scrolls from the top). */
  .kicker {
    margin-top: auto;
    display: flex;
    align-items: center;
    height: 28px;
  }
  .body {
    display: flex;
    flex-direction: column;
  }
  h1 {
    margin: 18px 0 0;
    font-family: var(--serif);
    font-size: 30px;
    font-weight: 500;
    letter-spacing: -0.01em;
    line-height: 1.15;
    text-wrap: balance;
  }
  .lead {
    margin: var(--s3) 0 0;
    font-size: var(--t-base);
    line-height: 1.55;
    color: var(--soft);
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
  .choices {
    margin-top: 22px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    min-height: 64px;
    padding: 12px 14px 12px 12px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--card);
    box-shadow: var(--card-shadow);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.16s ease,
      transform 0.16s var(--ease-out),
      opacity 0.2s ease;
  }
  .choice:hover:not(:disabled) {
    border-color: var(--box);
  }
  .choice:active:not(:disabled) {
    transform: scale(0.995);
  }
  .choice:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .choice:disabled {
    cursor: default;
  }
  /* While one runs, the others step back; one that can't be done stays quiet. */
  .choice:disabled:not([aria-busy="true"]) {
    opacity: 0.55;
    box-shadow: none;
  }
  .c-icon {
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--chip);
    color: var(--icon);
  }
  /* The first choice is the one to try: its mark in ink. */
  .choice.first:not(:disabled) .c-icon,
  .choice[aria-busy="true"] .c-icon {
    background: var(--ink);
    color: var(--card);
  }
  .choice[aria-busy="true"] .c-icon :global(svg) {
    animation: turn 1.1s linear infinite;
  }
  @keyframes turn {
    to {
      rotate: 1turn;
    }
  }
  .c-words {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .c-title {
    font-size: var(--t-base);
    font-weight: 600;
    color: var(--ink);
  }
  .c-detail {
    display: grid;
    overflow-wrap: anywhere;
    font-size: var(--t-md);
    line-height: 1.45;
    color: var(--muted);
  }
  .c-detail > .c-text {
    grid-area: 1 / 1;
    transition:
      opacity 0.16s ease,
      visibility 0s;
  }
  .c-path {
    display: block;
    color: var(--soft);
  }
  .c-detail > .c-text.off {
    opacity: 0;
    visibility: hidden;
    transition:
      opacity 0.16s ease,
      visibility 0s 0.16s;
  }
  .file {
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
  }
  /* (A whole folder may be long: it wraps anywhere rather than run past the card.) */
  .file.path {
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .c-go {
    flex-shrink: 0;
    display: grid;
    color: var(--muted);
    transition: translate 0.18s var(--ease-out);
  }
  .choice:hover:not(:disabled) .c-go,
  .choice:focus-visible .c-go {
    translate: 2px 0;
  }
  .said p {
    margin: var(--s3) 0 0;
    font-size: var(--t-md);
    color: var(--late-ink);
  }
  .tech {
    margin: var(--s4) 0 0;
    font-size: var(--t-xs);
    line-height: 1.5;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .foot {
    margin: 28px 0 auto;
    display: flex;
    align-items: center;
    gap: var(--s4);
  }
  .back {
    height: 40px;
    padding: 0 18px;
    margin-right: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    font-size: var(--t-base);
    cursor: pointer;
    transition: border-color 0.16s ease;
  }
  /* (As the choices, on hover.) */
  .back:hover:not(:disabled) {
    border-color: var(--box);
  }
  .back:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .link {
    padding: 2px;
    border: 0;
    background: transparent;
    font-size: var(--t-md);
    color: var(--soft);
    text-decoration: underline;
    text-decoration-color: var(--line);
    text-underline-offset: 3px;
    cursor: pointer;
    transition: text-decoration-color 0.16s ease;
  }
  .link:hover {
    text-decoration-color: var(--soft);
  }
  .back:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  @container recovery (max-width: 480px) {
    .page {
      padding: 28px 22px 20px;
    }
    h1 {
      font-size: 25px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .choice[aria-busy="true"] .c-icon :global(svg) {
      animation: none;
    }
    .c-detail > .c-text,
    .c-detail > .c-text.off {
      transition: none;
    }
  }
</style>
