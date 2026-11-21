// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// # suggestions while typing: which notes a #word may mean, the tag that completes it, and that
// the tag files the line into that same note (the loose matching stays as it was).
import { afterEach, describe, expect, it } from "vitest";
import { completeTag, findNoteForTag, parseCapture, suggestNotes, tagAt, tagOf } from "./capture";
import { i18n } from "./i18n.svelte";
import type { Note, Space } from "./types";

let nid = 0;
const note = (title: string, space: Space = "work", more: Partial<Note> = {}): Note => ({
  id: ++nid,
  title,
  space,
  palette: "soft",
  color: "Butter",
  sort: nid,
  archived: false,
  created_at: 0,
  wide: false,
  folded: false,
  paper: "plain",
  ...more,
});

const inbox = note("Inbox");
const hub = note("Hub");
const hubert = note("Hubert’s party", "personal");
const runway = note("Runway · Grand Palais");
const etudes = note("Études de cas");
const q4 = note("Q4 plan");
const shop = note("Shopping list", "personal");
const shopWork = note("Shopping list");
const old = note("Hub archive", "work", { archived: true });
const emoji = note("🎉🎉");
const notes = [inbox, hub, hubert, runway, etudes, q4, shop, shopWork, old, emoji];

afterEach(() => {
  i18n.choice = "en";
});

describe("the tag that names a note whole", () => {
  it("is its name in lower case, with anything not a letter or digit as one dash", () => {
    expect(tagOf(runway)).toBe("runway-grand-palais");
    expect(tagOf(hubert)).toBe("hubert-s-party");
    expect(tagOf(etudes)).toBe("études-de-cas");
    expect(tagOf(q4)).toBe("q4-plan");
    expect(tagOf(emoji)).toBe("");
  });

  it("files the line into that very note, also when another name starts the same way", () => {
    for (const n of [inbox, hub, hubert, runway, etudes, q4, shop])
      expect(findNoteForTag(parseCapture(`Call them #${tagOf(n)}`, "task").tag!, notes, n.space)?.id, n.title).toBe(n.id);
    // "#hub" is Hub, not Hubert's party (whole name first), wherever Hub sits in the list.
    expect(findNoteForTag("hub", [hubert, hub])?.id).toBe(hub.id);
  });

  it("the Inbox's tag is the name it shows: « à-trier » in French", () => {
    i18n.choice = "fr";
    expect(tagOf(inbox)).toBe("à-trier");
    expect(findNoteForTag("à-trier", notes)?.id).toBe(inbox.id);
  });
});

describe("the loose matching is kept", () => {
  it("a start of a name, or a piece of it, still finds the note; nothing found, nothing", () => {
    expect(findNoteForTag("run", notes)?.id).toBe(runway.id);
    expect(findNoteForTag("palais", notes)?.id).toBe(runway.id);
    expect(findNoteForTag("etudes", notes)?.id).toBe(etudes.id);
    expect(findNoteForTag("nowhere", notes)).toBeNull();
    // (Archived notes are never found.)
    expect(findNoteForTag("archive", notes)).toBeNull();
  });

  it("two notes with the same name: the one of the space new lines go to", () => {
    expect(findNoteForTag("shopping", notes, "personal")?.id).toBe(shop.id);
    expect(findNoteForTag("shopping", notes, "work")?.id).toBe(shopWork.id);
    // (Without a space, as before: the first one.)
    expect(findNoteForTag("shopping", notes)?.id).toBe(shop.id);
  });
});

describe("the list and filing agree", () => {
  it("two names with the same tag: the one the list shows is the one filed into, whatever the order in memory", () => {
    // ("Party!" was dragged before "Party" on the board: its sort is lower, the list in memory
    // still has it last.)
    const a = note("Party");
    const b = note("Party!", "work", { sort: -5 });
    const list = [a, b];
    const shown = suggestNotes("party", list, "work")[0];
    expect(shown.id).toBe(b.id);
    expect(findNoteForTag(parseCapture(completeTag("x #par", { start: 2, end: 6 }, shown).text, "task").tag!, list, "work")?.id).toBe(shown.id);
  });
});

describe("the #word under the caret", () => {
  it("is found from its # to the caret, and runs on to the end of the word", () => {
    expect(tagAt("Call Lea #hu", 12)).toEqual({ start: 9, end: 12, query: "hu" });
    expect(tagAt("#", 1)).toEqual({ start: 0, end: 1, query: "" });
    expect(tagAt("Call #hub tomorrow", 7)).toEqual({ start: 5, end: 9, query: "h" });
    expect(tagAt("Call #runway-grand", 18)?.query).toBe("runway-grand");
  });

  it("isn't one inside a word, after a space, or with no #", () => {
    expect(tagAt("issue#12", 8)).toBeNull();
    expect(tagAt("Call #hub ", 10)).toBeNull();
    expect(tagAt("Call Lea", 8)).toBeNull();
  });
});

describe("the suggestions", () => {
  const titles = (q: string, space: Space = "work") => suggestNotes(q, notes, space).map((n) => n.title);

  it("narrow as you type: whole name, then starts with, then contains", () => {
    expect(titles("hub")).toEqual(["Hub", "Hubert’s party"]);
    // ("pa" is inside both: the space new lines go to first.)
    expect(titles("pa")).toEqual(["Runway · Grand Palais", "Hubert’s party"]);
    expect(titles("pa", "personal")).toEqual(["Hubert’s party", "Runway · Grand Palais"]);
    expect(titles("zzz")).toEqual([]);
  });

  it("ignore case, accents and punctuation", () => {
    expect(titles("ETUDES")).toEqual(["Études de cas"]);
    expect(titles("grand-pal")).toEqual(["Runway · Grand Palais"]);
    expect(titles("hubertsp")).toEqual(["Hubert’s party"]);
  });

  it("put the notes of the space new lines go to first", () => {
    expect(titles("shop", "personal")[0]).toBe("Shopping list");
    expect(suggestNotes("shop", notes, "personal")[0].id).toBe(shop.id);
    expect(suggestNotes("shop", notes, "work")[0].id).toBe(shopWork.id);
  });

  it("show a name only once (its tag would find the first one anyway)", () => {
    expect(titles("shopping")).toEqual(["Shopping list"]);
  });

  it("a bare # offers the notes, the Inbox last; never an archived note or one with no name to type", () => {
    const all = suggestNotes("", notes, "work", 20).map((n) => n.title);
    expect(all.at(-1)).toBe("Inbox");
    expect(all).not.toContain("Hub archive");
    expect(all).not.toContain("🎉🎉");
    expect(suggestNotes("", notes, "work").length).toBe(6);
  });
});

describe("completing", () => {
  it("puts the whole tag in place of the #word, with one space after it", () => {
    expect(completeTag("Call Lea #run", { start: 9, end: 13 }, runway)).toEqual({ text: "Call Lea #runway-grand-palais ", caret: 30 });
    // In the middle of the line: the rest stays, with one space between.
    expect(completeTag("Call #ru tomorrow", { start: 5, end: 8 }, runway)).toEqual({ text: "Call #runway-grand-palais tomorrow", caret: 26 });
    // The whole word goes, also the part after the caret.
    expect(completeTag("Call #rutomorrow", tagAt("Call #rutomorrow", 7)!, runway).text).toBe("Call #runway-grand-palais ");
  });

  it("and the completed line files where the list said, its text without the tag", () => {
    const done = completeTag("Book the venue #grand", { start: 15, end: 21 }, runway).text;
    const p = parseCapture(done, "task");
    expect(p.text).toBe("Book the venue");
    expect(findNoteForTag(p.tag!, notes, "work")?.id).toBe(runway.id);
  });
});
