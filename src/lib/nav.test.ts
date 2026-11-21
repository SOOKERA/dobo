// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { back, follow, forward, goTo, once, pagesAt, upTo } from "./nav";

type P = "main" | "themes" | "edit" | "share" | "more" | "customise";
const PARENT: Record<P, P | null> = { main: null, themes: "main", edit: "themes", share: "themes", more: "main", customise: "main" };
const lineage = (p: P) => {
  const up: P[] = [];
  for (let q = PARENT[p]; q; q = PARENT[q]) up.unshift(q);
  return up;
};

describe("pages in a view", () => {
  it("goes down and back up one level at a time", () => {
    let n = goTo(goTo(pagesAt<P>("main"), "themes"), "edit");
    expect([n.trail, n.here]).toEqual([["main", "themes"], "edit"]);
    n = back(n)!;
    expect(n.here).toBe("themes");
    n = back(n)!;
    expect(n.here).toBe("main");
    expect(back(n)).toBeNull(); // leaves the view
  });

  it("forward walks back down the pages just left, in order", () => {
    let n = goTo(goTo(pagesAt<P>("main"), "themes"), "edit");
    n = back(back(n)!)!;
    expect(n.here).toBe("main");
    n = forward(n)!;
    expect(n.here).toBe("themes");
    n = forward(n)!;
    expect(n.here).toBe("edit");
    expect(forward(n)).toBeNull();
  });

  it("jumping up several levels keeps them all for forward", () => {
    let n = goTo(goTo(pagesAt<P>("main"), "themes"), "share");
    n = upTo(n, 0)!;
    expect([n.here, n.trail]).toEqual(["main", []]);
    expect(forward(forward(n)!)!.here).toBe("share");
  });

  it("going somewhere new forgets the forward pages", () => {
    let n = back(goTo(pagesAt<P>("main"), "themes"))!;
    n = goTo(n, "more");
    expect(n.ahead).toEqual([]);
    expect(forward(n)).toBeNull();
  });

  it("remembers the way it came (Customise from More mixes goes back to More mixes)", () => {
    const n = goTo(goTo(pagesAt<P>("main"), "more"), "customise");
    expect(back(n)!.here).toBe("more");
  });

  it("follows a page changed from elsewhere", () => {
    const n = goTo(goTo(pagesAt<P>("main"), "themes"), "share");
    const up = follow(n, "themes", lineage);
    expect([up.pages.here, up.up, up.pages.ahead]).toEqual(["themes", true, ["share"]]);
    const fresh = follow(pagesAt<P>("main"), "share", lineage);
    expect([fresh.pages.trail, fresh.pages.here, fresh.up]).toEqual([["main", "themes"], "share", false]);
  });

  it("one press is one step, however many ways it arrives", () => {
    expect(once("mouse", "back", 1000)).toBe(true);
    expect(once("app", "back", 1100)).toBe(false); // the same press as an app command
    expect(once("mouse", "back", 1150)).toBe(true); // a quick second press
    expect(once("mouse", "back", 1250)).toBe(true); // and a third: never swallowed
    expect(once("app", "back", 1300)).toBe(false);
    expect(once("key", "back", 2000)).toBe(true);
    expect(once("key", "back", 2080)).toBe(true); // Backspace twice, fast
    expect(once("app", "forward", 2100)).toBe(true); // the other way: not the same press
    expect(once("mouse", "back", 3000)).toBe(true);
    expect(once("app", "back", 3400)).toBe(true); // too late to be the same press
  });
});
