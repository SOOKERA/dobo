// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// French for Help › Report a problem (see fr.ts). « Signaler un problème », as in Pako.
import type { Entry } from "../i18n.svelte";

export const report = {
  // ---- The page ----
  "Report a problem": "Signaler un problème",
  "Report…": "Signaler…",
  "Something went wrong? Tell Sookera, in a few words. You see everything that goes with them first.":
    "Un souci ? Dites-le à Sookera en quelques mots. Vous voyez d’abord tout ce qui part avec.",
  "Tell Sookera what went wrong. Nothing goes until you click: everything that goes with your words is shown below.":
    "Dites à Sookera ce qui n’a pas marché. Rien ne part avant votre clic : tout ce qui accompagne vos mots est affiché ci-dessous.",
  "What happened?": "Que s’est-il passé ?",
  "What you did, what you expected, and what happened instead.": "Ce que vous avez fait, ce que vous attendiez, et ce qui s’est passé à la place.",
  "{n} of {max} characters": "{n} caractères sur {max}",
  "Sent with your report": "Envoyé avec votre rapport",
  Windows: "Windows",
  Look: "Apparence",
  "Time zone": "Fuseau horaire",
  "Include recent problems": "Joindre les problèmes récents",
  "The last problems {app} noted: when it had to stop, or couldn’t start. Your user folder, folder and file names and anything in quotes are taken out first, and links keep only their site.":
    "Les derniers problèmes notés par {app} : quand il a dû s’arrêter, ou n’a pas pu démarrer. Votre dossier utilisateur, les noms de dossiers et de fichiers et tout ce qui est entre guillemets sont retirés d’abord, et les liens ne gardent que leur site.",
  "{app} hasn’t noted any problem.": "{app} n’a noté aucun problème.",
  "Show all ({n})": "Tout afficher ({n})",
  "Show fewer": "Afficher moins",
  "{app} couldn’t read what goes with a report. Try again in a moment.": "{app} n’a pas pu lire ce qui accompagne un rapport. Réessayez dans un instant.",
  "Reading…": "Lecture…",
  "Send report": "Envoyer le rapport",
  "Sending…": "Envoi…",
  "Send by email": "Envoyer par e-mail",
  "Copy details": "Copier les détails",
  "Send report goes to Sookera’s report service. Reports are kept private and used only to fix {app}; your IP address isn’t stored.":
    "Envoyer le rapport passe par le service de rapports de Sookera. Les rapports restent privés et servent uniquement à corriger {app} ; votre adresse IP n’est pas conservée.",
  "An idea or a question instead?": "Plutôt une idée ou une question ?",
  "Share it on GitHub Discussions": "Partagez-la sur GitHub Discussions",

  // ---- What it says ----
  "Say what happened first.": "Dites d’abord ce qui s’est passé.",
  "Sent. Thank you.": "Envoyé. Merci !",
  "Your report is #{ref}. Mention it if you write to {email} about the same problem.":
    "Votre rapport porte le numéro {ref}. Indiquez-le si vous écrivez à {email} au sujet du même problème.",
  "Reports are kept private and used only to fix {app}.": "Les rapports restent privés et servent uniquement à corriger {app}.",
  "Report sent: #{ref}. Thank you.": "Rapport envoyé : numéro {ref}. Merci !",
  "The report didn’t go. Open Report a problem to try again, or send it by email.":
    "Le rapport n’est pas parti. Rouvrez Signaler un problème pour réessayer, ou envoyez-le par e-mail.",
  "Couldn’t put the details on your clipboard. Use Copy details, then paste them into an email to {email}.":
    "Impossible de mettre les détails dans le presse-papiers. Utilisez Copier les détails, puis collez-les dans un e-mail à {email}.",
  "Your email app opens a new message. The details are on your clipboard: paste them where it says.":
    "Votre messagerie ouvre un nouveau message. Les détails sont dans votre presse-papiers : collez-les à l’endroit indiqué.",
  "Your email app opens a new message with everything filled in. Check it, then send it.":
    "Votre messagerie ouvre un nouveau message déjà rempli. Relisez-le, puis envoyez-le.",
  "Couldn’t open your email app. Use Copy details and paste them into an email to {email}.":
    "Impossible d’ouvrir votre messagerie. Utilisez Copier les détails et collez-les dans un e-mail à {email}.",
  "Copied. Paste them into a new email to {email}.": "Copiés. Collez-les dans un nouvel e-mail à {email}.",
  "Couldn’t reach Sookera. Check your connection and try again, or use Send by email.":
    "Impossible de joindre Sookera. Vérifiez votre connexion et réessayez, ou utilisez Envoyer par e-mail.",
  "Too many reports from this connection. Try again in {n} minutes, or use Send by email.": {
    one: "Trop de rapports depuis cette connexion. Réessayez dans {n} minute, ou utilisez Envoyer par e-mail.",
    other: "Trop de rapports depuis cette connexion. Réessayez dans {n} minutes, ou utilisez Envoyer par e-mail.",
  },
  "Sookera’s report service isn’t answering right now. Try again later, or use Send by email.":
    "Le service de rapports de Sookera ne répond pas pour le moment. Réessayez plus tard, ou utilisez Envoyer par e-mail.",
  "Couldn’t send the report. Use Send by email instead.": "Impossible d’envoyer le rapport. Utilisez plutôt Envoyer par e-mail.",

  // ---- The report's words (Copy details, the email) ----
  "Stopped (problem)": "Arrêt",
  "Couldn’t start (problem)": "Démarrage impossible",
  "{date}, {time}": "{date} à {time}",
  "{v} (Setup)": "{v} (installée)",
  "{v} (Portable)": "{v} (portable)",
  "Dark (look)": "Sombre",
  "Light (look)": "Claire",
  "English (the language)": "Français",
  "Language: {language}": "Langue : {language}",
  "Look: {look}": "Apparence : {look}",
  "Time zone: {zone}": "Fuseau horaire : {zone}",
  "Recent problems: not included.": "Problèmes récents : non joints.",
  "Recent problems: none noted.": "Problèmes récents : aucun noté.",
  "Recent problems (newest first, {n} of {total}):": "Problèmes récents (du plus récent au plus ancien, {n} sur {total}) :",
  "{area}: {message}": "{area} : {message}",
  "What happened:": "Ce qui s’est passé :",
  "{app} {v}: problem report": "{app} {v} : signalement d’un problème",
  "The details are on your clipboard: paste them here.": "Les détails sont dans votre presse-papiers : collez-les ici.",
} satisfies Record<string, Entry>;
