// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { driveOf, groupBackups, KEEP, KINDS, noticeText, sameDrive } from "./backups";
import type { BackupInfo } from "./backend";
import { i18n } from "./i18n.svelte";

const b = (reason: string, at: number, version: string | null = null): BackupInfo => ({ name: `${reason}-${at}`, path: `x/${reason}-${at}`, at, bytes: 1, reason, version, kept_in_data: false });

describe("Settings › Backups", () => {
  it("lists the backups by kind, in a fixed order, each kind as listed", () => {
    const list = [b("manual", 9), b("daily", 8), b("from-a-newer-dobo", 7), b("before-update", 6, "1.1.0"), b("daily", 5), b("before-restore", 4)];
    const g = groupBackups(list);
    expect(g.map((x) => x.kind)).toEqual(["daily", "manual", "before-restore", "before-update", "from-a-newer-dobo"]);
    expect(g[0].items.map((x) => x.at)).toEqual([8, 5]);
    expect(g.map((x) => x.keep)).toEqual([14, 10, 5, 3, null]);
    expect(groupBackups([])).toEqual([]);
  });

  it("keeps as many of each kind as the app does", () => {
    // (src-tauri/src/backup.rs keep_of.)
    expect(KINDS.map((k) => KEEP[k])).toEqual([14, 10, 5, 3]);
  });

  it("knows when the backups sit on the notes' drive", () => {
    const r = String.raw;
    const data = r`C:\Users\Jo\AppData\Roaming\com.sookera.ztodo`;
    expect(sameDrive(r`${data}\backups`, data)).toBe(true);
    expect(sameDrive("c:/users/jo/appdata/roaming/com.sookera.ztodo/backups/", data)).toBe(true);
    // Outside the data folder, still the same drive.
    expect(sameDrive(r`C:\Backups\Dobo`, data)).toBe(true);
    expect(sameDrive(r`\\?\C:\Backups`, data)).toBe(true);
    expect(sameDrive(r`E:/Backups\Dobo`, data)).toBe(false);
    // A network share: its server and share name.
    const share = r`\\NAS\Home\Jo\Dobo`;
    expect(sameDrive(r`\\nas\home\Backups`, share)).toBe(true);
    expect(sameDrive("//nas/home/Backups", share)).toBe(true);
    expect(sameDrive(r`\\?\UNC\nas\home\x`, share)).toBe(true);
    expect(sameDrive(r`\\nas\other\Backups`, share)).toBe(false);
    expect(sameDrive(r`\\nas\home\x`, data)).toBe(false);
    expect(driveOf(r`\\nas\home\x`)).toBe(r`\\nas\home`);
    expect(sameDrive("", data)).toBe(false);
    expect(sameDrive(r`E:\x`, "")).toBe(false);
    expect(sameDrive("backups", "relative")).toBe(false);
  });
});

describe("a word of its own in Settings › Backups", () => {
  it("is said in the language of the moment, from what Rust keeps; an older version's words as they are", () => {
    const n = { code: "update_backup_elsewhere", args: { app: "Dobo", version: "1.1.0", path: "C:/data/backups/b", chosen: "E:/Backups" } } as const;
    i18n.choice = "en";
    expect(noticeText(n)).toBe("The backup made before Dobo 1.1.0 first opened your notes is in C:/data/backups/b: the backups folder you chose (E:/Backups) didn’t answer in time.");
    i18n.choice = "fr";
    expect(noticeText(n)).toBe("La sauvegarde faite avant que Dobo 1.1.0 n’ouvre vos notes pour la première fois est dans C:/data/backups/b\u00a0: le dossier des sauvegardes que vous avez choisi (E:/Backups) n’a pas répondu à temps.");
    expect(noticeText({ code: "update_backup_failed", args: { app: "Dobo", version: "1.1.0", error: "disque plein" } })).toBe("Aucune sauvegarde n’a pu être faite avant que Dobo 1.1.0 n’ouvre vos notes pour la première fois\u00a0: disque plein");
    expect(noticeText({ code: "restore_missed", args: { error: "x", path: "C:/d/s.sqlite" } })).toContain("Ils sont gardés dans C:/d/s.sqlite.");
    expect(noticeText("The backup made before Dobo 1.1.0 first opened your notes is in C:/b")).toBe("The backup made before Dobo 1.1.0 first opened your notes is in C:/b");
    i18n.choice = "system";
  });

  it("says why the folder chosen wasn't used, and a known cause in its own words", () => {
    const at = (why: "no_answer" | "not_writable" | "full" | "failed" | null) => ({ code: "update_backup_elsewhere", args: { app: "Dobo", version: "1.1.0", path: "C:/b", chosen: "E:/B", why } }) as const;
    const failed = (why: "not_writable" | "full" | "failed" | null) => ({ code: "update_backup_failed", args: { app: "Dobo", version: "1.1.0", error: "disk I/O error", why } }) as const;
    i18n.choice = "en";
    expect(noticeText(at(null))).toMatch(/\(E:\/B\) didn’t answer in time\.$/);
    expect(noticeText(at("no_answer"))).toMatch(/didn’t answer in time\.$/);
    expect(noticeText(at("not_writable"))).toMatch(/\(E:\/B\) can’t be written to\.$/);
    expect(noticeText(at("full"))).toMatch(/the drive of the backups folder you chose \(E:\/B\) is full\.$/);
    expect(noticeText(at("failed"))).toMatch(/\(E:\/B\) couldn’t be used\.$/);
    expect(noticeText(failed("full"))).toBe("No backup could be made before Dobo 1.1.0 first opened your notes: the drive is full.");
    expect(noticeText(failed("not_writable"))).toBe("No backup could be made before Dobo 1.1.0 first opened your notes: the data folder can’t be written to.");
    expect(noticeText(failed("failed"))).toBe("No backup could be made before Dobo 1.1.0 first opened your notes: disk I/O error");
    expect(noticeText(failed(null))).toMatch(/: disk I\/O error$/);
    i18n.choice = "fr";
    expect(noticeText(at("full"))).toBe("La sauvegarde faite avant que Dobo 1.1.0 n’ouvre vos notes pour la première fois est dans C:/b\u00a0: le disque du dossier des sauvegardes que vous avez choisi (E:/B) est plein.");
    expect(noticeText(at("not_writable"))).toMatch(/\(E:\/B\) ne peut pas être modifié\.$/);
    expect(noticeText(at("failed"))).toMatch(/n’a pas pu être utilisé\.$/);
    expect(noticeText(failed("full"))).toBe("Aucune sauvegarde n’a pu être faite avant que Dobo 1.1.0 n’ouvre vos notes pour la première fois : le disque est plein.");
    expect(noticeText(failed("not_writable"))).toMatch(/: le dossier des données ne peut pas être modifié\.$/);
    i18n.choice = "system";
  });
});
