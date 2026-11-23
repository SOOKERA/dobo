// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Dobo Setup's pictures (Sookera installer template 1.1.3). From Dobo's repo root:
//   node art/installer-art.mjs art/dobo.config.mjs src-tauri/installer/art
// (needs Playwright's Chromium: npx playwright install chromium). APP_SLIDES in
// src-tauri/installer/app.nsh must equal the number of slides.
//
// The pictures in art/shots are the real Dobo UI (the dev server's browser preview, a sample
// board: no personal data), captured headless at twice the size, each in the dark and the light
// look (<name>.png, <name>-light.png), with Dobo in English. art/shots-fr has the same pictures
// with Dobo in French (the same board, its notes written in French), for the French slides
// (`fr`, template 1.1.0; installer-art.mjs hands main() the slide's language, template 1.1.2).
import { readFileSync } from "node:fs";
import { join } from "node:path";

const shot = (dataUrl, name, k, lang) => dataUrl(join("art", lang === "fr" ? "shots-fr" : "shots", `${name}${k === "l" ? "-light" : ""}.png`));

// French typography as the app's own t() adds it (src/lib/i18n): a no-break space before : and »
// and after «, a narrow one before ; ! ?. Write plain spaces in the French below.
const fr = (title, desc) => {
  const typo = (t) => t.replace(/ ([:»])/g, "\u00a0$1").replace(/« /g, "«\u00a0").replace(/ ([;!?])/g, "\u202f$1");
  return { title: typo(title), desc: typo(desc) };
};

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
      fr: fr("Vos tâches et vos idées, au calme", "Écrivez une ligne, puis Entrée pour une tâche ou Maj+Entrée pour une idée. Chaque note garde les siennes, dans sa couleur."),
      main: (k, d, lang) => ({ src: shot(d, "board", k, lang) }),
    },
    {
      title: "Dump it now, sort it later",
      desc: "Ctrl+Shift+D drops a thought into the Inbox from any app. Drag its lines onto your notes, or let Tidy sort them.",
      fr: fr("Notez tout de suite, rangez plus tard", "Ctrl+Maj+D dépose une pensée dans « À trier », depuis n’importe quelle appli. Glissez ses lignes sur vos notes, ou laissez Trier les ranger."),
      main: (k, d, lang) => ({ src: shot(d, "inbox", k, lang) }),
    },
    {
      title: "Give a task a time",
      desc: "Drag a task onto the hours: Dobo reminds you then. End a line with ! for a deadline that rings early.",
      fr: fr("Donnez une heure à une tâche", "Glissez une tâche sur les heures : Dobo vous la rappelle à ce moment-là. Finissez une ligne par «\u00a0!\u00a0» pour une échéance qui sonne en avance."),
      main: (k, d, lang) => ({ src: shot(d, "ruler", k, lang) }),
    },
    {
      title: "Big tasks, small steps",
      desc: "Drop a line onto a task to make it one of its steps. The ring round its box fills as you tick them.",
      fr: fr("Les grandes tâches, pas à pas", "Déposez une ligne sur une tâche pour en faire une de ses étapes. L’anneau autour de sa case se remplit à mesure que vous les cochez."),
      main: (k, d, lang) => ({ src: shot(d, "steps", k, lang) }),
    },
    {
      title: "Make it yours",
      desc: "Paper, watercolour, your own picture: pick a style, and a theme that follows dark mode.",
      fr: fr("À votre image", "Papier, aquarelle, votre propre photo : choisissez un style, et un thème qui suit le mode sombre."),
      main: (k, d, lang) => ({ src: shot(d, "style", k, lang) }),
    },
  ],
  // The true logo (the textured note, without the app icon's tile: art/logo/mark*.svg, from the
  // final logo) beside the wordmark from src/components/Logo.svelte (mark 46 high, word 28). The
  // owner's choice (2026-11-23): the true logo everywhere (Setup, the app's header, the taskbar)
  // but the tray, which keeps the compact note drawn for small sizes.
  logo: (k) => {
    const logo = readFileSync(join("src", "components", "Logo.svelte"), "utf8");
    const wordSvg = logo.match(/<svg class="word"[\s\S]*?<\/svg>/)[0];
    const ink = k === "d" ? PAPER : INK;
    const clean = (s) => s.replace(/\s(class|aria-[a-z]+)="[^"]*"/g, "");
    const file = readFileSync(join("art", "logo", k === "d" ? "mark-dark.svg" : "mark.svg"), "utf8");
    const mark = file
      .replace(/<\?xml[^>]*>/, "")
      .replace(/<title>[\s\S]*?<\/title>/, "")
      .replace("<svg", '<svg width="46" height="46"');
    const word = clean(wordSvg).replace("<svg", `<svg height="28" style="width:${(28 * 290) / 116}px;fill:${ink}"`);
    return `${mark}${word}`;
  },
};
