// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The app's words in English and French. A string is written in English where it is used,
// `t("Done")`, and that English is its key: the French dictionary (./i18n/fr.ts) must have every
// key used, so a missing translation is a type error. Plurals: an entry can be { one, other },
// picked by `n` ("{n} task" → "1 tâche", "3 tâches"); English plurals live in ./i18n/en.ts.
// Placeholders: "{name}". The language follows the setting (Follow Windows · English ·
// Français) and is reactive: anything that calls t() in a template or $derived updates live.
import { fr } from "./i18n/fr";
import { en } from "./i18n/en";

export type Lang = "en" | "fr";
export type LangChoice = "system" | Lang;
export type Key = keyof typeof fr;
export interface Plural {
  one: string;
  other: string;
}
export type Entry = string | Plural;
type Params = Record<string, string | number>;

/** French typography: a non-breaking space before : and », after «; a thin one before ; ? !. And the names
 * that must never break: « À trier » (the Inbox), « menu ⋯ ». */
export function frType(s: string): string {
  // (« first: a key quoted on its own, « ! », keeps its quotes' spaces, never a thin one.)
  return s
    .replace(/À trier/g, "À trier")
    .replace(/menu ⋯/g, "menu ⋯")
    .replace(/« /g, "« ")
    .replace(/ :/g, " :")
    .replace(/ ([;?!])/g, " $1")
    .replace(/ »/g, " »");
}

class Lingo {
  /** What Settings says. */
  choice = $state<LangChoice>("system");
  /** What Windows speaks. */
  system = $state<Lang>(guessSystem());
  lang = $derived<Lang>(this.choice === "system" ? this.system : this.choice);
  /** For Intl: dates, numbers, plurals. */
  locale = $derived(this.lang === "fr" ? "fr-FR" : "en-GB");
  /** Windows shows the time on a 12-hour clock ("3:00 PM"): every time Dobo shows does too, in
   * either language. Read by Rust (clock.rs), again when Windows' settings change. */
  hour12 = $state(false);
}
export const i18n = new Lingo();

function guessSystem(): Lang {
  // Unit tests speak English whatever this computer speaks (they set French themselves).
  if ((globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env?.VITEST) return "en";
  const langs = typeof navigator === "undefined" ? [] : navigator.languages?.length ? navigator.languages : [navigator.language];
  return /^fr\b/i.test(langs[0] ?? "") ? "fr" : "en";
}

export function readChoice(v: unknown): LangChoice {
  return v === "en" || v === "fr" ? v : "system";
}

const frCache = new Map<string, Entry>();
function french(key: Key): Entry {
  let e = frCache.get(key);
  if (e === undefined) {
    const raw = fr[key] as Entry;
    e = typeof raw === "string" ? frType(raw) : { one: frType(raw.one), other: frType(raw.other) };
    frCache.set(key, e);
  }
  return e;
}

const rules = new Map<string, Intl.PluralRules>();
function plural(lang: Lang, n: number): "one" | "other" {
  let r = rules.get(lang);
  if (!r) rules.set(lang, (r = new Intl.PluralRules(lang === "fr" ? "fr-FR" : "en-GB")));
  return r.select(n) === "one" ? "one" : "other";
}

function fill(s: string, p?: Params): string {
  if (!p) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) => (k in p ? String(p[k]) : m));
}

/** Words on screen that may outlive a language switch (a toast, a status line): a function
 * gives them again in the language of the moment. */
export type Words = string | (() => string);
export const words = (w: Words): string => (typeof w === "function" ? w() : w);

/** The words for `key` in the current language, placeholders filled. */
export function t(key: Key, params?: Params): string {
  return tIn(i18n.lang, key, params);
}

/** The words for `key` in a given language (tests, the language picker's own names). */
export function tIn(lang: Lang, key: Key, params?: Params): string {
  const e: Entry = lang === "fr" ? french(key) : (en[key] ?? key);
  if (typeof e === "string") return fill(e, params);
  const n = Number(params?.n ?? 0);
  return fill(plural(lang, n) === "one" ? e.one : e.other, params);
}

/** A string that may or may not be one of ours (our style names, a note called "Inbox"): ours
 * are translated, anything else (the person's own names) stays as written. */
export function tMaybe(s: string): string {
  if (i18n.lang === "en" || !(s in fr)) return s;
  return t(s as Key);
}
export function isKey(s: string): s is Key {
  return s in fr;
}

// ---- Keys on the keyboard, as Windows names them in each language ----

const FR_KEYS: Record<string, string> = {
  Shift: "Maj",
  Esc: "Échap",
  Escape: "Échap",
  Enter: "Entrée",
  Delete: "Suppr",
  Del: "Suppr",
  Backspace: "Retour arrière",
  Space: "Espace",
  "Page Up": "Pg préc",
  "Page Down": "Pg suiv",
  PageUp: "Pg préc",
  PageDown: "Pg suiv",
  Home: "Début",
  End: "Fin",
  Up: "Haut",
  Down: "Bas",
  Left: "Gauche",
  Right: "Droite",
  Insert: "Inser",
  Comma: ",",
  "Mouse back": "Bouton Précédent",
  Wheel: "Molette",
  "Mouse forward": "Bouton Suivant",
};

/** One key's name ("Shift" → "Maj" in French). */
export function keyName(k: string): string {
  if (i18n.lang !== "fr") return k === "Comma" ? "," : k;
  return FR_KEYS[k] ?? k;
}

/** A shortcut as written ("Ctrl+Shift+Space") in the current language ("Ctrl+Maj+Espace"). */
export function keysText(accel: string): string {
  if (i18n.lang !== "fr") return accel.replace(/\+Comma$/, "+,");
  // Keep "Ctrl++" (the + key) whole.
  const parts = accel.split(/\+(?!$)/);
  return parts.map((p) => keyName(p.trim())).join("+");
}

// ---- Dates and times ----

const fmts = new Map<string, Intl.DateTimeFormat>();
/** A cached Intl.DateTimeFormat for the current language, for dates (times: `clockTime`). */
export function dateFmt(opts: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const id = `${i18n.locale}|${JSON.stringify(opts)}`;
  let f = fmts.get(id);
  if (!f) fmts.set(id, (f = new Intl.DateTimeFormat(i18n.locale, { hourCycle: "h23", ...opts })));
  return f;
}

/** A time as Windows shows it: "15:00", or "3:00 PM" on a 12-hour clock (the same in French:
 * the language is the app's, the clock is Windows'). The space before AM/PM never breaks. */
export function clockTime(h: number, m: number): string {
  const mm = String(m).padStart(2, "0");
  if (!i18n.hour12) return `${String(h).padStart(2, "0")}:${mm}`;
  return `${h % 12 || 12}:${mm}\u00a0${h < 12 ? "AM" : "PM"}`;
}

/** An hour on the drag ruler: "14" in English, « 14h » in French, or "2 PM" on a 12-hour clock. */
export function hourMark(h: number): string {
  if (i18n.hour12) return `${h % 12 || 12}\u00a0${h < 12 ? "AM" : "PM"}`;
  return i18n.lang === "fr" ? `${h}h` : String(h);
}

/** A day and its time: "Thu 19 Nov, 15:05" (long: "Thursday 19 November at 15:05"); French
 * « jeu. 19 nov., 15:05 », « jeudi 19 novembre à 15:05 ». */
export function dateAndTime(d: Date, long = false): string {
  const day = dateFmt(long ? { weekday: "long", day: "numeric", month: "long" } : { weekday: "short", day: "numeric", month: "short" }).format(d);
  const time = clockTime(d.getHours(), d.getMinutes());
  return long ? `${day} ${i18n.lang === "fr" ? "à" : "at"} ${time}` : `${day}, ${time}`;
}

/** "Thu" / "jeu." */
export const weekdayShort = (d: Date) => dateFmt({ weekday: "short" }).format(d);
/** "Thursday" / "jeudi" */
export const weekdayLong = (d: Date) => dateFmt({ weekday: "long" }).format(d);
/** "Nov" / "nov." */
export const monthShort = (d: Date) => dateFmt({ month: "short" }).format(d);
/** "19 Nov" / "19 nov." */
export const dayMonth = (d: Date) => dateFmt({ day: "numeric", month: "short" }).format(d);
/** "19 Nov 2025" / "19 nov. 2025" */
export const dayMonthYear = (d: Date) => dateFmt({ day: "numeric", month: "short", year: "numeric" }).format(d);
/** "19 November" / "19 novembre" */
export const dayMonthLong = (d: Date) => dateFmt({ day: "numeric", month: "long" }).format(d);

/** A weekday by number (0 = Sunday): "Mon" / "lun.", or whole: "Monday" / "lundi". */
export function dayName(i: number, long = false): string {
  const d = new Date(2023, 0, 1 + i);
  return long ? weekdayLong(d) : weekdayShort(d);
}

/** A number the way the language writes it ("1.5" / "1,5"). */
export function num(n: number, digits = 0): string {
  return new Intl.NumberFormat(i18n.locale, { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(n);
}

/** A file size: "820 KB", "1.4 MB" / "820 Ko", "1,4 Mo". */
export function fileSize(b: number): string {
  const fr = i18n.lang === "fr";
  return b > 1_048_576 ? `${num(b / 1_048_576, 1)} ${fr ? "Mo" : "MB"}` : `${Math.max(1, Math.round(b / 1024))} ${fr ? "Ko" : "KB"}`;
}

/** A capital first letter ("jeudi" → "Jeudi") for the start of a line. */
export const cap = (s: string) => (s ? s.charAt(0).toLocaleUpperCase(i18n.locale) + s.slice(1) : s);

/** "3rd" / "3e". */
export function ordinal(n: number): string {
  if (i18n.lang === "fr") return n === 1 ? "1er" : `${n}e`;
  const s = n % 100 >= 11 && n % 100 <= 13 ? "th" : ["th", "st", "nd", "rd"][n % 10] ?? "th";
  return `${n}${s}`;
}

/** "10 min", "1 h", "1 h 30" (French) / "10 min", "1 h", "1 h 30 min" (English). */
export function duration(min: number): string {
  const m = Math.round(min);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  const r = m % 60;
  if (!r) return `${h} h`;
  return i18n.lang === "fr" ? `${h} h ${String(r).padStart(2, "0")}` : `${h} h ${r} min`;
}

/** A sentence around a mark ("Empty. Type {tag} above…") as [before, after], to put something
 * styled in the middle. */
export function around(s: string, mark: string): [string, string] {
  const i = s.indexOf(mark);
  return i < 0 ? [s, ""] : [s.slice(0, i), s.slice(i + mark.length)];
}

/** A list joined the way each language does ("a, b and c" / "a, b et c"). */
export function list(items: string[]): string {
  return new Intl.ListFormat(i18n.locale, { type: "conjunction" }).format(items);
}

/** A window without the board (the quick dump bar): its language as the setting says, and again
 * whenever the setting changes. */
export function followLanguage(b: {
  languageNow(): Promise<{ choice: string; system: Lang; hour12?: boolean }>;
  onEvent(name: "language", cb: (payload: unknown) => void): void;
}) {
  const ask = () =>
    b.languageNow().then((l) => {
      i18n.system = l.system;
      i18n.choice = readChoice(l.choice);
      i18n.hour12 = !!l.hour12;
      document.documentElement.lang = i18n.lang;
    });
  b.onEvent("language", () => void ask().catch(() => {}));
  void ask().catch(() => {});
}

// ---- Switching, live and smooth ----

/** The words on screen fade out, change, and fade back in: the old and the new words are never
 * both showing (only the text fades; cards, colours and the board stay as they are). Only what
 * can be seen fades: `scope` (the view the switch is made in, which covers the rest) and what
 * floats over it (`data-lang-fade`: a toast, the tooltip). Fading the whole page restyled every
 * element of it each frame. */
export function switchLanguage(choice: LangChoice, apply: () => void | Promise<void> = () => {}, scope?: Element | null): Promise<void> {
  // One after the other: a second pick while the first still fades (a quick Space, Enter again)
  // waits for it, so neither cuts the other's fade short or tidies up under it, and the last
  // pick is the one that stays.
  asked = choice;
  const run = switching.then(() => switchNow(choice, apply, scope));
  switching = run.catch(() => {});
  void switching.then(() => {
    if (asked === choice) asked = null;
  });
  return run;
}
let switching: Promise<void> = Promise.resolve();
let asked: LangChoice | null = null;

/** The language last picked while a switch is still under way (null when none is): what a new
 * pick is compared with, since the setting changes only once the words have faded out. */
export function askedLanguage(): LangChoice | null {
  return asked;
}

async function switchNow(choice: LangChoice, apply: () => void | Promise<void>, scope?: Element | null) {
  const before = i18n.lang;
  const nextLang: Lang = choice === "system" ? i18n.system : choice;
  if (before === nextLang || typeof document === "undefined") {
    i18n.choice = choice;
    await apply();
    return;
  }
  const root = document.documentElement;
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const parts = [...new Set<Element>([scope ?? document.body, ...document.querySelectorAll("[data-lang-fade]")])];
  for (const p of parts) p.classList.add("lang-swap");
  const fade = (from: number, to: number, ms: number) =>
    Promise.all(
      parts.map((p) =>
        p.animate([{ "--lang-a": from } as Keyframe, { "--lang-a": to } as Keyframe], { duration: ms, easing: "ease-in-out", fill: "forwards" }).finished.catch(() => {}),
      ),
    );
  await fade(1, 0, reduced ? 60 : 120);
  // What the view covers (`data-lang-later`: the board) takes its new words now, but is laid out
  // again only once the new words have faded in: the whole board laid out anew made a pause
  // between the fade out and the fade in. (Held at its size meanwhile, so nothing moves; let go
  // at once if the view starts to close.)
  const held = scope ? [...document.querySelectorAll<HTMLElement>("[data-lang-later]")].filter((el) => !scope.contains(el)) : [];
  const sizes = held.map((el) => [el.offsetWidth, el.offsetHeight]);
  held.forEach((el, i) => {
    el.style.containIntrinsicSize = `${sizes[i][0]}px ${sizes[i][1]}px`;
    el.style.contentVisibility = "hidden";
  });
  let letGo = () => {
    letGo = () => {};
    for (const el of held) el.style.contentVisibility = el.style.containIntrinsicSize = "";
  };
  if (held.length && scope) {
    const watch = () => {
      if (!scope.isConnected || scope.closest("[inert]")) letGo();
      else if (held[0].style.contentVisibility) requestAnimationFrame(watch);
    };
    requestAnimationFrame(watch);
  }
  i18n.choice = choice;
  root.lang = nextLang;
  await apply();
  // Two frames: Svelte has written the new words and the browser has laid them out.
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  // What isn't drawn by Svelte (the tooltip showing now) takes its new words too.
  window.dispatchEvent(new Event("dobo-language"));
  await fade(0, 1, reduced ? 80 : 200);
  letGo();
  for (const p of parts) {
    p.getAnimations().forEach((a) => {
      const kf = (a.effect as KeyframeEffect | null)?.getKeyframes?.()[0] as Record<string, unknown> | undefined;
      if (kf && "--lang-a" in kf) a.cancel();
    });
    p.classList.remove("lang-swap");
  }
}
