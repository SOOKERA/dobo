<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The picture viewer, over the whole panel (Pictures board): arrows, thumbnails, copy, open
  // with the system viewer, remove. Left/Right browse, Esc closes.
  import Icon from "./Icon.svelte";
  import ViewHeader from "./ViewHeader.svelte";
  import { fadeQuick, veil } from "../lib/motion";
  import { store } from "../lib/store.svelte";
  import { viewer } from "../lib/viewer.svelte";
  import { pushLayer } from "../lib/layers";
  import { t, words, type Words } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";

  let pic = $derived(viewer.pictures[viewer.index]);
  let note = $derived(pic ? store.notes.find((n) => n.id === pic.note_id) : undefined);
  let name = $derived(pic ? (pic.path.startsWith("data:") ? t("Picture") : pic.path.split(/[\\/]/).pop()) : "");
  let msg = $state<Words>("");
  let confirm = $state(false);

  function go(d: number) {
    const n = viewer.pictures.length;
    viewer.index = (viewer.index + d + n) % n;
    confirm = false;
  }
  function close() {
    viewer.open = false;
  }

  $effect(() => {
    const key = (e: KeyboardEvent) => {
      // Alt+Left / Alt+Right are Back / Forward (they close the viewer), not browsing.
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key === "ArrowRight") go(1);
      else if (e.key === "ArrowLeft") go(-1);
    };
    const pop = pushLayer(close);
    window.addEventListener("keydown", key, true);
    return () => {
      pop();
      window.removeEventListener("keydown", key, true);
    };
  });

  async function copy() {
    try {
      const blob = await (await fetch(store.src(pic.path))).blob();
      const png = blob.type === "image/png" ? blob : await toPng(blob);
      await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
      msg = () => t("Copied");
    } catch {
      msg = () => t("Couldn't copy");
    }
    setTimeout(() => (msg = ""), 1600);
  }

  async function toPng(blob: Blob): Promise<Blob> {
    const bmp = await createImageBitmap(blob);
    const c = new OffscreenCanvas(bmp.width, bmp.height);
    c.getContext("2d")!.drawImage(bmp, 0, 0);
    return c.convertToBlob({ type: "image/png" });
  }

  async function remove() {
    const p = pic;
    viewer.pictures = viewer.pictures.filter((x) => x.id !== p.id);
    await store.deletePicture(p);
    confirm = false;
    if (!viewer.pictures.length) close();
    else viewer.index = Math.min(viewer.index, viewer.pictures.length - 1);
  }
</script>

{#if pic}
  <div class="viewer" role="dialog" aria-label={t("Picture viewer")} transition:veil|global={{ duration: 240 }}>
    <div class="bar" data-drag-area={store.win.mode === "floating" || undefined}>
      <ViewHeader crumbs={[{ label: note ? noteTitle(note) : t("Note"), go: close }, { label: name ?? t("Picture") }]} sub={t("{a} of {b}", { a: viewer.index + 1, b: viewer.pictures.length }) + (msg ? ` · ${words(msg)}` : "")} big={17} onBack={close} onClose={close}>
        <button class="tool" aria-label={t("Copy picture")} data-tip={t("Copy picture")} onclick={copy}><Icon name="copy" /></button>
        {#if !pic.path.startsWith("data:")}
          <button class="tool" aria-label={t("Open with the system viewer")} data-tip={t("Open in Photos")} onclick={() => store.backend.openPath(pic.path)}><Icon name="external" /></button>
        {/if}
        {#if confirm}
          <button class="tool danger" in:fadeQuick={{ duration: 180 }} onclick={remove}>{t("Remove")}</button>
        {:else}
          <button class="tool" in:fadeQuick={{ duration: 180 }} aria-label={t("Remove from the note")} data-tip={t("Remove from the note")} onclick={() => (confirm = true)}><Icon name="trash" /></button>
        {/if}
      </ViewHeader>
    </div>
    <div class="stage">
      {#key pic.id}
        <img src={store.src(pic.path)} alt="" draggable="false" />
      {/key}
      {#if viewer.pictures.length > 1}
        <button class="nav prev" aria-label={t("Previous picture")} data-tip={t("Previous picture")} data-tip-keys="Left" onclick={() => go(-1)}><Icon name="chev-left" stroke={2.2} /></button>
        <button class="nav next" aria-label={t("Next picture")} data-tip={t("Next picture")} data-tip-keys="Right" onclick={() => go(1)}><Icon name="chev-right" stroke={2.2} /></button>
      {/if}
    </div>
    <div class="strip">
      {#each viewer.pictures as p, k (p.id)}
        <button class="thumb" class:on={k === viewer.index} aria-label={t("Picture {n}", { n: k + 1 })} onclick={() => (viewer.index = k)}>
          <img src={store.src(p.path)} alt="" draggable="false" />
        </button>
      {/each}
      <span class="hint">{t("Left / Right to browse · Esc to close")}</span>
    </div>
  </div>
{/if}

<style>
  .viewer {
    position: absolute;
    inset: 0;
    z-index: 60;
    display: flex;
    flex-direction: column;
    background: rgba(20, 18, 16, 0.96);
    color: #f3efe8;
    /* The shared header's colours, on the dark viewer. */
    --ink: #f3efe8;
    --icon: #e4ded5;
    --hover: #2a2622;
  }
  .bar {
    height: 60px;
    flex-shrink: 0;
    padding: 0 var(--s2) 0 18px;
    display: flex;
    align-items: center;
    gap: var(--s1);
  }
  .bar > :global(.vh) {
    flex: 1;
    gap: var(--s1);
  }
  .tool {
    min-width: 40px;
    height: 40px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: #e4ded5;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .tool:hover {
    background: #2a2622;
  }
  .tool.danger {
    background: #c2410c;
    color: #fff;
    font-size: var(--t-md);
    font-weight: 600;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 0 64px;
  }
  .stage img {
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
    object-fit: contain;
    animation: pic-in 0.28s var(--ease-out);
  }
  @keyframes pic-in {
    from {
      opacity: 0;
      transform: scale(0.985);
    }
  }
  .nav {
    position: absolute;
    top: 50%;
    width: 44px;
    height: 44px;
    margin-top: -22px;
    border: 0;
    border-radius: 50%;
    background: #2a2622;
    color: #f3efe8;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .nav:hover {
    background: #3a342e;
  }
  .prev {
    left: 12px;
  }
  .next {
    right: 12px;
  }
  .strip {
    height: 84px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 0 var(--s4);
    overflow-x: auto;
  }
  .thumb {
    width: 64px;
    height: 42px;
    flex-shrink: 0;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 6px;
    overflow: hidden;
    background: #3a342e;
    opacity: 0.6;
    cursor: pointer;
  }
  .thumb.on {
    border-color: #f3efe8;
    opacity: 1;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .hint {
    margin-left: var(--s4);
    font-size: var(--t-sm);
    color: #b5ada2;
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: reduce) {
    .viewer,
    .stage img {
      animation: none;
    }
  }
</style>
