// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! When the notes can't be opened at start: a calm screen (Recovery.svelte) instead of an exit
//! with no word, with what can be done about it.
//!
//! - **Damaged** (SQLite finds the file malformed, or not a database at all): nothing is renamed
//!   until a choice is made (a drive that answered badly once may read it whole on Try again).
//!   Before anything is offered, a repair is tried: whatever can still be read copied into a new
//!   file (`salvage`); when it works, "Use the repaired notes" comes first. Then: try again, put
//!   back the newest backup that is whole (its date shown), start with an empty board (the
//!   settings that can still be read come along), or quit. Each of those puts the damaged file
//!   aside first, as `broken-<date>.sqlite` in the data folder, never deleted, with its journal;
//!   should the choice then fail, it is put back as it was. The pictures it names are kept while
//!   it is there (lib.rs `purge_in`).
//! - **Put aside, no notes** (an earlier start left only a `broken-…` file): asked again, never an
//!   empty board by itself. The newest one is read again at each look: when it reads whole now,
//!   putting it back comes first.
//! - **Locked** (another program has the file open: an antivirus, a sync or backup tool) and
//!   **read-only** (the file or its folder can't be written): nothing is renamed. Try again (once
//!   that program lets go, or the file is writable again), or quit.
//! - **Won't open** (any other reason: a full disk, a drive error): try again, put back the newest
//!   backup or start empty (both put the file aside first, and back if they fail), or quit.
//! - **Portable, can't write next to itself** (paths.rs `blocked`: a read-only USB stick, a copy in
//!   Program Files): where the notes would be and why, then use the usual folder, or quit. Never
//!   the usual folder without a word: the notes would be split. (Once notes are there, later
//!   starts keep using them: paths.rs `decide`.)
//!
//! The board itself only starts once the notes are open (`lib.rs` `open_board`): until then
//! nothing reads or writes them, no reminder rings and no backup is made.

use crate::{backup, db, paths};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What stands in the way.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Trouble {
    Damaged,
    Locked,
    Readonly,
    Failed,
    /// A portable copy whose folder can't be written (paths.rs `blocked`).
    Portable,
}

/// A backup that can be put back (checked whole).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Newest {
    pub path: String,
    pub at: i64,
    /// What it holds (None: it couldn't be counted).
    pub counts: Option<Counts>,
}

/// What notes hold, said on a choice's card so that it can be weighed ("12 notes and 87 lines"):
/// every note (archived ones too) and every line.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub notes: i64,
    pub lines: i64,
}

/// What the notes in `file` hold, read only (None when they can't be counted).
fn counts(file: &Path) -> Option<Counts> {
    let conn = Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    db::harden(&conn).ok()?;
    let count = |table: &str| conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get::<_, i64>(0)).ok();
    Some(Counts { notes: count("notes")?, lines: count("items")? })
}

/// What the recovery screen shows.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Problem {
    pub kind: Trouble,
    /// SQLite's or Windows' own words (shown small, for whoever helps).
    pub error: String,
    /// The data folder.
    pub data: String,
    /// The notes file.
    pub file: String,
    /// The file an earlier start put aside (no notes file in its place), in the data folder.
    pub kept_as: Option<String>,
    /// The newest backup that is whole.
    pub backup: Option<Newest>,
    /// Damaged: a repaired copy is ready (`salvage`), "Use the repaired notes".
    pub repaired: bool,
    /// What the repaired copy holds: a repair may miss notes it couldn't read.
    pub repaired_counts: Option<Counts>,
    /// The file put aside (`kept_as`) reads whole now: it can be put back.
    pub whole_again: bool,
    /// Portable: the folder next to Dobo that can't be written, and whether notes are in it.
    pub portable: Option<String>,
    pub portable_notes: bool,
}

/// Where the start stands (Tauri state, while the recovery screen shows).
pub enum Stage {
    /// The screen shows this.
    Waiting(Problem),
    /// The notes are open: the board can start (`lib.rs` `recovery_continue`); with the name the
    /// damaged file was put aside as, when it was.
    Ready(Connection, Option<String>),
    /// The board has started.
    Started,
}

pub struct Recovery(pub Mutex<Stage>);

/// What a first look at the notes found.
pub enum Look {
    Open(Connection),
    Trouble(Problem),
}

/// The notes file in `data`.
fn notes_file(data: &Path) -> PathBuf {
    data.join(paths::DB_FILE)
}

/// The repaired copy of damaged notes (`salvage`), in the data folder until it is used.
const REPAIRED: &str = ".ztodo-repaired.sqlite";

/// What an error from SQLite (or from opening the file) means here. SQLite's own words are fixed
/// English, whatever the language.
pub fn classify(error: &str) -> Trouble {
    let e = error.to_ascii_lowercase();
    if e.contains("malformed") || e.contains("not a database") || e.contains("file is encrypted") || e.contains("corrupt") {
        Trouble::Damaged
    } else if e.contains("database is locked") || e.contains("database table is locked") || e.contains("os error 32") || e.contains("os error 33") {
        Trouble::Locked
    } else if e.contains("readonly") || e.contains("read-only") || e.contains("os error 5)") || e.contains("access is denied") {
        Trouble::Readonly
    } else {
        Trouble::Failed
    }
}

/// Another program holds the file (Windows refuses to share it), or it can't be written (a
/// read-only file, no permission): found before SQLite tries, so it is said plainly.
fn held_or_read_only(file: &Path) -> Option<(Trouble, String)> {
    match std::fs::OpenOptions::new().read(true).write(true).open(file) {
        Ok(_) => None,
        // ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
        Err(e) if matches!(e.raw_os_error(), Some(32 | 33)) => Some((Trouble::Locked, e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Some((Trouble::Readonly, e.to_string())),
        Err(_) => None,
    }
}

/// Written in the data folder when Dobo ends normally with its notes open (`ended_normally`), and
/// taken by the next start's look: the notes then aren't read through to check them (`PRAGMA
/// quick_check` reads the whole file). After a crash, a power cut or a forced end, they are; and
/// whenever they don't open. It notes what the notes file looked like then (`fingerprint`): one
/// changed since, while Dobo was closed (a sync tool, a version put back from the file's history,
/// a bad sector rewritten), is read through all the same.
const CLEAN_EXIT: &str = ".clean-exit";

/// SQLite found damaged notes (or a file that isn't a database) at some point since Dobo started
/// (`listen_for_damage`): Dobo doesn't say it ended normally then, so the next start reads them
/// through and says what is wrong.
static DAMAGE_SEEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What SQLite says in its log (every error, on any connection): damage is noted (`DAMAGE_SEEN`).
/// (Must not use SQLite itself.)
fn sqlite_said(code: std::ffi::c_int, _message: &str) {
    if matches!(code & 0xff, rusqlite::ffi::SQLITE_CORRUPT | rusqlite::ffi::SQLITE_NOTADB) {
        DAMAGE_SEEN.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Listens to SQLite's log for damage (`DAMAGE_SEEN`). Once, at the very start, before anything
/// uses SQLite.
pub fn listen_for_damage() {
    // SAFETY: called once, first thing in `lib.rs` `run`, before any SQLite call and before any
    // other thread uses it; `sqlite_said` makes no SQLite call and only stores an atomic.
    if let Err(e) = unsafe { rusqlite::trace::config_log(Some(sqlite_said)) } {
        eprintln!("SQLite's log can't be listened to: {e}");
    }
}

/// What the notes file `file` looks like (its size, its time, its first and last pages), or None
/// when it can't be told or a log of changes not yet in it is there (`-wal`: not ended cleanly).
fn fingerprint(file: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    if std::fs::metadata(with_suffix(file, "-wal")).is_ok_and(|m| m.len() > 0) {
        return None;
    }
    let meta = std::fs::metadata(file).ok()?;
    let time = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let mut f = std::fs::File::open(file).ok()?;
    let mut bytes = vec![0u8; 4096];
    let head = f.read(&mut bytes).ok()?;
    bytes.truncate(head);
    if meta.len() > 4096 {
        f.seek(SeekFrom::End(-4096)).ok()?;
        let mut tail = vec![0u8; 4096];
        f.read_exact(&mut tail).ok()?;
        bytes.extend(tail);
    }
    // (FNV-1a: the same from one Dobo to the next.)
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3));
    Some(format!("{} {time} {hash:016x}", meta.len()))
}

/// Dobo ends normally, its notes open (`conn`): the next start needn't read them through first,
/// unless SQLite found damage meanwhile (`DAMAGE_SEEN`).
pub fn ended_normally(data: &Path, conn: &Connection) {
    if DAMAGE_SEEN.load(std::sync::atomic::Ordering::SeqCst) {
        let _ = std::fs::remove_file(data.join(CLEAN_EXIT));
        return;
    }
    closed_cleanly(data, conn);
}

/// `ended_normally`, damage or not: the log written into the file (so that the file alone is what
/// is noted), then what it looks like now (`fingerprint`).
fn closed_cleanly(data: &Path, conn: &Connection) {
    let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
    let mark = data.join(CLEAN_EXIT);
    match fingerprint(&notes_file(data)) {
        Some(seen) => {
            let words = format!("Dobo ended normally: its next start opens the notes without reading them through first, if they still look like this.\r\n{seen}\r\n");
            let _ = std::fs::write(mark, words);
        }
        None => {
            let _ = std::fs::remove_file(mark);
        }
    }
}

/// Did the last start end normally (`ended_normally`), the notes file as it was then? Taken now:
/// a crash during this one leaves none. (Taking it shows the folder can be written in, too.)
fn took_clean_exit(data: &Path) -> bool {
    let mark = data.join(CLEAN_EXIT);
    let Ok(words) = std::fs::read_to_string(&mark) else { return false };
    if std::fs::remove_file(&mark).is_err() {
        return false;
    }
    let seen = words.lines().nth(1).map(str::trim).filter(|s| !s.is_empty());
    seen.is_some() && seen.map(String::from) == fingerprint(&notes_file(data))
}

/// The notes file opened as it is, nothing changed in it (no table made, no column added), and
/// checked whole when `check` (else only its first page is read).
fn peek(file: &Path, check: bool) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
    db::harden(&conn)?;
    conn.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e| e.to_string())?;
    if check {
        let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        if check != "ok" {
            return Err(format!("database disk image is malformed ({check})"));
        }
    } else {
        // (A file that isn't a database at all says so here.)
        conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

/// The notes file opened as it is, for a quick write that needs no more (Setup's choices,
/// install.rs): nothing added to it, not read through.
pub fn open_as_is(file: &Path) -> Result<Connection, String> {
    peek(file, false)
}

/// No notes file in `data`, but something only the recovery screen may decide about: a file an
/// earlier start put aside, or a journal left alone (`clear_orphans`).
pub fn needs_a_look(data: &Path) -> bool {
    let file = notes_file(data);
    !file.exists() && (!put_aside_files(data).is_empty() || JOURNALS.iter().any(|j| with_suffix(&file, j).exists()))
}

/// A setting read from a notes file that may be damaged (None when it can't be).
pub fn salvaged_setting(file: &Path, key: &str) -> Option<serde_json::Value> {
    let conn = Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    db::harden(&conn).ok()?;
    db::get_setting(&conn, key)
}

/// What `whats_new_seen` the notes put aside call for (read while they still can be): their own,
/// else worked out from them as an update's first start would (`whatsnew::first_seen`), so a
/// board put back or started empty at an update's first start still shows What's new after the
/// recovery screen. None when nothing can be read from them.
fn seen_from(aside: &Path, running: &str) -> Option<serde_json::Value> {
    let all = salvaged_settings(aside);
    if all.is_empty() {
        return None;
    }
    let get = |k: &str| all.iter().find(|(key, _)| key == k).and_then(|(_, v)| serde_json::from_str::<serde_json::Value>(v).ok());
    if let Some(seen) = get(crate::whatsnew::SEEN) {
        return Some(seen);
    }
    let opened_by = get(backup::OPENED_BY).and_then(|v| v.as_str().map(String::from));
    Some(serde_json::json!(crate::whatsnew::first_seen(opened_by.as_deref(), get("onboarded").is_some(), running)))
}

/// Every setting that can still be read from a damaged file (empty when none can).
fn salvaged_settings(file: &Path) -> Vec<(String, String)> {
    let Ok(conn) = Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX) else { return vec![] };
    if db::harden(&conn).is_err() {
        return vec![];
    }
    conn.prepare("SELECT key, value FROM settings")
        .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>())
        .unwrap_or_default()
}

/// Settings never carried over to an empty board: the state of the backups (a failure to tell
/// about is the old notes'), the first start's own work (so the new board gets its Inbox), and
/// the version that last opened them (the new board is this version's: nothing to back up).
const NOT_CARRIED: [&str; 5] = ["first_run_done", "backup_error", "backup_failed_at", "backup_last", backup::OPENED_BY];

/// The files a notes file goes with (its journal).
const JOURNALS: [&str; 3] = ["-wal", "-shm", "-journal"];

/// The notes file `from` renamed `to`, its journal with it: all of it, or (should one of them not
/// go) none, put back as it was. (The write-ahead log holds the latest changes: never left behind
/// for a new file of the same name.)
fn move_notes(from: &Path, to: &Path) -> Result<(), String> {
    move_notes_with(from, to, |a, b| std::fs::rename(a, b))
}

fn move_notes_with(from: &Path, to: &Path, rename: impl Fn(&Path, &Path) -> std::io::Result<()>) -> Result<(), String> {
    rename(from, to).map_err(|e| e.to_string())?;
    let mut moved: Vec<&str> = vec![];
    for j in JOURNALS {
        let f = with_suffix(from, j);
        if !f.exists() {
            continue;
        }
        if let Err(e) = rename(&f, &with_suffix(to, j)) {
            // Those already moved go back first (the last first), then the file itself.
            for done in moved.iter().rev() {
                let _ = rename(&with_suffix(to, done), &with_suffix(from, done));
            }
            let _ = rename(to, from);
            return Err(e.to_string());
        }
        moved.push(j);
    }
    Ok(())
}

/// The notes put aside as `broken-<date>.sqlite` (then `broken-<date>-2.sqlite`…) beside them,
/// with their journal: never deleted. Returns the new name.
pub fn put_aside(file: &Path, now: i64) -> Result<PathBuf, String> {
    let dir = file.parent().ok_or("no folder")?;
    let date = crate::clock::local_date(now);
    let free = |p: &Path| !p.exists() && JOURNALS.iter().all(|j| !with_suffix(p, j).exists());
    let mut to = dir.join(format!("broken-{date}.sqlite"));
    let mut k = 2;
    while !free(&to) {
        to = dir.join(format!("broken-{date}-{k}.sqlite"));
        k += 1;
    }
    move_notes(file, &to)?;
    Ok(to)
}

/// Journal files left with no notes file (`file` isn't there): never applied to new notes made in
/// its place. The shared-memory index goes (it is made again); a log is kept, renamed. Err: one
/// stays (held by another program, a folder that can't be written): no new notes file then.
fn clear_orphans(file: &Path, now: i64) -> Result<(), String> {
    if file.exists() {
        return Ok(());
    }
    for j in JOURNALS {
        let f = with_suffix(file, j);
        if !f.exists() {
            continue;
        }
        let gone = if j == "-shm" { std::fs::remove_file(&f) } else { std::fs::rename(&f, with_suffix(file, &format!("{j}.orphan-{now}"))) };
        gone.map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Every file put aside (`broken-….sqlite`) in `data`, oldest first.
pub fn put_aside_files(data: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(data) else { return vec![] };
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.starts_with("broken-") && n.ends_with(".sqlite")
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            m.is_file().then_some(())?;
            Some((m.modified().ok()?, e.path()))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

/// The newest file put aside (`broken-….sqlite`) in `data`.
fn newest_put_aside(data: &Path) -> Option<PathBuf> {
    put_aside_files(data).pop()
}

/// The repaired copy (`REPAIRED`) and its journal, gone.
fn drop_repaired(data: &Path) {
    let r = data.join(REPAIRED);
    let _ = std::fs::remove_file(&r);
    for j in JOURNALS {
        let _ = std::fs::remove_file(with_suffix(&r, j));
    }
}

/// Damaged notes (`from`) copied into a new file, whatever can still be read (`VACUUM INTO` reads
/// every table through and makes every index again; it stops at a page it can't read), then
/// checked whole and Dobo's. Kept in the data folder as `REPAIRED` when it works.
fn salvage(from: &Path, data: &Path) -> bool {
    drop_repaired(data);
    let to = data.join(REPAIRED);
    let made = (|| -> Result<(), String> {
        let conn = Connection::open_with_flags(from, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
        db::harden(&conn)?;
        conn.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e| e.to_string())?;
        conn.execute("VACUUM INTO ?1", [to.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
        drop(conn);
        backup::whole(&to)
    })();
    if let Err(e) = &made {
        eprintln!("The damaged notes couldn't be repaired: {e}");
        drop_repaired(data);
    }
    made.is_ok()
}

/// The backups folders to look in: the one chosen in Settings (read from the notes, when they can
/// still be read), and the default one.
fn backup_roots(data: &Path, from: Option<&Path>) -> Vec<PathBuf> {
    let chosen = from.and_then(|f| salvaged_setting(f, "backup_dir")).and_then(|v| v.as_str().map(String::from)).filter(|s| !s.trim().is_empty());
    let mut roots = vec![];
    if let Some(c) = chosen {
        roots.push(PathBuf::from(c));
    }
    roots.push(data.join("backups"));
    roots
}

fn problem(kind: Trouble, error: String, data: &Path, kept_as: Option<&Path>) -> Problem {
    let backup = matches!(kind, Trouble::Damaged | Trouble::Failed)
        .then(|| backup::newest_whole(&backup_roots(data, kept_as.or(Some(&notes_file(data))))))
        .flatten()
        .map(|b| Newest { counts: counts(&Path::new(&b.path).join(paths::DB_FILE)), path: b.path, at: b.at });
    Problem {
        kind,
        error,
        data: data.to_string_lossy().into(),
        file: notes_file(data).to_string_lossy().into(),
        kept_as: kept_as.and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into()),
        backup,
        repaired: false,
        repaired_counts: None,
        whole_again: false,
        portable: None,
        portable_notes: false,
    }
}

/// Damaged notes in `from` (the notes file in place, or one put aside, `kept_as`): a repair is
/// tried first.
fn damaged(error: String, data: &Path, from: &Path, kept_as: Option<&Path>) -> Problem {
    let repaired = salvage(from, data);
    Problem { repaired, repaired_counts: repaired.then(|| counts(&data.join(REPAIRED))).flatten(), ..problem(Trouble::Damaged, error, data, kept_as) }
}

/// Where a file put aside is read again (`aside_problem`), in the data folder.
const ASIDE_COPY: &str = ".ztodo-aside-check.sqlite";

/// No notes file, but one an earlier start put aside (`aside`): read again. Whole now, it can be
/// put back; else, damaged notes. Read through a copy (with its journal), never the file itself:
/// opening it would roll its journal back or write its log into it, and it must stay as it was.
fn aside_problem(data: &Path, aside: &Path) -> Problem {
    let copy = data.join(ASIDE_COPY);
    let drop_copy = || {
        let _ = std::fs::remove_file(&copy);
        for j in JOURNALS {
            let _ = std::fs::remove_file(with_suffix(&copy, j));
        }
    };
    drop_copy();
    let copied = (|| -> Result<(), String> {
        std::fs::copy(aside, &copy).map_err(|e| e.to_string())?;
        // (Its log and journal go with it; the shared-memory index is made again.)
        for j in ["-wal", "-journal"] {
            let f = with_suffix(aside, j);
            if f.exists() {
                std::fs::copy(&f, with_suffix(&copy, j)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })();
    let found = match copied.and_then(|()| peek(&copy, true).map(drop)) {
        Ok(()) => Problem { whole_again: true, ..problem(Trouble::Damaged, String::new(), data, Some(aside)) },
        Err(e) => damaged(e, data, &copy, Some(aside)),
    };
    drop_copy();
    found
}

/// A portable copy that can't write in its own folder (`blocked`): asked before the usual folder
/// (`data`) is used.
pub fn portable_problem(data: &Path, blocked: &Path) -> Problem {
    Problem {
        portable: Some(blocked.to_string_lossy().into()),
        portable_notes: notes_file(blocked).is_file(),
        ..problem(Trouble::Portable, String::new(), data, None)
    }
}

/// A first look at the notes in `data`, before anything else uses them: open (`db::open`), or
/// what stands in the way. A look never renames anything.
pub fn first_look(data: &Path, now: i64) -> Look {
    look_with(data, env!("CARGO_PKG_VERSION"), now)
}

/// `first_look`, for the version `running`: a new version makes a backup of the notes as they
/// were before it first opens them (backup.rs `before_update`); should that fail, the notes open
/// all the same and Settings › Backups says so.
fn look_with(data: &Path, running: &str, now: i64) -> Look {
    let file = notes_file(data);
    // (Ended normally last time: the notes aren't read through first.)
    let mut clean = false;
    if file.exists() {
        if let Some((kind, error)) = held_or_read_only(&file) {
            return Look::Trouble(problem(kind, error, data, None));
        }
        clean = took_clean_exit(data);
    } else if let Some(aside) = newest_put_aside(data) {
        // Put aside by an earlier start, nothing in its place: asked again.
        return Look::Trouble(aside_problem(data, &aside));
    }
    // (A folder that can't be written: the notes can't be saved, nor their journal made; nor is
    // a backup made before an update, at each start, while it stays so.)
    if !clean && !paths::is_writable(data) {
        return Look::Trouble(problem(Trouble::Readonly, tr!("The folder can’t be written.", "Le dossier ne peut pas être modifié."), data, None));
    }
    // No notes yet (a new computer, or a file gone): a journal left alone never comes into the new
    // ones; one that can't be moved away, no new file.
    if let Err(e) = clear_orphans(&file, now) {
        return Look::Trouble(problem(classify(&e), e, data, None));
    }
    // A word for Settings › Backups about the backup made before this version opened the notes.
    let mut said = None;
    if file.exists() {
        match peek(&file, !clean) {
            Ok(conn) => match backup::before_update(&conn, data, running, now) {
                Ok(Some(backup::UpdateBackup { dir, instead_of: Some((chosen, why)) })) => {
                    let notice = backup::Notice::UpdateBackupElsewhere {
                        app: crate::APP_NAME.into(),
                        version: running.into(),
                        path: dir.display().to_string(),
                        chosen: chosen.display().to_string(),
                        why: Some(why),
                    };
                    // (Said while that backup is there.)
                    said = Some((notice, Some(dir)));
                }
                Ok(_) => {}
                // (The copy read the notes through and found them damaged: said now, on the
                // recovery screen, when a full read confirms it; never a board that fails.)
                Err(e) => {
                    if classify(&e.error) == Trouble::Damaged
                        && let Err(found) = peek(&file, true)
                    {
                        drop(conn);
                        return trouble_with(&file, data, found);
                    }
                    eprintln!("No backup before this version opened the notes: {}", e.error);
                    // (Said until a backup works.)
                    said = Some((backup::Notice::UpdateBackupFailed { app: crate::APP_NAME.into(), version: running.into(), error: e.error, why: Some(e.why) }, None));
                }
            },
            Err(e) => return trouble_with(&file, data, e),
        }
    }
    match db::open(&file) {
        Ok(conn) => {
            // This version has opened them (the next new one makes its backup first). Before it
            // says so: the version they had, for What's new (whatsnew.rs).
            let opened_by = db::get_setting(&conn, backup::OPENED_BY).and_then(|v| v.as_str().map(String::from));
            crate::whatsnew::note_first(&conn, opened_by.as_deref(), running);
            if opened_by.as_deref() != Some(running) {
                let _ = db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!(running));
            }
            // (Said in Settings › Backups on its own, until a backup works.)
            if let Some((e, file)) = said {
                backup::note_notice(&conn, now, &e, file.as_deref());
            }
            // (A repaired copy left by a screen that was quit: not needed any more.)
            drop_repaired(data);
            Look::Open(conn)
        }
        // (Not read through at this start: now they are, to say what is wrong.)
        Err(e) => trouble_with(&file, data, if clean { peek(&file, true).err().unwrap_or(e) } else { e }),
    }
}

/// The notes wouldn't open (`error`): nothing renamed; damaged ones get a repair tried.
fn trouble_with(file: &Path, data: &Path, error: String) -> Look {
    let kind = classify(&error);
    if kind == Trouble::Damaged {
        return Look::Trouble(damaged(error, data, file, None));
    }
    Look::Trouble(problem(kind, error, data, None))
}

/// What the person chose on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    /// Use the repaired copy (`salvage`).
    Repair,
    /// Put back the file an earlier start put aside (it reads whole now).
    Putback,
    /// Put back the newest backup.
    Restore,
    /// Start with an empty board.
    Empty,
    /// Look again (the other program let go, the file is writable again, the drive reads well).
    Retry,
    /// Portable: the usual folder.
    Usual,
}

/// Is `choice` one the screen offers for `problem`?
fn offered(problem: &Problem, choice: Choice) -> bool {
    use Trouble::*;
    match choice {
        Choice::Usual => problem.kind == Portable,
        Choice::Retry => matches!(problem.kind, Damaged | Locked | Readonly | Failed),
        Choice::Restore | Choice::Empty => matches!(problem.kind, Damaged | Failed),
        Choice::Repair => problem.kind == Damaged && problem.repaired,
        Choice::Putback => problem.kind == Damaged && problem.whole_again && problem.kept_as.is_some(),
    }
}

/// Does what was chosen for `problem`; then looks again (`first_look`). Err: it couldn't be done,
/// said on the screen, and nothing was changed (a file put aside for it is back in its place).
/// Done: `problem.kept_as` names the file put aside, if one was.
pub fn choose(data: &Path, problem: &mut Problem, choice: Choice, now: i64) -> Result<Look, String> {
    if !offered(problem, choice) {
        return Err("That isn’t one of the choices.".into());
    }
    if matches!(choice, Choice::Retry | Choice::Usual) {
        return Ok(first_look(data, now));
    }
    let file = notes_file(data);
    // A notes file is back in its place (a sync tool brought it back): what it is now decides,
    // never put aside to put the older one in its place.
    if choice == Choice::Putback && file.exists() {
        return Ok(first_look(data, now));
    }
    let repaired = data.join(REPAIRED);
    // (What it needs is there and whole, before anything is put aside.)
    let backup_dir = match choice {
        Choice::Restore => {
            let b = problem.backup.as_ref().ok_or_else(|| tr!("There is no backup to put back.", "Il n’y a pas de sauvegarde à restaurer."))?;
            backup::whole(&Path::new(&b.path).join(paths::DB_FILE))?;
            Some(PathBuf::from(&b.path))
        }
        Choice::Repair => {
            backup::whole(&repaired).map_err(|e| tr!("The repaired notes can’t be used ({}).", "Les notes réparées ne peuvent pas être utilisées ({}).", e))?;
            None
        }
        _ => None,
    };
    // The file still in place (it wouldn't open): put aside first, never replaced.
    let in_place = file.exists();
    let aside = if in_place { Some(put_aside(&file, now)?) } else { problem.kept_as.as_ref().map(|name| data.join(name)) };
    let done = clear_orphans(&file, now);
    let done = done.and_then(|()| match (choice, &backup_dir, &aside) {
        (Choice::Restore, Some(b), _) => {
            let given = aside.as_deref().map(backup::sequences_of).unwrap_or_default();
            // (This computer's own choices, from the notes put aside when they can still be read.)
            let mut here: Vec<(String, serde_json::Value)> =
                aside.as_deref().map(|a| backup::CHOSEN_HERE.iter().filter_map(|k| salvaged_setting(a, k).map(|v| (k.to_string(), v))).collect()).unwrap_or_default();
            // (And what What's new still has to show, as the notes put aside had it.)
            if let Some(seen) = aside.as_deref().and_then(|a| seen_from(a, env!("CARGO_PKG_VERSION"))) {
                here.push((crate::whatsnew::SEEN.to_string(), seen));
            }
            backup::put_back_into(b, data, &given, &here)
        }
        (Choice::Empty, _, _) => db::open(&file).map(|conn| {
            if let Some(from) = &aside {
                for (key, value) in salvaged_settings(from) {
                    if !NOT_CARRIED.contains(&key.as_str()) {
                        let _ = conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", [&key, &value]);
                    }
                }
            }
            // (What What's new still has to show, as the notes put aside had it.)
            if let Some(seen) = aside.as_deref().and_then(|a| seen_from(a, env!("CARGO_PKG_VERSION"))) {
                let _ = db::set_setting(&conn, crate::whatsnew::SEEN, &seen);
            }
            // (Made by this version: no backup of an empty board before it opens it.)
            let _ = db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!(env!("CARGO_PKG_VERSION")));
        }),
        (Choice::Repair, _, _) => move_notes(&repaired, &file),
        (Choice::Putback, _, Some(a)) => move_notes(a, &file),
        _ => Err("That isn’t one of the choices.".into()),
    });
    if let Err(e) = done {
        // Not done after all: the file put aside for it goes back in its place.
        return Err(match &aside {
            Some(a) if in_place => put_back_after(&file, a, e, problem),
            _ => e,
        });
    }
    problem.kept_as = if choice == Choice::Putback { None } else { aside.and_then(|a| a.file_name().map(|n| n.to_string_lossy().into())) };
    drop_repaired(data);
    Ok(first_look(data, now))
}

/// A choice that failed (`error`) once the notes `file` were put aside (`aside`) for it: they go
/// back in their place, as they were (a new one half-made meanwhile goes first). Should that fail
/// too, they stay where they are, said, and named on the screen from now on (`problem.kept_as`).
/// Returns what to say.
fn put_back_after(file: &Path, aside: &Path, error: String, problem: &mut Problem) -> String {
    for f in std::iter::once(file.to_path_buf()).chain(JOURNALS.iter().map(|j| with_suffix(file, j))) {
        let _ = std::fs::remove_file(f);
    }
    match move_notes(aside, file) {
        Ok(()) => error,
        Err(back) => {
            eprintln!("The notes put aside couldn't go back in their place: {back}");
            let name = aside.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            problem.kept_as = Some(name.clone());
            tr!("{} Your notes are kept as {}, in the data folder.", "{} Vos notes sont gardées sous le nom {}, dans le dossier des données.", error, name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dobo-recovery-{tag}-{}-{:?}", db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn notes_with(data: &Path, title: &str) {
        let conn = db::open(&notes_file(data)).unwrap();
        db::create_note(&conn, title, "work", "soft", "Butter").unwrap();
        db::set_setting(&conn, "language", &serde_json::json!("fr")).unwrap();
    }

    fn titles(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT title FROM notes ORDER BY id").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
    }

    /// A notes file whose pages are scribbled over (as a disk error leaves one).
    fn damage(file: &Path) {
        let mut bytes = std::fs::read(file).unwrap();
        for b in bytes.iter_mut().skip(100).take(4000) {
            *b = 0x5a;
        }
        std::fs::write(file, bytes).unwrap();
    }

    /// A backup of `title`'s notes in `data`'s default backups folder, as Dobo makes them.
    fn backup_of(data: &Path, title: &str, stamp: &str) -> PathBuf {
        let dir = data.join("backups").join(format!("dobo-{stamp}-daily"));
        std::fs::create_dir_all(&dir).unwrap();
        let conn = db::open(&dir.join(paths::DB_FILE)).unwrap();
        db::create_note(&conn, title, "work", "soft", "Butter").unwrap();
        conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
        dir
    }

    #[test]
    fn whole_notes_just_open() {
        let data = temp("whole");
        notes_with(&data, "Groceries");
        let Look::Open(conn) = first_look(&data, db::now_ms()) else { panic!("whole notes open") };
        assert_eq!(titles(&conn), ["Groceries"]);
        drop(conn);
        // A new computer: no notes yet, a new board.
        let fresh = temp("fresh");
        assert!(matches!(first_look(&fresh, db::now_ms()), Look::Open(_)));
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&fresh);
    }

    #[test]
    fn sqlite_s_words_are_read_right() {
        assert_eq!(classify("database disk image is malformed"), Trouble::Damaged);
        assert_eq!(classify("file is not a database"), Trouble::Damaged);
        assert_eq!(classify("database is locked"), Trouble::Locked);
        assert_eq!(classify("The process cannot access the file because it is being used by another process. (os error 32)"), Trouble::Locked);
        assert_eq!(classify("attempt to write a readonly database"), Trouble::Readonly);
        assert_eq!(classify("Access is denied. (os error 5)"), Trouble::Readonly);
        assert_eq!(classify("disk I/O error"), Trouble::Failed);
        assert_eq!(classify("unable to open database file"), Trouble::Failed);
    }

    /// Damaged: nothing renamed by a look (Quit leaves it as it was); the newest whole backup
    /// offered; putting it back puts the damaged file aside first (never deleted, as it was).
    #[test]
    fn damaged_notes_are_put_aside_only_once_a_backup_comes_back() {
        let data = temp("damaged");
        notes_with(&data, "Mine");
        let older = backup_of(&data, "From Monday", "2026-11-16_090000");
        let newer = backup_of(&data, "From Tuesday", "2026-11-17_090000");
        // The newest one is damaged too: the one before it is offered.
        let broken_backup = backup_of(&data, "From Wednesday", "2026-11-18_090000");
        damage(&broken_backup.join(paths::DB_FILE));
        let file = notes_file(&data);
        damage(&file);
        let before = std::fs::read(&file).unwrap();
        let now = db::now_ms();
        let Look::Trouble(p) = first_look(&data, now) else { panic!("damaged notes don't open") };
        assert_eq!(p.kind, Trouble::Damaged);
        assert_eq!(p.kept_as, None, "nothing renamed by a look");
        assert!(!p.repaired, "its first page is gone: nothing to repair");
        assert_eq!(std::fs::read(&file).unwrap(), before, "left as it was");
        assert_eq!(p.backup.as_ref().map(|b| PathBuf::from(&b.path)), Some(newer.clone()));
        assert_eq!(p.backup.as_ref().and_then(|b| b.counts), Some(Counts { notes: 1, lines: 0 }), "what it holds, said on its card");
        // Quit, start again: asked again, the same, never an empty board by itself.
        let Look::Trouble(mut again) = first_look(&data, now) else { panic!("asked again") };
        assert_eq!(again, p);
        // Try again: still damaged, still in place.
        let Ok(Look::Trouble(still)) = choose(&data, &mut again, Choice::Retry, now) else { panic!("still damaged") };
        assert_eq!((still.kind, std::fs::read(&file).unwrap() == before), (Trouble::Damaged, true));
        let Ok(Look::Open(conn)) = choose(&data, &mut again, Choice::Restore, now) else { panic!("the backup is back") };
        assert_eq!(titles(&conn), ["From Tuesday"]);
        drop(conn);
        let kept = again.kept_as.clone().expect("named once put aside");
        assert_eq!(kept, format!("broken-{}.sqlite", crate::clock::local_date(now)));
        assert_eq!(std::fs::read(data.join(&kept)).unwrap(), before, "kept as it was");
        assert!(older.is_dir() && newer.is_dir());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// The notes table's root page scribbled over (a disk error), the settings still readable.
    fn damage_the_notes_only(file: &Path) {
        let (root, size): (i64, i64) = {
            let conn = Connection::open(file).unwrap();
            conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
            let root = conn.query_row("SELECT rootpage FROM sqlite_master WHERE name = 'notes'", [], |r| r.get(0)).unwrap();
            let settings: i64 = conn.query_row("SELECT rootpage FROM sqlite_master WHERE name = 'settings'", [], |r| r.get(0)).unwrap();
            assert_ne!(root, settings);
            (root, conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap())
        };
        let mut bytes = std::fs::read(file).unwrap();
        let at = ((root - 1) * size) as usize;
        bytes[at..at + size as usize].fill(0x5a);
        std::fs::write(file, bytes).unwrap();
    }

    fn journal_delete(file: &Path) {
        Connection::open(file).unwrap().execute_batch("PRAGMA journal_mode = DELETE").unwrap();
    }

    /// Start empty: a new board, with the settings that could still be read. A choice that can't
    /// be done (no backup, nothing repaired) renames nothing.
    #[test]
    fn an_empty_board_keeps_the_settings_that_can_be_read() {
        let data = temp("empty");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        damage_the_notes_only(&file);
        let before = std::fs::read(&file).unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("damaged") };
        assert_eq!(p.kind, Trouble::Damaged, "{}", p.error);
        assert!(!p.repaired, "the notes can't be read: no repair");
        assert!(!data.join(REPAIRED).exists());
        assert_eq!(salvaged_setting(&file, "language"), Some(serde_json::json!("fr")), "the settings still read");
        assert_eq!(p.backup, None, "no backups yet");
        assert!(choose(&data, &mut p, Choice::Restore, db::now_ms()).is_err(), "nothing to put back");
        assert!(choose(&data, &mut p, Choice::Repair, db::now_ms()).is_err(), "nothing repaired");
        assert_eq!(std::fs::read(&file).unwrap(), before, "nothing renamed for a choice that can't be done");
        assert!(put_aside_files(&data).is_empty());
        let Ok(Look::Open(conn)) = choose(&data, &mut p, Choice::Empty, db::now_ms()) else { panic!("an empty board") };
        assert!(titles(&conn).is_empty());
        assert_eq!(db::get_setting(&conn, "language"), Some(serde_json::json!("fr")));
        assert_eq!(db::get_setting(&conn, backup::OPENED_BY), Some(serde_json::json!(env!("CARGO_PKG_VERSION"))), "this version's board");
        assert!(!data.join("backups").exists(), "no backup of an empty board");
        let aside = data.join(p.kept_as.clone().unwrap());
        assert_eq!(std::fs::read(&aside).unwrap(), before);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Damage nothing is lost to (pages no table uses: "Page 4 is never used"): repaired, and the
    /// repaired notes come first; using them puts the damaged file aside.
    #[test]
    fn notes_that_can_still_be_read_whole_are_repaired() {
        let data = temp("repair");
        let file = notes_file(&data);
        {
            let conn = db::open(&file).unwrap();
            for k in 0..40 {
                db::create_note(&conn, &format!("Note {k} {}", "x".repeat(2000)), "work", "soft", "Butter").unwrap();
            }
            conn.execute("DELETE FROM notes WHERE id > 20", []).unwrap();
            conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
            let free: i64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0)).unwrap();
            assert!(free > 0);
        }
        // The free pages forgotten (the header's list of them emptied): never used, says SQLite.
        let mut bytes = std::fs::read(&file).unwrap();
        bytes[32..40].fill(0);
        std::fs::write(&file, &bytes).unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("damaged") };
        assert_eq!(p.kind, Trouble::Damaged);
        assert!(p.error.contains("never used"), "{}", p.error);
        assert!(p.repaired && data.join(REPAIRED).is_file());
        assert_eq!(p.repaired_counts, Some(Counts { notes: 20, lines: 0 }), "what the repair holds, said on its card");
        assert_eq!(std::fs::read(&file).unwrap(), bytes, "left as it was");
        let Ok(Look::Open(conn)) = choose(&data, &mut p, Choice::Repair, db::now_ms()) else { panic!("repaired") };
        assert_eq!(titles(&conn).len(), 20, "every note");
        drop(conn);
        assert_eq!(std::fs::read(data.join(p.kept_as.unwrap())).unwrap(), bytes, "the damaged file kept as it was");
        assert!(!data.join(REPAIRED).exists(), "used");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Won't open for another reason (here: not a database at all): nothing renamed by a look nor
    /// by Try again; put aside, not lost, once a choice is made.
    #[test]
    fn a_file_that_wont_open_is_put_aside_only_when_a_choice_is_made() {
        let data = temp("failed");
        let file = notes_file(&data);
        std::fs::write(&file, b"not quite").unwrap();
        let mut p = problem(Trouble::Failed, "disk I/O error".into(), &data, None);
        let Ok(Look::Trouble(mut still)) = choose(&data, &mut p, Choice::Retry, db::now_ms()) else { panic!("still won't open") };
        assert_eq!(still.kind, Trouble::Damaged, "(this one turns out damaged)");
        assert_eq!(still.kept_as, None);
        assert_eq!(std::fs::read(&file).unwrap(), b"not quite", "untouched before a choice");
        assert!(put_aside_files(&data).is_empty());
        let Ok(Look::Open(_)) = choose(&data, &mut still, Choice::Empty, db::now_ms()) else { panic!() };
        let aside = data.join(still.kept_as.unwrap());
        assert_eq!(std::fs::read(aside).unwrap(), b"not quite");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A choice that fails once the file is put aside (here: the backup's pictures can't come in)
    /// puts it back in its place, as it was: the next look finds the same, never "damaged".
    #[test]
    fn a_choice_that_fails_puts_the_file_back() {
        let data = temp("failback");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        journal_delete(&file);
        let before = std::fs::read(&file).unwrap();
        let b = backup_of(&data, "From Tuesday", "2026-11-17_090000");
        std::fs::create_dir_all(b.join("pictures")).unwrap();
        std::fs::write(b.join("pictures").join("1-0.png"), b"png").unwrap();
        // ("pictures" is a file here: the backup's can't be copied in.)
        std::fs::write(data.join("pictures"), b"not a folder").unwrap();
        let mut p = problem(Trouble::Failed, "disk I/O error".into(), &data, None);
        assert!(p.backup.is_some());
        assert!(choose(&data, &mut p, Choice::Restore, db::now_ms()).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), before, "back in its place");
        assert!(put_aside_files(&data).is_empty(), "nothing left aside");
        assert_eq!(p.kept_as, None);
        assert!(matches!(first_look(&data, db::now_ms()), Look::Open(_)), "and it opens");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Only a file put aside, nothing in its place (an earlier start's): read again at each look;
    /// whole now, it can be put back as it was (a stray log left by itself never comes into it).
    #[test]
    fn notes_put_aside_that_read_whole_again_can_be_put_back() {
        let data = temp("putback");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        journal_delete(&file);
        let aside = put_aside(&file, db::now_ms()).unwrap();
        std::fs::write(with_suffix(&file, "-wal"), b"stray").unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("asked") };
        assert_eq!((p.kind, p.whole_again, p.kept_as.as_deref()), (Trouble::Damaged, true, aside.file_name().and_then(|n| n.to_str())));
        let Ok(Look::Open(conn)) = choose(&data, &mut p, Choice::Putback, db::now_ms()) else { panic!("put back") };
        assert_eq!(titles(&conn), ["Mine"]);
        drop(conn);
        assert!(!aside.exists() && p.kept_as.is_none());
        assert!(std::fs::read_dir(&data).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("ztodo.sqlite-wal.orphan-")), "the stray log kept aside");
        // Not whole: not offered.
        let mut other = problem(Trouble::Damaged, String::new(), &data, Some(&aside));
        assert!(choose(&data, &mut other, Choice::Putback, db::now_ms()).is_err());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Ended normally last time: the notes open without being read through (a fault in pages
    /// they don't use isn't looked for then); after a crash, they are read through.
    #[test]
    fn notes_are_read_through_only_after_a_start_that_didnt_end_normally() {
        let data = temp("clean");
        let file = notes_file(&data);
        {
            let conn = db::open(&file).unwrap();
            for k in 0..30 {
                db::create_note(&conn, &format!("Note {k} {}", "x".repeat(2000)), "work", "soft", "Butter").unwrap();
            }
            conn.execute("DELETE FROM notes WHERE id > 10", []).unwrap();
            conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
        }
        let mut bytes = std::fs::read(&file).unwrap();
        bytes[32..40].fill(0);
        std::fs::write(&file, &bytes).unwrap();
        mark_clean(&data);
        let Look::Open(conn) = first_look(&data, db::now_ms()) else { panic!("not read through") };
        drop(conn);
        assert!(!data.join(CLEAN_EXIT).exists(), "taken");
        // No word of a normal end (a crash): read through.
        let Look::Trouble(p) = first_look(&data, db::now_ms()) else { panic!("read through") };
        assert_eq!(p.kind, Trouble::Damaged);
        // A file that isn't a database says so even when not read through.
        std::fs::write(&file, b"not a database at all, not even its first page").unwrap();
        mark_clean(&data);
        let Look::Trouble(p) = first_look(&data, db::now_ms()) else { panic!("not a database") };
        assert_eq!(p.kind, Trouble::Damaged);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// `closed_cleanly` with a connection of its own, as the app's at its end.
    fn mark_clean(data: &Path) {
        let conn = Connection::open(notes_file(data)).unwrap();
        closed_cleanly(data, &conn);
    }

    /// The word of a normal end holds only for the notes file as it was then: changed since
    /// (a sync tool, a version put back from the file's history), it is read through all the
    /// same. The app's own end (its connection open, its log in WAL mode, closed after) is
    /// trusted.
    #[test]
    fn a_normal_end_is_trusted_only_for_the_file_as_it_was() {
        let data = temp("cleanfile");
        let file = notes_file(&data);
        {
            let conn = db::open(&file).unwrap();
            for k in 0..30 {
                db::create_note(&conn, &format!("Note {k} {}", "x".repeat(2000)), "work", "soft", "Butter").unwrap();
            }
            conn.execute("DELETE FROM notes WHERE id > 10", []).unwrap();
        }
        // The app at its end: WAL, its connection open while the word is written, a change just
        // before; closed after.
        let conn = db::open(&file).unwrap();
        db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!(env!("CARGO_PKG_VERSION"))).unwrap();
        db::create_note(&conn, "Last one", "work", "soft", "Butter").unwrap();
        closed_cleanly(&data, &conn);
        drop(conn);
        assert!(took_clean_exit(&data), "the app's own end");
        // A change while Dobo was closed (here, the free pages forgotten, as a bad copy would):
        // the word no longer holds, the notes are read through and found damaged.
        let conn = db::open(&file).unwrap();
        closed_cleanly(&data, &conn);
        drop(conn);
        Connection::open(&file).unwrap().execute_batch("PRAGMA journal_mode = DELETE").unwrap();
        let mut bytes = std::fs::read(&file).unwrap();
        bytes[32..40].fill(0);
        std::fs::write(&file, &bytes).unwrap();
        let Look::Trouble(p) = first_look(&data, db::now_ms()) else { panic!("read through") };
        assert_eq!(p.kind, Trouble::Damaged);
        assert!(!data.join(CLEAN_EXIT).exists(), "taken all the same");
        // A log left with changes not in the file: not said to have ended normally.
        let other = temp("cleanwal");
        let f2 = notes_file(&other);
        std::fs::write(&f2, b"x").unwrap();
        std::fs::write(with_suffix(&f2, "-wal"), b"changes").unwrap();
        assert_eq!(fingerprint(&f2), None);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&other);
    }

    /// Damage SQLite meets during a start (any connection) means no word of a normal end.
    #[test]
    fn damage_seen_during_a_start_means_no_word_of_a_normal_end() {
        let data = temp("damageseen");
        notes_with(&data, "Mine");
        let conn = db::open(&notes_file(&data)).unwrap();
        sqlite_said(rusqlite::ffi::SQLITE_BUSY, "database is locked");
        assert!(!DAMAGE_SEEN.load(std::sync::atomic::Ordering::SeqCst));
        // (SQLITE_CORRUPT_INDEX: an extended code of the same.)
        sqlite_said(rusqlite::ffi::SQLITE_CORRUPT | (3 << 8), "database corruption at line 1");
        ended_normally(&data, &conn);
        assert!(!data.join(CLEAN_EXIT).exists());
        DAMAGE_SEEN.store(false, std::sync::atomic::Ordering::SeqCst);
        sqlite_said(rusqlite::ffi::SQLITE_NOTADB, "file is not a database");
        assert!(DAMAGE_SEEN.swap(false, std::sync::atomic::Ordering::SeqCst));
        ended_normally(&data, &conn);
        assert!(data.join(CLEAN_EXIT).is_file());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Ended normally, but the backup before an update finds the notes damaged as it copies them:
    /// read through then, and the recovery screen says so (never a board that fails).
    #[test]
    fn damage_found_by_the_backup_before_an_update_goes_to_the_recovery_screen() {
        let data = temp("updatedamage");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        damage_the_notes_only(&file);
        mark_clean(&data);
        let said = std::fs::read_to_string(data.join(CLEAN_EXIT)).unwrap();
        assert_eq!(said.lines().nth(1).map(String::from), fingerprint(&file), "trusted: not read through before the backup");
        let Look::Trouble(p) = look_with(&data, "9.3.0", db::now_ms()) else { panic!("damaged") };
        assert_eq!(p.kind, Trouble::Damaged, "{}", p.error);
        assert!(!data.join("backups").exists() || backups_in(&data).is_empty(), "no backup of damaged notes");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A choice that failed once the notes were put aside: a new file half-made meanwhile goes
    /// first, then they go back in their place. Should that fail too, they are named on the screen
    /// from then on, and said.
    #[test]
    fn a_failed_choice_puts_the_notes_back_or_says_where_they_are() {
        let data = temp("putbackafter");
        let file = notes_file(&data);
        let aside = data.join("broken-x.sqlite");
        std::fs::write(&aside, b"mine").unwrap();
        std::fs::write(with_suffix(&aside, "-wal"), b"my log").unwrap();
        // (Half-made by the choice: a new file and its log.)
        std::fs::write(&file, b"half-made").unwrap();
        std::fs::write(with_suffix(&file, "-wal"), b"its log").unwrap();
        let mut p = problem(Trouble::Damaged, String::new(), &data, None);
        assert_eq!(put_back_after(&file, &aside, "it failed".into(), &mut p), "it failed");
        assert_eq!((std::fs::read(&file).unwrap(), std::fs::read(with_suffix(&file, "-wal")).unwrap()), (b"mine".to_vec(), b"my log".to_vec()));
        assert!(!aside.exists() && p.kept_as.is_none());
        // Can't go back (here a folder stands in its place): kept as it is, named, and said.
        std::fs::rename(&file, &aside).unwrap();
        std::fs::remove_file(with_suffix(&file, "-wal")).unwrap();
        std::fs::create_dir_all(&file).unwrap();
        let said = put_back_after(&file, &aside, "it failed".into(), &mut p);
        assert!(said.starts_with("it failed") && said.contains("broken-x.sqlite"), "{said}");
        assert_eq!(p.kept_as.as_deref(), Some("broken-x.sqlite"));
        assert_eq!(std::fs::read(&aside).unwrap(), b"mine");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Put my notes back, when a notes file came back in its place meanwhile (a sync tool): looked
    /// at again, the one there decides; the file put aside stays as it is.
    #[test]
    fn notes_back_in_their_place_are_never_put_aside_for_a_put_back() {
        let data = temp("reappeared");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        journal_delete(&file);
        let aside = put_aside(&file, db::now_ms()).unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("asked") };
        assert!(p.whole_again);
        // (Brought back by a sync tool.)
        std::fs::copy(&aside, &file).unwrap();
        let Ok(Look::Open(conn)) = choose(&data, &mut p, Choice::Putback, db::now_ms()) else { panic!("the one there") };
        assert_eq!(titles(&conn), ["Mine"]);
        drop(conn);
        assert!(aside.is_file(), "the file put aside stays");
        assert_eq!(put_aside_files(&data), [aside], "nothing else put aside");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A journal left with no notes file: moved away at the look (never applied to new notes); one
    /// that can't be, no new notes file (and no empty board over it).
    #[test]
    fn a_journal_left_alone_never_comes_into_new_notes() {
        let data = temp("orphans");
        let file = notes_file(&data);
        std::fs::write(with_suffix(&file, "-wal"), b"stray").unwrap();
        let Look::Open(conn) = first_look(&data, db::now_ms()) else { panic!("new notes") };
        drop(conn);
        assert!(std::fs::read_dir(&data).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("ztodo.sqlite-wal.orphan-")), "kept aside");
        let other = temp("orphans-held");
        let file = notes_file(&other);
        // (A folder where the index would be: it can't be removed.)
        std::fs::create_dir_all(with_suffix(&file, "-shm")).unwrap();
        std::fs::write(with_suffix(&file, "-shm").join("x"), b"x").unwrap();
        let Look::Trouble(p) = first_look(&other, db::now_ms()) else { panic!("said") };
        assert!(!file.exists(), "no new file");
        // Nor through a choice.
        let mut p = Problem { kind: Trouble::Failed, ..p };
        assert!(choose(&other, &mut p, Choice::Empty, db::now_ms()).is_err());
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&other);
    }

    /// The file put aside is read again through a copy: it and its log stay exactly as they were.
    #[test]
    fn notes_put_aside_are_read_again_without_being_changed() {
        let data = temp("asideasis");
        let src = temp("asideasis-src");
        let file = notes_file(&src);
        let conn = db::open(&file).unwrap();
        conn.execute_batch("PRAGMA wal_autocheckpoint = 0").unwrap();
        db::create_note(&conn, "In the log only", "work", "soft", "Butter").unwrap();
        // (Copied while its log still holds the last changes, as a file put aside may be.)
        let aside = data.join("broken-2026-11-20.sqlite");
        std::fs::copy(&file, &aside).unwrap();
        std::fs::copy(with_suffix(&file, "-wal"), with_suffix(&aside, "-wal")).unwrap();
        drop(conn);
        let (before, log) = (std::fs::read(&aside).unwrap(), std::fs::read(with_suffix(&aside, "-wal")).unwrap());
        assert!(!log.is_empty());
        let Look::Trouble(p) = first_look(&data, db::now_ms()) else { panic!("asked") };
        assert!(p.whole_again, "{}", p.error);
        assert_eq!(std::fs::read(&aside).unwrap(), before, "never changed");
        assert_eq!(std::fs::read(with_suffix(&aside, "-wal")).unwrap(), log, "its log neither");
        assert!(!data.join(ASIDE_COPY).exists(), "the copy gone");
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&src);
    }

    /// Each trouble takes only its own choices.
    #[test]
    fn only_the_choices_offered_are_taken() {
        let data = temp("offered");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        let mut b = problem(Trouble::Locked, String::new(), &data, None);
        b.backup = Some(Newest { path: data.join("nowhere").to_string_lossy().into(), at: 0, counts: None });
        for kind in [Trouble::Locked, Trouble::Readonly, Trouble::Portable] {
            let mut p = Problem { kind, ..b.clone() };
            for c in [Choice::Restore, Choice::Empty, Choice::Repair, Choice::Putback] {
                assert!(choose(&data, &mut p, c, db::now_ms()).is_err(), "{kind:?} {c:?}");
            }
        }
        // A backup that isn't there any more: refused before anything is put aside.
        let mut p = Problem { kind: Trouble::Failed, ..b.clone() };
        assert!(choose(&data, &mut p, Choice::Restore, db::now_ms()).is_err());
        assert!(file.is_file() && put_aside_files(&data).is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// One of the journal's files won't go: those that went come back, then the file itself.
    #[test]
    fn a_file_put_aside_half_way_comes_back_whole() {
        let data = temp("half");
        let file = notes_file(&data);
        for (f, what) in [(file.clone(), "notes"), (with_suffix(&file, "-wal"), "log"), (with_suffix(&file, "-shm"), "index")] {
            std::fs::write(f, what).unwrap();
        }
        let to = data.join("broken-x.sqlite");
        let refused = move_notes_with(&file, &to, |a, b| if a.to_string_lossy().ends_with("-shm") { Err(std::io::Error::other("in use")) } else { std::fs::rename(a, b) });
        assert!(refused.is_err());
        for (suffix, what) in [("", "notes"), ("-wal", "log"), ("-shm", "index")] {
            assert_eq!(std::fs::read_to_string(with_suffix(&file, suffix)).unwrap(), what, "{suffix}");
            assert!(!with_suffix(&to, suffix).exists(), "{suffix}");
        }
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Held by another program: said, nothing renamed; Try again opens it once let go.
    #[cfg(windows)]
    #[test]
    fn a_file_held_by_another_program_is_never_renamed() {
        use std::os::windows::fs::OpenOptionsExt;
        let data = temp("locked");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        // (Shared with no one: as an antivirus or a sync tool may hold it.)
        let held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&file).unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("held") };
        assert_eq!(p.kind, Trouble::Locked);
        assert_eq!(p.kept_as, None);
        assert!(file.is_file(), "never renamed");
        let Ok(Look::Trouble(_)) = choose(&data, &mut p, Choice::Retry, db::now_ms()) else { panic!("still held") };
        drop(held);
        let Ok(Look::Open(conn)) = choose(&data, &mut p, Choice::Retry, db::now_ms()) else { panic!("let go") };
        assert_eq!(titles(&conn), ["Mine"]);
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_read_only_file_is_said_and_left_as_it_is() {
        let data = temp("readonly");
        notes_with(&data, "Mine");
        let file = notes_file(&data);
        let mut perm = std::fs::metadata(&file).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&file, perm.clone()).unwrap();
        let Look::Trouble(mut p) = first_look(&data, db::now_ms()) else { panic!("read-only") };
        assert_eq!(p.kind, Trouble::Readonly);
        assert!(file.is_file());
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        std::fs::set_permissions(&file, perm).unwrap();
        assert!(matches!(choose(&data, &mut p, Choice::Retry, db::now_ms()), Ok(Look::Open(_))));
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A portable copy that can't write next to itself: said, with where the notes go then;
    /// only that choice (or Quit) is taken.
    #[test]
    fn a_portable_copy_that_cant_write_next_to_itself_asks_first() {
        let root = temp("portable");
        let (usual, next) = (root.join("usual"), root.join("Dobo").join("data"));
        std::fs::create_dir_all(&usual).unwrap();
        std::fs::create_dir_all(&next).unwrap();
        notes_with(&next, "On the stick");
        let mut p = portable_problem(&usual, &next);
        assert_eq!(p.kind, Trouble::Portable);
        assert_eq!(p.portable.as_deref(), Some(next.to_string_lossy().as_ref()));
        assert!(p.portable_notes);
        assert_eq!(p.data, usual.to_string_lossy());
        assert!(choose(&usual, &mut p, Choice::Empty, db::now_ms()).is_err());
        assert!(!usual.join(paths::DB_FILE).exists(), "nothing made before it is chosen");
        let Ok(Look::Open(conn)) = choose(&usual, &mut p, Choice::Usual, db::now_ms()) else { panic!("the usual folder") };
        assert!(titles(&conn).is_empty(), "its own notes (none yet), never the stick's moved");
        drop(conn);
        assert!(notes_file(&next).is_file(), "the notes next to Dobo stay as they are");
        assert!(!portable_problem(&usual, &root.join("empty")).portable_notes);
        // Every other trouble refuses it.
        let mut other = problem(Trouble::Locked, String::new(), &usual, None);
        assert!(choose(&usual, &mut other, Choice::Usual, db::now_ms()).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The first start after an update from 1.1.x notes the version the notes had, for What's
    /// new, before it writes its own (whatsnew.rs); a new board has nothing to show.
    #[test]
    fn the_version_before_an_update_is_noted_for_whats_new() {
        let data = temp("whats-new");
        {
            let conn = db::open(&notes_file(&data)).unwrap();
            db::set_setting(&conn, "onboarded", &serde_json::json!(true)).unwrap();
            db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!("1.1.1")).unwrap();
        }
        for k in 0..2 {
            // (Again at a later start, until the window notes it shown.)
            let Look::Open(conn) = look_with(&data, "1.2.0", db::now_ms() + k * 2000) else { panic!() };
            assert_eq!(db::get_setting(&conn, crate::whatsnew::SEEN), Some(serde_json::json!("1.1.1")));
            assert_eq!(db::get_setting(&conn, backup::OPENED_BY), Some(serde_json::json!("1.2.0")));
        }
        let fresh = temp("whats-new-first");
        let Look::Open(conn) = look_with(&fresh, "1.2.0", db::now_ms()) else { panic!() };
        assert_eq!(db::get_setting(&conn, crate::whatsnew::SEEN), Some(serde_json::json!("1.2.0")), "a first install");
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&fresh);
    }

    /// An update's first start finding the notes damaged: putting a backup back, or starting
    /// empty, still shows What's new after the recovery screen (the version the notes put aside
    /// were last opened by, or their own `whats_new_seen`).
    #[test]
    fn whats_new_still_shows_after_the_recovery_screen() {
        let running = env!("CARGO_PKG_VERSION");
        for (choice, had_seen) in [(Choice::Restore, false), (Choice::Empty, false), (Choice::Restore, true), (Choice::Empty, true)] {
            let data = temp("whats-new-recovery");
            notes_with(&data, "Mine");
            {
                let conn = db::open(&notes_file(&data)).unwrap();
                db::set_setting(&conn, "onboarded", &serde_json::json!(true)).unwrap();
                db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!("0.9.0")).unwrap();
                if had_seen {
                    db::set_setting(&conn, crate::whatsnew::SEEN, &serde_json::json!("0.9.5")).unwrap();
                }
            }
            if choice == Choice::Restore {
                backup_of(&data, "From Monday", "2026-11-16_090000");
            }
            damage_the_notes_only(&notes_file(&data));
            let Look::Trouble(mut p) = look_with(&data, running, db::now_ms()) else { panic!("damaged") };
            let Ok(Look::Open(conn)) = choose(&data, &mut p, choice, db::now_ms()) else { panic!("{choice:?}") };
            let want = if had_seen { "0.9.5" } else { "0.9.0" };
            assert_eq!(db::get_setting(&conn, crate::whatsnew::SEEN), Some(serde_json::json!(want)), "{choice:?}");
            drop(conn);
            let _ = std::fs::remove_dir_all(&data);
        }
        // Nothing readable put aside: nothing is carried over, so there is nothing to show
        // (Start empty: a new board; Restore: the backup's notes, opened by this version).
        // (Both files say an older version and were set up, so a readable put-aside file or the
        // backup's own "seen" would show here instead of the running version.)
        for choice in [Choice::Empty, Choice::Restore] {
            let data = temp("whats-new-recovery-none");
            notes_with(&data, "Mine");
            {
                let conn = db::open(&notes_file(&data)).unwrap();
                db::set_setting(&conn, "onboarded", &serde_json::json!(true)).unwrap();
                db::set_setting(&conn, backup::OPENED_BY, &serde_json::json!("0.9.0")).unwrap();
            }
            if choice == Choice::Restore {
                let dir = backup_of(&data, "From Monday", "2026-11-16_090000");
                let conn = db::open(&dir.join(paths::DB_FILE)).unwrap();
                db::set_setting(&conn, "onboarded", &serde_json::json!(true)).unwrap();
                db::set_setting(&conn, crate::whatsnew::SEEN, &serde_json::json!("0.9.5")).unwrap();
                conn.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
            }
            damage(&notes_file(&data));
            let Look::Trouble(mut p) = look_with(&data, running, db::now_ms()) else { panic!("damaged") };
            let Ok(Look::Open(conn)) = choose(&data, &mut p, choice, db::now_ms()) else { panic!("{choice:?}") };
            assert_eq!(db::get_setting(&conn, crate::whatsnew::SEEN), Some(serde_json::json!(running)), "{choice:?}");
            let on_board = titles(&conn);
            assert!(!on_board.contains(&"Mine".to_string()), "{choice:?}: {on_board:?}");
            if choice == Choice::Restore {
                assert_eq!(on_board, ["From Monday"]);
            }
            drop(conn);
            let _ = std::fs::remove_dir_all(&data);
        }
    }

    fn backups_in(data: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(data.join("backups")).map(|it| it.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
        v.sort();
        v
    }

    /// A new version's first start backs up the notes as they were, before anything is added to
    /// them; once, and never for the same or an older version.
    #[test]
    fn a_new_version_backs_up_the_notes_before_it_first_opens_them() {
        let data = temp("update");
        // Notes from before 1.1 (no version noted), as 1.0 left them.
        {
            let conn = db::open(&notes_file(&data)).unwrap();
            db::create_note(&conn, "From 1.0", "work", "soft", "Butter").unwrap();
            conn.execute_batch("ALTER TABLE notes DROP COLUMN archived_at").unwrap();
            std::fs::create_dir_all(data.join("pictures")).unwrap();
            std::fs::write(data.join("pictures").join("1-0.png"), b"png").unwrap();
        }
        let Look::Open(conn) = look_with(&data, "1.1.0", db::now_ms()) else { panic!() };
        assert_eq!(db::get_setting(&conn, backup::OPENED_BY), Some(serde_json::json!("1.1.0")));
        drop(conn);
        let made = backups_in(&data);
        assert_eq!(made.len(), 1);
        assert!(made[0].starts_with("dobo-") && made[0].ends_with("-before-update-1.1.0"), "{made:?}");
        let dir = data.join("backups").join(&made[0]);
        assert!(dir.join("pictures").join("1-0.png").is_file(), "with the pictures");
        let copy = Connection::open(dir.join(paths::DB_FILE)).unwrap();
        assert_eq!(titles(&copy), ["From 1.0"]);
        let cols: Vec<String> = copy.prepare("SELECT name FROM pragma_table_info('notes')").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert!(!cols.contains(&"archived_at".to_string()), "as they were, before this version added to them");
        drop(copy);
        // Again, or an older version: none.
        for v in ["1.1.0", "1.0.0"] {
            assert!(matches!(look_with(&data, v, db::now_ms()), Look::Open(_)));
        }
        assert_eq!(backups_in(&data).len(), 1);
        // The next version: one of its own.
        assert!(matches!(look_with(&data, "1.2.0", db::now_ms() + 2000), Look::Open(_)));
        assert_eq!(backups_in(&data).iter().filter(|n| n.ends_with("-before-update-1.2.0")).count(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A new computer (no notes yet): no backup of nothing.
    #[test]
    fn no_backup_before_the_first_notes() {
        let data = temp("noupdate");
        assert!(matches!(look_with(&data, "1.1.0", db::now_ms()), Look::Open(_)));
        assert!(backups_in(&data).is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// The backups folder can't be used: the notes open all the same, and Settings says so.
    #[test]
    fn a_backup_before_an_update_that_fails_never_stops_the_notes() {
        let data = temp("updatefail");
        notes_with(&data, "Mine");
        // ("backups" is a file: the folder can't be made.)
        std::fs::write(data.join("backups"), b"not a folder").unwrap();
        let Look::Open(conn) = look_with(&data, "9.0.0", db::now_ms()) else { panic!() };
        assert_eq!(titles(&conn), ["Mine"]);
        let st = backup::status(&conn);
        assert!(
            st.error.is_none() && matches!(st.notice, Some(backup::Said::Notice(backup::Notice::UpdateBackupFailed { ref version, .. })) if version == "9.0.0"),
            "said on its own, not as a failed daily backup"
        );
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A start that goes no further than the backup (the notes then don't open): the next one
    /// makes none again for the same version, so the others are never pushed out.
    #[test]
    fn one_backup_per_version_however_many_starts() {
        let data = temp("onceper");
        notes_with(&data, "Mine");
        let conn = peek(&notes_file(&data), true).unwrap();
        let first = backup::before_update(&conn, &data, "9.1.0", db::now_ms()).unwrap();
        assert!(first.is_some_and(|b| b.dir.is_dir() && b.instead_of.is_none()));
        for k in 1..4 {
            assert_eq!(backup::before_update(&conn, &data, "9.1.0", db::now_ms() + k * 2000).unwrap(), None);
        }
        assert_eq!(backups_in(&data).len(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// The backups folder chosen in Settings can't be used (a drive that is gone): the backup
    /// before an update goes into the data folder's own, and Settings › Backups says where.
    #[test]
    fn a_chosen_backups_folder_that_doesnt_answer_leaves_it_to_the_data_folder() {
        let data = temp("chosen");
        notes_with(&data, "Mine");
        std::fs::create_dir_all(data.join("pictures")).unwrap();
        std::fs::write(data.join("pictures").join("1-0.png"), b"png").unwrap();
        std::fs::write(data.join("gone"), b"a file, not a drive").unwrap();
        let chosen = data.join("gone").join("Backups");
        {
            let conn = db::open(&notes_file(&data)).unwrap();
            db::set_setting(&conn, "backup_dir", &serde_json::json!(chosen.to_string_lossy())).unwrap();
        }
        let Look::Open(conn) = look_with(&data, "9.2.0", db::now_ms()) else { panic!() };
        let made = backups_in(&data);
        assert_eq!(made.len(), 1, "in the data folder's own");
        let dir = data.join("backups").join(&made[0]);
        assert_eq!(std::fs::read(dir.join("pictures").join("1-0.png")).unwrap(), b"png", "with the pictures");
        let notice = backup::status(&conn).notice.unwrap();
        assert!(
            matches!(&notice, backup::Said::Notice(backup::Notice::UpdateBackupElsewhere { path, chosen: c, version, why: Some(backup::Missed::Failed), .. }) if *path == dir.display().to_string() && *c == chosen.display().to_string() && version == "9.2.0"),
            "{notice:?}"
        );
        // Said while that backup is there, whatever backups work meanwhile.
        conn.execute("DELETE FROM settings WHERE key = 'backup_error'", []).unwrap();
        assert!(backup::status(&conn).notice.is_some());
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(backup::status(&conn).notice, None, "gone with it");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_second_file_put_aside_the_same_day_gets_a_name_of_its_own() {
        let data = temp("names");
        let now = db::now_ms();
        let date = crate::clock::local_date(now);
        for (k, want) in [(1, format!("broken-{date}.sqlite")), (2, format!("broken-{date}-2.sqlite"))] {
            let file = notes_file(&data);
            std::fs::write(&file, format!("copy {k}")).unwrap();
            std::fs::write(with_suffix(&file, "-wal"), b"log").unwrap();
            let to = put_aside(&file, now).unwrap();
            assert_eq!(to.file_name().unwrap().to_string_lossy(), want);
            assert!(with_suffix(&to, "-wal").is_file(), "the log goes with it");
            assert!(!with_suffix(&file, "-wal").exists());
        }
        let _ = std::fs::remove_dir_all(&data);
    }
}
