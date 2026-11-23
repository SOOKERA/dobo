<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The short message at the bottom: "Deleted · Undo", "Undone · Redo", "Saved as a picture",
  // "Archived · Undo · See archive".
  import { words } from "../lib/i18n.svelte";
  import { store } from "../lib/store.svelte";
  import { reduced, rise } from "../lib/motion";
  import { toastRoom } from "../lib/toastroom.svelte";

  // A toast replacing another: the old words step aside and fade first, the new ones arrive just
  // after (never both strong at once).
  let prevId: number | null = null;
  let replacing = $state(false);
  $effect.pre(() => {
    const id = store.toast?.id ?? null;
    replacing = prevId != null && id != null && id !== prevId;
    prevId = id;
  });
  // What screen readers hear: emptied, then the words a frame later, so the same words coming
  // again (two toasts alike in a row) are said again too.
  let said = $state("");
  $effect(() => {
    const toast = store.toast;
    said = "";
    if (!toast) return;
    const f = requestAnimationFrame(() => (said = words(toast.text)));
    return () => cancelAnimationFrame(f);
  });
  function swapOut(_n: Element) {
    if (reduced()) return { duration: 80, css: (t: number) => `position:absolute;opacity:${t}` };
    return { duration: 120, css: (t: number) => `position:absolute;opacity:${t};transform:translateY(${(-(1 - t) * 6).toFixed(2)}px)` };
  }
</script>

<!-- What it says, for screen readers: one region that is always there (one made with its words is
     often not heard). -->
<span class="visually-hidden" role="status" aria-live="polite">{said}</span>
{#if store.toast}
  <!-- The holder leaves the way the toast came (down and out); a new toast replacing one only
       swaps what's inside. -->
  <div class="holder" class:lifted={toastRoom.lift > 0} style:--lift="{toastRoom.lift}px" data-lang-fade data-modal-also out:rise|global={{ y: 8, scale: 1, duration: 280 }}>
  {#key store.toast.id}
    <div class="toast" class:replacing class:two={!!store.toast.action && !!store.toast.more} out:swapOut>
      <span class="text">{words(store.toast.text)}</span>
      {#if store.toast.action || store.toast.more}
        <span class="acts">
          {#each [store.toast.action, store.toast.more] as a, i (i)}
            {#if a}
              <button
                class:more={i === 1}
                onclick={() => {
                  // (Taken first: once the toast is gone, `a` reads nothing.)
                  const run = a.run;
                  store.toast = null;
                  run();
                }}>{words(a.label)}</button
              >
            {/if}
          {/each}
        </span>
      {/if}
    </div>
  {/key}
  </div>
{/if}

<style>
  /* Clear of the note sizes and "?" at the board's corner; in a narrow panel, just above them.
     Centred on the panel while it fits between two corners' worth of room (the spacer on the
     left mirrors the corner on the right); a longer one gives up the left room first, its
     right end staying clear of the corner. */
  .holder {
    --base: 18px;
    position: absolute;
    left: 16px;
    right: 168px;
    bottom: var(--base);
    z-index: 60;
    display: flex;
    justify-content: center;
    pointer-events: none;
    /* (Moved, never laid out again: only its translate glides.) */
    transition: translate 0.32s var(--ease-glide);
  }
  /* A view with buttons along its bottom (Tidy, New note): just above them. */
  .holder.lifted {
    translate: 0 calc(var(--base) - var(--lift) - 12px);
  }
  .holder::before {
    content: "";
    flex: 0 1 152px;
  }
  .toast {
    pointer-events: auto;
    flex-shrink: 0;
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 6px 6px 6px 14px;
    border-radius: 11px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-md);
    box-shadow: var(--pop-shadow);
    animation: toast-in 0.28s cubic-bezier(0.2, 0.85, 0.25, 1) backwards;
  }
  .toast.replacing {
    animation-delay: 0.1s;
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @container shell (max-width: 620px) {
    .holder {
      --base: 62px;
      right: 16px;
    }
    .holder::before {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .holder {
      transition: none;
    }
    .toast {
      animation: none;
    }
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A narrow panel: the words may take two lines rather than be cut short beside the buttons. */
  @container shell (max-width: 620px) {
    .text {
      white-space: normal;
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
      line-clamp: 2;
      line-height: 1.35;
      padding: 2px 0;
    }
  }
  /* The buttons stay together (wrapped, they move as one). */
  .acts {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  button {
    flex-shrink: 0;
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: color-mix(in srgb, var(--card) 16%, transparent);
    color: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  button:hover {
    background: color-mix(in srgb, var(--card) 26%, transparent);
  }
  /* Two buttons in a narrow panel ("« Chanel VR shoot » archivée · Annuler · Voir les
     archives"): when the words don't fit beside them, the words keep the first row (the name
     whole when it can be) and the buttons go to a row of their own, at the right. */
  .toast.two {
    flex-wrap: wrap;
    row-gap: 2px;
  }
  .toast.two .text {
    flex: 1 1 auto;
    padding: 4px 8px 4px 0;
  }
  .toast.two .acts {
    margin-left: auto;
  }
  /* A second button ("See archive") sits right after the first, quieter. */
  button.more {
    background: transparent;
    font-weight: 500;
  }
  button.more:hover {
    background: color-mix(in srgb, var(--card) 16%, transparent);
  }
</style>
