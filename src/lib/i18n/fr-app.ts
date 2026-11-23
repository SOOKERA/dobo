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
  Copy: "Copier",
  "Copy link": "Copier le lien",
  // (A right-click on a note or a line: its menu.)
  "Copy text": "Copier le texte",
  "Copy as text": "Copier comme texte",
  "Idea options": "Options de l’idée",
  "Step options": "Options de l’étape",

  // ---- The board (App) ----
  "Panel width, {n} pixels. Drag, or use the arrow keys.": "Largeur du panneau, {n} pixels. Faites glisser, ou utilisez les flèches.",
  "Drag to change the width": "Faites glisser pour changer la largeur",
  "Couldn’t open your notes: {error}": "Impossible d’ouvrir vos notes : {error}",
  "That change couldn’t be saved: {error}": "Ce changement n’a pas pu être enregistré : {error}",
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
  "Tidy the Inbox, {n} waiting": "Faire le tri, {n} en attente",
  "Tidy the Inbox ({n} waiting)": "Faire le tri ({n} en attente)",
  "The Inbox is clear": "Plus rien à trier",
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
  "Jot something down": "Qu’avez-vous en tête ?",
  Task: "Tâche",
  Idea: "Idée",
  "Deadline: rings early, then more often": "Échéance : sonne en avance, puis de plus en plus souvent",
  "Reminds you at the time · end with\u00a0! for a deadline": "Rappel à l’heure dite · finissez par « ! » pour une échéance",
  "Deadline: end with\u00a0!": "Échéance : finissez par « ! »",
  "Into {title}": "Dans {title}",
  "Back to the Inbox": "Revenir à « À trier »",
  "Add a time like “3pm” or “tomorrow 9am”, and #note to pick where it goes": "Une heure comme « 15h » ou « demain 9h », #note pour la ranger",
  "“3pm”, “tomorrow 9am”, #note": "« 15h », « demain 9h », #note",
  Now: "Maintenant",

  // ---- View header ----
  Back: "Retour",
  "You are here": "Vous êtes ici",
  "Go to {label}": "Aller à {label}",
  Close: "Fermer",

  // ---- Dobo's own updates (the board's message) ----
  "{app} {version} is ready. It starts the next time {app} opens.": "{app} {version} est prêt. Vous l’aurez à la prochaine ouverture de {app}.",

  // ---- The terms of use (first start) ----
  "Terms of use": "Conditions d’utilisation",
  "{app} now has terms of use": "{app} a maintenant des conditions d’utilisation",
  "Please read them: {app} carries on as soon as you agree. Your notes stay as they are.":
    "Prenez le temps de les lire : {app} continue dès que vous les acceptez. Vos notes restent telles quelles.",
  "Before you start, please read {app}’s terms of use. {app} starts as soon as you agree.":
    "Avant de commencer, lisez les conditions d’utilisation de {app}. {app} démarre dès que vous les acceptez.",
  "{app} couldn’t save that you agreed: {error}": "{app} n’a pas pu enregistrer votre accord : {error}",
  // (Said in French only: the terms are shown in French as a courtesy, the English counts.)
  "A courtesy translation: the English version is the binding one.": "Traduction de courtoisie : c’est la version anglaise qui fait foi.",
  "Read the English original": "Lire l’original en anglais",
  "The text below is in English; it is the binding version.": "Le texte qui suit est en anglais ; c’est la version qui fait foi.",
  "Read the French translation": "Lire la traduction en français",
  "Quit {app}": "Quitter {app}",
  "Agree and continue": "Accepter et continuer",
  // ---- The recovery screen (the notes didn't open at start) ----
  "Your notes file is damaged": "Le fichier de vos notes est endommagé",
  "Your notes can be put back": "Vos notes peuvent être récupérées",
  "Another program is using your notes": "Un autre programme utilise vos notes",
  "Your notes can’t be saved": "Vos notes ne peuvent pas être enregistrées",
  "Your notes didn’t open": "Vos notes ne se sont pas ouvertes",
  "The board couldn’t start": "Le tableau n’a pas pu démarrer",
  "Try again. If it still doesn’t start, quit {app} and open it again. Your notes are kept as they are.":
    "Réessayez. S’il ne démarre toujours pas, quittez {app} et rouvrez-le. Vos notes restent telles quelles.",
  "{app} tries to start the board again.": "{app} essaie de nouveau de démarrer le tableau.",
  "An earlier start put your notes aside as {name}, in the data folder, because they couldn’t be read. They read whole now: put them back to carry on where you left off.":
    "Un démarrage précédent a mis vos notes de côté sous le nom {name}, dans le dossier des données, parce qu’elles ne pouvaient pas être lues. Elles se lisent en entier maintenant : récupérez-les pour reprendre là où vous en étiez.",
  "{app} couldn’t read {name} whole. Nothing has been changed: whatever you choose, the file is kept as it is in the data folder, under a new name.":
    "{app} n’a pas pu lire {name} en entier. Rien n’a été modifié : quel que soit votre choix, le fichier est gardé tel quel dans le dossier des données, sous un nouveau nom.",
  "{app} couldn’t read it, so it put it aside as it was, as {name} in its data folder. Nothing was deleted.":
    "{app} n’a pas pu le lire : il l’a mis de côté tel quel, sous le nom {name}, dans son dossier des données. Rien n’a été supprimé.",
  "Something else has {name} open (often an antivirus, a sync or a backup tool), so {app} can’t open it now. Your notes are fine: wait a moment, then try again.":
    "{name} est ouvert par autre chose (souvent un antivirus, un outil de synchronisation ou de sauvegarde) : {app} ne peut pas l’ouvrir pour l’instant. Vos notes ne risquent rien : patientez un instant, puis réessayez.",
  "Windows lets {app} read {name} but not change it: the file or its folder is read-only, or this account can’t write there. Your notes are fine. Untick Read-only in the file’s Properties (or ask whoever looks after this computer), then try again.":
    "Windows laisse {app} lire {name}, mais pas le modifier : le fichier ou son dossier est en lecture seule, ou ce compte ne peut pas y écrire. Vos notes ne risquent rien. Décochez « Lecture seule » dans les Propriétés du fichier (ou demandez à la personne qui s’occupe de cet ordinateur), puis réessayez.",
  "{app} couldn’t open {name}. Try again first. If it still won’t open, put back the newest backup or start with an empty board: the file is put aside either way, never deleted.":
    "{app} n’a pas pu ouvrir {name}. Réessayez d’abord. S’il ne s’ouvre toujours pas, restaurez la sauvegarde la plus récente ou commencez avec un tableau vide : le fichier est mis de côté dans les deux cas, jamais supprimé.",
  "Try again": "Réessayer",
  "{app} opens your notes as soon as it can.": "{app} ouvre vos notes dès qu’il le peut.",
  "{app} checks the file again, once Read-only is unticked.": "{app} vérifie de nouveau le fichier, une fois la lecture seule retirée.",
  "{app} tries to open the file again now.": "{app} essaie de nouveau d’ouvrir le fichier.",
  "{app} reads the file again: a drive that answered badly may read it whole now.":
    "{app} relit le fichier : un lecteur qui a mal répondu peut le lire en entier maintenant.",
  "Use the repaired notes": "Utiliser les notes réparées",
  "Everything {app} could still read, copied into a new file. The damaged one is kept in the data folder.":
    "Tout ce que {app} a encore pu lire, copié dans un nouveau fichier. Le fichier endommagé est gardé dans le dossier des données.",
  "Put my notes back": "Récupérer mes notes",
  "As they were when they were put aside.": "Telles qu’elles étaient quand elles ont été mises de côté.",
  "Put back the newest backup": "Restaurer la sauvegarde la plus récente",
  "From {when}: your notes and pictures as they were then.": "Du {when} : vos notes et images telles qu’elles étaient alors.",
  "From {when}: {notes} and {lines}, with their pictures, as they were then.": "Du {when} : {notes} et {lines}, avec leurs images, telles qu’elles étaient alors.",
  "Everything {app} could still read, copied into a new file: {notes} and {lines}. The damaged one is kept in the data folder.":
    "Tout ce que {app} a encore pu lire, copié dans un nouveau fichier : {notes} et {lines}. Le fichier endommagé est gardé dans le dossier des données.",
  "{n} lines": { one: "{n} ligne", other: "{n} lignes" },
  "No backup was found.": "Aucune sauvegarde trouvée.",
  "Start with an empty board": "Commencer avec un tableau vide",
  "Your settings come along when they can be read. The old file stays in the data folder.":
    "Vos paramètres suivent s’ils peuvent être lus. L’ancien fichier reste dans le dossier des données.",
  "Opening the repaired notes…": "Ouverture des notes réparées…",
  "Putting your notes back…": "Récupération de vos notes…",
  "Putting the backup back…": "Restauration de la sauvegarde…",
  "Starting an empty board…": "Ouverture d’un tableau vide…",
  "Looking again…": "Nouvel essai…",
  "It’s still in use. Try again in a moment.": "Il est toujours utilisé. Réessayez dans un instant.",
  "It still can’t be changed.": "Il ne peut toujours pas être modifié.",
  "It still won’t open.": "Il ne s’ouvre toujours pas.",
  "{app} can’t keep its notes next to itself": "{app} ne peut pas garder ses notes dans son propre dossier",
  "This copy of {app} keeps its notes in {name}, next to it, but Windows doesn’t let it write there (a read-only drive or folder). So that your notes aren’t split, {app} asks first. To keep them there, quit, then move {app} to a folder it can write in, such as Documents.":
    "Cette copie de {app} garde ses notes dans {name}, à côté d’elle, mais Windows ne la laisse pas y écrire (un lecteur ou un dossier en lecture seule). Pour que vos notes ne soient pas séparées, {app} demande d’abord. Pour les garder là, quittez, puis placez {app} dans un dossier où il peut écrire, comme Documents.",
  "This copy of {app} would keep its notes in {name}, next to it, but Windows doesn’t let it write there (a read-only drive or folder, or Program Files). To keep them there, quit, then move {app} to a folder it can write in, such as Documents.":
    "Cette copie de {app} garderait ses notes dans {name}, à côté d’elle, mais Windows ne la laisse pas y écrire (un lecteur ou un dossier en lecture seule, ou Program Files). Pour les garder là, quittez, puis placez {app} dans un dossier où il peut écrire, comme Documents.",
  "Use the usual folder": "Utiliser le dossier habituel",
  "Your notes stay there from now on, apart from the notes next to {app}.": "Vos notes y restent désormais, séparément des notes qui sont à côté de {app}.",
  "Your notes stay there from now on, even once {app} can write next to itself.":
    "Vos notes y restent désormais, même quand {app} pourra écrire dans son propre dossier.",
  "Opening the notes…": "Ouverture des notes…",
  "What to do": "Que faire",
  "What Windows or SQLite said: {error}": "Message de Windows ou de SQLite : {error}",
  "Open the data folder": "Ouvrir le dossier des données",
  "Your notes from {when} are back. The damaged file is kept as {name}.": "Vos notes du {when} sont de retour. Le fichier endommagé est gardé sous le nom {name}.",
  "Your notes from {when} are back.": "Vos notes du {when} sont de retour.",
  "A new board. The damaged file is kept as {name} in the data folder.": "Un nouveau tableau. Le fichier endommagé est gardé sous le nom {name} dans le dossier des données.",
  "Your notes are back, repaired. The damaged file is kept as {name}.": "Vos notes sont de retour, réparées. Le fichier endommagé est gardé sous le nom {name}.",
  "Your notes are back, repaired.": "Vos notes sont de retour, réparées.",
  "Your notes are back.": "Vos notes sont de retour.",
} satisfies Record<string, Entry>;
