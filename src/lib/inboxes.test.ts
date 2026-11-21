// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { arrivedSince, firstInboxes, inboxesLost, savedInboxes } from "./inboxes";
import { SAMPLE_ITEM, SAMPLE_NOTE } from "./sample";
import type { Item, Note, Picture, Space } from "./types";

const note = (id: number, title: string, space: Space = "work", archived = false): Note => ({
  id,
  title,
  space,
  palette: "soft",
  color: "Butter",
  sort: id,
  archived,
  created_at: 0,
  wide: false,
  folded: false,
  paper: "plain",
  archived_at: null,
});
const item = (id: number, note_id: number, parent_id: number | null = null) => ({ id, note_id, parent_id }) as Item;
const picture = (id: number, note_id: number, item_id: number | null = null): Picture => ({ id, note_id, item_id, path: "", sort: 0, created_at: 0 });

describe("one Inbox per space", () => {
  it("is each space's first note called Inbox, outside the archive", () => {
    const notes = [note(9, "Inbox"), note(3, "Inbox", "work", true), note(5, "Inbox"), note(7, "Inbox", "personal"), note(2, "Hub")];
    expect([...firstInboxes(notes)]).toEqual([
      ["work", 5],
      ["personal", 7],
    ]);
  });

  it("an old Inbox put back, the newer one archived, the restore undone: the space has lost its Inbox", () => {
    // An Inbox archived before 1.0 (3) and today's (8).
    const start = [note(3, "Inbox", "work", true), note(8, "Inbox")];
    const restored = [note(3, "Inbox"), note(8, "Inbox")];
    expect(firstInboxes(restored).get("work")).toBe(3);
    const newerArchived = [note(3, "Inbox"), note(8, "Inbox", "work", true)];
    const undone = [note(3, "Inbox", "work", true), note(8, "Inbox", "work", true)];
    expect(inboxesLost(firstInboxes(start), firstInboxes(restored))).toEqual([]);
    expect(inboxesLost(firstInboxes(newerArchived), firstInboxes(undone))).toEqual(["work"]);
  });

  it("another note called Inbox takes the place by itself; a space that had none isn't one that lost it", () => {
    const before = firstInboxes([note(3, "Inbox"), note(8, "Inbox"), note(4, "Hub", "personal")]);
    expect(inboxesLost(before, firstInboxes([note(3, "Trip"), note(8, "Inbox"), note(4, "Hub", "personal")]))).toEqual([]);
    expect(inboxesLost(before, firstInboxes([note(3, "Inbox", "personal"), note(8, "Trip")]))).toEqual(["work"]);
    expect(inboxesLost(firstInboxes([note(4, "Hub")]), firstInboxes([note(4, "Hub", "work", true)]))).toEqual([]);
  });

  it("the tour's sample Inbox stands in only while the space has none, and never counts as one that goes", () => {
    const sample = note(SAMPLE_NOTE, "Inbox");
    expect(firstInboxes([sample]).get("work")).toBe(SAMPLE_NOTE);
    expect(firstInboxes([sample, note(12, "Inbox")]).get("work")).toBe(12);
    expect(firstInboxes([note(12, "Inbox"), sample]).get("work")).toBe(12);
    // The tour ends (its sample goes) while a note is saved: no space lost a saved Inbox.
    expect(inboxesLost(savedInboxes([sample, note(4, "Hub")]), savedInboxes([note(4, "Hub")]))).toEqual([]);
    expect(savedInboxes([sample, note(12, "Inbox")]).get("work")).toBe(12);
  });

  it("what reached a note since a moment: its new lines (not their steps, which follow) and loose pictures", () => {
    const then = { items: new Set([1, 2]), pictures: new Set([10]) };
    const items = [item(1, 3), item(2, 3, 1), item(4, 3), item(5, 3, 4), item(6, 3, 1), item(7, 8), item(SAMPLE_ITEM, 3)];
    const pictures = [picture(10, 3), picture(11, 3), picture(12, 3, 4), picture(13, 8)];
    const got = arrivedSince(items, pictures, 3, then);
    expect(got.items.map((i) => i.id)).toEqual([4]);
    expect(got.pictures.map((p) => p.id)).toEqual([11]);
  });
});
