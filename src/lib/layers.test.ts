// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { boardCovered, closeTopLayer, layerBack, layerCount, pushLayer } from "./layers";

describe("layers", () => {
  it("a card beside the board doesn't cover it; a view does", () => {
    const closed: string[] = [];
    const popCard = pushLayer(() => closed.push("card"), undefined, { beside: true });
    expect(layerCount()).toBe(1);
    expect(boardCovered()).toBe(false);
    const popView = pushLayer(() => closed.push("view"));
    expect(boardCovered()).toBe(true);
    popView();
    expect(boardCovered()).toBe(false);
    // Back with only the card open closes it.
    expect(layerBack()).toBe(true);
    expect(closed).toEqual(["card"]);
    popCard();
    expect(layerCount()).toBe(0);
    expect(closeTopLayer()).toBe(false);
  });
});
