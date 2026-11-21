// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo Setup's pictures (Sookera installer template 1.0.3). From Dobo's repo root:
//   node art/installer-art.mjs art/dobo.config.mjs src-tauri/installer/art
// (needs Playwright's Chromium: npx playwright install chromium). APP_SLIDES in
// src-tauri/installer/app.nsh must equal the number of slides.
//
// The pictures in art/shots are the real Dobo UI (the dev server's browser preview, a sample
// board: no personal data), captured headless at twice the size, each in the dark and the light
// look (<name>.png, <name>-light.png).
import { readFileSync } from "node:fs";
import { join } from "node:path";

const shots = join("art", "shots");
const shot = (dataUrl, name, k) => dataUrl(join(shots, `${name}${k === "l" ? "-light" : ""}.png`));

// Dobo's ink and paper (the logo's), as the app swaps them in dark mode.
const INK = "#1f1d1a";
const PAPER = "#fffcf4";

export default {
  name: "Dobo",
  // Ink on the light look; on the dark look the ink would vanish, so paper (with ink on it).
  // Same as APP_ACCENT / APP_ACCENT_DARK in app.nsh.
  accent: INK,
  accentHover: "#3a3631",
  onAccent: PAPER,
  accentDark: PAPER,
  accentDarkHover: "#ece6d8",
  onAccentDark: INK,
  slides: [
    {
      title: "Tasks and ideas on one calm board",
      desc: "Type a line, press Enter for a task or Shift+Enter for an idea. Each note keeps its own, in its own colour.",
      main: (k, d) => ({ src: shot(d, "board", k) }),
    },
    {
      title: "Dump it now, sort it later",
      desc: "Ctrl+Shift+D drops a thought into the Inbox from any app. Drag its lines onto your notes, or let Tidy sort them.",
      main: (k, d) => ({ src: shot(d, "inbox", k) }),
    },
    {
      title: "Give a task a time",
      desc: "Drag a task onto the hours: Dobo reminds you then. End a line with ! for a deadline that rings early.",
      main: (k, d) => ({ src: shot(d, "ruler", k) }),
    },
    {
      title: "Big tasks, small steps",
      desc: "Drop a line onto a task to make it one of its steps. The ring round its box fills as you tick them.",
      main: (k, d) => ({ src: shot(d, "steps", k) }),
    },
    {
      title: "Make it yours",
      desc: "Paper, watercolour, your own picture: pick a style, and a theme that follows dark mode.",
      main: (k, d) => ({ src: shot(d, "style", k) }),
    },
  ],
  // The mark and the wordmark from src/components/Logo.svelte (mark 44 high, word 28, 12 apart).
  logo: (k) => {
    const logo = readFileSync(join("src", "components", "Logo.svelte"), "utf8");
    const svgs = [...logo.matchAll(/<svg[\s\S]*?<\/svg>/g)].map((m) => m[0]);
    const ink = k === "d" ? PAPER : INK;
    const clean = (s) => s.replace(/\s(class|aria-[a-z]+)="[^"]*"/g, "");
    // The note's edge: ink on light grounds, paper on dark ones (as in the app).
    const mark = clean(svgs[0].replace('class="edge"', `fill="${ink}"`)).replace("<svg", '<svg width="44" height="44"');
    const word = clean(svgs[1]).replace("<svg", `<svg height="28" style="width:70px;fill:${ink}"`);
    return `${mark}${word}`;
  },
};
