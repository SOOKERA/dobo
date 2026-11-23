// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// What Report a problem holds between two opens (until Dobo closes): the words, until they're
// sent; the Include recent problems switch; and the one send under way, so a page opened again
// meanwhile can't send the same report twice.
export const reportDraft = $state({ words: "", include: true, sending: false });
