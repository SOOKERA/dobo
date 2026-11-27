// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The notes' pictures on the board come from their thumbs (thumbs.rs): a board-size copy, made
// once, so a photo of 4000 × 3000 shown in a box of 300 × 150 never costs its whole file and 48
// MB decoded. The viewer shows the picture itself.
//
// A picture without a thumb yet (just added, or from before thumbs) shows itself meanwhile: its
// thumb is made at once when it is added, else in calm moments, a few at a time; it replaces the
// picture once it is loaded and decoded, so the swap is never seen (same shape, same place).
// A box bigger than the thumb (a wide note on a 200% screen) shows the picture itself.
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { calmMoment } from "./calm";
import type { Picture, Thumb } from "./types";
import { NOT_OURS } from "./ownFiles";

export interface ThumbMaker {
  makeThumb(path: string): Promise<Thumb | null>;
  fileSrc(path: string): string;
}

/** How many thumbs are made in one calm moment. */
const FEW = 2;

/** A file of the pictures folder (not a drawn sample, nor a picture in memory). */
const ownFile = (path: string) => !!path && !/^(data|blob|https?):/i.test(path);

/** Its file loaded and decoded, so showing it is instant. */
function preload(url: string): Promise<void> {
  const img = new Image();
  img.decoding = "async";
  img.src = url;
  return img.decode();
}

/** Whether a picture's file shows now (loaded and decoded, out of sight): a picture seen failing
 * is tried again with it, never shown broken meanwhile. */
export async function canShow(url: string): Promise<boolean> {
  try {
    await preload(url);
    return true;
  } catch {
    return false;
  }
}

/** How long a picture tried again may take to show (`inTime`). */
export const PROBE_LIMIT = 10_000;

/** A picture tried again (`probe`), given up after `PROBE_LIMIT`: one load that hangs (a drive
 * half there) counts as not back, so the next try is never held up by it. */
export function inTime(probe: () => Promise<boolean>, ms = PROBE_LIMIT): Promise<boolean> {
  return new Promise((resolve) => {
    const late = setTimeout(() => resolve(false), ms);
    const done = (ok: boolean) => {
      clearTimeout(late);
      resolve(ok);
    };
    Promise.resolve()
      .then(probe)
      .then(done, () => done(false));
  });
}

class Thumbs {
  /** Thumbs made since Dobo opened, by picture file; null: the picture shows itself (small
   * enough, or its thumb couldn't be made or shown). */
  private made = new SvelteMap<string, Thumb | null>();
  private asked = new Set<string>();
  private now: string[] = [];
  private later: string[] = [];
  private running = false;
  private api: ThumbMaker | null = null;

  /** The picture's thumb, if it has one (null: show the picture itself). */
  of(p: Picture): Thumb | null {
    const m = this.made.get(p.path);
    return m !== undefined ? m : (p.thumb ?? null);
  }

  /** The thumb when it is sharp in a box of `box` device pixels (unknown yet: assumed), else
   * null. */
  sharpIn(p: Picture, box: { w: number; h: number } | null): Thumb | null {
    const t = this.of(p);
    // (Half a pixel of rounding in the box's measure.)
    return t && (!box || (t.w + 0.5 >= box.w && t.h + 0.5 >= box.h)) ? t : null;
  }

  /** What an `<img>` shows for a small box (a strip, a row): the thumb, else the picture; for a
   * picture that can't be shown (`unreadable`), `NOT_OURS`: its calm place (`.pic-ph`), never a
   * broken picture. */
  src(p: Picture, fileSrc: (path: string) => string): string {
    return this.broken.has(p.path) ? NOT_OURS : fileSrc(this.of(p)?.path ?? p.path);
  }

  /** Pictures whose own file couldn't be shown (gone, damaged), by picture file. */
  private broken = new SvelteSet<string>();
  /** The picture's own file couldn't be shown: its place shows instead, and the viewer can't copy
   * it nor open it in Photos. */
  unreadable(p: Picture): boolean {
    return this.broken.has(p.path);
  }

  /** A file shown for `p` couldn't be (`path`: the one shown, by default as `src` picks it): its
   * thumb (gone) → the picture itself from now on; the picture itself → `unreadable`. */
  failed(p: Picture, path: string = this.of(p)?.path ?? p.path) {
    if (path !== p.path) {
      if (this.of(p)) this.made.set(p.path, null);
    } else if (!this.broken.has(p.path)) this.broken.add(p.path);
  }

  /** The pictures seen failing tried again, calmly (a restore brought them back, a drive is back):
   * each once, out of sight; those that show again are shown, the others stay as they are. */
  private retrying = false;
  async retry(fileSrc: (path: string) => string, probe: (url: string) => Promise<boolean> = canShow) {
    if (this.retrying || !this.broken.size) return;
    this.retrying = true;
    try {
      const paths = [...this.broken];
      const back = await Promise.all(paths.map((path) => inTime(() => probe(fileSrc(path)))));
      paths.forEach((path, k) => back[k] && this.broken.delete(path));
    } finally {
      this.retrying = false;
    }
  }

  /** The notes' pictures: those without a thumb get one, in calm moments, a few at a time. */
  fill(api: ThumbMaker, pictures: Picture[]) {
    this.api = api;
    // (Pictures seen failing that the notes no longer have, after a restore: forgotten.)
    if (this.broken.size) {
      const now = new Set(pictures.map((p) => p.path));
      for (const path of [...this.broken]) if (!now.has(path)) this.broken.delete(path);
    }
    for (const p of pictures) {
      if (p.thumb || this.asked.has(p.path) || !ownFile(p.path)) continue;
      this.asked.add(p.path);
      this.later.push(p.path);
    }
    void this.pump();
  }

  /** A picture just added: its thumb made now, before any other. */
  soon(api: ThumbMaker, p: Picture) {
    this.api = api;
    if (p.thumb || this.asked.has(p.path) || !ownFile(p.path)) return;
    this.asked.add(p.path);
    this.now.push(p.path);
    void this.pump();
  }

  private async pump() {
    if (this.running) return;
    this.running = true;
    try {
      while (this.now.length || this.later.length) {
        if (this.now.length) {
          await this.make(this.now.shift()!);
          continue;
        }
        await calmMoment(() => this.now.length > 0);
        for (let k = 0; k < FEW && this.later.length && !this.now.length; k++) await this.make(this.later.shift()!);
      }
    } finally {
      this.running = false;
    }
  }

  private async make(path: string) {
    const api = this.api;
    if (!api) return;
    let t: Thumb | null = null;
    try {
      t = await api.makeThumb(path);
      if (t) await preload(api.fileSrc(t.path));
    } catch {
      t = null;
    }
    this.made.set(path, t);
  }
}

export const thumbs = new Thumbs();

/** The screen's scale (2 on a 200% screen), followed when the window moves to another screen. */
class Screen {
  scale = $state(typeof window === "undefined" ? 1 : window.devicePixelRatio || 1);
  constructor() {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const follow = () => {
      this.scale = window.devicePixelRatio || 1;
      window.matchMedia(`(resolution: ${this.scale}dppx)`).addEventListener("change", follow, { once: true });
    };
    window.matchMedia(`(resolution: ${this.scale}dppx)`).addEventListener("change", follow, { once: true });
  }
}
const screen = new Screen();

/** A note's picture box in device pixels, from the note's width (before the note size's zoom:
 * its layout width, a little more than the box's) and the box's height. */
export function inDevicePixels(css: [number, number], zoom: number, scale = screen.scale): { w: number; h: number } {
  const r = scale * zoom;
  return { w: css[0] * r, h: css[1] * r };
}
