// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { ownFile } from "./ownFiles";
// (The same table is checked against thumbs.rs `directly_in`: the two rules can't drift apart.)
import cases from "./ownFiles.cases.json";

describe("the board asks only for its own folders' files", () => {
  it("a file right inside one of them, any slashes or letter case, with or without \\\\?\\", () => {
    for (const p of cases.own) expect(ownFile(p, cases.folders), p).toBe(true);
  });
  it("never a network share, another folder, a sub-folder, a device or another stream", () => {
    for (const p of cases.notOwn) expect(ownFile(p, cases.folders), p).toBe(false);
  });
  it("never a path of 1000 characters or more", () => {
    const dir = cases.folders[0];
    // (999 and 1000 in all.)
    expect(ownFile(`${dir}\\${"x".repeat(994 - dir.length)}.png`, cases.folders)).toBe(true);
    expect(ownFile(`${dir}\\${"x".repeat(995 - dir.length)}.png`, cases.folders)).toBe(false);
  });
  it("nothing at all when the folders aren't known", () => {
    expect(ownFile(cases.own[0], [])).toBe(false);
  });
});
