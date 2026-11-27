// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Your picture, once imported, stays yours: the picture tile keeps it after you switch to
// another background, after Undo of the change, and over a restart (a setting look.rs keeps the
// file of). The store over the preview's backend.
import { beforeEach, describe, expect, it, vi } from "vitest";
import { readLastPicture } from "./looks";

async function start() {
  const { testStore } = await import("./testStore");
  const { store, api } = await testStore();
  const { look } = await import("./look.svelte");
  return { store, api, look };
}

describe("your last picture", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("is read with care", () => {
    for (const good of ["C:/d/backgrounds/1.jpg", "D:\\Dobo\\data\\Backgrounds\\1.jpg", "blob:http://localhost/abc"]) expect(readLastPicture({ picture: good })).toBe(good);
    const bad: unknown[] = [null, "C:/x.jpg", { picture: "" }, { picture: 3 }, { picture: `C:/d/backgrounds/${"x".repeat(1000)}.jpg` }, []];
    // Only a picture in the backgrounds folder (pictures are always copied there).
    bad.push({ picture: "\\\\host\\share\\x.jpg" }, { picture: "C:/d/pictures/1.jpg" }, { picture: "C:/d/backgrounds/sub/1.jpg" }, { picture: "C:/d/backgrounds/.." });
    for (const v of bad) expect(readLastPicture(v)).toBeNull();
  });

  it("survives Undo, another background and a restart", async () => {
    const { store, api, look } = await start();
    expect(look.ownPicture).toBeNull();
    const was = look.active.bg;
    api.pickBackgroundPicture = async () => "C:/d/backgrounds/1.jpg";
    await look.pickPicture();
    expect(look.active).toMatchObject({ bg: "picture", picture: "C:/d/backgrounds/1.jpg" });
    expect(store.settings.look_last_picture).toEqual({ picture: "C:/d/backgrounds/1.jpg" });
    // Undo puts the background back; the tile still knows your picture.
    await store.history.undo();
    expect(look.active.bg).toBe(was);
    expect(look.ownPicture).toBe("C:/d/backgrounds/1.jpg");
    // Back to it by its tile (no picker), then away again: still yours.
    await look.setBg("picture");
    expect(look.active.picture).toBe("C:/d/backgrounds/1.jpg");
    await look.setBg("cork");
    expect(look.ownPicture).toBe("C:/d/backgrounds/1.jpg");
    // The settings read again, as at a restart.
    await store.reload();
    expect(look.ownPicture).toBe("C:/d/backgrounds/1.jpg");
  });

  it("one outside Dobo's folders, or seen failing, is never shown: the tile imports, the board is plain", async () => {
    const { store, look } = await start();
    const share = "\\\\host\\share\\backgrounds\\1.jpg";
    await store.setSetting("look_last_picture", { picture: share });
    await store.setSetting("look_board", { light: { bg: "picture", colour: "", held: "auto", picture: share }, dark: null });
    // (The preview stands a network share's path for one outside Dobo's folders.)
    expect(look.usable(share)).toBe(false);
    expect(look.ownPicture).toBeNull();
    expect(look.shown).toMatchObject({ bg: "plain", colour: "Warm white" });
    // A picture of the folder that fails to show: the same once seen.
    const gone = "C:/d/backgrounds/2.jpg";
    await store.setSetting("look_board", { light: { bg: "picture", colour: "", held: "auto", picture: gone }, dark: null });
    expect(look.shown).toMatchObject({ bg: "picture", picture: gone });
    look.pictureFailed(gone);
    expect(look.usable(gone)).toBe(false);
    expect(look.shown.bg).toBe("plain");
    // Tried again (after a restore, the window showing again): still failing, nothing changes;
    // once it shows (the restore brought it back, the drive is back), it's shown again.
    await look.retryPictures(async () => false);
    expect(look.usable(gone)).toBe(false);
    await look.retryPictures(async () => true);
    expect(look.usable(gone)).toBe(true);
    expect(look.shown).toMatchObject({ bg: "picture", picture: gone });
  });
});
