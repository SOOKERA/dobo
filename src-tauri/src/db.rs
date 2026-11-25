// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The notes, their lines (tasks and ideas, with steps), pictures and the settings, in one
//! SQLite file in the app's data folder.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    pub id: i64,
    pub title: String,
    /// "work" or "personal".
    pub space: String,
    /// "soft", "vivid" or "custom".
    pub palette: String,
    /// The colour's name in its palette ("Lilac"), or the slot number for custom ("3").
    pub color: String,
    pub sort: f64,
    pub archived: bool,
    pub created_at: i64,
    /// Covers two columns of the board.
    #[serde(default)]
    pub wide: bool,
    /// Folded down to its title.
    #[serde(default)]
    pub folded: bool,
    /// "plain", "ruled", "grid" or "antique".
    #[serde(default = "paper_default")]
    pub paper: String,
    /// When it was archived (ms; set by `update_note`, None when not archived or archived by
    /// a version that didn't keep the time).
    #[serde(default)]
    pub archived_at: Option<i64>,
    /// Its done tasks move to the end of its list (else they stay where they are, the default).
    #[serde(default)]
    pub done_last: bool,
}

fn paper_default() -> String {
    "plain".into()
}

fn alarm_default() -> String {
    "reminder".into()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Item {
    pub id: i64,
    pub note_id: i64,
    /// "task" or "idea".
    pub kind: String,
    pub text: String,
    /// Milliseconds since 1970 (UTC).
    pub due_at: Option<i64>,
    pub done_at: Option<i64>,
    pub sort: f64,
    pub created_at: i64,
    /// A step belongs to a task (or is the first step of an idea).
    #[serde(default)]
    pub parent_id: Option<i64>,
    /// "reminder" (rings at the time), "deadline" (the rhythm) or "off".
    #[serde(default = "alarm_default")]
    pub alarm: String,
    /// A deadline's rhythm: {"preset": "normal"} or {"preset": "custom", "start": 60, …}.
    #[serde(default)]
    pub rhythm: Option<serde_json::Value>,
    /// A web page kept with the line: {"url", "title", "site", "image"}.
    #[serde(default)]
    pub link: Option<serde_json::Value>,
    /// A file on this computer kept with the line (full path).
    #[serde(default)]
    pub file: Option<String>,
    /// Repeats: {"freq": "week", "interval": 1, "days": [1], "ends": {"kind": "never"}, "done": 0}.
    #[serde(default)]
    pub repeat: Option<serde_json::Value>,

    // Written by the reminder clock only (the page sends them back only to bring back a deleted
    // line, with Undo).
    /// "" (quiet), "early" (before the time), "ringing", "check" (is it really done?),
    /// "on_it" (I'm on it), "snoozed", "passed" (a step whose time came while its task was done:
    /// quiet until its time changes).
    #[serde(default)]
    pub ring_state: String,
    /// Rings since the last answer (shown as "3rd reminder").
    #[serde(default)]
    pub rings: i64,
    #[serde(default)]
    pub last_ring_at: Option<i64>,
    #[serde(default)]
    pub snooze_until: Option<i64>,
    #[serde(default)]
    pub on_it_at: Option<i64>,
    #[serde(default)]
    pub check_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Picture {
    pub id: i64,
    pub note_id: i64,
    /// Set when the picture belongs to one line (dumped into the Inbox with it).
    pub item_id: Option<i64>,
    pub path: String,
    pub sort: f64,
    pub created_at: i64,
    /// Its board-size copy (thumbs.rs), when it has one: found when the notes are read, never
    /// stored with them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<crate::thumbs::Thumb>,
}

/// One time a repeating task was done (its history: the History card, the look back). Kept by
/// the line's id, so it follows the line wherever it goes (another note, a step, a new note made
/// of it). A deleted line's history stays until the next start, so Undo finds it again
/// (`add_tables`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Done {
    pub id: i64,
    pub item_id: i64,
    /// The occurrence that was done (None: a repeat without a time).
    pub due_at: Option<i64>,
    pub done_at: i64,
}

#[derive(Serialize)]
pub struct Everything {
    pub notes: Vec<Note>,
    pub items: Vec<Item>,
    pub pictures: Vec<Picture>,
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub done_log: Vec<Done>,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn add_column(conn: &Connection, table: &str, col: &str, decl: &str) -> Result<()> {
    let exists: bool = conn
        .prepare(&format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1"))
        .and_then(|mut s| s.exists([col]))
        .map_err(err)?;
    if !exists {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {col} {decl}")).map_err(err)?;
    }
    Ok(())
}

/// A connection that never runs what a database file brings with it beyond Dobo's own tables: no
/// SQL functions with side effects from its schema (`trusted_schema = OFF`), and none of the
/// tricks that can damage a file on purpose (SQLite's defensive mode). Dobo's own notes, and any
/// backup opened to be restored (backup.rs: it may come from someone else).
pub fn harden(conn: &Connection) -> Result<()> {
    conn.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true).map_err(err)?;
    conn.execute_batch("PRAGMA trusted_schema = OFF").map_err(err)
}

/// Is this file on another computer (a `\\server\share` path, or a drive letter for one: a
/// redirected AppData, a portable Dobo on a network share)?
fn on_network(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\UNC\") || (s.starts_with(r"\\") && !s.starts_with(r"\\?\") && !s.starts_with(r"\\.\")) {
        return true;
    }
    #[cfg(windows)]
    {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: a NUL-terminated path that lives through the call.
        unsafe { windows::Win32::UI::Shell::PathIsNetworkPathW(windows::core::PCWSTR(wide.as_ptr())).as_bool() }
    }
    #[cfg(not(windows))]
    false
}

/// The journal the notes use: WAL (readers and the writer never wait for each other), except on
/// another computer's disk, where SQLite can't keep WAL's shared index safely: the classic
/// journal there.
fn journal_mode(path: &Path) -> &'static str {
    if path != Path::new(":memory:") && on_network(path) { "DELETE" } else { "WAL" }
}

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(err)?;
    harden(&conn)?;
    conn.execute_batch(&format!("PRAGMA journal_mode = {};", journal_mode(path))).map_err(err)?;
    // (AUTOINCREMENT: an id is never given again once its note, line or picture is gone, so an
    // undo that puts one back can't land on something made meanwhile: `ids_never_reused`.)
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 3000;
         CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            space TEXT NOT NULL DEFAULT 'work',
            palette TEXT NOT NULL DEFAULT 'soft',
            color TEXT NOT NULL DEFAULT 'Paper',
            sort REAL NOT NULL DEFAULT 0,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
            kind TEXT NOT NULL DEFAULT 'task',
            text TEXT NOT NULL,
            due_at INTEGER,
            done_at INTEGER,
            sort REAL NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS items_note ON items(note_id);
         CREATE TABLE IF NOT EXISTS pictures (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            sort REAL NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )
    .map_err(err)?;
    // Before the one-time rebuild of an older file (`ids_never_reused`), a copy of it as it was,
    // next to it (once). The rebuild is all or nothing anyway; this is in case of the unforeseen.
    if path != Path::new(":memory:")
        && !tables_reusing_ids(&conn)?.is_empty()
        && let Some(dir) = path.parent()
    {
        let copy = dir.join("ztodo-before-unique-ids.sqlite");
        if !copy.exists()
            && let Err(e) = conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().to_string()])
        {
            eprintln!("No copy of the notes before giving them unique ids: {e}");
        }
    }
    migrate(&conn)?;
    Ok(conn)
}

/// The tables whose ids are pointed at (by lines, pictures, the board's saved columns, undo):
/// an id must never be given twice.
const ID_TABLES: [&str; 3] = ["notes", "items", "pictures"];

fn table_sql(conn: &Connection, table: &str) -> Result<Option<String>> {
    conn.query_row("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1", [table], |r| r.get(0)).optional().map_err(err)
}

/// Tables made before ids were kept unique: SQLite then gives a new row the highest id + 1, the
/// id of the last one deleted when it was the highest.
fn tables_reusing_ids(conn: &Connection) -> Result<Vec<&'static str>> {
    let mut out = vec![];
    for t in ID_TABLES {
        if table_sql(conn, t)?.is_some_and(|sql| !sql.to_ascii_uppercase().contains("AUTOINCREMENT")) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Rows whose note (or line) isn't there: counted before and after the rebuild, which must never
/// add one.
fn broken_links(conn: &Connection) -> Result<usize> {
    let mut st = conn.prepare("PRAGMA foreign_key_check").map_err(err)?;
    let n = st.query_map([], |_| Ok(())).map_err(err)?.count();
    Ok(n)
}

/// A file made before 1.0's unique ids (every file until now): its notes, lines and pictures
/// tables are made again with AUTOINCREMENT, rows, ids, columns, indexes and links exactly as
/// they were, all or nothing. Otherwise an undo putting back a deleted note (or line, or picture)
/// that had the highest id would land on the one made meanwhile with the same id, and replace it.
pub fn ids_never_reused(conn: &Connection) -> Result<()> {
    let todo = tables_reusing_ids(conn)?;
    if todo.is_empty() {
        return Ok(());
    }
    if !conn.is_autocommit() {
        return Err("not inside a transaction".into());
    }
    // Links off while the tables are swapped: dropping the old notes table would otherwise take
    // every line and picture with it (ON DELETE CASCADE). (Not settable inside a transaction:
    // checked, never assumed.)
    conn.execute_batch("PRAGMA foreign_keys = OFF").map_err(err)?;
    let off = conn.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0)).map_err(err)? == 0;
    let done = if off { rebuild_with_unique_ids(conn, &todo) } else { Err("the links between tables couldn’t be paused".into()) };
    let on = conn.execute_batch("PRAGMA foreign_keys = ON").map_err(err);
    done.and(on)
}

fn rebuild_with_unique_ids(conn: &Connection, todo: &[&str]) -> Result<()> {
    // (Dropped without a commit, everything is rolled back.)
    let tx = conn.unchecked_transaction().map_err(err)?;
    let broken = broken_links(conn)?;
    let shape = |t: &str| -> Result<(i64, i64, i64)> {
        conn.query_row(&format!("SELECT COUNT(*), COALESCE(SUM(id), 0), COALESCE(MAX(id), 0) FROM {t}"), [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(err)
    };
    for &t in todo {
        let sql = table_sql(conn, t)?.ok_or_else(|| format!("no {t} table"))?;
        // The table as it is (every column added since, in its order), with AUTOINCREMENT.
        let head = format!("CREATE TABLE {t} (");
        let key = "id INTEGER PRIMARY KEY,";
        if !sql.starts_with(&head) || sql.matches(key).count() != 1 {
            return Err(format!("the {t} table isn’t as expected: {sql}"));
        }
        let new = format!("{t}_unique_ids");
        let ddl = sql.replacen(&head, &format!("CREATE TABLE {new} ("), 1).replacen(key, "id INTEGER PRIMARY KEY AUTOINCREMENT,", 1);
        // Its indexes go with the old table: made again on the new one.
        let extras: Vec<String> = conn
            .prepare("SELECT sql FROM sqlite_master WHERE tbl_name = ?1 AND type IN ('index', 'trigger') AND sql IS NOT NULL")
            .and_then(|mut st| st.query_map([t], |r| r.get(0))?.collect())
            .map_err(err)?;
        let before = shape(t)?;
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {new}; {ddl};")).map_err(err)?;
        conn.execute(&format!("INSERT INTO {new} SELECT * FROM {t}"), []).map_err(err)?;
        conn.execute_batch(&format!("DROP TABLE {t}; ALTER TABLE {new} RENAME TO {t};")).map_err(err)?;
        for s in extras {
            conn.execute_batch(&s).map_err(err)?;
        }
        if shape(t)? != before {
            return Err(format!("the {t} table didn’t come through whole"));
        }
    }
    if table_sql(conn, "notes")?.is_none() || broken_links(conn)? > broken {
        return Err("lines or pictures lost their note".into());
    }
    tx.commit().map_err(err)
}

/// Adds the columns newer versions use (to files made by older ones, or restored backups), and
/// gives an older file unique ids.
pub fn migrate(conn: &Connection) -> Result<()> {
    add_columns(conn)?;
    add_tables(conn)?;
    // A repeating task unticked by 1.2.0 (started after 1.2.1, then 1.2.1 again) is open with
    // what unticking gives back still on it: open, it waits for nothing. (Never in the way of
    // opening the notes either: `not_stale` copes with one left.)
    if let Err(e) = conn.execute(
        "UPDATE items SET repeat = json_remove(repeat, '$.ticked') WHERE done_at IS NULL AND repeat LIKE '%\"ticked\"%' AND json_valid(repeat)",
        [],
    ) {
        eprintln!("Repeating tasks open again couldn't be tidied: {e}");
    }
    // (Never in the way of opening the notes: they work as before if this can't be done now, and
    // it is tried again at the next start.)
    if let Err(e) = ids_never_reused(conn) {
        eprintln!("The notes couldn't be given unique ids yet: {e}");
    }
    Ok(())
}

fn add_columns(conn: &Connection) -> Result<()> {
    // Milestone 2 columns (added to files made by milestone 1).
    add_column(conn, "items", "parent_id", "INTEGER")?;
    add_column(conn, "items", "alarm", "TEXT NOT NULL DEFAULT 'reminder'")?;
    add_column(conn, "items", "rhythm", "TEXT")?;
    add_column(conn, "items", "link", "TEXT")?;
    add_column(conn, "items", "file", "TEXT")?;
    add_column(conn, "items", "ring_state", "TEXT NOT NULL DEFAULT ''")?;
    add_column(conn, "items", "rings", "INTEGER NOT NULL DEFAULT 0")?;
    add_column(conn, "items", "last_ring_at", "INTEGER")?;
    add_column(conn, "items", "snooze_until", "INTEGER")?;
    add_column(conn, "items", "on_it_at", "INTEGER")?;
    add_column(conn, "items", "check_at", "INTEGER")?;
    add_column(conn, "pictures", "item_id", "INTEGER")?;
    // Milestone 3 columns.
    add_column(conn, "notes", "wide", "INTEGER NOT NULL DEFAULT 0")?;
    add_column(conn, "notes", "folded", "INTEGER NOT NULL DEFAULT 0")?;
    add_column(conn, "notes", "paper", "TEXT NOT NULL DEFAULT 'plain'")?;
    add_column(conn, "items", "repeat", "TEXT")?;
    // The Archive page.
    add_column(conn, "notes", "archived_at", "INTEGER")?;
    // Done tasks moved to the end of their note (1.2.1; before, they always were).
    add_column(conn, "notes", "done_last", "INTEGER NOT NULL DEFAULT 0")?;
    Ok(())
}

/// The tables newer versions use, made in files from older ones (or restored backups).
fn add_tables(conn: &Connection) -> Result<()> {
    // Repeating tasks' history (1.2.1). No link to the line (ON DELETE CASCADE): a line deleted
    // and brought back with Undo finds its history where it was (ids are never given again), and
    // what no line has any more is cleared here, at the next start, when no Undo can need it.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS done_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id INTEGER NOT NULL,
            due_at INTEGER,
            done_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS done_log_item ON done_log(item_id);
         DELETE FROM done_log WHERE item_id NOT IN (SELECT id FROM items);",
    )
    .map_err(err)
}

fn note_from_row(r: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        title: r.get(1)?,
        space: r.get(2)?,
        palette: r.get(3)?,
        color: r.get(4)?,
        sort: r.get(5)?,
        archived: r.get::<_, i64>(6)? != 0,
        created_at: r.get(7)?,
        wide: r.get::<_, i64>(8)? != 0,
        folded: r.get::<_, i64>(9)? != 0,
        paper: r.get(10)?,
        archived_at: r.get(11)?,
        done_last: r.get::<_, i64>(12)? != 0,
    })
}

fn json_col(v: Option<String>) -> Option<serde_json::Value> {
    v.and_then(|s| serde_json::from_str(&s).ok())
}

fn item_from_row(r: &rusqlite::Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        note_id: r.get(1)?,
        kind: r.get(2)?,
        text: r.get(3)?,
        due_at: r.get(4)?,
        done_at: r.get(5)?,
        sort: r.get(6)?,
        created_at: r.get(7)?,
        parent_id: r.get(8)?,
        alarm: r.get(9)?,
        rhythm: json_col(r.get(10)?),
        link: json_col(r.get(11)?),
        file: r.get(12)?,
        ring_state: r.get(13)?,
        rings: r.get(14)?,
        last_ring_at: r.get(15)?,
        snooze_until: r.get(16)?,
        on_it_at: r.get(17)?,
        check_at: r.get(18)?,
        repeat: json_col(r.get(19)?),
    })
}

fn done_from_row(r: &rusqlite::Row) -> rusqlite::Result<Done> {
    Ok(Done { id: r.get(0)?, item_id: r.get(1)?, due_at: r.get(2)?, done_at: r.get(3)? })
}

fn picture_from_row(r: &rusqlite::Row) -> rusqlite::Result<Picture> {
    Ok(Picture { id: r.get(0)?, note_id: r.get(1)?, item_id: r.get(2)?, path: r.get(3)?, sort: r.get(4)?, created_at: r.get(5)?, thumb: None })
}

const NOTE_COLS: &str = "id, title, space, palette, color, sort, archived, created_at, wide, folded, paper, archived_at, done_last";
pub const ITEM_COLS: &str = "id, note_id, kind, text, due_at, done_at, sort, created_at, parent_id, alarm, rhythm, link, file, \
     ring_state, rings, last_ring_at, snooze_until, on_it_at, check_at, repeat";
const PICTURE_COLS: &str = "id, note_id, item_id, path, sort, created_at";

pub fn load_all(conn: &Connection) -> Result<Everything> {
    let mut st = conn.prepare(&format!("SELECT {NOTE_COLS} FROM notes ORDER BY sort, id")).map_err(err)?;
    let notes = st.query_map([], note_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    let mut st = conn.prepare(&format!("SELECT {ITEM_COLS} FROM items ORDER BY sort, id")).map_err(err)?;
    let items = st.query_map([], item_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    let mut st = conn.prepare(&format!("SELECT {PICTURE_COLS} FROM pictures ORDER BY sort, id")).map_err(err)?;
    let pictures = st.query_map([], picture_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    let mut settings = serde_json::Map::new();
    let mut st = conn.prepare("SELECT key, value FROM settings").map_err(err)?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(err)?;
    for row in rows {
        let (k, v) = row.map_err(err)?;
        settings.insert(k, serde_json::from_str(&v).unwrap_or(serde_json::Value::Null));
    }
    let mut st = conn.prepare("SELECT id, item_id, due_at, done_at FROM done_log ORDER BY done_at, id").map_err(err)?;
    let done_log = st.query_map([], done_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    Ok(Everything { notes, items, pictures, settings, done_log })
}

/// One time a line was done leaves its history (Undo of a "Done").
pub fn forget_done(conn: &Connection, item_id: i64, done_at: i64) -> Result<()> {
    conn.execute("DELETE FROM done_log WHERE item_id = ?1 AND done_at = ?2", params![item_id, done_at]).map_err(err)?;
    Ok(())
}

/// A line's history (oldest first).
pub fn done_log_of(conn: &Connection, item_id: i64) -> Result<Vec<Done>> {
    let mut st = conn.prepare("SELECT id, item_id, due_at, done_at FROM done_log WHERE item_id = ?1 ORDER BY done_at, id").map_err(err)?;
    st.query_map([item_id], done_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)
}

pub fn get_note(conn: &Connection, id: i64) -> Result<Note> {
    conn.query_row(&format!("SELECT {NOTE_COLS} FROM notes WHERE id = ?1"), [id], note_from_row).map_err(err)
}

pub fn get_item(conn: &Connection, id: i64) -> Result<Item> {
    conn.query_row(&format!("SELECT {ITEM_COLS} FROM items WHERE id = ?1"), [id], item_from_row).map_err(err)
}

/// Items that may ring: open timed tasks whose reminder is on, in notes that aren't archived
/// (an archived note is put away: silent until it comes back, then it rings once if overdue),
/// and not steps of a task that is done (paused with it, until it is unticked).
pub fn timed_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut st = conn
        .prepare(&format!(
            "{ANCESTORS} SELECT {ITEM_COLS} FROM items WHERE kind = 'task' AND due_at IS NOT NULL AND done_at IS NULL AND alarm != 'off'
               AND note_id NOT IN (SELECT id FROM notes WHERE archived != 0)
               AND id NOT IN (SELECT up.id FROM up JOIN items a ON a.id = up.anc WHERE a.done_at IS NOT NULL)"
        ))
        .map_err(err)?;
    st.query_map([], item_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)
}

/// Every line of a note (to clear their notifications when the note goes away).
pub fn item_ids_of_note(conn: &Connection, note_id: i64) -> Vec<i64> {
    let Ok(mut st) = conn.prepare("SELECT id FROM items WHERE note_id = ?1") else { return vec![] };
    st.query_map([note_id], |r| r.get(0)).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
}

/// A line and everything under it.
pub fn subtree_ids(conn: &Connection, id: i64) -> Vec<i64> {
    let Ok(mut st) = conn.prepare(&format!("{SUBTREE} SELECT id FROM sub")) else { return vec![] };
    st.query_map([id], |r| r.get(0)).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
}

fn next_sort(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> Result<f64> {
    let max: Option<f64> = conn.query_row(sql, p, |r| r.get(0)).optional().map_err(err)?.flatten();
    Ok(max.map(|m| m + 1.0).unwrap_or(0.0))
}

pub fn create_note(conn: &Connection, title: &str, space: &str, palette: &str, color: &str) -> Result<Note> {
    let sort = next_sort(conn, "SELECT MAX(sort) FROM notes", [])?;
    conn.execute(
        "INSERT INTO notes (title, space, palette, color, sort, archived, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        params![title, space, palette, color, sort, now_ms()],
    )
    .map_err(err)?;
    get_note(conn, conn.last_insert_rowid())
}

/// Saves what the person can change. The archive time follows `archived`: set when the note
/// goes into the archive (now, or the time it had, when an undo puts it back in), kept while
/// it stays there, cleared when it comes out.
pub fn update_note(conn: &Connection, n: &Note) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET title = ?2, space = ?3, palette = ?4, color = ?5, sort = ?6, archived = ?7, wide = ?8, folded = ?9, paper = ?10, done_last = ?13,
            archived_at = CASE WHEN ?7 = 0 THEN NULL
                               WHEN archived = 0 THEN COALESCE(?12, ?11)
                               ELSE COALESCE(archived_at, ?12, ?11) END
         WHERE id = ?1",
        params![n.id, n.title, n.space, n.palette, n.color, n.sort, n.archived as i64, n.wide as i64, n.folded as i64, n.paper, now_ms(), n.archived_at, n.done_last as i64],
    )
    .map_err(err)?;
    get_note(conn, n.id)
}

pub fn delete_note(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM notes WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}

/// Puts the notes in this order (the board places them in its columns in this order).
pub fn reorder_notes(conn: &mut Connection, ids: &[i64]) -> Result<()> {
    let tx = conn.transaction().map_err(err)?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute("UPDATE notes SET sort = ?2 WHERE id = ?1", params![id, i as f64]).map_err(err)?;
    }
    tx.commit().map_err(err)
}

pub struct NewItem<'a> {
    pub note_id: i64,
    pub kind: &'a str,
    pub text: &'a str,
    pub due_at: Option<i64>,
    pub parent_id: Option<i64>,
    pub alarm: &'a str,
    pub link: Option<&'a serde_json::Value>,
    pub file: Option<&'a str>,
}

pub fn create_item(conn: &Connection, n: NewItem) -> Result<Item> {
    let sort = next_sort(conn, "SELECT MAX(sort) FROM items WHERE note_id = ?1", [n.note_id])?;
    conn.execute(
        "INSERT INTO items (note_id, kind, text, due_at, done_at, sort, created_at, parent_id, alarm, link, file)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            n.note_id,
            n.kind,
            n.text,
            n.due_at,
            sort,
            now_ms(),
            n.parent_id,
            n.alarm,
            n.link.map(|v| v.to_string()),
            n.file
        ],
    )
    .map_err(err)?;
    let id = conn.last_insert_rowid();
    // (Never someone else's history: only possible in a file whose ids couldn't be made unique.)
    conn.execute("DELETE FROM done_log WHERE item_id = ?1", [id]).map_err(err)?;
    get_item(conn, id)
}

/// Saves what the person can change. When the time, the reminder type, the rhythm or "done"
/// changes, the reminder starts again from scratch.
#[cfg(test)]
pub fn update_item(conn: &Connection, it: &Item) -> Result<Item> {
    update_item_full(conn, it).map(|(i, _, _)| i)
}

/// Every line under a task, with each of the lines above it (`up(id, anc)`), for the steps of a
/// task that is done. UNION also stops on a loop.
const ANCESTORS: &str = "WITH RECURSIVE up(id, anc) AS (SELECT id, parent_id FROM items WHERE parent_id IS NOT NULL
    UNION SELECT up.id, i.parent_id FROM up JOIN items i ON i.id = up.anc WHERE i.parent_id IS NOT NULL)";

/// What saving a line changed: the line, a repeating task's history when it changed (all of
/// it, for the board), and the steps under it whose reminder was paused or brought back (their
/// notifications to clear, the board to tell).
pub type Saved = (Item, Option<Vec<Done>>, Vec<Item>);

/// Like `update_item`; a repeating task ticked or unticked also changes its history (returned
/// second) and its time (`finish_tick`, `after_unticked`).
pub fn update_item_full(conn: &Connection, it: &Item) -> Result<Saved> {
    update_item_at(conn, it, now_ms())
}

/// The reminder-clock state a "Not done" sets, at `now`:
/// - the task itself, overdue (or with an early ring already past), waits a minute before it
///   rings, never at once as it is unticked;
/// - its steps were paused while it was done: nothing that came meanwhile rings now. A step whose
///   time came while the task was done stays quiet ("passed"); the others go on with their next
///   ring to come.
pub const UNDONE_WAIT: i64 = 60_000;

fn after_undone(conn: &Connection, task: &Item, done_at: i64, now: i64) -> Result<Vec<i64>> {
    wait_a_minute(conn, task, now)?;
    let mut changed = Vec::new();
    for id in subtree_ids(conn, task.id).into_iter().filter(|i| *i != task.id) {
        if unpause_step(conn, &get_item(conn, id)?, done_at, now)? {
            changed.push(id);
        }
    }
    Ok(changed)
}

/// A step paused while its task was done (from `done_at`), going on at `now` (`after_undone`):
/// its time come meanwhile, it stays quiet ("passed"); else it goes on with its next ring to
/// come. Whether it changed.
fn unpause_step(conn: &Connection, s: &Item, done_at: i64, now: i64) -> Result<bool> {
    let Some(due) = s.due_at else { return Ok(false) };
    if s.done_at.is_some() || s.kind != "task" || s.alarm == "off" {
        return Ok(false);
    }
    if due > done_at && due <= now {
        conn.execute("UPDATE items SET ring_state = 'passed' WHERE id = ?1", [s.id]).map_err(err)?;
    } else {
        conn.execute("UPDATE items SET last_ring_at = MAX(COALESCE(last_ring_at, ?2), ?2) WHERE id = ?1", params![s.id, now]).map_err(err)?;
    }
    Ok(true)
}

/// The task's own wait (`after_undone`): overdue, or with an early ring already past, it rings a
/// minute from `now`, never at once.
fn wait_a_minute(conn: &Connection, task: &Item, now: i64) -> Result<()> {
    let r = crate::reminders::rhythm_of(conn, task);
    if crate::reminders::slots(task, &r).first().is_some_and(|(t, _)| *t <= now) {
        conn.execute("UPDATE items SET snooze_until = ?2 WHERE id = ?1", params![task.id, now + UNDONE_WAIT]).map_err(err)?;
    }
    Ok(())
}

/// A task just ticked: its steps' rings stop showing (their reminders are kept, paused).
pub fn after_done(conn: &Connection, task: &Item) -> Result<Vec<i64>> {
    let mut changed = Vec::new();
    for id in subtree_ids(conn, task.id).into_iter().filter(|i| *i != task.id) {
        let n = conn
            .execute("UPDATE items SET ring_state = '' WHERE id = ?1 AND done_at IS NULL AND ring_state IN ('early', 'ringing', 'check')", [id])
            .map_err(err)?;
        if n > 0 {
            changed.push(id);
        }
    }
    Ok(changed)
}

/// A repeat's own time of day (`Repeat::at`): the time it is given, kept while the time stays the
/// same (an occurrence moved by a clock change keeps the repeat's own), set from the time
/// otherwise.
fn repeat_with_time(it: &Item, old: &Item) -> Option<serde_json::Value> {
    let mut rule = it.repeat.clone()?;
    let (Some(obj), Some(due)) = (rule.as_object_mut(), it.due_at) else { return Some(rule) };
    let kept = if old.due_at == it.due_at {
        obj.get("at").or_else(|| old.repeat.as_ref().and_then(|r| r.get("at"))).and_then(|v| v.as_str()).filter(|s| crate::repeat::parse_hhmm(s).is_some()).map(String::from)
    } else {
        None
    };
    obj.insert("at".into(), serde_json::json!(kept.unwrap_or_else(|| crate::clock::local_hhmm(due))));
    Some(rule)
}

/// `update_item_full` at a given time.
pub fn update_item_at(conn: &Connection, it: &Item, now: i64) -> Result<Saved> {
    let old = get_item(conn, it.id)?;
    // (A stale copy's history may be stale too: the board is sent the line's.)
    let (it, stale) = not_stale(it, &old);
    let it = &it;
    let it = &Item { repeat: repeat_with_time(it, &old), ..it.clone() };
    let it = &still_ticked(it, &old, now);
    let restart = old.due_at != it.due_at || old.done_at != it.done_at || old.alarm != it.alarm || old.rhythm != it.rhythm || old.kind != it.kind;
    // All or nothing (the line, its steps' note, a repeat's history and time).
    let tx = conn.unchecked_transaction().map_err(err)?;
    conn.execute(
        "UPDATE items SET note_id = ?2, kind = ?3, text = ?4, due_at = ?5, done_at = ?6, sort = ?7, parent_id = ?8,
            alarm = ?9, rhythm = ?10, link = ?11, file = ?12, repeat = ?13 WHERE id = ?1",
        params![
            it.id,
            it.note_id,
            it.kind,
            it.text,
            it.due_at,
            it.done_at,
            it.sort,
            it.parent_id,
            it.alarm,
            it.rhythm.as_ref().map(|v| v.to_string()),
            it.link.as_ref().map(|v| v.to_string()),
            it.file,
            it.repeat.as_ref().map(|v| v.to_string())
        ],
    )
    .map_err(err)?;
    if restart {
        reset_ring(conn, it.id)?;
    }
    // Steps (and steps of steps) follow their task into another note, with their pictures.
    if old.note_id != it.note_id {
        conn.execute(&format!("{SUBTREE} UPDATE items SET note_id = ?2 WHERE id IN (SELECT id FROM sub) AND id != ?1"), params![it.id, it.note_id])
            .map_err(err)?;
        conn.execute(&format!("{SUBTREE} UPDATE pictures SET note_id = ?2 WHERE item_id IN (SELECT id FROM sub)"), params![it.id, it.note_id])
            .map_err(err)?;
    }
    let (log, fresh) = match (old.done_at, it.done_at) {
        (None, Some(_)) => finish_tick(conn, it.id, now)?,
        (Some(_), None) => (after_unticked(conn, &old, it.id)?, None),
        _ => (if stale { Some(done_log_of(conn, it.id)?) } else { None }, None),
    };
    let saved = get_item(conn, it.id)?;
    // (Ticked and open again at once: its steps were never paused, they start fresh.)
    let steps = match (old.done_at, saved.done_at, fresh) {
        (_, _, Some(fresh)) => fresh,
        (None, Some(_), _) => after_done(conn, &saved)?,
        (Some(was), None, _) => after_undone(conn, &saved, was, now)?,
        _ => vec![],
    };
    let saved = get_item(conn, it.id)?;
    let steps = steps.into_iter().map(|id| get_item(conn, id)).collect::<Result<Vec<_>>>()?;
    tx.commit().map_err(err)?;
    Ok((saved, log, steps))
}

/// A line and everything under it (its steps, their steps…), as the table `sub(id)`.
/// `?1` is the line. UNION (not UNION ALL) also stops on a loop.
const SUBTREE: &str = "WITH RECURSIVE sub(id) AS (SELECT ?1 UNION SELECT i.id FROM items i JOIN sub ON i.parent_id = sub.id)";

/// A repeating task's rule, when it is one (a task with a rule that reads).
fn rule_of(it: &Item) -> Option<crate::repeat::Repeat> {
    if it.kind != "task" {
        return None;
    }
    serde_json::from_value(it.repeat.clone()?).ok()
}

fn rule_json(r: &crate::repeat::Repeat) -> Result<String> {
    serde_json::to_string(r).map_err(err)
}

/// "My day", its start and its end (Settings › My day; 08:00–22:00 when not set).
pub fn my_day(conn: &Connection) -> (String, String) {
    let day = get_setting(conn, "day");
    let time = |k: &str, or: &str| {
        day.as_ref().and_then(|d| d.get(k)).and_then(|v| v.as_str()).filter(|s| crate::repeat::parse_hhmm(s).is_some()).map_or_else(|| or.to_string(), String::from)
    };
    (time("start", "08:00"), time("end", "22:00"))
}

/// When a ticked repeating task waiting for its next day is a to-do again (`reopens_at`), or
/// None when it isn't one.
fn reopen_time(it: &Item, day: &(String, String)) -> Option<i64> {
    rule_of(it)?.ticked?;
    it.done_at?;
    Some(crate::repeat::reopens_at(it.due_at?, &day.0, &day.1))
}

fn has_ticked(it: &Item) -> bool {
    it.repeat.as_ref().and_then(|r| r.get("ticked")).is_some()
}

/// A copy of a repeating task the board kept from another moment never ticks it a second time,
/// nor unticks a tick it never saw. Its other changes (its words, its note…) are kept; its
/// "done", its time and its rule stay as the line has them:
/// - a copy from before it opened again (still ticked, with what unticking gives back), saved on
///   the open line: a real tick always comes from the open line, without `ticked`;
/// - a copy from before it was ticked (open, or ticked on the board a moment before the tick came
///   back moved on: at the occurrence ticked, without `ticked`), saved while it waits ticked for
///   its next day.
///
/// An open line never takes a `ticked`, and one left on an open line (a 1.2.0 started meanwhile
/// unticked it) is dropped: the copy is the line as it is. Returns the line to save, and whether
/// the copy was stale (the board is then sent the line's history).
fn not_stale(it: &Item, old: &Item) -> (Item, bool) {
    let kept = || Item { done_at: old.done_at, due_at: old.due_at, repeat: old.repeat.clone(), ..it.clone() };
    if old.done_at.is_none() {
        if !has_ticked(it) {
            return (it.clone(), false);
        }
        if !has_ticked(old) {
            return (kept(), true);
        }
        let mut repeat = it.repeat.clone();
        if let Some(obj) = repeat.as_mut().and_then(|r| r.as_object_mut()) {
            obj.remove("ticked");
        }
        return (Item { repeat, ..it.clone() }, false);
    }
    let Some(back) = rule_of(old).and_then(|r| r.ticked) else { return (it.clone(), false) };
    let before_tick = rule_of(it).is_some() && !has_ticked(it) && it.due_at == Some(back.due) && old.due_at != Some(back.due);
    if before_tick { (kept(), true) } else { (it.clone(), false) }
}

/// What ticking a line did (`finish_tick`): a repeating task's history, and, when it is open
/// again at once, its steps made fresh.
pub type Ticked = (Option<Vec<Done>>, Option<Vec<i64>>);

/// The line, just ticked: a repeating task's completion goes into its history, and while the
/// repeat goes on the line moves to its next occurrence, still ticked until that day comes
/// (`reopen_due`). When that day has come already (yesterday's ticked this morning), it is open
/// again at once, at that next occurrence. Returns its history (a repeating task's) and, when it
/// is open again, its steps made fresh for it (`open_row`).
pub fn finish_tick(conn: &Connection, id: i64, now: i64) -> Result<Ticked> {
    let it = get_item(conn, id)?;
    let (Some(done_at), Some(mut rule)) = (it.done_at, rule_of(&it)) else { return Ok((None, None)) };
    conn.execute("INSERT INTO done_log (item_id, due_at, done_at) VALUES (?1, ?2, ?3)", params![id, it.due_at, done_at]).map_err(err)?;
    let log = Some(done_log_of(conn, id)?);
    // (A repeat without a time: its history only; it stays done.)
    let Some(due) = it.due_at else { return Ok((log, None)) };
    // (A repeat made before it kept its own time: the time of this one.)
    if rule.at.as_deref().and_then(crate::repeat::parse_hhmm).is_none() {
        rule.at = Some(crate::clock::local_hhmm(due));
    }
    rule.ticked = None;
    // The repeat is over: the line stays done, as any task.
    let Some((next, rule)) = rule.tick(due, now) else { return Ok((log, None)) };
    conn.execute("UPDATE items SET due_at = ?2, repeat = ?3 WHERE id = ?1", params![id, next, rule_json(&rule)?]).map_err(err)?;
    let moved = get_item(conn, id)?;
    match reopen_time(&moved, &my_day(conn)) {
        Some(at) if now >= at => Ok((log, Some(open_row(conn, &moved, at, now)?))),
        _ => Ok((log, None)),
    }
}

/// How late a ticked repeating task may open again and still be on time (the reminder clock looks
/// every 5 seconds).
const ON_TIME: i64 = 60_000;

/// A ticked repeating task is a to-do again, at the occurrence it waited for (`it`, as it waited;
/// `at`, when it was to open): its reminder ready for that occurrence, and its steps fresh for it
/// too (`fresh_steps`). Opened on time, it rings at its own times (a 07:00 task opening at 07:00
/// rings at 07:00); opened late (the computer asleep, Dobo not running) with its time or an early
/// ring past meanwhile, it waits a minute, as after "Not done", and so do its steps. Returns the
/// steps changed.
fn open_row(conn: &Connection, it: &Item, at: i64, now: i64) -> Result<Vec<i64>> {
    let rule = rule_of(it);
    // (How far it moved on, in calendar days: from the occurrence ticked to the one it opens at.)
    let days = match (rule.as_ref().and_then(|r| r.ticked.as_ref()), it.due_at) {
        (Some(t), Some(due)) => crate::repeat::days_apart(t.due, due).unwrap_or(0),
        _ => 0,
    };
    let json = rule.map(|r| crate::repeat::Repeat { ticked: None, ..r }).as_ref().map(rule_json).transpose()?;
    conn.execute("UPDATE items SET done_at = NULL, repeat = COALESCE(?2, repeat) WHERE id = ?1", params![it.id, json]).map_err(err)?;
    reset_ring(conn, it.id)?;
    let late = now > at.saturating_add(ON_TIME);
    if late {
        wait_a_minute(conn, &get_item(conn, it.id)?, now)?;
    }
    fresh_steps(conn, it, days, late, now)
}

/// A repeating task open again (`task`, as it waited, ticked): its steps start fresh too,
/// unticked, their times moved on by as many calendar days as the task's (`days`), each at the
/// same time of day (never a fixed number of hours: the days the clocks change are 23 or 25
/// hours long; `repeat::shift_days`), their reminders ready for
/// them. Each one that was done keeps that time in the history (the look back counts it on its
/// day). Opened `late`, a step whose time (or early ring) is past already waits a minute, as the
/// task does, never ringing at once. A step with a repeat of its own keeps its own rhythm: it was
/// paused while the task waited, and goes on as after "Not done" (`unpause_step`). Returns the
/// steps changed.
fn fresh_steps(conn: &Connection, task: &Item, days: i64, late: bool, now: i64) -> Result<Vec<i64>> {
    let mut changed = Vec::new();
    for id in subtree_ids(conn, task.id).into_iter().filter(|i| *i != task.id) {
        let s = get_item(conn, id)?;
        if s.kind != "task" {
            continue;
        }
        if rule_of(&s).is_some() {
            if let Some(since) = task.done_at
                && unpause_step(conn, &s, since, now)?
            {
                changed.push(id);
            }
            continue;
        }
        if s.done_at.is_none() && s.due_at.is_none() && s.ring_state.is_empty() && s.snooze_until.is_none() && s.check_at.is_none() && s.on_it_at.is_none() {
            continue;
        }
        if let Some(done) = s.done_at {
            conn.execute(
                "INSERT INTO done_log (item_id, due_at, done_at) SELECT ?1, ?2, ?3 WHERE NOT EXISTS (SELECT 1 FROM done_log WHERE item_id = ?1 AND done_at = ?3)",
                params![id, s.due_at, done],
            )
            .map_err(err)?;
        }
        let due = s.due_at.map(|d| crate::repeat::shift_days(d, days).unwrap_or(d));
        conn.execute("UPDATE items SET done_at = NULL, due_at = ?2 WHERE id = ?1", params![id, due]).map_err(err)?;
        reset_ring(conn, id)?;
        if late {
            wait_a_minute(conn, &get_item(conn, id)?, now)?;
        }
        changed.push(id);
    }
    Ok(changed)
}

/// Undo of a repeating task's "Done" once it opened again: the steps its opening made fresh
/// (`fresh_steps`), back exactly as they were (`steps`, as they were then: ticked or not, their
/// times, their reminders, the task they were under), and each one's time done out of the history
/// again. One whose time or snooze is past by `now` waits a minute, as the task does, never
/// ringing at once. Returns them as saved: one changed since is left out (deleted, made a
/// repeating task, ticked, or moved under another task: a board's copy from before can't
/// rewrite it).
#[cfg(test)]
pub fn steps_back(conn: &Connection, steps: &[Item], now: i64) -> Result<Vec<Item>> {
    let tx = conn.unchecked_transaction().map_err(err)?;
    let saved = steps_back_in(conn, steps, now)?;
    tx.commit().map_err(err)?;
    Ok(saved)
}

/// `steps_back`, inside a transaction of the caller's.
fn steps_back_in(conn: &Connection, steps: &[Item], now: i64) -> Result<Vec<Item>> {
    let mut saved = Vec::new();
    for s in steps {
        let n = conn
            .execute(
                "UPDATE items SET done_at = ?2, due_at = ?3, ring_state = ?4, rings = ?5, last_ring_at = ?6, snooze_until = ?7, on_it_at = ?8, check_at = ?9
                 WHERE id = ?1 AND kind = 'task' AND repeat IS NULL AND done_at IS NULL AND parent_id IS ?10",
                params![s.id, s.done_at, s.due_at, s.ring_state, s.rings, s.last_ring_at, s.snooze_until, s.on_it_at, s.check_at, s.parent_id],
            )
            .map_err(err)?;
        if n == 0 {
            continue;
        }
        if let Some(done) = s.done_at {
            conn.execute("DELETE FROM done_log WHERE item_id = ?1 AND done_at = ?2", params![s.id, done]).map_err(err)?;
        }
        wait_a_minute(conn, &get_item(conn, s.id)?, now)?;
        saved.push(get_item(conn, s.id)?);
    }
    Ok(saved)
}

/// The Undo of what opened a repeating task again (its "Done", once its next day came; or a change
/// that opened it at once, a time set earlier), all in one go, so the reminder clock never sees a
/// part of it: the line exactly as it was then (`it`: its "done", its time and its rule, ticked
/// or open, with whatever else that change changed), its steps back as they were (`steps_back`),
/// and the time that tick was done out of its history (`forget`; None when the tick stays: the
/// line goes back to waiting for its next day, its history as it is, never a second row). Open
/// again with its time (or an early ring) past, it waits a minute, as after "Not done", never
/// ringing at once. Only while it is open: ticked again since (a notification's Done), it stays
/// as it is. Returns the line, its history and the steps given back.
pub fn undo_tick(conn: &Connection, it: &Item, forget: Option<i64>, steps: &[Item], now: i64) -> Result<Saved> {
    let tx = conn.unchecked_transaction().map_err(err)?;
    let old = get_item(conn, it.id)?;
    if old.done_at.is_some() {
        return Ok((old, Some(done_log_of(conn, it.id)?), vec![]));
    }
    conn.execute(
        "UPDATE items SET kind = ?2, text = ?3, due_at = ?4, done_at = ?5, alarm = ?6, rhythm = ?7, link = ?8, file = ?9, repeat = ?10 WHERE id = ?1",
        params![
            it.id,
            it.kind,
            it.text,
            it.due_at,
            it.done_at,
            it.alarm,
            it.rhythm.as_ref().map(|v| v.to_string()),
            it.link.as_ref().map(|v| v.to_string()),
            it.file,
            it.repeat.as_ref().map(|v| v.to_string())
        ],
    )
    .map_err(err)?;
    reset_ring(conn, it.id)?;
    if let Some(done) = forget {
        forget_done(conn, it.id, done)?;
    }
    let mut ids: Vec<i64> = steps_back_in(conn, steps, now)?.iter().map(|s| s.id).collect();
    let saved = get_item(conn, it.id)?;
    if saved.done_at.is_none() {
        wait_a_minute(conn, &saved, now)?;
    } else {
        // (Ticked again: its steps' rings stop showing, as at a tick.)
        for id in after_done(conn, &saved)? {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let saved = get_item(conn, it.id)?;
    let steps = ids.into_iter().map(|id| get_item(conn, id)).collect::<Result<Vec<_>>>()?;
    let log = done_log_of(conn, it.id)?;
    tx.commit().map_err(err)?;
    Ok((saved, Some(log), steps))
}

/// The history of each step among `items` that has one (`fresh_steps` writes it), for the board.
pub fn steps_logs(conn: &Connection, items: &[Item]) -> Result<Vec<(i64, Vec<Done>)>> {
    let mut out = Vec::new();
    for it in items.iter().filter(|i| i.parent_id.is_some()) {
        let rows = done_log_of(conn, it.id)?;
        if !rows.is_empty() {
            out.push((it.id, rows));
        }
    }
    Ok(out)
}

/// The line unticked (`old`: as it was, ticked): its completion leaves its history, and a
/// repeating task waiting for its next day goes back to the occurrence that was ticked, with its
/// count as it was. Returns its history when it is a repeating task's.
fn after_unticked(conn: &Connection, old: &Item, id: i64) -> Result<Option<Vec<Done>>> {
    let Some(was) = old.done_at else { return Ok(None) };
    let gone = conn.execute("DELETE FROM done_log WHERE item_id = ?1 AND done_at = ?2", params![id, was]).map_err(err)?;
    let old_rule = rule_of(old);
    if let Some(back) = old_rule.as_ref().and_then(|r| r.ticked.clone()) {
        let now = get_item(conn, id)?;
        let rule = rule_of(&now).map(|r| crate::repeat::Repeat { done: back.done, ticked: None, ..r });
        let json = rule.as_ref().map(rule_json).transpose()?;
        conn.execute("UPDATE items SET due_at = ?2, repeat = COALESCE(?3, repeat) WHERE id = ?1", params![id, back.due, json]).map_err(err)?;
    }
    Ok(if gone > 0 || old_rule.is_some() { Some(done_log_of(conn, id)?) } else { None })
}

/// A ticked repeating task changed while it waits for its next day (its words, its note, its
/// rule): it keeps what unticking gives back. A new rule moves it to the new rule's next
/// occurrence after the one ticked; no rule any more, it goes back to the one ticked (done).
fn still_ticked(it: &Item, old: &Item, now: i64) -> Item {
    if old.done_at.is_none() || it.done_at != old.done_at {
        return it.clone();
    }
    let Some(was) = rule_of(old) else { return it.clone() };
    let Some(back) = was.ticked.clone() else { return it.clone() };
    let same_time = it.due_at == old.due_at;
    let Some(rule) = rule_of(it) else {
        return if same_time && it.kind == "task" { Item { due_at: Some(back.due), ..it.clone() } } else { it.clone() };
    };
    let same_rule = (&rule.freq, rule.interval, &rule.days, rule.monthday, &rule.ends) == (&was.freq, was.interval, &was.days, was.monthday, &was.ends);
    let with = |r: &crate::repeat::Repeat, due: Option<i64>| Item { due_at: due, repeat: serde_json::to_value(r).ok(), ..it.clone() };
    // (Its time changed by hand: that is its next time now.)
    if same_rule || !same_time {
        return with(&crate::repeat::Repeat { ticked: Some(back), ..rule }, it.due_at);
    }
    let base = crate::repeat::Repeat { done: back.done, ticked: None, ..rule };
    match base.tick(back.due, now) {
        Some((next, r)) => with(&r, Some(next)),
        None => with(&base, Some(back.due)),
    }
}

/// Ticked repeating tasks whose next day has come (`reopens_at`): to-dos again, at that
/// occurrence, their reminders ready for it, their steps fresh for it (`open_row`). Run by the
/// reminder clock and when the board reads the notes. Returns the lines changed.
pub fn reopen_due(conn: &Connection, now: i64) -> Result<Vec<Item>> {
    let day = my_day(conn);
    let waiting = {
        let mut st = conn
            .prepare(&format!("SELECT {ITEM_COLS} FROM items WHERE kind = 'task' AND done_at IS NOT NULL AND due_at IS NOT NULL AND repeat LIKE '%\"ticked\"%'"))
            .map_err(err)?;
        st.query_map([], item_from_row).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?
    };
    let mut changed = Vec::new();
    for it in waiting {
        let Some(at) = reopen_time(&it, &day) else { continue };
        if now < at {
            continue;
        }
        // (Each one on its own: one that can't open now never holds back the others, nor what
        // they changed from the board.)
        let opened = (|| {
            let tx = conn.unchecked_transaction().map_err(err)?;
            let steps = open_row(conn, &it, at, now)?;
            let saved = get_item(conn, it.id)?;
            let steps = steps.into_iter().map(|id| get_item(conn, id)).collect::<Result<Vec<_>>>()?;
            tx.commit().map_err(err)?;
            Ok::<_, String>((saved, steps))
        })();
        match opened {
            Ok((saved, steps)) => {
                changed.push(saved);
                changed.extend(steps);
            }
            Err(e) => eprintln!("A repeating task couldn't open again ({}): {e}", it.id),
        }
    }
    Ok(changed)
}

/// `restore_rows` in a transaction of its own (tests).
#[cfg(test)]
pub fn restore(conn: &mut Connection, notes: &[Note], items: &[Item], pictures: &[Picture]) -> Result<()> {
    let tx = conn.transaction().map_err(err)?;
    restore_rows(&tx, notes, items, pictures)?;
    tx.commit().map_err(err)
}

/// Puts back notes, lines and pictures exactly as they were (undo after a delete), inside a
/// transaction of the caller's.
pub fn restore_rows(tx: &Connection, notes: &[Note], items: &[Item], pictures: &[Picture]) -> Result<()> {
    for n in notes {
        // An upsert, never INSERT OR REPLACE: replacing a note that still exists deletes it
        // first, and its lines and pictures would go with it (ON DELETE CASCADE).
        tx.execute(
            "INSERT INTO notes (id, title, space, palette, color, sort, archived, created_at, wide, folded, paper, archived_at, done_last)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, space = excluded.space, palette = excluded.palette,
                color = excluded.color, sort = excluded.sort, archived = excluded.archived, created_at = excluded.created_at,
                wide = excluded.wide, folded = excluded.folded, paper = excluded.paper, archived_at = excluded.archived_at,
                done_last = excluded.done_last",
            params![n.id, n.title, n.space, n.palette, n.color, n.sort, n.archived as i64, n.created_at, n.wide as i64, n.folded as i64, n.paper, n.archived_at, n.done_last as i64],
        )
        .map_err(err)?;
    }
    // Tasks before their steps (the steps point at them).
    let mut sorted: Vec<&Item> = items.iter().collect();
    sorted.sort_by_key(|i| i.parent_id.is_some());
    for i in sorted {
        tx.execute(
            // Its reminder as it was too (rung, snoozed, "I'm on it"), so a line brought back
            // doesn't ring again at once; a line still there keeps its own, which is newer.
            "INSERT OR REPLACE INTO items (id, note_id, kind, text, due_at, done_at, sort, created_at, parent_id, alarm, rhythm, link, file, repeat,
                ring_state, rings, last_ring_at, snooze_until, on_it_at, check_at)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                COALESCE(o.ring_state, ?15), COALESCE(o.rings, ?16), CASE WHEN o.id IS NULL THEN ?17 ELSE o.last_ring_at END,
                CASE WHEN o.id IS NULL THEN ?18 ELSE o.snooze_until END, CASE WHEN o.id IS NULL THEN ?19 ELSE o.on_it_at END,
                CASE WHEN o.id IS NULL THEN ?20 ELSE o.check_at END
             FROM (SELECT 1) LEFT JOIN items o ON o.id = ?1",
            params![
                i.id,
                i.note_id,
                i.kind,
                i.text,
                i.due_at,
                i.done_at,
                i.sort,
                i.created_at,
                i.parent_id,
                i.alarm,
                i.rhythm.as_ref().map(|v| v.to_string()),
                i.link.as_ref().map(|v| v.to_string()),
                i.file,
                i.repeat.as_ref().map(|v| v.to_string()),
                i.ring_state,
                i.rings,
                i.last_ring_at,
                i.snooze_until,
                i.on_it_at,
                i.check_at
            ],
        )
        .map_err(err)?;
    }
    for p in pictures {
        tx.execute(
            "INSERT OR REPLACE INTO pictures (id, note_id, item_id, path, sort, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![p.id, p.note_id, p.item_id, p.path, p.sort, p.created_at],
        )
        .map_err(err)?;
    }
    Ok(())
}

/// Every picture file the notes still use. A read that fails is an error, never an empty list
/// (the clean-up at start would take it for "no picture is used": lib.rs `used_files`).
pub fn picture_paths(conn: &Connection) -> Result<Vec<String>> {
    conn.prepare("SELECT path FROM pictures")
        .and_then(|mut st| st.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>())
        .map_err(err)
}

/// How many pictures the notes have (a check on `picture_paths`).
pub fn picture_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM pictures", [], |r| r.get(0)).map_err(err)
}

/// Link previews' pictures still used: (line id, the picture's file). As with `picture_paths`, a
/// failed read is an error.
pub fn preview_images(conn: &Connection) -> Result<Vec<(i64, String)>> {
    Ok(conn
        .prepare("SELECT id, link FROM items WHERE link IS NOT NULL")
        .and_then(|mut st| st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>())
        .map_err(err)?
        .into_iter()
        .filter_map(|(id, link)| {
            let v: serde_json::Value = serde_json::from_str(&link).ok()?;
            v.get("image")?.as_str().filter(|s| !s.is_empty()).map(|s| (id, s.to_string()))
        })
        .collect())
}

/// Points a line's preview picture at another file (after a restore from another computer).
pub fn set_preview_image(conn: &Connection, id: i64, path: &str) -> Result<()> {
    let link: Option<String> = conn.query_row("SELECT link FROM items WHERE id = ?1", [id], |r| r.get(0)).map_err(err)?;
    let Some(mut v) = link.and_then(|l| serde_json::from_str::<serde_json::Value>(&l).ok()) else { return Ok(()) };
    v["image"] = serde_json::json!(path);
    conn.execute("UPDATE items SET link = ?2 WHERE id = ?1", params![id, v.to_string()]).map_err(err)?;
    Ok(())
}

pub fn reset_ring(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE items SET ring_state = '', rings = 0, last_ring_at = NULL, snooze_until = NULL, on_it_at = NULL, check_at = NULL WHERE id = ?1",
        [id],
    )
    .map_err(err)?;
    Ok(())
}

/// The reminder clock's own fields.
pub fn save_ring(conn: &Connection, it: &Item) -> Result<()> {
    conn.execute(
        "UPDATE items SET ring_state = ?2, rings = ?3, last_ring_at = ?4, snooze_until = ?5, on_it_at = ?6, check_at = ?7, done_at = ?8 WHERE id = ?1",
        params![it.id, it.ring_state, it.rings, it.last_ring_at, it.snooze_until, it.on_it_at, it.check_at, it.done_at],
    )
    .map_err(err)?;
    Ok(())
}

pub fn delete_item(conn: &Connection, id: i64) -> Result<Vec<String>> {
    // Its steps (and their steps) go with it; so do their own pictures (files returned so they
    // can be removed). Nothing is left behind pointing at a line that is gone.
    let tx = conn.unchecked_transaction().map_err(err)?;
    let mut paths = Vec::new();
    {
        let mut st = conn.prepare(&format!("{SUBTREE} SELECT path FROM pictures WHERE item_id IN (SELECT id FROM sub)")).map_err(err)?;
        for p in st.query_map([id], |r| r.get::<_, String>(0)).map_err(err)? {
            paths.push(p.map_err(err)?);
        }
    }
    conn.execute(&format!("{SUBTREE} DELETE FROM pictures WHERE item_id IN (SELECT id FROM sub)"), [id]).map_err(err)?;
    conn.execute(&format!("{SUBTREE} DELETE FROM items WHERE id IN (SELECT id FROM sub)"), [id]).map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(paths)
}

/// Only the link of a line (a preview that arrived later). Everything else stays as it is now,
/// even if the line was changed in the meantime.
pub fn set_link(conn: &Connection, id: i64, link: Option<&serde_json::Value>) -> Result<Item> {
    conn.execute("UPDATE items SET link = ?2 WHERE id = ?1", params![id, link.map(|v| v.to_string())]).map_err(err)?;
    get_item(conn, id)
}

pub fn add_picture(conn: &Connection, note_id: i64, item_id: Option<i64>, path: &str) -> Result<Picture> {
    let sort = next_sort(conn, "SELECT MAX(sort) FROM pictures WHERE note_id = ?1", [note_id])?;
    conn.execute(
        "INSERT INTO pictures (note_id, item_id, path, sort, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![note_id, item_id, path, sort, now_ms()],
    )
    .map_err(err)?;
    let id = conn.last_insert_rowid();
    conn.query_row(&format!("SELECT {PICTURE_COLS} FROM pictures WHERE id = ?1"), [id], picture_from_row).map_err(err)
}

/// A picture that belonged to a line becomes the note's own (when the line is put away).
pub fn release_picture(conn: &Connection, id: i64, note_id: i64) -> Result<()> {
    conn.execute("UPDATE pictures SET item_id = NULL, note_id = ?2 WHERE id = ?1", params![id, note_id]).map_err(err)?;
    Ok(())
}

pub fn delete_picture(conn: &Connection, id: i64) -> Result<Option<String>> {
    let path: Option<String> = conn.query_row("SELECT path FROM pictures WHERE id = ?1", [id], |r| r.get(0)).optional().map_err(err)?;
    conn.execute("DELETE FROM pictures WHERE id = ?1", [id]).map_err(err)?;
    Ok(path)
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<serde_json::Value> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get::<_, String>(0))
        .optional()
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
}

/// A setting, where a failed read (or a value that isn't JSON) is an error, not "not set".
pub fn get_setting_strict(conn: &Connection, key: &str) -> Result<Option<serde_json::Value>> {
    let raw = conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get::<_, String>(0)).optional().map_err(err)?;
    raw.map(|v| serde_json::from_str(&v).map_err(err)).transpose()
}

pub fn set_setting(conn: &Connection, key: &str, value: &serde_json::Value) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )
    .map_err(err)?;
    Ok(())
}

#[cfg(test)]
pub fn note_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0)).unwrap_or(0)
}

/// Each space's Inbox: its first note called "Inbox" that isn't in the archive (a second one is a
/// plain note). The one rule `ensure_inbox`, the tray's count and the board (src/lib/inboxes.ts)
/// all follow.
const FIRST_INBOXES: &str = "SELECT MIN(id) AS id, space FROM notes WHERE title = 'Inbox' AND archived = 0 GROUP BY space";

/// The Inbox of a space, when it has one. (A failed read is an error, never "none": a second
/// Inbox would be made.)
pub fn inbox_of(conn: &Connection, space: &str) -> Result<Option<i64>> {
    conn.query_row(&format!("SELECT id FROM ({FIRST_INBOXES}) WHERE space = ?1"), [space], |r| r.get(0)).optional().map_err(err)
}

/// The spaces that have their Inbox.
pub fn inbox_spaces(conn: &Connection) -> Result<Vec<String>> {
    let mut st = conn.prepare(&format!("SELECT space FROM ({FIRST_INBOXES}) ORDER BY space")).map_err(err)?;
    st.query_map([], |r| r.get(0)).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)
}

/// Open lines in the Inbox notes (both spaces).
pub fn inbox_count(conn: &Connection) -> i64 {
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM items i
             WHERE i.note_id IN (SELECT id FROM ({FIRST_INBOXES})) AND i.done_at IS NULL AND i.parent_id IS NULL"
        ),
        [],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> Connection {
        let dir = std::env::temp_dir().join(format!("dobo-test-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        open(&dir.join("t.sqlite")).unwrap()
    }

    fn new_item(note_id: i64, text: &str, due: Option<i64>) -> NewItem<'_> {
        NewItem { note_id, kind: "task", text, due_at: due, parent_id: None, alarm: "reminder", link: None, file: None }
    }

    /// A notes file as the version before the Archive page made it (no `archived_at`), with an
    /// archived note and a WAL like the real one.
    fn file_before_archive_page(path: &Path) {
        let c = open(path).unwrap();
        let kept = create_note(&c, "Old trip", "personal", "soft", "Sky").unwrap();
        c.execute("UPDATE notes SET archived = 1 WHERE id = ?1", [kept.id]).unwrap();
        create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        c.execute_batch("ALTER TABLE notes DROP COLUMN archived_at").unwrap();
        let cols: Vec<String> = c.prepare("SELECT name FROM pragma_table_info('notes')").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert!(!cols.contains(&"archived_at".to_string()));
    }

    /// A notes file as every version until 1.0's unique ids made it: the first version's tables
    /// (ids given again), the columns added by later versions, notes, lines with steps, pictures
    /// (one a line's), an archived note, settings, a deleted note that had the highest id, and a
    /// WAL like the real one. Returns everything in it.
    fn file_reusing_ids(path: &Path) -> Everything {
        let c = Connection::open(path).unwrap();
        c.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                space TEXT NOT NULL DEFAULT 'work',
                palette TEXT NOT NULL DEFAULT 'soft',
                color TEXT NOT NULL DEFAULT 'Paper',
                sort REAL NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY,
                note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'task',
                text TEXT NOT NULL,
                due_at INTEGER,
                done_at INTEGER,
                sort REAL NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS items_note ON items(note_id);
             CREATE TABLE IF NOT EXISTS pictures (
                id INTEGER PRIMARY KEY,
                note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                sort REAL NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );",
        )
        .unwrap();
        // A first-version file: a note and a line before the later columns came.
        c.execute("INSERT INTO notes (title, space, sort, created_at) VALUES ('Inbox', 'work', -1, 1)", []).unwrap();
        c.execute("INSERT INTO items (note_id, text, created_at) VALUES (1, 'From the first version', 2)", []).unwrap();
        add_columns(&c).unwrap();
        add_tables(&c).unwrap();
        assert!(!tables_reusing_ids(&c).unwrap().is_empty());
        let hub = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let trip = create_note(&c, "Trip", "personal", "vivid", "Coral").unwrap();
        let trip = update_note(&c, &Note { archived: true, wide: true, paper: "grid".into(), done_last: true, ..trip.clone() }).unwrap();
        assert!(trip.done_last && !hub.done_last, "done tasks move to the end only when asked");
        let task = create_item(&c, new_item(hub.id, "Migrate", Some(5))).unwrap();
        let step = create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(hub.id, "Ask", None) }).unwrap();
        update_item(&c, &Item { repeat: Some(serde_json::json!({"freq": "week", "interval": 1})), done_at: Some(9), ..step.clone() }).unwrap();
        create_item(&c, new_item(trip.id, "Book", None)).unwrap();
        add_picture(&c, hub.id, None, "C:/a.png").unwrap();
        add_picture(&c, hub.id, Some(task.id), "C:/b.png").unwrap();
        set_setting(&c, "view", &serde_json::json!("all")).unwrap();
        // The highest id, deleted: the next note made here would be given it again.
        let gone = create_note(&c, "Gone", "work", "soft", "Sky").unwrap();
        create_item(&c, new_item(gone.id, "Gone too", None)).unwrap();
        delete_note(&c, gone.id).unwrap();
        load_all(&c).unwrap()
    }

    fn same(a: &Everything, b: &Everything) {
        assert_eq!(serde_json::to_value(a).unwrap(), serde_json::to_value(b).unwrap());
    }

    fn schema(c: &Connection) -> Vec<(String, String)> {
        c.prepare("SELECT name, COALESCE(sql, '') FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn an_older_file_gets_unique_ids_once_and_keeps_everything() {
        let dir = std::env::temp_dir().join(format!("dobo-ids-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ztodo.sqlite");
        let before = file_reusing_ids(&path);
        let old_schema = schema(&Connection::open(&path).unwrap());
        // Every start: the first one rebuilds, the next ones find nothing to do.
        for _ in 0..3 {
            let c = open(&path).unwrap();
            assert!(tables_reusing_ids(&c).unwrap().is_empty());
            same(&load_all(&c).unwrap(), &before);
            assert_eq!(c.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
            assert_eq!(c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0)).unwrap(), "ok");
            assert_eq!(broken_links(&c).unwrap(), 0);
            // The same tables, columns, links and index, only AUTOINCREMENT added (SQLite quotes
            // the name of a renamed table).
            let now = schema(&c);
            assert_eq!(now.len(), old_schema.len());
            for ((n1, s1), (n2, s2)) in old_schema.iter().zip(&now) {
                assert_eq!(n1, n2);
                let want = s1.replace("id INTEGER PRIMARY KEY,", "id INTEGER PRIMARY KEY AUTOINCREMENT,");
                assert_eq!(want, s2.replacen(&format!("CREATE TABLE \"{n2}\""), &format!("CREATE TABLE {n2}"), 1));
            }
            assert!(now.iter().any(|(n, s)| n == "items" && s.contains("REFERENCES notes(id) ON DELETE CASCADE")));
            assert!(now.iter().any(|(n, s)| n == "items_note" && s.contains("ON items(note_id)")));
        }
        // The copy made before is the file as it was.
        let copy = Connection::open(dir.join("ztodo-before-unique-ids.sqlite")).unwrap();
        same(&load_all(&copy).unwrap(), &before);
        assert_eq!(schema(&copy), old_schema);
        // Links still work: a note deleted takes its lines and pictures; a new one gets a new id.
        let mut c = open(&path).unwrap();
        let hub = before.notes.iter().find(|n| n.title == "Hub").unwrap().clone();
        let max = before.notes.iter().map(|n| n.id).max().unwrap();
        let hub_items: Vec<Item> = before.items.iter().filter(|i| i.note_id == hub.id).cloned().collect();
        let hub_pics: Vec<Picture> = before.pictures.iter().filter(|p| p.note_id == hub.id).cloned().collect();
        delete_note(&c, hub.id).unwrap();
        assert!(load_all(&c).unwrap().items.iter().all(|i| i.note_id != hub.id));
        assert!(load_all(&c).unwrap().pictures.is_empty());
        let fresh = create_note(&c, "New", "work", "soft", "Sky").unwrap();
        assert!(fresh.id > max && fresh.id > hub.id);
        restore(&mut c, &[hub], &hub_items, &hub_pics).unwrap();
        let all = load_all(&c).unwrap();
        assert!(all.notes.iter().any(|n| n.id == fresh.id && n.title == "New"));
        assert_eq!(all.items.len(), before.items.len());
        assert_eq!(all.pictures.len(), before.pictures.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_deleted_notes_lines_or_pictures_id_is_never_given_again() {
        let mut c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let line = create_item(&c, new_item(a.id, "Call", None)).unwrap();
        let pic = add_picture(&c, a.id, None, "C:/x.png").unwrap();
        let b = create_note(&c, "Trip", "work", "soft", "Sky").unwrap();
        let in_b = create_item(&c, new_item(b.id, "Pack", None)).unwrap();
        let snap = load_all(&c).unwrap();
        // The highest of each deleted, then new ones made: never with those ids.
        delete_note(&c, b.id).unwrap();
        delete_item(&c, line.id).unwrap();
        delete_picture(&c, pic.id).unwrap();
        let b2 = create_note(&c, "Made meanwhile", "work", "soft", "Sky").unwrap();
        let line2 = create_item(&c, new_item(a.id, "Made meanwhile", None)).unwrap();
        let pic2 = add_picture(&c, a.id, None, "C:/y.png").unwrap();
        assert!(b2.id > b.id && line2.id > in_b.id && pic2.id > pic.id);
        // Undo puts everything back: what was made meanwhile is still there, unchanged.
        restore(&mut c, &snap.notes, &snap.items, &snap.pictures).unwrap();
        let all = load_all(&c).unwrap();
        assert_eq!(all.notes.len(), 3);
        assert_eq!(get_note(&c, b2.id).unwrap().title, "Made meanwhile");
        assert_eq!(get_item(&c, line2.id).unwrap().text, "Made meanwhile");
        assert_eq!(all.items.len(), 3);
        assert_eq!(all.pictures.len(), 2);
    }

    #[test]
    fn a_table_not_as_expected_is_left_as_it_is() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE notes (id integer primary key, title TEXT NOT NULL, space TEXT NOT NULL DEFAULT 'work', palette TEXT NOT NULL DEFAULT 'soft',
                color TEXT NOT NULL DEFAULT 'Paper', sort REAL NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL);
             INSERT INTO notes (title, created_at) VALUES ('Hub', 1);",
        )
        .unwrap();
        let sql = table_sql(&c, "notes").unwrap();
        assert!(ids_never_reused(&c).is_err());
        assert_eq!(table_sql(&c, "notes").unwrap(), sql);
        assert_eq!(note_count(&c), 1);
        assert_eq!(c.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }

    #[test]
    fn an_older_backup_with_ids_given_again_still_restores() {
        let dir = std::env::temp_dir().join(format!("dobo-idsbackup-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("backup.sqlite");
        let before = file_reusing_ids(&old);
        // What backup::restore does: the backup's pages over the open file, then migrate.
        let mut c = open(&dir.join("now.sqlite")).unwrap();
        for _ in 0..5 {
            create_note(&c, "Will be replaced", "work", "soft", "Sage").unwrap();
        }
        let src = Connection::open(&old).unwrap();
        rusqlite::backup::Backup::new(&src, &mut c).unwrap().run_to_completion(256, std::time::Duration::ZERO, None).unwrap();
        assert!(!tables_reusing_ids(&c).unwrap().is_empty());
        migrate(&c).unwrap();
        assert!(tables_reusing_ids(&c).unwrap().is_empty());
        same(&load_all(&c).unwrap(), &before);
        let max = before.notes.iter().map(|n| n.id).max().unwrap();
        assert!(create_note(&c, "After", "work", "soft", "Sky").unwrap().id > max);
        // And a backup made now (VACUUM INTO) restores with its unique ids.
        let now = dir.join("now-backup.sqlite");
        c.execute("VACUUM INTO ?1", [now.to_string_lossy().to_string()]).unwrap();
        let back = Connection::open(&now).unwrap();
        assert!(tables_reusing_ids(&back).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_older_notes_file_gets_the_archive_time_once_and_keeps_its_notes() {
        let dir = std::env::temp_dir().join(format!("dobo-migrate-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ztodo.sqlite");
        file_before_archive_page(&path);
        // Opened by this version, then again (every start): the same notes, no error.
        for _ in 0..3 {
            let c = open(&path).unwrap();
            migrate(&c).unwrap();
            let all = load_all(&c).unwrap();
            assert_eq!(all.notes.len(), 2);
            let old = all.notes.iter().find(|n| n.title == "Old trip").unwrap();
            assert!(old.archived);
            assert_eq!(old.archived_at, None, "archived before the time was kept: no made-up date");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_backup_made_by_an_older_version_still_restores() {
        let dir = std::env::temp_dir().join(format!("dobo-oldbackup-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("backup.sqlite");
        file_before_archive_page(&old);
        // What backup::restore does: the backup's pages over the open file, then migrate.
        let mut c = open(&dir.join("now.sqlite")).unwrap();
        create_note(&c, "Will be replaced", "work", "soft", "Sage").unwrap();
        let src = Connection::open(&old).unwrap();
        rusqlite::backup::Backup::new(&src, &mut c).unwrap().run_to_completion(256, std::time::Duration::ZERO, None).unwrap();
        migrate(&c).unwrap();
        let all = load_all(&c).unwrap();
        let titles: Vec<&str> = all.notes.iter().map(|n| n.title.as_str()).collect();
        assert!(titles.contains(&"Old trip") && titles.contains(&"Hub") && !titles.contains(&"Will be replaced"));
        // And the archive works on it.
        let mut n = all.notes.iter().find(|n| n.title == "Hub").unwrap().clone();
        n.archived = true;
        assert!(update_note(&c, &n).unwrap().archived_at.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notes_and_items_round_trip() {
        let mut c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let b = create_note(&c, "Tour du monde", "personal", "soft", "Ocean").unwrap();
        assert!(b.sort > a.sort);
        let mut it = create_item(&c, new_item(a.id, "Call général", Some(1))).unwrap();
        it.done_at = Some(2);
        let it = update_item(&c, &it).unwrap();
        assert_eq!(it.done_at, Some(2));
        reorder_notes(&mut c, &[b.id, a.id]).unwrap();
        let all = load_all(&c).unwrap();
        assert_eq!(all.notes[0].id, b.id);
        delete_note(&c, a.id).unwrap();
        assert!(load_all(&c).unwrap().items.is_empty());
        set_setting(&c, "mode", &serde_json::json!("floating")).unwrap();
        assert_eq!(get_setting(&c, "mode"), Some(serde_json::json!("floating")));
    }

    #[test]
    fn steps_move_and_go_with_their_task() {
        let c = temp_db();
        let a = create_note(&c, "Logitech", "work", "soft", "Sky").unwrap();
        let b = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let task = create_item(&c, new_item(a.id, "Migrate", None)).unwrap();
        let mut step = new_item(a.id, "Ask Mélanie", None);
        step.parent_id = Some(task.id);
        let step = create_item(&c, step).unwrap();
        let mut moved = task.clone();
        moved.note_id = b.id;
        update_item(&c, &moved).unwrap();
        assert_eq!(get_item(&c, step.id).unwrap().note_id, b.id);
        delete_item(&c, task.id).unwrap();
        assert!(get_item(&c, step.id).is_err());
    }

    #[test]
    fn notes_on_another_computer_use_the_classic_journal() {
        assert_eq!(journal_mode(Path::new(r"\\server\home\Jo\AppData\Roaming\com.sookera.ztodo\ztodo.sqlite")), "DELETE");
        assert_eq!(journal_mode(Path::new(r"\\?\UNC\server\home\ztodo.sqlite")), "DELETE");
        assert_eq!(journal_mode(&std::env::temp_dir().join("ztodo.sqlite")), "WAL");
        assert_eq!(journal_mode(Path::new(":memory:")), "WAL");
        assert_eq!(journal_mode(Path::new(r"\\?\C:\x\ztodo.sqlite")), "WAL");
    }

    #[test]
    fn link_preview_pictures_are_found_and_repointed() {
        let c = temp_db();
        let a = create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        let link = serde_json::json!({"url": "https://example.com", "title": "Ex", "image": "C:/old/previews/preview-1.png"});
        let it = create_item(&c, NewItem { link: Some(&link), ..new_item(a.id, "Page", None) }).unwrap();
        create_item(&c, new_item(a.id, "Plain", None)).unwrap();
        assert_eq!(preview_images(&c).unwrap(), vec![(it.id, "C:/old/previews/preview-1.png".to_string())]);
        set_preview_image(&c, it.id, "D:/here/previews/preview-1.png").unwrap();
        let saved = get_item(&c, it.id).unwrap();
        let l = saved.link.unwrap();
        assert_eq!(l["image"], "D:/here/previews/preview-1.png");
        assert_eq!(l["title"], "Ex");
    }

    #[test]
    fn archived_notes_are_silent_until_they_come_back() {
        let c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let due = now_ms() - 60 * 60 * 1000;
        let it = create_item(&c, new_item(a.id, "Call", Some(due))).unwrap();
        let step = create_item(&c, NewItem { parent_id: Some(it.id), ..new_item(a.id, "Step", Some(due)) }).unwrap();
        assert_eq!(timed_items(&c).unwrap().len(), 2);
        let mut n = get_note(&c, a.id).unwrap();
        n.archived = true;
        update_note(&c, &n).unwrap();
        assert!(timed_items(&c).unwrap().is_empty());
        let mut ids = item_ids_of_note(&c, a.id);
        ids.sort();
        assert_eq!(ids, vec![it.id, step.id]);
        assert_eq!(subtree_ids(&c, it.id).len(), 2);
        n.archived = false;
        update_note(&c, &n).unwrap();
        assert_eq!(timed_items(&c).unwrap().len(), 2);
    }

    #[test]
    fn the_archive_keeps_when_each_note_went_in() {
        let mut c = temp_db();
        let a = create_note(&c, "Trip", "personal", "soft", "Sky").unwrap();
        assert_eq!(a.archived_at, None);
        let mut n = a.clone();
        n.archived = true;
        let before = now_ms();
        let saved = update_note(&c, &n).unwrap();
        let at = saved.archived_at.expect("archive time");
        assert!(at >= before);
        // Saved again while archived (renamed, or the page sends no time): the time stays.
        let mut again = saved.clone();
        again.title = "Trip 2025".into();
        again.archived_at = None;
        assert_eq!(update_note(&c, &again).unwrap().archived_at, Some(at));
        // Out of the archive: no time; an undo puts it back in with the time it had.
        again.archived = false;
        assert_eq!(update_note(&c, &again).unwrap().archived_at, None);
        let undo = Note { archived: true, archived_at: Some(at), ..again.clone() };
        assert_eq!(update_note(&c, &undo).unwrap().archived_at, Some(at));
        again.archived = false;
        update_note(&c, &again).unwrap();
        // Undo of a delete puts the time back exactly.
        let kept = Note { archived: true, archived_at: Some(42), ..again.clone() };
        delete_note(&c, a.id).unwrap();
        restore(&mut c, &[kept], &[], &[]).unwrap();
        assert_eq!(get_note(&c, a.id).unwrap().archived_at, Some(42));
    }

    /// A local time on this computer's clock.
    fn local(y: i16, m: i8, d: i8, h: i8, mi: i8) -> i64 {
        crate::repeat::to_ms(jiff::civil::date(y, m, d).at(h, mi, 0, 0)).unwrap()
    }

    /// A repeating task due at `due`, with this rule.
    fn repeating(c: &Connection, text: &str, due: i64, rule: serde_json::Value) -> Item {
        let n = create_note(c, "Hub", "work", "soft", "Sage").unwrap();
        let it = create_item(c, new_item(n.id, text, Some(due))).unwrap();
        update_item(c, &Item { repeat: Some(rule), ..it }).unwrap()
    }

    /// Ticked at `now`, as the board does it.
    fn tick(c: &Connection, it: &Item, now: i64) -> Item {
        update_item_at(c, &Item { done_at: Some(now), ..get_item(c, it.id).unwrap() }, now).unwrap().0
    }

    fn ticked_of(it: &Item) -> Option<crate::repeat::Ticked> {
        rule_of(it).and_then(|r| r.ticked)
    }

    /// Is it open again at `now` (the clock's look)?
    fn opens_at(c: &Connection, it: &Item, now: i64) -> bool {
        let open = reopen_due(c, now).unwrap().iter().any(|x| x.id == it.id);
        assert_eq!(open, get_item(c, it.id).unwrap().done_at.is_none());
        open
    }

    #[test]
    fn ticking_a_repeating_task_keeps_one_line_ticked_until_its_next_day() {
        let c = temp_db();
        // Every day at 10:00, Wednesday 18 November's ticked at 10:30.
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let (saved, log, _) = update_item_at(&c, &Item { done_at: Some(wed + 30 * 60_000), ..it.clone() }, wed + 30 * 60_000).unwrap();
        // One line, ticked, already at Thursday's time; its history has Wednesday's.
        assert_eq!(load_all(&c).unwrap().items.len(), 1);
        assert_eq!(saved.done_at, Some(wed + 30 * 60_000));
        assert_eq!(saved.due_at, Some(local(2026, 11, 19, 10, 0)));
        assert_eq!(saved.repeat.as_ref().unwrap()["done"], 1);
        assert_eq!(saved.repeat.as_ref().unwrap()["at"], "10:00");
        assert_eq!(ticked_of(&saved), Some(crate::repeat::Ticked { due: wed, done: 0 }));
        let log = log.expect("its history");
        assert_eq!((log.len(), log[0].item_id, log[0].due_at, log[0].done_at), (1, it.id, Some(wed), wed + 30 * 60_000));
        // Not ringing while ticked; open again at the start of Thursday (08:00 by default).
        assert!(timed_items(&c).unwrap().is_empty());
        assert!(!opens_at(&c, &it, local(2026, 11, 19, 7, 59)));
        assert!(opens_at(&c, &it, local(2026, 11, 19, 8, 0)));
        let open = get_item(&c, it.id).unwrap();
        assert_eq!((open.due_at, ticked_of(&open), open.ring_state.as_str()), (Some(local(2026, 11, 19, 10, 0)), None, ""));
        assert_eq!(timed_items(&c).unwrap().len(), 1, "its reminder rings again for Thursday's");
        // Ticked again: two times in its history, still one line.
        tick(&c, &it, local(2026, 11, 19, 10, 5));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 2);
        assert_eq!(get_item(&c, it.id).unwrap().due_at, Some(local(2026, 11, 20, 10, 0)));
    }

    #[test]
    fn it_opens_again_on_the_next_day_of_its_own_rule() {
        let c = temp_db();
        // Every weekday at 09:00: Friday's ticked, back on Monday (never over the weekend).
        let fri = local(2026, 11, 20, 9, 0);
        let wd = repeating(&c, "Standup", fri, serde_json::json!({"freq": "weekday", "interval": 1, "ends": {"kind": "never"}}));
        assert_eq!(tick(&c, &wd, fri + 60_000).due_at, Some(local(2026, 11, 23, 9, 0)));
        assert!(!opens_at(&c, &wd, local(2026, 11, 21, 12, 0)) && !opens_at(&c, &wd, local(2026, 11, 22, 20, 0)));
        assert!(opens_at(&c, &wd, local(2026, 11, 23, 8, 0)));
        // Every Friday at 18:00, this Friday's ticked early on Wednesday: back next Friday.
        let wk = repeating(&c, "Invoices", local(2026, 11, 20, 18, 0), serde_json::json!({"freq": "week", "interval": 1, "days": [5], "ends": {"kind": "never"}}));
        assert_eq!(tick(&c, &wk, local(2026, 11, 18, 12, 0)).due_at, Some(local(2026, 11, 27, 18, 0)));
        assert!(!opens_at(&c, &wk, local(2026, 11, 20, 12, 0)));
        assert!(!opens_at(&c, &wk, local(2026, 11, 27, 7, 59)));
        assert!(opens_at(&c, &wk, local(2026, 11, 27, 8, 0)));
        // Every 2 weeks on Monday: two weeks later.
        let two = repeating(&c, "Plants", local(2026, 11, 16, 9, 0), serde_json::json!({"freq": "week", "interval": 2, "days": [1], "ends": {"kind": "never"}}));
        assert_eq!(tick(&c, &two, local(2026, 11, 16, 9, 30)).due_at, Some(local(2026, 11, 30, 9, 0)));
        assert!(!opens_at(&c, &two, local(2026, 11, 23, 12, 0)));
        assert!(opens_at(&c, &two, local(2026, 11, 30, 8, 0)));
        // Every month on the 31st: the 28th in February, the 31st in March.
        let mo = repeating(&c, "Rent", local(2027, 1, 31, 8, 30), serde_json::json!({"freq": "month", "interval": 1, "monthday": 31, "ends": {"kind": "never"}}));
        assert_eq!(tick(&c, &mo, local(2027, 1, 31, 9, 0)).due_at, Some(local(2027, 2, 28, 8, 30)));
        assert!(!opens_at(&c, &mo, local(2027, 2, 27, 12, 0)));
        assert!(opens_at(&c, &mo, local(2027, 2, 28, 8, 0)));
        assert_eq!(tick(&c, &mo, local(2027, 2, 28, 9, 0)).due_at, Some(local(2027, 3, 31, 8, 30)));
    }

    #[test]
    fn a_task_before_the_day_starts_opens_by_its_own_time() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 7, 0);
        let it = repeating(&c, "Pills", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        tick(&c, &it, wed + 10 * 60_000);
        // The day starts at 08:00, the task is at 07:00: open by 07:00.
        assert!(!opens_at(&c, &it, local(2026, 11, 19, 6, 59)));
        assert!(opens_at(&c, &it, local(2026, 11, 19, 7, 0)));
        // A day that starts at 06:00: then.
        set_setting(&c, "day", &serde_json::json!({"start": "06:00", "end": "22:00"})).unwrap();
        tick(&c, &it, local(2026, 11, 19, 7, 10));
        assert!(!opens_at(&c, &it, local(2026, 11, 20, 5, 59)));
        assert!(opens_at(&c, &it, local(2026, 11, 20, 6, 0)));
    }

    #[test]
    fn ticked_once_its_next_day_has_come_it_is_open_again_at_once() {
        let c = temp_db();
        // Every day at 22:00: yesterday's ticked at 09:00 this morning.
        let it = repeating(&c, "Journal", local(2026, 11, 17, 22, 0), serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let now = local(2026, 11, 18, 9, 0);
        let (saved, log, _) = update_item_at(&c, &Item { done_at: Some(now), ..it.clone() }, now).unwrap();
        assert_eq!((saved.done_at, saved.due_at, ticked_of(&saved)), (None, Some(local(2026, 11, 18, 22, 0)), None));
        assert_eq!(log.unwrap()[0].due_at, Some(local(2026, 11, 17, 22, 0)));
        // The notification's Done does the same.
        let it2 = repeating(&c, "Journal 2", local(2026, 11, 17, 22, 0), serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let (_, log, _) = crate::reminders::save_answer(&c, it2.id, "done", now, None).unwrap();
        let after = get_item(&c, it2.id).unwrap();
        assert_eq!((after.done_at, after.due_at, log.map(|l| l.len())), (None, Some(local(2026, 11, 18, 22, 0)), Some(1)));
    }

    #[test]
    fn unticking_gives_back_the_occurrence_ticked() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let before = get_item(&c, it.id).unwrap();
        let ticked = tick(&c, &it, wed + 60_000);
        // The box clicked again: the line as the board has it, unticked.
        let (back, log, _) = update_item_at(&c, &Item { done_at: None, ..ticked.clone() }, wed + 2 * 60_000).unwrap();
        assert_eq!((back.done_at, back.due_at, rule_of(&back)), (None, Some(wed), rule_of(&before)));
        assert_eq!(log, Some(vec![]));
        // Undo of "Done" (the board puts back its "done" alone, onto the line as it is): the same.
        let ticked = tick(&c, &it, wed + 3 * 60_000);
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        let back = update_item_at(&c, &Item { done_at: None, ..ticked }, wed + 4 * 60_000).unwrap().0;
        assert_eq!((back.done_at, back.due_at, rule_of(&back)), (None, Some(wed), rule_of(&before)));
        assert!(done_log_of(&c, it.id).unwrap().is_empty());
    }

    /// The board puts a line in place before the backend answers: a tick sent shows at once as the
    /// occurrence ticked, done, without what unticking gives back. A change saved on that copy
    /// meanwhile (its words), or a tick of the board's open copy after a notification's Done, never
    /// puts it back on the occurrence just ticked nor stops its series: it stays ticked at its
    /// next one, its history once.
    #[test]
    fn a_copy_from_before_the_tick_never_unticks_it_nor_stops_it() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let t = wed + 30 * 60_000;
        let shown = Item { done_at: Some(t), ..it.clone() };
        let ticked = update_item_at(&c, &shown, t).unwrap().0;
        let (saved, log, _) = update_item_at(&c, &Item { text: "Stretch!".into(), ..shown.clone() }, t + 100).unwrap();
        assert_eq!((saved.text.as_str(), saved.done_at, saved.due_at, ticked_of(&saved)), ("Stretch!", Some(t), ticked.due_at, ticked_of(&ticked)));
        assert_eq!(log.map(|l| l.len()), Some(1), "the board is sent its history");
        assert!(!opens_at(&c, &it, t + 5_000), "never Wednesday's again");
        assert!(opens_at(&c, &it, local(2026, 11, 19, 8, 0)));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        // The open copy from before it, saved (its words) while it waits: the same.
        let c = temp_db();
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let ticked = tick(&c, &it, t);
        let saved = update_item_at(&c, &Item { text: "Stretch, 10 min".into(), ..it.clone() }, t + 100).unwrap().0;
        assert_eq!((saved.text.as_str(), saved.done_at, saved.due_at, ticked_of(&saved)), ("Stretch, 10 min", Some(t), ticked.due_at, ticked_of(&ticked)));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        // Ticked from its notification, then on the board from its open copy: one tick.
        let c = temp_db();
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        crate::reminders::save_answer(&c, it.id, "done", t, Some(wed)).unwrap();
        let saved = update_item_at(&c, &Item { done_at: Some(t + 2_000), ..it.clone() }, t + 2_000).unwrap().0;
        assert_eq!((saved.done_at, saved.due_at, ticked_of(&saved).map(|b| b.due)), (Some(t), Some(local(2026, 11, 19, 10, 0)), Some(wed)));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        assert!(opens_at(&c, &it, local(2026, 11, 20, 9, 0)), "its series goes on");
        // A rule made afresh by the page (no `ticked`, at its next time) is still a new rule.
        let c = temp_db();
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let ticked = tick(&c, &it, t);
        let rule = serde_json::json!({"freq": "week", "interval": 1, "days": [1], "ends": {"kind": "never"}, "done": 1});
        let weekly = update_item_at(&c, &Item { repeat: Some(rule), ..ticked }, t + 100).unwrap().0;
        assert_eq!(weekly.due_at, Some(local(2026, 11, 23, 10, 0)));
    }

    /// A repeating task unticked by 1.2.0 (started meanwhile) is open with what unticking gives
    /// back still on it: tidied at the start, and a tick of it works even when one is left.
    #[test]
    fn an_open_line_left_with_a_tick_on_it_ticks_again() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let mut rule = it.repeat.clone().unwrap();
        rule["ticked"] = serde_json::json!({"due": wed - 86_400_000, "done": 0});
        c.execute("UPDATE items SET repeat = ?2 WHERE id = ?1", params![it.id, rule.to_string()]).unwrap();
        let open = get_item(&c, it.id).unwrap();
        assert!(ticked_of(&open).is_some());
        let (s, _, _) = update_item_at(&c, &Item { done_at: Some(wed + 60_000), ..open.clone() }, wed + 60_000).unwrap();
        assert_eq!((s.done_at, s.due_at, ticked_of(&s).map(|b| b.due)), (Some(wed + 60_000), Some(local(2026, 11, 19, 10, 0)), Some(wed)));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        // At the next start, an open one is tidied (a ticked one keeps its own); a rule that isn't
        // JSON is left as it is.
        let other = repeating(&c, "Plants", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        c.execute("UPDATE items SET repeat = ?2 WHERE id = ?1", params![other.id, rule.to_string()]).unwrap();
        let broken = create_item(&c, new_item(other.note_id, "Odd", None)).unwrap();
        c.execute("UPDATE items SET repeat = '{\"ticked\": oops' WHERE id = ?1", [broken.id]).unwrap();
        migrate(&c).unwrap();
        assert!(ticked_of(&get_item(&c, other.id).unwrap()).is_none());
        assert_eq!(get_item(&c, other.id).unwrap().repeat.unwrap()["freq"], "day");
        assert_eq!(ticked_of(&get_item(&c, it.id).unwrap()).map(|b| b.due), Some(wed));
        let raw: String = c.query_row("SELECT repeat FROM items WHERE id = ?1", [broken.id], |r| r.get(0)).unwrap();
        assert_eq!(raw, "{\"ticked\": oops");
    }

    /// One repeating task that can't open again never holds back the others.
    #[test]
    fn one_that_cant_open_again_never_holds_back_the_others() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let rule = serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}});
        let a = repeating(&c, "Stretch", wed, rule.clone());
        let b = repeating(&c, "Plants", wed, rule);
        tick(&c, &a, wed + 60_000);
        tick(&c, &b, wed + 60_000);
        c.execute_batch(&format!("CREATE TRIGGER stuck BEFORE UPDATE OF done_at ON items WHEN OLD.id = {} BEGIN SELECT RAISE(ABORT, 'stuck'); END;", a.id)).unwrap();
        let opened = reopen_due(&c, local(2026, 11, 19, 8, 0)).unwrap();
        assert_eq!(opened.iter().map(|x| x.id).collect::<Vec<_>>(), vec![b.id]);
        assert!(get_item(&c, a.id).unwrap().done_at.is_some(), "still waiting, as it was");
        assert_eq!(rule_of(&get_item(&c, a.id).unwrap()).and_then(|r| r.ticked).map(|t| t.due), Some(wed));
    }

    /// Undo of "Done" once it opened again, in one go: back at the occurrence ticked, that time out
    /// of its history, and never ringing at once for a time past (it waits a minute).
    #[test]
    fn undo_of_done_once_open_again_never_rings_at_once() {
        use crate::reminders::{Rhythm, due_ring};
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let prev = get_item(&c, it.id).unwrap();
        tick(&c, &it, wed + 60_000);
        reopen_due(&c, local(2026, 11, 19, 8, 0)).unwrap();
        let now = get_item(&c, it.id).unwrap();
        let thu = local(2026, 11, 19, 8, 15);
        let back = Item { done_at: prev.done_at, due_at: prev.due_at, repeat: prev.repeat.clone(), ..now };
        let (s, log, _) = undo_tick(&c, &back, Some(wed + 60_000), &[], thu).unwrap();
        assert_eq!((s.done_at, s.due_at, rule_of(&s)), (None, Some(wed), rule_of(&prev)));
        assert_eq!(log, Some(vec![]));
        assert_eq!(s.snooze_until, Some(thu + UNDONE_WAIT));
        let std = Rhythm::standard();
        assert_eq!(due_ring(&s, &std, thu + 5_000), None, "never at once");
        assert!(due_ring(&s, &std, thu + UNDONE_WAIT).is_some());
        // Done again since (a notification's Done, saved first): left as it is.
        c.execute("UPDATE items SET done_at = ?2 WHERE id = ?1", params![it.id, thu + 2 * 60_000]).unwrap();
        c.execute("INSERT INTO done_log (item_id, due_at, done_at) VALUES (?1, ?2, ?3)", params![it.id, wed, thu + 2 * 60_000]).unwrap();
        let (s, log, _) = undo_tick(&c, &Item { done_at: None, ..back }, Some(thu + 2 * 60_000), &[], thu + 3 * 60_000).unwrap();
        assert_eq!((s.done_at, s.due_at), (Some(thu + 2 * 60_000), Some(wed)));
        assert_eq!(log.map(|l| l.len()), Some(1));
    }

    /// A change that opens a ticked task at once (its time set to an hour from now, today) opens
    /// it; its Undo puts it back exactly: ticked, waiting for its next day, its steps as they were,
    /// never a second time in its history.
    #[test]
    fn undo_of_a_change_that_opened_it_puts_it_back_ticked() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let task = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let step = create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, "Mat", Some(local(2026, 11, 18, 9, 0))) }).unwrap();
        update_item_at(&c, &Item { done_at: Some(local(2026, 11, 18, 9, 10)), ..step.clone() }, local(2026, 11, 18, 9, 10)).unwrap();
        let waiting = tick(&c, &task, wed + 30 * 60_000);
        let steps = vec![get_item(&c, step.id).unwrap()];
        // Its time set to 13:00 today: open at once (the clock's next look).
        let now = local(2026, 11, 18, 12, 0);
        update_item_at(&c, &Item { due_at: Some(local(2026, 11, 18, 13, 0)), ..waiting.clone() }, now).unwrap();
        assert!(opens_at(&c, &task, now + 5_000));
        let open = get_item(&c, task.id).unwrap();
        assert_eq!(get_item(&c, step.id).unwrap().done_at, None, "its step fresh");
        // Undo: the line as it was (its time too), its step back, one time in its history.
        let back = Item { done_at: waiting.done_at, due_at: waiting.due_at, repeat: waiting.repeat.clone(), ..open };
        let (s, log, given) = undo_tick(&c, &back, None, &steps, now + 60_000).unwrap();
        assert_eq!((s.done_at, s.due_at, ticked_of(&s), rule_of(&s).map(|r| r.done)), (waiting.done_at, waiting.due_at, ticked_of(&waiting), Some(1)));
        assert_eq!(log.map(|l| l.len()), Some(1));
        assert_eq!(given.iter().map(|x| (x.id, x.done_at)).collect::<Vec<_>>(), vec![(step.id, steps[0].done_at)]);
        assert!(done_log_of(&c, step.id).unwrap().is_empty());
        assert!(!opens_at(&c, &task, now + 2 * 60_000), "waiting for Thursday again");
        assert!(opens_at(&c, &task, local(2026, 11, 19, 8, 0)));
    }

    #[test]
    fn changed_while_ticked_it_keeps_what_unticking_gives_back() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let ticked = tick(&c, &it, wed + 60_000);
        // Renamed: still waiting for Thursday, with what it gives back.
        let renamed = update_item_at(&c, &Item { text: "Stretch, 10 min".into(), ..ticked.clone() }, wed + 2 * 60_000).unwrap().0;
        assert_eq!((renamed.due_at, ticked_of(&renamed)), (ticked.due_at, ticked_of(&ticked)));
        // Its rule made afresh by the page (every Monday): the next Monday after the one ticked.
        let rule = serde_json::json!({"freq": "week", "interval": 1, "days": [1], "ends": {"kind": "never"}, "done": 1});
        let weekly = update_item_at(&c, &Item { repeat: Some(rule), ..renamed }, wed + 3 * 60_000).unwrap().0;
        assert_eq!((weekly.due_at, ticked_of(&weekly).map(|t| t.due)), (Some(local(2026, 11, 23, 10, 0)), Some(wed)));
        let back = update_item_at(&c, &Item { done_at: None, ..weekly }, wed + 4 * 60_000).unwrap().0;
        assert_eq!((back.due_at, back.repeat.as_ref().unwrap()["done"].as_i64()), (Some(wed), Some(0)));
    }

    #[test]
    fn a_repeat_that_is_over_stays_done() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        // "After 2 times", one done already: this is the last.
        let it = repeating(&c, "Course", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "after", "count": 2}, "done": 1}));
        let done = tick(&c, &it, wed + 60_000);
        assert_eq!((done.done_at, done.due_at, ticked_of(&done)), (Some(wed + 60_000), Some(wed), None));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        assert!(!opens_at(&c, &it, local(2027, 1, 1, 12, 0)));
    }

    #[test]
    fn a_deleted_repeating_task_brought_back_has_its_history() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        tick(&c, &it, wed + 60_000);
        let kept = get_item(&c, it.id).unwrap();
        delete_item(&c, it.id).unwrap();
        // Undo: the line back as it was, its history with it.
        restore_rows(&c, &[], std::slice::from_ref(&kept), &[]).unwrap();
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        assert_eq!(load_all(&c).unwrap().done_log.len(), 1);
        // Deleted for good: its history goes at the next start.
        delete_item(&c, it.id).unwrap();
        migrate(&c).unwrap();
        assert!(load_all(&c).unwrap().done_log.is_empty());
        // A new line never gets someone else's (even in a file that gave ids again).
        c.execute("INSERT INTO done_log (item_id, due_at, done_at) VALUES (9999, NULL, 1)", []).unwrap();
        c.execute("UPDATE sqlite_sequence SET seq = 9998 WHERE name = 'items'", []).unwrap();
        let n = create_note(&c, "Other", "work", "soft", "Sky").unwrap();
        let fresh = create_item(&c, new_item(n.id, "New", None)).unwrap();
        assert_eq!(fresh.id, 9999);
        assert!(done_log_of(&c, fresh.id).unwrap().is_empty());
    }

    #[test]
    fn an_older_notes_file_gets_the_history_table() {
        let dir = std::env::temp_dir().join(format!("dobo-test-log-{}-{:?}", now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sqlite");
        {
            let c = open(&path).unwrap();
            let n = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
            create_item(&c, new_item(n.id, "Old copy", Some(1))).unwrap();
            c.execute_batch("DROP INDEX done_log_item; DROP TABLE done_log;").unwrap();
        }
        let c = open(&path).unwrap();
        let all = load_all(&c).unwrap();
        assert_eq!((all.items.len(), all.done_log.len()), (1, 0));
        // And it works there.
        let wed = local(2026, 11, 18, 10, 0);
        let it = Item { due_at: Some(wed), repeat: Some(serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}})), ..all.items[0].clone() };
        update_item(&c, &it).unwrap();
        tick(&c, &it, wed + 60_000);
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
    }

    #[test]
    fn a_stale_ticked_copy_never_ticks_it_a_second_time() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let ticked = tick(&c, &it, wed + 30 * 60_000);
        assert!(opens_at(&c, &it, local(2026, 11, 19, 8, 0)));
        // The board still has the ticked copy and saves it, renamed: the words are kept, the line
        // stays open at Thursday's, and its history keeps Wednesday's alone.
        let now = local(2026, 11, 19, 8, 5);
        let (saved, log, _) = update_item_at(&c, &Item { text: "Stretch, 10 min".into(), ..ticked.clone() }, now).unwrap();
        assert_eq!((saved.text.as_str(), saved.done_at, saved.due_at, ticked_of(&saved)), ("Stretch, 10 min", None, Some(local(2026, 11, 19, 10, 0)), None));
        assert_eq!(saved.repeat.as_ref().unwrap()["done"], 1);
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
        assert_eq!(log.map(|l| l.len()), Some(1), "the board is sent its history");
        // Its box clicked on that copy ("Not done"): still open, never with a `ticked`.
        let (saved, _, _) = update_item_at(&c, &Item { done_at: None, ..ticked.clone() }, now).unwrap();
        assert_eq!((saved.done_at, saved.due_at, ticked_of(&saved)), (None, Some(local(2026, 11, 19, 10, 0)), None));
        assert!(saved.repeat.as_ref().unwrap().get("ticked").is_none());
        // A real tick, from the open line, still works.
        tick(&c, &it, local(2026, 11, 19, 10, 5));
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 2);
        assert_eq!(get_item(&c, it.id).unwrap().due_at, Some(local(2026, 11, 20, 10, 0)));
    }

    /// Without its `ticked` (the board's Undo onto an open line, store `revertItem`), the copy as
    /// it waited.
    fn as_open(waiting: &Item) -> Option<serde_json::Value> {
        let mut r = waiting.repeat.clone();
        if let Some(obj) = r.as_mut().and_then(|r| r.as_object_mut()) {
            obj.remove("ticked");
        }
        r
    }

    #[test]
    fn undo_of_time_change_after_reopen() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let waiting = tick(&c, &it, wed + 60_000);
        // Its time moved to 11:00 on Thursday while it waits, then it opens again.
        let edited = update_item_at(&c, &Item { due_at: Some(local(2026, 11, 19, 11, 0)), ..waiting.clone() }, wed + 2 * 60_000).unwrap().0;
        assert_eq!((edited.done_at, edited.due_at), (waiting.done_at, Some(local(2026, 11, 19, 11, 0))));
        assert!(opens_at(&c, &it, local(2026, 11, 19, 8, 0)));
        let open = get_item(&c, it.id).unwrap();
        let now = local(2026, 11, 19, 8, 5);
        // The Undo as it waited (with `ticked`) is a stale copy: the open line stays as it is.
        let kept = update_item_at(&c, &Item { due_at: waiting.due_at, repeat: waiting.repeat.clone(), ..open.clone() }, now).unwrap().0;
        assert_eq!(kept.due_at, Some(local(2026, 11, 19, 11, 0)));
        // As the board sends it on an open line (without `ticked`): its time and rule back.
        let undone = update_item_at(&c, &Item { due_at: waiting.due_at, repeat: as_open(&waiting), ..open.clone() }, now).unwrap().0;
        assert_eq!((undone.done_at, undone.due_at, ticked_of(&undone)), (None, waiting.due_at, None));
        assert_eq!(undone.repeat.as_ref().unwrap()["at"], "10:00");
        assert_eq!(undone.repeat.as_ref().unwrap()["done"], waiting.repeat.as_ref().unwrap()["done"]);
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
    }

    #[test]
    fn undo_of_a_change_that_opened_it_at_once_after_its_own_reopen_time() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let it = repeating(&c, "Stretch", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let waiting = tick(&c, &it, wed + 60_000);
        // Its time set to 15:00 today: open at once (the clock's next look).
        update_item_at(&c, &Item { due_at: Some(local(2026, 11, 18, 15, 0)), ..waiting.clone() }, wed + 2 * 60_000).unwrap();
        assert!(opens_at(&c, &it, wed + 2 * 60_000 + 5_000));
        let opened = get_item(&c, it.id).unwrap();
        assert_eq!((opened.done_at, opened.due_at, ticked_of(&opened)), (None, Some(local(2026, 11, 18, 15, 0)), None));
        // Undone on Thursday, after it would have opened anyway: its time and rule back, open,
        // never ticked a second time.
        let now = local(2026, 11, 19, 8, 5);
        let undone = update_item_at(&c, &Item { due_at: waiting.due_at, repeat: as_open(&waiting), ..get_item(&c, it.id).unwrap() }, now).unwrap().0;
        assert_eq!((undone.done_at, undone.due_at, ticked_of(&undone)), (None, waiting.due_at, None));
        assert_eq!(undone.repeat.as_ref().unwrap()["at"], "10:00");
        assert_eq!(undone.repeat.as_ref().unwrap()["done"], waiting.repeat.as_ref().unwrap()["done"]);
        assert_eq!(done_log_of(&c, it.id).unwrap().len(), 1);
    }

    #[test]
    fn a_night_shift_task_opens_the_evening_before() {
        let c = temp_db();
        // "My day" 22:00–06:00, every day at 02:00: Thursday's ticked at 02:10, Friday's 02:00
        // belongs to the night starting at 22:00 on Thursday.
        set_setting(&c, "day", &serde_json::json!({"start": "22:00", "end": "06:00"})).unwrap();
        let thu = local(2026, 11, 19, 2, 0);
        let it = repeating(&c, "Rounds", thu, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        assert_eq!(tick(&c, &it, thu + 10 * 60_000).due_at, Some(local(2026, 11, 20, 2, 0)));
        assert!(!opens_at(&c, &it, local(2026, 11, 19, 21, 59)));
        assert!(opens_at(&c, &it, local(2026, 11, 19, 22, 0)));
    }

    #[test]
    fn its_steps_start_fresh_when_it_opens_again() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let task = repeating(&c, "Weekly review", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let step = |text: &str, due: Option<i64>| create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, text, due) }).unwrap();
        let timed = step("Inbox zero", Some(local(2026, 11, 18, 9, 0)));
        let plain = step("Notes", None);
        let snoozed = step("Call", Some(local(2026, 11, 18, 11, 0)));
        let undone = step("Plan", None);
        // Two steps ticked, one snoozed, then the task.
        let done = |s: &Item, at: i64| update_item_at(&c, &Item { done_at: Some(at), ..get_item(&c, s.id).unwrap() }, at).unwrap();
        done(&timed, local(2026, 11, 18, 9, 10));
        done(&plain, local(2026, 11, 18, 9, 20));
        c.execute("UPDATE items SET ring_state = 'snoozed', snooze_until = ?2, rings = 2 WHERE id = ?1", params![snoozed.id, local(2026, 11, 18, 12, 0)]).unwrap();
        tick(&c, &task, local(2026, 11, 18, 10, 30));
        // Unticked while it waits: its steps as they were.
        let before = load_all(&c).unwrap().items;
        update_item_at(&c, &Item { done_at: None, ..get_item(&c, task.id).unwrap() }, local(2026, 11, 18, 10, 40)).unwrap();
        tick(&c, &task, local(2026, 11, 18, 10, 45));
        let same = |id: i64| before.iter().find(|x| x.id == id).map(|x| (x.done_at, x.due_at)) == Some((get_item(&c, id).unwrap().done_at, get_item(&c, id).unwrap().due_at));
        assert!(same(timed.id) && same(plain.id) && same(snoozed.id));
        // Open again on Thursday: each step unticked, its time a day on, its reminder fresh.
        let changed = reopen_due(&c, local(2026, 11, 19, 8, 0)).unwrap();
        for s in [&timed, &plain, &snoozed] {
            assert!(changed.iter().any(|x| x.id == s.id), "{} is sent to the board", s.text);
        }
        assert!(!changed.iter().any(|x| x.id == undone.id), "nothing to change");
        let t = get_item(&c, timed.id).unwrap();
        assert_eq!((t.done_at, t.due_at, t.ring_state.as_str(), t.last_ring_at), (None, Some(local(2026, 11, 19, 9, 0)), "", None));
        assert_eq!(get_item(&c, plain.id).unwrap().done_at, None);
        let s = get_item(&c, snoozed.id).unwrap();
        assert_eq!((s.due_at, s.ring_state.as_str(), s.snooze_until, s.rings), (Some(local(2026, 11, 19, 11, 0)), "", None, 0));
        // They ring at their new times.
        let ids: Vec<i64> = timed_items(&c).unwrap().iter().map(|x| x.id).collect();
        assert!(ids.contains(&timed.id) && ids.contains(&snoozed.id));
        // The times they were done stay in the history (the look back counts them on Wednesday),
        // once each; the task's own history is only its own.
        let row = |id: i64| done_log_of(&c, id).unwrap().iter().map(|r| (r.due_at, r.done_at)).collect::<Vec<_>>();
        assert_eq!(row(timed.id), vec![(Some(local(2026, 11, 18, 9, 0)), local(2026, 11, 18, 9, 10))]);
        assert_eq!(row(plain.id), vec![(None, local(2026, 11, 18, 9, 20))]);
        assert!(row(snoozed.id).is_empty() && row(undone.id).is_empty());
        assert_eq!(row(task.id).len(), 1);
        assert_eq!(steps_logs(&c, &changed).unwrap().iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![timed.id, plain.id]);
    }

    #[test]
    fn undo_gives_the_fresh_steps_back_exactly_with_their_history() {
        let c = temp_db();
        let wed = local(2026, 11, 18, 10, 0);
        let task = repeating(&c, "Weekly review", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let step = create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, "Inbox zero", Some(local(2026, 11, 18, 9, 0))) }).unwrap();
        let snoozed = create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, "Call", Some(local(2026, 11, 18, 11, 0))) }).unwrap();
        update_item_at(&c, &Item { done_at: Some(local(2026, 11, 18, 9, 10)), ..step.clone() }, local(2026, 11, 18, 9, 10)).unwrap();
        c.execute("UPDATE items SET ring_state = 'snoozed', snooze_until = ?2, rings = 2, last_ring_at = ?3 WHERE id = ?1", params![snoozed.id, local(2026, 11, 18, 12, 0), local(2026, 11, 18, 11, 0)]).unwrap();
        let was = [get_item(&c, step.id).unwrap(), get_item(&c, snoozed.id).unwrap()];
        tick(&c, &task, local(2026, 11, 18, 10, 30));
        reopen_due(&c, local(2026, 11, 19, 8, 0)).unwrap();
        assert_eq!(done_log_of(&c, step.id).unwrap().len(), 1);
        // Undo (still on Wednesday, before its snooze ends): each step as it was, its reminder
        // too, and its time done out of the history.
        let back = steps_back(&c, &was, local(2026, 11, 18, 11, 30)).unwrap();
        assert_eq!(back.len(), 2);
        let s = get_item(&c, step.id).unwrap();
        assert_eq!((s.done_at, s.due_at), (was[0].done_at, was[0].due_at));
        assert!(done_log_of(&c, step.id).unwrap().is_empty());
        let z = get_item(&c, snoozed.id).unwrap();
        assert_eq!((z.due_at, z.ring_state.as_str(), z.snooze_until, z.rings, z.last_ring_at), (was[1].due_at, "snoozed", was[1].snooze_until, 2, was[1].last_ring_at));
        // A step deleted since is left out (the other one fresh again).
        c.execute("UPDATE items SET done_at = NULL WHERE id = ?1", [step.id]).unwrap();
        delete_item(&c, snoozed.id).unwrap();
        assert_eq!(steps_back(&c, &was, local(2026, 11, 18, 11, 30)).unwrap().len(), 1);
    }

    #[test]
    fn undo_never_rings_a_step_at_once_nor_rewrites_one_changed_since() {
        use crate::reminders::{Rhythm, due_ring};
        let c = temp_db();
        let std = Rhythm::standard();
        let wed = local(2026, 11, 18, 10, 0);
        let task = repeating(&c, "Weekly review", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let other = create_item(&c, new_item(task.note_id, "Errands", None)).unwrap();
        let step = |text: &str, due: i64| create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, text, Some(due)) }).unwrap();
        let snoozed = step("Call", local(2026, 11, 18, 11, 0));
        let timed = step("Post", local(2026, 11, 18, 10, 45));
        let ticked = step("Bins", local(2026, 11, 18, 9, 0));
        let moved = step("Plants", local(2026, 11, 18, 9, 30));
        c.execute("UPDATE items SET ring_state = 'snoozed', snooze_until = ?2, rings = 1, last_ring_at = ?3 WHERE id = ?1", params![snoozed.id, local(2026, 11, 18, 12, 0), local(2026, 11, 18, 11, 0)]).unwrap();
        let was: Vec<Item> = [&snoozed, &timed, &ticked, &moved].iter().map(|s| get_item(&c, s.id).unwrap()).collect();
        tick(&c, &task, local(2026, 11, 18, 10, 30));
        reopen_due(&c, local(2026, 11, 19, 8, 0)).unwrap();
        // Since: one ticked, one moved under another task.
        update_item_at(&c, &Item { done_at: Some(local(2026, 11, 19, 8, 10)), ..get_item(&c, ticked.id).unwrap() }, local(2026, 11, 19, 8, 10)).unwrap();
        c.execute("UPDATE items SET parent_id = ?2 WHERE id = ?1", params![moved.id, other.id]).unwrap();
        let moved_now = get_item(&c, moved.id).unwrap();
        // Undo on Thursday at 08:20: the snooze and the time given back are past.
        let now = local(2026, 11, 19, 8, 20);
        let back = steps_back(&c, &was, now).unwrap();
        assert_eq!(back.iter().map(|s| s.id).collect::<Vec<_>>(), vec![snoozed.id, timed.id]);
        for id in [snoozed.id, timed.id] {
            let s = get_item(&c, id).unwrap();
            assert_eq!(s.snooze_until, Some(now + UNDONE_WAIT), "{} waits a minute", s.text);
            assert_eq!(due_ring(&s, &std, now), None, "{} never rings at once", s.text);
            assert_eq!(due_ring(&s, &std, now + UNDONE_WAIT).map(|(t, _)| t), Some(now + UNDONE_WAIT));
        }
        // The one ticked since stays ticked; the one moved stays where it went, as it is.
        assert_eq!(get_item(&c, ticked.id).unwrap().done_at, Some(local(2026, 11, 19, 8, 10)));
        let m = get_item(&c, moved.id).unwrap();
        assert_eq!((m.parent_id, m.due_at, m.done_at), (Some(other.id), moved_now.due_at, None));
    }

    #[test]
    fn opened_late_its_steps_never_ring_at_once() {
        use crate::reminders::{Rhythm, due_ring};
        let c = temp_db();
        let std = Rhythm::standard();
        // Every day at 10:00, a step at 09:00 and one at 11:00, and a step with a daily repeat of
        // its own at 09:15. Ticked on Wednesday; the computer asleep from Thursday 08:00 to 09:30.
        let wed = local(2026, 11, 18, 10, 0);
        let task = repeating(&c, "Weekly review", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let step = |text: &str, due: i64| create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, text, Some(due)) }).unwrap();
        let early = step("Inbox zero", local(2026, 11, 18, 9, 0));
        let later = step("Call", local(2026, 11, 18, 11, 0));
        let own = step("Water", local(2026, 11, 19, 9, 15));
        let own = update_item(&c, &Item { repeat: Some(serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}})), ..own }).unwrap();
        tick(&c, &task, local(2026, 11, 18, 10, 30));
        let now = local(2026, 11, 19, 9, 30);
        let changed = reopen_due(&c, now).unwrap();
        assert!(changed.iter().any(|x| x.id == own.id), "the step with its own repeat is sent to the board too");
        // The one at 09:00 (moved to Thursday's, past): a minute's wait, as the task; never at once.
        let e = get_item(&c, early.id).unwrap();
        assert_eq!(e.due_at, Some(local(2026, 11, 19, 9, 0)));
        assert!(due_ring(&e, &std, now).is_none());
        assert_eq!(due_ring(&e, &std, now + UNDONE_WAIT).map(|(t, _)| t), Some(now + UNDONE_WAIT));
        // The one at 11:00 rings at its time.
        let l = get_item(&c, later.id).unwrap();
        assert_eq!(l.snooze_until, None);
        assert_eq!(due_ring(&l, &std, local(2026, 11, 19, 11, 0)).map(|(t, _)| t), Some(local(2026, 11, 19, 11, 0)));
        // The one with its own repeat, its 09:15 come while the task waited: quiet ("passed"),
        // its own time and rhythm kept.
        let o = get_item(&c, own.id).unwrap();
        assert_eq!((o.due_at, o.ring_state.as_str()), (Some(local(2026, 11, 19, 9, 15)), "passed"));
        assert!(due_ring(&o, &std, now).is_none() && due_ring(&o, &std, now + 30 * 60_000).is_none());
        // Opened on time (08:00), the step at 09:00 rings at 09:00, no wait.
        let c2 = temp_db();
        let task2 = repeating(&c2, "Weekly review", wed, serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let s2 = create_item(&c2, NewItem { parent_id: Some(task2.id), ..new_item(task2.note_id, "Inbox zero", Some(local(2026, 11, 18, 9, 0))) }).unwrap();
        tick(&c2, &task2, local(2026, 11, 18, 10, 30));
        reopen_due(&c2, local(2026, 11, 19, 8, 0)).unwrap();
        let s2 = get_item(&c2, s2.id).unwrap();
        assert_eq!(s2.snooze_until, None);
        assert_eq!(due_ring(&s2, &std, local(2026, 11, 19, 9, 0)).map(|(t, _)| t), Some(local(2026, 11, 19, 9, 0)));
    }

    #[test]
    fn open_again_at_once_its_steps_start_fresh_too() {
        let c = temp_db();
        // Every day at 22:00: yesterday's ticked this morning, with its step done.
        let task = repeating(&c, "Journal", local(2026, 11, 17, 22, 0), serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let s = create_item(&c, NewItem { parent_id: Some(task.id), ..new_item(task.note_id, "Three lines", Some(local(2026, 11, 17, 21, 30))) }).unwrap();
        update_item_at(&c, &Item { done_at: Some(local(2026, 11, 17, 21, 40)), ..s.clone() }, local(2026, 11, 17, 21, 40)).unwrap();
        let now = local(2026, 11, 18, 9, 0);
        let (saved, _, steps) = update_item_at(&c, &Item { done_at: Some(now), ..get_item(&c, task.id).unwrap() }, now).unwrap();
        assert_eq!((saved.done_at, saved.due_at), (None, Some(local(2026, 11, 18, 22, 0))));
        assert_eq!(steps.iter().map(|x| (x.id, x.done_at, x.due_at)).collect::<Vec<_>>(), vec![(s.id, None, Some(local(2026, 11, 18, 21, 30)))]);
        // The same from a notification's Done.
        let task2 = repeating(&c, "Journal 2", local(2026, 11, 17, 22, 0), serde_json::json!({"freq": "day", "interval": 1, "ends": {"kind": "never"}}));
        let s2 = create_item(&c, NewItem { parent_id: Some(task2.id), ..new_item(task2.note_id, "Lines", None) }).unwrap();
        update_item_at(&c, &Item { done_at: Some(local(2026, 11, 17, 21, 40)), ..s2.clone() }, local(2026, 11, 17, 21, 40)).unwrap();
        let (_, _, steps) = crate::reminders::save_answer(&c, task2.id, "done", now, None).unwrap();
        assert_eq!(steps.iter().map(|x| (x.id, x.done_at)).collect::<Vec<_>>(), vec![(s2.id, None)]);
    }

    #[test]
    fn steps_are_paused_while_their_task_is_done() {
        use crate::reminders::{Rhythm, apply_ring, due_ring};
        const MIN: i64 = 60_000;
        let c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = now_ms();
        let task = create_item(&c, new_item(a.id, "Move", None)).unwrap();
        let step = |text: &'static str, parent: i64, due: i64| create_item(&c, NewItem { parent_id: Some(parent), ..new_item(a.id, text, Some(due)) }).unwrap();
        // One rang just before the task was ticked; one comes while it is done; one (a step of a
        // step) comes after it is unticked again.
        let ringing = step("Pack", task.id, now - 2 * MIN);
        let mut r = get_item(&c, ringing.id).unwrap();
        apply_ring(&mut r, crate::reminders::Ring::Due, now - 2 * MIN);
        save_ring(&c, &r).unwrap();
        let meanwhile = step("Call", task.id, now + 10 * MIN);
        let later = step("Keys", meanwhile.id, now + 60 * MIN);
        assert_eq!(timed_items(&c).unwrap().len(), 3);
        // Ticked: none of them rings, the one ringing stops showing; nothing is deleted.
        let (_, _, steps) = update_item_at(&c, &Item { done_at: Some(now), ..task.clone() }, now).unwrap();
        assert_eq!(steps.iter().map(|s| s.id).collect::<Vec<_>>(), vec![ringing.id]);
        assert_eq!(steps[0].ring_state, "");
        assert!(timed_items(&c).unwrap().is_empty());
        assert_eq!(get_item(&c, later.id).unwrap().due_at, Some(now + 60 * MIN));
        // Unticked 20 minutes later: all three are back…
        let back = now + 20 * MIN;
        let (_, _, steps) = update_item_at(&c, &Item { done_at: None, ..get_item(&c, task.id).unwrap() }, back).unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(timed_items(&c).unwrap().len(), 3);
        let std = Rhythm::standard();
        // …the one whose time came meanwhile stays quiet, even after its usual 5-minute rings…
        let m = get_item(&c, meanwhile.id).unwrap();
        assert_eq!(m.ring_state, "passed");
        assert!(due_ring(&m, &std, back).is_none() && due_ring(&m, &std, back + 30 * MIN).is_none());
        // …(until its time is changed),
        let moved = update_item_at(&c, &Item { due_at: Some(back + 5 * MIN), ..m }, back).unwrap().0;
        assert_eq!(due_ring(&moved, &std, back + 5 * MIN).map(|(t, _)| t), Some(back + 5 * MIN));
        // the one that was ringing goes on with its next ring, not at once,
        let r = get_item(&c, ringing.id).unwrap();
        assert!(due_ring(&r, &std, back).is_none());
        assert_eq!(due_ring(&r, &std, now + 23 * MIN).map(|(t, _)| t), Some(now + 23 * MIN));
        // and the step of a step rings at its time.
        let l = get_item(&c, later.id).unwrap();
        assert!(due_ring(&l, &std, now + 59 * MIN).is_none());
        assert!(due_ring(&l, &std, now + 60 * MIN).is_some());
    }

    #[test]
    fn not_done_on_an_overdue_task_waits_a_minute() {
        use crate::reminders::{Rhythm, due_ring};
        const MIN: i64 = 60_000;
        let c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let now = now_ms();
        let std = Rhythm::standard();
        let late = create_item(&c, new_item(a.id, "Pay", Some(now - 30 * MIN))).unwrap();
        let done = update_item_at(&c, &Item { done_at: Some(now - 10 * MIN), ..late }, now - 10 * MIN).unwrap().0;
        let undone = update_item_at(&c, &Item { done_at: None, ..done }, now).unwrap().0;
        assert_eq!(undone.snooze_until, Some(now + UNDONE_WAIT));
        assert!(due_ring(&undone, &std, now).is_none());
        assert!(due_ring(&undone, &std, now + UNDONE_WAIT - 1).is_none());
        assert_eq!(due_ring(&undone, &std, now + UNDONE_WAIT).map(|(t, _)| t), Some(now + UNDONE_WAIT));
        // A deadline whose early rings have begun waits too; one still to come rings at its time.
        let soon = create_item(&c, NewItem { alarm: "deadline", ..new_item(a.id, "Ship", Some(now + 20 * MIN)) }).unwrap();
        let done = update_item_at(&c, &Item { done_at: Some(now), ..soon }, now).unwrap().0;
        assert!(update_item_at(&c, &Item { done_at: None, ..done }, now).unwrap().0.snooze_until.is_some());
        let later = create_item(&c, new_item(a.id, "Call", Some(now + 20 * MIN))).unwrap();
        let done = update_item_at(&c, &Item { done_at: Some(now), ..later }, now).unwrap().0;
        let undone = update_item_at(&c, &Item { done_at: None, ..done }, now).unwrap().0;
        assert_eq!(undone.snooze_until, None);
        assert_eq!(due_ring(&undone, &std, now + 20 * MIN).map(|(t, _)| t), Some(now + 20 * MIN));
    }

    #[test]
    fn restore_brings_back_a_deleted_note_with_its_lines() {
        let mut c = temp_db();
        let a = create_note(&c, "Logitech", "work", "soft", "Sky").unwrap();
        let task = create_item(&c, new_item(a.id, "Migrate", None)).unwrap();
        let mut step = new_item(a.id, "List repos", None);
        step.parent_id = Some(task.id);
        let step = create_item(&c, step).unwrap();
        let pic = add_picture(&c, a.id, None, "C:/x.png").unwrap();
        let before = load_all(&c).unwrap();
        delete_note(&c, a.id).unwrap();
        assert!(load_all(&c).unwrap().notes.is_empty());
        restore(&mut c, &before.notes, &before.items, &before.pictures).unwrap();
        let after = load_all(&c).unwrap();
        assert_eq!(after.notes[0].id, a.id);
        assert_eq!(get_item(&c, step.id).unwrap().parent_id, Some(task.id));
        assert_eq!(after.pictures[0].id, pic.id);
    }

    #[test]
    fn undo_of_a_note_that_still_exists_keeps_its_lines() {
        let mut c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        create_item(&c, new_item(a.id, "Keep me", None)).unwrap();
        add_picture(&c, a.id, None, "C:/x.png").unwrap();
        let mut renamed = a.clone();
        renamed.title = "Hub 2".into();
        restore(&mut c, &[renamed], &[], &[]).unwrap();
        let all = load_all(&c).unwrap();
        assert_eq!(all.notes[0].title, "Hub 2");
        assert_eq!(all.items.len(), 1);
        assert_eq!(all.pictures.len(), 1);
    }

    #[test]
    fn steps_of_steps_move_and_go_with_the_task() {
        let c = temp_db();
        let a = create_note(&c, "Logitech", "work", "soft", "Sky").unwrap();
        let b = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let task = create_item(&c, new_item(a.id, "Migrate", None)).unwrap();
        let mut step = new_item(a.id, "Ask", None);
        step.parent_id = Some(task.id);
        let step = create_item(&c, step).unwrap();
        let mut sub = new_item(a.id, "Ring at 9", Some(1000));
        sub.parent_id = Some(step.id);
        let sub = create_item(&c, sub).unwrap();
        add_picture(&c, a.id, Some(sub.id), "C:/sub.png").unwrap();
        let mut moved = task.clone();
        moved.note_id = b.id;
        update_item(&c, &moved).unwrap();
        assert_eq!(get_item(&c, sub.id).unwrap().note_id, b.id);
        // Deleting the old note no longer takes the step of a step with it.
        delete_note(&c, a.id).unwrap();
        assert!(get_item(&c, sub.id).is_ok());
        let paths = delete_item(&c, task.id).unwrap();
        assert_eq!(paths, vec!["C:/sub.png".to_string()]);
        assert!(get_item(&c, sub.id).is_err());
        assert!(load_all(&c).unwrap().pictures.is_empty());
        assert!(timed_items(&c).unwrap().is_empty());
    }

    #[test]
    fn the_inbox_count_is_each_spaces_first_inbox() {
        let c = temp_db();
        let w = create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        let p = create_note(&c, "Inbox", "personal", "soft", "Butter").unwrap();
        // A second note called "Inbox" (an old Inbox put back from the archive): a plain note.
        let again = create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        create_item(&c, new_item(w.id, "One", None)).unwrap();
        create_item(&c, new_item(p.id, "Two", None)).unwrap();
        create_item(&c, new_item(again.id, "Not waiting", None)).unwrap();
        assert_eq!(inbox_count(&c), 2);
    }

    #[test]
    fn a_late_preview_only_changes_the_link() {
        let c = temp_db();
        let a = create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        let b = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let it = create_item(&c, new_item(a.id, "A page", None)).unwrap();
        let mut moved = it.clone();
        moved.note_id = b.id;
        update_item(&c, &moved).unwrap();
        let after = set_link(&c, it.id, Some(&serde_json::json!({"url": "https://a.b", "title": "A"}))).unwrap();
        assert_eq!(after.note_id, b.id);
        assert_eq!(after.link.unwrap()["title"], "A");
    }

    #[test]
    fn changing_the_time_restarts_the_reminder() {
        let c = temp_db();
        let a = create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        let mut it = create_item(&c, new_item(a.id, "Weekly", Some(1000))).unwrap();
        it.ring_state = "ringing".into();
        it.rings = 3;
        it.last_ring_at = Some(5);
        save_ring(&c, &it).unwrap();
        // The page sends back stale clock fields: they are ignored.
        let mut edit = it.clone();
        edit.text = "Weekly call".into();
        edit.ring_state = String::new();
        let kept = update_item(&c, &edit).unwrap();
        assert_eq!(kept.ring_state, "ringing");
        assert_eq!(kept.rings, 3);
        edit.due_at = Some(2000);
        let restarted = update_item(&c, &edit).unwrap();
        assert_eq!(restarted.ring_state, "");
        assert_eq!(restarted.rings, 0);
    }
}
