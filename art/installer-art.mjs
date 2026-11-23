// The installer's pictures for any app: the feature slides on the left of the window, the logo,
// the switches, the folder box and the buttons, each in a dark and a light version (Setup follows
// Windows). Drawn with Playwright's Chromium at twice the size; Setup scales them to the screen
// (nsis/ui.nsh: AppImage). Made from Pako's scripts/installer-art.mjs (Pako 1.3.0).
//
//   node installer-art.mjs <app config .mjs> <output folder>
//   e.g. node art/installer-art.mjs art/pako.config.mjs ../Pako/src-tauri/installer/art
//
// Needs `playwright` (npm i -D playwright; npx playwright install chromium). Run it again after a
// picture used by a slide changes, or the wording. Slides are JPEG (small enough for Setup), the
// rest PNG. Every picture is drawn on its own opaque background colour: Windows' picture controls
// don't blend, and 32-bit pictures make Windows keep a copy each time one is swapped (a GDI leak),
// so nothing is transparent. The number of slides must match APP_SLIDES in app.nsh.
//
// French (template 1.1.0, for an app with APP_FRENCH 1): give every slide `fr: { title, desc }`
// and the script also makes the French set: slideN-d-fr.jpg / slideN-l-fr.jpg ("1 sur 5") and the
// buttons with their French words, b-<name>-<state>-<look>-fr.png. Setup shows them when French is
// picked. Write the French with its typography (a no-break space before : and inside « », a
// narrow one before ; ! ?, ’ for the apostrophe), as in nsis/French.nsh.
import { chromium } from "playwright";
import { mkdirSync, readFileSync, writeFileSync, statSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [cfgPath, outArg] = process.argv.slice(2);
if (!cfgPath || !outArg) {
  console.error("Usage: node installer-art.mjs <app config .mjs> <output folder>");
  process.exit(1);
}
const cfg = (await import(pathToFileURL(resolve(cfgPath)).href)).default;
const out = resolve(outArg);
mkdirSync(out, { recursive: true });

// Must match nsis/ui.nsh (colours, sizes, the slide's click areas). An app may override a look's
// colours in its config (cfg.looks.d / cfg.looks.l); the accent comes from the config.
const looks = {
  d: { bg: "#14171b", pane: "#0f1114", text: "#e9eaee", mut: "#8b919c", ln: "#262a31", off: "#343a44", ra: "#1b1f24", btn: "#20242a", btnLine: "#3a404a", btnHover: "#272c33", shadow: "rgba(0,0,0,.5)", ...(cfg.looks?.d ?? {}) },
  l: { bg: "#ffffff", pane: "#f6f7f9", text: "#17181c", mut: "#6b7280", ln: "#e2e7ee", off: "#cdd4de", ra: "#f3f5f8", btn: "#ffffff", btnLine: "#828b9b", btnHover: "#f1f3f6", shadow: "rgba(23,24,28,.18)", ...(cfg.looks?.l ?? {}) },
};
// An accent per look (template 1.1.1): cfg.accentDark (and accentDarkHover) for the dark one when
// the app's colour is too dark to read there (Dobo: ink on paper, paper on dark), with the words
// and the switch's knob on it in cfg.onAccent / cfg.onAccentDark (white by default). Same as
// APP_ACCENT_DARK in app.nsh.
const accentOf = (k) => (k === "d" && cfg.accentDark ? cfg.accentDark : cfg.accent);
const hoverOf = (k) => (k === "d" && cfg.accentDark ? (cfg.accentDarkHover ?? cfg.accentDark) : (cfg.accentHover ?? cfg.accent));
const onAccentOf = (k) => (k === "d" ? (cfg.onAccentDark ?? "#ffffff") : (cfg.onAccent ?? "#ffffff"));
const font = `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`;
const fontDisplay = `"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif`;

// Pictures go in as data: addresses (a page set from a string can't open files).
export const dataUrl = (file) =>
  `data:${file.endsWith(".svg") ? "image/svg+xml" : file.endsWith(".jpg") ? "image/jpeg" : "image/png"};base64,${readFileSync(file).toString("base64")}`;

const slides = cfg.slides; // [{ title, desc, fr?: { title, desc }, main: (k, dataUrl, lang) => ({ src, fit? }), card?: (k, dataUrl, lang) => ({ src, side, width }) }]
// French: all the slides or none (Setup would show a slide without words otherwise).
const withFr = slides.filter((s) => s.fr);
if (withFr.length && withFr.length < slides.length) {
  const missing = slides.map((s, i) => (s.fr ? null : i + 1)).filter(Boolean);
  console.error(`Slides ${missing.join(", ")} have no fr: { title, desc } (every slide needs one, or none).`);
  process.exit(1);
}
const french = withFr.length > 0;

// The left panel: 530 × 540. Dots start at x 28 (7 wide, the current one 20, 6 apart), centre
// y 514; the arrows are 26 × 26 boxes at x 448 and 476, y 501.
function slideHtml(i, k, lang = "en") {
  const c = looks[k];
  const s = slides[i];
  const words = lang === "fr" ? s.fr : s;
  // The slide's language too ("en" / "fr"), so French slides can show the app in French (Dobo:
  // art/shots-fr). A config whose main / card don't take it works as before.
  const m = s.main(k, dataUrl, lang);
  const card = s.card?.(k, dataUrl, lang);
  const dots = slides
    .map((_, j) => `<span style="width:${j === i ? 20 : 7}px;background:${j === i ? accentOf(k) : c.off}"></span>`)
    .join("");
  const chevron = (d) =>
    `<svg width="26" height="26" viewBox="0 0 26 26"><path d="${d}" fill="none" stroke="${c.mut}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  return `<!doctype html><html lang="${lang}"><head><meta charset="utf-8"><style>
    html,body{margin:0;width:530px;height:540px;overflow:hidden;background:${c.pane};font-family:${font};-webkit-font-smoothing:antialiased}
    .k{position:absolute;left:28px;top:28px;font-size:12px;color:${accentOf(k)};letter-spacing:.02em;margin:0}
    h2{position:absolute;left:28px;top:46px;right:28px;margin:0;font:600 20px/26px ${fontDisplay};color:${c.text}}
    .d{position:absolute;left:28px;top:78px;right:28px;margin:0;font-size:13px;line-height:20px;color:${c.mut}}
    .shot{position:absolute;left:28px;right:28px;top:130px;bottom:52px;border-radius:10px;overflow:hidden;border:1px solid ${c.ln};background:${c.bg}}
    .shot>img.main{width:100%;height:100%;display:block;object-fit:${m.fit ?? "cover"};object-position:${m.fit ? "center" : "top left"}}
    .card{position:absolute;bottom:14px;${card?.side === "right" ? "right" : "left"}:14px;width:${card?.width ?? 40}%;border-radius:8px;border:1px solid ${c.btnLine};box-shadow:0 10px 26px ${c.shadow};display:block}
    .dots{position:absolute;left:28px;top:511px;display:flex;gap:6px}
    .dots span{height:7px;border-radius:4px;display:block}
    .nav{position:absolute;top:501px;left:448px;display:flex;gap:2px}
  </style></head><body>
    <p class="k">${i + 1} ${lang === "fr" ? "sur" : "of"} ${slides.length}</p>
    <h2>${words.title}</h2>
    <p class="d">${words.desc}</p>
    <div class="shot"><img class="main" src="${m.src}">${card ? `<img class="card" src="${card.src}">` : ""}</div>
    <div class="dots">${dots}</div>
    <div class="nav">${chevron("M15 8 L10 13 L15 18")}${chevron("M11 8 L16 13 L11 18")}</div>
  </body></html>`;
}

// The logo at the top of the right side: 160 × 44, on the window's background. The config gives
// the inner HTML (an <img>, or inline SVG) for each look.
const logoHtml = (k) =>
  `<!doctype html><html><head><style>html,body{margin:0;background:${looks[k].bg}}
    div{display:flex;align-items:center;gap:12px;width:160px;height:44px;justify-content:center}
  </style></head><body><div>${cfg.logo(k, { ...looks[k], accent: accentOf(k) }, dataUrl)}</div></body></html>`;

const sw = (k, on) => {
  const c = looks[k];
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    i{display:block;position:relative;width:32px;height:18px;border-radius:9px;background:${on ? accentOf(k) : c.off}}
    i:after{content:"";position:absolute;top:2px;${on ? "right" : "left"}:2px;width:14px;height:14px;border-radius:50%;background:${on ? onAccentOf(k) : "#fff"};box-shadow:0 1px 2px rgba(0,0,0,.2)}
  </style></head><body><i></i></body></html>`;
};

const field = (k) => {
  const c = looks[k];
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    div{width:282px;height:32px;border-radius:7px;background:${c.ra};display:flex;align-items:center;padding-left:10px;box-sizing:border-box}
  </style></head><body><div><svg width="15" height="15" viewBox="0 0 16 16"><path d="M1.5 4.5 V12 a1 1 0 0 0 1 1 H13.5 a1 1 0 0 0 1-1 V6 a1 1 0 0 0-1-1 H8 L6.5 3 H2.5 a1 1 0 0 0-1 1 Z" fill="none" stroke="${c.mut}" stroke-width="1.3" stroke-linejoin="round"/></svg></div></body></html>`;
};

// kind: "main" (the accent) or "second"; state: n(ormal), h(over), x (can't be clicked yet).
const button = (k, label, width, kind, state) => {
  const c = looks[k];
  const main = kind === "main";
  const bg = main ? (state === "h" ? hoverOf(k) : accentOf(k)) : state === "h" ? c.btnHover : c.btn;
  const edge = main ? bg : c.btnLine;
  const color = main ? onAccentOf(k) : c.text;
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    div{width:${width}px;height:40px;box-sizing:border-box;border-radius:8px;background:${bg};border:1px solid ${edge};color:${color};
      display:flex;align-items:center;justify-content:center;font:600 14px ${font};opacity:${state === "x" ? 0.45 : 1}}
  </style></head><body><div>${label}</div></body></html>`;
};

// name, label, width (logical px), kind. The installer's buttons are one wide button; the
// uninstaller's two sit side by side. The names are what nsis/ui.nsh loads.
const buttons = [
  ["install", "Install", 282, "main"],
  ["update", "Update", 282, "main"],
  ["again", "Install again", 282, "main"],
  ["older", "Install this version", 282, "main"],
  ["open", `Open ${cfg.name}`, 282, "main"],
  ["done", "Close", 282, "second"],
  ["uninstall", "Uninstall", 200, "main"],
  ["cancel", "Cancel", 200, "second"],
  ["close", "Close", 412, "second"],
];
// Their French words (the same names, pictures ending in -fr).
const buttonsFr = {
  install: "Installer",
  update: "Mettre à jour",
  again: "Réinstaller",
  older: "Installer cette version",
  open: `Ouvrir ${cfg.name}`,
  done: "Fermer",
  uninstall: "Désinstaller",
  cancel: "Annuler",
  close: "Fermer",
};

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 2 });
const written = [];
async function render(name, html, w, h, type = "png") {
  await page.setViewportSize({ width: w, height: h });
  await page.setContent(html, { waitUntil: "load" });
  await page.evaluate(() => document.fonts.ready);
  const file = join(out, `${name}.${type === "jpeg" ? "jpg" : "png"}`);
  // omitBackground stays off: every picture is opaque (see the top).
  await page.screenshot({ path: file, type, ...(type === "jpeg" ? { quality: 86 } : {}), clip: { x: 0, y: 0, width: w, height: h } });
  written.push(file);
}

for (const k of ["d", "l"]) {
  for (let i = 0; i < slides.length; i++) await render(`slide${i + 1}-${k}`, slideHtml(i, k), 530, 540, "jpeg");
  if (french) for (let i = 0; i < slides.length; i++) await render(`slide${i + 1}-${k}-fr`, slideHtml(i, k, "fr"), 530, 540, "jpeg");
  await render(`logo-${k}`, logoHtml(k), 160, 44);
  await render(`on-${k}`, sw(k, true), 32, 18);
  await render(`off-${k}`, sw(k, false), 32, 18);
  await render(`field-${k}`, field(k), 282, 32);
  for (const [name, label, width, kind] of buttons) {
    for (const st of kind === "main" ? ["n", "h", "x"] : ["n", "h"]) {
      await render(`b-${name}-${st}-${k}`, button(k, label, width, kind, st), width, 40);
      if (french) await render(`b-${name}-${st}-${k}-fr`, button(k, buttonsFr[name], width, kind, st), width, 40);
    }
  }
}
await browser.close();

// Old pictures this script no longer makes would still be built into Setup.
const keep = new Set([...written.map((f) => f.split(/[\\/]/).pop()), "README.md"]);
for (const f of readdirSync(out)) if (!keep.has(f)) console.warn(`not made any more (delete it): ${f}`);
const total = written.reduce((n, f) => n + statSync(f).size, 0);
console.log(`${written.length} pictures, ${(total / 1024).toFixed(0)} KB in ${out} (${slides.length} slides: APP_SLIDES must be ${slides.length}${french ? "; French too: APP_FRENCH 1" : ""})`);
writeFileSync(join(out, "README.md"), "Made by the installer template's art/installer-art.mjs: don't edit these by hand.\n");
