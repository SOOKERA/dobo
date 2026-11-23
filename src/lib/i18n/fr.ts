// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// French. Keys are the English words as the app writes them; values are the French, in parts by
// area. Typography (non-breaking spaces before : ; ? ! » and after «) is added by t(), so plain
// spaces in the parts.
//
// Glossary (keep it consistent): note · tâche · idée · étape · rappel · échéance (deadline) ·
// À trier (Inbox, the note; « À trier » in a sentence, bare only as its own label) · Faire le tri / Trier (Tidy) · tableau (board) · Bilan du jour (Look back) ·
// Style · thème · mélange (mix) · fond (background) · couleur · Personnaliser (Customise) ·
// Ancré / Flottant (Docked / Floating) · Toujours au premier plan · Épingler (Pin) ·
// Paramètres (Settings) · Travail / Perso · modèle (template) · rythme · Annuler / Rétablir
// (Undo / Redo) · Pense-bêtes (Windows Sticky Notes) · appli (app, never « app ») · vous.
import { app } from "./fr-app";
import { settings } from "./fr-settings";
import { board } from "./fr-board";
import { views } from "./fr-views";
import { style } from "./fr-style";

export const parts = { app, settings, board, views, style };
export const fr = { ...app, ...settings, ...board, ...views, ...style };
