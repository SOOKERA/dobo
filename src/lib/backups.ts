// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Settings › Backups: the backups by kind, and whether they sit on the notes' drive.
import type { BackupInfo, BackupNotice } from "./backend";
import { t } from "./i18n.svelte";

/** The kinds of backups, in the order Settings lists them, and how many of each are kept
 * (src-tauri/src/backup.rs keep_of): one kind never pushes another out. */
export const KINDS = ["daily", "manual", "before-restore", "before-update"] as const;
export const KEEP: Record<string, number> = { daily: 14, manual: 10, "before-restore": 5, "before-update": 3 };

export interface BackupGroup {
  kind: string;
  /** How many of it are kept (null: a kind from a newer Dobo, never removed). */
  keep: number | null;
  items: BackupInfo[];
}

/** The backups by kind, in `KINDS`' order (a kind this version doesn't know last), each as listed
 * (newest first). */
export function groupBackups(list: BackupInfo[]): BackupGroup[] {
  const rank = (k: string) => {
    const i = (KINDS as readonly string[]).indexOf(k);
    return i < 0 ? KINDS.length : i;
  };
  const kinds = [...new Set(list.map((b) => b.reason))].sort((a, b) => rank(a) - rank(b));
  return kinds.map((kind) => ({ kind, keep: KEEP[kind] ?? null, items: list.filter((b) => b.reason === kind) }));
}

/** The drive a Windows path is on: "c:", or a network share's "\\server\share" (whatever the
 * slashes or the letter case; "\\?\" long paths too), or null when it can't be told. */
export function driveOf(path: string): string | null {
  const p = path
    .trim()
    .replace(/\//g, "\\")
    .replace(/^\\\\[?.]\\UNC\\/i, "\\\\")
    .replace(/^\\\\[?.]\\/, "");
  const letter = /^([a-z]):/i.exec(p);
  if (letter) return `${letter[1].toLowerCase()}:`;
  const share = /^\\\\([^\\]+)\\([^\\]+)/.exec(p);
  return share ? `\\\\${share[1]}\\${share[2]}`.toLowerCase() : null;
}

/** Is `folder` on the same drive (or network share) as `dir`, so that one drive failing loses
 * both? */
export function sameDrive(folder: string, dir: string): boolean {
  const a = driveOf(folder);
  return a != null && a === driveOf(dir);
}

/** A word of its own for Settings › Backups, in the language of the moment (what Rust keeps:
 * backup.rs `Notice`). One kept as finished words by an older version is said as it is. */
export function noticeText(n: BackupNotice): string {
  if (typeof n === "string") return n;
  switch (n.code) {
    case "restore_missed":
      return t("The backup made before the restore misses the changes made during it ({error}). They are kept in {path}.", n.args);
    case "update_backup_elsewhere": {
      // (Why the folder chosen wasn't used; none: kept by an earlier 1.1 build, no answer.)
      const { why, ...args } = n.args;
      switch (why) {
        case "not_writable":
          return t("The backup made before {app} {version} first opened your notes is in {path}: the backups folder you chose ({chosen}) can’t be written to.", args);
        case "full":
          return t("The backup made before {app} {version} first opened your notes is in {path}: the drive of the backups folder you chose ({chosen}) is full.", args);
        case "failed":
          return t("The backup made before {app} {version} first opened your notes is in {path}: the backups folder you chose ({chosen}) couldn’t be used.", args);
        default:
          return t("The backup made before {app} {version} first opened your notes is in {path}: the backups folder you chose ({chosen}) didn’t answer in time.", args);
      }
    }
    case "update_backup_failed": {
      // (A cause of its own in the language of the moment; any other, Windows' or SQLite's words.)
      const { why, ...args } = n.args;
      return why === "full"
        ? t("No backup could be made before {app} {version} first opened your notes: the drive is full.", args)
        : why === "not_writable"
          ? t("No backup could be made before {app} {version} first opened your notes: the data folder can’t be written to.", args)
          : t("No backup could be made before {app} {version} first opened your notes: {error}", args);
    }
  }
}
