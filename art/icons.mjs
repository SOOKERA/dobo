// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo's logo as pictures, from the drawings in art/logo. From Dobo's repo root:
//   node art/icons.mjs
// (needs Playwright's Chromium: npx playwright install chromium). It makes:
//
// 1 · src-tauri/icons: the app's icon (Dobo.exe, the taskbar, Alt+Tab, the Start menu,
//     Explorer, Setup, notifications). The owner's choice (2026-11-23): from 24 px up, the true
//     logo, the block note alone with no tile (art/logo/mark.svg, cropped to the note so it
//     fills the icon); at 16 and 20 px, the compact note drawn for those sizes (art/logo/icon-16.svg,
//     icon-20.svg), as in the tray. src-tauri/icons/README.md lists each file's source. The
//     tray's own pictures (src-tauri/icons/tray) are not made here.
// 2 · public/logo: the true mark without its tile (art/logo/mark.svg, mark-dark.svg for dark
//     grounds) at the sizes src/components/Logo.svelte shows it, at 1×, 1.5×, 2× and 3× (screens
//     at 100 % to 300 %), as WebP: the drawings are 900 KB each, too heavy for the page.
//
// Every picture is drawn at its own size from the vector drawing (nothing scaled down from a
// bigger picture), on a transparent ground.
import { chromium } from "playwright";
import { mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const LOGO = join("art", "logo");
const ICONS = join("src-tauri", "icons");
const WEB = join("public", "logo");
const svg = (f) => readFileSync(join(LOGO, f), "utf8");

// The mark's sizes in the app (Logo.svelte's `size`: the header 32, the welcome / terms /
// recovery kickers 22, About 20) and the screen scales they are drawn for.
const MARK_SIZES = [20, 22, 32];
const MARK_SCALES = [1, 1.5, 2, 3];
// The note in mark.svg's own units (its viewBox is 20.7 19.3 224 224): measured from the drawing,
// its hatched shadow included. Logo.svelte shows it as tall as the compact note was (30 of 32),
// in a box as wide as the note and a hair more, so the gap beside it is the gap to the note.
const NOTE = { x0: 44.2, y0: 30.05, x1: 218.7, y1: 229.3 };
const NOTE_OF_BOX = 30 / 32;
const markWidth = (size) => 2 * Math.ceil(((size * NOTE_OF_BOX * (NOTE.x1 - NOTE.x0)) / (NOTE.y1 - NOTE.y0) + 1) / 2);

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setContent("<!doctype html><html><body></body></html>");

/** Draws `text` (an SVG) into a w × h canvas; returns { png, webp (quality q), rgba }. */
async function draw(text, w, h = w, q = 0.92) {
  const url = "data:image/svg+xml;base64," + Buffer.from(text).toString("base64");
  const r = await page.evaluate(
    async ([url, w, h, q]) => {
      const im = new Image();
      im.src = url;
      await im.decode();
      const c = document.createElement("canvas");
      c.width = w;
      c.height = h;
      const x = c.getContext("2d");
      x.imageSmoothingQuality = "high";
      x.drawImage(im, 0, 0, w, h);
      const d = x.getImageData(0, 0, w, h).data;
      let s = "";
      for (let i = 0; i < d.length; i += 32768) s += String.fromCharCode.apply(null, d.subarray(i, i + 32768));
      return { png: c.toDataURL("image/png"), webp: c.toDataURL("image/webp", q), rgba: btoa(s) };
    },
    [url, w, h, q],
  );
  const bytes = (u) => Buffer.from(u.split(",")[1], "base64");
  return { png: bytes(r.png), webp: bytes(r.webp), rgba: Buffer.from(r.rgba, "base64") };
}

// ── 1 · The app's icon ───────────────────────────────────────────────────────────────────────
// From 24 px up: the block note alone (mark.svg), no tile (the owner's choice, 2026-11-23: "Note
// only, everywhere"). Its viewBox is cropped to a square round the note (NOTE, plus a hair), so
// the note fills the icon's height: as big and readable as it can be.
const S = NOTE.y1 - NOTE.y0 + 2;
const noteOnly = () =>
  svg("mark.svg").replace(
    /viewBox="[^"]*"/,
    `viewBox="${((NOTE.x0 + NOTE.x1) / 2 - S / 2).toFixed(2)} ${(NOTE.y0 - 1).toFixed(2)} ${S.toFixed(2)} ${S.toFixed(2)}"`,
  );
const sourceText = (n) => (n <= 20 ? svg(`icon-${n}.svg`) : noteOnly());
const made = new Map();
const icon = async (n) => {
  if (!made.has(n)) made.set(n, await draw(sourceText(n), n));
  return made.get(n);
};

// A Windows .ico from ready-drawn pictures: each entry a 32-bit bitmap (BITMAPINFOHEADER,
// bottom-up BGRA, an empty AND mask: the alpha does the work), 256 px the PNG itself (as
// Windows Vista and later read it).
function ico(entries) {
  const blobs = entries.map(({ n, rgba, png }) => {
    if (n >= 256) return png;
    const head = Buffer.alloc(40);
    head.writeUInt32LE(40, 0);
    head.writeInt32LE(n, 4);
    head.writeInt32LE(n * 2, 8);
    head.writeUInt16LE(1, 12);
    head.writeUInt16LE(32, 14);
    const maskRow = Math.ceil(n / 32) * 4;
    head.writeUInt32LE(n * n * 4 + maskRow * n, 20);
    const px = Buffer.alloc(n * n * 4);
    for (let y = 0; y < n; y++)
      for (let x = 0; x < n; x++) {
        const s = (y * n + x) * 4;
        const d = ((n - 1 - y) * n + x) * 4;
        px[d] = rgba[s + 2];
        px[d + 1] = rgba[s + 1];
        px[d + 2] = rgba[s];
        px[d + 3] = rgba[s + 3];
      }
    return Buffer.concat([head, px, Buffer.alloc(maskRow * n)]);
  });
  const dir = Buffer.alloc(6 + 16 * entries.length);
  dir.writeUInt16LE(1, 2);
  dir.writeUInt16LE(entries.length, 4);
  let off = dir.length;
  entries.forEach(({ n }, i) => {
    const e = 6 + 16 * i;
    dir.writeUInt8(n >= 256 ? 0 : n, e);
    dir.writeUInt8(n >= 256 ? 0 : n, e + 1);
    dir.writeUInt16LE(1, e + 4);
    dir.writeUInt16LE(32, e + 6);
    dir.writeUInt32LE(blobs[i].length, e + 8);
    dir.writeUInt32LE(off, e + 12);
    off += blobs[i].length;
  });
  return Buffer.concat([dir, ...blobs]);
}

// A macOS .icns of PNGs (Dobo ships for Windows only; Tauri's config lists one all the same).
function icns(entries) {
  const parts = entries.map(([type, png]) => {
    const h = Buffer.alloc(8);
    h.write(type, 0, "latin1");
    h.writeUInt32BE(png.length + 8, 4);
    return Buffer.concat([h, png]);
  });
  const h = Buffer.alloc(8);
  h.write("icns", 0, "latin1");
  h.writeUInt32BE(8 + parts.reduce((s, p) => s + p.length, 0), 4);
  return Buffer.concat([h, ...parts]);
}

const icoSizes = [16, 20, 24, 32, 40, 48, 64, 256];
const icoEntries = [];
for (const n of icoSizes) icoEntries.push({ n, ...(await icon(n)) });
writeFileSync(join(ICONS, "icon.ico"), ico(icoEntries));
const pngs = {
  "32x32.png": 32,
  "64x64.png": 64,
  "128x128.png": 128,
  "128x128@2x.png": 256,
  "icon.png": 512,
  "StoreLogo.png": 50,
  ...Object.fromEntries([30, 44, 71, 89, 107, 142, 150, 284, 310].map((n) => [`Square${n}x${n}Logo.png`, n])),
};
for (const [f, n] of Object.entries(pngs)) writeFileSync(join(ICONS, f), (await icon(n)).png);
const icnsTypes = [["icp4", 16], ["icp5", 32], ["ic11", 32], ["icp6", 64], ["ic12", 64], ["ic07", 128], ["ic08", 256], ["ic13", 256], ["ic09", 512], ["ic14", 512], ["ic10", 1024]];
const icnsEntries = [];
for (const [t, n] of icnsTypes) icnsEntries.push([t, (await icon(n)).png]);
writeFileSync(join(ICONS, "icon.icns"), icns(icnsEntries));
console.log(`${ICONS}: icon.ico (${icoSizes.join(", ")}), icon.icns, ${Object.keys(pngs).length} PNGs`);

// ── 2 · The mark in the app ─────────────────────────────────────────────────────────────────
rmSync(WEB, { recursive: true, force: true });
mkdirSync(WEB, { recursive: true });
let total = 0;
for (const [file, look] of [["mark.svg", "light"], ["mark-dark.svg", "dark"]]) {
  const text = svg(file);
  for (const size of MARK_SIZES) {
    const w = markWidth(size);
    // Units per CSS pixel: the note NOTE_OF_BOX of the box's height, centred in the box.
    const u = (NOTE.y1 - NOTE.y0) / (size * NOTE_OF_BOX);
    const cx = (NOTE.x0 + NOTE.x1) / 2;
    const cy = (NOTE.y0 + NOTE.y1) / 2;
    const vb = [cx - (w * u) / 2, cy - (size * u) / 2, w * u, size * u].map((v) => +v.toFixed(3)).join(" ");
    const cropped = text.replace(/viewBox="[^"]*"/, `viewBox="${vb}" preserveAspectRatio="none"`);
    for (const k of MARK_SCALES) {
      const out = await draw(cropped, Math.round(w * k), Math.round(size * k));
      writeFileSync(join(WEB, `mark-${look}-${size}@${k}x.webp`), out.webp);
      total += out.webp.length;
    }
  }
}
console.log(`${WEB}: ${readdirSync(WEB).length} pictures, ${(total / 1024).toFixed(1)} KB`);
await browser.close();
