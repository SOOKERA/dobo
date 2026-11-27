// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board's look, live: what shows now (the Light or the Dark style), the saved styles,
// themes, watercolour mixes and colours, the dark-mode switch — and every change to them,
// each one undoable (Ctrl+Z, or Undo on the little message).
import { tick } from "svelte";
import { store } from "./store.svelte";
import { list, t, tMaybe } from "./i18n.svelte";
import {
  addYourColour,
  background,
  bgToast,
  coloursInUse,
  darkBySwitch,
  newId,
  ourStyle,
  OUR_SEED,
  OUR_STYLES,
  OUR_THEMES,
  readBoard,
  readColours,
  readDarkMode,
  readLastPicture,
  readMixes,
  readStyles,
  readThemes,
  sameLook,
  shownLook,
  uniqueName,
  WC_OURS,
  WC_PAINTS,
  type BgKind,
  type BoardLook,
  type DarkMode,
  type Held,
  type Look,
  type Pattern,
  type Ref,
  type ThemeRef,
  refFor,
  refBaseName,
  refName,
  resolveRef,
  themePairOf,
  themeShownName,
  themeKey,
  type SavedMix,
  type SavedStyle,
  type SavedTheme,
  type WcLook,
} from "./looks";
import { buildExport, exportFileName, mergeImport, parseZtheme, type ExportScope, type ZTheme } from "./ztheme";
import type { PaletteName } from "./types";
import { NOT_OURS } from "./ownFiles";
import { canShow, inTime } from "./thumbs.svelte";

/** An undo label: its words worked out when shown (in the language of that moment). */
type Label = string | (() => string);

export type PanelPage = "main" | "more" | "customise" | "themes" | "edit-theme" | "share";

const clone = <T>(v: T): T => structuredClone($state.snapshot(v) as T);

class LookState {
  /** Windows says dark (used by "Follow Windows"). */
  systemDark = $state(false);
  /** The style panel, and the page showing in it. */
  panelOpen = $state(false);
  page = $state<PanelPage>("main");
  /** Edit theme: which half is being changed. */
  editHalf = $state<"light" | "dark">("light");
  /** A .ztheme waiting to be added (dropped on the board or picked). */
  importing = $state<{ name: string; file: ZTheme } | null>(null);
  importError = $state<string | null>(null);
  /** The board's size (paintings are laid out at its shape). */
  boardW = $state(744);
  boardH = $state(900);
  /** Bumped when the board's look is chosen by hand (the board then paints in visibly). */
  changedByHand = $state(0);

  board = $derived<BoardLook>(readBoard(store.settings.look_board));
  styles = $derived<SavedStyle[]>(readStyles(store.settings.look_styles).sort((a, b) => b.at - a.at));
  themes = $derived<SavedTheme[]>(readThemes(store.settings.look_themes).sort((a, b) => b.at - a.at));
  mixes = $derived<SavedMix[]>(readMixes(store.settings.look_mixes).sort((a, b) => b.at - a.at));
  colours = $derived(readColours(store.settings.your_colours).sort((a, b) => b.at - a.at));
  darkMode = $derived<DarkMode>(readDarkMode(store.settings.dark_mode));

  /** Dark mode is on now (the app's chrome and the notes go dark). */
  dark = $derived(darkBySwitch(this.darkMode, store.now) ?? this.systemDark);
  /** The board shows (and the panel changes) the Dark style. */
  editingDark = $derived(this.dark && this.board.dark != null);
  active = $derived<Look>(this.editingDark ? this.board.dark! : this.board.light);
  activeFrom = $derived<Ref | null>(this.editingDark ? this.board.darkFrom : this.board.lightFrom);

  /** Background pictures seen failing to show this session (a file gone, damaged): replaced, as
   * one outside Dobo's folders is, by the import tile and a plain board. Never mutated in place. */
  private missing = $state.raw<ReadonlySet<string>>(new Set());
  /** A background picture failed to show (`<img>` or the board's own reading of it). */
  pictureFailed(path: string) {
    if (!this.missing.has(path)) this.missing = new Set([...this.missing, path]);
  }
  /** The background pictures seen failing tried again, calmly (a restore brought one back, a
   * drive is back): each once, out of sight; one that shows again is shown (its tile, the board),
   * the others stay as they are: no flicker, no word. */
  private retrying = false;
  async retryPictures(probe: (url: string) => Promise<boolean> = canShow) {
    if (this.retrying || !this.missing.size) return;
    this.retrying = true;
    try {
      const paths = [...this.missing];
      const back = await Promise.all(paths.map((p) => inTime(() => probe(store.src(p)))));
      const again = new Set(paths.filter((_, k) => back[k]));
      if (again.size) this.missing = new Set([...this.missing].filter((p) => !again.has(p)));
    } finally {
      this.retrying = false;
    }
  }
  /** Whether a background picture can be shown: one of Dobo's own folders' files (ownFiles.ts:
   * one elsewhere is never asked for), not seen failing. */
  usable(path: string | null | undefined): path is string {
    return !!path && store.src(path) !== NOT_OURS && !this.missing.has(path);
  }
  /** The look the board shows: the active one, but a picture background whose picture can't be
   * shown is a plain board in the mode's own colour (`shownLook`). */
  shown = $derived<Look>(shownLook(this.active, this.usable(this.active.picture), this.dark));

  init() {
    const q = window.matchMedia("(prefers-color-scheme: dark)");
    this.systemDark = q.matches;
    q.addEventListener("change", () => (this.systemDark = q.matches));
    // The clock (Rust) says when the hours cross a boundary, even while the window is hidden.
    store.backend.onEvent("dark-mode", () => (store.now = new Date()));
    // Pictures seen failing are tried again after a restore (the notes read again) and when the
    // window shows again (a drive plugged back meanwhile).
    store.backend.onEvent("reload", () => void this.retryPictures());
    document.addEventListener("visibilitychange", () => !document.hidden && void this.retryPictures());
    $effect.root(() => {
      $effect(() => applyMode(this.dark));
    });
  }

  // ---- Names and bases ----

  /** What the style showing is compared with: a style, or a half of the theme it came from. */
  baseOf(ref: Ref | null): Look | null {
    return resolveRef(ref, this.styles, this.themes);
  }
  nameOf(ref: Ref | null): string | null {
    return refName(ref, this.styles, this.themes);
  }
  /** The same without a theme half's "· Dark" / "· Light" (shown by its sun or moon). */
  baseNameOf(ref: Ref | null): string | null {
    return refBaseName(ref, this.styles, this.themes);
  }
  /** A theme's name as shown (ours in your language). */
  themeNameOf(ref: ThemeRef | null): string | null {
    const pair = themePairOf(ref, this.themes);
    return pair == null ? null : ref?.kind === "ours" ? tMaybe(pair.name) : themeShownName(pair);
  }
  themePair(ref: ThemeRef | null): { name: string; light: Look; dark: Look | null } | null {
    return themePairOf(ref, this.themes);
  }

  // ---- Saving (one undoable step per change) ----

  /** `before` runs as the change is made (a crossfade's may wait for the one before it). */
  private async apply(changes: Record<string, unknown>, before?: () => void) {
    const run = async () => {
      before?.();
      for (const [k, v] of Object.entries(changes)) await store.setSetting(k, v ?? null);
    };
    // What the board or the mode looks like changes (a theme, a style, a mix, light or dark, and
    // their Undo): the old look crossfades into the new one.
    const seen = "look_board" in changes || ("dark_mode" in changes && (darkBySwitch(readDarkMode(changes.dark_mode), store.now) ?? this.systemDark) !== this.dark);
    // (Either way after a crossfade's change still on its way: the last one made stays.)
    return seen ? crossfade(run) : afterMaking(run);
  }
  /** Undo labels are worked out when shown (in the language of that moment), not when recorded. */
  private async put(changes: Record<string, unknown>, label: Label | null, toast?: Label) {
    // (What it was is read, and the Undo step recorded, as the change is made: after a change
    // still crossfading in, that one, and its step comes first.)
    const prev: Record<string, unknown> = {};
    const next = clone(changes);
    await this.apply(next, () => {
      for (const k of Object.keys(changes)) prev[k] = clone(store.settings[k] ?? null);
      if (label) store.history.record({ label, undo: () => this.apply(prev), redo: () => this.apply(next) });
    });
    if (toast) store.showToast(toast, { label: () => t("Undo"), run: () => void store.undo() });
  }

  /** Changes the style showing now (the Dark one when dark mode shows it). */
  async setActive(l: Look, from: Ref | null | undefined, label: Label, toast?: Label) {
    const b = clone(this.board);
    const look = clone(l);
    if (this.editingDark) {
      b.dark = look;
      if (from !== undefined) b.darkFrom = from;
    } else {
      b.light = look;
      if (from !== undefined) b.lightFrom = from;
    }
    this.changedByHand++;
    await this.put({ look_board: b }, label, toast);
  }

  pickStyle(ref: Ref) {
    const l = this.baseOf(ref);
    const was = this.nameOf(ref) ?? "";
    if (l) return this.setActive(l, ref, () => t("Style: {name}", { name: this.nameOf(ref) ?? was }));
  }
  async reset() {
    const l = this.baseOf(this.activeFrom);
    if (l) await this.setActive(l, this.activeFrom, () => t("Reset"));
  }

  /** The last painting and picture chosen, so switching background and back keeps them. The
   * picture is a setting (look.rs keeps its file): it stays over a restart, and Undo of the
   * change of background never forgets it. */
  private lastWc: WcLook | null = null;
  private lastPicture = $derived(readLastPicture(store.settings.look_last_picture));
  private rememberPicture(p: string) {
    // (A failed save is told as every one is, backend.ts `guardWrites`: the usual toast, and the
    // board shows what is really saved. Only said here, never left unhandled.)
    if (p !== this.lastPicture) void store.setSetting("look_last_picture", { picture: p }).catch((e) => console.error("Your last picture wasn't saved:", e));
  }
  /** Your picture, if you have imported one: the last one chosen, or one a style or theme of
   * yours uses (the picture tile shows it; without one, the tile imports). */
  get ownPicture(): string | null {
    const last = this.lastPicture;
    return this.usable(last) ? last : this.knownPicture();
  }

  async setBg(bg: BgKind) {
    const a = this.active;
    if (a.bg === bg) return;
    if (a.wc) this.lastWc = clone(a.wc);
    if (this.usable(a.picture)) this.rememberPicture(a.picture);
    const next: Look = { bg, colour: "", held: a.held };
    if (bg === "watercolour") next.wc = this.lastWc ?? { colours: WC_OURS[0].colours, pattern: "Scattered", seed: OUR_SEED, paper: "white", name: WC_OURS[0].name };
    else if (bg === "picture") {
      const pic = this.ownPicture;
      if (!pic) return this.pickPicture();
      next.picture = pic;
    } else next.colour = background(bg).palette[0].name;
    await this.setActive(next, undefined, () => bgToast(bg));
  }
  private knownPicture(): string | null {
    const all = [this.board.light, this.board.dark, ...this.styles.map((s) => s.look), ...this.themes.flatMap((t) => [t.light, t.dark])];
    // (One that can be shown: one outside Dobo's folders, or gone, makes the tile import again.)
    return all.find((l) => l?.bg === "picture" && this.usable(l.picture))?.picture ?? null;
  }
  async pickPicture() {
    try {
      const p = await store.backend.pickBackgroundPicture();
      if (!p) return;
      this.rememberPicture(p);
      await this.setActive({ bg: "picture", colour: "", held: this.active.held, picture: p }, undefined, () => t("Your picture"));
    } catch (e) {
      store.showToast(String(e));
    }
  }
  setColour(name: string) {
    return this.setActive({ ...this.active, colour: name, custom: undefined }, undefined, () => t("Colour: {name}", { name: tMaybe(name) }));
  }
  setCustom(hex: string) {
    return this.setActive({ ...this.active, colour: "Custom", custom: hex.toUpperCase() }, undefined, () => t("Custom colour"));
  }
  setHeld(held: Held) {
    return this.setActive({ ...this.active, held }, undefined, () => held === "auto" ? t("Notes held as usual") : held === "none" ? t("No fixings") : t("Notes held by {how}", { how: held === "pins" ? t("pins") : t("tape") }));
  }

  // ---- Watercolour ----

  get wc(): WcLook {
    return this.active.wc ?? { colours: WC_OURS[0].colours, pattern: "Scattered", seed: OUR_SEED, paper: "white", name: WC_OURS[0].name };
  }
  private setWc(w: Partial<WcLook>, label: Label, toast?: Label) {
    const next: WcLook = { ...clone(this.wc), ...w };
    if (next.name === undefined) delete next.name;
    return this.setActive({ ...this.active, bg: "watercolour", colour: "", wc: next }, undefined, label, toast);
  }
  pickMix(colours: string[], name: string, pattern?: Pattern, paper?: "white" | "dark") {
    return this.setWc({ colours: [...colours], name, ...(pattern ? { pattern } : {}), ...(paper ? { paper } : {}) }, () => t("Watercolour: {name}", { name: tMaybe(name) }));
  }
  setPattern(p: Pattern) {
    return this.setWc({ pattern: p }, () => t("Pattern: {name}", { name: t(p) }));
  }
  repaint() {
    return this.setWc({ seed: 1 + Math.floor(Math.random() * 2147483000) }, () => t("Repaint"), () => t("Repainted · same colours, a fresh painting"));
  }
  setPaper(paper: "white" | "dark") {
    return this.setWc({ paper }, () => paper === "dark" ? t("Dark paper") : t("White paper"));
  }
  setSlot(i: number, rgb: string) {
    const c = [...this.wc.colours];
    c[i] = rgb;
    return this.setWc({ colours: c, name: undefined }, () => t("Colour changed"));
  }
  removeSlot(i: number) {
    if (this.wc.colours.length <= 1) return;
    return this.setWc({ colours: this.wc.colours.filter((_, k) => k !== i), name: undefined }, () => t("Colour removed"));
  }
  addSlot() {
    const have = this.wc.colours;
    if (have.length >= 3) return;
    const next = WC_PAINTS.map((p) => p[1]).find((c) => !have.includes(c)) ?? WC_PAINTS[0][1];
    return this.setWc({ colours: [...have, next], name: undefined }, () => t("Colour added"));
  }

  async saveMix(name: string) {
    const w = this.wc;
    const m: SavedMix = { id: newId("m"), name: uniqueName(name, this.mixes.map((x) => x.name)), colours: [...w.colours], pattern: w.pattern, paper: w.paper, at: Date.now() };
    await this.put({ look_mixes: [m, ...clone(this.mixes)] }, () => t("Saved “{name}”", { name: m.name }), () => t("Saved “{name}” in your watercolours", { name: m.name }));
    await this.setWc({ name: m.name }, "");
  }
  renameMix(id: string, name: string) {
    const n = uniqueName(name, this.mixes.filter((m) => m.id !== id).map((m) => m.name));
    return this.put({ look_mixes: clone(this.mixes).map((m) => (m.id === id ? { ...m, name: n } : m)) }, () => t("Renamed"));
  }
  deleteMix(id: string) {
    const m = this.mixes.find((x) => x.id === id);
    return this.put({ look_mixes: clone(this.mixes).filter((x) => x.id !== id) }, () => t("Mix deleted"), () => t("Deleted “{name}” (look)", { name: m?.name ?? t("mix") }));
  }

  // ---- Your colours (shared with the note colours) ----

  addColour(rgb: string) {
    const inUse = coloursInUse(this.styles, this.themes, this.mixes, this.board);
    return this.put({ your_colours: addYourColour(clone(this.colours), rgb, inUse) }, null);
  }
  removeColour(rgb: string) {
    return this.put({ your_colours: clone(this.colours).filter((c) => c.rgb !== rgb) }, () => t("Colour removed"), () => t("Removed from your colours"));
  }

  // ---- Your styles ----

  async saveStyle(name: string) {
    const s: SavedStyle = { id: newId("s"), name: uniqueName(name, this.styles.map((x) => x.name)), look: clone(this.active), at: Date.now() };
    const b = clone(this.board);
    if (this.editingDark) b.darkFrom = { kind: "yours", id: s.id };
    else b.lightFrom = { kind: "yours", id: s.id };
    await this.put({ look_styles: [s, ...clone(this.styles)], look_board: b }, () => t("Saved “{name}”", { name: s.name }), () => t("Saved “{name}” in your styles", { name: s.name }));
  }
  updateStyle(id: string) {
    const s = this.styles.find((x) => x.id === id);
    return this.put({ look_styles: clone(this.styles).map((x) => (x.id === id ? { ...x, look: clone(this.active) } : x)) }, () => t("Updated “{name}”", { name: s?.name ?? "" }), () => t("Updated “{name}”", { name: s?.name ?? "" }));
  }
  renameStyle(id: string, name: string) {
    const n = uniqueName(name, this.styles.filter((s) => s.id !== id).map((s) => s.name));
    return this.put({ look_styles: clone(this.styles).map((s) => (s.id === id ? { ...s, name: n } : s)) }, () => t("Renamed"));
  }
  duplicateStyle(id: string) {
    const s = this.styles.find((x) => x.id === id);
    if (!s) return;
    const copy: SavedStyle = { ...clone(s), id: newId("s"), name: uniqueName(s.name, this.styles.map((x) => x.name)), at: Date.now() };
    return this.put({ look_styles: [copy, ...clone(this.styles)] }, () => t("Duplicated"));
  }
  deleteStyle(id: string) {
    const s = this.styles.find((x) => x.id === id);
    return this.put({ look_styles: clone(this.styles).filter((x) => x.id !== id) }, () => t("Style deleted"), () => t("Deleted “{name}” (look)", { name: s?.name ?? t("style") }));
  }

  // ---- Themes ----

  /** Uses a pair (the board switches at once, Undo brings the old one back). */
  useTheme(pair: { light: Look; dark: Look | null }, ref: ThemeRef | null, name: string) {
    const b: BoardLook = {
      light: clone(pair.light),
      dark: pair.dark ? clone(pair.dark) : null,
      lightFrom: refFor(pair.light, "light", ref, this.styles),
      darkFrom: pair.dark ? refFor(pair.dark, "dark", ref, this.styles) : null,
      theme: ref,
    };
    this.changedByHand++;
    return this.put({ look_board: b }, () => t("Theme: {name}", { name: this.themeNameOf(ref) ?? name }));
  }
  /** Changes one half of the board's pair (Edit theme). null: the dark half follows the light one. */
  setHalf(half: "light" | "dark", l: Look | null, from: Ref | null) {
    const b = clone(this.board);
    if (half === "light") {
      b.light = clone(l!);
      b.lightFrom = from;
    } else {
      b.dark = l ? clone(l) : null;
      b.darkFrom = l ? from : null;
    }
    this.changedByHand++;
    return this.put({ look_board: b }, () => half === "light" ? t("Light style") : t("Dark style"));
  }
  resetTheme() {
    const pair = this.themePair(this.board.theme);
    if (pair) return this.useTheme(pair, this.board.theme, t("Reset"));
  }
  async saveTheme(name: string) {
    const th: SavedTheme = { id: newId("t"), name: uniqueName(name, this.themes.map((x) => x.name)), light: clone(this.board.light), dark: this.board.dark ? clone(this.board.dark) : null, at: Date.now() };
    // The halves now come from the new theme (unless they are one of the styles exactly).
    const ref: ThemeRef = { kind: "yours", id: th.id };
    const b = clone(this.board);
    if (b.lightFrom?.kind === "theme") b.lightFrom = { kind: "theme", id: themeKey(ref), half: "light" };
    if (b.darkFrom?.kind === "theme") b.darkFrom = { kind: "theme", id: themeKey(ref), half: "dark" };
    b.theme = ref;
    await this.put({ look_themes: [th, ...clone(this.themes)], look_board: b }, () => t("Saved “{name}”", { name: th.name }), () => t("Saved “{name}” in your themes", { name: th.name }));
  }
  updateTheme(id: string) {
    const th = this.themes.find((x) => x.id === id);
    return this.put(
      { look_themes: clone(this.themes).map((x) => (x.id === id ? { ...x, light: clone(this.board.light), dark: this.board.dark ? clone(this.board.dark) : null } : x)) },
      () => t("Updated “{name}”", { name: th?.name ?? "" }),
      () => t("Updated “{name}”", { name: th?.name ?? "" }),
    );
  }
  renameTheme(id: string, name: string) {
    const n = uniqueName(name, this.themes.filter((x) => x.id !== id).map((x) => x.name));
    return this.put({ look_themes: clone(this.themes).map((x) => (x.id === id ? { ...x, name: n } : x)) }, () => t("Renamed"));
  }
  duplicateTheme(pair: { light: Look; dark: Look | null }, name: string) {
    const th: SavedTheme = { id: newId("t"), name: uniqueName(name, [...this.themes.map((x) => x.name), ...OUR_THEMES.map((x) => x.name), ...OUR_THEMES.map((x) => tMaybe(x.name))]), light: clone(pair.light), dark: pair.dark ? clone(pair.dark) : null, at: Date.now() };
    return this.put({ look_themes: [th, ...clone(this.themes)] }, () => t("Duplicated")).then(() => th);
  }
  deleteTheme(id: string) {
    const th = this.themes.find((x) => x.id === id);
    const b = clone(this.board);
    if (b.theme?.kind === "yours" && b.theme.id === id) b.theme = null;
    // Halves that came from it keep their look; they are compared with a style from now on.
    const gone = themeKey({ kind: "yours", id });
    if (b.lightFrom?.kind === "theme" && b.lightFrom.id === gone) b.lightFrom = refFor(b.light, "light", null, this.styles);
    if (b.darkFrom?.kind === "theme" && b.darkFrom.id === gone && b.dark) b.darkFrom = refFor(b.dark, "dark", null, this.styles);
    return this.put({ look_themes: clone(this.themes).filter((x) => x.id !== id), look_board: b }, () => t("Theme deleted"), () => t("Deleted “{name}” (look)", { name: th?.name ?? t("theme") }));
  }

  // ---- Dark mode, note size, new notes ----

  setDarkMode(m: DarkMode) {
    store.now = new Date();
    return this.put({ dark_mode: m }, () => t("Appearance"));
  }
  get newNoteColour(): { palette: PaletteName; color: string } | null {
    const v = store.settings.new_note_colour as { palette?: string; color?: string } | null | undefined;
    if (!v || typeof v.color !== "string" || !["soft", "vivid", "custom"].includes(v.palette ?? "")) return null;
    return { palette: v.palette as PaletteName, color: v.color };
  }
  setNewNoteColour(v: { palette: PaletteName; color: string } | null) {
    return this.put({ new_note_colour: v }, () => t("New notes’ colour"));
  }

  // ---- Export / import ----

  async exportLook(scope: ExportScope, theme?: { name: string; light: Look; dark: Look | null }) {
    const th: SavedTheme | undefined = theme ? { id: newId("t"), name: theme.name, light: clone(theme.light), dark: theme.dark ? clone(theme.dark) : null, at: Date.now() } : undefined;
    const z = buildExport(scope, { theme: th, themes: clone(this.themes), styles: clone(this.styles), mixes: clone(this.mixes), colours: clone(this.colours), darkMode: clone(this.darkMode) });
    try {
      // The file keeps our names as they are underneath (they show in each language on import);
      // only the file's own name is in your language.
      const saved = await store.backend.exportLook(JSON.stringify({ ...z, app: "Dobo" }), exportFileName(scope, theme ? tMaybe(theme.name) : undefined));
      if (saved) store.showToast(() => t("Saved {file}", { file: saved.split(/[\\/]/).pop() ?? "" }));
    } catch (e) {
      store.showToast(String(e));
    }
  }
  /** Reads a look file (dropped, or picked when path is null) and shows what's inside. */
  async readImport(path: string | null) {
    this.importError = null;
    try {
      const got = await store.backend.importLook(path);
      if (!got) return;
      this.importing = { name: got[0], file: parseZtheme(got[1]) };
    } catch (e) {
      this.importing = null;
      this.importError = e instanceof Error ? e.message : String(e);
    }
    this.panelOpen = true;
    this.page = "share";
  }
  async addImport() {
    const f = this.importing;
    if (!f) return;
    const m = mergeImport(f.file, { themes: clone(this.themes), styles: clone(this.styles), mixes: clone(this.mixes), colours: clone(this.colours) });
    this.importing = null;
    const parts = [
      m.added.themes && t("{n} themes", { n: m.added.themes }),
      m.added.styles && t("{n} styles", { n: m.added.styles }),
      m.added.mixes && t("{n} mixes", { n: m.added.mixes }),
      m.added.colours && t("{n} colours", { n: m.added.colours }),
    ].filter((x): x is string => !!x);
    await this.put({ look_themes: m.themes, look_styles: m.styles, look_mixes: m.mixes, your_colours: m.colours }, () => t("Import"), () => parts.length ? t("Added {list}", { list: list(parts) }) : t("Nothing new to add"));
    this.page = "themes";
  }

  open(page: PanelPage = "main") {
    this.panelOpen = true;
    this.page = page;
  }
}

export const look = new LookState();

let fading = false;
let lastFade = -Infinity;
/** A crossfade's change until it is made (it waits for the old frame to be kept): a change that
 * comes meanwhile waits for it, so the last one made is the one that stays. */
let making: Promise<void> | null = null;
function afterMaking(update: () => Promise<void>): Promise<void> {
  if (!making) return update();
  const run = making.then(update);
  hold(run);
  return run;
}
function hold(p: Promise<unknown>) {
  const mine = p.then(
    () => {},
    () => {},
  );
  making = mine;
  void mine.then(() => {
    if (making === mine) making = null;
  });
}
/** A change of look, crossfaded (about 200 ms) where the web view can (View Transitions;
 * WebView2 can): the old frame stays on screen while the whole page restyles, then fades into the
 * new one, instead of a frozen frame and a jump. Not with reduced motion, not in a hidden window,
 * and not for a change that follows another at once (a colour dragged, mixes clicked through):
 * those just apply. Only ever from a change made by hand (never at start, nor while views are
 * built off the page). */
export async function crossfade(update: () => Promise<void>): Promise<void> {
  const doc = typeof document === "undefined" ? null : (document as Document & { startViewTransition?: (cb: () => Promise<void>) => { finished: Promise<void>; updateCallbackDone: Promise<void> } });
  const now = performance.now();
  const quick = fading || now - lastFade < 450;
  if (!doc?.startViewTransition || quick || doc.visibilityState !== "visible" || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    if (doc && quick) lastFade = now;
    return afterMaking(update);
  }
  fading = true;
  lastFade = now;
  let made!: (p: Promise<void>) => void;
  hold(new Promise<void>((r) => (made = (p) => r(p))));
  let done: Promise<void> | undefined;
  const vt = doc.startViewTransition(async () => {
    done = update();
    made(done);
    // The new look drawn (the settings are set at once; saving them may take a moment longer,
    // and never keeps the old frame up).
    await Promise.race([done, new Promise((r) => setTimeout(r, 150))]);
    await tick();
  });
  void vt.finished.catch(() => {}).finally(() => (fading = false));
  await vt.updateCallbackDone.catch(() => {});
  if (!done) {
    // (The page never ran it: made now.)
    done = update();
    made(done);
  }
  await done;
}

/** Dark mode on the page: the app's chrome, menus and notes switch together. */
export function applyMode(dark: boolean) {
  const root = document.documentElement;
  root.dataset.mode = dark ? "dark" : "light";
  root.style.colorScheme = dark ? "dark" : "light";
}
