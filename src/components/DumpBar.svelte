<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The quick dump bar (Dump board): only this bar appears, from anywhere. Type, Enter, it is in
  // the Inbox with nothing to decide. Paste a picture too. When a browser was in front, the
  // page's link is kept (and shown as a preview card later). Esc closes and keeps your text.
  import { composing } from "../lib/compose";
  import Icon from "./Icon.svelte";
  import { backendPromise, type Backend } from "../lib/backend";
  import type { LinkPreview, Source } from "../lib/types";
  import { followLanguage, keysText, t, words, type Words } from "../lib/i18n.svelte";
  import { blip, fadeQuick } from "../lib/motion";

  let api = $state<Backend | null>(null);
  let text = $state("");
  let source = $state<Source | null>(null);
  let keepLink = $state(true);
  let pics = $state<{ blob: Blob; url: string }[]>([]);
  let toast = $state<Words>("");
  let busy = $state(false);
  let shown = $state(false);
  let input: HTMLInputElement;
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");

  function appear() {
    shown = false;
    toast = "";
    source = null;
    keepLink = true;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        shown = true;
        input?.focus();
        input?.select();
      }),
    );
  }

  $effect(() => {
    backendPromise.then((b) => {
      api = b;
      followLanguage(b);
      b.onEvent("dump-shown", appear);
      b.onEvent("dump-source", (s) => (source = s as Source));
      b.onFocusChanged((f) => {
        if (!f && !busy) close();
      });
      appear();
    });
  });

  function close() {
    shown = false;
    setTimeout(() => api?.hideDump(), reduce.matches ? 0 : 140);
  }

  let host = $derived(source?.url ? source.url.replace(/^https?:\/\//, "").split(/[/?#]/)[0].replace(/^www\./, "") : "");

  async function dump() {
    if (!api || busy) return;
    const line = text.trim() || (pics.length ? t("Picture") : source?.url && keepLink ? source.title || host : "");
    if (!line) return;
    busy = true;
    try {
      const link: LinkPreview | null = source?.url && keepLink ? { url: source.url, title: source.title, site: host, image: null } : null;
      const [item, waiting] = await api.dumpItem(line, link, null);
      for (const p of pics) {
        const ext = (p.blob.type.split("/")[1] || "png").replace("jpeg", "jpg");
        await api.savePicture(item.note_id, item.id, new Uint8Array(await p.blob.arrayBuffer()), ext);
      }
      text = "";
      pics.forEach((p) => URL.revokeObjectURL(p.url));
      pics = [];
      toast = () => t("Dumped. {n} waiting in the Inbox.", { n: waiting });
      // The page's title and picture arrive a moment later. Only the link is written then: the
      // line may have been moved, timed or ticked on the board meanwhile.
      if (link) {
        const b = api;
        b.fetchPreview(link.url)
          .then((pv) => b.setItemLink(item.id, { ...pv, title: pv.title || link.title }))
          .catch(() => {});
      }
      setTimeout(close, reduce.matches ? 600 : 900);
    } catch (e) {
      toast = () => t("Couldn't dump it: {error}", { error: String(e) });
    } finally {
      busy = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    // (A key that finishes a composed character, a dead key's accent or an IME's word, isn't ours.)
    if (composing(e)) return;
    if (e.key === "Enter") {
      e.preventDefault();
      dump();
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  function onPaste(e: ClipboardEvent) {
    const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith("image/"));
    if (!files.length) return;
    e.preventDefault();
    pics = [...pics, ...files.map((f) => ({ blob: f as Blob, url: URL.createObjectURL(f) }))];
  }
</script>

<svelte:window onkeydown={onKey} onpaste={onPaste} />

<div class="wrap" class:shown>
  <div class="bar" role="dialog" aria-label={t("Quick dump")}>
    <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden="true"><path d="M3 4a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v9l-6 6H4a1 1 0 0 1-1-1z" fill="currentColor"></path><path d="M13 19v-5a1 1 0 0 1 1-1h5" fill="#E9C46A"></path></svg>
    <label for="dump" class="visually-hidden">{t("Dump into the inbox")}</label>
    <input id="dump" bind:this={input} bind:value={text} placeholder={t("Dump it here")} autocomplete="off" spellcheck="true" />
    {#each pics as p, i (p.url)}
      <button class="pic" in:blip out:blip aria-label={t("Remove this picture")} data-tip={t("Remove this picture")} data-tip-side="left" onclick={() => (pics = pics.filter((_, k) => k !== i))}><img src={p.url} alt="" /></button>
    {/each}
    <span class="key">{keysText("Enter")}</span>
  </div>
  <div class="under">
    {#if toast}
      <span class="toast" in:fadeQuick={{ duration: 200 }}><span class="tick"><Icon name="check" size={11} stroke={3.4} /></span>{words(toast)}</span>
    {:else if source?.url}
      <span class="src" class:off={!keepLink} in:fadeQuick={{ duration: 200 }}>
        <Icon name="link" size={12} stroke={2.2} />
        <span class="src-text">{keepLink ? t("Keeps the link to") : t("Without the link to")} <b>{source.title || host}</b> · {host}</span>
        <button onclick={() => (keepLink = !keepLink)}>{keepLink ? t("Leave it out") : t("Keep it")}</button>
      </span>
      <span class="esc" in:fadeQuick={{ duration: 200 }}>{t("Esc closes")}</span>
    {:else}
      <span in:fadeQuick={{ duration: 200 }}>{t("Into the Inbox, nothing to decide. Paste a picture too.")}</span>
      <span class="esc" in:fadeQuick={{ duration: 200 }}>{t("Esc closes")}</span>
    {/if}
  </div>
</div>

<style>
  :global(html.dump:not(.preview)),
  :global(html.dump:not(.preview) body) {
    background: transparent !important;
  }
  .wrap {
    position: fixed;
    inset: 0;
    padding: 16px 24px 0;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    opacity: 0;
    transform: translateY(-10px) scale(0.98);
    transition: opacity 0.12s ease-in, transform 0.12s ease-in;
  }
  .wrap.shown {
    opacity: 1;
    transform: none;
    transition: opacity 0.18s ease-out, transform 0.3s cubic-bezier(0.2, 0.85, 0.25, 1);
  }
  @media (prefers-reduced-motion: reduce) {
    .wrap,
    .wrap.shown {
      transition: none;
    }
  }
  .bar {
    height: 64px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 var(--s3) 0 var(--s5);
    border-radius: 16px;
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.32), 0 0 0 1px rgba(0, 0, 0, 0.08);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: 18px;
    color: var(--ink);
  }
  input::placeholder {
    color: var(--muted);
  }
  .pic {
    width: 44px;
    height: 34px;
    flex-shrink: 0;
    padding: 0;
    border: 0;
    border-radius: 6px;
    overflow: hidden;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .key {
    flex-shrink: 0;
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 6px;
    padding: 3px 8px;
  }
  .under {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--s3);
    min-height: 28px;
    padding: 0 var(--s1);
    font-size: var(--t-sm);
    color: #f3f1ec;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }
  .src,
  .toast {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 4px 6px 4px 10px;
    border-radius: 10px;
    background: rgba(20, 22, 26, 0.72);
    text-shadow: none;
  }
  .toast {
    padding: 5px 12px 5px 6px;
    animation: pop 0.25s var(--ease-out);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
  .tick {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #6f9c67;
    display: grid;
    place-items: center;
    color: #fff;
  }
  .src.off .src-text {
    opacity: 0.6;
  }
  .src-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .src b {
    font-weight: 600;
  }
  .src button {
    flex-shrink: 0;
    height: 22px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.14);
    color: #fff;
    font-size: var(--t-xs);
    cursor: pointer;
  }
  .esc {
    flex-shrink: 0;
  }
</style>
