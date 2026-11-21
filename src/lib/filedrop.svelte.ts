// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Files dropped from the computer onto the board: a picture goes into the note's pictures,
// any other file becomes a line with a file card. Outside a note: into the Inbox.
import { store, IMAGE_EXT } from "./store.svelte";
import type { DropEvent } from "./backend";
import { look } from "./look.svelte";
import { inMemory } from "./sample";

class FileDrop {
  /** The note under the pointer while files are carried over the window. */
  note = $state<number | null>(null);
  over = $state(false);
}

export const fileDrop = new FileDrop();

function noteAt(x: number, y: number): number | null {
  const el = document.elementFromPoint(x, y) as HTMLElement | null;
  const card = el?.closest<HTMLElement>("[data-drop-note]");
  const id = card ? Number(card.dataset.dropNote) : null;
  // (The tour's sample Inbox lives in memory only: what is dropped goes to the real one.)
  return inMemory(id) ? null : id;
}

export async function onFileDrop(e: DropEvent) {
  if (e.kind === "leave") {
    fileDrop.note = null;
    fileDrop.over = false;
    return;
  }
  const at = noteAt(e.x, e.y);
  if (e.kind === "over") {
    fileDrop.over = true;
    fileDrop.note = at;
    return;
  }
  fileDrop.note = null;
  fileDrop.over = false;
  // A look file (.ztheme): shown in Export & import, added only when you say so.
  const theme = e.paths.find((p) => /\.ztheme$/i.test(p));
  if (theme) {
    await look.readImport(theme);
    return;
  }
  const noteId = at ?? (await store.inboxId(store.space));
  for (const path of e.paths) {
    if (IMAGE_EXT.test(path)) {
      // A picture too big (or unreadable) says so, and the other dropped files still come in.
      try {
        await store.addPictureFile(noteId, null, path);
      } catch (err) {
        store.showToast(String(err));
      }
    } else {
      const name = path.split(/[\\/]/).pop() || path;
      const it = await store.backend.createItem({ noteId, kind: "task", text: name, file: path });
      store.upsertItem(it);
    }
  }
}
