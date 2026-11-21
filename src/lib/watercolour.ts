// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The watercolour painter (recipe v6, approved in the design boards: wc-patterns/patterns.js),
// ported as is. Same random sequence, same strokes, same filters. The only difference: the
// painting is split into its patches, so the board can paint it a patch at a time in idle
// moments (and watch it paint in), instead of one long blocking render.
//
// wcSvg() gives back exactly the original single SVG (used for the small chips); wcPlan() gives
// the patches for paintCanvas().
import type { Pattern } from "./looks";

function rng(seed: number) {
  return () => {
    seed = (seed * 16807) % 2147483647;
    return (seed - 1) / 2147483646;
  };
}

const WC_BLOOMS = true; // the light "bloom" spots: on but very faint (user: organic, not distracting)
// Night mode: dark paper, washes lifted toward light and laid with "screen" so overlaps glow instead of darkening.
const WC_NIGHT = { paper: "27,28,32", lift: 0.08, strength: 0.55, blend: "screen", grain: "soft-light", grainOpacity: 0.3 } as const;
function wcLift(c: string, t: number) {
  return c
    .split(",")
    .map((v) => Math.round(+v + (255 - +v) * t))
    .join(",");
}

/** The SVG filters the painter needs (dry brush, flicks, glazes, paper grain). */
export const WC_FILTERS = `<filter id="dry" x="-15%" y="-15%" width="130%" height="130%" color-interpolation-filters="sRGB"><feTurbulence type="fractalNoise" baseFrequency="0.008 0.05" numOctaves="3" seed="5" result="warp"/><feDisplacementMap in="SourceGraphic" in2="warp" scale="18" xChannelSelector="R" yChannelSelector="G" result="shape"/><feTurbulence type="fractalNoise" baseFrequency="0.02 0.22" numOctaves="2" seed="21" result="br"/><feColorMatrix in="br" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.8 0.45" result="brm0"/><feGaussianBlur in="brm0" stdDeviation="1.2 0.6" result="brm"/><feComposite in="shape" in2="brm" operator="in" result="streaky"/><feGaussianBlur in="streaky" stdDeviation="0.8" result="sharp"/><feGaussianBlur in="streaky" stdDeviation="4" result="soft"/><feComposite in="sharp" in2="soft" operator="arithmetic" k2="0.55" k3="-0.45" result="edge"/><feMerge><feMergeNode in="sharp"/><feMergeNode in="edge"/></feMerge></filter><filter id="flick" x="-15%" y="-30%" width="130%" height="160%" color-interpolation-filters="sRGB"><feTurbulence type="fractalNoise" baseFrequency="0.015 0.08" numOctaves="2" seed="7" result="warp"/><feDisplacementMap in="SourceGraphic" in2="warp" scale="14" xChannelSelector="R" yChannelSelector="G" result="shape"/><feTurbulence type="fractalNoise" baseFrequency="0.03 0.3" numOctaves="2" seed="33" result="br"/><feColorMatrix in="br" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 2 -0.45" result="brm0"/><feGaussianBlur in="brm0" stdDeviation="1.5 0.7" result="brm"/><feComposite in="shape" in2="brm" operator="in"/></filter><filter id="glaze" x="-30%" y="-30%" width="160%" height="160%" color-interpolation-filters="sRGB"><feTurbulence type="fractalNoise" baseFrequency="0.012" numOctaves="4" seed="8" result="w"/><feDisplacementMap in="SourceGraphic" in2="w" scale="60" xChannelSelector="R" yChannelSelector="G" result="s"/><feGaussianBlur in="s" stdDeviation="5" result="soft"/><feGaussianBlur in="s" stdDeviation="1" result="sharp"/><feComposite in="sharp" in2="soft" operator="arithmetic" k2="1.3" k3="-1.05" result="edge"/><feTurbulence type="fractalNoise" baseFrequency="0.7" numOctaves="2" seed="9" result="g"/><feColorMatrix in="g" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.55 0.7" result="gm"/><feComposite in="soft" in2="gm" operator="in" result="grainy"/><feMerge><feMergeNode in="grainy"/><feMergeNode in="edge"/></feMerge></filter><filter id="paper" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.04" numOctaves="4" seed="2" result="n"/><feDiffuseLighting in="n" lighting-color="#ffffff" surfaceScale="1.4" result="lit"><feDistantLight azimuth="225" elevation="58"/></feDiffuseLighting><feColorMatrix in="lit" type="matrix" values="0.3 0 0 0 0.7  0.3 0 0 0 0.7  0.3 0 0 0 0.7  0 0 0 0 1"/></filter>`;

export interface WcPatch {
  /** The patch's glaze, flicks and strokes (laid with the painting's blend). */
  body: string;
  /** Its bloom, laid normally on top (may be empty). */
  bloom: string;
  /** Where it can paint (layout units, generous: flicks, turns, filter spread), so each step
   * only rasterizes and blends its own corner of the board. */
  box: { x: number; y: number; w: number; h: number };
}
export interface WcPlanned {
  W: number;
  H: number;
  grads: string;
  patches: WcPatch[];
  paper: string;
  blend: "multiply" | "screen";
  grain: "multiply" | "soft-light";
  grainOpacity: number;
}

let uidCounter = 0;

/** The painting, patch by patch (the original wcPaint's exact drawing). */
export function wcPlan(mixIn: string[], W: number, H: number, seed: number, pattern: Pattern, opts?: { night?: boolean }): WcPlanned {
  const night = !!opts?.night;
  let mix = mixIn;
  if (night) mix = mix.map((c) => wcLift(c, WC_NIGHT.lift));
  const BLEND = night ? WC_NIGHT.blend : "multiply";
  const PAPER = night ? WC_NIGHT.paper : "253,252,249";
  const r = rng(seed);
  let grads = "";
  const uid = "p" + seed + pattern.replace(/\W/g, "") + "_" + ++uidCounter; // unique per painting: same seed + different colours must not share gradient ids
  mix.forEach((c, i) => {
    grads += `<linearGradient id="${uid}s${i}" x1="0" x2="1"><stop offset="0" stop-color="rgb(${c})" stop-opacity="0"/><stop offset=".14" stop-color="rgb(${c})"/><stop offset=".86" stop-color="rgb(${c})"/><stop offset="1" stop-color="rgb(${c})" stop-opacity="0"/></linearGradient>`;
    grads += `<linearGradient id="${uid}l${i}" x1="0" x2="1"><stop offset="0" stop-color="rgb(${c})" stop-opacity="0"/><stop offset=".75" stop-color="rgb(${c})"/></linearGradient>`;
    grads += `<linearGradient id="${uid}r${i}" x1="0" x2="1"><stop offset=".25" stop-color="rgb(${c})"/><stop offset="1" stop-color="rgb(${c})" stop-opacity="0"/></linearGradient>`;
  });
  const solid = mix.length === 1;
  const patches: WcPatch[] = [];
  const strength = night ? WC_NIGHT.strength : 1;
  // one brushed patch in one colour, its own multiply layer (overlaps deepen like real glazing)
  function patch(ci: number, x0: number, x1: number, y0: number, y1: number, angle?: number) {
    const pw = x1 - x0,
      ph = y1 - y0;
    const pa = angle === undefined ? (r() - 0.5) * 80 : angle,
      pcx = (x0 + x1) / 2,
      pcy = (y0 + y1) / 2;
    const PT = `rotate(${pa.toFixed(1)} ${pcx} ${pcy}) `;
    const k = (solid ? 0.75 + r() * 0.5 : 1) * strength;
    const flip = r() < 0.5;
    const grade = (f: number) => 0.5 + 0.5 * (flip ? f : 1 - f);
    let gl = "",
      st = "",
      fls = "";
    gl += `<ellipse transform="${PT}" cx="${pcx}" cy="${pcy}" rx="${pw * 0.42}" ry="${ph * 0.4}" fill="rgba(${mix[ci]},${(0.2 * k).toFixed(2)})"/>`;
    const fl = Math.max(2, Math.round(ph / 34));
    for (let i = 0; i < fl; i++) {
      const f = 0.1 + r() * 0.8,
        cy = y0 + ph * f;
      const th = 6 + r() * 10;
      const len = pw * (0.12 + r() * 0.14);
      const left = r() < 0.5;
      const sx = left ? x0 + pw * 0.08 - len : x1 - pw * 0.08;
      const fa = (r() - 0.5) * 26,
        px = left ? sx + len : sx;
      fls += `<g transform="${PT}rotate(${fa.toFixed(1)} ${px} ${cy + th / 2})"><rect x="${sx}" y="${cy}" width="${len}" height="${th}" rx="${th / 2}" fill="url(#${uid}${left ? "l" : "r"}${ci})" fill-opacity="${((0.3 + r() * 0.15) * k * grade(f)).toFixed(2)}" filter="url(#flick)"/></g>`;
    }
    const rows = Math.max(5, Math.round(ph / 24));
    for (let i = 0; i < rows; i++) {
      const f = (i + 0.5) / rows,
        cy = y0 + ph * f + (r() - 0.5) * 6;
      const n = 1 + Math.floor(r() * 2);
      for (let q = 0; q < n; q++) {
        const edge = Math.abs(f - 0.5) * 2;
        const len = pw * (0.45 + r() * 0.4) * (1 - edge * 0.45);
        const sx = x0 + (pw - len) * (0.5 + (r() - 0.5) * 0.8);
        const th = (ph / rows) * (1.6 + r() * 1.2);
        const a = (0.2 + r() * 0.16) * k * grade(f);
        const ang = (r() - 0.5) * 16;
        st += `<g transform="${PT}rotate(${ang.toFixed(1)} ${sx + len / 2} ${cy})"><rect x="${sx}" y="${cy - th / 2}" width="${len}" height="${th}" rx="${th / 2}" fill="url(#${uid}s${ci})" fill-opacity="${a.toFixed(2)}" filter="url(#dry)"/></g>`;
      }
    }
    const g = 1 + Math.round(r());
    for (let i = 0; i < g; i++) {
      const f = 0.3 + r() * 0.4;
      const cx = x0 + pw * (0.3 + r() * 0.4),
        cy = y0 + ph * f;
      gl += `<ellipse transform="${PT}" cx="${cx}" cy="${cy}" rx="${pw * (0.18 + r() * 0.14)}" ry="${ph * (0.16 + r() * 0.12)}" fill="rgba(${mix[ci]},${((0.14 + r() * 0.1) * k * grade(f)).toFixed(2)})"/>`;
    }
    let bloom = "";
    if (r() < 0.45) {
      const bx = x0 + pw * (0.3 + r() * 0.4),
        by = y0 + ph * (0.3 + r() * 0.4),
        br = Math.min(pw, ph) * (0.064 + r() * 0.048);
      if (WC_BLOOMS)
        bloom = `<g filter="url(#glaze)"><circle cx="${bx}" cy="${by}" r="${br}" fill="rgba(${PAPER},${night ? 0.22 : 0.16})"/><circle cx="${bx}" cy="${by}" r="${br}" fill="none" stroke="rgba(${mix[ci]},0.1)" stroke-width="1.5"/></g>`;
    }
    // How far anything of the patch can reach from its centre before its turn: flicks stick out
    // ~0.18 of the width past each side, strokes reach 0.68 of the height, flicks/strokes tilt a
    // little more; + the filters' spread (displacement up to 30 px, blurs) and a safe margin.
    const ex = 0.7 * pw + 60,
      ey = 0.72 * ph + 0.08 * pw + 60;
    const ca = Math.abs(Math.cos((pa * Math.PI) / 180)),
      sa = Math.abs(Math.sin((pa * Math.PI) / 180));
    const hx = ca * ex + sa * ey,
      hy = sa * ex + ca * ey;
    const bx0 = Math.max(0, pcx - hx),
      by0 = Math.max(0, pcy - hy),
      bx1 = Math.min(W, pcx + hx),
      by1 = Math.min(H, pcy + hy);
    patches.push({ body: `<g filter="url(#glaze)">${gl}</g>${fls}${st}`, bloom, box: { x: bx0, y: by0, w: Math.max(0, bx1 - bx0), h: Math.max(0, by1 - by0) } });
  }
  const n = mix.length;
  const at = (ci: number, cx: number, cy: number, pw: number, ph: number, angle?: number) => patch(solid ? 0 : ci % n, cx - pw / 2, cx + pw / 2, cy - ph / 2, cy + ph / 2, angle);

  // Every pattern uses the SAME brush unit as the approved Scattered painting (same patch size, same overlap,
  // same jittered grid). A pattern only decides which cells get a patch and which way the patches turn.
  const cols = Math.max(3, Math.round(W / 190)),
    rws = Math.max(3, Math.round(H / 150));
  const cw = W / cols,
    chh = H / rws;
  const A = -20,
    tA = Math.tan((A * Math.PI) / 180);
  const ox = 0.42 + r() * 0.16,
    oy = 0.42 + r() * 0.16; // One patch: where the bloom sits
  const third = r() < 0.5; // Corners: which spare corner gets a small touch
  const weight = (u: number, v: number) => {
    if (pattern === "Corners") {
      const d = Math.min(Math.hypot(u, v), Math.hypot(1 - u, 1 - v));
      const d3 = third ? Math.hypot(1 - u, v) : Math.hypot(u, 1 - v);
      return d < 0.4 ? 1 : d < 0.62 ? 0.6 : d3 < 0.25 ? 0.5 : 0;
    }
    if (pattern === "One patch") {
      const d = Math.hypot((u - ox) / 0.5, (v - oy) / 0.5);
      return d < 0.4 ? 1 : d < 0.75 ? 0.55 : 0;
    }
    return 1;
  };
  const j0 = pattern === "Sweep" ? -1 : 0,
    j1 = pattern === "Sweep" ? rws + 1 : rws; // Sweep rows lean, so add a row above and below
  const cells: [number, number][] = [];
  for (let j = j0; j < j1; j++) for (let i = 0; i < cols; i++) cells.push([i, j]);
  cells.sort(() => r() - 0.5);
  for (const [i, j] of cells) {
    if (r() < 0.08) continue; // leave some paper breathing
    const sz = 0.85 + r() * 0.75;
    const cx = cw * (i + 0.5) + (r() - 0.5) * cw * 0.7;
    let cy = chh * (j + 0.5) + (r() - 0.5) * chh * 0.7;
    if (r() >= weight(cx / W, cy / H)) continue;
    let angle: number | undefined;
    if (pattern === "Sweep") {
      cy = chh * (j + 0.5) + (r() - 0.5) * chh * 0.12 + (cx - W / 2) * tA; // rows of patches tilted the same way: diagonal bands
      angle = A + (r() - 0.5) * 8;
    }
    const ci = pattern === "Sweep" ? j + 1 + seed : i + j * 2 + seed; // Sweep: one colour per band
    const sw = pattern === "Sweep" ? [1.2, 0.8] : [1, 1]; // Sweep: same brush, pulled a little longer along the band
    at(ci, cx, cy, cw * 1.25 * sz * sw[0], chh * 1.15 * sz * sw[1], angle);
  }
  return {
    W,
    H,
    grads,
    patches,
    paper: PAPER,
    blend: BLEND,
    grain: night ? WC_NIGHT.grain : "multiply",
    grainOpacity: night ? WC_NIGHT.grainOpacity : 0.35,
  };
}

/** The whole painting as one SVG (exactly what wcPaint made), with its filters inside. */
export function wcSvg(p: WcPlanned, outW = p.W, outH = p.H): string {
  const patches = p.patches.map((x) => `<g style="mix-blend-mode:${p.blend}">${x.body}</g>${x.bloom}`).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${p.W} ${p.H}" width="${outW}" height="${outH}" style="isolation:isolate"><defs>${WC_FILTERS}${p.grads}</defs><rect width="${p.W}" height="${p.H}" fill="rgb(${p.paper})"/>${patches}<rect width="${p.W}" height="${p.H}" filter="url(#paper)" style="mix-blend-mode:${p.grain}" opacity="${p.grainOpacity}"/></svg>`;
}

/** A part of the board, in whole output pixels (and the layout area that maps onto them). */
interface Region {
  px: number;
  py: number;
  pw: number;
  ph: number;
  vb: string;
}
function region(p: WcPlanned, outW: number, outH: number, box?: WcPatch["box"]): Region {
  if (!box) return { px: 0, py: 0, pw: outW, ph: outH, vb: `0 0 ${p.W} ${p.H}` };
  const sx = outW / p.W,
    sy = outH / p.H;
  const px = Math.max(0, Math.floor(box.x * sx)),
    py = Math.max(0, Math.floor(box.y * sy));
  const pw = Math.min(outW, Math.ceil((box.x + box.w) * sx)) - px,
    ph = Math.min(outH, Math.ceil((box.y + box.h) * sy)) - py;
  // The view box follows the whole pixels exactly, so the cut-out lands on the same pixels the
  // full-board layer would have (same scale, same noise: filters work in board units).
  return { px, py, pw, ph, vb: `${px / sx} ${py / sy} ${pw / sx} ${ph / sy}` };
}

function layerSvg(p: WcPlanned, inner: string, r: Region, withGrads: boolean): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${r.vb}" width="${r.pw}" height="${r.ph}"><defs>${WC_FILTERS}${withGrads ? p.grads : ""}</defs>${inner}</svg>`;
}

export async function svgImage(svg: string): Promise<HTMLImageElement> {
  const img = new Image();
  img.decoding = "async";
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  await img.decode();
  return img;
}

/** A picture ready to draw: rasterized already (ImageBitmap), or the SVG itself where that
 * isn't possible. */
export type Raster = ImageBitmap | HTMLImageElement;

/** The SVG rasterized, filters and all, off the main thread. Drawing an SVG picture straight into
 * a canvas looks free, but its filters are then rasterized on the main thread the next time the
 * canvas is shown: 50–230 ms for one patch of the board, a visible hitch every time a watercolour
 * was painted. createImageBitmap does the same work on its own (measured: an 89 ms patch held up
 * no frame) and gives the same pixels. */
export async function svgBitmap(svg: string): Promise<Raster> {
  const img = await svgImage(svg);
  if (typeof createImageBitmap !== "function") return img;
  try {
    const bm = await createImageBitmap(img);
    // Let go of the SVG picture right away (each one is a whole little document): less for the
    // garbage collector to sweep later in one go, mid paint-in.
    img.src = "";
    return bm;
  } catch {
    return img;
  }
}

/** Lets go of a bitmap's memory once it has been drawn (no-op for a picture). */
export function release(r: Raster | null | undefined) {
  if (r && "close" in r) r.close();
}

export interface PaintStep {
  /** Prepared off the clock (rasterized on its own, off the main thread). */
  load: Promise<Raster>;
  /** The drawing itself: a plain copy of a ready picture (run it in an idle moment). */
  draw: (img: Raster) => void;
}

/** Paints into a canvas one patch per step (each step a quick copy); the caller runs the steps in
 * idle moments. The result is the same picture as wcSvg(). */
export function* paintSteps(p: WcPlanned, ctx: CanvasRenderingContext2D, outW: number, outH: number): Generator<PaintStep> {
  ctx.globalCompositeOperation = "source-over";
  ctx.globalAlpha = 1;
  ctx.fillStyle = `rgb(${p.paper})`;
  ctx.fillRect(0, 0, outW, outH);
  for (const patch of p.patches) {
    const r = region(p, outW, outH, patch.box);
    if (r.pw < 1 || r.ph < 1) continue;
    yield {
      load: svgBitmap(layerSvg(p, patch.body, r, true)),
      draw: (img) => {
        ctx.globalCompositeOperation = p.blend;
        ctx.drawImage(img, r.px, r.py, r.pw, r.ph);
      },
    };
    if (patch.bloom)
      yield {
        load: svgBitmap(layerSvg(p, patch.bloom, r, false)),
        draw: (img) => {
          ctx.globalCompositeOperation = "source-over";
          ctx.drawImage(img, r.px, r.py, r.pw, r.ph);
        },
      };
  }
  const all = region(p, outW, outH);
  yield {
    load: svgBitmap(layerSvg(p, `<rect width="${p.W}" height="${p.H}" filter="url(#paper)"/>`, all, false)),
    draw: (img) => {
      ctx.globalCompositeOperation = p.grain;
      ctx.globalAlpha = p.grainOpacity;
      ctx.drawImage(img, 0, 0, outW, outH);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },
  };
}
