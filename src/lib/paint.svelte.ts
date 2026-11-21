// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Pictures of the board's look, made once and kept:
// - texture tiles: small seamless pictures (the grain of paper, cork, felt…) made from the
//   design boards' filters, rendered once into a PNG and repeated as a plain background (never a
//   live SVG filter on the board: those stutter while scrolling);
// - watercolour paintings: painted a patch at a time in idle moments into a canvas, kept per
//   size and settings.
// Every chip, swatch and card uses the same pictures, so what you pick is what you get.
import { baseHex, colourOf, lookIsDark, type Look, type WcLook } from "./looks";
import { paintSteps, release, svgBitmap, wcPlan, type PaintStep, type Raster } from "./watercolour";
import { loadStored, store as storePainting } from "./paintStore";

const dpr = () => Math.min(2, Math.max(1, window.devicePixelRatio || 1));

// ---- Idle-time painting ----

interface Job {
  gen: Generator<PaintStep>;
  /** The next steps, already rasterizing (off the main thread) while earlier ones are drawn. */
  ahead: { step: PaintStep; img: Promise<Raster | null> }[];
  ended: boolean;
  /** Shared with its painting, so it can be raised while it waits. */
  pr: { v: number };
  cancelled: boolean;
  onStep?: () => void;
  resolve: () => void;
}
const queue: Job[] = [];
let running = false;

/** From this priority up, someone is waiting for the picture (the board, an open panel): its steps
 * are drawn as soon as they are ready, several rasterizing at once. Below, in idle moments only. */
const URGENT = 5;

// Drawing happens in idle moments: as many steps as fit in one, at least one per moment. Each
// step's picture is rasterized beforehand off the main thread, so the drawing is a quick copy.
let deadline: IdleDeadline | null = null;
const idleTime = async () => {
  if (deadline && deadline.timeRemaining() > 4) return;
  deadline = await new Promise<IdleDeadline | null>((r) => {
    if ("requestIdleCallback" in window) requestIdleCallback((d) => r(d), { timeout: 120 });
    else setTimeout(() => r(null), 16);
  });
};

function fill(job: Job) {
  const want = job.pr.v >= 10 ? 3 : 1;
  while (!job.ended && job.ahead.length < want) {
    const s = job.gen.next();
    if (s.done) job.ended = true;
    else job.ahead.push({ step: s.value, img: s.value.load.catch(() => null) });
  }
}

async function pump() {
  if (running) return;
  running = true;
  try {
    while (queue.length) {
      queue.sort((a, b) => b.pr.v - a.pr.v);
      const job = queue[0];
      if (!job.cancelled) fill(job);
      if (job.cancelled || !job.ahead.length) {
        for (const a of job.ahead) void a.img.then(release);
        queue.shift();
        job.resolve();
        continue;
      }
      const { step, img } = job.ahead.shift()!;
      const ready = await img;
      fill(job);
      if (job.pr.v < URGENT) await idleTime();
      if (ready && !job.cancelled) {
        step.draw(ready);
        job.onStep?.();
      }
      release(ready);
    }
  } finally {
    running = false;
    deadline = null;
  }
}

function enqueue(gen: Generator<PaintStep>, pr: { v: number }, onStep?: () => void): { done: Promise<void>; cancel: () => void } {
  let job!: Job;
  const done = new Promise<void>((resolve) => {
    job = { gen, ahead: [], ended: false, pr, cancelled: false, onStep, resolve };
  });
  queue.push(job);
  void pump();
  return { done, cancel: () => (job.cancelled = true) };
}

// ---- Watercolour paintings ----

export interface Painting {
  canvas: HTMLCanvasElement;
  done: Promise<boolean>;
  cancel: () => void;
}

const paintings = new Map<string, HTMLCanvasElement>();
const pending = new Map<string, Painting>();
// Kept in memory, the least recently used going first: at most 80 pictures and 96 MB of pixels
// (a board-size painting at 2x is ~10 MB: eighty of them would be far too much). One that is
// dropped here is only forgotten, never cleared: what shows it keeps it.
const MAX_KEPT = 80;
const MAX_KEPT_BYTES = 96 * 1024 * 1024;
let keptBytes = 0;
const bytesOf = (c: HTMLCanvasElement) => c.width * c.height * 4;
function keep(key: string, c: HTMLCanvasElement) {
  const old = paintings.get(key);
  if (old) keptBytes -= bytesOf(old);
  paintings.delete(key);
  paintings.set(key, c);
  keptBytes += bytesOf(c);
  for (const [k, v] of paintings) {
    if (paintings.size <= 1 || (paintings.size <= MAX_KEPT && keptBytes <= MAX_KEPT_BYTES)) break;
    if (k === key) continue;
    paintings.delete(k);
    keptBytes -= bytesOf(v);
  }
}

export function wcKey(wc: WcLook, W: number, H: number, outW: number, outH: number): string {
  return `${wc.colours.join("|")}~${wc.pattern}~${wc.seed}~${wc.paper}~${Math.round(W)}x${Math.round(H)}~${outW}x${outH}`;
}

/** A painting at the layout size W×H, drawn at outW×outH pixels. Already painted: comes back at
 * once (done resolves true right away). Painted before (an earlier launch): read back from the
 * kept pictures. Else painted off-screen, a patch at a time, and kept. `live` shows each patch as
 * it lands. */
export function paint(wc: WcLook, W: number, H: number, outW: number, outH: number, opts: { priority?: number; live?: boolean; onStep?: () => void } = {}): Painting {
  W = Math.max(8, Math.round(W));
  H = Math.max(8, Math.round(H));
  outW = Math.max(1, Math.round(outW));
  outH = Math.max(1, Math.round(outH));
  const key = wcKey(wc, W, H, outW, outH);
  const kept = paintings.get(key);
  if (kept) {
    paintings.delete(key);
    paintings.set(key, kept);
    return { canvas: kept, done: Promise.resolve(true), cancel: () => {} };
  }
  const already = pending.get(key);
  if (already && !opts.live) {
    // Someone is waiting now: the painting under way moves up the queue.
    raise(already, opts.priority ?? 1);
    return already;
  }
  const canvas = document.createElement("canvas");
  canvas.width = outW;
  canvas.height = outH;
  const ctx = canvas.getContext("2d")!;
  let cancelled = false;
  let job: ReturnType<typeof enqueue> | null = null;
  const priority = { v: opts.priority ?? 1 };
  let self: Painting | null = null;
  const done = (async () => {
    try {
      const stored = await loadStored(key);
      if (cancelled) return (release(stored), false);
      if (stored) {
        ctx.drawImage(stored, 0, 0, outW, outH);
        release(stored);
      } else {
        // Painted off-screen; a live painting draws straight into the canvas you see.
        const work = opts.live ? canvas : document.createElement("canvas");
        work.width = outW;
        work.height = outH;
        const plan = wcPlan(wc.colours, W, H, wc.seed, wc.pattern, { night: wc.paper === "dark" });
        job = enqueue(paintSteps(plan, work.getContext("2d")!, outW, outH), priority, opts.onStep);
        await job.done;
        if (cancelled) return false;
        if (work !== canvas) ctx.drawImage(work, 0, 0);
        storePainting(key, canvas);
      }
      keep(key, canvas);
      return true;
    } finally {
      if (pending.get(key) === self) pending.delete(key);
    }
  })();
  const p: Painting = {
    canvas,
    done,
    cancel: () => {
      cancelled = true;
      job?.cancel();
      // Asked for again right away (a look picked, another, then the first again), it must be a
      // new painting, not this cancelled one (which would never show).
      if (pending.get(key) === p) pending.delete(key);
    },
  };
  self = p;
  priorities.set(p, priority);
  pending.set(key, p);
  return p;
}

const priorities = new WeakMap<Painting, { v: number }>();
function raise(p: Painting, to: number) {
  const pr = priorities.get(p);
  if (pr && pr.v < to) pr.v = to;
}

/** A swatch's painting: laid out at the board's shape (shrunk to cover the swatch) or at the
 * swatch's own size. Shared by LookSwatch and the warm-up, so both ask for the very same picture. */
export function swatchPaint(wc: WcLook, w: number, h: number, own: boolean, boardW: number, boardH: number) {
  const LW = own ? w : boardW;
  const LH = own ? h : boardH;
  const s = Math.max(w / LW, h / LH);
  const outW = Math.round(LW * s * dpr());
  const outH = Math.round(LH * s * dpr());
  return {
    key: wcKey(wc, Math.round(LW), Math.round(LH), outW, outH),
    outW,
    outH,
    paint: (priority: number) => paint(wc, LW, LH, outW, outH, { priority }),
  };
}

/** Already painted (or being painted)? */
export function painted(key: string): boolean {
  return paintings.has(key) || pending.has(key);
}

/** Already made (or being made)? */
export function tileMade(l: Look): boolean {
  if (l.bg === "plain" || l.bg === "watercolour" || l.bg === "picture") return true;
  const key = tileKey(l);
  return tileCache.has(key) || making.has(key);
}

// ---- Texture tiles ----

class Tiles {
  /** Bumps when a tile is ready (chips showing a flat colour meanwhile pick it up). */
  v = $state(0);
}
export const tiles = new Tiles();
const tileCache = new Map<string, { url: string; size: number }>();
/** Tiles being made: one making per tile, which whoever asks next waits for (a second making
 * would leave the first one's blob: URL behind). */
const making = new Map<string, Promise<void>>();
function make(l: Look, key: string): Promise<void> {
  let p = making.get(key);
  if (!p) {
    p = makeTile(l, key).finally(() => making.delete(key));
    making.set(key, p);
  }
  return p;
}

// The grain filters of the design board "BgColours" (dark grain + light grain per texture),
// made seamless (stitchTiles) so a small tile repeats without a seam.
const grain = (id: string, freq: string, oct: number, seed: number, rgb: string, a: string) =>
  `<filter id="${id}" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="${freq}" numOctaves="${oct}" seed="${seed}" stitchTiles="stitch"/><feColorMatrix type="matrix" values="0 0 0 0 ${rgb.split(" ")[0]}  0 0 0 0 ${rgb.split(" ")[1]}  0 0 0 0 ${rgb.split(" ")[2]}  ${a}"/></filter>`;
const linen = (id: string, s1: number, s2: number, rgb: string) =>
  `<filter id="${id}" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.012 0.75" numOctaves="2" seed="${s1}" stitchTiles="stitch" result="a"/><feTurbulence type="fractalNoise" baseFrequency="0.75 0.012" numOctaves="2" seed="${s2}" stitchTiles="stitch" result="b"/><feComposite in="a" in2="b" operator="arithmetic" k2="0.5" k3="0.5"/><feColorMatrix type="matrix" values="0 0 0 0 ${rgb.split(" ")[0]}  0 0 0 0 ${rgb.split(" ")[1]}  0 0 0 0 ${rgb.split(" ")[2]}  1.4 0 0 0 -0.55"/></filter>`;

interface Grain {
  size: number;
  d: string;
  l: string;
  /** Dark-grain and light-grain opacity on light colours / on dark ones. */
  dL: number;
  lL: number;
  dD: number;
  lD: number;
}
const GRAINS: Partial<Record<Look["bg"], Grain>> = {
  paper: { size: 256, d: grain("d", "0.85", 2, 2, "0 0 0", "0.9 0 0 0 -0.35"), l: grain("l", "0.85", 2, 9, "1 1 1", "0.9 0 0 0 -0.35"), dL: 0.28, lL: 0, dD: 0.25, lD: 0.1 },
  linen: { size: 512, d: linen("d", 4, 8, "0 0 0"), l: linen("l", 14, 18, "1 1 1"), dL: 0.45, lL: 0.15, dD: 0.4, lD: 0.18 },
  cork: { size: 256, d: grain("d", "0.7", 2, 5, "0.12 0.07 0.03", "1.9 0 0 0 -0.88"), l: grain("l", "0.55", 2, 11, "1 0.92 0.78", "1.8 0 0 0 -0.98"), dL: 1, lL: 0.75, dD: 1, lD: 0.35 },
  plaster: { size: 512, d: grain("d", "0.009", 4, 4, "0 0 0", "1.1 0 0 0 -0.42"), l: grain("l", "0.011", 4, 10, "1 1 1", "1.1 0 0 0 -0.42"), dL: 0.4, lL: 0.25, dD: 0.4, lD: 0.2 },
  kraft: { size: 512, d: grain("d", "0.018 0.4", 2, 7, "0.1 0.06 0.02", "1.4 0 0 0 -0.62"), l: grain("l", "0.9", 1, 12, "1 0.95 0.85", "1.7 0 0 0 -0.98"), dL: 0.55, lL: 0.5, dD: 0.5, lD: 0.3 },
  felt: { size: 256, d: grain("d", "1.1", 2, 3, "0 0 0", "1.3 0 0 0 -0.5"), l: grain("l", "1.1", 2, 13, "1 1 1", "1.3 0 0 0 -0.5"), dL: 0.5, lL: 0.12, dD: 0.5, lD: 0.14 },
};

/** A seeded little random for the terrazzo chips (the board's own recipe). */
function chipRandom(seed: number) {
  return () => {
    seed = (seed * 9301 + 49297) % 233280;
    return seed / 233280;
  };
}

function tileSvg(l: Look): { svg: string; size: number } | null {
  const hex = baseHex(l);
  const dark = lookIsDark(l);
  const g = GRAINS[l.bg];
  if (g) {
    const S = g.size;
    const dO = dark ? g.dD : g.dL;
    const lO = dark ? g.lD : g.lL;
    return {
      size: S,
      svg: `<defs>${g.d}${g.l}</defs><rect width="${S}" height="${S}" fill="${hex}"/>${dO ? `<rect width="${S}" height="${S}" filter="url(#d)" opacity="${dO}"/>` : ""}${lO ? `<rect width="${S}" height="${S}" filter="url(#l)" opacity="${lO}"/>` : ""}`,
    };
  }
  if (l.bg === "dotted") {
    const S = 18;
    return { size: S, svg: `<rect width="${S}" height="${S}" fill="${hex}"/><circle cx="9" cy="9" r="1.25" fill="${dark ? "rgba(255,255,255,0.28)" : "rgba(120,110,95,0.42)"}"/>` };
  }
  if (l.bg === "terrazzo") {
    const S = 280;
    const chips = colourOf(l).chips ?? ["#C9785A", "#9DB09A", "#3E4552", "#D9B26A"];
    const rnd = chipRandom(7);
    let out = "";
    for (let i = 0; i < 64; i++) {
      const w = 3 + Math.round(rnd() * 9);
      const h = Math.max(2, Math.round(w * (0.5 + rnd() * 0.7)));
      const x = rnd() * S;
      const y = rnd() * S;
      const col = chips[Math.floor(rnd() * chips.length)];
      const rx = (w * (20 + rnd() * 40)) / 100;
      const ry = (h * (20 + rnd() * 50)) / 100;
      const r = Math.round(rnd() * 180);
      // Chips crossing an edge are drawn again on the other side (the tile repeats seamlessly).
      for (const dx of [0, x + w > S ? -S : 0, x < 0 ? S : 0].filter((v, k, a) => a.indexOf(v) === k))
        for (const dy of [0, y + h > S ? -S : 0].filter((v, k, a) => a.indexOf(v) === k))
          out += `<rect x="${(x + dx).toFixed(1)}" y="${(y + dy).toFixed(1)}" width="${w}" height="${h}" rx="${rx.toFixed(1)}" ry="${ry.toFixed(1)}" fill="${col}" transform="rotate(${r} ${(x + dx + w / 2).toFixed(1)} ${(y + dy + h / 2).toFixed(1)})"/>`;
    }
    return {
      size: S,
      svg: `<defs>${grain("g", "0.85", 2, 21, "0 0 0", "0.9 0 0 0 -0.38")}</defs><rect width="${S}" height="${S}" fill="${hex}"/>${out}<rect width="${S}" height="${S}" filter="url(#g)" opacity="0.18"/>`,
    };
  }
  if (l.bg === "blueprint") {
    const S = 100;
    const line = dark ? "255,255,255" : "31,78,140";
    const [major, minor] = dark ? [0.22, 0.09] : [0.3, 0.12];
    let lines = "";
    for (let k = 20; k < S; k += 20) lines += `<rect x="0" y="${k}" width="${S}" height="1" fill="rgba(${line},${minor})"/><rect x="${k}" y="0" width="1" height="${S}" fill="rgba(${line},${minor})"/>`;
    return {
      size: S,
      svg: `<defs>${grain("g", "0.9", 1, 31, dark ? "1 1 1" : "0.12 0.3 0.55", "1.6 0 0 0 -0.98")}</defs><rect width="${S}" height="${S}" fill="${hex}"/>${lines}<rect x="0" y="0" width="${S}" height="1" fill="rgba(${line},${major})"/><rect x="0" y="0" width="1" height="${S}" fill="rgba(${line},${major})"/><rect width="${S}" height="${S}" filter="url(#g)" opacity="0.12"/>`,
    };
  }
  return null;
}

function tileKey(l: Look): string {
  return `${l.bg}~${baseHex(l)}~${l.colour}`;
}

async function makeTile(l: Look, key: string) {
  const t = tileSvg(l);
  if (!t) return;
  const px = Math.round(t.size * dpr());
  const img = await svgBitmap(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${t.size} ${t.size}" width="${px}" height="${px}">${t.svg}</svg>`);
  const c = document.createElement("canvas");
  c.width = px;
  c.height = px;
  c.getContext("2d")!.drawImage(img, 0, 0, px, px);
  release(img);
  // A short blob: URL, not a data: URL: the tiles are large PNGs, and a data: URL is the whole
  // picture as text, encoded on the spot and parsed again every time a style uses it (that was
  // most of the Style panel's opening time). Kept for the app's life, so never revoked.
  const blob = await new Promise<Blob | null>((r) => c.toBlob(r, "image/png"));
  if (!blob || tileCache.has(key)) return;
  tileCache.set(key, { url: URL.createObjectURL(blob), size: t.size });
  tiles.v++;
}

/** The CSS background of a textured look (its tile repeated over its colour). Watercolour and
 * pictures are painted separately; until a tile is ready this is the plain colour. */
export function textureCss(l: Look): string {
  void tiles.v;
  const hex = baseHex(l);
  if (l.bg === "plain" || l.bg === "watercolour" || l.bg === "picture") return hex;
  const key = tileKey(l);
  const t = tileCache.get(key);
  if (t) return `url(${t.url}) 0 0 / ${t.size}px ${t.size}px repeat, ${hex}`;
  void make(l, key).catch(() => {});
  return hex;
}

/** Ready-made for a look (so a crossfade can wait for it). */
export async function tileReady(l: Look): Promise<void> {
  if (l.bg === "plain" || l.bg === "watercolour" || l.bg === "picture") return;
  const key = tileKey(l);
  if (tileCache.has(key)) return;
  await make(l, key);
}
