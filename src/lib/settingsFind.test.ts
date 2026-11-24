// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { fold, matchSettings, terms, type Group } from "./settingsFind";

// What Settings shows, roughly (one group per line).
const G: Group[] = [
  { section: "Appearance", text: "Light Dark Follow Windows By hours Dark mode darkens the app and the notes" },
  { section: "Window", text: "Docked to the edge A floating window" },
  { section: "Window", text: "Side of the screen Left Right" },
  { section: "Window", text: "Width Narrow Medium Wide Or drag the panel’s inner edge." },
  { section: "Notes", text: "Note size Also Ctrl+wheel Small Normal Large" },
  { section: "Notes", text: "Dates written in a note Ask Automatic Off" },
  { section: "My day", text: "Starts at Ends at" },
  { section: "Screen", text: "Show on Screen with the mouse One screen" },
  { section: "Work hours", text: "Switch between Work and Personal by the clock" },
  { section: "Reminders", text: "Default rhythm for deadlines End a line with ! to make a deadline." },
  { section: "Reminders", text: "Tidy the Inbox Every day at When things wait" },
  { section: "Keyboard", text: "Open or hide Dobo, from anywhere Ctrl+Shift+Space Change" },
  { section: "Backups", text: "Back up every day A copy of your notes and pictures" },
  { section: "Backups", text: "Your backups Restore…" },
  { section: "Start", text: "Start Dobo with Windows Reminders only ring while it runs" },
  { section: "About", text: "Dobo 1.0.0 Portable Data folder Open folder" },
  { section: "Archive", text: "Archived notes Open the archive" },
];
const sections = (q: string) => [...new Set(G.filter((_, i) => matchSettings(q, G).show[i]).map((g) => g.section))];

describe("Settings search", () => {
  it("shows everything when nothing is typed", () => {
    expect(matchSettings("", G).show.every(Boolean)).toBe(true);
    expect(matchSettings("   ", G).show.every(Boolean)).toBe(true);
  });

  it("finds words people use for a section", () => {
    expect(sections("dark")).toEqual(["Appearance"]);
    expect(sections("theme")).toEqual(["Appearance"]);
    expect(sections("night")).toEqual(["Appearance", "My day"]);
    expect(sections("hotkey")).toEqual(["Keyboard"]);
    expect(sections("shortcut")).toEqual(["Keyboard"]);
    expect(sections("startup")).toEqual(["Start"]);
    expect(sections("boot")).toEqual(["Start"]);
    expect(sections("restore")).toEqual(["Backups"]);
    expect(sections("snooze")).toEqual(["Reminders"]);
    expect(sections("nag")).toEqual(["Reminders"]);
    expect(sections("notification")).toEqual(["Reminders"]);
    expect(sections("dock")).toEqual(["Window"]);
    expect(sections("on top")).toEqual(["Window"]);
  });

  it("finds the archive, English and French", () => {
    for (const q of ["archive", "archived", "archives", "archivées", "archivees", "corbeille", "put away"]) expect(sections(q), q).toEqual(["Archive"]);
  });

  it("finds where the notes are kept (installed or portable)", () => {
    for (const q of ["installed", "portable", "data folder", "version", "données", "emplacement", "a propos", "confidentialité"]) expect(sections(q), q).toEqual(["About"]);
  });

  it("finds Updates, English and French, with or without accents", () => {
    const U: Group[] = [
      ...G,
      // (As Settings shows it: the version line with its state and button, then the switch.)
      { section: "Updates", text: "Dobo 1.2.0 · Installed You’re up to date. Last checked Tue 24 Nov, 10:24. Dobo looks once a day. Check for updates" },
      { section: "Updates", text: "Check for updates automatically A new version is downloaded and checked, then used the next time Dobo starts." },
      { section: "Updates", text: "What’s new What this version brings. Read" },
    ];
    const found = (q: string) => [...new Set(U.filter((_, i) => matchSettings(q, U).show[i]).map((g) => g.section))];
    for (const q of ["update", "updates", "upgrade", "new version", "check for updates", "mise à jour", "mises a jour", "mettre à jour", "nouvelle version", "redémarrer", "up to date", "latest version", "dernière version", "once a day", "une fois par jour"]) expect(found(q), q).toEqual(["Updates"]);
    for (const q of ["what's new", "what’s new", "whats new", "changelog", "release notes", "nouveautés", "nouveautes", "quoi de neuf", "notes de version"]) expect(found(q), q).toEqual(["Updates"]);
    // "version" is in both: the version installed (About) and the new one (Updates).
    expect(found("version")).toEqual(["About", "Updates"]);
  });

  it("finds Help by the steps of a task, English and French, with or without accents", () => {
    const H: Group[] = [...G, { section: "Help", text: "Quick tour Take the tour Keys to know Gestures" }];
    for (const q of ["steps", "subtask", "étapes", "etapes", "sous-tâche", "sous-taches"]) expect(matchSettings(q, H).show.at(-1), q).toBe(true);
  });

  it("finds the note sizes by their names, old and new, in both languages", () => {
    for (const q of ["small", "normal", "large", "compact", "comfortable", "petite", "grandes", "compacte", "confortable"]) expect(sections(q), q).toContain("Notes");
  });

  it("understands French, with or without accents", () => {
    expect(sections("raccourci")).toEqual(["Keyboard"]);
    expect(sections("écran")).toEqual(["Screen"]);
    expect(sections("ecran")).toEqual(["Screen"]);
    expect(sections("sauvegarde")).toEqual(["Backups"]);
    expect(sections("démarrage")).toEqual(["Start"]);
    expect(sections("rappel")).toEqual(["Reminders"]);
    expect(sections("mode sombre")).toEqual(["Appearance"]);
  });

  it("a section's own words keep all its lines; other words keep only the matching lines", () => {
    const window = matchSettings("window", G).show.filter((_, i) => G[i].section === "Window");
    expect(window.every(Boolean)).toBe(true);
    const width = matchSettings("drag", G);
    expect(G.filter((_, i) => width.show[i]).map((g) => g.text.split(" ")[0])).toEqual(["Width"]);
  });

  it("needs every typed word", () => {
    expect(sections("tidy inbox")).toEqual(["Reminders"]);
    expect(matchSettings("tidy inbox", G).show.filter(Boolean).length).toBe(2);
    expect(sections("copy pictures")).toEqual(["Backups"]);
    expect(matchSettings("copy pictures", G).show.filter(Boolean).length).toBe(1);
    expect(matchSettings("zebra dark", G).any).toBe(false);
  });

  it("says when nothing matches", () => {
    const f = matchSettings("zzzz", G);
    expect(f.any).toBe(false);
    expect(f.show.some(Boolean)).toBe(false);
  });

  it("marks the typed words, not the connecting ones", () => {
    expect(matchSettings("the inbox", G).marks).toEqual(["inbox"]);
    expect(terms("Dark, mode")).toEqual(["dark", "mode"]);
    expect(fold("Écran Démarrage")).toBe("ecran demarrage");
  });
});
