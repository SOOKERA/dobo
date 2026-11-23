// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A repeat keeps its own time of day across the clock changes, in Paris and New York (the same
// rules as src-tauri/src/repeat.rs).
import { afterAll, describe, expect, it } from "vitest";
import { nextAfterDone, nextOccurrence, wallClock } from "./repeat";
import type { Repeat } from "./types";

// (Node reads the time zone again whenever TZ is set.)
const env = (globalThis as unknown as { process: { env: Record<string, string | undefined> } }).process.env;
const was = env.TZ;
afterAll(() => {
  if (was === undefined) delete env.TZ;
  else env.TZ = was;
});

const hm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};
const daily = (at?: string): Repeat => ({ freq: "day", interval: 1, ends: { kind: "never" }, done: 0, ...(at ? { at } : {}) });

describe.each([
  ["Europe/Paris", [2, 29], [9, 25], "02:30"],
  ["America/New_York", [2, 8], [10, 1], "01:30"],
] as const)("repeats across the clock changes in %s", (zone, spring, autumn, doubled) => {
  it("spring: the first minute after a skipped time, then its own time again", () => {
    env.TZ = zone;
    const before = new Date(2026, spring[0], spring[1] - 1, 2, 30).getTime();
    const skipped = nextOccurrence(daily("02:30"), before)!;
    expect(hm(skipped)).toBe(`03-${String(spring[1]).padStart(2, "0")} 03:00`);
    expect(hm(nextOccurrence(daily("02:30"), skipped)!)).toBe(`03-${String(spring[1] + 1).padStart(2, "0")} 02:30`);
    expect(hm(nextAfterDone(daily("02:30"), before, skipped + 60_000)!.at)).toBe(`03-${String(spring[1] + 1).padStart(2, "0")} 02:30`);
    // A day of 23 hours: 09:00 to 09:00.
    const nine = new Date(2026, spring[0], spring[1] - 1, 9).getTime();
    expect(nextOccurrence(daily("09:00"), nine)! - nine).toBe(23 * 3_600_000);
  });

  it("autumn: the time that comes twice is the first of the two", () => {
    env.TZ = zone;
    const [h, m] = doubled.split(":").map(Number);
    const eve = new Date(2026, autumn[0], autumn[1] - 1, h, m).getTime();
    const twice = nextOccurrence(daily(doubled), eve)!;
    expect(twice - eve).toBe(24 * 3_600_000);
    expect(hm(nextOccurrence(daily(doubled), twice)!).endsWith(doubled)).toBe(true);
    expect(wallClock(2026, spring[0], spring[1], 2, 15)).toBe(wallClock(2026, spring[0], spring[1], 3, 0));
  });
});
