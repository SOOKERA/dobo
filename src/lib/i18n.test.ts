// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { afterEach, describe, expect, it } from "vitest";
import { fr, parts } from "./i18n/fr";
import { en } from "./i18n/en";
import { cap, duration, fileSize, frType, i18n, keysText, ordinal, t, tIn, tMaybe, type Entry, type Key } from "./i18n.svelte";
import { clockDate, formatDue, formatDueLong } from "./time";
import { repeatLabel } from "./repeat";
import { describeChange, OUR_STYLES, OUR_THEMES, themeShownName } from "./looks";
import { backWords } from "./undoWords";
import { findNoteForTag, parseCapture } from "./capture";
import type { Note } from "./types";

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
const forms = (e: Entry) => (typeof e === "string" ? [e] : [e.one, e.other]);
/** The placeholders a form must have: all of the key's, except that a plural's "one" may leave
 * out its number ("Your note", not "Your 1 note"). */
const expected = (e: Entry, f: string, want: string[]) => (typeof e !== "string" && f === e.one && !f.includes("{n}") ? want.filter((p) => p !== "n") : want);

afterEach(() => {
  i18n.choice = "en";
});

describe("the French dictionary", () => {
  it("has a French value for every key, with the same placeholders", () => {
    for (const [key, value] of Object.entries(fr) as [Key, Entry][]) {
      const want = placeholders(key);
      for (const f of forms(value)) {
        expect(f.trim(), key).not.toBe("");
        expect(placeholders(f), `${key} → ${f}`).toEqual(expected(value, f, want));
      }
    }
  });

  it("English overrides keep the key's placeholders, and plurals have both forms", () => {
    for (const [key, value] of Object.entries(en) as [Key, Entry][]) {
      expect(key in fr, key).toBe(true);
      const want = placeholders(key.replace(/ \([a-z ]+\)$/, ""));
      for (const f of forms(value!)) expect(placeholders(f), key).toEqual(expected(value!, f, want));
      if (typeof value !== "string") expect(typeof fr[key], `${key} is a plural in English, so in French too`).toBe("object");
    }
    for (const [key, value] of Object.entries(fr)) if (typeof value !== "string") expect(key in en, `${key} needs an English plural`).toBe(true);
  });

  it("a key found in two parts says the same thing in both", () => {
    const seen = new Map<string, string>();
    for (const [part, dict] of Object.entries(parts)) {
      for (const [key, value] of Object.entries(dict as Record<string, Entry>)) {
        const v = JSON.stringify(value);
        if (seen.has(key)) expect(v, `"${key}" in ${part}`).toBe(seen.get(key));
        seen.set(key, v);
      }
    }
  });

  it("keeps our style and theme names short enough for one line on their cards", () => {
    for (const s of [...OUR_STYLES, ...OUR_THEMES]) expect(tIn("fr", s.name as Key).length, s.name).toBeLessThanOrEqual(15);
  });

  it("uses French quotes and apostrophes, not English ones", () => {
    for (const [key, value] of Object.entries(fr)) for (const f of forms(value as Entry)) expect(f, key).not.toMatch(/[“”"']/);
  });
});

describe("t()", () => {
  it("English is the key itself, placeholders filled", () => {
    expect(t("Into {title}", { title: "Hub" })).toBe("Into Hub");
    expect(t("{n} ideas", { n: 1 })).toBe("1 idea");
    expect(t("{n} ideas", { n: 3 })).toBe("3 ideas");
  });

  it("French: plurals (0 and 1 are singular), typography", () => {
    i18n.choice = "fr";
    expect(t("{n} ideas", { n: 0 })).toBe("0 idée");
    expect(t("{n} ideas", { n: 1 })).toBe("1 idée");
    expect(t("{n} tasks", { n: 3 })).toBe("3 tâches");
    expect(t("Couldn’t back up: {error}", { error: "x" })).toBe("Sauvegarde impossible : x");
    expect(t("Is “{text}” really done?", { text: "Payer" })).toBe("« Payer », vraiment fini ?");
  });

  it("tMaybe translates our names and leaves yours", () => {
    i18n.choice = "fr";
    expect(tMaybe("Cork board")).toBe("Panneau liège");
    expect(tMaybe("My lovely cork")).toBe("My lovely cork");
  });

  it("frType keeps a quoted « ! » whole", () => {
    expect(frType("avec « ! » devant")).toBe("avec « ! » devant");
    expect(frType("tapez « ? » ici ?")).toBe("tapez « ? » ici ?");
  });

  it("frType puts the right spaces", () => {
    expect(frType("Note : « x » ; ok ? oui !")).toBe("Note : « x » ; ok ? oui !");
  });
});

describe("keys, dates and numbers", () => {
  const thu = new Date(2026, 10, 19, 19, 2);

  it("names keys as Windows does", () => {
    expect(keysText("Ctrl+Shift+Space")).toBe("Ctrl+Shift+Space");
    i18n.choice = "fr";
    expect(keysText("Ctrl+Shift+Space")).toBe("Ctrl+Maj+Espace");
    expect(keysText("Esc")).toBe("Échap");
    expect(keysText("Ctrl++")).toBe("Ctrl++");
  });

  it("dates and times", () => {
    expect(clockDate(thu)).toBe("Thu 19 Nov · 19:02");
    expect(formatDueLong(new Date(2026, 10, 20, 9, 0).getTime(), thu)).toBe("Tomorrow 09:00");
    i18n.choice = "fr";
    expect(clockDate(thu)).toBe("jeu. 19 nov. · 19:02");
    expect(clockDate(thu, true)).toBe("jeu. 19 · 19:02");
    expect(formatDueLong(new Date(2026, 10, 20, 9, 0).getTime(), thu)).toBe("Demain 09:00");
    expect(formatDue(new Date(2026, 10, 18, 18, 0).getTime(), thu)).toBe("Hier 18:00");
    expect(formatDue(new Date(2026, 10, 23, 9, 0).getTime(), thu)).toBe("lun. 09:00");
  });

  it("ordinals, durations, sizes", () => {
    expect(ordinal(3)).toBe("3rd");
    expect(duration(90)).toBe("1 h 30 min");
    i18n.choice = "fr";
    expect(ordinal(1)).toBe("1er");
    expect(ordinal(3)).toBe("3e");
    expect(duration(10)).toBe("10 min");
    expect(duration(60)).toBe("1 h");
    expect(duration(90)).toBe("1 h 30");
    expect(fileSize(1_572_864)).toBe("1,5 Mo");
    expect(cap("jeudi")).toBe("Jeudi");
  });

  it("repeats and look changes read naturally in French", () => {
    i18n.choice = "fr";
    expect(repeatLabel({ freq: "week", interval: 1, days: [1], ends: { kind: "never" } })).toBe("Chaque lundi");
    expect(repeatLabel({ freq: "month", interval: 1, monthday: 1, ends: { kind: "after", count: 4 } })).toBe("Chaque mois, le 1er · 4 fois");
    expect(repeatLabel({ freq: "day", interval: 3, ends: { kind: "never" } })).toBe("Tous les 3 jours");
    expect(describeChange({ bg: "cork", colour: "Natural", held: "auto" }, { bg: "cork", colour: "Honey", held: "pins" })).toBe("couleur Miel · fixées par punaises");
  });
});

describe("typing works the same in either language", () => {
  const thu = new Date(2026, 10, 19, 10, 0);
  const notes = [
    { id: 1, title: "Inbox", archived: false },
    { id: 2, title: "Hub", archived: false },
  ] as Note[];

  for (const lang of ["en", "fr"] as const) {
    it(`French and English times parse with the app in ${lang}`, () => {
      i18n.choice = lang;
      expect(parseCapture("Call Chanel 15h", "task", thu).due?.getHours()).toBe(15);
      const d = parseCapture("Payer demain 9h", "task", thu).due!;
      expect([d.getDate(), d.getHours()]).toEqual([20, 9]);
      const e = parseCapture("Pay tomorrow 9am", "task", thu).due!;
      expect([e.getDate(), e.getHours()]).toEqual([20, 9]);
      expect(parseCapture("Milk #hub", "task", thu).tag).toBe("hub");
      expect(findNoteForTag("hub", notes)?.id).toBe(2);
      expect(findNoteForTag("inbox", notes)?.id).toBe(1);
    });
  }

  it("the Inbox is found by the name it shows in French", () => {
    i18n.choice = "fr";
    expect(findNoteForTag("à-trier", notes)?.id).toBe(1);
    expect(findNoteForTag("trier", notes)?.id).toBe(1);
  });
});

describe("French wording decisions", () => {
  it("repeat days read « Les lun. et jeu. »", () => {
    i18n.choice = "fr";
    expect(repeatLabel({ freq: "week", interval: 1, days: [1, 4], ends: { kind: "never" } })).toBe("Les lun. et jeu.");
    expect(repeatLabel({ freq: "week", interval: 1, days: [1, 3, 5], ends: { kind: "never" } })).toBe("Les lun., mer. et ven.");
    i18n.choice = "en";
    expect(repeatLabel({ freq: "week", interval: 1, days: [1, 3, 5], ends: { kind: "never" } })).toBe("Every Mon, Wed and Fri");
  });

  it("undo and redo say what happened in plain words", () => {
    i18n.choice = "fr";
    expect(backWords("Done", 0)).toBe("Tâche remise à faire");
    expect(backWords("Done", 1)).toBe("Tâche de nouveau cochée");
    expect(backWords(() => "Liège", 0)).toBe(frType("Annulé : Liège"));
    i18n.choice = "en";
    expect(backWords("Not done", 0)).toBe("Task ticked again");
  });

  it("a theme of yours that is exactly one of ours shows our name, translated", () => {
    i18n.choice = "fr";
    const ours = OUR_THEMES.find((x) => x.id === "plain")!;
    expect(themeShownName({ name: ours.name, light: ours.light, dark: ours.dark })).toBe(tMaybe(ours.name));
    expect(themeShownName({ name: ours.name, light: ours.light, dark: OUR_THEMES[0].dark })).toBe(ours.name);
    expect(tIn("fr", "Your {n} sticky notes are on the board now.", { n: 1 })).toBe("Votre note est maintenant sur le tableau.");
    expect(tIn("fr", "Your {n} sticky notes are on the board now.", { n: 3 })).toBe("Vos 3 notes sont maintenant sur le tableau.");
    expect(tIn("fr", "Bring in your {n} Windows Sticky Notes?", { n: 1 })).toBe("Récupérer votre note de Pense-bêtes ?");
    expect(tIn("fr", "Bring in your {n} Windows Sticky Notes?", { n: 3 })).toBe("Récupérer vos 3 notes de Pense-bêtes ?");
    expect(tIn("en", "Bring in your {n} Windows Sticky Notes?", { n: 1 })).toBe("Bring in your note from Windows Sticky Notes?");
    expect(tIn("en", "Your {n} sticky notes are on the board now.", { n: 1 })).toBe("Your sticky note is on the board now.");
  });
});
