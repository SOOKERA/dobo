// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// French for the board, header, entry line and views (see fr.ts).
import type { Entry } from "../i18n.svelte";

export const app = {
  // ---- Times and dates ----
  "Yesterday {time}": "Hier {time}",
  "Today {time}": "Aujourd’hui {time}",
  "Tomorrow {time}": "Demain {time}",
  or: "ou",

  // ---- The board (App) ----
  "Panel width, {n} pixels. Drag, or use the arrow keys.": "Largeur du panneau, {n} pixels. Faites glisser, ou utilisez les flèches.",
  "Drag to change the width": "Faites glisser pour changer la largeur",
  "Couldn’t open your notes: {error}": "Impossible d’ouvrir vos notes : {error}",
  Picture: "Image",
  "No notes here yet. Type a line with #name to start one.": "Pas encore de note ici. Écrivez une ligne avec #nom pour en commencer une.",
  "Drop to put it in the Inbox": "Déposez ici pour « À trier »",
  "Drop on this note": "Déposez sur cette note",
  "Change background…": "Changer le fond…",
  Inbox: "À trier",

  // ---- Header ----
  All: "Tout",
  Work: "Travail",
  Personal: "Perso",
  Space: "Espace",
  "Look back on today": "Bilan du jour",
  Settings: "Paramètres",
  "Always on top": "Toujours au premier plan",
  On: "Oui",
  Off: "Non",
  Light: "Clair",
  Dark: "Sombre",
  "Follow Windows": "Comme Windows",
  "By hours": "Selon l’heure",
  light: "clair",
  dark: "sombre",
  "Switch to light": "Passer en clair",
  "Switch to dark": "Passer en sombre",
  "Switch to light (now: {state})": "Passer en clair (actuellement : {state})",
  "Switch to dark (now: {state})": "Passer en sombre (actuellement : {state})",
  "{state}, like Windows": "{state}, comme Windows",
  "{state}, by the hour": "{state}, selon l’heure",
  Style: "Style",
  "New note": "Nouvelle note",
  "New note, empty or from a template": "Nouvelle note, vide ou d’après un modèle",
  "Tidy the inbox, {n} waiting": "Faire le tri, {n} en attente",
  "Tidy the inbox ({n} waiting)": "Faire le tri ({n} en attente)",
  "The inbox is clear": "Plus rien à trier",
  Search: "Rechercher",
  "More: {list}": "Plus : {list}",
  More: "Plus",
  "Always on top · on": "Toujours au premier plan · activé",
  "Always on top · off": "Toujours au premier plan · désactivé",
  Minimise: "Réduire",
  "Close to the tray": "Fermer dans la zone de notification",
  "Close (keeps running in the tray)": "Fermer (reste actif dans la zone de notification)",
  "Keep the panel open": "Garder le panneau ouvert",
  "Keep the panel open · on": "Garder le panneau ouvert · activé",
  "Keep the panel open · off": "Garder le panneau ouvert · désactivé",
  "Hide the panel": "Masquer le panneau",
  Hide: "Masquer",

  // ---- Entry line ----
  "New note “{name}”": "Nouvelle note « {name} »",
  "Notes for #{tag}": "Notes pour #{tag}",
  "{space} picks · {arrows} move · {esc} closes": "{space} choisit · {arrows} pour bouger · {esc} ferme",
  "{arrows} to choose · {esc} closes": "{arrows} pour choisir · {esc} ferme",
  "{n} notes match": { one: "{n} note correspond", other: "{n} notes correspondent" },
  "New task or idea": "Nouvelle tâche ou idée",
  "Jot something down": "Notez quelque chose",
  Task: "Tâche",
  Idea: "Idée",
  "Deadline: rings early, then more often": "Échéance : sonne en avance, puis plus souvent",
  "Reminds you at the time · end with ! for a deadline": "Rappel à l’heure dite · finissez par ! pour une échéance",
  "Into {title}": "Dans {title}",
  "Back to the Inbox": "Revenir à « À trier »",
  "Add a time like “15h” or “tomorrow 9am”, and #note to pick where it goes": "Une heure comme « 15h » ou « demain 9h », #note pour la ranger",
  Now: "Maintenant",

  // ---- View header ----
  Back: "Retour",
  "You are here": "Vous êtes ici",
  "Go to {label}": "Aller à {label}",
  Close: "Fermer",

  // ---- Dobo's own updates (the board's message) ----
  "{app} {version} is ready. It starts the next time {app} opens.": "{app} {version} est prêt. Il démarre à la prochaine ouverture de {app}.",

  // ---- The terms of use (first start) ----
  "Terms of use": "Conditions d’utilisation",
  "{app} now has terms of use": "{app} a maintenant des conditions d’utilisation",
  "Please read them: {app} carries on as soon as you agree. Your notes stay as they are.":
    "Prenez le temps de les lire : {app} continue dès que vous les acceptez. Vos notes restent telles quelles.",
  "Before you start, please read {app}’s terms of use. {app} starts as soon as you agree.":
    "Avant de commencer, lisez les conditions d’utilisation de {app}. {app} démarre dès que vous les acceptez.",
  "{app} couldn’t save that you agreed: {error}": "{app} n’a pas pu enregistrer votre accord : {error}",
  "Quit {app}": "Quitter {app}",
  "Agree and continue": "Accepter et continuer",
} satisfies Record<string, Entry>;
