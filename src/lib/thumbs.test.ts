// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board's thumbs: shown when sharp in the box, the picture itself otherwise or meanwhile; a
// picture just added asked first; drawn samples never asked; a thumb that fails gives way.
import { describe, expect, it, vi } from "vitest";
import type { Picture, Thumb } from "./types";

vi.mock("./calm", () => ({ calmMoment: () => Promise.resolve(null) }));
// (No page here: a picture "loads" at once, or fails when its name says so.)
class FakeImage {
  decoding = "";
  src = "";
  decode() {
    return this.src.includes("broken") ? Promise.reject(new Error("broken")) : Promise.resolve();
  }
}
(globalThis as unknown as { Image: unknown }).Image = FakeImage;

const { thumbs, inDevicePixels } = await import("./thumbs.svelte");

const pic = (id: number, path: string, thumb?: Thumb): Picture => ({ id, note_id: 1, item_id: null, path, sort: id, created_at: 0, thumb });
const T = (path: string, w = 1200, h = 900): Thumb => ({ path: `${path}.${w}x${h}.webp`, w, h });
const tick = () => new Promise((r) => setTimeout(r, 0));

describe("thumbs", () => {
  it("shows the thumb only when it is sharp in the box", () => {
    const p = pic(1, "C:/d/pictures/1-0.jpg", T("C:/d/pictures/thumbs/1-0.jpg"));
    expect(thumbs.sharpIn(p, null)?.w).toBe(1200);
    expect(thumbs.sharpIn(p, { w: 640, h: 300 })?.w).toBe(1200);
    expect(thumbs.sharpIn(p, { w: 1200, h: 344 })?.w).toBe(1200);
    // A wide note on a 200% screen: the picture itself.
    expect(thumbs.sharpIn(p, { w: 1740, h: 345 })).toBeNull();
    // Too tall for it (a panorama's thumb in a tall box).
    expect(thumbs.sharpIn(pic(2, "C:/d/pictures/2-0.jpg", T("x", 2880, 360)), { w: 600, h: 400 })).toBeNull();
    expect(thumbs.src(p, (x) => "asset:" + x)).toBe("asset:C:/d/pictures/thumbs/1-0.jpg.1200x900.webp");
    expect(thumbs.src(pic(3, "C:/d/pictures/3-0.jpg"), (x) => "asset:" + x)).toBe("asset:C:/d/pictures/3-0.jpg");
  });

  it("sizes the box in device pixels, note size and screen scale included", () => {
    // At the large note size on a 200% screen.
    expect(inDevicePixels([550, 150], 1.15, 2)).toEqual({ w: 1265, h: 345 });
    expect(inDevicePixels([300, 150], 1, 2)).toEqual({ w: 600, h: 300 });
    expect(inDevicePixels([300, 150], 1, 1)).toEqual({ w: 300, h: 150 });
  });

  it("makes the missing ones, the one just added first, never a drawn sample", async () => {
    const asked: string[] = [];
    const api = {
      makeThumb: async (path: string) => {
        asked.push(path);
        return path.includes("small") ? null : T(path.replace("pictures/", "pictures/thumbs/"));
      },
      fileSrc: (path: string) => path,
    };
    const old = [pic(10, "C:/d/pictures/a.jpg"), pic(11, "C:/d/pictures/small.png"), pic(12, "data:image/png;base64,xx"), pic(13, "C:/d/pictures/has.jpg", T("h"))];
    const added = pic(14, "C:/d/pictures/new.jpg");
    thumbs.fill(api, old);
    thumbs.soon(api, added);
    for (let i = 0; i < 10; i++) await tick();
    expect(asked[0]).toBe("C:/d/pictures/new.jpg");
    expect(asked).toContain("C:/d/pictures/a.jpg");
    expect(asked).toContain("C:/d/pictures/small.png");
    expect(asked).not.toContain("data:image/png;base64,xx");
    expect(asked).not.toContain("C:/d/pictures/has.jpg");
    expect(thumbs.of(added)?.path).toBe("C:/d/pictures/thumbs/new.jpg.1200x900.webp");
    expect(thumbs.of(old[1])).toBeNull();
    // Asked once only, however often the board is read again.
    thumbs.fill(api, [...old, added]);
    for (let i = 0; i < 5; i++) await tick();
    expect(asked.filter((p) => p === "C:/d/pictures/a.jpg")).toHaveLength(1);
  });

  it("a thumb that can't be shown gives way to the picture", async () => {
    const p = pic(20, "C:/d/pictures/b.jpg", T("C:/d/pictures/thumbs/b.jpg"));
    thumbs.failed(p);
    expect(thumbs.of(p)).toBeNull();
    expect(thumbs.src(p, (x) => x)).toBe("C:/d/pictures/b.jpg");
    // One made that won't load: never shown.
    const api = { makeThumb: async () => T("C:/d/pictures/thumbs/broken"), fileSrc: (x: string) => x };
    const q = pic(21, "C:/d/pictures/c.jpg");
    thumbs.soon(api, q);
    for (let i = 0; i < 5; i++) await tick();
    expect(thumbs.of(q)).toBeNull();
  });
});
