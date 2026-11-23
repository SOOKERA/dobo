// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Searching Settings: type what you need ("dark", "raccourci", "backup") and only the matching
// settings stay, under their section titles. The text searched is what Settings shows (so it
// never drifts from the screen) plus a few other words people use for each section, English
// and French.

/** Other words for each section (by its title), lower case, accents optional. */
export const SYNONYMS: Record<string, string[]> = {
  appearance: ["dark", "light", "theme", "night", "day mode", "dark mode", "colour", "color", "black", "sombre", "clair", "nuit", "jour", "theme", "apparence", "mode sombre", "mode clair", "couleur", "noir", "language", "langue", "francais", "french", "english", "anglais", "lang"],
  window: ["width", "wide", "narrow", "dock", "docked", "floating", "float", "on top", "always on top", "edge", "side", "left", "right", "peek", "panel", "fenetre", "largeur", "bord", "cote", "flottant", "ancre", "au-dessus", "premier plan", "panneau", "gauche", "droite"],
  notes: ["size", "zoom", "bigger", "smaller", "small", "normal", "large", "compact", "comfortable", "text", "font", "date", "dates", "smart", "taille", "police", "texte", "plus grand", "petite", "petites", "normale", "normales", "grande", "grandes", "compacte", "compactes", "confortable", "confortables", "date"],
  archive: ["archive", "archived", "archives", "archiver", "archivee", "archivees", "archive note", "put away", "hidden notes", "old notes", "deleted notes", "trash", "bin", "notes rangees", "ranger", "anciennes notes", "notes cachees", "corbeille"],
  "my day": ["day", "morning", "evening", "night", "start", "end", "bedtime", "journee", "matin", "soir", "debut", "fin", "ma journee"],
  screen: ["monitor", "display", "screens", "second screen", "ecran", "moniteur", "affichage"],
  "work hours": ["work", "personal", "office", "job", "schedule", "space", "travail", "bureau", "perso", "personnel", "horaires", "heures de travail", "espace"],
  reminders: ["notification", "notifications", "nag", "snooze", "alarm", "alert", "deadline", "rhythm", "ring", "tidy", "inbox", "rappel", "rappels", "alarme", "alerte", "echeance", "sonner", "ranger", "boite", "relance", "trier", "tri", "a trier"],
  keyboard: ["shortcut", "shortcuts", "hotkey", "hotkeys", "keys", "key", "keybinding", "raccourci", "raccourcis", "clavier", "touche", "touches"],
  backups: ["backup", "back up", "restore", "save", "copy", "export", "sauvegarde", "sauvegarder", "restaurer", "copie", "enregistrer"],
  help: ["help", "bug", "report", "problem", "crash", "feedback", "support", "contact", "email", "signaler", "probleme", "plantage", "rapport", "assistance", "tour", "tutorial", "guide", "intro", "introduction", "how to", "learn", "gesture", "gestures", "drag", "f1", "aide", "visite", "didacticiel", "tutoriel", "apprendre", "geste", "gestes", "glisser", "comment", "step", "steps", "subtask", "subtasks", "etape", "etapes", "sous-tache", "sous-taches", "strip", "bandeau", "inbox", "a trier"],
  about: ["version", "portable", "installed", "install", "installer", "data", "data folder", "folder", "where", "location", "usb", "privacy", "private", "internet", "online", "offline", "network", "a propos", "installe", "installee", "installation", "donnees", "dossier", "dossier des donnees", "emplacement", "ou sont", "cle usb", "confidentialite", "vie privee", "reseau", "en ligne", "hors ligne", "licence", "license", "gpl", "open source", "free software", "sookera", "credits", "logiciel libre"],
  updates: ["update", "updates", "upgrade", "new version", "latest", "version", "download", "restart", "check for updates", "auto update", "automatic", "mise a jour", "mises a jour", "mettre a jour", "nouvelle version", "telecharger", "redemarrer", "automatique", "maj"],
  start: ["startup", "boot", "launch", "autostart", "login", "sticky", "sticky notes", "import", "demarrage", "demarrer", "lancer", "lancement", "ouverture", "importer", "pense-bete"],
};

/** Lower case without accents ("Écran" → "ecran"). */
export function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** The words typed, folded; short connecting words ignored when there are others. */
export function terms(q: string): string[] {
  const w = fold(q)
    .split(/[\s,;·]+/)
    .map((x) => x.trim())
    .filter(Boolean);
  const keep = w.filter((x) => !["the", "a", "an", "of", "to", "le", "la", "les", "de", "du", "des", "un", "une"].includes(x));
  return keep.length ? keep : w;
}

export interface Group {
  /** The section's title, as shown ("Work hours", "Heures de travail"). */
  section: string;
  /** Its English name, when the title shows in another language (finds its other words). */
  key?: string;
  /** Everything the group shows, as text. */
  text: string;
}

export interface Found {
  /** One per group: stays visible. */
  show: boolean[];
  /** The words to mark in what stays. */
  marks: string[];
  any: boolean;
}

/**
 * Which groups match. A section whose title or other words match keeps all its groups;
 * otherwise a group stays when every typed word is in its text, its section's title or its
 * section's other words. A phrase from the other words ("on top", "mode sombre") counts as
 * a whole too.
 */
export function matchSettings(q: string, groups: Group[]): Found {
  const t = terms(q);
  if (!t.length) return { show: groups.map(() => true), marks: [], any: true };
  const whole = fold(q).trim();
  const show = groups.map((g) => {
    const sec = fold(g.section);
    const other = SYNONYMS[fold(g.key ?? g.section)] ?? [];
    const words = [sec, ...other].join(" | ");
    if (other.some((o) => o.length > 3 && o.includes(" ") && whole.includes(o))) return true;
    if (t.every((x) => words.includes(x))) return true;
    const hay = `${fold(g.text)} | ${words}`;
    return t.every((x) => hay.includes(x));
  });
  return { show, marks: t.filter((x) => x.length > 1), any: show.some(Boolean) };
}

// ---- On the page ----

/** Splits a section's children into groups: a line, a heading with what follows it, the
 * notes under a line. */
export function groupsOf(section: HTMLElement): HTMLElement[][] {
  const out: HTMLElement[][] = [];
  let cur: HTMLElement[] | null = null;
  let underH4 = false;
  for (const el of Array.from(section.children) as HTMLElement[]) {
    if (el.tagName === "H3") continue;
    const starts = el.tagName === "H4" || ((el.classList.contains("line") || el.classList.contains("modes")) && !underH4);
    if (el.tagName === "H4") underH4 = true;
    if (starts || !cur) {
      cur = [el];
      out.push(cur);
    } else cur.push(el);
  }
  return out;
}

/** Marks the typed words in what is visible (CSS Custom Highlight API, no change to the page). */
export function highlight(root: HTMLElement, marks: string[]) {
  const reg = (globalThis as { CSS?: { highlights?: Map<string, unknown> } }).CSS?.highlights;
  const H = (globalThis as { Highlight?: new (...r: Range[]) => unknown }).Highlight;
  if (!reg || !H) return;
  if (!marks.length) {
    reg.delete("settings-find");
    return;
  }
  const ranges: Range[] = [];
  const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => ((n.parentElement?.closest(".gone, .find-field, .vh, input, textarea") ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT)),
  });
  for (let n = walk.nextNode() as Text | null; n; n = walk.nextNode() as Text | null) {
    const raw = n.data;
    // fold() keeps one character per character for Latin text, so indexes line up.
    const f = fold(raw);
    if (f.length !== raw.length) continue;
    for (const m of marks) {
      for (let i = f.indexOf(m); i >= 0; i = f.indexOf(m, i + m.length)) {
        const r = document.createRange();
        r.setStart(n, i);
        r.setEnd(n, i + m.length);
        ranges.push(r);
      }
    }
  }
  reg.set("settings-find", new H(...ranges));
}
