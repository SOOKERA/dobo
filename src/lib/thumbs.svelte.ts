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
import { SvelteMap } from "svelte/reactivity";
import { calmMoment } from "./calm";
import type { Picture, Thumb } from "./types";

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

  /** What an `<img>` shows for a small box (a strip, a row): the thumb, else the picture. */
  src(p: Picture, fileSrc: (path: string) => string): string {
    return fileSrc(this.of(p)?.path ?? p.path);
  }

  /** The thumb couldn't be shown (its file gone): the picture itself from now on. */
  failed(p: Picture) {
    if (this.of(p)) this.made.set(p.path, null);
  }

  /** The notes' pictures: those without a thumb get one, in calm moments, a few at a time. */
  fill(api: ThumbMaker, pictures: Picture[]) {
    this.api = api;
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
