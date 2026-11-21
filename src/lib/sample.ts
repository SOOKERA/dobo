// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The quick tour's sample line ("Call the bank"), when there is no Inbox yet its Inbox note, and
// the bigger task it is dropped onto to become a step ("Move house", with two steps of its own):
// they live in memory only. No saved note or line has these ids (saved ones start at 1), and
// nothing that saves, backs up or records an undo step takes them (the store refuses them).
export const SAMPLE_NOTE = -9001;
export const SAMPLE_ITEM = -9002;
export const SAMPLE_PARENT = -9003;
export const SAMPLE_STEPS = [-9004, -9005] as const;

/** A note or line that only the tour holds (never saved). */
export const inMemory = (id: number | null | undefined): boolean => id != null && id < 0;
