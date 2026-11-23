// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The .ztheme file: themes, styles, watercolour mixes, your colours and dark-mode hours.
// Never notes. Pictures stay on this computer (their file is never written into it).
// An imported file is untrusted: it is read strictly (Rust checks it too) and nothing existing
// is ever replaced — a name already used gets "(2)".
// (".ztheme" and its "ztheme" field come from the app's working name, ZToDo. They stay: files
// people already exported must still open.)
import { t } from "./i18n.svelte";
import {
  cleanLook,
  readColours,
  readDarkMode,
  readMixes,
  readStyles,
  readThemes,
  uniqueName,
  newId,
  type DarkMode,
  type Look,
  type SavedMix,
  type SavedStyle,
  type SavedTheme,
  type YourColour,
} from "./looks";

export const ZTHEME_VERSION = 1;
/** Bigger files are refused before reading (a look is a few kilobytes). */
export const ZTHEME_MAX_BYTES = 512 * 1024;

export type ExportScope = "theme" | "themes" | "everything";

export interface ZTheme {
  ztheme: number;
  kind: ExportScope;
  themes: SavedTheme[];
  styles: SavedStyle[];
  mixes: SavedMix[];
  colours: YourColour[];
  dark_mode?: DarkMode;
}

const noPicture = (l: Look | null): Look | null => (l && l.bg === "picture" ? { ...l, picture: undefined } : l);

/** Custom colours a look uses (watercolour colours that aren't plain paints are kept with it). */
function coloursOf(looks: (Look | null)[], yours: YourColour[]): YourColour[] {
  const used = new Set<string>();
  for (const l of looks) l?.wc?.colours.forEach((c) => used.add(c));
  return yours.filter((c) => used.has(c.rgb));
}

export function buildExport(
  scope: ExportScope,
  data: { theme?: SavedTheme; themes: SavedTheme[]; styles: SavedStyle[]; mixes: SavedMix[]; colours: YourColour[]; darkMode: DarkMode },
): ZTheme {
  const strip = (t: SavedTheme): SavedTheme => ({ ...t, light: noPicture(t.light)!, dark: noPicture(t.dark) });
  if (scope === "theme") {
    const t = data.theme!;
    return { ztheme: ZTHEME_VERSION, kind: scope, themes: [strip(t)], styles: [], mixes: [], colours: coloursOf([t.light, t.dark], data.colours) };
  }
  if (scope === "themes") {
    return { ztheme: ZTHEME_VERSION, kind: scope, themes: data.themes.map(strip), styles: [], mixes: [], colours: coloursOf(data.themes.flatMap((t) => [t.light, t.dark]), data.colours) };
  }
  return {
    ztheme: ZTHEME_VERSION,
    kind: scope,
    themes: data.themes.map(strip),
    styles: data.styles.map((s) => ({ ...s, look: noPicture(s.look)! })),
    mixes: data.mixes,
    colours: data.colours,
    dark_mode: data.darkMode,
  };
}

/** Reads a .ztheme file's text strictly. Throws a friendly message when it isn't one. */
export function parseZtheme(text: string): ZTheme {
  if (text.length > ZTHEME_MAX_BYTES) throw new Error(t("That file is too big to be a Dobo look."));
  let v: unknown;
  try {
    v = JSON.parse(text);
  } catch {
    throw new Error(t("That file isn’t a Dobo look (it can’t be read)."));
  }
  if (typeof v !== "object" || v === null || Array.isArray(v)) throw new Error(t("That file isn’t a Dobo look."));
  const o = v as Record<string, unknown>;
  if (o.ztheme !== ZTHEME_VERSION) throw new Error(t("That look was made by another version of Dobo."));
  const known = new Set(["ztheme", "kind", "themes", "styles", "mixes", "colours", "dark_mode", "app"]);
  for (const k of Object.keys(o)) if (!known.has(k)) throw new Error(t("That file has things Dobo doesn’t know; it wasn’t added."));
  for (const k of ["themes", "styles", "mixes", "colours"]) if (o[k] !== undefined && !Array.isArray(o[k])) throw new Error(t("That file is damaged."));
  const kind: ExportScope = o.kind === "theme" || o.kind === "themes" ? o.kind : "everything";
  // Pictures are never taken from a file (a path from someone else's computer means nothing here).
  const themes = readThemes(o.themes, false);
  const styles = readStyles(o.styles, false);
  const mixes = readMixes(o.mixes);
  const colours = readColours(o.colours);
  if (!themes.length && !styles.length && !mixes.length && !colours.length) throw new Error(t("There is nothing to add in that file."));
  return { ztheme: ZTHEME_VERSION, kind, themes, styles, mixes, colours, dark_mode: o.dark_mode !== undefined ? readDarkMode(o.dark_mode) : undefined };
}

/** Adds an imported file to what you have: nothing replaced, clashing names get "(2)", new ids. */
export function mergeImport(
  file: ZTheme,
  have: { themes: SavedTheme[]; styles: SavedStyle[]; mixes: SavedMix[]; colours: YourColour[] },
  now = Date.now(),
): { themes: SavedTheme[]; styles: SavedStyle[]; mixes: SavedMix[]; colours: YourColour[]; added: { themes: number; styles: number; mixes: number; colours: number } } {
  const themes = [...have.themes];
  const styles = [...have.styles];
  const mixes = [...have.mixes];
  const colours = [...have.colours];
  let n = 0;
  const fresh = () => now + n++;
  for (const t of file.themes) themes.unshift({ ...t, id: newId("t"), name: uniqueName(t.name, themes.map((x) => x.name)), at: fresh(), light: cleanLook(t.light)!, dark: cleanLook(t.dark) });
  for (const s of file.styles) styles.unshift({ ...s, id: newId("s"), name: uniqueName(s.name, styles.map((x) => x.name)), at: fresh() });
  for (const m of file.mixes) mixes.unshift({ ...m, id: newId("m"), name: uniqueName(m.name, mixes.map((x) => x.name)), at: fresh() });
  let newColours = 0;
  for (const c of file.colours)
    if (!colours.some((x) => x.rgb === c.rgb)) {
      colours.unshift({ rgb: c.rgb, at: fresh() });
      newColours++;
    }
  return { themes, styles, mixes, colours, added: { themes: file.themes.length, styles: file.styles.length, mixes: file.mixes.length, colours: newColours } };
}

export function exportFileName(scope: ExportScope, themeName?: string): string {
  const safe = (s: string) => s.replace(/[\\/:*?"<>|]+/g, " ").trim() || t("My look");
  if (scope === "theme") return `${safe(themeName ?? t("Theme"))}.ztheme`;
  return scope === "themes" ? `${t("My themes")}.ztheme` : `${t("My Dobo look")}.ztheme`;
}
