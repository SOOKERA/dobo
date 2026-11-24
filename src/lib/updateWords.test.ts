// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { afterEach, describe, expect, it } from "vitest";
import { updateState, updateWords } from "./updateWords";
import type { UpdateView } from "./backend";
import { i18n } from "./i18n.svelte";

const v = (p: Partial<UpdateView> = {}): UpdateView => ({ current: "1.2.0", latest: null, notes: null, ready: false, busy: false, error: null, checked: false, last_checked: null, ...p });
const when = () => "Tue 24 Nov, 10:24";

afterEach(() => {
  i18n.choice = "en";
});

describe("Settings › Updates: where the update stands", () => {
  it("tells the states apart, a running check first", () => {
    expect(updateState(null)).toBe("idle");
    expect(updateState(v())).toBe("idle");
    expect(updateState(v({ busy: true, ready: true, latest: "1.3.0" }))).toBe("checking");
    expect(updateState(v({ ready: true, latest: "1.3.0" }))).toBe("ready");
    // (Ready needs its version: updates.rs always gives it.)
    expect(updateState(v({ ready: true }))).toBe("idle");
    expect(updateState(v({ latest: "1.3.0", checked: true, error: "No room" }))).toBe("out");
    expect(updateState(v({ checked: true, last_checked: 1 }))).toBe("current");
    // A check that failed doesn't say "up to date"; its error says why, under the line.
    expect(updateState(v({ checked: true, error: "Couldn’t reach…" }))).toBe("idle");
  });

  it("says it in a few words, and that Dobo looks once a day when the automatic check is on", () => {
    i18n.choice = "en";
    expect(updateWords(v({ busy: true }), true, when)).toBe("Checking for updates…");
    expect(updateWords(v({ ready: true, latest: "1.3.0" }), true, when)).toBe("Dobo 1.3.0 is ready. It starts the next time Dobo opens.");
    expect(updateWords(v({ latest: "1.3.0", checked: true, error: "No room" }), true, when)).toBe("Dobo 1.3.0 is out.");
    expect(updateWords(v({ latest: "1.3.0", checked: true }), true, when)).toBe("Dobo 1.3.0 is out. Check for updates to get it.");
    expect(updateWords(v({ checked: true, last_checked: 1 }), true, when)).toBe("You’re up to date. Last checked Tue\u00a024\u00a0Nov,\u00a010:24. Dobo looks once a day.");
    expect(updateWords(v({ checked: true, last_checked: 1 }), false, when)).toBe("You’re up to date. Last checked Tue\u00a024\u00a0Nov,\u00a010:24.");
    expect(updateWords(v({ last_checked: 1 }), true, when)).toBe("Last checked Tue\u00a024\u00a0Nov,\u00a010:24. Dobo looks once a day.");
    expect(updateWords(null, true, when)).toBe("Not checked yet. Dobo looks once a day.");
    expect(updateWords(v(), false, when)).toBe("Not checked yet.");
    // A development or test build never looks: nothing about the daily check.
    expect(updateWords(v({ enabled: false }), true, when)).toBe("Not checked yet.");
    expect(updateWords(v({ enabled: true }), true, when)).toBe("Not checked yet. Dobo looks once a day.");
  });

  it("in French too, with its spaces", () => {
    i18n.choice = "fr";
    expect(updateWords(v({ checked: true, last_checked: 1 }), true, when)).toBe("Vous avez la dernière version. Dernière recherche\u00a0: Tue\u00a024\u00a0Nov,\u00a010:24. Dobo vérifie une fois par jour.");
    expect(updateWords(v({ ready: true, latest: "1.3.0" }), true, when)).toBe("Dobo 1.3.0 est prêt. Vous l’aurez à la prochaine ouverture de Dobo.");
    expect(updateWords(v({ latest: "1.3.0", checked: true }), false, when)).toBe("Dobo 1.3.0 est disponible. Pour l’avoir, cliquez sur Rechercher les mises à jour.");
  });
});
