// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { toDrop } from "./paintStore";

describe("kept paintings: clean-up", () => {
  it("drops other recipes and the least recently used past the cap", () => {
    const e = [
      { key: "0~old", size: 10, used: 999 },
      { key: "1~a", size: 40, used: 300 },
      { key: "1~b", size: 40, used: 200 },
      { key: "1~c", size: 40, used: 100 },
    ];
    expect(toDrop(e, 85, 1).sort()).toEqual(["0~old", "1~c"]);
  });
  it("keeps everything under the cap", () => {
    expect(toDrop([{ key: "1~a", size: 5, used: 1 }], 10, 1)).toEqual([]);
  });
});
