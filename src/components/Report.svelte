<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Help › Report a problem (as Pako's). What happened, in the person's words; under it, always
  // in view, everything that goes with them ("Sent with your report"), with a switch for Dobo's
  // recent problems. Nothing goes until a click: Send report (Sookera's report service, private),
  // Send by email (their own email app, to support@sookera.com) or Copy details. The words stay
  // until the report is sent; closing while it sends lets it finish, and a toast says how it went.
  // Opened from the "?" menu (Back: the board) or Settings › Help (Back: there).
  import Icon from "./Icon.svelte";
  import ViewHeader, { type Crumb } from "./ViewHeader.svelte";
  import { store } from "../lib/store.svelte";
  import { look } from "../lib/look.svelte";
  import { pushLayer } from "../lib/layers";
  import { modal, takeFocus } from "../lib/modal";
  import { copyText } from "../lib/contextmenu";
  import { composing } from "../lib/compose";
  import { reportDraft } from "../lib/reportDraft.svelte";
  import { APP_NAME } from "../lib/app";
  import { arrive, easeHeight, fadeQuick, fold, leave, sheet } from "../lib/motion";
  import { i18n, t } from "../lib/i18n.svelte";
  import {
    areaLabel,
    DESCRIPTION_MAX,
    DISCUSSIONS_URL,
    hasWords,
    languageLine,
    lookLine,
    mailPlan,
    problemLine,
    reportText,
    sendFailText,
    sentProblems,
    shownTime,
    SUPPORT_EMAIL,
    versionLine,
    zoneOf,
    zoneNow,
    type ReportInfo,
  } from "../lib/report";

  let { from, onBack, onClose }: { from: "board" | "settings"; onBack: () => void; onClose: () => void } = $props();

  // Esc: straight to the board; Back (mouse back, Alt+Left, Backspace outside the box): where it
  // was opened from.
  $effect(() => pushLayer(() => onClose(), { back: () => onBack() }));

  let crumbs = $derived<Crumb[]>(from === "settings" ? [{ label: t("Settings"), go: onBack }, { label: t("Report a problem") }] : [{ label: t("Report a problem") }]);

  let info = $state<ReportInfo | null>(null);
  let infoFailed = $state(false);
  // (Read when the page opens, never while it is only rehearsed: effects don't run then.)
  $effect(() => {
    let live = true;
    store.backend
      .reportInfo()
      .then((i) => live && (info = i))
      .catch(() => live && (infoFailed = true));
    return () => (live = false);
  });

  // (Kept between opens: reportDraft.svelte.ts.)
  const draft = reportDraft;
  let showAll = $state(false);
  let sentRef = $state<string | null>(null);
  let failText = $state("");
  let mailNote = $state("");
  let field = $state<HTMLTextAreaElement>();

  let dark = $derived(look.dark);
  let parts = $derived(info ? { info, dark, include: draft.include } : null);
  let chars = $derived([...draft.words].length);
  let tooLong = $derived(chars > DESCRIPTION_MAX);
  let problems = $derived(info?.problems ?? []);
  let listed = $derived(showAll ? problems : problems.slice(0, 3));
  let zone = $derived(problems.length ? zoneOf(problems[0].time) || zoneNow() : "");

  /** Says first what's missing (the words), then takes the keyboard there. */
  function needWords(): boolean {
    if (hasWords(draft.words)) return false;
    failText = t("Say what happened first.");
    field?.focus();
    return true;
  }

  // The page may close while it sends: the toast then says how it went.
  let open = true;
  $effect(() => () => (open = false));

  async function send() {
    if (draft.sending || !parts || needWords() || tooLong) return;
    draft.sending = true;
    failText = mailNote = "";
    const at = parts;
    const words = draft.words;
    try {
      const ref = await store.backend.reportSend(words, at.dark ? "dark" : "light", i18n.lang, sentProblems(at));
      // Sent: the words go (unless more were typed meanwhile).
      if (draft.words === words) draft.words = "";
      if (open) sentRef = ref;
      else store.showToast(() => t("Report sent: #{ref}. Thank you.", { ref }), undefined, 6000);
    } catch (e) {
      if (open) failText = sendFailText(e);
      else store.showToast(() => t("The report didn’t go. Open Report a problem to try again, or send it by email."), undefined, 7000);
    } finally {
      draft.sending = false;
    }
  }

  async function byEmail() {
    if (!parts || needWords()) return;
    failText = mailNote = "";
    const plan = mailPlan(draft.words, parts);
    if (plan.clipboard != null && !(await copyText(plan.clipboard))) {
      failText = t("Couldn’t put the details on your clipboard. Use Copy details, then paste them into an email to {email}.", { email: SUPPORT_EMAIL });
      return;
    }
    try {
      await store.backend.reportMail(plan.url);
      mailNote = plan.clipboard != null ? t("Your email app opens a new message. The details are on your clipboard: paste them where it says.") : t("Your email app opens a new message with everything filled in. Check it, then send it.");
    } catch (e) {
      failText = typeof e === "string" ? e : t("Couldn’t open your email app. Use Copy details and paste them into an email to {email}.", { email: SUPPORT_EMAIL });
    }
  }

  async function copyDetails() {
    if (!parts) return;
    failText = mailNote = "";
    if (await copyText(reportText(draft.words, parts))) mailNote = t("Copied. Paste them into a new email to {email}.", { email: SUPPORT_EMAIL });
    else failText = t("Couldn’t copy");
  }

  /** Ctrl+Enter sends, from anywhere on the page. */
  function keys(e: KeyboardEvent) {
    // (An accent or an IME word being typed finishes first.)
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && !e.repeat && !composing(e) && sentRef == null) {
      e.preventDefault();
      void send();
    }
  }

  // Typing again after a "Say what happened first": the note goes.
  $effect(() => {
    if (hasWords(draft.words) && failText === t("Say what happened first.")) failText = "";
  });

  /** A problem's time; its own offset too when it isn't the newest one's (a clock change since,
   * or another time zone). */
  const timeOf = (iso: string) => {
    const z = zoneOf(iso);
    return z && z !== zone ? `${shownTime(iso)} (${z})` : shownTime(iso);
  };
</script>

<div class="sheet" role="dialog" aria-label={t("Report a problem")} tabindex="-1" transition:sheet|global use:modal={{ initial: "textarea" }} onkeydown={keys}>
  <div class="top">
    <ViewHeader {crumbs} {onBack} {onClose} />
  </div>

  <div class="stack">
    {#key sentRef != null}
      {#if sentRef != null}
        <div class="sent-room" role="status" in:arrive={{ delay: 120, duration: 300, y: 6 }} out:leave={{ duration: 120, y: 0 }}>
          <span class="tick" aria-hidden="true"><Icon name="check" size={22} stroke={2} /></span>
          <h3>{t("Sent. Thank you.")}</h3>
          {#if sentRef}
            <p class="lead">{t("Your report is #{ref}. Mention it if you write to {email} about the same problem.", { ref: sentRef, email: SUPPORT_EMAIL })}</p>
          {/if}
          <p class="quiet">{t("Reports are kept private and used only to fix {app}.", { app: APP_NAME })}</p>
          <button class="small strong" use:takeFocus onclick={onClose}>{t("Back to the board")}</button>
        </div>
      {:else}
        <div class="form" in:arrive={{ delay: 80, duration: 240, y: 0 }} out:leave={{ duration: 120, y: 0 }}>
          <p class="intro">{t("Tell Sookera what went wrong. Nothing goes until you click: everything that goes with your words is shown below.")}</p>

          <label class="what">
            <span class="label">{t("What happened?")}</span>
            <textarea
              bind:this={field}
              bind:value={draft.words}
              rows="6"
              spellcheck="true"
              placeholder={t("What you did, what you expected, and what happened instead.")}
              aria-invalid={tooLong}
></textarea>
            {#if chars > DESCRIPTION_MAX - 500}
              <small class="count" class:over={tooLong} transition:fadeQuick>{t("{n} of {max} characters", { n: chars.toLocaleString(i18n.locale), max: DESCRIPTION_MAX.toLocaleString(i18n.locale) })}</small>
            {/if}
          </label>

          <section class="sent" aria-labelledby="report-sent">
            <h4 id="report-sent">{t("Sent with your report")}</h4>
            {#if info}
              <dl in:fadeQuick>
                <dt>{APP_NAME}</dt>
                <dd>{versionLine(info)}</dd>
                <dt>{t("Windows")}</dt>
                <dd>{info.windows}</dd>
                <dt>{t("Language")}</dt>
                <dd>{languageLine()}</dd>
                <dt>{t("Look")}</dt>
                <dd>{lookLine(dark)}</dd>
                {#if draft.include && problems.length}
                  <dt transition:fadeQuick>{t("Time zone")}</dt>
                  <dd transition:fadeQuick>{zone}</dd>
                {/if}
              </dl>

              <label class="line check">
                <span>
                  {t("Include recent problems")}
                  <small>{problems.length ? t("The last problems {app} noted: when it had to stop, or couldn’t start. Your user folder, folder and file names and anything in quotes are taken out first, and links keep only their site.", { app: APP_NAME }) : t("{app} hasn’t noted any problem.", { app: APP_NAME })}</small>
                </span>
                <input type="checkbox" bind:checked={draft.include} disabled={!problems.length} />
                <span class="sw" aria-hidden="true"></span>
              </label>

              {#if draft.include && problems.length}
                <div class="problems-room" use:easeHeight transition:fold={{ duration: 260 }}>
                  <ul class="problems">
                    {#each listed as p, i (i)}
                      <li transition:fold={{ duration: 220 }}>
                        <span class="when">{timeOf(p.time)} · {areaLabel(p.area)}</span>
                        <span class="msg">{problemLine(p, info.version)}</span>
                      </li>
                    {/each}
                  </ul>
                  {#if problems.length > 3}
                    <button class="link" onclick={() => (showAll = !showAll)}>{showAll ? t("Show fewer") : t("Show all ({n})", { n: problems.length })}</button>
                  {/if}
                </div>
              {/if}
            {:else if infoFailed}
              <p class="error" transition:fold>{t("{app} couldn’t read what goes with a report. Try again in a moment.", { app: APP_NAME })}</p>
            {:else}
              <p class="quiet" aria-busy="true">{t("Reading…")}</p>
            {/if}
          </section>

          <div class="actions">
            <button class="small strong" disabled={!info || draft.sending || tooLong} onclick={send} data-tip={t("Send report")} data-tip-keys="Ctrl+Enter">
              {#if draft.sending}<span class="spin" aria-hidden="true"></span>{t("Sending…")}{:else}<Icon name="send" size={14} stroke={1.9} />{t("Send report")}{/if}
            </button>
            <button class="small" disabled={!info} onclick={byEmail}>{t("Send by email")}</button>
            <button class="small" disabled={!info} onclick={copyDetails}>{t("Copy details")}</button>
          </div>
          <div class="msg-room" use:easeHeight aria-live="polite">
            {#if failText}
              <p class="error" in:arrive={{ delay: 40, duration: 200, y: 0 }} out:leave={{ duration: 110, y: 0 }}>{failText}</p>
            {:else if mailNote}
              <p class="quiet" in:arrive={{ delay: 40, duration: 200, y: 0 }} out:leave={{ duration: 110, y: 0 }}>{mailNote}</p>
            {/if}
          </div>
          <p class="quiet">{t("Send report goes to Sookera’s report service. Reports are kept private and used only to fix {app}; your IP address isn’t stored.", { app: APP_NAME })}</p>
          <p class="ideas">
            {t("An idea or a question instead?")}
            <button class="link" onclick={() => void store.backend.openUrl(DISCUSSIONS_URL)}>{t("Share it on GitHub Discussions")}</button>
          </p>
        </div>
      {/if}
    {/key}
  </div>
</div>

<style>
  .sheet {
    position: absolute;
    inset: 0;
    z-index: 41;
    outline: none;
    overflow-y: auto;
    padding: 0 var(--s6) var(--s6);
    background: var(--ground);
  }
  .top {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 calc(-1 * var(--s6));
    padding: var(--s3) var(--s6);
    background: var(--ground);
  }
  /* The form and the "Sent" page take turns in one place. */
  .stack {
    display: grid;
    max-width: 640px;
  }
  .stack > * {
    grid-area: 1 / 1;
    min-width: 0;
  }
  .intro {
    margin: var(--s1) 0 var(--s4);
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--muted);
  }
  .what {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .label,
  h4 {
    margin: 0;
    font-size: var(--t-md);
    font-weight: 600;
    color: var(--ink);
  }
  textarea {
    width: 100%;
    min-height: 128px;
    box-sizing: border-box;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: var(--t-base);
    line-height: 1.5;
    resize: vertical;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  textarea:focus {
    border-color: color-mix(in srgb, var(--ink) 35%, var(--line));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ink) 7%, transparent);
  }
  textarea[aria-invalid="true"] {
    border-color: var(--late-ink);
  }
  textarea::placeholder {
    color: var(--muted);
  }
  .count {
    align-self: flex-end;
    font-size: var(--t-xs);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .count.over {
    color: var(--late-ink);
  }
  .sent {
    margin: var(--s5) 0 0;
    padding: var(--s4) var(--s4) var(--s2);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--card);
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 6px var(--s4);
    margin: var(--s3) 0 var(--s2);
    font-size: var(--t-md);
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    color: var(--ink);
    overflow-wrap: anywhere;
  }
  .line {
    position: relative;
    min-height: 44px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s2) 0;
    border-top: 1px solid var(--line);
    font-size: var(--t-base);
    cursor: pointer;
  }
  .line > span:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .line small {
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--muted);
  }
  .check input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .sw {
    width: 42px;
    height: 26px;
    flex-shrink: 0;
    border-radius: 13px;
    background: var(--edge);
    position: relative;
    transition: background-color 0.2s;
  }
  .sw::after {
    content: "";
    position: absolute;
    left: 3px;
    top: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
    transition: transform 0.25s var(--ease-out);
  }
  .check input:checked + .sw {
    background: var(--ink);
  }
  .check input:checked + .sw::after {
    transform: translateX(16px);
  }
  .check input:focus-visible + .sw {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .check input:disabled + .sw {
    opacity: 0.45;
  }
  .check:has(input:disabled) {
    cursor: default;
  }
  .problems-room {
    padding: 0 0 var(--s2);
  }
  .problems {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .problems li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s2) 0;
    border-top: 1px dashed var(--line);
  }
  .problems li:first-child {
    border-top: 0;
    padding-top: var(--s1);
  }
  .when {
    font-size: var(--t-xs);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .msg {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--soft);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin: var(--s5) 0 0;
  }
  .small {
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, opacity 0.15s;
  }
  .small:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--ink) 30%, var(--line));
  }
  .small:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .small.strong {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .small.strong:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ink) 88%, var(--card));
  }
  .spin {
    width: 12px;
    height: 12px;
    border: 2px solid color-mix(in srgb, var(--card) 35%, transparent);
    border-top-color: var(--card);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation-duration: 2.4s;
    }
  }
  .msg-room {
    margin: var(--s2) 0 0;
  }
  .error {
    margin: 0;
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--late-ink);
  }
  .quiet {
    margin: var(--s2) 0 0;
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--muted);
  }
  .ideas {
    margin: var(--s4) 0 0;
    padding: var(--s3) 0 0;
    border-top: 1px solid var(--line);
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .link {
    padding: 2px;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: var(--t-sm);
    color: var(--soft);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .sent-room {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s2);
    padding: var(--s6) 0;
  }
  .sent-room h3 {
    margin: var(--s2) 0 0;
    font-family: var(--serif);
    font-size: 20px;
    font-weight: 600;
  }
  .sent-room .lead {
    margin: 0;
    font-size: var(--t-base);
    line-height: 1.5;
  }
  .sent-room .small {
    margin-top: var(--s3);
  }
  .tick {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--ink) 8%, var(--card));
    color: var(--ink);
  }
</style>
