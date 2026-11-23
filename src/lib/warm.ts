// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Warm-up: the first open of the Style panel (and its pages) must be as smooth as the 999th.
// What makes a cold open stutter is the pictures it shows: texture tiles (an SVG grain filter
// rendered into a PNG) and watercolour paintings. So after launch, in calm moments only (calm.ts:
// nothing pressed, typed or scrolled for a while, nothing moving on screen), one small piece at a
// time, we make the ones the panel shows first: its first page. Customise, the board-size
// paintings, the Themes page and More mixes come later, a minute after launch at the earliest
// (opened before that, the panel paints what it shows itself, first). Nothing here blocks startup
// or anything the user does: the user acting pushes the rest further back.
// Called again when the look changes (new styles, themes or mixes), it only does what's missing.
import { look } from "./look.svelte";
import { BACKGROUNDS, OUR_STYLES, OUR_THEMES, PATTERNS, WC_MOODS, WC_OURS, WC_PAINTS, type Look, type WcLook } from "./looks";
import { paint, painted, swatchPaint, tileMade, tileReady, wcKey } from "./paint.svelte";
import { isStored, prune } from "./paintStore";
import { calmMoment } from "./calm";

type Piece = () => Promise<unknown> | void;

/** The texture looks the panel shows: its background chips, the current background's colours,
 * our styles and yours, the theme halves (main page and Themes page). */
export function panelLooks(): Look[] {
  const a = look.active;
  const out: Look[] = [a, look.board.light];
  if (look.board.dark) out.push(look.board.dark);
  for (const b of BACKGROUNDS) out.push(b.key === a.bg ? a : { bg: b.key, colour: b.palette[0]?.name ?? "", held: "auto" });
  const cur = BACKGROUNDS.find((b) => b.key === a.bg);
  if (cur) for (const c of cur.palette) out.push({ bg: cur.key, colour: c.name, held: "auto" });
  for (const s of OUR_STYLES) out.push(s.look);
  for (const s of look.styles) out.push(s.look);
  for (const t of OUR_THEMES) out.push(t.light, t.dark);
  for (const t of look.themes) {
    out.push(t.light);
    if (t.dark) out.push(t.dark);
  }
  return out;
}

interface Swatch {
  wc: WcLook;
  w: number;
  h: number;
  /** Laid out at its own size (the paints), not at the board's shape. */
  own?: boolean;
  /** "main": the panel's first page; "customise": what Customise shows; "later": the rest. */
  group: "main" | "customise" | "later";
}

/** The watercolour swatches, with the sizes they're shown at (first the main page's). */
export function panelPaintings(): Swatch[] {
  const wc = look.wc;
  const out: Swatch[] = [];
  const push = (group: Swatch["group"], x: Omit<Swatch, "group">) => out.push({ ...x, group });
  const wcOf = (l: Look | null | undefined) => (l?.bg === "watercolour" && l.wc ? l.wc : null);
  // Main page: the theme halves, the Watercolour chip, our five, your mixes, style cards.
  for (const l of [look.board.light, look.board.dark ?? look.board.light]) {
    const w = wcOf(l);
    if (w) push("main", { wc: w, w: 30, h: 22 });
  }
  push("main", { wc, w: 76, h: 30 });
  for (const m of WC_OURS) push("main", { wc: { ...wc, colours: m.colours, pattern: "Scattered" }, w: 58, h: 46 });
  for (const m of look.mixes) push("main", { wc: { ...wc, colours: m.colours, pattern: m.pattern, paper: m.paper }, w: 58, h: 46 });
  for (const s of [...OUR_STYLES, ...look.styles]) {
    const w = wcOf(s.look);
    if (w) push("main", { wc: w, w: 100, h: 34 });
  }
  // Customise: the pattern chips in the current colours, the colour slots, the 16 paints and the
  // first row of your colours (the same washes Customise asks for).
  const wash = (rgb: string, seed: number): WcLook => ({ colours: [rgb], pattern: "Scattered", seed, paper: wc.paper });
  for (const p of PATTERNS) push("customise", { wc: { ...wc, pattern: p }, w: 76, h: 42 });
  wc.colours.forEach((c, i) => push("customise", { wc: wash(c, 30 + i * 11), w: 104, h: 46, own: true }));
  WC_PAINTS.forEach(([, rgb], i) => push("customise", { wc: wash(rgb, 60 + i * 3), w: 28, h: 28, own: true }));
  look.colours.slice(0, 8).forEach((c, i) => push("customise", { wc: wash(c.rgb, 70 + i * 5), w: 28, h: 28, own: true }));
  // Themes page, then More mixes.
  for (const t of [...OUR_THEMES, ...look.themes]) {
    for (const l of [t.light, t.dark ?? t.light]) {
      const w = wcOf(l);
      if (w) push("later", { wc: w, w: 104, h: 58 });
    }
  }
  for (const m of WC_MOODS.flatMap((x) => x.mixes)) push("later", { wc: { ...wc, colours: m.colours, pattern: "Scattered", name: m.name }, w: 104, h: 56 });
  return out;
}

/** The board-size paintings a click on Watercolour (or on one of our five) will show: the
 * current painting and our five front-row mixes, in the current pattern, seed and paper (that is
 * what picking them keeps). */
export function boardPaintings(): WcLook[] {
  const wc = look.wc;
  const out: WcLook[] = [wc];
  for (const m of WC_OURS) out.push({ ...wc, colours: m.colours });
  return out;
}

let running = false;
let again = false;
let pruned = false;

/** What isn't on the panel's first page waits this long after launch. */
const LATER_MS = 60_000;
const launched = performance.now();
const pause = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Starts (or refreshes) the warm-up. Safe to call often. */
export function warmStyle(delay = 1200) {
  if (running) {
    again = true;
    return;
  }
  running = true;
  setTimeout(() => void run(), delay);
}

async function run() {
  try {
    do {
      again = false;
      // The first page's pieces; then (`later` from that index on) the rest.
      const pieces: Piece[] = [];
      const seenTiles = new Set<string>();
      for (const l of panelLooks()) {
        const k = JSON.stringify([l.bg, l.colour, l.custom ?? ""]);
        if (seenTiles.has(k)) continue;
        seenTiles.add(k);
        pieces.push(() => (tileMade(l) ? undefined : tileReady(l)));
      }
      const seenPaint = new Set<string>();
      const swatch = (p: Swatch): Piece | null => {
        const sp = swatchPaint(p.wc, p.w, p.h, !!p.own, look.boardW, look.boardH);
        if (seenPaint.has(sp.key)) return null;
        seenPaint.add(sp.key);
        // Below everything the panel asks for itself (5) and the board (10): pure spare time.
        return async () => (painted(sp.key) || (await isStored(sp.key)) ? undefined : sp.paint(0).done);
      };
      const all = panelPaintings();
      const add = (x: Piece | null) => x && pieces.push(x);
      // The panel's first page, then Customise, then the board-size paintings a click on
      // Watercolour or one of our five will show, then the rest.
      for (const p of all) if (p.group === "main") add(swatch(p));
      const later = pieces.length;
      for (const p of all) if (p.group === "customise") add(swatch(p));
      const W = Math.round(look.boardW);
      const H = Math.round(look.boardH);
      if (W > 8 && H > 8)
        for (const wc of boardPaintings()) {
          const key = wcKey(wc, W, H, W, H);
          if (seenPaint.has(key)) continue;
          seenPaint.add(key);
          pieces.push(async () => (painted(key) || (await isStored(key)) ? undefined : paint(wc, W, H, W, H, { priority: 0 }).done));
        }
      for (const p of all) if (p.group === "later") add(swatch(p));
      if (!pruned)
        pieces.push(() => {
          pruned = true;
          return prune();
        });
      for (const [i, piece] of pieces.entries()) {
        // What the panel shows changed meanwhile (a new size while the window is resized, a new
        // mix): start over with the new list rather than painting pictures no one will ask for.
        if (again) break;
        if (i >= later)
          while (!again && performance.now() - launched < LATER_MS) await pause(Math.min(5000, LATER_MS - (performance.now() - launched)));
        if (again) break;
        await calmMoment(() => again);
        if (again) break;
        try {
          await piece();
        } catch {
          // One picture that can't be made mustn't stop the rest of the warm-up.
        }
      }
    } while (again);
  } finally {
    running = false;
  }
}
