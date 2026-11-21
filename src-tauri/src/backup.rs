// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Automatic backups: once a day a copy of the notes (the database and the pictures) goes into
//! a folder of its own, in the backups folder chosen in Settings (default: inside the data
//! folder, `paths::data_dir`). The last 14 are kept. "Back up now" and "Restore…" are in Settings.

use crate::{AppState, db};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const KEEP: usize = 14;

#[derive(Serialize, Clone, Debug)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    /// Milliseconds (the folder's time).
    pub at: i64,
    pub bytes: u64,
    /// "daily", "manual" or "before-restore".
    pub reason: String,
}

pub fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    let chosen = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        db::get_setting(&conn, "backup_dir").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.trim().is_empty())
    };
    Ok(match chosen {
        Some(p) => PathBuf::from(p),
        None => crate::paths::sub("backups")?,
    })
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    if !from.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        if p.is_file() {
            let dest = to.join(e.file_name());
            if !dest.exists() {
                total += std::fs::copy(&p, &dest)?;
            }
        }
    }
    Ok(total)
}

fn dir_size(p: &Path) -> u64 {
    std::fs::read_dir(p)
        .map(|it| {
            it.flatten()
                .map(|e| {
                    let p = e.path();
                    if p.is_dir() { dir_size(&p) } else { e.metadata().map(|m| m.len()).unwrap_or(0) }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Makes one backup now. Returns its folder.
pub fn make(app: &AppHandle, reason: &str) -> Result<BackupInfo, String> {
    let now = db::now_ms();
    let root = folder(app)?;
    std::fs::create_dir_all(&root).map_err(|e| tr!("Couldn't use the backups folder: {}", "Impossible d’utiliser le dossier des sauvegardes : {}", e))?;
    let name = format!("{PREFIX}{}-{reason}", crate::clock::stamp(now));
    let dir = root.join(&name);
    if dir.exists() {
        return Err(tr!("A backup was just made. Try again in a second.", "Une sauvegarde vient d’être faite. Réessayez dans une seconde."));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let made = (|| -> Result<(), String> {
        {
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            conn.execute("VACUUM INTO ?1", [dir.join(crate::paths::DB_FILE).to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
        }
        let data = crate::paths::data_dir()?;
        for folder in ["pictures", "previews"] {
            copy_dir(&data.join(folder), &dir.join(folder)).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = made {
        // A half-made backup is not a backup: it must never be listed, restored or counted.
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let _ = db::set_setting(&conn, "backup_last", &serde_json::json!(now));
    }
    prune(&root);
    Ok(BackupInfo { name, path: dir.to_string_lossy().into(), at: now, bytes: dir_size(&dir), reason: reason.into() })
}

/// Backup folders are named "dobo-<date>_<time>-<reason>".
const PREFIX: &str = "dobo-";
/// Before the app was named Dobo (its working name was ZToDo), they were "ztodo-…": those are
/// still its own (listed, restored, counted and pruned with the new ones).
const OLD_PREFIX: &str = "ztodo-";

/// The part of a backup folder's name after its prefix (it starts with the date and time, so it
/// sorts old and new names together), or None for any other folder.
fn stamp_of(name: &str) -> Option<&str> {
    name.strip_prefix(PREFIX).or_else(|| name.strip_prefix(OLD_PREFIX))
}

/// A folder Dobo made and finished: "dobo-…" (or "ztodo-…") with the notes inside. Only these
/// are listed, counted and removed (the backups folder may be one the person also uses for other
/// things). (The notes file inside keeps its name, `ztodo.sqlite`, as in the data folder.)
fn is_backup(dir: &Path) -> bool {
    dir.file_name().and_then(|n| n.to_str()).and_then(stamp_of).is_some() && dir.join(crate::paths::DB_FILE).is_file()
}

/// Keeps the newest 14 of Dobo's own backup folders.
fn prune(root: &Path) {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .map(|it| it.flatten().filter(|e| e.path().is_dir() && is_backup(&e.path())).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort_by(|a, b| stamp_of(a).cmp(&stamp_of(b)));
    while names.len() > KEEP {
        let old = names.remove(0);
        let _ = std::fs::remove_dir_all(root.join(old));
    }
}

pub fn list(app: &AppHandle) -> Result<Vec<BackupInfo>, String> {
    let root = folder(app)?;
    let mut out: Vec<BackupInfo> = std::fs::read_dir(&root)
        .map(|it| {
            it.flatten()
                .filter(|e| is_backup(&e.path()))
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let at = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
                    let reason = name.rsplit('-').next().unwrap_or("daily").to_string();
                    let reason = if name.ends_with("before-restore") { "before-restore".into() } else { reason };
                    BackupInfo { path: e.path().to_string_lossy().into(), bytes: dir_size(&e.path()), name, at, reason }
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| stamp_of(&b.name).cmp(&stamp_of(&a.name)));
    Ok(out)
}

/// Opens a backup's notes and checks they are whole and really Dobo's, before anything is
/// replaced with them.
fn open_checked(file: &Path) -> Result<rusqlite::Connection, String> {
    let bad = |e: &dyn std::fmt::Display| tr!("That backup can't be read ({}). Nothing was changed.", "Cette sauvegarde ne peut pas être lue ({}). Rien n’a été changé.", e);
    let src = rusqlite::Connection::open(file).map_err(|e| bad(&e))?;
    let check: String = src.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(|e| bad(&e))?;
    if check != "ok" {
        return Err(bad(&tr!("it is damaged", "elle est abîmée")));
    }
    for table in ["notes", "items", "pictures", "settings"] {
        let found: bool = src
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)", [table], |r| r.get(0))
            .map_err(|e| bad(&e))?;
        if !found {
            return Err(tr!("That folder isn't a Dobo backup. Nothing was changed.", "Ce dossier n’est pas une sauvegarde Dobo. Rien n’a été changé."));
        }
    }
    Ok(src)
}

/// The same for link previews' pictures.
fn repoint_previews(conn: &rusqlite::Connection, previews: &Path) -> Result<(), String> {
    for (id, path) in db::preview_images(conn) {
        let old = Path::new(&path);
        let Some(name) = old.file_name() else { continue };
        let here = previews.join(name);
        if !old.is_file() && here.is_file() {
            db::set_preview_image(conn, id, &here.to_string_lossy())?;
        }
    }
    Ok(())
}

/// Pictures saved on another computer (or under another user folder) point at a folder that
/// isn't here: they are pointed at the copies just put back next to the notes.
fn repoint_pictures(conn: &rusqlite::Connection, pictures: &Path) -> Result<(), String> {
    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT id, path FROM pictures")
        .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(|e| e.to_string())?;
    for (id, path) in rows {
        let old = Path::new(&path);
        let Some(name) = old.file_name() else { continue };
        let here = pictures.join(name);
        if !old.is_file() && here.is_file() {
            conn.execute("UPDATE pictures SET path = ?2 WHERE id = ?1", rusqlite::params![id, here.to_string_lossy().to_string()])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Board backgrounds ("picture" in the looks) that point at a file that isn't there any more:
/// pointed at the copy of the same name in `backgrounds`, when there is one.
fn repoint_backgrounds(conn: &rusqlite::Connection, backgrounds: &Path) -> Result<(), String> {
    fn walk(v: &mut serde_json::Value, dir: &Path) -> bool {
        match v {
            serde_json::Value::Object(m) => {
                let mut changed = false;
                for (k, x) in m.iter_mut() {
                    if k == "picture"
                        && let Some(s) = x.as_str()
                    {
                        let old = Path::new(s);
                        let Some(name) = old.file_name() else { continue };
                        let here = dir.join(name);
                        if !old.is_file() && here.is_file() {
                            *x = serde_json::json!(here.to_string_lossy());
                            changed = true;
                        }
                    } else {
                        changed |= walk(x, dir);
                    }
                }
                changed
            }
            serde_json::Value::Array(a) => a.iter_mut().fold(false, |c, x| walk(x, dir) | c),
            _ => false,
        }
    }
    for key in ["look_board", "look_styles", "look_themes"] {
        if let Some(mut v) = db::get_setting(conn, key)
            && walk(&mut v, backgrounds)
        {
            db::set_setting(conn, key, &v)?;
        }
    }
    Ok(())
}

/// A portable data folder that was moved (another drive letter on a USB stick, another folder):
/// the pictures, link previews' pictures and board backgrounds still name where they were. Each
/// one whose file is gone is pointed at its copy in this data folder. (Only for a data folder of
/// its own: the usual folder never moves.)
pub fn repoint_moved(conn: &rusqlite::Connection, data: &Path) -> Result<(), String> {
    repoint_pictures(conn, &data.join("pictures"))?;
    repoint_previews(conn, &data.join("previews"))?;
    repoint_backgrounds(conn, &data.join("backgrounds"))
}

/// Puts a backup back (after saving the current notes as a backup first).
pub fn restore(app: &AppHandle, path: &str) -> Result<(), String> {
    let dir = PathBuf::from(path);
    let file = dir.join(crate::paths::DB_FILE);
    if !file.is_file() {
        return Err(tr!("That folder isn't a Dobo backup.", "Ce dossier n’est pas une sauvegarde Dobo."));
    }
    let src = open_checked(&file)?;
    make(app, "before-restore")?;
    // Pictures (and link previews' pictures) first: adding files never harms the notes there now.
    let pictures = crate::paths::sub("pictures")?;
    let previews = crate::paths::sub("previews")?;
    copy_dir(&dir.join("pictures"), &pictures).map_err(|e| e.to_string())?;
    copy_dir(&dir.join("previews"), &previews).map_err(|e| e.to_string())?;
    {
        let state = app.state::<AppState>();
        let mut conn = state.db.lock().unwrap();
        {
            let b = rusqlite::backup::Backup::new(&src, &mut conn).map_err(|e| e.to_string())?;
            b.run_to_completion(256, std::time::Duration::ZERO, None).map_err(|e| e.to_string())?;
        }
        db::migrate(&conn)?;
        repoint_pictures(&conn, &pictures)?;
        repoint_previews(&conn, &previews)?;
    }
    // The restored settings work right away (shortcuts, edge opening…).
    crate::reload_settings(app);
    let _ = app.emit("reload", ());
    Ok(())
}

/// When a failed daily backup is tried again.
const RETRY_MS: i64 = 3_600_000;

/// Is a daily backup due now? Once per local day; after a failure, not again for an hour.
fn daily_due(last: i64, failed_at: i64, now: i64) -> bool {
    crate::clock::local_date(last) != crate::clock::local_date(now) && !(failed_at <= now && now - failed_at < RETRY_MS)
}

/// Once a day (the first look of the clock on a new day).
pub fn maybe_daily(app: &AppHandle, now: i64) {
    let (last, failed_at) = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        if db::get_setting(&conn, "backup_auto").and_then(|v| v.as_bool()) == Some(false) {
            return;
        }
        (
            db::get_setting(&conn, "backup_last").and_then(|v| v.as_i64()).unwrap_or(0),
            db::get_setting(&conn, "backup_failed_at").and_then(|v| v.as_i64()).unwrap_or(0),
        )
    };
    if !daily_due(last, failed_at, now) {
        return;
    }
    if let Err(e) = make(app, "daily") {
        eprintln!("Daily backup failed: {e}");
        // Try again in an hour, not every 5 seconds.
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let _ = db::set_setting(&conn, "backup_failed_at", &serde_json::json!(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dobo-backup-{tag}-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_failed_daily_backup_waits_an_hour() {
        let now = crate::clock::today_at(crate::db::now_ms(), "12:00").unwrap();
        let yesterday = now - 24 * 3_600_000;
        assert!(daily_due(yesterday, 0, now));
        assert!(!daily_due(yesterday, now - 60_000, now));
        assert!(daily_due(yesterday, now - RETRY_MS, now));
        assert!(!daily_due(now - 60_000, 0, now));
    }

    #[test]
    fn prune_keeps_14_real_backups_and_leaves_other_folders() {
        let root = temp_dir("prune");
        for d in 1..=20 {
            let dir = root.join(format!("ztodo-2026-10-{d:02}_090000-daily"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("ztodo.sqlite"), b"x").unwrap();
        }
        // Newer but unfinished (no notes inside), or not Dobo's: never counted nor removed.
        for name in ["ztodo-2026-11-30_090000-daily", "ztodo-notes", "Photos"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        prune(&root);
        let left: Vec<String> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(left.len(), 17);
        assert!(left.contains(&"ztodo-2026-10-07_090000-daily".to_string()));
        assert!(!left.contains(&"ztodo-2026-10-06_090000-daily".to_string()));
        assert!(left.contains(&"ztodo-notes".to_string()) && left.contains(&"Photos".to_string()));
    }

    /// Backups made before the rename ("ztodo-…") and after it ("dobo-…") are one list, by date:
    /// the newest are kept whatever their prefix.
    #[test]
    fn backups_from_before_the_rename_count_with_the_new_ones() {
        let root = temp_dir("renamed");
        let make = |name: &str| {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(crate::paths::DB_FILE), b"x").unwrap();
        };
        for d in 1..=10 {
            make(&format!("ztodo-2026-11-{d:02}_090000-daily"));
        }
        for d in 11..=20 {
            make(&format!("dobo-2026-11-{d:02}_090000-daily"));
        }
        assert!(is_backup(&root.join("ztodo-2026-11-01_090000-daily")) && is_backup(&root.join("dobo-2026-11-20_090000-daily")));
        prune(&root);
        let mut left: Vec<String> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        left.sort_by(|a, b| stamp_of(a).cmp(&stamp_of(b)));
        assert_eq!(left.len(), KEEP);
        assert_eq!(left.first().unwrap(), "ztodo-2026-11-07_090000-daily", "the oldest went, not the old-named ones first");
        assert_eq!(left.last().unwrap(), "dobo-2026-11-20_090000-daily");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_wrong_or_damaged_file_is_refused_before_anything_changes() {
        let dir = temp_dir("check");
        let other = dir.join("other.sqlite");
        rusqlite::Connection::open(&other).unwrap().execute_batch("CREATE TABLE t (x)").unwrap();
        assert!(open_checked(&other).is_err());
        let junk = dir.join("junk.sqlite");
        std::fs::write(&junk, vec![7u8; 8192]).unwrap();
        assert!(open_checked(&junk).is_err());
        let good = dir.join("good.sqlite");
        drop(crate::db::open(&good).unwrap());
        assert!(open_checked(&good).is_ok());
    }

    #[test]
    fn pictures_from_another_computer_point_here() {
        let dir = temp_dir("repoint");
        let pics = dir.join("pictures");
        std::fs::create_dir_all(&pics).unwrap();
        std::fs::write(pics.join("1-0.png"), b"png").unwrap();
        let conn = crate::db::open(&dir.join("a.sqlite")).unwrap();
        let n = crate::db::create_note(&conn, "Hub", "work", "soft", "Sage").unwrap();
        crate::db::add_picture(&conn, n.id, None, "Z:/Users/someone/AppData/Roaming/com.sookera.ztodo/pictures/1-0.png").unwrap();
        crate::db::add_picture(&conn, n.id, None, "Z:/elsewhere/missing.png").unwrap();
        repoint_pictures(&conn, &pics).unwrap();
        let paths = crate::db::picture_paths(&conn);
        assert!(paths.contains(&pics.join("1-0.png").to_string_lossy().to_string()));
        assert!(paths.contains(&"Z:/elsewhere/missing.png".to_string()));
    }

    #[test]
    fn a_moved_portable_folder_finds_its_pictures_and_backgrounds_again() {
        let old = temp_dir("moved-old").join("data");
        let new = temp_dir("moved-new").join("data");
        for d in ["pictures", "previews", "backgrounds"] {
            std::fs::create_dir_all(new.join(d)).unwrap();
        }
        std::fs::write(new.join("pictures").join("1-0.png"), b"png").unwrap();
        std::fs::write(new.join("previews").join("p.jpg"), b"jpg").unwrap();
        std::fs::write(new.join("backgrounds").join("9.jpg"), b"jpg").unwrap();
        let conn = crate::db::open(&new.join("ztodo.sqlite")).unwrap();
        let n = crate::db::create_note(&conn, "Hub", "work", "soft", "Sage").unwrap();
        crate::db::add_picture(&conn, n.id, None, &old.join("pictures").join("1-0.png").to_string_lossy()).unwrap();
        let link = serde_json::json!({ "url": "https://example.com", "title": "x", "image": old.join("previews").join("p.jpg").to_string_lossy() });
        conn.execute("INSERT INTO items (note_id, text, created_at, link) VALUES (?1, 'Read', 0, ?2)", rusqlite::params![n.id, link.to_string()]).unwrap();
        let bg = old.join("backgrounds").join("9.jpg").to_string_lossy().to_string();
        crate::db::set_setting(&conn, "look_board", &serde_json::json!({ "bg": "picture", "picture": bg })).unwrap();
        crate::db::set_setting(&conn, "look_styles", &serde_json::json!([{ "id": "s1", "look": { "bg": "picture", "picture": bg } }, { "id": "s2", "look": { "picture": "" } }])).unwrap();
        repoint_moved(&conn, &new).unwrap();
        assert_eq!(crate::db::picture_paths(&conn), vec![new.join("pictures").join("1-0.png").to_string_lossy().to_string()]);
        assert_eq!(crate::db::preview_images(&conn)[0].1, new.join("previews").join("p.jpg").to_string_lossy());
        let here = new.join("backgrounds").join("9.jpg").to_string_lossy().to_string();
        assert_eq!(crate::db::get_setting(&conn, "look_board").unwrap()["picture"], serde_json::json!(here));
        let styles = crate::db::get_setting(&conn, "look_styles").unwrap();
        assert_eq!(styles[0]["look"]["picture"], serde_json::json!(here));
        assert_eq!(styles[1]["look"]["picture"], serde_json::json!(""));
        // Run again (every start): nothing changes.
        repoint_moved(&conn, &new).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "look_board").unwrap()["picture"], serde_json::json!(here));
    }

    #[test]
    fn vacuum_into_and_backup_api_round_trip() {
        let dir = std::env::temp_dir().join(format!("dobo-backup-test-{}", crate::db::now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let conn = crate::db::open(&dir.join("a.sqlite")).unwrap();
        crate::db::create_note(&conn, "Hub", "work", "soft", "Sage").unwrap();
        let copy = dir.join("copy.sqlite");
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().to_string()]).unwrap();
        crate::db::create_note(&conn, "Later", "work", "soft", "Sky").unwrap();
        assert_eq!(crate::db::note_count(&conn), 2);
        let src = rusqlite::Connection::open(&copy).unwrap();
        let mut conn = conn;
        {
            let b = rusqlite::backup::Backup::new(&src, &mut conn).unwrap();
            b.run_to_completion(256, std::time::Duration::ZERO, None).unwrap();
        }
        assert_eq!(crate::db::note_count(&conn), 1);
    }
}
