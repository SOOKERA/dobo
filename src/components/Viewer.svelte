<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The picture viewer, over the whole panel (Pictures board): arrows, thumbnails, copy, open
  // with the system viewer, remove. Left/Right browse, Esc closes.
  import Icon from "./Icon.svelte";
  import ViewHeader from "./ViewHeader.svelte";
  import { fadeQuick, reduced, veil } from "../lib/motion";
  import { NOT_OURS } from "../lib/ownFiles";
  import { store } from "../lib/store.svelte";
  import { thumbs } from "../lib/thumbs.svelte";
  import { viewer } from "../lib/viewer.svelte";
  import { pushLayer } from "../lib/layers";
  import { isTopModal, modal, takeFocus } from "../lib/modal";
  import { t, words, type Words } from "../lib/i18n.svelte";
  import { noteTitle } from "../lib/names";

  let pic = $derived(viewer.pictures[viewer.index]);
  let note = $derived(pic ? store.notes.find((n) => n.id === pic.note_id) : undefined);
  let name = $derived(pic ? (pic.path.startsWith("data:") ? t("Picture") : pic.path.split(/[\\/]/).pop()) : "");
  let msg = $state<Words>("");
  // A picture outside Dobo's own folders (a backup from elsewhere naming a network share): never
  // asked for (ownFiles.ts), so it can't be copied, nor opened in Photos (Windows would reach
  // that place); its place shows calmly.
  let away = $derived(pic ? store.src(pic.path) === NOT_OURS : false);
  // One of Dobo's own whose file can't be shown (gone, damaged): the same calm place; nothing to
  // copy, nor to open in Photos.
  let gone = $derived(pic ? thumbs.unreadable(pic) : false);
  let off = $derived(away || gone);
  let offTip = $derived(away ? t("This picture isn’t in Dobo’s folders") : t("This picture can’t be read"));
  let confirm = $state(false);
  let viewerEl = $state<HTMLElement>();
  let stripEl = $state<HTMLElement>();

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
      // Alt+Left / Alt+Right are Back / Forward (they close the viewer), not browsing. (Nor
      // while something opened over it has the keyboard.)
      if (e.altKey || e.ctrlKey || e.metaKey || !isTopModal(viewerEl)) return;
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

  // The shown picture's thumb kept in sight in the strip (a long strip in a narrow window): at
  // once when the viewer opens, gliding when it changes. (Nothing while rehearsed off the page.)
  let glide = false;
  $effect(() => {
    const on = stripEl?.children[viewer.index] as HTMLElement | undefined;
    if (!on?.isConnected) return;
    on.scrollIntoView({ inline: "nearest", block: "nearest", behavior: glide && !reduced() ? "smooth" : "auto" });
    glide = true;
  });

  async function copy() {
    if (off) return;
    try {
      // (A picture outside Dobo's own folders is never asked for: nothing to copy.)
      const src = store.src(pic.path);
      if (src === NOT_OURS) throw new Error("not ours");
      const blob = await (await fetch(src)).blob();
      const png = blob.type === "image/png" ? blob : await toPng(blob);
      await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
      msg = () => t("Copied");
    } catch {
      msg = () => t("Couldn’t copy");
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
  <div class="viewer" role="dialog" aria-label={t("Picture viewer")} tabindex="-1" bind:this={viewerEl} transition:veil|global={{ duration: 240 }} use:modal={{ initial: "self" }}>
    <div class="bar" data-drag-area={store.win.mode === "floating" || undefined}>
      <ViewHeader crumbs={[{ label: note ? noteTitle(note) : t("Note"), go: close }, { label: name ?? t("Picture") }]} sub={t("{a} of {b}", { a: viewer.index + 1, b: viewer.pictures.length }) + (msg ? ` · ${words(msg)}` : "")} big={17} onBack={close} onClose={close}>
        <button class="tool" aria-label={t("Copy picture")} aria-disabled={off || undefined} data-tip={off ? offTip : t("Copy picture")} onclick={copy}><Icon name="copy" /></button>
        {#if !pic.path.startsWith("data:")}
          <button class="tool" aria-label={t("Open with the system viewer")} aria-disabled={off || undefined} data-tip={off ? offTip : t("Open in Photos")} onclick={() => !off && store.backend.openPath(pic.path)}><Icon name="external" /></button>
        {/if}
        {#if confirm}
          <button class="tool danger" use:takeFocus in:fadeQuick={{ duration: 180 }} onclick={remove}>{t("Remove")}</button>
        {:else}
          <button class="tool" in:fadeQuick={{ duration: 180 }} aria-label={t("Remove from the note")} data-tip={t("Remove from the note")} onclick={() => (confirm = true)}><Icon name="trash" /></button>
        {/if}
      </ViewHeader>
    </div>
    <div class="stage">
      {#key pic.id}
        {#if off}
          <span class="pic-ph away"><Icon name="image" size={34} stroke={1.5} /></span>
        {:else}
          {@const p = pic}
          <img src={store.src(p.path)} alt="" draggable="false" onerror={() => thumbs.failed(p, p.path)} />
        {/if}
      {/key}
      {#if viewer.pictures.length > 1}
        <button class="nav prev" aria-label={t("Previous picture")} data-tip={t("Previous picture")} data-tip-keys="Left" onclick={() => go(-1)}><Icon name="chev-left" stroke={2.2} /></button>
        <button class="nav next" aria-label={t("Next picture")} data-tip={t("Next picture")} data-tip-keys="Right" onclick={() => go(1)}><Icon name="chev-right" stroke={2.2} /></button>
      {/if}
    </div>
    <div class="strip" bind:this={stripEl}>
      {#each viewer.pictures as p, k (p.id)}
        {@const src = store.thumbSrc(p)}
        <button class="thumb" class:on={k === viewer.index} aria-label={t("Picture {n}", { n: k + 1 })} onclick={() => (viewer.index = k)}>
          {#if src === NOT_OURS}
            <span class="pic-ph"><Icon name="image" size={16} stroke={1.7} /></span>
          {:else}
            <img {src} alt="" loading="lazy" decoding="async" draggable="false" onerror={() => thumbs.failed(p)} />
          {/if}
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
  /* (Can't be used for this picture: dimmed, its tooltip says why.) */
  .tool[aria-disabled="true"] {
    opacity: 0.38;
    cursor: default;
  }
  .tool[aria-disabled="true"]:hover {
    background: transparent;
  }
  .tool.danger {
    background: var(--danger-fill);
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
  /* The viewer's own dark chip, a picture's usual shape. */
  .away {
    width: min(100%, 420px);
    max-height: 100%;
    aspect-ratio: 4 / 3;
    border-radius: 6px;
    background: #2a2622;
    color: #8f877c;
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
    /* (Centred while it fits; a longer strip starts at its first thumb, all of it reachable.) */
    justify-content: safe center;
    gap: 10px;
    padding: 0 var(--s4);
    overflow-x: auto;
    /* (A thumb scrolled into sight keeps the strip's own margin from its edge.) */
    scroll-padding-inline: var(--s4);
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
  .thumb img,
  .thumb .pic-ph {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .thumb .pic-ph {
    display: grid;
    background: #3a342e;
    color: #b5ada2;
  }
  .hint {
    margin-left: var(--s4);
    font-size: var(--t-sm);
    color: #b5ada2;
    white-space: nowrap;
  }
  /* A narrow window: the strip keeps its room for the thumbs (the arrows say the same). */
  @container shell (max-width: 560px) {
    .hint {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .viewer,
    .stage img,
    .away {
      animation: none;
    }
  }
  /* (It takes the keyboard itself when it opens: no ring round the whole view.) */
  .viewer:focus {
    outline: none;
  }
</style>
