// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Names the app gives that the person may also see as data: the Inbox note is called "Inbox"
// underneath (that is how it is found), and shows in the person's language.
import { t } from "./i18n.svelte";

/** A note's title as shown ("Inbox" → "Boîte de réception"); null: the Inbox. */
export function noteTitle(n: { title: string } | null | undefined): string {
  if (!n) return t("Inbox");
  return n.title === "Inbox" ? t("Inbox") : n.title;
}
