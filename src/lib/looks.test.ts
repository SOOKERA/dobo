// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board's look: palettes, holding notes, "Changed from", your colours, dark mode by hours,
// reading damaged settings, and the .ztheme files (export, strict import, merging).
import { describe, expect, it } from "vitest";
import {
  addYourColour,
  BACKGROUNDS,
  cleanLook,
  darkBySwitch,
  DEFAULT_BOARD,
  describeChange,
  heldBy,
  inHours,
  lookIsDark,
  OUR_STYLES,
  OUR_THEMES,
  readBoard,
  readDarkMode,
  sameLook,
  soften,
  uniqueName,
  WC_MOODS,
  WC_PAINTS,
  YOUR_COLOURS_CAP,
  type Look,
  type YourColour,
} from "./looks";
import { buildExport, mergeImport, parseZtheme } from "./ztheme";

const cork: Look = { bg: "cork", colour: "Natural", held: "auto" };

describe("backgrounds and styles", () => {
  it("has the twelve backgrounds, nine of our styles, our themes, 14 mixes and 17 paints", () => {
    expect(BACKGROUNDS.map((b) => b.name)).toEqual(["Plain", "Paper", "Dotted", "Cork", "Kraft", "Terrazzo", "Watercolour", "Blueprint", "Felt", "Linen", "Plaster", "Picture"]);
    expect(OUR_STYLES.map((s) => s.name)).toEqual(["Plain", "Paper day", "Dotted journal", "Cork board", "Kraft workshop", "Terrazzo", "Watercolour", "Blueprint", "Felt night"]);
    expect(WC_MOODS.flatMap((m) => m.mixes)).toHaveLength(14);
    expect(WC_PAINTS).toHaveLength(17);
    expect(WC_PAINTS.find((p) => p[0] === "Terracotta")?.[1]).toBe("205,115,85");
    for (const t of OUR_THEMES) expect(cleanLook(t.light)).toEqual(t.light);
  });

  it("holds notes the Auto way: pins on cork and felt, tape on linen, plaster, kraft, none on paper", () => {
    const by = (bg: Look["bg"]) => heldBy({ bg, colour: "", held: "auto" });
    expect([by("cork"), by("felt")]).toEqual(["pins", "pins"]);
    expect([by("linen"), by("plaster"), by("kraft")]).toEqual(["tape", "tape", "tape"]);
    expect([by("paper"), by("dotted"), by("plain")]).toEqual(["none", "none", "none"]);
    expect(heldBy({ ...cork, held: "tape" })).toBe("tape");
  });

  it("says what changed from the style it came from", () => {
    expect(describeChange(cork, { ...cork, colour: "Honey" })).toBe("colour Honey");
    expect(describeChange(cork, { ...cork, held: "none" })).toBe("no fixings");
    expect(describeChange(cork, { bg: "felt", colour: "Forest", held: "auto" })).toBe("Felt background · Forest");
    const wc = OUR_STYLES.find((s) => s.id === "watercolour")!.look;
    expect(describeChange(wc, { ...wc, wc: { ...wc.wc!, pattern: "Sweep", seed: 5 } })).toBe("Sweep · repainted");
    expect(sameLook(cork, { ...cork })).toBe(true);
    expect(sameLook(cork, { ...cork, colour: "Custom", custom: "#112233" })).toBe(false);
  });

  it("knows dark backgrounds (custom ones too) so grain and shadows adapt", () => {
    expect(lookIsDark({ bg: "felt", colour: "Forest", held: "auto" })).toBe(true);
    expect(lookIsDark({ bg: "paper", colour: "Custom", custom: "#101820", held: "auto" })).toBe(true);
    expect(lookIsDark({ bg: "paper", colour: "Custom", custom: "#F0E0D0", held: "auto" })).toBe(false);
  });
});

describe("watercolour colours", () => {
  it("softens a custom pick toward a pigment (saturation ≤ 0.62, lightness 0.52–0.72)", () => {
    expect(soften("#FF2D9B")).toEqual([215, 85, 153]); // the design board's own example (#FF2D9B, neon pink)
    const [r, g, b] = soften("#000000");
    expect(Math.max(r, g, b)).toBeGreaterThan(120);
    expect(soften("#FFFFFF")[0]).toBeLessThan(190);
  });

  it("keeps your colours newest first, without duplicates, capped gently at 48", () => {
    let list: YourColour[] = [];
    for (let i = 0; i < 50; i++) list = addYourColour(list, `${i},0,0`, new Set(["0,0,0"]), i);
    expect(list).toHaveLength(YOUR_COLOURS_CAP);
    expect(list[0].rgb).toBe("49,0,0");
    expect(list.some((c) => c.rgb === "0,0,0")).toBe(true); // used by a saved style: kept
    expect(list.some((c) => c.rgb === "1,0,0")).toBe(false); // the oldest unused one made room
    list = addYourColour(list, "30,0,0", new Set(), 99);
    expect(list[0].rgb).toBe("30,0,0");
    expect(list.filter((c) => c.rgb === "30,0,0")).toHaveLength(1);
    // Every one in use: the list may grow past the cap.
    const all = new Set(list.map((c) => c.rgb));
    expect(addYourColour(list, "9,9,9", all)).toHaveLength(YOUR_COLOURS_CAP + 1);
  });
});

describe("dark mode", () => {
  it("by hours wraps past midnight", () => {
    expect(inHours("21:40", "19:00", "07:00")).toBe(true);
    expect(inHours("06:59", "19:00", "07:00")).toBe(true);
    expect(inHours("07:00", "19:00", "07:00")).toBe(false);
    expect(inHours("12:00", "08:00", "18:00")).toBe(true);
  });
  it("the switch: Windows, hours, always light, always dark", () => {
    const at = new Date(2026, 10, 18, 21, 40);
    expect(darkBySwitch({ mode: "windows", from: "19:00", until: "07:00" }, at)).toBeNull();
    expect(darkBySwitch({ mode: "hours", from: "19:00", until: "07:00" }, at)).toBe(true);
    expect(darkBySwitch({ mode: "light", from: "19:00", until: "07:00" }, at)).toBe(false);
    expect(darkBySwitch({ mode: "dark", from: "19:00", until: "07:00" }, at)).toBe(true);
    expect(readDarkMode({ mode: "hours", from: "7pm", until: "07:00" }).from).toBe("19:00");
  });
});

describe("settings from before (or damaged) fall back safely", () => {
  it("reads the board", () => {
    expect(readBoard(undefined)).toBe(DEFAULT_BOARD);
    expect(readBoard({ light: { bg: "lava" } })).toBe(DEFAULT_BOARD);
    const b = readBoard({ light: { bg: "cork", colour: "Nope", held: "glue" }, dark: null });
    expect(b.light).toEqual({ bg: "cork", colour: "Natural", held: "auto" });
    expect(b.dark).toBeNull();
    expect(cleanLook({ bg: "paper", colour: "Custom", custom: "red", held: "auto" })?.colour).toBe("Warm ivory");
  });
  it("names: free ones get (2), (3)", () => {
    expect(uniqueName("Cork", ["cork"])).toBe("Cork (2)");
    expect(uniqueName("Cork", ["Cork", "Cork (2)"])).toBe("Cork (3)");
    expect(uniqueName("  ", [])).toBe("Untitled");
  });
});

describe(".ztheme files", () => {
  const theme = { id: "t1", name: "Cork & kraft", light: cork, dark: { bg: "picture", colour: "", held: "tape", picture: "C:\\me\\holiday.jpg" } as Look, at: 1 };
  const data = {
    theme,
    themes: [theme],
    styles: [{ id: "s1", name: "My cork", look: cork, at: 1 }],
    mixes: [{ id: "m1", name: "Mine", colours: ["216,96,150"], pattern: "Sweep" as const, paper: "white" as const, at: 1 }],
    colours: [{ rgb: "216,96,150", at: 1 }],
    darkMode: { mode: "hours" as const, from: "19:00", until: "07:00" },
  };

  it("export never carries notes or picture paths; import reads it back", () => {
    const z = buildExport("everything", data);
    const text = JSON.stringify(z);
    expect(text).not.toContain("holiday.jpg");
    const back = parseZtheme(text);
    expect(back.themes[0].name).toBe("Cork & kraft");
    expect(back.styles).toHaveLength(1);
    expect(back.dark_mode?.mode).toBe("hours");
    expect(buildExport("theme", data).styles).toHaveLength(0);
  });

  it("refuses what isn't a look", () => {
    expect(() => parseZtheme("{")).toThrow();
    expect(() => parseZtheme("[]")).toThrow();
    expect(() => parseZtheme(JSON.stringify({ ztheme: 2, colours: [{ rgb: "1,2,3" }] }))).toThrow(/version/);
    expect(() => parseZtheme(JSON.stringify({ ztheme: 1, colours: [{ rgb: "1,2,3" }], script: "x" }))).toThrow();
    expect(() => parseZtheme(JSON.stringify({ ztheme: 1, themes: "x" }))).toThrow();
    expect(() => parseZtheme(JSON.stringify({ ztheme: 1, kind: "theme" }))).toThrow(/nothing/);
    expect(() => parseZtheme("x".repeat(600 * 1024))).toThrow(/big/);
    // Bad entries are left out, good ones kept.
    const z = parseZtheme(JSON.stringify({ ztheme: 1, kind: "everything", colours: [{ rgb: "999,0,0" }, { rgb: "1,2,3" }] }));
    expect(z.colours.map((c) => c.rgb)).toEqual(["1,2,3"]);
  });

  it("adds without replacing; clashing names get (2)", () => {
    const file = parseZtheme(JSON.stringify(buildExport("everything", data)));
    const m = mergeImport(file, { themes: [theme], styles: [], mixes: [], colours: [{ rgb: "216,96,150", at: 0 }] });
    expect(m.themes.map((t) => t.name)).toEqual(["Cork & kraft (2)", "Cork & kraft"]);
    expect(m.themes[0].id).not.toBe("t1");
    expect(m.added).toEqual({ themes: 1, styles: 1, mixes: 1, colours: 0 });
  });
});

describe("the watercolour painter", async () => {
  const { wcPlan, wcSvg } = await import("./watercolour");
  const sky = ["60,190,232", "150,130,215", "110,205,235"];

  it("paints the same picture for the same seed, a new one for a new seed", () => {
    const strip = (s: string) => s.replace(/p\d+[A-Za-z]*_\d+/g, "");
    expect(strip(wcSvg(wcPlan(sky, 744, 900, 18, "Scattered")))).toBe(strip(wcSvg(wcPlan(sky, 744, 900, 18, "Scattered"))));
    expect(strip(wcSvg(wcPlan(sky, 744, 900, 19, "Scattered")))).not.toBe(strip(wcSvg(wcPlan(sky, 744, 900, 18, "Scattered"))));
  });

  it("lays patches on a jittered grid of the same brush unit; patterns only choose cells", () => {
    const scattered = wcPlan(sky, 760, 900, 18, "Scattered").patches.length;
    // 4 columns x 6 rows, about 8% left as breathing paper.
    expect(scattered).toBeGreaterThan(18);
    expect(scattered).toBeLessThanOrEqual(24);
    expect(wcPlan(sky, 760, 900, 18, "Corners").patches.length).toBeLessThan(scattered);
    expect(wcPlan(sky, 760, 900, 18, "One patch").patches.length).toBeLessThan(scattered);
    // Sweep adds a row above and below (its bands lean).
    expect(wcPlan(sky, 760, 900, 18, "Sweep").patches.length).toBeGreaterThan(scattered);
  });

  it("dark paper: lifted colours laid with screen, softer grain", () => {
    const night = wcPlan(sky, 400, 300, 5, "Scattered", { night: true });
    expect(night.paper).toBe("27,28,32");
    expect(night.blend).toBe("screen");
    expect(night.grain).toBe("soft-light");
    expect(wcPlan(sky, 400, 300, 5, "Scattered").blend).toBe("multiply");
  });

  it("a single colour is a solid wash; blooms stay very faint", () => {
    const one = wcSvg(wcPlan(["60,190,232"], 400, 300, 7, "Scattered"));
    expect(one).not.toContain("150,130,215");
    const blooms = wcPlan(sky, 744, 900, 18, "Scattered").patches.map((p) => p.bloom).filter(Boolean);
    for (const b of blooms) expect(b).toContain("253,252,249,0.16");
  });

  it("each patch knows its own corner of the board (painted and blended on its own)", () => {
    for (const pattern of ["Scattered", "Sweep", "Corners", "One patch"] as const) {
      const W = 880,
        H = 760;
      const plan = wcPlan(sky, W, H, 11, pattern);
      for (const p of plan.patches) {
        expect(p.box.x).toBeGreaterThanOrEqual(0);
        expect(p.box.y).toBeGreaterThanOrEqual(0);
        expect(p.box.x + p.box.w).toBeLessThanOrEqual(W);
        expect(p.box.y + p.box.h).toBeLessThanOrEqual(H);
      }
      // Far smaller than the board overall: that's the point (a patch is ~190x150 of it).
      const avg = plan.patches.reduce((s, p) => s + p.box.w * p.box.h, 0) / plan.patches.length;
      expect(avg).toBeLessThan(W * H * 0.6);
    }
  });
});

describe("what the style showing is compared with", async () => {
  const { refFor, resolveRef, refName } = await import("./looks");
  const halves = (t: (typeof OUR_THEMES)[number]) =>
    (["light", "dark"] as const).map((half) => {
      const l = half === "dark" ? t.dark : t.light;
      return { half, l, from: refFor(l, half, { kind: "ours", id: t.id }, []) };
    });

  it("an untouched board reads as unchanged in both modes (the default and every theme of ours)", () => {
    for (const [half, l, from] of [
      ["light", DEFAULT_BOARD.light, DEFAULT_BOARD.lightFrom],
      ["dark", DEFAULT_BOARD.dark!, DEFAULT_BOARD.darkFrom],
    ] as const) {
      const base = resolveRef(from, [], []);
      expect(base, half).not.toBeNull();
      expect(sameLook(base!, l), `default ${half}`).toBe(true);
    }
    for (const t of OUR_THEMES)
      for (const { half, l, from } of halves(t)) expect(sameLook(resolveRef(from, [], [])!, l), `${t.name} ${half}`).toBe(true);
  });

  it("a half that is one of our styles points at it (its card is highlighted); else at the theme's half", () => {
    const paperFelt = OUR_THEMES.find((t) => t.id === "paper-felt")!;
    expect(refFor(paperFelt.dark, "dark", { kind: "ours", id: "paper-felt" }, [])).toEqual({ kind: "ours", id: "felt-night" });
    const paper = OUR_THEMES.find((t) => t.id === "paper")!;
    const night = refFor(paper.dark, "dark", { kind: "ours", id: "paper" }, []);
    expect(night).toEqual({ kind: "theme", id: "ours:paper", half: "dark" });
    expect(refName(night, [], [])).toBe("Paper · Dark");
    // Changed afterwards: compared with the theme's half, so "Changed from Paper · Dark · colour Blush".
    expect(describeChange(resolveRef(night, [], [])!, { ...paper.dark, colour: "Blush" })).toBe("colour Blush");
  });

  it("a half of one of your themes follows that theme; a deleted theme leaves nothing to compare with", () => {
    const mine = { id: "t9", name: "Late studio", light: cork, dark: { bg: "felt", colour: "Navy", held: "auto" } as Look, at: 1 };
    const ref = refFor(mine.dark, "dark", { kind: "yours", id: "t9" }, []);
    expect(resolveRef(ref, [], [mine])).toEqual(mine.dark);
    expect(resolveRef(ref, [], [])).toBeNull();
    expect(readBoard({ light: cork, lightFrom: ref, darkFrom: { kind: "theme", id: "x", half: "up" } }).lightFrom).toEqual(ref);
  });
});

describe("the docked panel's width", async () => {
  const { dockWidth, widthForScreen, presetName } = await import("./dock");
  it("snaps gently to Narrow / Medium / Wide while dragging, any width in between allowed", () => {
    expect(dockWidth(812, 1920, true)).toBe(812);
    expect(dockWidth(708, 1920, true)).toBe(720);
    expect(dockWidth(573, 1920, true)).toBe(560);
    expect(dockWidth(708, 1920, false)).toBe(708);
  });
  it("stays between 420 and the smaller of 1200 or 70% of the screen", () => {
    expect(dockWidth(300, 1920, true)).toBe(420);
    expect(dockWidth(5000, 2560, true)).toBe(1200);
    expect(dockWidth(1000, 1280, false)).toBe(896);
    expect(dockWidth(900, 500, false)).toBe(420);
  });
  it("is remembered per screen, falling back to the last width chosen", () => {
    const w = { "DISPLAY1@0,0": 812, "DISPLAY2@1920,0": "x" };
    expect(widthForScreen(w, "DISPLAY1@0,0", 560)).toBe(812);
    expect(widthForScreen(w, "DISPLAY2@1920,0", 560)).toBe(560);
    expect(widthForScreen(undefined, "any", undefined)).toBe(720);
  });
  it("names the presets; a dragged width is custom", () => {
    expect(presetName(720)).toBe("Medium");
    expect(presetName(812)).toBeNull();
  });
});
