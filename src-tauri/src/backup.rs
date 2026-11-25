// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Automatic backups: once a day a copy of the notes (the database, their pictures and the
//! board's background pictures; not the pictures' thumbs, made again from them) goes into a
//! folder of its own, in the backups folder chosen in Settings (default: inside the data folder,
//! `paths::data_dir`). "Back up now" and "Restore…" are in Settings, which also says when the
//! automatic one doesn't work.
//!
//! Each kind of backup keeps its own last few (`keep_of`), so one kind never pushes another out:
//! the last 14 daily ones, 10 made by hand, 5 made before a restore and 3 before an update. The
//! kind is in the folder's name ("dobo-<date>_<time>-<kind>"), as it always was: backups made
//! before the kinds were kept apart count as theirs, nothing is moved or renamed.
//! Before a new version of Dobo first opens the notes (and adds what it needs to them), a copy as
//! they were: "before-update-<version>" (`before_update`).

use crate::{AppState, db};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

/// How many daily backups are kept.
const DAILY_KEEP: usize = 14;

/// How many of a kind of backup are kept (None: a kind this version doesn't know, from a newer
/// one: never removed).
fn keep_of(kind: &str) -> Option<usize> {
    match kind {
        "daily" => Some(DAILY_KEEP),
        "manual" => Some(10),
        "before-restore" => Some(5),
        "before-update" => Some(3),
        _ => None,
    }
}

/// A backup folder's kind ("daily", "manual", "before-restore", "before-update"…) and, before an
/// update, the version it was for. (A name with no kind after its date: an early daily one.)
fn kind_of(name: &str) -> (String, Option<String>) {
    let rest = stamp_of(name).and_then(|s| s.get(STAMP_LEN..)).unwrap_or("");
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    if let Some(v) = rest.strip_prefix("before-update-") {
        return ("before-update".into(), Some(v.to_string()));
    }
    (if rest.is_empty() { "daily".into() } else { rest.to_string() }, None)
}

/// "2026-11-22_090000": the date and time a backup's name starts with (clock::stamp).
const STAMP_LEN: usize = 17;

#[derive(Serialize, Clone, Debug)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    /// Milliseconds (the folder's time).
    pub at: i64,
    pub bytes: u64,
    /// The kind: "daily", "manual", "before-restore" or "before-update".
    pub reason: String,
    /// Before an update: the version it was made for.
    pub version: Option<String>,
    /// In the data folder's own backups folder, not the one chosen in Settings: a backup before
    /// an update made there instead (`before_update`).
    pub kept_in_data: bool,
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

/// The files right in `from` copied into `to` (none already there is replaced). Only files: the
/// pictures' thumbs, in a folder of their own (thumbs.rs), are never in a backup, nor put back
/// from one; they are made again from the pictures.
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

/// One backup or restore at a time (the daily one, "Back up now" and "Restore…" may meet): each
/// clears what another left unfinished, never one still being made; and none is made, nor any
/// pruned, while a restore puts a backup's notes back. Always taken before the notes' lock, never
/// while holding it.
static MAKING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    MAKING.lock().unwrap_or_else(|e| e.into_inner())
}

/// The notes' lock (the app's connection).
fn held(notes: &std::sync::Mutex<rusqlite::Connection>) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
    notes.lock().unwrap_or_else(|e| e.into_inner())
}

/// How long a copy of the notes may wait for them (busy all along, a write held for minutes):
/// given up on rather than waited for forever.
const COPY_LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

/// Makes one backup now. Returns its folder.
pub fn make(app: &AppHandle, reason: &str) -> Result<BackupInfo, String> {
    let one = one_at_a_time();
    make_held(app, reason, None, &one).map(|m| m.info)
}

/// A backup just made, and the app's connection's count of changes its notes match (see
/// `make_at`).
struct Made {
    info: BackupInfo,
    changes: u64,
}

/// The same, `MAKING` already held (`_one`, by a restore); the backup `keep` (one being
/// restored) is never pruned to make room.
fn make_held(app: &AppHandle, reason: &str, keep: Option<&Path>, _one: &std::sync::MutexGuard<'static, ()>) -> Result<Made, String> {
    let now = db::now_ms();
    let root = folder(app)?;
    std::fs::create_dir_all(&root).map_err(|e| tr!("Couldn’t use the backups folder: {}", "Impossible d’utiliser le dossier des sauvegardes : {}", e))?;
    let name = format!("{PREFIX}{}-{reason}", crate::clock::stamp(now));
    if root.join(&name).exists() {
        return Err(tr!("A backup was just made. Try again in a second.", "Une sauvegarde vient d’être faite. Réessayez dans une seconde."));
    }
    // One left unfinished by a backup cut short goes now.
    clear_unfinished(&root);
    let data = crate::paths::data_dir()?;
    let tmp = crate::temp_dir(app)?;
    let state = app.state::<AppState>();
    let (dir, changes) = make_at(&root, &name, &data, &tmp, &state.db, now, COPY_LIMIT)?;
    prune(&root, keep);
    Ok(Made { info: BackupInfo { name, path: dir.to_string_lossy().into(), at: now, bytes: dir_size(&dir), reason: reason.into(), version: None, kept_in_data: false }, changes })
}

/// The backup `name` in `root` (`make_in`), then noted as made in the notes. Also returns the
/// count of changes made through the app's connection (`notes`) that the backup's notes match:
/// counted before they were copied, so a change made while they were (or since) never matches.
/// (The backup's own two settings, written once it is made, don't count.)
fn make_at(root: &Path, name: &str, data: &Path, tmp: &Path, notes: &std::sync::Mutex<rusqlite::Connection>, now: i64, limit: std::time::Duration) -> Result<(PathBuf, u64), String> {
    let seen = held(notes).total_changes();
    let dir = make_in(root, name, data, tmp, notes, limit)?;
    let conn = held(notes);
    let before = conn.total_changes();
    let _ = db::set_setting(&conn, "backup_last", &serde_json::json!(now));
    // Backups work again: no failure to tell about any more (nor a word that only said one
    // couldn't be made; one about a file kept somewhere stays while that file is there).
    let _ = conn.execute("DELETE FROM settings WHERE key = ?1", [FAILED_ERROR]);
    if db::get_setting(&conn, NOTICE_FILE).is_none() {
        let _ = conn.execute("DELETE FROM settings WHERE key IN (?1, ?2)", [NOTICE, NOTICE_AT]);
    }
    Ok((dir, seen + (conn.total_changes() - before)))
}

/// The backup `name` made in `root`: the notes (`notes`, the app's connection; their file is in
/// `data`) and the data folder's files. Made under a name of its own, then given its real name
/// once whole: a backup cut short (Dobo closed, the PC off, the drive pulled out) never looks
/// like one. Never while holding the notes (the backups folder may be a slow or missing drive:
/// the whole app would wait).
fn make_in(root: &Path, name: &str, data: &Path, tmp: &Path, notes: &std::sync::Mutex<rusqlite::Connection>, limit: std::time::Duration) -> Result<PathBuf, String> {
    let dir = root.join(name);
    let part = unfinished(root, name);
    std::fs::create_dir_all(&part).map_err(|e| e.to_string())?;
    let made = (|| -> Result<(), String> {
        copy_notes(notes, &data.join(crate::paths::DB_FILE), tmp, &part.join(crate::paths::DB_FILE), limit)?;
        copy_files(data, &part)?;
        std::fs::rename(&part, &dir).map_err(|e| e.to_string())
    })();
    if let Err(e) = made {
        // A half-made backup is not a backup: it must never be listed, restored or counted.
        let _ = std::fs::remove_dir_all(&part);
        return Err(e);
    }
    Ok(dir)
}

/// A copy of the notes (`notes`, the app's connection; `db`, their file) at `to`, made into a
/// local file in `tmp` first, then copied there: the notes are never held for as long as a slow
/// drive takes. With WAL, through a connection of its own: the app keeps reading and writing
/// meanwhile. With the classic journal (notes on a network share, where reading the whole file
/// may take longer than the app waits for it), a few pages at a time (`copy_paged`). The copy is
/// on the disk (flushed) before this returns: only then may the backup be given its real name.
fn copy_notes(notes: &std::sync::Mutex<rusqlite::Connection>, db: &Path, tmp: &Path, to: &Path, limit: std::time::Duration) -> Result<(), String> {
    std::fs::create_dir_all(tmp).map_err(|e| e.to_string())?;
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let local = tmp.join(format!("backup-{}-{}-{}.sqlite", db::now_ms(), std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let made = (|| {
        let mode: String = held(notes).query_row("PRAGMA journal_mode", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        if mode.eq_ignore_ascii_case("wal") {
            let conn = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
            conn.busy_timeout(std::time::Duration::from_secs(10)).map_err(|e| e.to_string())?;
            conn.execute("VACUUM INTO ?1", [local.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
        } else {
            // (Locked by SQLite too: the app's writes reach it, from the app's threads, while the
            // copy goes on: `copy_paged`.)
            let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX;
            let mut out = rusqlite::Connection::open_with_flags(&local, flags).map_err(|e| e.to_string())?;
            copy_paged(notes, &mut out, limit, || std::thread::sleep(std::time::Duration::from_millis(5)))?;
            drop(out);
        }
        std::fs::copy(&local, to).map_err(|e| e.to_string())?;
        // (A power cut may otherwise keep the backup's new name, a note of the folder, before
        // the file's own bytes: a backup that looks whole and isn't.)
        std::fs::OpenOptions::new().write(true).open(to).and_then(|f| f.sync_all()).map_err(|e| e.to_string())
    })();
    let _ = std::fs::remove_file(&local);
    made
}

/// How many pages `copy_paged` copies in its first steps.
const FIRST_STEP: std::ffi::c_int = 64;
/// The most it ever copies in one step (16 MB of 4 KB pages): never the whole file at once.
const LAST_STEP: std::ffi::c_int = 4096;

/// How many pages a step of `copy_paged` copies after `restarts`: 64, then 512, then 4096.
fn step_pages(restarts: u32) -> std::ffi::c_int {
    (FIRST_STEP << (3 * restarts.min(2))).min(LAST_STEP)
}

/// What `copy_paged` met on its way.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Copied {
    /// Times it had to start again (the notes changed other than through the app's connection).
    restarts: u32,
    /// Steps that found the notes busy (being written), tried again.
    busy: u32,
}

/// The notes (`notes`, the app's own connection) into `to`, a few pages at a time, holding the
/// notes' lock only for each step, never for the whole file: the app writes between steps
/// (`between`), waiting at most one step. Its writes reach the copy as they are made (SQLite
/// keeps a copy made from a connection up to date with what that connection writes), so typing
/// all along never starts the copy again, and it always ends. Should the file change some other
/// way (each such change starts the copy again), the steps grow (`step_pages`) so that it still
/// ends, never past `LAST_STEP`; and after `limit`, it is given up on.
fn copy_paged(notes: &std::sync::Mutex<rusqlite::Connection>, to: &mut rusqlite::Connection, limit: std::time::Duration, mut between: impl FnMut()) -> Result<Copied, String> {
    use rusqlite::ffi;
    let said = |rc: std::ffi::c_int| {
        // SAFETY: SQLite's own text for a result code, a static NUL-terminated string.
        unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_errstr(rc)) }.to_string_lossy().into_owned()
    };
    /// The copy under way, ended under the notes' lock too, however this ends (the app's
    /// connection keeps a note of it while it lasts).
    struct Copy<'a> {
        b: *mut ffi::sqlite3_backup,
        notes: &'a std::sync::Mutex<rusqlite::Connection>,
    }
    impl Copy<'_> {
        fn end(&mut self) -> std::ffi::c_int {
            if self.b.is_null() {
                return ffi::SQLITE_OK;
            }
            let _conn = held(self.notes);
            // SAFETY: a copy started below and not ended yet; both connections are still open.
            let rc = unsafe { ffi::sqlite3_backup_finish(self.b) };
            self.b = std::ptr::null_mut();
            rc
        }
    }
    impl Drop for Copy<'_> {
        fn drop(&mut self) {
            self.end();
        }
    }
    let main = c"main";
    let b = {
        let conn = held(notes);
        // SAFETY: two open connections that outlive the copy (it ends before this returns, in
        // `Copy::end`). The app's connection is only ever used under its lock, held for every
        // call on the copy; `to` is not used by anything else while it lasts (borrowed here).
        unsafe { ffi::sqlite3_backup_init(to.handle(), main.as_ptr(), conn.handle(), main.as_ptr()) }
    };
    if b.is_null() {
        // SAFETY: `to` is open; its last error is a NUL-terminated string it owns.
        return Err(unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_errmsg(to.handle())) }.to_string_lossy().into_owned());
    }
    let mut copy = Copy { b, notes };
    let mut copied = Copied::default();
    let (mut done, until) = (0, std::time::Instant::now() + limit);
    loop {
        let pages = step_pages(copied.restarts);
        let (rc, now) = {
            let _conn = held(notes);
            // SAFETY: the copy is under way (not ended), the notes' lock held.
            unsafe { (ffi::sqlite3_backup_step(copy.b, pages), ffi::sqlite3_backup_pagecount(copy.b) - ffi::sqlite3_backup_remaining(copy.b)) }
        };
        match rc & 0xff {
            ffi::SQLITE_DONE => break,
            ffi::SQLITE_OK => {
                // Fewer pages copied than one more step's worth: it started again from the first.
                if now < done + pages {
                    copied.restarts += 1;
                }
                done = now;
            }
            // (The app is writing, or something else holds the notes: tried again.)
            ffi::SQLITE_BUSY | ffi::SQLITE_LOCKED => copied.busy += 1,
            _ => return Err(said(rc)),
        }
        if std::time::Instant::now() >= until {
            return Err(tr!("The notes stayed busy too long to copy.", "Les notes sont restées occupées trop longtemps pour être copiées."));
        }
        between();
    }
    match copy.end() {
        ffi::SQLITE_OK => Ok(copied),
        rc => Err(said(rc)),
    }
}

/// Where a backup is made before it is whole: ".dobo-<date>_<time>-<reason>.part" (never one of
/// Dobo's backups for `is_backup`, whatever it holds).
fn unfinished(root: &Path, name: &str) -> PathBuf {
    root.join(format!(".{name}.part"))
}

/// Backups that were never finished (cut short by a crash or the PC turned off).
fn clear_unfinished(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let ours = name.strip_prefix('.').and_then(|n| n.strip_suffix(".part")).is_some_and(|n| stamp_of(n).is_some());
        if ours && e.path().is_dir() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
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

/// The same folder, however its path is written.
fn same_dir(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()).to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    norm(a) == norm(b)
}

/// Keeps the newest few of each kind of Dobo's own backup folders (`keep_of`), and `keep` (a
/// backup being restored: it is still being read, and removing it would take its pictures from the
/// notes it brings back).
fn prune(root: &Path, keep: Option<&Path>) {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .map(|it| {
            it.flatten()
                .filter(|e| e.path().is_dir() && is_backup(&e.path()) && !keep.is_some_and(|k| same_dir(&e.path(), k)))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort_by(|a, b| stamp_of(a).cmp(&stamp_of(b)));
    let mut kinds: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for n in names {
        kinds.entry(kind_of(&n).0).or_default().push(n);
    }
    for (kind, mut names) in kinds {
        let Some(max) = keep_of(&kind) else { continue };
        while names.len() > max {
            let old = names.remove(0);
            let _ = std::fs::remove_dir_all(root.join(old));
        }
    }
}

/// The backups listed in Settings: those in the backups folder (`folder`), and, when one was
/// chosen there, the backups made before an update in the data folder's own backups folder
/// instead (`before_update`: the chosen one couldn't be used then). Newest first.
pub fn list(app: &AppHandle) -> Result<Vec<BackupInfo>, String> {
    let root = folder(app)?;
    let own = crate::paths::data_dir()?.join("backups");
    Ok(list_in(&root, &own))
}

/// `list`, for the backups folder `root` and the data folder's own `own`.
fn list_in(root: &Path, own: &Path) -> Vec<BackupInfo> {
    let read = |dir: &Path, kept_in_data: bool| -> Vec<BackupInfo> {
        std::fs::read_dir(dir)
            .map(|it| {
                it.flatten()
                    .filter(|e| is_backup(&e.path()))
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        let at = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
                        let (reason, version) = kind_of(&name);
                        BackupInfo { path: e.path().to_string_lossy().into(), bytes: dir_size(&e.path()), name, at, reason, version, kept_in_data }
                    })
                    .filter(|b| !kept_in_data || b.reason == "before-update")
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = read(root, false);
    if !same_dir(root, own) {
        out.extend(read(own, true));
    }
    out.sort_by(|a, b| stamp_of(&b.name).cmp(&stamp_of(&a.name)));
    out
}

/// Opens a backup's notes and checks they are whole and really Dobo's, before anything is
/// replaced with them.
fn open_checked(file: &Path) -> Result<rusqlite::Connection, String> {
    let bad = |e: &dyn std::fmt::Display| tr!("That backup can’t be read ({}). Nothing was changed.", "Cette sauvegarde ne peut pas être lue ({}). Rien n’a été changé.", e);
    let src = rusqlite::Connection::open(file).map_err(|e| bad(&e))?;
    db::harden(&src).map_err(|e| bad(&e))?;
    let check: String = src.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(|e| bad(&e))?;
    if check != "ok" {
        return Err(bad(&tr!("it is damaged", "elle est endommagée")));
    }
    for table in ["notes", "items", "pictures", "settings"] {
        let found: bool = src
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)", [table], |r| r.get(0))
            .map_err(|e| bad(&e))?;
        if !found {
            return Err(tr!("That folder isn’t a Dobo backup. Nothing was changed.", "Ce dossier n’est pas une sauvegarde Dobo. Rien n’a été changé."));
        }
    }
    // Repeating tasks' history (1.2.1), when it has one: as Dobo keeps it, or the notes it brings
    // couldn't be read once put back (an older backup has none: it is made when restored).
    let cols: Vec<String> = src
        .prepare("SELECT name FROM pragma_table_info('done_log')")
        .and_then(|mut st| st.query_map([], |r| r.get(0))?.collect())
        .map_err(|e| bad(&e))?;
    if !cols.is_empty() && !["id", "item_id", "due_at", "done_at"].iter().all(|c| cols.iter().any(|x| x == c)) {
        return Err(tr!("That folder isn’t a Dobo backup. Nothing was changed.", "Ce dossier n’est pas une sauvegarde Dobo. Rien n’a été changé."));
    }
    Ok(src)
}

/// Are the notes in `file` whole and Dobo's (`open_checked`)? For the recovery screen: a backup
/// before anything is put aside for it, a repaired copy before it is offered.
pub fn whole(file: &Path) -> Result<(), String> {
    open_checked(file).map(|_| ())
}

/// The same for link previews' pictures.
fn repoint_previews(conn: &rusqlite::Connection, previews: &Path) -> Result<(), String> {
    for (id, path) in db::preview_images(conn)? {
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

/// The folders of files a backup keeps with the notes: pictures, link previews' pictures and
/// the board's background pictures (their files, not the pictures' thumbs: `copy_dir`).
const FILE_FOLDERS: [&str; 3] = ["pictures", "previews", "backgrounds"];

/// The data folder's files, into a backup being made.
fn copy_files(data: &Path, backup: &Path) -> Result<(), String> {
    for folder in FILE_FOLDERS {
        copy_dir(&data.join(folder), &backup.join(folder)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A backup's notes over the open ones, its files into the data folder first (adding files
/// never harms the notes there now), then everything they name pointed at those files.
#[cfg(test)]
fn put_back(conn: &mut rusqlite::Connection, src: &rusqlite::Connection, backup: &Path, data: &Path) -> Result<(), String> {
    put_back_files(backup, data)?;
    put_back_notes(conn, src, data)
}

/// A backup's files into the data folder (only adds files, so done before the notes are held:
/// the backup may be on a slow drive).
fn put_back_files(backup: &Path, data: &Path) -> Result<(), String> {
    for folder in FILE_FOLDERS {
        copy_dir(&backup.join(folder), &data.join(folder)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A backup's notes over the open ones (`put_back_files` first), everything they name pointed at
/// the files here. All of it is made ready on a copy first (a backup from another computer or an
/// odd older build may not take a step): the open notes change only once it all worked, in one
/// step that either works or changes nothing.
#[cfg(test)]
fn put_back_notes(conn: &mut rusqlite::Connection, src: &rusqlite::Connection, data: &Path) -> Result<(), String> {
    // The ids given so far: after an older backup, never given again (they may still be named
    // outside the notes file, by a notification's buttons for one).
    let ready = prepared(src, &sequences(conn), data)?;
    put_ready(conn, &ready)
}

/// The backup's notes made ready (`prepared`) over the open ones. Holding the notes only for
/// this: the ids given so far are read again (the app may have given some since `prepared`), so
/// none of them is ever given again, then the copy, in one step: SQLite keeps the open notes'
/// write transaction for the whole copy and commits it only at the end, so a failure here leaves
/// them as they were.
fn put_ready(conn: &mut rusqlite::Connection, ready: &rusqlite::Connection) -> Result<(), String> {
    keep_given(ready, &sequences(conn))?;
    keep_local(ready, |k| db::get_setting(conn, k))?;
    keep_today(ready, Some(conn), db::now_ms())?;
    let b = rusqlite::backup::Backup::new(ready, conn).map_err(|e| e.to_string())?;
    match b.step(-1).map_err(|e| e.to_string())? {
        rusqlite::backup::StepResult::Done => Ok(()),
        other => Err(format!("the notes are in use ({other:?})")),
    }
}

/// `put_ready` for a restore whose backup of the notes as they were matches `seen`, a count of
/// the app's connection's changes (`make_at`). The board stays usable while the backup to put
/// back is read and made ready: anything written meanwhile is copied to `snapshot` first (a local
/// file: quick), to be put in that backup's place once the notes are let go (`replace_notes`),
/// so that it always holds exactly the notes replaced. Returns whether there was anything; if
/// the notes couldn't be replaced, `snapshot` is gone again.
fn put_ready_keeping(conn: &mut rusqlite::Connection, ready: &rusqlite::Connection, seen: u64, snapshot: &Path) -> Result<bool, String> {
    let changed = conn.total_changes() != seen;
    if changed {
        let _ = std::fs::remove_file(snapshot);
        conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
    }
    if let Err(e) = put_ready(conn, ready) {
        let _ = std::fs::remove_file(snapshot);
        return Err(e);
    }
    Ok(changed)
}

/// The notes file `to` (in a backup) replaced by `with`, in one step: it is either the one or
/// the other, whole, never a mix (made beside it under a name of its own, on the disk, then
/// renamed over it).
fn replace_notes(with: &Path, to: &Path) -> Result<(), String> {
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part = to.with_file_name(format!(".{name}.part"));
    let done = (|| {
        std::fs::copy(with, &part)?;
        std::fs::OpenOptions::new().write(true).open(&part)?.sync_all()?;
        std::fs::rename(&part, to)
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    done.map_err(|e| e.to_string())
}

/// Settings that belong to this computer, never brought by a backup (it may be someone else's, or
/// from another day): the computers file cards may open files from (files.rs), which only the
/// person here allows; where the backups go here (another computer's folder may not be there, or
/// be someone else's); and how the backups stand here (a failure or a word from the backup's own
/// time would be said again, out of date); and the last What's new shown here (an older backup
/// would show it again: whatsnew.rs).
const LOCAL_SETTINGS: [&str; 9] = ["file_hosts", "backup_dir", FAILED_ERROR, "backup_failed_at", "backup_last", NOTICE, NOTICE_AT, NOTICE_FILE, crate::whatsnew::SEEN];

/// Of those, the person's own choices, carried from notes that were put aside on the recovery
/// screen when they can still be read (`put_back_into`): not how the backups stood for them.
pub const CHOSEN_HERE: [&str; 2] = ["file_hosts", "backup_dir"];

/// `ready` (a backup's notes made ready) keeps this computer's own settings (`LOCAL_SETTINGS`) as
/// `live` gives them (none: not set here).
fn keep_local(ready: &rusqlite::Connection, live: impl Fn(&str) -> Option<serde_json::Value>) -> Result<(), String> {
    for key in LOCAL_SETTINGS {
        match live(key) {
            Some(v) => db::set_setting(ready, key, &v)?,
            None => {
                ready.execute("DELETE FROM settings WHERE key = ?1", [key]).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// The evening's look back and Tidy's reminder as they are today, whatever day the backup is
/// from: an older backup never brings back a look back already seen or closed, nor a Tidy already
/// answered (`ready`: a backup's notes made ready; `live`: the notes it replaces, or None on the
/// recovery screen, where what was done today up to `now` counts as done).
fn keep_today(ready: &rusqlite::Connection, live: Option<&rusqlite::Connection>, now: i64) -> Result<(), String> {
    let int = |c: &rusqlite::Connection, k: &str| db::get_setting(c, k).and_then(|v| v.as_i64());
    let text = |c: &rusqlite::Connection, k: &str| db::get_setting(c, k).and_then(|v| v.as_str().map(String::from));
    // When each last rang or was answered: the later of the backup's and today's.
    for key in ["lookback_last", "tidy_last"] {
        let today = match live {
            Some(c) => int(c, key),
            None => Some(now),
        };
        if let Some(t) = today.filter(|t| int(ready, key).is_none_or(|b| *t > b)) {
            db::set_setting(ready, key, &serde_json::json!(t))?;
        }
    }
    if let Some(c) = live {
        // Tidy's "later" as it is now (the backup's belongs to its own day).
        match int(c, "tidy_snooze") {
            Some(s) => db::set_setting(ready, "tidy_snooze", &serde_json::json!(s))?,
            None => {
                ready.execute("DELETE FROM settings WHERE key = 'tidy_snooze'", []).map_err(|e| e.to_string())?;
            }
        }
        // The last day whose look back was closed ("YYYY-MM-DD": the later one).
        if let Some(d) = text(c, "lookback_dismissed").filter(|d| text(ready, "lookback_dismissed").is_none_or(|b| d.as_str() > b.as_str())) {
            db::set_setting(ready, "lookback_dismissed", &serde_json::json!(d))?;
        }
    }
    Ok(())
}

/// The ids given so far in each table (`given`) never given again by `conn`.
fn keep_given(conn: &rusqlite::Connection, given: &[(String, i64)]) -> Result<(), String> {
    for (table, seq) in given {
        let raised = conn.execute("UPDATE sqlite_sequence SET seq = MAX(seq, ?2) WHERE name = ?1", rusqlite::params![table, seq]).map_err(|e| e.to_string())?;
        // (A table the backup had never given an id in has no record yet.)
        if raised == 0 {
            conn.execute("INSERT INTO sqlite_sequence (name, seq) VALUES (?1, ?2)", rusqlite::params![table, seq]).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// A backup's notes, made ready off to the side (in memory, without holding the open notes: the
/// backup may be on a slow drive): what Dobo never makes taken out, the columns of today's Dobo
/// added, the ids given so far (`given`) never given again, and the files they name pointed at
/// those in `data`.
fn prepared(src: &rusqlite::Connection, given: &[(String, i64)], data: &Path) -> Result<rusqlite::Connection, String> {
    let mut ready = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    db::harden(&ready).map_err(|e| e.to_string())?;
    {
        let b = rusqlite::backup::Backup::new(src, &mut ready).map_err(|e| e.to_string())?;
        b.run_to_completion(256, std::time::Duration::ZERO, None).map_err(|e| e.to_string())?;
    }
    // Before anything writes to them: what Dobo never makes goes (a backup from someone else may
    // bring a trigger that deletes the notes on the next change, or a view).
    strip_foreign_schema(&ready)?;
    db::migrate(&ready)?;
    // (Made ready by this version: its next start has nothing to back up first, `before_update`.)
    db::set_setting(&ready, OPENED_BY, &serde_json::json!(env!("CARGO_PKG_VERSION")))?;
    keep_given(&ready, given)?;
    repoint_moved(&ready, data)?;
    Ok(ready)
}

/// Takes out the triggers and views a restored file brought: Dobo makes none.
fn strip_foreign_schema(conn: &rusqlite::Connection) -> Result<(), String> {
    let found: Vec<(String, String)> = conn
        .prepare("SELECT type, name FROM sqlite_master WHERE type IN ('trigger', 'view')")
        .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(|e| e.to_string())?;
    for (kind, name) in found {
        let what = if kind == "trigger" { "TRIGGER" } else { "VIEW" };
        conn.execute_batch(&format!("DROP {what} IF EXISTS \"{}\"", name.replace('"', "\"\""))).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The highest id given so far in each table (AUTOINCREMENT's own record).
fn sequences(conn: &rusqlite::Connection) -> Vec<(String, i64)> {
    conn.prepare("SELECT name, seq FROM sqlite_sequence").and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect()).unwrap_or_default()
}

/// Every line's id.
fn item_ids(conn: &rusqlite::Connection) -> Vec<i64> {
    conn.prepare("SELECT id FROM items").and_then(|mut st| st.query_map([], |r| r.get(0))?.collect()).unwrap_or_default()
}

/// Puts a backup back (after saving the current notes as a backup first).
pub fn restore(app: &AppHandle, path: &str) -> Result<(), String> {
    let dir = PathBuf::from(path);
    let file = dir.join(crate::paths::DB_FILE);
    if !file.is_file() {
        return Err(tr!("That folder isn’t a Dobo backup.", "Ce dossier n’est pas une sauvegarde Dobo."));
    }
    let src = open_checked(&file)?;
    // No backup made meanwhile, all through (`MAKING`, then the notes' lock, as everywhere): a
    // daily one could prune this backup's pictures, or hold the notes when they are put back.
    let one = one_at_a_time();
    // (The notes as they are now, kept first. Never in place of the backup being restored: with
    // 14 backups already, the oldest would go to make room, and it may be this one.)
    let before = make_held(app, "before-restore", Some(&dir), &one)?;
    let data = crate::paths::data_dir()?;
    // The pictures first, without holding the notes (a slow drive would hold up the whole app).
    put_back_files(&dir, &data)?;
    let tmp = crate::temp_dir(app)?;
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let snapshot = tmp.join(format!("before-restore-{}-{}.sqlite", db::now_ms(), std::process::id()));
    // The backup's notes made ready without holding the open ones (read from a slow drive, the
    // whole app would wait), then put over them in one step under the lock (`put_ready`, which
    // reads the ids given so far again: any given meanwhile are kept from being given again).
    let state = app.state::<AppState>();
    let given = sequences(&held(&state.db));
    let (shown, put) = match prepared(&src, &given, &data) {
        Ok(ready) => {
            let mut conn = held(&state.db);
            let shown = item_ids(&conn);
            (shown, put_ready_keeping(&mut conn, &ready, before.changes, &snapshot))
        }
        Err(e) => (Vec::new(), Err(e)),
    };
    let written_meanwhile = match put {
        Ok(w) => w,
        Err(e) => {
            // Nothing was changed (`put_ready` changes the notes in one step or not at all);
            // the board is read again all the same, so it can never act on rows it doesn't show.
            let _ = app.emit("reload", ());
            return Err(tr!("That backup couldn’t be put back ({}). Nothing was changed.", "Cette sauvegarde n’a pas pu être restaurée ({}). Rien n’a été changé.", e));
        }
    };
    // What was written while the backup was made ready goes into the backup of the notes as
    // they were (it was made before). Should that fail (the backups drive gone just now), the
    // copy stays where it is, and Settings › Backups says where.
    if written_meanwhile {
        match replace_notes(&snapshot, &Path::new(&before.info.path).join(crate::paths::DB_FILE)) {
            Ok(()) => {
                let _ = std::fs::remove_file(&snapshot);
            }
            Err(e) => {
                eprintln!("The backup made before the restore couldn't be brought up to date: {e}");
                let said = Notice::RestoreMissed { error: e.to_string(), path: snapshot.display().to_string() };
                // (A word of its own, not a failed automatic backup: said until that file goes.)
                note_notice(&held(&state.db), db::now_ms(), &said, Some(&snapshot));
            }
        }
    }
    // The notifications of the lines there were: their buttons name lines by id, and after an
    // older backup a new line may be given one of those ids ("Done" would tick it).
    crate::clear_items(app, &shown);
    // The restored settings work right away (shortcuts, edge opening…).
    crate::reload_settings(app);
    let _ = app.emit("reload", ());
    Ok(())
}

/// Dobo's own backups in `roots` (the folder chosen in Settings, the default one), newest first:
/// the first whose notes are whole (`open_checked`). For the recovery screen, when the notes
/// themselves can't be opened.
pub fn newest_whole(roots: &[PathBuf]) -> Option<BackupInfo> {
    // (Each folder read on a thread of its own, within `REACH_LIMIT`: the chosen one may be a
    // network drive that is gone, which can take a minute to say so. Not read: not looked in.)
    let mut found: Vec<PathBuf> = roots
        .iter()
        .flat_map(|r| {
            let r = r.clone();
            within(REACH_LIMIT, move || std::fs::read_dir(&r).map(|it| it.flatten().map(|e| e.path()).filter(|p| is_backup(p)).collect::<Vec<_>>()).unwrap_or_default()).unwrap_or_default()
        })
        .collect();
    found.sort_by(|a, b| {
        let s = |p: &PathBuf| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        stamp_of(&s(b)).map(String::from).cmp(&stamp_of(&s(a)).map(String::from))
    });
    found.dedup_by(|a, b| same_dir(a, b));
    let dir = found.into_iter().find(|d| open_checked(&d.join(crate::paths::DB_FILE)).is_ok())?;
    let name = dir.file_name()?.to_string_lossy().to_string();
    let at = std::fs::metadata(&dir).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
    let (reason, version) = kind_of(&name);
    Some(BackupInfo { path: dir.to_string_lossy().into(), bytes: dir_size(&dir), name, at, reason, version, kept_in_data: false })
}

/// The ids given so far in a notes file that may be damaged (none when they can't be read): after
/// it is replaced, never given again.
pub fn sequences_of(file: &Path) -> Vec<(String, i64)> {
    rusqlite::Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map(|c| sequences(&c))
        .unwrap_or_default()
}

/// A backup put back where there are no notes (the recovery screen, Dobo not started yet): its
/// pictures into `data`, its notes made ready (`prepared`) and written as the notes file in one
/// step (made beside it under a name of its own, on the disk, then renamed). There must be no
/// notes file in `data` (a damaged one was put aside first). `here`: this computer's own choices
/// (`CHOSEN_HERE`) as the notes put aside had them, and what What's new still has to show (`whatsnew::SEEN`).
pub fn put_back_into(backup: &Path, data: &Path, given: &[(String, i64)], here: &[(String, serde_json::Value)]) -> Result<(), String> {
    let couldnt = |e: &dyn std::fmt::Display| tr!("That backup couldn’t be put back ({}).", "Cette sauvegarde n’a pas pu être restaurée ({}).", e);
    let to = data.join(crate::paths::DB_FILE);
    if to.exists() {
        return Err(couldnt(&"notes are there"));
    }
    let src = open_checked(&backup.join(crate::paths::DB_FILE))?;
    put_back_files(backup, data).map_err(|e| couldnt(&e))?;
    let ready = prepared(&src, given, data).map_err(|e| couldnt(&e))?;
    keep_local(&ready, |k| here.iter().find(|(key, _)| key == k && (CHOSEN_HERE.contains(&k) || k == crate::whatsnew::SEEN)).map(|(_, v)| v.clone())).map_err(|e| couldnt(&e))?;
    keep_today(&ready, None, db::now_ms()).map_err(|e| couldnt(&e))?;
    let part = data.join(format!(".{}.part", crate::paths::DB_FILE));
    let _ = std::fs::remove_file(&part);
    let done = (|| -> Result<(), String> {
        ready.execute("VACUUM INTO ?1", [part.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
        std::fs::OpenOptions::new().write(true).open(&part).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &to).map_err(|e| e.to_string())
    })();
    if let Err(e) = done {
        let _ = std::fs::remove_file(&part);
        return Err(couldnt(&e));
    }
    Ok(())
}

/// The setting saying which version of Dobo last opened the notes (none: before 1.1).
pub const OPENED_BY: &str = "app_version";
/// A backup made before a new version first opened the notes: "dobo-<date>_<time>-before-update-<version>".
const BEFORE_UPDATE: &str = "-before-update-";

/// Is `stored` (the version that last opened the notes; none before 1.1) older than `running`?
/// A version that can't be read counts as older: a copy too many is better than none.
fn older(stored: Option<&str>, running: &str) -> bool {
    let parse = |v: &str| semver::Version::parse(v.trim()).ok();
    match (stored.and_then(parse), parse(running)) {
        (Some(s), Some(r)) => s < r,
        (_, Some(_)) => true,
        (_, None) => false,
    }
}

/// Before a new version of Dobo (`running`) first opens the notes (`conn`, opened as they are, not
/// yet changed: `db::open` adds what the new version needs), a backup of them as they are, with
/// their pictures, in the backups folder (the one chosen in Settings, or `data`'s): its own kind,
/// "before-update-<version>". None when none is needed (the notes were opened by this version or
/// a newer one, or there are none yet), or one was made for this version already (a start that
/// went no further: never a new one at each start, pushing out the others).
///
/// It never holds up the start for long. The notes are copied at once into the data folder (one
/// local file, never skipped), then into the chosen folder: everything done there (reaching it,
/// looking in it, the copy) has `UPDATE_LIMIT` in all, else the backup goes into the data
/// folder's own `backups` (said in Settings › Backups: `UpdateBackup.instead_of`, with why). The
/// pictures, link previews' pictures and backgrounds come after: linked at once where they can be
/// (the data folder's own backups, on the same drive), else copied by `finish_later` once the
/// board has started (they are still in the data folder). Until then the backup says so
/// (`FILES_TO_COME`): a start that ends first leaves it to the next one.
pub fn before_update(conn: &rusqlite::Connection, data: &Path, running: &str, now: i64) -> Result<Option<UpdateBackup>, UpdateFailed> {
    let fail = |e: String| UpdateFailed { why: Missed::of(&e), error: e };
    let has_notes: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'notes')", [], |r| r.get(0)).map_err(|e| fail(e.to_string()))?;
    let stored = db::get_setting(conn, OPENED_BY).and_then(|v| v.as_str().map(String::from));
    if !has_notes || !older(stored.as_deref(), running) {
        return Ok(None);
    }
    let default = data.join("backups");
    if made_for(&default, running) {
        return Ok(None);
    }
    // (Compared as written: a share that is gone may take a minute to say where it leads.)
    let as_written = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let chosen = db::get_setting(conn, "backup_dir")
        .and_then(|v| v.as_str().map(PathBuf::from))
        .filter(|p| !p.as_os_str().is_empty() && !p.to_string_lossy().trim().is_empty() && as_written(p) != as_written(&default));
    let name = format!("{PREFIX}{}{BEFORE_UPDATE}{running}", crate::clock::stamp(now));
    // The notes, copied at once (nothing else has them yet) into a local file.
    let copy = data.join(NOTES_COPY);
    let _ = std::fs::remove_file(&copy);
    let copied = (|| -> Result<(), String> {
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
        std::fs::OpenOptions::new().write(true).open(&copy).and_then(|f| f.sync_all()).map_err(|e| e.to_string())
    })();
    let made = copied.map_err(fail).and_then(|()| {
        let mut instead_of = None;
        if let Some(c) = chosen {
            match into_chosen(&c, &name, UPDATE_LIMIT, chosen_work(&c, &copy, &name, running)) {
                Chosen::Made(dir) => return Ok(Some(UpdateBackup { dir, instead_of: None })),
                Chosen::There => return Ok(None),
                Chosen::Missed(why) => instead_of = Some((c, why)),
            }
        }
        let dir = into_root(&default, &copy, &name, true, &|| true).map_err(fail)?;
        // (Same drive: linked at once. Else, or should a link be refused: `finish_later`.)
        if link_files(data, &dir, &std::sync::atomic::AtomicBool::new(false), false).is_ok_and(|all| all) {
            let _ = std::fs::remove_file(dir.join(FILES_TO_COME));
        }
        Ok(Some(UpdateBackup { dir, instead_of }))
    });
    let _ = std::fs::remove_file(&copy);
    made
}

/// A backup made before an update (`before_update`).
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateBackup {
    pub dir: PathBuf,
    /// Made in the data folder's own backups folder instead of this one, the one chosen in
    /// Settings, and why.
    pub instead_of: Option<(PathBuf, Missed)>,
}

/// No backup could be made before an update (`before_update`): Windows' or SQLite's own words,
/// and what they come to.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateFailed {
    pub error: String,
    pub why: Missed,
}

/// Why a backup before an update didn't go where it should: a word of its own in Settings ›
/// Backups, in the language of the moment (src/lib/backups.ts `noticeText`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Missed {
    /// No answer in time (a network drive that is gone, a drive asleep or too slow).
    NoAnswer,
    /// It can't be written in (read-only, no permission).
    NotWritable,
    /// Its drive is full.
    Full,
    /// Anything else (Windows' or SQLite's own words say what).
    Failed,
}

impl Missed {
    /// What an error (Windows' or SQLite's words) comes to.
    fn of(error: &str) -> Missed {
        let e = error.to_ascii_lowercase();
        if e.contains("os error 112)") || e.contains("os error 39)") || e.contains("disk is full") {
            Missed::Full
        } else if e.contains("os error 5)") || e.contains("os error 19)") || e.contains("readonly") || e.contains("read-only") || e.contains("unable to open database file") {
            Missed::NotWritable
        } else {
            Missed::Failed
        }
    }
}

/// How long everything done in the chosen backups folder for a backup before an update may take
/// (reaching it, looking in it, copying the notes there), before it goes into the data folder's
/// own instead.
const UPDATE_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);
/// How long the chosen backups folder may take to answer when it is only read (the recovery
/// screen's newest backup).
const REACH_LIMIT: std::time::Duration = std::time::Duration::from_secs(3);
/// The notes' copy for a backup before an update, in the data folder until it is in the backup.
const NOTES_COPY: &str = ".before-update.sqlite.part";
/// In a backup made before an update whose files are still to come (`finish_later`).
const FILES_TO_COME: &str = ".files-to-come";

/// `f`'s answer, given within `limit` (on a thread of its own, left to end by itself if not).
fn within<T: Send + 'static>(limit: std::time::Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).ok()
}

/// Is there a finished backup made before `running` first opened the notes, in `root`?
fn made_for(root: &Path, running: &str) -> bool {
    std::fs::read_dir(root).is_ok_and(|it| {
        it.flatten().any(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            is_backup(&e.path()) && kind_of(&name) == ("before-update".to_string(), Some(running.to_string()))
        })
    })
}

/// What became of the chosen backups folder (`into_chosen`).
#[derive(Debug, PartialEq)]
enum Chosen {
    Made(PathBuf),
    /// A backup for this version is there already.
    There,
    Missed(Missed),
}

/// The backup made in the chosen backups folder (`root`) by `work` (on a thread of its own: a
/// backup there of the notes' copy, `chosen_work`), all of it within `limit`. Given up on after
/// it (`work`'s `may_name` then says no: what it made goes), unless it was already giving the
/// backup its name (`name`): then its answer, or the backup itself, is waited for a little, never
/// made twice.
fn into_chosen(root: &Path, name: &str, limit: std::time::Duration, work: impl FnOnce(&dyn Fn() -> bool) -> Result<Option<PathBuf>, String> + Send + 'static) -> Chosen {
    use std::sync::atomic::{AtomicU8, Ordering};
    const GOING: u8 = 0;
    const NAMING: u8 = 1;
    const GIVEN_UP: u8 = 2;
    let state = std::sync::Arc::new(AtomicU8::new(GOING));
    let (tx, rx) = std::sync::mpsc::channel();
    {
        let state = state.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work(&|| state.compare_exchange(GOING, NAMING, Ordering::SeqCst, Ordering::SeqCst).is_ok()));
        });
    }
    let answer = |made: Result<Option<PathBuf>, String>| match made {
        Ok(Some(dir)) => Chosen::Made(dir),
        Ok(None) => Chosen::There,
        Err(e) => Chosen::Missed(Missed::of(&e)),
    };
    match rx.recv_timeout(limit) {
        Ok(made) => answer(made),
        Err(_) if state.compare_exchange(GOING, GIVEN_UP, Ordering::SeqCst, Ordering::SeqCst).is_ok() => Chosen::Missed(Missed::NoAnswer),
        // (It was giving the backup its name just then.)
        Err(_) => match rx.recv_timeout(REACH_LIMIT) {
            Ok(made) => answer(made),
            Err(_) if root.join(name).is_dir() => Chosen::Made(root.join(name)),
            Err(_) => Chosen::Missed(Missed::NoAnswer),
        },
    }
}

/// `into_chosen`'s work for the backup `name` before `running` first opens the notes: the chosen
/// folder `root` reached (made if need be), looked in, and the notes' copy (`copy`) copied there.
fn chosen_work(root: &Path, copy: &Path, name: &str, running: &str) -> impl FnOnce(&dyn Fn() -> bool) -> Result<Option<PathBuf>, String> + Send + 'static {
    let (root, copy, name, running) = (root.to_path_buf(), copy.to_path_buf(), name.to_string(), running.to_string());
    move |may_name| {
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        if made_for(&root, &running) {
            return Ok(None);
        }
        into_root(&root, &copy, &name, false, may_name).map(Some)
    }
}

/// The backup `name` made in `root` from the notes' copy (`copy`: moved there when `take`, else
/// copied), its files still to come (`FILES_TO_COME`), given its real name once its notes are on
/// the disk, if `may_name` still says so; then the oldest of its kind pruned.
fn into_root(root: &Path, copy: &Path, name: &str, take: bool, may_name: &dyn Fn() -> bool) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let dir = root.join(name);
    if dir.exists() {
        return Ok(dir);
    }
    clear_unfinished(root);
    let part = unfinished(root, name);
    std::fs::create_dir_all(&part).map_err(|e| e.to_string())?;
    let made = (|| -> Result<(), String> {
        let to = part.join(crate::paths::DB_FILE);
        if !(take && std::fs::rename(copy, &to).is_ok()) {
            std::fs::copy(copy, &to).map_err(|e| e.to_string())?;
        }
        std::fs::OpenOptions::new().write(true).open(&to).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
        std::fs::write(part.join(FILES_TO_COME), b"The pictures of this backup are still being copied: Dobo finishes it at its next start.\r\n").map_err(|e| e.to_string())?;
        if !may_name() {
            return Err("given up".into());
        }
        std::fs::rename(&part, &dir).map_err(|e| e.to_string())
    })();
    if let Err(e) = made {
        let _ = std::fs::remove_dir_all(&part);
        return Err(e);
    }
    prune(root, None);
    Ok(dir)
}

/// The data folder's files (`FILE_FOLDERS`) into a backup (`backup`): linked where they can be
/// (the same drive), else copied when `copies` (each under a name of its own, then renamed: a
/// copy cut short never looks whole). Stops early when `stop` is set. Ok(false): a link was
/// refused and copies weren't wanted (the rest is left for `finish_later`).
fn link_files(data: &Path, backup: &Path, stop: &std::sync::atomic::AtomicBool, copies: bool) -> Result<bool, String> {
    let mut links = true;
    for folder in FILE_FOLDERS {
        let (from, to) = (data.join(folder), backup.join(folder));
        if !from.is_dir() {
            continue;
        }
        std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
        for e in std::fs::read_dir(&from).map_err(|e| e.to_string())? {
            if stop.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("stopped".into());
            }
            let e = e.map_err(|e| e.to_string())?;
            let p = e.path();
            let dest = to.join(e.file_name());
            if !p.is_file() || dest.exists() {
                continue;
            }
            // (Once a link is refused, another drive or a drive that has none: copies from then.)
            if links && std::fs::hard_link(&p, &dest).is_ok() {
                continue;
            }
            links = false;
            if !copies {
                return Ok(false);
            }
            let part = to.join(format!("{}.part", e.file_name().to_string_lossy()));
            std::fs::copy(&p, &part).and_then(|_| std::fs::rename(&part, &dest)).map_err(|e| {
                let _ = std::fs::remove_file(&part);
                e.to_string()
            })?;
        }
    }
    Ok(true)
}

/// The backups made before an update whose files are still to come (`FILES_TO_COME`), in `roots`
/// (the data folder's own backups folder, the one chosen in Settings), finished from the data
/// folder `data`.
fn finish_files(roots: &[PathBuf], data: &Path) {
    let never = std::sync::atomic::AtomicBool::new(false);
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if !(is_backup(&dir) && kind_of(&name).0 == "before-update" && dir.join(FILES_TO_COME).is_file()) {
                continue;
            }
            match link_files(data, &dir, &never, true) {
                Ok(_) => {
                    let _ = std::fs::remove_file(dir.join(FILES_TO_COME));
                }
                Err(e) => eprintln!("The pictures of {} couldn't all be copied yet: {e}", dir.display()),
            }
        }
    }
}

/// Once the board has started: the backups made before an update whose files are still to come
/// (`before_update`) get them, on a thread of their own (a slow drive never holds up anything).
pub fn finish_later(app: &AppHandle) {
    let Ok(data) = crate::paths::data_dir() else { return };
    let mut roots = vec![data.join("backups")];
    if let Ok(chosen) = folder(app)
        && !same_dir(&chosen, &roots[0])
    {
        roots.push(chosen);
    }
    std::thread::spawn(move || finish_files(&roots, &data));
}

/// When a failed daily backup is tried again.
const RETRY_MS: i64 = 3_600_000;

/// Why the last automatic backup didn't work (cleared by the next backup that works).
const FAILED_ERROR: &str = "backup_error";

/// A word about the backups that isn't a failed automatic one (said in Settings › Backups as it
/// is, with no "The automatic backup didn't work" before it): the backup made before a restore
/// misses what was typed during it (kept in a file, `NOTICE_FILE`: said until that file goes), or
/// no backup could be made before a new version opened the notes (said until a backup works).
const NOTICE: &str = "backup_notice";
const NOTICE_AT: &str = "backup_notice_at";
const NOTICE_FILE: &str = "backup_notice_file";

/// A word of its own for Settings › Backups (`NOTICE`), kept as what it is and its parts, never
/// as finished words: the app says it in the language of the moment (src/lib/backups.ts
/// `noticeText`). Kept as `{"code": "restore_missed", "args": {"error": …, "path": …}}`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "code", content = "args", rename_all = "snake_case")]
pub enum Notice {
    /// The backup made before a restore misses the changes made during it (`error`: why); they
    /// are kept in `path`.
    RestoreMissed { error: String, path: String },
    /// The backup made before `app` `version` first opened the notes went to `path`, in the data
    /// folder: the backups folder chosen (`chosen`) couldn't be used (`why`; none: kept by an
    /// earlier 1.1 build, it didn't answer in time). Said while that backup is there.
    UpdateBackupElsewhere {
        app: String,
        version: String,
        path: String,
        chosen: String,
        #[serde(default)]
        why: Option<Missed>,
    },
    /// No backup could be made before `app` `version` first opened the notes: `why`, or, for any
    /// other reason (or none, kept by an earlier 1.1 build), Windows' or SQLite's own words
    /// (`error`). Said until a backup works.
    UpdateBackupFailed {
        app: String,
        version: String,
        error: String,
        #[serde(default)]
        why: Option<Missed>,
    },
}

/// The notice as kept: what it is and its parts (this version's), or the finished words an
/// older version kept (said as they are).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Said {
    Notice(Notice),
    Words(String),
}

/// Settings › Backups: the automatic backup that didn't work, if the last one didn't; and a word
/// of its own (`NOTICE`), if there is one.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Status {
    pub error: Option<String>,
    pub failed_at: Option<i64>,
    pub notice: Option<Said>,
    pub notice_at: Option<i64>,
}

pub fn status(conn: &rusqlite::Connection) -> Status {
    let error = db::get_setting(conn, FAILED_ERROR).and_then(|v| v.as_str().map(String::from));
    let failed_at = error.as_ref().and_then(|_| db::get_setting(conn, "backup_failed_at").and_then(|v| v.as_i64()));
    // (One about a file: only while it is there.)
    let file_there = db::get_setting(conn, NOTICE_FILE).and_then(|v| v.as_str().map(PathBuf::from)).is_none_or(|f| f.exists());
    // (One this version can't read, from a newer one: not said.)
    let notice = db::get_setting(conn, NOTICE).and_then(|v| serde_json::from_value::<Said>(v).ok()).filter(|_| file_there);
    let notice_at = notice.as_ref().and_then(|_| db::get_setting(conn, NOTICE_AT).and_then(|v| v.as_i64()));
    Status { error, failed_at, notice, notice_at }
}

/// Keeps a word to say in Settings › Backups (`NOTICE`); `file`: the file it is about (said while
/// that file is there).
pub fn note_notice(conn: &rusqlite::Connection, now: i64, notice: &Notice, file: Option<&Path>) {
    let _ = db::set_setting(conn, NOTICE, &serde_json::json!(notice));
    let _ = db::set_setting(conn, NOTICE_AT, &serde_json::json!(now));
    match file {
        Some(f) => {
            let _ = db::set_setting(conn, NOTICE_FILE, &serde_json::json!(f.to_string_lossy()));
        }
        None => {
            let _ = conn.execute("DELETE FROM settings WHERE key = ?1", [NOTICE_FILE]);
        }
    }
}

/// A daily backup didn't work: kept to tell, and tried again in an hour. Returns whether this is
/// the first failure since one worked (said on the board once, not every hour).
pub fn note_failure(conn: &rusqlite::Connection, now: i64, error: &str) -> bool {
    let first = status(conn).error.is_none();
    let _ = db::set_setting(conn, "backup_failed_at", &serde_json::json!(now));
    let _ = db::set_setting(conn, FAILED_ERROR, &serde_json::json!(error));
    first
}

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
    // Made on a thread of its own: copying the pictures (or a backups folder on a slow or missing
    // drive) must never hold up the reminders, whose clock this is (the first look after
    // midnight makes it, when reminders may be due). One at a time: the clock looks again every
    // few seconds while it is being made.
    static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if RUNNING.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = make(&app, "daily") {
            eprintln!("Daily backup failed: {e}");
            // Try again in an hour, not every 5 seconds; and say so (never a silent failure: the
            // backups folder may be a drive that is gone, full or read-only).
            let first = {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                note_failure(&conn, now, &e)
            };
            if first {
                let _ = app.emit_to("main", "backup-failed", e);
            }
        }
        RUNNING.store(false, std::sync::atomic::Ordering::Release);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dobo-backup-{tag}-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The files of a backup made before an update are linked where they can be, else copied;
    /// a copy given up on stops at the next file.
    #[test]
    fn files_before_an_update_are_linked_or_copied_and_can_be_stopped() {
        let data = temp_dir("link");
        for folder in FILE_FOLDERS {
            std::fs::create_dir_all(data.join(folder)).unwrap();
            std::fs::write(data.join(folder).join("1-0.png"), folder).unwrap();
        }
        let part = data.join("part");
        let go = std::sync::atomic::AtomicBool::new(false);
        assert!(link_files(&data, &part, &go, true).unwrap());
        for folder in FILE_FOLDERS {
            assert_eq!(std::fs::read_to_string(part.join(folder).join("1-0.png")).unwrap(), folder);
        }
        // (A link keeps its content when the data folder's file goes.)
        std::fs::remove_file(data.join("pictures").join("1-0.png")).unwrap();
        assert_eq!(std::fs::read_to_string(part.join("pictures").join("1-0.png")).unwrap(), "pictures");
        let stop = std::sync::atomic::AtomicBool::new(true);
        assert!(link_files(&data, &data.join("part2"), &stop, true).is_err());
        assert!(within(std::time::Duration::from_secs(5), || 7).is_some_and(|v| v == 7));
        assert_eq!(within(std::time::Duration::from_millis(10), || std::thread::sleep(std::time::Duration::from_secs(2))), None);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_failed_backup_is_kept_to_tell_until_one_works() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        assert_eq!(status(&conn), Status { error: None, failed_at: None, notice: None, notice_at: None });
        assert!(note_failure(&conn, 1000, "the drive is gone"), "the first failure is said");
        assert!(!note_failure(&conn, 2000, "the drive is gone"), "not again every hour");
        assert_eq!(status(&conn), Status { error: Some("the drive is gone".into()), failed_at: Some(2000), notice: None, notice_at: None });
        // One works (what `make` does): nothing to tell any more.
        conn.execute("DELETE FROM settings WHERE key = ?1", [FAILED_ERROR]).unwrap();
        assert_eq!(status(&conn).error, None);
        assert!(note_failure(&conn, 3000, "full"));
    }

    /// A word of its own (not a failed automatic backup): one about a file is said while that file
    /// is there; a backup that works clears only one that isn't about a file. Neither is ever
    /// brought back by a restored backup.
    #[test]
    fn a_notice_is_said_on_its_own_while_it_matters() {
        let dir = temp_dir("notice");
        let kept = dir.join("before-restore-1.sqlite");
        std::fs::write(&kept, b"x").unwrap();
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let missed = Notice::RestoreMissed { error: "full".into(), path: kept.display().to_string() };
        note_notice(&conn, 5000, &missed, Some(&kept));
        let st = status(&conn);
        assert_eq!((st.error, st.notice, st.notice_at), (None, Some(Said::Notice(missed)), Some(5000)), "not a failed backup");
        std::fs::remove_file(&kept).unwrap();
        assert_eq!(status(&conn).notice, None, "the file is gone: nothing to say");
        note_notice(&conn, 6000, &failed("full"), None);
        assert!(status(&conn).notice.is_some());
        // A backup that works (what `make_at` does): that one goes.
        let notes = std::sync::Mutex::new(conn);
        let data = dir.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let file = data.join(crate::paths::DB_FILE);
        {
            let c = crate::db::open(&file).unwrap();
            note_notice(&c, 6000, &failed("full"), None);
        }
        let live = std::sync::Mutex::new(crate::db::open(&file).unwrap());
        make_at(&dir.join("backups"), "dobo-2026-11-22_090000-manual", &data, &dir.join("tmp"), &live, 7000, COPY_LIMIT).unwrap();
        assert_eq!(status(&held(&live)).notice, None);
        drop(notes);
        // Restored: the backup's own word (and failure) never comes back; today's stay.
        let theirs = dir.join("theirs.sqlite");
        {
            let c = crate::db::open(&theirs).unwrap();
            note_notice(&c, 1, &failed("an old word"), None);
            note_failure(&c, 1, "an old failure");
        }
        let mut now = crate::db::open(&dir.join("now.sqlite")).unwrap();
        put_back(&mut now, &open_checked(&theirs).unwrap(), &dir.join("no-files"), &dir.join("data2")).unwrap();
        assert_eq!(status(&now), Status { error: None, failed_at: None, notice: None, notice_at: None });
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn failed(error: &str) -> Notice {
        Notice::UpdateBackupFailed { app: "Dobo".into(), version: "1.1.0".into(), error: error.into(), why: None }
    }

    /// The notice is kept as what it is and its parts (the app says it in the language of the
    /// moment); one kept as finished words by an older version is still said, as it is; one from
    /// a newer version this one can't read is not said.
    #[test]
    fn a_notice_is_kept_as_its_code_and_parts() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let elsewhere = Notice::UpdateBackupElsewhere { app: "Dobo".into(), version: "1.1.0".into(), path: r"C:\data\backups\b".into(), chosen: r"E:\Backups".into(), why: Some(Missed::Full) };
        note_notice(&conn, 1, &elsewhere, None);
        assert_eq!(
            db::get_setting(&conn, NOTICE),
            Some(serde_json::json!({ "code": "update_backup_elsewhere", "args": { "app": "Dobo", "version": "1.1.0", "path": r"C:\data\backups\b", "chosen": r"E:\Backups", "why": "full" } }))
        );
        assert_eq!(serde_json::to_value(status(&conn).notice).unwrap()["code"], "update_backup_elsewhere", "the app gets it as kept");
        // One kept by an earlier 1.1 build, with no why: read all the same.
        db::set_setting(&conn, NOTICE, &serde_json::json!({ "code": "update_backup_failed", "args": { "app": "Dobo", "version": "1.1.0", "error": "x" } })).unwrap();
        assert_eq!(status(&conn).notice, Some(Said::Notice(Notice::UpdateBackupFailed { app: "Dobo".into(), version: "1.1.0".into(), error: "x".into(), why: None })));
        db::set_setting(&conn, NOTICE, &serde_json::json!("La sauvegarde faite avant que Dobo 1.1.0 ouvre vos notes…")).unwrap();
        assert_eq!(status(&conn).notice, Some(Said::Words("La sauvegarde faite avant que Dobo 1.1.0 ouvre vos notes…".into())));
        db::set_setting(&conn, NOTICE, &serde_json::json!({ "code": "from_a_newer_dobo", "args": {} })).unwrap();
        assert_eq!(status(&conn).notice, None);
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
        prune(&root, None);
        let left: Vec<String> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(left.len(), 17);
        assert!(left.contains(&"ztodo-2026-10-07_090000-daily".to_string()));
        assert!(!left.contains(&"ztodo-2026-10-06_090000-daily".to_string()));
        assert!(left.contains(&"ztodo-notes".to_string()) && left.contains(&"Photos".to_string()));
    }

    /// A backup cut short (its notes written, its pictures not yet) is never listed, counted or
    /// restored, and goes at the next backup; other folders stay.
    #[test]
    fn a_backup_cut_short_never_counts() {
        let root = temp_dir("part");
        let part = unfinished(&root, "dobo-2026-11-21_090000-daily");
        std::fs::create_dir_all(&part).unwrap();
        std::fs::write(part.join(crate::paths::DB_FILE), b"x").unwrap();
        assert!(!is_backup(&part));
        for other in [".git", "dobo-notes.part", ".hidden.part"] {
            std::fs::create_dir_all(root.join(other)).unwrap();
        }
        clear_unfinished(&root);
        assert!(!part.exists());
        for other in [".git", "dobo-notes.part", ".hidden.part"] {
            assert!(root.join(other).is_dir(), "{other} isn't Dobo's");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A backup whose repeating tasks' history isn't as Dobo keeps it (another app's, a future
    /// one's) is refused before anything is changed; one with none (an older one) is fine.
    #[test]
    fn a_backup_with_a_history_not_as_dobo_keeps_it_is_refused() {
        let dir = temp_dir("done-log");
        let older = dir.join("older.sqlite");
        {
            let c = crate::db::open(&older).unwrap();
            c.execute_batch("DROP INDEX done_log_item; DROP TABLE done_log;").unwrap();
        }
        assert!(open_checked(&older).is_ok());
        let other = dir.join("other.sqlite");
        {
            let c = crate::db::open(&other).unwrap();
            c.execute_batch("DROP INDEX done_log_item; DROP TABLE done_log; CREATE TABLE done_log (id INTEGER PRIMARY KEY, line INTEGER, at INTEGER);").unwrap();
        }
        assert!(open_checked(&other).is_err(), "refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// After an older backup comes back, the ids given since it are still never given again (a
    /// notification's buttons may name one).
    #[test]
    fn ids_given_after_a_backup_are_never_given_again_once_it_is_restored() {
        let dir = temp_dir("ids");
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        let n = crate::db::create_note(&conn, "Hub", "work", "soft", "Sage").unwrap();
        let line = |c: &rusqlite::Connection, text: &str| crate::db::create_item(c, crate::db::NewItem { note_id: n.id, kind: "task", text, due_at: Some(1), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        line(&conn, "Before");
        let backup = dir.join("b.sqlite");
        conn.execute("VACUUM INTO ?1", [backup.to_string_lossy().to_string()]).unwrap();
        let later = line(&conn, "Made after the backup (its notification is showing)");
        // A picture too: the backup has never given a picture id.
        let pic = crate::db::add_picture(&conn, n.id, None, "C:/p.png").unwrap();
        let shown = item_ids(&conn);
        assert!(shown.contains(&later.id));
        let src = open_checked(&backup).unwrap();
        let data = dir.join("data");
        put_back(&mut conn, &src, &dir.join("no-files"), &data).unwrap();
        assert!(crate::db::get_item(&conn, later.id).is_err(), "the backup is from before it");
        let again = line(&conn, "New");
        assert!(again.id > later.id, "a new id, never the one a notification may still name");
        let n2 = crate::db::create_note(&conn, "New note", "work", "soft", "Sky").unwrap();
        assert!(n2.id > n.id);
        assert!(crate::db::add_picture(&conn, n.id, None, "C:/q.png").unwrap().id > pic.id);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The backup is made ready without holding the notes: a line given an id meanwhile still
    /// never has its id given again once the backup is put back.
    #[test]
    fn ids_given_while_a_backup_is_made_ready_are_never_given_again() {
        let dir = temp_dir("ids-meanwhile");
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        let n = crate::db::create_note(&conn, "Hub", "work", "soft", "Sage").unwrap();
        let line = |c: &rusqlite::Connection, text: &str| crate::db::create_item(c, crate::db::NewItem { note_id: n.id, kind: "task", text, due_at: Some(1), parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        line(&conn, "Before");
        let backup = dir.join("b.sqlite");
        conn.execute("VACUUM INTO ?1", [backup.to_string_lossy().to_string()]).unwrap();
        let src = open_checked(&backup).unwrap();
        let ready = prepared(&src, &sequences(&conn), &dir.join("data")).unwrap();
        let meanwhile = line(&conn, "Made while the backup was being made ready");
        put_ready(&mut conn, &ready).unwrap();
        assert!(crate::db::get_item(&conn, meanwhile.id).is_err(), "the backup is from before it");
        assert!(line(&conn, "New").id > meanwhile.id, "a new id, never the one given meanwhile");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Each kind keeps its own last few: backups made by hand, before a restore or before an
    /// update never push the daily ones out (nor each other). Backups made before the kinds were
    /// kept apart count as theirs; a kind from a newer Dobo is never removed.
    #[test]
    fn each_kind_keeps_its_own_last_few() {
        let root = temp_dir("kinds");
        let make = |name: &str| {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(crate::paths::DB_FILE), b"x").unwrap();
        };
        for d in 1..=20 {
            make(&format!("dobo-2026-11-{d:02}_090000-daily"));
            make(&format!("dobo-2026-11-{d:02}_100000-manual"));
            make(&format!("dobo-2026-11-{d:02}_110000-before-restore"));
            make(&format!("dobo-2026-11-{d:02}_120000-before-update-1.{d}.0"));
            make(&format!("dobo-2026-11-{d:02}_130000-sometime-later"));
        }
        // From before the rename, and from before kinds were written at all (early daily ones).
        make("ztodo-2026-10-01_090000-manual");
        make("ztodo-2026-09-01_090000");
        prune(&root, None);
        let names: Vec<String> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        let count = |k: &str| names.iter().filter(|n| kind_of(n).0 == k).count();
        assert_eq!((count("daily"), count("manual"), count("before-restore"), count("before-update")), (14, 10, 5, 3));
        assert_eq!(count("sometime-later"), 20, "a kind this version doesn't know is left alone");
        assert!(names.contains(&"dobo-2026-11-07_090000-daily".into()) && !names.contains(&"dobo-2026-11-06_090000-daily".into()));
        assert!(names.contains(&"dobo-2026-11-20_120000-before-update-1.20.0".into()));
        assert!(!names.iter().any(|n| n.starts_with("ztodo-")), "the old ones are the oldest of their kind");
        assert_eq!(kind_of("ztodo-2026-09-01_090000"), ("daily".into(), None));
        assert_eq!(kind_of("dobo-2026-11-22_090000-before-update-1.1.0"), ("before-update".into(), Some("1.1.0".into())));
        assert_eq!(kind_of("dobo-2026-11-22_090000-before-restore"), ("before-restore".into(), None));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Restoring the oldest of 14 backups: the backup of the notes as they are now (made first)
    /// takes the place of another one, never of the one being restored.
    #[test]
    fn the_backup_being_restored_is_never_pruned() {
        let root = temp_dir("keep");
        for d in 1..=DAILY_KEEP + 2 {
            let dir = root.join(format!("dobo-2026-11-{d:02}_090000-daily"));
            std::fs::create_dir_all(dir.join("pictures")).unwrap();
            std::fs::write(dir.join(crate::paths::DB_FILE), b"x").unwrap();
            std::fs::write(dir.join("pictures").join("1-0.png"), b"png").unwrap();
        }
        let oldest = root.join("dobo-2026-11-01_090000-daily");
        // Named another way (letter case, a trailing slash), as the page may send it.
        let asked = PathBuf::from(format!("{}\\", oldest.to_string_lossy().to_uppercase()));
        prune(&root, Some(&asked));
        assert!(oldest.join("pictures").join("1-0.png").is_file(), "the backup being restored stays whole");
        assert!(!root.join("dobo-2026-11-02_090000-daily").exists(), "the next oldest went instead");
        let left = std::fs::read_dir(&root).unwrap().count();
        assert_eq!(left, DAILY_KEEP + 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A backup keeps the board's own picture too, and a restore (here or on another computer)
    /// brings it back with the notes' pictures, everything pointed at the copies here.
    #[test]
    fn a_backup_brings_back_the_pictures_and_the_board_background() {
        let elsewhere = temp_dir("bg-old").join("data");
        let data = temp_dir("bg-new").join("data");
        for d in FILE_FOLDERS {
            std::fs::create_dir_all(elsewhere.join(d)).unwrap();
        }
        std::fs::write(elsewhere.join("pictures").join("1-0.png"), b"png").unwrap();
        std::fs::write(elsewhere.join("backgrounds").join("9.jpg"), b"jpg").unwrap();
        // The notes as they were on the other computer, backed up there.
        let old = crate::db::open(&elsewhere.join("ztodo.sqlite")).unwrap();
        let n = crate::db::create_note(&old, "Hub", "work", "soft", "Sage").unwrap();
        crate::db::add_picture(&old, n.id, None, &elsewhere.join("pictures").join("1-0.png").to_string_lossy()).unwrap();
        let bg = elsewhere.join("backgrounds").join("9.jpg").to_string_lossy().to_string();
        crate::db::set_setting(&old, "look_board", &serde_json::json!({ "light": { "bg": "picture", "picture": bg } })).unwrap();
        let backup = temp_dir("bg-backup").join("dobo-2026-11-21_090000-manual");
        std::fs::create_dir_all(&backup).unwrap();
        old.execute("VACUUM INTO ?1", [backup.join(crate::paths::DB_FILE).to_string_lossy().to_string()]).unwrap();
        copy_files(&elsewhere, &backup).unwrap();
        assert!(backup.join("backgrounds").join("9.jpg").is_file(), "the background is in the backup");
        drop(old);
        let _ = std::fs::remove_dir_all(&elsewhere);
        // Restored into this data folder.
        std::fs::create_dir_all(&data).unwrap();
        let mut conn = crate::db::open(&data.join("ztodo.sqlite")).unwrap();
        crate::db::create_note(&conn, "Will be replaced", "work", "soft", "Sky").unwrap();
        let src = open_checked(&backup.join(crate::paths::DB_FILE)).unwrap();
        put_back(&mut conn, &src, &backup, &data).unwrap();
        let here_pic = data.join("pictures").join("1-0.png");
        let here_bg = data.join("backgrounds").join("9.jpg");
        assert!(here_pic.is_file() && here_bg.is_file());
        assert_eq!(crate::db::picture_paths(&conn).unwrap(), vec![here_pic.to_string_lossy().to_string()]);
        assert_eq!(crate::db::get_setting(&conn, "look_board").unwrap()["light"]["picture"], serde_json::json!(here_bg.to_string_lossy()));
        assert_eq!(crate::db::load_all(&conn).unwrap().notes.len(), 1);
    }

    /// The pictures' thumbs are made again from the pictures: never in a backup, never put back
    /// from one (a restore leaves the thumbs here as they are, the board makes the missing ones).
    #[test]
    fn thumbs_are_never_backed_up_nor_put_back() {
        let data = temp_dir("thumbs-data").join("data");
        let thumbs = data.join("pictures").join(crate::thumbs::DIR);
        std::fs::create_dir_all(&thumbs).unwrap();
        std::fs::write(data.join("pictures").join("1-0.jpg"), b"jpg").unwrap();
        std::fs::write(thumbs.join(crate::thumbs::name_for("1-0.jpg", 1200, 900)), b"webp").unwrap();
        let backup = temp_dir("thumbs-backup").join("dobo-2026-11-21_090000-manual");
        std::fs::create_dir_all(&backup).unwrap();
        copy_files(&data, &backup).unwrap();
        assert!(backup.join("pictures").join("1-0.jpg").is_file(), "the picture is in the backup");
        assert!(!backup.join("pictures").join(crate::thumbs::DIR).exists(), "its thumb isn't");
        // A backup holding thumbs all the same (copied by hand): only its pictures come back.
        let given = backup.join("pictures").join(crate::thumbs::DIR);
        std::fs::create_dir_all(&given).unwrap();
        std::fs::write(given.join(crate::thumbs::name_for("2-0.jpg", 1200, 900)), b"webp").unwrap();
        std::fs::write(backup.join("pictures").join("2-0.jpg"), b"jpg").unwrap();
        let other = temp_dir("thumbs-other").join("data");
        put_back_files(&backup, &other).unwrap();
        assert!(other.join("pictures").join("1-0.jpg").is_file() && other.join("pictures").join("2-0.jpg").is_file());
        assert!(!other.join("pictures").join(crate::thumbs::DIR).exists());
        // Put back over the data folder itself: the thumbs there stay.
        put_back_files(&backup, &data).unwrap();
        assert_eq!(std::fs::read_dir(&thumbs).unwrap().count(), 1);
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
        prune(&root, None);
        let mut left: Vec<String> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        left.sort_by(|a, b| stamp_of(a).cmp(&stamp_of(b)));
        assert_eq!(left.len(), DAILY_KEEP);
        assert_eq!(left.first().unwrap(), "ztodo-2026-11-07_090000-daily", "the oldest went, not the old-named ones first");
        assert_eq!(left.last().unwrap(), "dobo-2026-11-20_090000-daily");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A "backup" made to harm (a trigger that empties the notes on the next change, a view):
    /// restored, it brings its notes and nothing else.
    #[test]
    fn a_foreign_backups_triggers_and_views_never_come_in() {
        let dir = temp_dir("triggers");
        let bad = dir.join("bad.sqlite");
        {
            let c = crate::db::open(&bad).unwrap();
            let n = crate::db::create_note(&c, "Theirs", "work", "soft", "Sage").unwrap();
            crate::db::create_item(&c, crate::db::NewItem { note_id: n.id, kind: "task", text: "x", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
            c.execute_batch(
                "CREATE TRIGGER \"wipe \"\"all\"\"\" AFTER UPDATE ON items BEGIN DELETE FROM notes; END;
                 CREATE TRIGGER wipe2 AFTER INSERT ON notes BEGIN DELETE FROM items; END;
                 CREATE VIEW v AS SELECT * FROM notes;",
            )
            .unwrap();
        }
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        let src = open_checked(&bad).unwrap();
        put_back(&mut conn, &src, &dir.join("no-files"), &dir.join("data")).unwrap();
        let left: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type IN ('trigger', 'view')", [], |r| r.get(0)).unwrap();
        assert_eq!(left, 0);
        conn.execute("UPDATE items SET text = 'changed'", []).unwrap();
        crate::db::create_note(&conn, "Mine", "work", "soft", "Sky").unwrap();
        assert_eq!(crate::db::note_count(&conn), 2, "nothing was deleted");
        assert_eq!(item_ids(&conn).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The computers file cards may open files from are this computer's own choice: a restored
    /// backup (perhaps someone else's) never brings its own.
    #[test]
    fn a_backup_never_brings_computers_to_open_files_from() {
        let dir = temp_dir("hosts");
        let theirs = dir.join("theirs.sqlite");
        {
            let c = crate::db::open(&theirs).unwrap();
            crate::db::set_setting(&c, "file_hosts", &serde_json::json!(["attacker.example"])).unwrap();
            crate::db::set_setting(&c, "backup_dir", &serde_json::json!(r"\\their-nas\backups")).unwrap();
            crate::db::set_setting(&c, "language", &serde_json::json!("fr")).unwrap();
            crate::db::set_setting(&c, crate::whatsnew::SEEN, &serde_json::json!("1.1.0")).unwrap();
        }
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        crate::db::set_setting(&conn, "file_hosts", &serde_json::json!(["nas"])).unwrap();
        crate::db::set_setting(&conn, crate::whatsnew::SEEN, &serde_json::json!("1.2.0")).unwrap();
        put_back(&mut conn, &open_checked(&theirs).unwrap(), &dir.join("no-files"), &dir.join("data")).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "file_hosts"), Some(serde_json::json!(["nas"])), "kept as they were here");
        assert_eq!(crate::db::get_setting(&conn, crate::whatsnew::SEEN), Some(serde_json::json!("1.2.0")), "What's new already shown here stays shown");
        assert_eq!(crate::db::get_setting(&conn, "backup_dir"), None, "nor where their backups went");
        assert_eq!(crate::db::get_setting(&conn, "language"), Some(serde_json::json!("fr")), "the rest comes back");
        // None here: none after either.
        let mut fresh = crate::db::open(&dir.join("fresh.sqlite")).unwrap();
        put_back(&mut fresh, &open_checked(&theirs).unwrap(), &dir.join("no-files"), &dir.join("data")).unwrap();
        assert_eq!(crate::db::get_setting(&fresh, "file_hosts"), None);
        // Put back where there are no notes (the recovery screen): none either.
        let backup = dir.join("backup");
        std::fs::create_dir_all(&backup).unwrap();
        std::fs::copy(&theirs, backup.join(crate::paths::DB_FILE)).unwrap();
        let data = dir.join("data2");
        std::fs::create_dir_all(&data).unwrap();
        put_back_into(&backup, &data, &[], &[]).unwrap();
        let back = crate::db::open(&data.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&back, "file_hosts"), None);
        assert_eq!(crate::db::get_setting(&back, "backup_dir"), None);
        drop(back);
        // The notes put aside here had their own: those come along (never their backups' state).
        let data = dir.join("data3");
        std::fs::create_dir_all(&data).unwrap();
        let here = [("backup_dir".to_string(), serde_json::json!(r"D:\Backups")), ("backup_last".to_string(), serde_json::json!(5))];
        put_back_into(&backup, &data, &[], &here).unwrap();
        let back = crate::db::open(&data.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&back, "backup_dir"), Some(serde_json::json!(r"D:\Backups")));
        assert_eq!(crate::db::get_setting(&back, "backup_last"), None);
        assert_eq!(crate::db::get_setting(&back, "file_hosts"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An older backup brings its notes back, never its days: the look back already closed today
    /// stays closed, Tidy already answered stays answered.
    #[test]
    fn a_restore_keeps_todays_look_back_and_tidy() {
        let dir = temp_dir("today");
        let old = dir.join("old.sqlite");
        let (day, today) = (86_400_000i64, 1_800_000_000_000i64);
        {
            let c = crate::db::open(&old).unwrap();
            crate::db::create_note(&c, "Old", "work", "soft", "Sage").unwrap();
            crate::db::set_setting(&c, "lookback_last", &serde_json::json!(today - 3 * day)).unwrap();
            crate::db::set_setting(&c, "lookback_dismissed", &serde_json::json!("2027-01-12")).unwrap();
            crate::db::set_setting(&c, "tidy_last", &serde_json::json!(today - 3 * day)).unwrap();
            crate::db::set_setting(&c, "tidy_snooze", &serde_json::json!(today - 2 * day)).unwrap();
        }
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        crate::db::set_setting(&conn, "lookback_last", &serde_json::json!(today)).unwrap();
        crate::db::set_setting(&conn, "lookback_dismissed", &serde_json::json!("2027-01-15")).unwrap();
        crate::db::set_setting(&conn, "tidy_last", &serde_json::json!(today - 1000)).unwrap();
        put_back(&mut conn, &open_checked(&old).unwrap(), &dir.join("no-files"), &dir.join("data")).unwrap();
        let get = |k: &str| crate::db::get_setting(&conn, k);
        assert_eq!(crate::db::note_count(&conn), 1, "the notes are the backup's");
        assert_eq!(get("lookback_last"), Some(serde_json::json!(today)));
        assert_eq!(get("lookback_dismissed"), Some(serde_json::json!("2027-01-15")));
        assert_eq!(get("tidy_last"), Some(serde_json::json!(today - 1000)));
        assert_eq!(get("tidy_snooze"), None, "no old \"later\"");
        // A backup newer than today's state (from another computer, a clock set ahead) keeps its own.
        let ready = crate::db::open(Path::new(":memory:")).unwrap();
        crate::db::set_setting(&ready, "lookback_last", &serde_json::json!(today + day)).unwrap();
        keep_today(&ready, Some(&conn), today).unwrap();
        assert_eq!(crate::db::get_setting(&ready, "lookback_last"), Some(serde_json::json!(today + day)));
        // The recovery screen (nothing open to ask): what happened today up to now counts as done.
        let ready = crate::db::open(Path::new(":memory:")).unwrap();
        crate::db::set_setting(&ready, "tidy_last", &serde_json::json!(today - 3 * day)).unwrap();
        keep_today(&ready, None, today).unwrap();
        assert_eq!(crate::db::get_setting(&ready, "tidy_last"), Some(serde_json::json!(today)));
        assert_eq!(crate::db::get_setting(&ready, "lookback_last"), Some(serde_json::json!(today)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A backup that can be read but not made ready (here its pictures have no path, so they
    /// can't be pointed at the files here): the open notes stay exactly as they were.
    #[test]
    fn a_backup_that_cant_be_made_ready_changes_nothing() {
        let dir = temp_dir("not-ready");
        let odd = dir.join("odd.sqlite");
        {
            let c = rusqlite::Connection::open(&odd).unwrap();
            c.execute_batch(
                "CREATE TABLE notes (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, space TEXT NOT NULL DEFAULT 'work',
                    palette TEXT NOT NULL DEFAULT 'soft', color TEXT NOT NULL DEFAULT 'Paper', sort REAL NOT NULL DEFAULT 0,
                    archived INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL);
                 CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, note_id INTEGER NOT NULL, kind TEXT NOT NULL DEFAULT 'task',
                    text TEXT NOT NULL, due_at INTEGER, done_at INTEGER, sort REAL NOT NULL DEFAULT 0, created_at INTEGER NOT NULL);
                 CREATE TABLE pictures (id INTEGER PRIMARY KEY AUTOINCREMENT, note_id INTEGER NOT NULL);
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO notes (title, created_at) VALUES ('Theirs', 1);
                 CREATE TRIGGER wipe AFTER INSERT ON notes BEGIN DELETE FROM items; END;",
            )
            .unwrap();
        }
        let mut conn = crate::db::open(&dir.join("now.sqlite")).unwrap();
        let mine = crate::db::create_note(&conn, "Mine", "work", "soft", "Sky").unwrap();
        let src = open_checked(&odd).unwrap();
        assert!(put_back(&mut conn, &src, &dir.join("no-files"), &dir.join("data")).is_err());
        let notes = crate::db::load_all(&conn).unwrap().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!((notes[0].id, notes[0].title.as_str()), (mine.id, "Mine"), "the open notes are untouched");
        let foreign: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type IN ('trigger', 'view')", [], |r| r.get(0)).unwrap();
        assert_eq!(foreign, 0, "nothing of the backup came in");
        assert!(crate::db::create_note(&conn, "Still works", "work", "soft", "Sage").unwrap().id > mine.id);
        drop(conn);
        drop(src);
        let _ = std::fs::remove_dir_all(&dir);
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
        let paths = crate::db::picture_paths(&conn).unwrap();
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
        assert_eq!(crate::db::picture_paths(&conn).unwrap(), vec![new.join("pictures").join("1-0.png").to_string_lossy().to_string()]);
        assert_eq!(crate::db::preview_images(&conn).unwrap()[0].1, new.join("previews").join("p.jpg").to_string_lossy());
        let here = new.join("backgrounds").join("9.jpg").to_string_lossy().to_string();
        assert_eq!(crate::db::get_setting(&conn, "look_board").unwrap()["picture"], serde_json::json!(here));
        let styles = crate::db::get_setting(&conn, "look_styles").unwrap();
        assert_eq!(styles[0]["look"]["picture"], serde_json::json!(here));
        assert_eq!(styles[1]["look"]["picture"], serde_json::json!(""));
        // Run again (every start): nothing changes.
        repoint_moved(&conn, &new).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "look_board").unwrap()["picture"], serde_json::json!(here));
    }

    /// The app's connection, as the app holds it.
    fn app_notes(file: &Path) -> std::sync::Mutex<rusqlite::Connection> {
        std::sync::Mutex::new(crate::db::open(file).unwrap())
    }

    /// Notes on a network share (the classic journal), more than a few steps' worth of pages
    /// (about 200), the app's connection waiting only a moment for a busy file.
    fn classic(tag: &str) -> (PathBuf, PathBuf, std::sync::Mutex<rusqlite::Connection>) {
        let dir = temp_dir(tag);
        let file = dir.join(crate::paths::DB_FILE);
        let conn = crate::db::open(&file).unwrap();
        conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
        conn.busy_timeout(std::time::Duration::from_millis(10)).unwrap();
        let n = crate::db::create_note(&conn, "Big", "work", "soft", "Sage").unwrap();
        conn.execute("UPDATE notes SET title = ?1 WHERE id = ?2", rusqlite::params!["x".repeat(4096 * FIRST_STEP as usize * 3), n.id]).unwrap();
        (dir, file, std::sync::Mutex::new(conn))
    }

    fn titles(conn: &rusqlite::Connection) -> Vec<String> {
        let mut t: Vec<String> = crate::db::load_all(conn).unwrap().notes.into_iter().map(|n| n.title).collect();
        t.sort();
        t
    }

    fn whole(conn: &rusqlite::Connection) -> bool {
        conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)).unwrap() == "ok"
    }

    /// The copy is made while the app's own connection holds a change it hasn't finished: it
    /// never waits for it, and has only what was saved.
    #[test]
    fn the_notes_are_copied_without_waiting_for_the_app() {
        let dir = temp_dir("copy");
        let file = dir.join(crate::paths::DB_FILE);
        let notes = app_notes(&file);
        crate::db::create_note(&held(&notes), "Saved", "work", "soft", "Sage").unwrap();
        held(&notes).execute_batch("BEGIN IMMEDIATE; INSERT INTO notes (title, created_at) VALUES ('Not yet', 0);").unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let started = std::time::Instant::now();
        copy_notes(&notes, &file, &dir.join("tmp"), &out.join(crate::paths::DB_FILE), COPY_LIMIT).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        held(&notes).execute_batch("COMMIT").unwrap();
        let copy = rusqlite::Connection::open(out.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::note_count(&copy), 1);
        assert_eq!(std::fs::read_dir(dir.join("tmp")).unwrap().count(), 0, "no local copy left");
        drop(copy);
        drop(notes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Notes on a network share use the classic journal: the copy is made there too, whole.
    #[test]
    fn the_notes_are_copied_on_a_classic_journal_too() {
        let (dir, file, notes) = classic("copy-delete");
        crate::db::create_note(&held(&notes), "Saved", "work", "soft", "Sage").unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        copy_notes(&notes, &file, &dir.join("tmp"), &out.join(crate::paths::DB_FILE), COPY_LIMIT).unwrap();
        let copy = rusqlite::Connection::open(out.join(crate::paths::DB_FILE)).unwrap();
        assert!(whole(&copy));
        assert_eq!(titles(&copy), titles(&held(&notes)));
        assert_eq!(std::fs::read_dir(dir.join("tmp")).unwrap().count(), 0, "no local copy left");
        drop((copy, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Notes typed in all along while they are copied on a classic journal, pages already copied
    /// among them: the copy never starts again, never waits, and ends whole, with the notes as
    /// they were at its end.
    #[test]
    fn typing_all_along_never_starts_the_copy_again() {
        let (dir, _, notes) = classic("copy-typing");
        let mut to = rusqlite::Connection::open_in_memory().unwrap();
        let mut writes = 0;
        let copied = copy_paged(&notes, &mut to, COPY_LIMIT, || {
            writes += 1;
            let conn = held(&notes);
            crate::db::create_note(&conn, "Typed", "work", "soft", "Sky").unwrap();
            if writes == 2 {
                // (Pages the copy has already.)
                conn.execute("UPDATE notes SET title = ?1 WHERE title LIKE 'x%'", ["y".repeat(4096 * FIRST_STEP as usize * 3)]).unwrap();
            }
        })
        .unwrap();
        assert!(writes >= 2, "more than one step: {writes}");
        assert_eq!(copied, Copied { restarts: 0, busy: 0 });
        assert!(whole(&to));
        assert_eq!(titles(&to), titles(&held(&notes)), "whole, as the notes were at its end");
        drop((to, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The same with the app writing from threads of its own (as its commands do), into a local
    /// file as a backup does.
    #[test]
    fn the_app_writing_from_other_threads_meanwhile_never_spoils_the_copy() {
        let (dir, _, notes) = classic("copy-threads");
        let local = dir.join("local.sqlite");
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let mut to = rusqlite::Connection::open_with_flags(&local, flags).unwrap();
        let stop = std::sync::atomic::AtomicBool::new(false);
        let copied = std::thread::scope(|s| {
            for t in 0..3 {
                let (notes, stop) = (&notes, &stop);
                s.spawn(move || {
                    let mut i = 0;
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        i += 1;
                        let conn = held(notes);
                        crate::db::create_note(&conn, &format!("Typed {t}-{i}"), "work", "soft", "Sky").unwrap();
                        if i % 3 == 0 {
                            conn.execute("UPDATE notes SET title = ?1 WHERE title LIKE 'x%' OR title LIKE 'y%'", [if i % 2 == 0 { "x" } else { "y" }.repeat(4096 * 150)]).unwrap();
                        }
                        drop(conn);
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                });
            }
            let copied = copy_paged(&notes, &mut to, COPY_LIMIT, || std::thread::sleep(std::time::Duration::from_millis(5)));
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            copied
        })
        .unwrap();
        assert_eq!(copied.restarts, 0);
        assert!(whole(&to));
        let (copy, now) = (titles(&to), titles(&held(&notes)));
        assert!(copy.len() > 1 && copy.iter().filter(|t| t.starts_with("Typed")).all(|t| now.contains(t)), "notes as they were at one moment");
        drop((to, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The notes file changed other than through the app's connection: the copy starts again,
    /// with bigger steps, and still ends, whole.
    #[test]
    fn a_copy_started_again_takes_bigger_steps() {
        assert_eq!([0, 1, 2, 3, 40].map(step_pages), [64, 512, 4096, 4096, 4096]);
        let (dir, file, notes) = classic("copy-restart");
        let other = rusqlite::Connection::open(&file).unwrap();
        let mut to = rusqlite::Connection::open_in_memory().unwrap();
        let mut writes = 0;
        let copied = copy_paged(&notes, &mut to, COPY_LIMIT, || {
            writes += 1;
            other.execute("INSERT INTO notes (title, created_at) VALUES ('Elsewhere', 0)", []).unwrap();
        })
        .unwrap();
        // 64 pages, then 64 again from the first (started again), then all the rest (512).
        assert_eq!((copied.restarts, writes), (1, 2));
        assert!(whole(&to));
        assert_eq!(titles(&to), titles(&held(&notes)));
        drop((to, other, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Something else holds the notes file for a moment: waited for, step after step.
    #[test]
    fn a_busy_notes_file_is_waited_for() {
        let (dir, file, notes) = classic("copy-busy");
        let other = rusqlite::Connection::open(&file).unwrap();
        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let mut to = rusqlite::Connection::open_in_memory().unwrap();
        let mut steps = 0;
        let copied = copy_paged(&notes, &mut to, COPY_LIMIT, || {
            steps += 1;
            if steps == 3 {
                other.execute_batch("COMMIT").unwrap();
            }
        })
        .unwrap();
        assert_eq!(copied, Copied { restarts: 0, busy: 3 });
        assert_eq!(titles(&to), titles(&held(&notes)));
        drop((to, other, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The app's connection has a change under way (it holds the notes for it): waited for, and
    /// the copy has it once it is saved.
    #[test]
    fn a_change_under_way_is_waited_for() {
        let (dir, _, notes) = classic("copy-locked");
        held(&notes).execute_batch("BEGIN IMMEDIATE; INSERT INTO notes (title, created_at) VALUES ('Under way', 0);").unwrap();
        let mut to = rusqlite::Connection::open_in_memory().unwrap();
        let mut steps = 0;
        let copied = copy_paged(&notes, &mut to, COPY_LIMIT, || {
            steps += 1;
            if steps == 2 {
                held(&notes).execute_batch("COMMIT").unwrap();
            }
        })
        .unwrap();
        assert_eq!(copied, Copied { restarts: 0, busy: 2 });
        assert!(titles(&to).contains(&"Under way".to_string()));
        drop((to, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Busy all along: the copy is given up on after its time, and neither its local copy nor the
    /// unfinished backup is left behind.
    #[test]
    fn a_copy_that_waits_too_long_is_given_up_and_leaves_nothing() {
        let (dir, file, notes) = classic("copy-timeout");
        let other = rusqlite::Connection::open(&file).unwrap();
        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let limit = std::time::Duration::from_millis(200);
        let too_long = tr!("The notes stayed busy too long to copy.", "Les notes sont restées occupées trop longtemps pour être copiées.");
        let started = std::time::Instant::now();
        let mut to = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(copy_paged(&notes, &mut to, limit, || {}), Err(too_long.clone()));
        assert!(started.elapsed() >= limit);
        let (root, tmp) = (dir.join("backups"), dir.join("tmp"));
        std::fs::create_dir_all(&root).unwrap();
        let name = "dobo-2026-11-22_090000-daily";
        assert_eq!(make_in(&root, name, &dir, &tmp, &notes, limit), Err(too_long));
        assert!(!unfinished(&root, name).exists(), "no unfinished backup left");
        assert!(!root.join(name).exists());
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0, "no local copy left");
        other.execute_batch("COMMIT").unwrap();
        // Free again: made.
        assert!(is_backup(&make_in(&root, name, &dir, &tmp, &notes, limit).unwrap()));
        drop((to, other, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Notes typed while a restore reads the backup and makes it ready (the board stays usable):
    /// they are in the backup of the notes as they were, made before the restore. With WAL and
    /// with the classic journal.
    #[test]
    fn notes_typed_while_a_restore_runs_are_in_the_backup_made_before_it() {
        for classic in [false, true] {
            let dir = temp_dir(if classic { "meanwhile-classic" } else { "meanwhile" });
            let data = dir.join("data");
            std::fs::create_dir_all(&data).unwrap();
            let notes = app_notes(&data.join(crate::paths::DB_FILE));
            if classic {
                held(&notes).execute_batch("PRAGMA journal_mode = DELETE").unwrap();
            }
            crate::db::create_note(&held(&notes), "Kept", "work", "soft", "Sage").unwrap();
            let old = dir.join("old.sqlite");
            held(&notes).execute("VACUUM INTO ?1", [old.to_string_lossy().to_string()]).unwrap();
            crate::db::create_note(&held(&notes), "Made after the backup", "work", "soft", "Sky").unwrap();
            // What `restore` does: the notes as they are kept first, then the backup made ready.
            let (root, tmp) = (dir.join("backups"), dir.join("tmp"));
            std::fs::create_dir_all(&root).unwrap();
            let (before, seen) = make_at(&root, "dobo-2026-11-22_090000-before-restore", &data, &tmp, &notes, 1, COPY_LIMIT).unwrap();
            let src = open_checked(&old).unwrap();
            let ready = prepared(&src, &sequences(&held(&notes)), &data).unwrap();
            crate::db::create_note(&held(&notes), "Typed during the restore", "work", "soft", "Sky").unwrap();
            let snapshot = tmp.join("before-restore.sqlite");
            assert!(put_ready_keeping(&mut held(&notes), &ready, seen, &snapshot).unwrap(), "something was typed");
            replace_notes(&snapshot, &before.join(crate::paths::DB_FILE)).unwrap();
            assert_eq!(titles(&held(&notes)), ["Kept"], "the backup is back");
            let kept = open_checked(&before.join(crate::paths::DB_FILE)).unwrap();
            assert_eq!(titles(&kept), ["Kept", "Made after the backup", "Typed during the restore"]);
            assert_eq!(std::fs::read_dir(&before).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".part")).count(), 0);
            drop((kept, src, ready, notes));
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Nothing typed meanwhile: the backup made before the restore is the one kept, as it was
    /// (its own settings, written once it is made, don't count).
    #[test]
    fn a_restore_with_nothing_typed_meanwhile_keeps_its_backup_as_made() {
        let dir = temp_dir("meanwhile-none");
        let data = dir.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let notes = app_notes(&data.join(crate::paths::DB_FILE));
        crate::db::create_note(&held(&notes), "Kept", "work", "soft", "Sage").unwrap();
        let old = dir.join("old.sqlite");
        held(&notes).execute("VACUUM INTO ?1", [old.to_string_lossy().to_string()]).unwrap();
        let (root, tmp) = (dir.join("backups"), dir.join("tmp"));
        std::fs::create_dir_all(&root).unwrap();
        let (_, seen) = make_at(&root, "dobo-2026-11-22_090000-before-restore", &data, &tmp, &notes, 1, COPY_LIMIT).unwrap();
        let src = open_checked(&old).unwrap();
        let ready = prepared(&src, &sequences(&held(&notes)), &data).unwrap();
        let snapshot = tmp.join("before-restore.sqlite");
        assert!(!put_ready_keeping(&mut held(&notes), &ready, seen, &snapshot).unwrap());
        assert!(!snapshot.exists());
        drop((src, ready, notes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The open notes can't take the backup (something else is writing to them): the restore
    /// changes nothing, leaves no copy behind, and works once they are free.
    #[test]
    fn a_restore_the_notes_cant_take_changes_nothing() {
        let dir = temp_dir("put-busy");
        let file = dir.join(crate::paths::DB_FILE);
        let mut conn = crate::db::open(&file).unwrap();
        conn.busy_timeout(std::time::Duration::from_millis(10)).unwrap();
        crate::db::create_note(&conn, "In the backup", "work", "soft", "Sage").unwrap();
        let backup = dir.join("b.sqlite");
        conn.execute("VACUUM INTO ?1", [backup.to_string_lossy().to_string()]).unwrap();
        let seen = conn.total_changes();
        crate::db::create_note(&conn, "Typed meanwhile", "work", "soft", "Sky").unwrap();
        let src = open_checked(&backup).unwrap();
        let ready = prepared(&src, &sequences(&conn), &dir.join("data")).unwrap();
        let other = rusqlite::Connection::open(&file).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        let (was, ids) = (titles(&conn), sequences(&conn));
        let snapshot = dir.join("snapshot.sqlite");
        assert!(put_ready_keeping(&mut conn, &ready, seen, &snapshot).is_err());
        assert!(!snapshot.exists(), "no copy left behind");
        other.execute_batch("ROLLBACK").unwrap();
        assert_eq!((titles(&conn), sequences(&conn)), (was, ids), "nothing changed");
        assert!(put_ready_keeping(&mut conn, &ready, seen, &snapshot).unwrap());
        assert_eq!(titles(&conn), ["In the backup"]);
        drop((conn, other, src, ready));
        let _ = std::fs::remove_dir_all(&dir);
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

    /// What an error comes to, for Settings › Backups' words.
    #[test]
    fn why_a_backup_before_an_update_went_elsewhere_is_told_apart() {
        assert_eq!(Missed::of("There is not enough space on the disk. (os error 112)"), Missed::Full);
        assert_eq!(Missed::of("database or disk is full"), Missed::Full);
        assert_eq!(Missed::of("Access is denied. (os error 5)"), Missed::NotWritable);
        assert_eq!(Missed::of("The media is write protected. (os error 19)"), Missed::NotWritable);
        assert_eq!(Missed::of("unable to open database file"), Missed::NotWritable);
        assert_eq!(Missed::of("The system cannot find the path specified. (os error 3)"), Missed::Failed);
        assert_eq!(serde_json::to_value(Missed::NoAnswer).unwrap(), "no_answer");
        assert_eq!(serde_json::to_value(Missed::NotWritable).unwrap(), "not_writable");
    }

    /// The chosen folder has its time and no more: given up on, what it went on making is
    /// removed (never named); just naming it when the time is up, its backup is used (never two).
    #[test]
    fn the_chosen_folder_has_its_time_and_no_more() {
        let root = temp_dir("chosen-time");
        let name = "dobo-2026-11-23_090000-before-update-9.0.0";
        let short = std::time::Duration::from_millis(50);
        // Slow to answer: given up on; the work, when it comes to naming, is told no.
        let (told, heard) = std::sync::mpsc::channel();
        let slow = into_chosen(&root, name, short, move |may_name| {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = told.send(may_name());
            Err("given up".into())
        });
        assert_eq!(slow, Chosen::Missed(Missed::NoAnswer));
        assert_eq!(heard.recv_timeout(std::time::Duration::from_secs(5)), Ok(false), "never named once given up on");
        // Naming it just as the time is up: its answer is waited for.
        let dir = root.join(name);
        let made = {
            let dir = dir.clone();
            into_chosen(&root, name, short, move |may_name| {
                assert!(may_name());
                std::thread::sleep(std::time::Duration::from_millis(300));
                std::fs::create_dir_all(&dir).unwrap();
                Ok(Some(dir))
            })
        };
        assert_eq!(made, Chosen::Made(dir.clone()), "the backup it named, not a second one");
        // Its own words: why.
        assert_eq!(into_chosen(&root, name, short * 20, |_| Err("There is not enough space on the disk. (os error 112)".into())), Chosen::Missed(Missed::Full));
        assert_eq!(into_chosen(&root, name, short * 20, |_| Ok(None)), Chosen::There);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Notes in a folder of their own, with a picture, a link preview's and a background.
    fn data_with_files(tag: &str) -> PathBuf {
        let data = temp_dir(tag);
        {
            let conn = crate::db::open(&data.join(crate::paths::DB_FILE)).unwrap();
            crate::db::create_note(&conn, "Mine", "work", "soft", "Butter").unwrap();
            crate::db::set_setting(&conn, OPENED_BY, &serde_json::json!("1.0.0")).unwrap();
        }
        for folder in FILE_FOLDERS {
            std::fs::create_dir_all(data.join(folder)).unwrap();
            std::fs::write(data.join(folder).join("1-0.png"), folder).unwrap();
        }
        data
    }

    /// A chosen folder that works: the backup is made there only, its notes at once, its files
    /// to come (another drive: no links) until `finish_files`; a start that ended first leaves
    /// them to the next one.
    #[test]
    fn a_backup_before_an_update_in_the_chosen_folder_gets_its_files_after_the_start() {
        let data = data_with_files("chosen-files");
        // (Another drive when there is one: the project's own, not the temp folder's.)
        let chosen = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("dobo-chosen-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        {
            let conn = crate::db::open(&data.join(crate::paths::DB_FILE)).unwrap();
            crate::db::set_setting(&conn, "backup_dir", &serde_json::json!(chosen.to_string_lossy())).unwrap();
        }
        let conn = rusqlite::Connection::open(data.join(crate::paths::DB_FILE)).unwrap();
        let made = before_update(&conn, &data, "9.0.0", crate::db::now_ms()).unwrap().unwrap();
        assert_eq!(made.instead_of, None);
        assert!(made.dir.starts_with(&chosen) && made.dir.join(crate::paths::DB_FILE).is_file());
        assert!(!data.join("backups").exists(), "made in the chosen folder only");
        assert!(made.dir.join(FILES_TO_COME).is_file(), "its files to come (in the background)");
        assert!(!data.join(NOTES_COPY).exists(), "the notes' copy gone");
        // The start ended before they came: the next one finishes it.
        finish_files(&[data.join("backups"), chosen.clone()], &data);
        for folder in FILE_FOLDERS {
            assert_eq!(std::fs::read_to_string(made.dir.join(folder).join("1-0.png")).unwrap(), folder);
        }
        assert!(!made.dir.join(FILES_TO_COME).exists(), "finished");
        assert!(std::fs::read_dir(made.dir.join("pictures")).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".part")));
        // Made once per version.
        assert_eq!(before_update(&conn, &data, "9.0.0", crate::db::now_ms() + 2000).unwrap(), None);
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&chosen);
    }

    /// Links refused (a drive that has none, as exFAT and FAT32 sticks): the notes are backed up
    /// at once all the same, the files left for later; never a picture half-copied.
    #[test]
    fn files_that_cant_be_linked_are_left_for_later() {
        let data = data_with_files("nolinks");
        let other = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("dobo-nolinks-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&other).unwrap();
        // (Links across drives are refused, as on a drive that has none; on one drive, linked.)
        let across = std::fs::hard_link(data.join("pictures").join("1-0.png"), other.join("probe")).is_err();
        let all = link_files(&data, &other.join("b"), &std::sync::atomic::AtomicBool::new(false), false).unwrap();
        assert_eq!(all, !across, "copies are never made at once");
        if across {
            assert!(!other.join("b").join("pictures").join("1-0.png").exists());
        }
        assert!(link_files(&data, &other.join("b"), &std::sync::atomic::AtomicBool::new(false), true).unwrap());
        assert_eq!(std::fs::read_to_string(other.join("b").join("previews").join("1-0.png")).unwrap(), "previews");
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&other);
    }

    /// Settings lists the backups made before an update in the data folder's own backups folder
    /// when another one is chosen (only those, never twice).
    #[test]
    fn backups_before_an_update_kept_in_the_data_folder_are_listed_too() {
        let base = temp_dir("list");
        let (own, chosen) = (base.join("backups"), base.join("chosen"));
        for (root, name) in [(&own, "dobo-2026-11-20_090000-daily"), (&own, "dobo-2026-11-21_090000-before-update-1.1.0"), (&chosen, "dobo-2026-11-22_090000-daily")] {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join(crate::paths::DB_FILE), b"x").unwrap();
        }
        let listed: Vec<(String, bool)> = list_in(&chosen, &own).into_iter().map(|b| (b.name, b.kept_in_data)).collect();
        assert_eq!(listed, [("dobo-2026-11-22_090000-daily".to_string(), false), ("dobo-2026-11-21_090000-before-update-1.1.0".to_string(), true)]);
        let own_only: Vec<bool> = list_in(&own, &own).into_iter().map(|b| b.kept_in_data).collect();
        assert_eq!(own_only, [false, false], "the default folder: each once, as its own");
        let _ = std::fs::remove_dir_all(&base);
    }
}
