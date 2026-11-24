// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { buildEntries, bundled, entriesOf, inline, linkTarget, linkTip, ownSection, parse, planAfterStart, sinceSeen, titleOf, versionParts, whatsNewPage, whatsNewStep, UNKNOWN, type Entry } from "./whatsnew";
import { tIn } from "./i18n.svelte";

const files = import.meta.glob<string>(["/docs/help/whats-new.md", "/docs/help/fr/whats-new.md"], { query: "?raw", import: "default", eager: true });
const EN = files["/docs/help/whats-new.md"];
const FR = files["/docs/help/fr/whats-new.md"];

const entry = (version: string): Entry => ({ version, blocks: [{ kind: "para", inline: [{ kind: "text", text: version }] }] });
const versions = (list: Entry[]) => list.map((e) => e.version);

describe("the What's new files", () => {
  for (const [lang, md] of [
    ["en", EN],
    ["fr", FR],
  ] as const) {
    it(`every "##" heading of ${lang}/whats-new.md is a version, newest first, each with words`, () => {
      const headings = [...md.matchAll(/^## (.*)$/gm)].map((m) => m[1].trim());
      const entries = entriesOf(md);
      expect(versions(entries)).toEqual(headings);
      expect(headings.length).toBeGreaterThanOrEqual(4);
      for (const e of entries) {
        expect(versionParts(e.version), e.version).not.toBeNull();
        expect(e.blocks.length, e.version).toBeGreaterThan(0);
      }
      const sorted = [...entries].sort((a, b) => {
        const [x, y] = [versionParts(a.version)!, versionParts(b.version)!];
        return y[0] - x[0] || y[1] - x[1] || y[2] - x[2];
      });
      expect(versions(entries)).toEqual(versions(sorted));
    });
  }

  it("French has every English version, so nothing falls back to English", () => {
    expect(versions(entriesOf(FR))).toEqual(versions(entriesOf(EN)));
    const fr = bundled("fr");
    expect(fr[0].blocks).toEqual(entriesOf(FR)[0].blocks);
    expect(bundled("en")[0].blocks).toEqual(entriesOf(EN)[0].blocks);
  });

  it("every 1.1.0 and later section opens with its one-sentence intro, then its headings", () => {
    for (const md of [EN, FR]) {
      for (const e of entriesOf(md).filter((x) => versionParts(x.version)![1] >= 1)) {
        expect(e.blocks[0].kind, e.version).toBe("para");
        expect(e.blocks[1], e.version).toMatchObject({ kind: "heading", level: 3 });
      }
    }
  });

  it("every link is a guide that exists (and its part, a heading of it in that language), or a web page", () => {
    const guides = new Set(Object.keys(import.meta.glob("/docs/help/*.md")).map((p) => p.split("/").pop()));
    const pages = import.meta.glob<string>(["/docs/help/*.md", "/docs/help/fr/*.md"], { query: "?raw", import: "default", eager: true });
    // (As the website makes a heading's anchor: lower case, spaces to "-", punctuation dropped,
    // accents kept.)
    const slug = (h: string) =>
      h
        .toLowerCase()
        .replace(/[*_`]/g, "")
        .replace(/[^\p{L}\p{N}\s-]/gu, "")
        .trim()
        .replace(/\s+/g, "-");
    const anchors = (file: string) => new Set([...(pages[file] ?? "").matchAll(/^#{2,3}\s+(.+)$/gm)].map((m) => slug(m[1])));
    expect(slug("Mises à jour")).toBe("mises-à-jour");
    for (const [md, dir] of [
      [EN, "/docs/help/"],
      [FR, "/docs/help/fr/"],
    ]) {
      const links = parse(md).flatMap((b) => (b.kind === "para" ? [b.inline] : b.kind === "list" ? b.items : [])).flat().filter((p) => p.kind === "link");
      expect(links.length).toBeGreaterThan(3);
      for (const l of links) {
        if (l.kind !== "link") continue;
        expect(linkTarget(l.href, "en"), l.href).not.toBeNull();
        if (/^https?:/.test(l.href)) continue;
        const [page, part] = l.href.split("#");
        expect(guides.has(page), l.href).toBe(true);
        if (part) expect(anchors(dir + page).has(part), `${dir}${l.href}`).toBe(true);
      }
    }
  });

  it("a link's tooltip reads as people read it", () => {
    expect(linkTip(linkTarget("install-and-update.md#mises-à-jour", "fr")!)).toBe("dobo.sookera.com/fr/help/install-and-update#mises-à-jour");
    expect(linkTip("https://example.com/a%ZZ")).toBe("example.com/a%ZZ");
  });

  it("a translation without a version's words shows the English ones", () => {
    const en = "# What's new\n\n## 1.1.0\n\nTwo.\n\n## 1.0.0\n\nOne.\n";
    const fr = "# Nouveautés\n\n## 1.1.0\n\n## 1.0.0\n\nUn.\n";
    const built = buildEntries(en, fr);
    expect(built.map((e) => (e.blocks[0] as { inline: { text: string }[] }).inline[0].text)).toEqual(["Two.", "Un."]);
  });
});

describe("the Markdown reader", () => {
  it("reads paragraphs, ### headings, lists and the marks they use", () => {
    const blocks = parse("# Title\n\nA line\nwrapped.\n\n### Highlights\n\n- **Bold,** then [a guide](the-board.md) and `code`.\n  More of it.\n\n- Second\n\nAfter.\n");
    expect(blocks).toEqual([
      { kind: "heading", level: 1, text: "Title" },
      { kind: "para", inline: [{ kind: "text", text: "A line wrapped." }] },
      { kind: "heading", level: 3, text: "Highlights" },
      {
        kind: "list",
        items: [
          [
            { kind: "bold", text: "Bold," },
            { kind: "text", text: " then " },
            { kind: "link", text: "a guide", href: "the-board.md" },
            { kind: "text", text: " and " },
            { kind: "code", text: "code" },
            { kind: "text", text: ". More of it." },
          ],
          [{ kind: "text", text: "Second" }],
        ],
      },
      { kind: "para", inline: [{ kind: "text", text: "After." }] },
    ]);
  });

  it("shows HTML in the text as text, never as HTML", () => {
    const parts = inline(`<img src=x onerror="alert(1)"> **<b>bold</b>** [<i>x</i>](https://example.com)`);
    expect(parts).toEqual([
      { kind: "text", text: `<img src=x onerror="alert(1)"> ` },
      { kind: "bold", text: "<b>bold</b>" },
      { kind: "text", text: " " },
      { kind: "link", text: "<i>x</i>", href: "https://example.com" },
    ]);
    expect(parse("<script>alert(1)</script>")).toEqual([{ kind: "para", inline: [{ kind: "text", text: "<script>alert(1)</script>" }] }]);
  });

  it("keeps French typography and unclosed marks as they are", () => {
    expect(inline("« Tout est fait » : **Échap** ;")).toEqual([
      { kind: "text", text: "« Tout est fait » : " },
      { kind: "bold", text: "Échap" },
      { kind: "text", text: " ;" },
    ]);
    expect(inline("a ** b [c](d e)")).toEqual([{ kind: "text", text: "a ** b [c](d e)" }]);
  });
});

describe("which versions show", () => {
  const all = ["1.2.0", "1.1.1", "1.1.0", "1.0.0"].map(entry);

  it("every version since the one you had, up to the running one, newest first", () => {
    expect(versions(sinceSeen(all, "1.1.0", "1.2.0"))).toEqual(["1.2.0", "1.1.1"]);
    expect(versions(sinceSeen(all, "1.0.0", "1.2.0"))).toEqual(["1.2.0", "1.1.1", "1.1.0"]);
    expect(versions(sinceSeen(all, "1.1.1", "1.2.0"))).toEqual(["1.2.0"]);
    expect(versions(sinceSeen(all, "1.2.0", "1.2.0"))).toEqual([]);
    // (Out of order, written twice, or not a version: still right.)
    const odd = [entry("1.1.1"), entry("Next version"), entry("1.2.0"), entry("1.1.1")];
    expect(versions(sinceSeen(odd, "1.1.0", "1.2.0"))).toEqual(["1.2.0", "1.1.1"]);
  });

  it("never a section above the running version (1.2.0 written while 1.1.1 runs)", () => {
    expect(versions(sinceSeen(all, "1.1.0", "1.1.1"))).toEqual(["1.1.1"]);
    expect(versions(ownSection(all, "1.1.1"))).toEqual(["1.1.1"]);
    expect(versions(ownSection(all, "1.1.5"))).toEqual(["1.1.1"]);
    expect(versions(ownSection(all, "0.9.0"))).toEqual([]);
    expect(planAfterStart("1.1.0", "1.1.1", all)).toEqual({ kind: "show", entries: [all[1]], since: null });
  });

  it("first install, update from 1.1.x, update from an unknown version, already seen", () => {
    // A first install (nothing noted): nothing shows, the running version is noted.
    expect(planAfterStart(undefined, "1.2.0", all)).toEqual({ kind: "note" });
    expect(planAfterStart(null, "1.2.0", all)).toEqual({ kind: "note" });
    // An update from 1.1.0 (noted by whatsnew.rs from the notes' own version).
    expect(planAfterStart("1.1.0", "1.2.0", all)).toEqual({ kind: "show", entries: [all[0], all[1]], since: "1.1.0" });
    expect(planAfterStart("1.1.1", "1.2.0", all)).toEqual({ kind: "show", entries: [all[0]], since: null });
    // From a version that can't be told: only the running version's own part.
    expect(planAfterStart(UNKNOWN, "1.2.0", all)).toEqual({ kind: "show", entries: [all[0]], since: null });
    expect(planAfterStart("garbled", "1.2.0", all)).toEqual({ kind: "show", entries: [all[0]], since: null });
    expect(planAfterStart(UNKNOWN, "1.3.0", all)).toEqual({ kind: "note" });
    // Seen already, or a newer one (gone back): nothing, and nothing noted.
    expect(planAfterStart("1.2.0", "1.2.0", all)).toEqual({ kind: "none" });
    expect(planAfterStart("1.3.0", "1.2.0", all)).toEqual({ kind: "none" });
    // An update to a version without words of its own (a build between releases): noted only.
    expect(planAfterStart("1.2.0", "1.2.1", all)).toEqual({ kind: "note" });
    // A running version that can't be read: nothing.
    expect(planAfterStart("1.1.0", "", all)).toEqual({ kind: "none" });
  });

  it("titles say the one version, or since the one you had", () => {
    for (const [lang, one, since] of [
      ["en", "What’s new in Dobo 1.2.0", "What’s new since Dobo 1.1.0"],
      ["fr", "Nouveautés de Dobo 1.2.0", "Nouveautés depuis Dobo 1.1.0"],
    ] as const) {
      const say: Parameters<typeof titleOf>[1] = (k, p) => tIn(lang, k, p);
      expect(titleOf({ entries: [all[0]], since: null }, say)).toBe(one);
      expect(titleOf({ entries: [all[0], all[1]], since: "1.1.0" }, say)).toBe(since);
      // (One version shown: its own title, whatever you had.)
      expect(titleOf({ entries: [all[0]], since: "1.1.1" }, say)).toBe(one);
    }
  });
});

describe("links", () => {
  it("a guide opens on dobo.sookera.com/help, in the language shown", () => {
    expect(linkTarget("the-board.md", "en")).toBe("https://dobo.sookera.com/help/the-board");
    expect(linkTarget("the-board.md", "fr")).toBe("https://dobo.sookera.com/fr/help/the-board");
    expect(linkTarget("install-and-update.md#updates", "en")).toBe("https://dobo.sookera.com/help/install-and-update#updates");
    expect(linkTarget("install-and-update.md#mises-à-jour", "fr")).toBe("https://dobo.sookera.com/fr/help/install-and-update#mises-%C3%A0-jour");
    expect(linkTarget("README.md", "en")).toBe("https://dobo.sookera.com/help");
    expect(whatsNewPage("fr")).toBe("https://dobo.sookera.com/fr/help/whats-new");
  });

  it("a web page opens as it is; nothing else opens", () => {
    expect(linkTarget("https://github.com/sookera/dobo/discussions", "fr")).toBe("https://github.com/sookera/dobo/discussions");
    for (const bad of ["javascript:alert(1)", "file:///C:/x.md", "../secret.md", "C:\\x.md", "mailto:a@b.c", "data:text/html,x", "a/b.md", "x.md?y", "https://a b"]) expect(linkTarget(bad, "en"), bad).toBeNull();
  });
});

describe("when the start's What's new shows", () => {
  const free = { hidden: false, setup: false, covered: false, typing: false };
  it("shows on a free board, notes a first install, does nothing when seen", () => {
    expect(whatsNewStep("show", free)).toBe("show");
    expect(whatsNewStep("note", free)).toBe("note");
    expect(whatsNewStep("none", free)).toBe("done");
  });
  it("waits for the next show: hidden, behind the first-run window or the terms, over a view, the tour, a carry, or someone typing", () => {
    expect(whatsNewStep("show", { ...free, hidden: true })).toBe("wait");
    expect(whatsNewStep("show", { ...free, setup: true })).toBe("wait");
    expect(whatsNewStep("note", { ...free, setup: true })).toBe("wait");
    expect(whatsNewStep("show", { ...free, covered: true })).toBe("wait");
    expect(whatsNewStep("show", { ...free, typing: true })).toBe("wait");
    // (Noting only: nothing shows, so nothing to wait for but the window.)
    expect(whatsNewStep("note", { ...free, covered: true, typing: true })).toBe("note");
  });
});
