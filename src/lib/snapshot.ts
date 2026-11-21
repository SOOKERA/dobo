// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// "Save as a picture": the note drawn into a PNG (off screen, nothing opens), saved into
// Pictures › Dobo and copied, ready to paste in a chat.
import { toBlob } from "html-to-image";
import { store } from "./store.svelte";
import { t } from "./i18n.svelte";

const SKIP = ["menu", "grip", "lift-shadow", "peel", "more", "chev", "acts", "tidy"];

export async function saveNotePicture(card: HTMLElement, title: string) {
  const ground = getComputedStyle(document.documentElement).getPropertyValue("--ground").trim() || "#f2f0eb";
  const blob = await toBlob(card, {
    pixelRatio: 2,
    backgroundColor: ground,
    // A little paper around the note, like a photo of it.
    style: { margin: "0", transform: "none", boxShadow: "none" },
    filter: (node) => !(node instanceof HTMLElement && SKIP.some((c) => node.classList.contains(c))),
  });
  if (!blob) throw new Error(t("Couldn't draw the note."));
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const path = await store.backend.saveNoteImage(bytes, title);
  let copied = false;
  try {
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    copied = true;
  } catch {
    /* some systems don't allow pictures on the clipboard from here */
  }
  store.showToast(() => copied ? t("Saved in Pictures › Dobo, and copied") : t("Saved in Pictures › Dobo"), { label: () => t("Show"), run: () => void store.backend.reveal(path) });
}
