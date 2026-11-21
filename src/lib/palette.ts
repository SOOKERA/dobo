// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The three note palettes. Each colour has a light and a dark version of:
// bg (the ideas area tint), ink (idea text on it), corner (the folded corner), dot (title dot).
import type { Note, PaletteName } from "./types";

export interface Tone {
  bg: string;
  ink: string;
  corner: string;
  dot: string;
}
export interface Swatch {
  key: string;
  name: string;
  light: Tone;
  dark: Tone;
}

// name, light bg, light ink, light corner, dot, dark bg, dark ink, dark corner (Fold v3 board)
const SOFT: [string, string, string, string, string, string, string, string][] = [
  ["Paper", "#F5F3EE", "#3A3732", "#E0DCD2", "#B5B0A5", "rgba(255,255,255,0.04)", "#D9D5CE", "#3A383E"],
  ["Butter", "#FBF4DF", "#4A3C14", "#EADBA9", "#D9A928", "rgba(217,169,40,0.12)", "#EAD9A0", "#5E4B19"],
  ["Peach", "#FCEFE6", "#5C3418", "#F0CDB5", "#E08A55", "rgba(224,138,85,0.14)", "#F0CBB3", "#6A3E22"],
  ["Rose", "#FAEEEE", "#5A2A2E", "#EBC8CA", "#C9727A", "rgba(201,114,122,0.14)", "#EFC6C9", "#6A3238"],
  ["Lilac", "#F2EFFA", "#3D3262", "#D5CCEE", "#8D7BCB", "rgba(141,123,203,0.15)", "#D6CDF2", "#463A70"],
  ["Sky", "#ECF2F9", "#223E5D", "#C4D6EC", "#5D8BC2", "rgba(93,139,194,0.15)", "#C3D6EF", "#2C4A6D"],
  ["Ocean", "#E8F2F3", "#1D4650", "#BED8DE", "#4F98A6", "rgba(79,152,166,0.15)", "#BEDDE3", "#24535E"],
  ["Sage", "#EDF3EA", "#2D452B", "#C8DAC2", "#6F9C67", "rgba(111,156,103,0.14)", "#C6DCBF", "#3A5536"],
  ["Sand", "#F7F0E6", "#533F28", "#E3D1B9", "#B48A57", "rgba(180,138,87,0.14)", "#E5D2B8", "#5E4830"],
  ["Stone", "#EFF1F1", "#2E4243", "#CBD5D5", "#7E9596", "rgba(126,149,150,0.15)", "#CCD8D8", "#384E50"],
];

// name, light bg, light ink, light corner, dot (Colour menu board)
const VIVID: [string, string, string, string, string][] = [
  ["Lemon", "#FFF0A8", "#3D3200", "#F0D65A", "#E8C21A"],
  ["Tangerine", "#FFD8B0", "#4A2300", "#F5B67A", "#F08A2C"],
  ["Coral", "#FFC9C4", "#4D1512", "#F2A29B", "#E85D52"],
  ["Pink", "#FFC8E4", "#4A1233", "#F09ACB", "#E0559E"],
  ["Violet", "#DCCBFF", "#2A1660", "#BBA0F5", "#8457E6"],
  ["Blue", "#C4DCFF", "#0F2D5C", "#93BDF5", "#3D7FE0"],
  ["Teal", "#B8ECE6", "#0B3D38", "#86D6CC", "#1FA595"],
  ["Lime", "#D8F2A8", "#253D05", "#B5DE72", "#79B825"],
  ["Kraft", "#E8D3B4", "#3D2A10", "#D4B58A", "#A97B3F"],
  ["Graphite", "#DADCE0", "#1E2126", "#B9BDC4", "#5F6672"],
];

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHsl([r, g, b]: [number, number, number]): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l * 100];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = (g - b) / d + (g < b ? 6 : 0);
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  return [h * 60, s * 100, l * 100];
}

const hsl = (h: number, s: number, l: number) => `hsl(${h.toFixed(0)} ${s.toFixed(0)}% ${l.toFixed(0)}%)`;
const rgba = (hex: string, a: number) => {
  const [r, g, b] = hexToRgb(hex);
  return `rgba(${r},${g},${b},${a})`;
};

/** A full light + dark tone from one chosen colour, keeping text readable. */
export function toneFromDot(dot: string): { light: Tone; dark: Tone } {
  const [h, s] = rgbToHsl(hexToRgb(dot));
  return {
    light: { bg: hsl(h, Math.min(s, 62), 95), ink: hsl(h, Math.min(s, 55), 20), corner: hsl(h, Math.min(s, 50), 83), dot },
    dark: { bg: rgba(dot, 0.15), ink: hsl(h, Math.min(s, 60) * 0.8, 84), corner: hsl(h, Math.min(s, 45), 26), dot },
  };
}

export const softSwatches: Swatch[] = SOFT.map(([name, lb, li, lc, dot, db, di, dc]) => ({
  key: name,
  name,
  light: { bg: lb, ink: li, corner: lc, dot },
  dark: { bg: db, ink: di, corner: dc, dot },
}));

export const vividSwatches: Swatch[] = VIVID.map(([name, lb, li, lc, dot]) => {
  const [h, s] = rgbToHsl(hexToRgb(dot));
  return {
    key: name,
    name,
    light: { bg: lb, ink: li, corner: lc, dot },
    dark: { bg: rgba(dot, 0.2), ink: lb, corner: hsl(h, Math.min(s, 55), 28), dot },
  };
});

export interface CustomColour {
  name: string;
  dot: string;
}

export const defaultCustom: CustomColour[] = SOFT.map(([name, , , , dot]) => ({ name, dot }));

export function customSwatches(list: CustomColour[]): Swatch[] {
  return list.map((c, i) => ({ key: String(i), name: c.name, ...toneFromDot(c.dot) }));
}

export function swatchesFor(p: PaletteName, custom: CustomColour[]): Swatch[] {
  return p === "vivid" ? vividSwatches : p === "custom" ? customSwatches(custom) : softSwatches;
}

export function swatchOf(note: Pick<Note, "palette" | "color">, custom: CustomColour[]): Swatch {
  const list = swatchesFor(note.palette, custom);
  return list.find((s) => s.key === note.color) ?? softSwatches[0];
}

/** Colours given to new notes in turn (not Paper). */
export const rotation = ["Butter", "Sage", "Rose", "Sky", "Lilac", "Peach", "Ocean", "Sand", "Stone"];
