// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The look of the board (Milestone 4): backgrounds with their own palettes, our styles and
// yours, watercolour mixes, themes (a Light style + a Dark style) and the dark-mode switch.
// Everything here is plain data and pure functions (tested in looks.test.ts); the reactive
// part lives in look.svelte.ts. Names here stay English (they are data and keys); what shows
// them translates (tMaybe).
import { t, tMaybe } from "./i18n.svelte";

export type BgKind =
  | "plain"
  | "paper"
  | "dotted"
  | "cork"
  | "kraft"
  | "terrazzo"
  | "watercolour"
  | "blueprint"
  | "felt"
  | "linen"
  | "plaster"
  | "picture";
export type Held = "auto" | "pins" | "tape" | "none";
export type Pattern = "Scattered" | "Sweep" | "Corners" | "One patch";
export const PATTERNS: Pattern[] = ["Scattered", "Sweep", "Corners", "One patch"];

/** A watercolour painting: 1 to 3 colours ("r,g,b"), a layout, a seed, white or dark paper. */
export interface WcLook {
  colours: string[];
  pattern: Pattern;
  seed: number;
  paper: "white" | "dark";
  /** The mix's name when it came from ours, More mixes or yours (shown under the front row). */
  name?: string;
}

/** One complete style of the board. */
export interface Look {
  bg: BgKind;
  /** A colour of the background's own palette, or "Custom". */
  colour: string;
  /** "#rrggbb" when the colour is Custom. */
  custom?: string;
  held: Held;
  wc?: WcLook;
  /** Your picture (a file in the app's folder). */
  picture?: string;
}

export type ThemeRef = { kind: "ours" | "yours"; id: string };
/** Where a style showing on the board comes from: one of our styles, one of yours, or a half of
 * a theme (e.g. the Paper theme's dark paper, which isn't one of the nine styles itself). */
export type Ref = ThemeRef | { kind: "theme"; id: string; half: "light" | "dark" };

export interface SavedStyle {
  id: string;
  name: string;
  look: Look;
  at: number;
}
export interface SavedTheme {
  id: string;
  name: string;
  light: Look;
  /** null: the board never changes with dark mode. */
  dark: Look | null;
  at: number;
}
export interface SavedMix {
  id: string;
  name: string;
  colours: string[];
  pattern: Pattern;
  paper: "white" | "dark";
  at: number;
}
export interface YourColour {
  /** "r,g,b" */
  rgb: string;
  at: number;
}
export type DarkSwitch = "windows" | "hours" | "light" | "dark";
export interface DarkMode {
  mode: DarkSwitch;
  /** Dark from … to … ("HH:MM"), used by "By hours". */
  from: string;
  until: string;
}
export const defaultDarkMode: DarkMode = { mode: "windows", from: "19:00", until: "07:00" };

/** What the board shows: the Light style, the Dark style (null: same as light), where each came from. */
export interface BoardLook {
  light: Look;
  dark: Look | null;
  lightFrom: Ref | null;
  darkFrom: Ref | null;
  /** The theme the pair came from. */
  theme: ThemeRef | null;
}

// ---- Backgrounds and their palettes ----

export interface Colour {
  name: string;
  hex: string;
  /** Terrazzo: its chips. */
  chips?: string[];
}
export interface Background {
  key: BgKind;
  name: string;
  /** Title of its own options' card under the tiles ("Cork · colours"). */
  colourLabel: string;
  palette: Colour[];
  /** Custom colours allowed (all but Watercolour and Picture). */
  custom: boolean;
  /** Held by, when Notes held by is Auto. */
  auto: "pins" | "tape" | "none";
}

const TERRAZZO_CLASSIC = ["#C9785A", "#9DB09A", "#3E4552", "#D9B26A"];

export const BACKGROUNDS: Background[] = [
  {
    key: "plain",
    name: "Plain",
    colourLabel: "Plain · colours",
    custom: true,
    auto: "none",
    palette: [
      { name: "Warm white", hex: "#F6F2EA" },
      { name: "Soft grey", hex: "#E7E6E2" },
      { name: "Sage", hex: "#DFE6DD" },
      { name: "Dusty blue", hex: "#DDE4EC" },
      { name: "Blush", hex: "#F2E5E0" },
      { name: "Charcoal", hex: "#2B2C30" },
    ],
  },
  {
    key: "paper",
    name: "Paper",
    colourLabel: "Paper · colours",
    custom: true,
    auto: "none",
    palette: [
      { name: "Warm ivory", hex: "#F2F0EB" },
      { name: "Cool grey", hex: "#ECEEF0" },
      { name: "Blush", hex: "#F4ECE8" },
      { name: "Mist green", hex: "#E9EEE7" },
      { name: "Night", hex: "#1C1B20" },
    ],
  },
  {
    key: "dotted",
    name: "Dotted",
    colourLabel: "Dotted · colours",
    custom: true,
    auto: "none",
    palette: [
      { name: "Ivory", hex: "#F6F4EF" },
      { name: "Bright white", hex: "#FCFCFA" },
      { name: "Pale grey", hex: "#E8E8E5" },
      { name: "Kraft dot", hex: "#E7D7BB" },
      { name: "Ink", hex: "#1E1E22" },
    ],
  },
  {
    key: "cork",
    name: "Cork",
    colourLabel: "Cork · colours",
    custom: true,
    auto: "pins",
    palette: [
      { name: "Natural", hex: "#C4955F" },
      { name: "Honey", hex: "#D4AA70" },
      { name: "Whitewashed", hex: "#DDCFBA" },
      { name: "Dark roast", hex: "#8A5C35" },
      { name: "Black cork", hex: "#2A2522" },
    ],
  },
  {
    key: "kraft",
    name: "Kraft",
    colourLabel: "Kraft · colours",
    custom: true,
    auto: "tape",
    palette: [
      { name: "Brown kraft", hex: "#C9A576" },
      { name: "Light kraft", hex: "#DCC39E" },
      { name: "Grey board", hex: "#BAB6AE" },
      { name: "Olive board", hex: "#A9A27F" },
      { name: "Black kraft", hex: "#262422" },
    ],
  },
  {
    key: "terrazzo",
    name: "Terrazzo",
    colourLabel: "Terrazzo · mixes",
    custom: true,
    auto: "tape",
    palette: [
      { name: "Classic", hex: "#EFE9E1", chips: TERRAZZO_CLASSIC },
      { name: "Blush", hex: "#F3E1DA", chips: ["#C9727A", "#7A2E3B", "#FFF6EE", "#E3A99A"] },
      { name: "Mint", hex: "#E1EEE6", chips: ["#5E8C6A", "#2F6E73", "#FFFFFF", "#A9CDB6"] },
      { name: "Night", hex: "#2A2A2E", chips: ["#D9B26A", "#F3EADB", "#C99AA0", "#8C8A86"] },
      { name: "Tone on tone", hex: "#F1ECE3", chips: ["#D9CDB8", "#C9B9A0", "#FFFFFF", "#E3D8C5"] },
    ],
  },
  { key: "watercolour", name: "Watercolour", colourLabel: "Watercolour · mixes", custom: false, auto: "none", palette: [] },
  {
    key: "blueprint",
    name: "Blueprint",
    colourLabel: "Blueprint · colours",
    custom: true,
    auto: "tape",
    palette: [
      { name: "Classic blue", hex: "#1F4E8C" },
      { name: "Navy", hex: "#1B2E52" },
      { name: "Teal", hex: "#1E5560" },
      { name: "Graphite", hex: "#2E3238" },
      { name: "White print", hex: "#F4F6F8" },
    ],
  },
  {
    key: "felt",
    name: "Felt",
    colourLabel: "Felt · colours",
    custom: true,
    auto: "pins",
    palette: [
      { name: "Forest", hex: "#22433A" },
      { name: "Navy", hex: "#22314A" },
      { name: "Burgundy", hex: "#4A2430" },
      { name: "Charcoal", hex: "#2C2C30" },
      { name: "Mustard", hex: "#7A5E22" },
    ],
  },
  {
    key: "linen",
    name: "Linen",
    colourLabel: "Linen · colours",
    custom: true,
    auto: "tape",
    palette: [
      { name: "Natural", hex: "#ECE6DA" },
      { name: "Oat", hex: "#E2D6BF" },
      { name: "Flax grey", hex: "#DCDBD5" },
      { name: "Indigo wash", hex: "#D3DCE6" },
      { name: "Charcoal", hex: "#2B2B2E" },
    ],
  },
  {
    key: "plaster",
    name: "Plaster",
    colourLabel: "Plaster · colours",
    custom: true,
    auto: "tape",
    palette: [
      { name: "Chalk white", hex: "#F3F0EA" },
      { name: "Bone", hex: "#EAE1D0" },
      { name: "Terracotta", hex: "#E3C3B1" },
      { name: "Sage", hex: "#D3DBCB" },
      { name: "Ochre", hex: "#E5D2A8" },
    ],
  },
  { key: "picture", name: "Your picture", colourLabel: "Your picture", custom: false, auto: "tape", palette: [] },
];

export const BG_KEYS = BACKGROUNDS.map((b) => b.key);
/** "Cork background" (what a change of background is called); a picture of yours is just
 * "Your picture". */
export function bgToast(key: BgKind): string {
  return key === "picture" ? t("Your picture") : t("{name} background", { name: tMaybe(background(key).name) });
}
export function background(key: BgKind): Background {
  return BACKGROUNDS.find((b) => b.key === key) ?? BACKGROUNDS[1];
}

// ---- Watercolour ----

/** The watercolour paints (tested as washes), Terracotta added for the new mixes. */
export const WC_PAINTS: [string, string][] = [
  ["Sky", "60,190,232"],
  ["Aqua", "110,205,235"],
  ["Ultramarine", "90,125,205"],
  ["Indigo", "80,95,160"],
  ["Lilac", "150,130,215"],
  ["Plum", "165,105,165"],
  ["Rose", "226,130,150"],
  ["Coral", "232,125,115"],
  ["Apricot", "240,165,105"],
  ["Ochre", "232,190,110"],
  ["Leaf", "185,200,110"],
  ["Sage", "110,170,120"],
  ["Teal", "60,175,190"],
  ["Slate", "120,135,155"],
  ["Sepia", "165,125,95"],
  ["Sand", "215,185,145"],
  ["Terracotta", "205,115,85"],
];
const paint = (n: string) => WC_PAINTS.find((p) => p[0] === n)![1];
export function paintName(rgb: string): string | null {
  return WC_PAINTS.find((p) => p[1] === rgb)?.[0] ?? null;
}

export interface Mix {
  name: string;
  colours: string[];
}
/** Our five (the front row, fixed). */
export const WC_OURS: Mix[] = [
  { name: "Sky & lilac", colours: ["60,190,232", "150,130,215", "110,205,235"] },
  { name: "Sunset", colours: ["226,130,140", "240,165,105", "232,190,110"] },
  { name: "Meadow", colours: ["110,170,120", "60,175,190", "185,200,110"] },
  { name: "Dusk", colours: ["150,130,215", "226,130,150", "90,125,205"] },
  { name: "Solid sky", colours: ["60,190,232"] },
];
const mix = (name: string, ...paints: string[]): Mix => ({ name, colours: paints.map(paint) });
const ours = (name: string) => WC_OURS.find((m) => m.name === name)!;
/** More mixes, by mood (14, ours included). */
export const WC_MOODS: { mood: string; mixes: Mix[] }[] = [
  { mood: "Calm & cool", mixes: [ours("Sky & lilac"), mix("Lagoon", "Teal", "Sage", "Sand"), mix("Sea glass", "Aqua", "Coral", "Slate"), mix("Harbour", "Ultramarine", "Apricot", "Sand")] },
  { mood: "Warm & sunny", mixes: [ours("Sunset"), mix("Teal sunset", "Teal", "Coral", "Ochre"), mix("Citrus grove", "Leaf", "Apricot", "Teal"), mix("Spice market", "Sepia", "Apricot", "Teal")] },
  { mood: "Garden", mixes: [ours("Meadow"), mix("Rose moss", "Sage", "Rose", "Sand"), mix("Fig garden", "Plum", "Ochre", "Sage")] },
  { mood: "Deep & cosy", mixes: [ours("Dusk"), mix("Indigo & rust", "Indigo", "Terracotta", "Ochre"), mix("Terracotta", "Terracotta", "Ochre", "Slate")] },
];
export const OUR_SEED = 18;

// ---- Our styles and themes ----

const look = (bg: BgKind, colour: string, extra: Partial<Look> = {}): Look => ({ bg, colour, held: "auto", ...extra });
const wcLook = (m: Mix, paper: "white" | "dark" = "white"): WcLook => ({ colours: m.colours, pattern: "Scattered", seed: OUR_SEED, paper, name: m.name });

export const OUR_STYLES: { id: string; name: string; look: Look }[] = [
  { id: "plain", name: "Plain", look: look("plain", "Sage") },
  { id: "paper-day", name: "Paper day", look: look("paper", "Warm ivory") },
  { id: "dotted-journal", name: "Dotted journal", look: look("dotted", "Ivory") },
  { id: "cork-board", name: "Cork board", look: look("cork", "Natural") },
  { id: "kraft-workshop", name: "Kraft workshop", look: look("kraft", "Brown kraft") },
  { id: "terrazzo", name: "Terrazzo", look: look("terrazzo", "Classic") },
  { id: "watercolour", name: "Watercolour", look: look("watercolour", "", { wc: wcLook(WC_OURS[0]) }) },
  { id: "blueprint", name: "Blueprint", look: look("blueprint", "Classic blue") },
  { id: "felt-night", name: "Felt night", look: look("felt", "Forest") },
];
export function ourStyle(id: string) {
  return OUR_STYLES.find((s) => s.id === id) ?? null;
}

export const OUR_THEMES: { id: string; name: string; light: Look; dark: Look }[] = [
  { id: "paper", name: "Paper", light: look("paper", "Warm ivory"), dark: look("paper", "Night") },
  { id: "paper-felt", name: "Paper & felt", light: look("paper", "Warm ivory"), dark: look("felt", "Forest") },
  { id: "watercolour", name: "Watercolour", light: look("watercolour", "", { wc: wcLook(WC_OURS[0]) }), dark: look("watercolour", "", { wc: wcLook(WC_OURS[0], "dark") }) },
  { id: "plain", name: "Plain", light: look("plain", "Warm white"), dark: look("plain", "Charcoal") },
  { id: "terrazzo", name: "Terrazzo", light: look("terrazzo", "Classic"), dark: look("terrazzo", "Night") },
  { id: "blueprint", name: "Blueprint", light: look("blueprint", "Classic blue"), dark: look("blueprint", "Navy") },
];
export function ourTheme(id: string) {
  return OUR_THEMES.find((t) => t.id === id) ?? null;
}

/** Today's look before Milestone 4: warm paper by day, a dark paper at night. */
export const DEFAULT_BOARD: BoardLook = {
  light: OUR_THEMES[0].light,
  dark: OUR_THEMES[0].dark,
  lightFrom: { kind: "ours", id: "paper-day" },
  darkFrom: { kind: "theme", id: "ours:paper", half: "dark" },
  theme: { kind: "ours", id: "paper" },
};

// ---- Colours ----

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
export function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, "0")).join("").toUpperCase();
}
export function rgbStrToHex(rgb: string): string {
  const [r, g, b] = rgb.split(",").map(Number);
  return rgbToHex(r, g, b);
}
/** Relative brightness 0–1 (perceived). */
export function luminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255;
}
export function isDarkHex(hex: string): boolean {
  return luminance(hex) < 0.45;
}

/** A custom pick softened toward a pigment for watercolour: saturation ≤ 0.62, lightness 0.52–0.72. */
export function soften(hex: string): [number, number, number] {
  const n = parseInt(hex.replace("#", ""), 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const mx = Math.max(r, g, b);
  const mn = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  let l = (mx + mn) / 2;
  const d = mx - mn;
  if (d) {
    s = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
    h = mx === r ? (g - b) / d + (g < b ? 6 : 0) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
    h /= 6;
  }
  s = Math.min(s, 0.62);
  l = Math.min(Math.max(l, 0.52), 0.72);
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const f = (t: number) => {
    t = (t + 1) % 1;
    return t < 1 / 6 ? p + (q - p) * 6 * t : t < 1 / 2 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p;
  };
  return [f(h + 1 / 3), f(h), f(h - 1 / 3)].map((v) => Math.round(v * 255)) as [number, number, number];
}

/** The base colour of a look (its palette colour or its custom one). */
export function baseHex(l: Look): string {
  if (l.bg === "watercolour") return l.wc?.paper === "dark" ? "#1B1C20" : "#FDFCF9";
  if (l.bg === "picture") return "#C9D2D6";
  if (l.colour === "Custom" && l.custom && /^#[0-9a-fA-F]{6}$/.test(l.custom)) return l.custom.toUpperCase();
  const bg = background(l.bg);
  return (bg.palette.find((c) => c.name === l.colour) ?? bg.palette[0]).hex;
}
export function colourOf(l: Look): Colour {
  const bg = background(l.bg);
  if (l.colour === "Custom") return { name: "Custom", hex: baseHex(l), chips: bg.key === "terrazzo" ? TERRAZZO_CLASSIC : undefined };
  return bg.palette.find((c) => c.name === l.colour) ?? bg.palette[0] ?? { name: "", hex: baseHex(l) };
}
/** The board reads dark (grain, shadows and fixings adapt). */
export function lookIsDark(l: Look): boolean {
  if (l.bg === "watercolour") return l.wc?.paper === "dark";
  if (l.bg === "picture") return false;
  return isDarkHex(baseHex(l));
}

/** The look the board shows for `l`: a picture background whose picture can't be shown (`usable`
 * false: outside Dobo's own folders, gone, never one imported) is a plain board in the mode's
 * own colour (`dark`), so dark mode never shows a light board. Anything else as it is. */
export function shownLook(l: Look, usable: boolean, dark: boolean): Look {
  if (l.bg !== "picture" || usable) return l;
  return { bg: "plain", colour: dark ? "Charcoal" : "Warm white", held: l.held };
}

/** What actually holds the notes: Auto follows the background. */
export function heldBy(l: Look): "pins" | "tape" | "none" {
  if (l.held !== "auto") return l.held;
  return background(l.bg).auto;
}

// ---- Comparing looks ("Changed from …") ----

function sameWc(a?: WcLook, b?: WcLook): boolean {
  if (!a || !b) return !a && !b;
  return a.pattern === b.pattern && a.seed === b.seed && a.paper === b.paper && a.colours.join("|") === b.colours.join("|");
}
export function sameLook(a: Look, b: Look): boolean {
  if (a.bg !== b.bg || a.held !== b.held) return false;
  if (a.bg === "watercolour") return sameWc(a.wc, b.wc);
  if (a.bg === "picture") return (a.picture ?? "") === (b.picture ?? "");
  if (a.colour !== b.colour) return false;
  return a.colour !== "Custom" || (a.custom ?? "").toUpperCase() === (b.custom ?? "").toUpperCase();
}

/** "colour Honey", "Felt background", "Sweep · repainted" — what differs from the base. */
export function describeChange(base: Look, now: Look): string {
  if (base.bg !== now.bg) {
    // (A picture of yours is "your picture", not a "Your picture background".)
    if (now.bg === "picture") return t("your picture");
    const bg = bgToast(now.bg);
    return now.bg === "watercolour" ? bg : `${bg} · ${now.colour === "Custom" ? t("custom colour") : tMaybe(now.colour)}`;
  }
  const parts: string[] = [];
  if (now.bg === "watercolour" && base.wc && now.wc) {
    if (base.wc.colours.join("|") !== now.wc.colours.join("|")) parts.push(now.wc.name ? tMaybe(now.wc.name) : t("your colours"));
    if (base.wc.pattern !== now.wc.pattern) parts.push(t(now.wc.pattern));
    if (base.wc.paper !== now.wc.paper) parts.push(now.wc.paper === "dark" ? t("dark paper") : t("white paper"));
    if (base.wc.seed !== now.wc.seed) parts.push(t("repainted"));
  } else if (now.bg === "picture") {
    if ((base.picture ?? "") !== (now.picture ?? "")) parts.push(t("your picture"));
  } else if (base.colour !== now.colour || (now.colour === "Custom" && base.custom !== now.custom)) {
    parts.push(now.colour === "Custom" ? t("custom colour") : t("colour {name}", { name: tMaybe(now.colour) }));
  }
  if (base.held !== now.held) parts.push(now.held === "auto" ? t("held as usual") : now.held === "none" ? t("no fixings") : t("held by {how}", { how: now.held === "pins" ? t("pins") : t("tape") }));
  return parts.join(" · ");
}

// ---- Names, ids, lists ----

export function newId(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.floor(Math.random() * 1e6).toString(36)}`;
}

/** A free name: "Name", else "Name (2)", "Name (3)"… */
export function uniqueName(name: string, taken: string[]): string {
  const base = name.trim() || t("Untitled");
  const used = new Set(taken.map((t) => t.trim().toLowerCase()));
  if (!used.has(base.toLowerCase())) return base;
  for (let i = 2; ; i++) {
    const n = `${base} (${i})`;
    if (!used.has(n.toLowerCase())) return n;
  }
}

/** Your colours: newest first, no duplicates; past 48 the oldest one no saved style uses makes room
 * (a gentle cap: when every one is in use, the list may grow). */
export const YOUR_COLOURS_CAP = 48;
export function addYourColour(list: YourColour[], rgb: string, inUse: Set<string>, now = Date.now()): YourColour[] {
  const next = [{ rgb, at: now }, ...list.filter((c) => c.rgb !== rgb)];
  while (next.length > YOUR_COLOURS_CAP) {
    let i = -1;
    for (let k = next.length - 1; k > 0; k--)
      if (!inUse.has(next[k].rgb)) {
        i = k;
        break;
      }
    if (i < 0) break;
    next.splice(i, 1);
  }
  return next;
}

/** Colours used by saved things (they never make room). */
export function coloursInUse(styles: SavedStyle[], themes: SavedTheme[], mixes: SavedMix[], board: BoardLook): Set<string> {
  const s = new Set<string>();
  const add = (l: Look | null) => l?.wc?.colours.forEach((c) => s.add(c));
  styles.forEach((x) => add(x.look));
  themes.forEach((t) => (add(t.light), add(t.dark)));
  mixes.forEach((m) => m.colours.forEach((c) => s.add(c)));
  add(board.light);
  add(board.dark);
  return s;
}

// ---- Dark mode by hours ----

/** "HH:MM" inside [from, until) — the window may wrap past midnight (19:00–07:00). */
export function inHours(hhmm: string, from: string, until: string): boolean {
  if (from === until) return false;
  return from < until ? hhmm >= from && hhmm < until : hhmm >= from || hhmm < until;
}
export function hhmmOf(d: Date): string {
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
/** Dark now? null: follow Windows. */
export function darkBySwitch(m: DarkMode, now: Date): boolean | null {
  if (m.mode === "light") return false;
  if (m.mode === "dark") return true;
  if (m.mode === "hours") return inHours(hhmmOf(now), m.from, m.until);
  return null;
}

// ---- Reading settings safely (old or damaged values fall back) ----

const HELD: Held[] = ["auto", "pins", "tape", "none"];
const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
export const RGB_RE = /^(25[0-5]|2[0-4]\d|1?\d?\d),(25[0-5]|2[0-4]\d|1?\d?\d),(25[0-5]|2[0-4]\d|1?\d?\d)$/;
export const HEX_RE = /^#[0-9a-fA-F]{6}$/;
const HHMM_RE = /^([01]\d|2[0-3]):[0-5]\d$/;

/** A look from untrusted data (settings or an imported file); null when it isn't one. */
export function cleanLook(v: unknown, keepPicture = true): Look | null {
  if (!isObj(v)) return null;
  const bg = v.bg as BgKind;
  if (!BG_KEYS.includes(bg)) return null;
  const held = HELD.includes(v.held as Held) ? (v.held as Held) : "auto";
  const out: Look = { bg, colour: "", held };
  const b = background(bg);
  if (bg === "watercolour") {
    const w = isObj(v.wc) ? v.wc : {};
    const colours = Array.isArray(w.colours) ? w.colours.filter((c): c is string => typeof c === "string" && RGB_RE.test(c)).slice(0, 3) : [];
    out.wc = {
      colours: colours.length ? colours : WC_OURS[0].colours,
      pattern: PATTERNS.includes(w.pattern as Pattern) ? (w.pattern as Pattern) : "Scattered",
      seed: Number.isInteger(w.seed) && (w.seed as number) > 0 && (w.seed as number) < 2147483647 ? (w.seed as number) : OUR_SEED,
      paper: w.paper === "dark" ? "dark" : "white",
    };
    if (typeof w.name === "string" && w.name.length <= 60) out.wc.name = w.name;
  } else if (bg === "picture") {
    if (keepPicture && typeof v.picture === "string" && v.picture.length < 1000) out.picture = v.picture;
  } else if (v.colour === "Custom" && typeof v.custom === "string" && HEX_RE.test(v.custom)) {
    out.colour = "Custom";
    out.custom = v.custom.toUpperCase();
  } else {
    out.colour = b.palette.some((c) => c.name === v.colour) ? (v.colour as string) : b.palette[0].name;
  }
  return out;
}
function cleanThemeRef(v: unknown): ThemeRef | null {
  if (!isObj(v) || (v.kind !== "ours" && v.kind !== "yours") || typeof v.id !== "string") return null;
  return { kind: v.kind, id: v.id };
}
function cleanRef(v: unknown): Ref | null {
  if (isObj(v) && v.kind === "theme" && typeof v.id === "string" && (v.half === "light" || v.half === "dark")) return { kind: "theme", id: v.id, half: v.half };
  return cleanThemeRef(v);
}
export function cleanName(v: unknown): string | null {
  if (typeof v !== "string") return null;
  // eslint-disable-next-line no-control-regex
  const t = v.replace(/[\u0000-\u001f\u007f]/g, "").trim();
  return t ? t.slice(0, 60) : null;
}
/** A file right inside a folder called "backgrounds" (where every picture picked is copied:
 * look.rs `pick_background_picture`, which checks it is the data folder's own). */
const IN_BACKGROUNDS = /[\\/]backgrounds[\\/](?!\.+$)[^\\/]+$/i;
/** Your last picture (the picture tile's, setting `look_last_picture` = `{ picture }`): kept
 * when you switch to another background or Undo, and over a restart. Read with care: only a
 * picture in the backgrounds folder (look.rs `in_backgrounds`), or the preview's own (blob:). */
export function readLastPicture(v: unknown): string | null {
  if (!isObj(v) || typeof v.picture !== "string" || !v.picture || v.picture.length >= 1000) return null;
  return v.picture.startsWith("blob:") || IN_BACKGROUNDS.test(v.picture) ? v.picture : null;
}
export function readBoard(v: unknown): BoardLook {
  if (!isObj(v)) return DEFAULT_BOARD;
  const light = cleanLook(v.light);
  if (!light) return DEFAULT_BOARD;
  return { light, dark: cleanLook(v.dark), lightFrom: cleanRef(v.lightFrom), darkFrom: cleanRef(v.darkFrom), theme: cleanThemeRef(v.theme) };
}
export function readStyles(v: unknown, keepPicture = true): SavedStyle[] {
  if (!Array.isArray(v)) return [];
  const out: SavedStyle[] = [];
  for (const x of v.slice(0, 500)) {
    if (!isObj(x)) continue;
    const look = cleanLook(x.look, keepPicture);
    const name = cleanName(x.name);
    if (look && name) out.push({ id: typeof x.id === "string" && x.id.length <= 80 ? x.id : newId("s"), name, look, at: Number(x.at) || 0 });
  }
  return out;
}
export function readThemes(v: unknown, keepPicture = true): SavedTheme[] {
  if (!Array.isArray(v)) return [];
  const out: SavedTheme[] = [];
  for (const x of v.slice(0, 200)) {
    if (!isObj(x)) continue;
    const light = cleanLook(x.light, keepPicture);
    const name = cleanName(x.name);
    if (light && name) out.push({ id: typeof x.id === "string" && x.id.length <= 80 ? x.id : newId("t"), name, light, dark: cleanLook(x.dark, keepPicture), at: Number(x.at) || 0 });
  }
  return out;
}
export function readMixes(v: unknown): SavedMix[] {
  if (!Array.isArray(v)) return [];
  const out: SavedMix[] = [];
  for (const x of v.slice(0, 200)) {
    if (!isObj(x)) continue;
    const name = cleanName(x.name);
    const colours = Array.isArray(x.colours) ? x.colours.filter((c): c is string => typeof c === "string" && RGB_RE.test(c)).slice(0, 3) : [];
    if (!name || !colours.length) continue;
    out.push({
      id: typeof x.id === "string" && x.id.length <= 80 ? x.id : newId("m"),
      name,
      colours,
      pattern: PATTERNS.includes(x.pattern as Pattern) ? (x.pattern as Pattern) : "Scattered",
      paper: x.paper === "dark" ? "dark" : "white",
      at: Number(x.at) || 0,
    });
  }
  return out;
}
export function readColours(v: unknown): YourColour[] {
  if (!Array.isArray(v)) return [];
  const seen = new Set<string>();
  const out: YourColour[] = [];
  for (const x of v.slice(0, 500)) {
    if (!isObj(x) || typeof x.rgb !== "string" || !RGB_RE.test(x.rgb) || seen.has(x.rgb)) continue;
    seen.add(x.rgb);
    out.push({ rgb: x.rgb, at: Number(x.at) || 0 });
  }
  return out;
}
export function readDarkMode(v: unknown): DarkMode {
  if (!isObj(v)) return defaultDarkMode;
  return {
    mode: (["windows", "hours", "light", "dark"] as DarkSwitch[]).includes(v.mode as DarkSwitch) ? (v.mode as DarkSwitch) : "windows",
    from: typeof v.from === "string" && HHMM_RE.test(v.from) ? v.from : defaultDarkMode.from,
    until: typeof v.until === "string" && HHMM_RE.test(v.until) ? v.until : defaultDarkMode.until,
  };
}

/** A mix's colours as a readable list ("Teal · Coral · Ochre"). */
export function mixLabel(colours: string[]): string {
  return colours.map((c) => paintName(c) ?? rgbStrToHex(c)).join(" · ");
}

// ---- What a style on the board is compared with ("Changed from", Reset, the highlighted card) ----

export const themeKey = (t: ThemeRef) => `${t.kind}:${t.id}`;

/** A theme's pair, ours or yours. */
export function themePairOf(t: ThemeRef | null, themes: SavedTheme[]): { name: string; light: Look; dark: Look | null } | null {
  if (!t) return null;
  if (t.kind === "ours") return OUR_THEMES.find((x) => x.id === t.id) ?? null;
  return themes.find((x) => x.id === t.id) ?? null;
}

/** The look a ref stands for (null when it is gone). */
export function resolveRef(ref: Ref | null, styles: SavedStyle[], themes: SavedTheme[]): Look | null {
  if (!ref) return null;
  if (ref.kind !== "theme") return ref.kind === "ours" ? (ourStyle(ref.id)?.look ?? null) : (styles.find((s) => s.id === ref.id)?.look ?? null);
  const [kind, ...rest] = ref.id.split(":");
  const pair = themePairOf({ kind: kind === "yours" ? "yours" : "ours", id: rest.join(":") }, themes);
  if (!pair) return null;
  return ref.half === "dark" ? (pair.dark ?? pair.light) : pair.light;
}

/** A theme of yours that is exactly one of ours (an import of ours, untouched): it shows under
 * our name, in your language. Anything else keeps the name it was given. */
export function themeShownName(th: { name: string; light: Look; dark: Look | null }): string {
  const o = OUR_THEMES.find((x) => x.name === th.name);
  const same = !!o && sameLook(o.light, th.light) && !!th.dark && sameLook(o.dark, th.dark);
  return same ? tMaybe(th.name) : th.name;
}

/** Its name: the style's, or "Paper · Dark" for a theme's half. */
export function refName(ref: Ref | null, styles: SavedStyle[], themes: SavedTheme[]): string | null {
  const name = refBaseName(ref, styles, themes);
  if (name == null || ref?.kind !== "theme") return name;
  return ref.half === "dark" ? t("{name} · Dark", { name }) : t("{name} · Light", { name });
}

/** The same without the half a theme's look is (where the half shows already: a sun or a moon). */
export function refBaseName(ref: Ref | null, styles: SavedStyle[], themes: SavedTheme[]): string | null {
  if (!ref) return null;
  if (ref.kind !== "theme") {
    if (ref.kind === "ours") {
      const s = ourStyle(ref.id);
      return s ? tMaybe(s.name) : null;
    }
    return styles.find((s) => s.id === ref.id)?.name ?? null;
  }
  const [kind, ...rest] = ref.id.split(":");
  const pair = themePairOf({ kind: kind === "yours" ? "yours" : "ours", id: rest.join(":") }, themes);
  if (!pair) return null;
  return kind === "yours" ? themeShownName(pair) : tMaybe(pair.name);
}

/** Where a theme's half comes from: a style that is exactly it (ours, then yours), else that
 * half of the theme itself — so an untouched board never reads as "changed". */
export function refFor(l: Look, half: "light" | "dark", theme: ThemeRef | null, styles: SavedStyle[]): Ref | null {
  const ours = OUR_STYLES.find((s) => sameLook(s.look, l));
  if (ours) return { kind: "ours", id: ours.id };
  const yours = styles.find((s) => sameLook(s.look, l));
  if (yours) return { kind: "yours", id: yours.id };
  if (theme) return { kind: "theme", id: themeKey(theme), half };
  const near = OUR_STYLES.find((s) => s.look.bg === l.bg);
  return near ? { kind: "ours", id: near.id } : null;
}
