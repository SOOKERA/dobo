// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Bring in the notes from Windows Sticky Notes, when asked (the first start offers it; Settings ›
//! Start any time). Its database is copied first and only the copy is opened, so the original is
//! never touched.

use crate::db;
use rusqlite::Connection;

/// A sticky note turned into a Dobo note: a title and its lines.
#[derive(Debug, PartialEq)]
pub struct Imported {
    pub title: String,
    pub lines: Vec<String>,
}

/// A sticky note read from Sticky Notes: its own id (to recognise it next time), its text as
/// Sticky Notes keeps it (to recognise one without an id) and its shape.
pub struct Found {
    pub id: String,
    pub text: String,
    pub note: Imported,
}

/// Sticky Notes writes each paragraph as `\id=<uuid> text`. Returns the non-empty lines.
pub fn clean_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            let line = line.trim_start();
            let line = match line.strip_prefix("\\id=") {
                Some(rest) => rest.split_once(' ').map(|(_, t)| t).unwrap_or(""),
                None => line,
            };
            let line = line.trim();
            let line = line.strip_prefix("- ").or_else(|| line.strip_prefix("• ")).unwrap_or(line);
            line.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|l| !l.is_empty() && l != "-")
        .collect()
}

const TITLE_MAX: usize = 40;

/// One line: it goes to the Inbox (title empty). Several: the first line is the title; when it
/// is too long to be a title, it is kept as the first line too, so nothing is lost.
pub fn shape(lines: Vec<String>) -> Option<Imported> {
    match lines.len() {
        0 => None,
        1 => Some(Imported { title: String::new(), lines }),
        _ => {
            let first = lines[0].trim_end_matches(':').trim_end_matches(" -").trim().to_string();
            if first.chars().count() <= TITLE_MAX {
                Some(Imported { title: first, lines: lines[1..].to_vec() })
            } else {
                let short: String = first.chars().take(TITLE_MAX - 1).collect();
                Some(Imported { title: format!("{}…", short.trim_end()), lines })
            }
        }
    }
}

/// Only one copy of Sticky Notes' file is read at a time (the first start's count and an import
/// share the folder the copy goes to).
static READING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The sticky notes in a Sticky Notes folder (`LocalState`, holding plum.sqlite): its file is
/// copied to `tmp` and only the copy is opened. No file there: none.
pub fn read_from(src: &std::path::Path, tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    let db_file = src.join("plum.sqlite");
    if !db_file.exists() {
        return Ok(Vec::new());
    }
    let _one = READING.lock().unwrap_or_else(|e| e.into_inner());
    // A fresh folder: a journal left by an earlier try must never be mixed into this copy.
    let _ = std::fs::remove_dir_all(tmp);
    std::fs::create_dir_all(tmp).map_err(|e| e.to_string())?;
    for name in ["plum.sqlite", "plum.sqlite-wal", "plum.sqlite-shm"] {
        let from = src.join(name);
        if from.exists() {
            std::fs::copy(&from, tmp.join(name)).map_err(|e| e.to_string())?;
        }
    }
    let rows = (|| -> rusqlite::Result<Vec<(Option<String>, Option<String>)>> {
        let conn = Connection::open(tmp.join("plum.sqlite"))?;
        let mut st = conn.prepare("SELECT Id, Text FROM Note WHERE DeletedAt IS NULL ORDER BY CreatedAt")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    })();
    let _ = std::fs::remove_dir_all(tmp);
    Ok(rows
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|(id, text)| {
            let text = text?;
            let note = shape(clean_lines(&text))?;
            Some(Found { id: id.unwrap_or_default(), text, note })
        })
        .collect())
}

#[cfg(windows)]
fn read_sticky_notes(tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    let local = std::env::var("LOCALAPPDATA").map_err(|e| e.to_string())?;
    let src = std::path::Path::new(&local)
        .join("Packages")
        .join("Microsoft.MicrosoftStickyNotes_8wekyb3d8bbwe")
        .join("LocalState");
    read_from(&src, tmp)
}

#[cfg(not(windows))]
fn read_sticky_notes(_tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    Ok(Vec::new())
}

/// What a sticky note is recorded as once brought in: its Sticky Notes id, or, for one without
/// an id, its fingerprint (`fingerprint`). Such a note edited in Sticky Notes afterwards is
/// another text: it counts as new, and is offered again.
fn key_of(f: &Found) -> String {
    if f.id.is_empty() { fingerprint(&f.text) } else { f.id.clone() }
}

/// "text:" and a fingerprint of a sticky note's text as Sticky Notes keeps it (its Text column),
/// never of what Dobo makes of it: how Dobo reads the words may change from one version to the
/// next, the text itself doesn't. Only trimmed, its line ends made the same (CRLF as LF). FNV-1a
/// of its UTF-8 bytes: the same on every computer and every version (a backup restored
/// elsewhere still recognises it), so it must never change.
fn fingerprint(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.trim().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("text:{h:016x}")
}

/// Already brought in earlier: it was recorded (`key_of`), or, for a board that imported before
/// ids were recorded (`legacy`), a note with the same title, or an Inbox line with the same text,
/// is there. (Only then: on any other board, a sticky note that happens to share its title with
/// one of your notes was never brought in, and must not be skipped.)
fn already_here(conn: &Connection, seen: &std::collections::HashSet<String>, legacy: bool, f: &Found) -> bool {
    if seen.contains(&key_of(f)) {
        return true;
    }
    if !legacy {
        return false;
    }
    if f.note.title.is_empty() {
        let text = f.note.lines.first().map(String::as_str).unwrap_or("");
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM items i JOIN notes n ON n.id = i.note_id WHERE n.title = 'Inbox' AND i.text = ?1)",
            [text],
            |r| r.get(0),
        )
        .unwrap_or(false)
    } else {
        conn.query_row("SELECT EXISTS(SELECT 1 FROM notes WHERE title = ?1)", [&f.note.title], |r| r.get(0)).unwrap_or(false)
    }
}

const COLORS: [&str; 9] = ["Butter", "Sage", "Rose", "Sky", "Lilac", "Peach", "Ocean", "Sand", "Stone"];

/// Sticky Notes' notes, read from a copy of their file (no Dobo notes held meanwhile).
pub fn read(tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    read_sticky_notes(tmp)
}

/// The sticky notes recorded as brought in, and whether this board imported them before ids
/// were recorded (Dobo 1.0.0's first start imported by itself).
fn known(conn: &Connection) -> (std::collections::HashSet<String>, bool) {
    let ids = db::get_setting(conn, "sticky_ids");
    let legacy = ids.is_none() && db::get_setting(conn, "sticky_imported").is_some();
    (ids.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(), legacy)
}

/// How many of these sticky notes an import would bring in (those not already here). Nothing is
/// written: the first start asks with this number, and shows nothing when it is 0.
pub fn count_new(conn: &Connection, found: &[Found]) -> usize {
    let (seen, legacy) = known(conn);
    found.iter().filter(|f| !already_here(conn, &seen, legacy, f)).count()
}

/// Adds the sticky notes not already here to the board (no duplicates). Returns how many sticky
/// notes came in: a note each, or a line in the Inbox for a one-line one.
pub fn import_found(conn: &Connection, found: Vec<Found>) -> Result<usize, String> {
    let (mut seen, legacy) = known(conn);
    let (old, found): (Vec<Found>, Vec<Found>) = found.into_iter().partition(|f| already_here(conn, &seen, legacy, f));
    // Those recognised by their title or text are recorded now (from now on, by what they are
    // recorded as only).
    seen.extend(old.iter().map(key_of));
    // All the notes or none (a failure halfway would leave half of them, and trying again
    // would add the first half twice).
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let brought = found.len();
    let mut inbox: Option<i64> = None;
    for (i, f) in found.into_iter().enumerate() {
        seen.insert(key_of(&f));
        let n = f.note;
        let note_id = if n.title.is_empty() {
            match inbox {
                Some(id) => id,
                None => {
                    let id = crate::ensure_inbox(conn, "work")?;
                    inbox = Some(id);
                    id
                }
            }
        } else {
            db::create_note(conn, &n.title, "work", "soft", COLORS[i % COLORS.len()])?.id
        };
        for line in n.lines {
            db::create_item(
                conn,
                db::NewItem { note_id, kind: "task", text: &line, due_at: None, parent_id: None, alarm: "reminder", link: None, file: None },
            )?;
        }
    }
    db::set_setting(conn, "sticky_ids", &serde_json::json!(seen))?;
    // (The board then offers, once, the times found in these lines: StickyDates.)
    if brought > 0 {
        db::set_setting(conn, "sticky_imported", &serde_json::json!(brought))?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(brought)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ids_and_blank_lines() {
        let raw = "\\id=55eb8f91-6767 Naviguer dans l'ocean\n\\id=63b3 \n\\id=e9bc - La prouess  de rester\n";
        assert_eq!(clean_lines(raw), vec!["Naviguer dans l'ocean", "La prouess de rester"]);
    }

    fn found(id: &str, lines: &[&str]) -> Found {
        Found { id: id.into(), text: lines.join("\n"), note: shape(lines.iter().map(|s| s.to_string()).collect()).unwrap() }
    }

    #[test]
    fn importing_again_adds_no_duplicates() {
        let path = std::env::temp_dir().join(format!("dobo-sticky-{}-{:?}.sqlite", db::now_ms(), std::thread::current().id()));
        let c = db::open(&path).unwrap();
        let first = vec![found("a1", &["Logitech", "Migrate"]), found("b2", &["Book of concord"])];
        // Both came in: one as a note, one as an Inbox line.
        assert_eq!(import_found(&c, first).unwrap(), 2);
        let again = vec![found("a1", &["Logitech", "Migrate"]), found("b2", &["Book of concord"]), found("c3", &["Hub", "Weekly"])];
        assert_eq!(import_found(&c, again).unwrap(), 1);
        assert_eq!(db::note_count(&c), 3); // Logitech, Inbox, Hub
        assert_eq!(db::inbox_count(&c), 1);
        drop(c);
        let _ = std::fs::remove_file(&path);
    }

    /// A sticky note Sticky Notes gave no id: recognised by its words the next time.
    #[test]
    fn a_sticky_note_without_an_id_comes_in_once() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let first = vec![found("", &["Call the plumber"]), found("", &["Trip", "Tickets", "Hotel"])];
        assert_eq!(import_found(&c, first).unwrap(), 2);
        let again = vec![found("", &["Call the plumber"]), found("", &["Trip", "Tickets", "Hotel"]), found("", &["Trip", "Tickets"])];
        assert_eq!(count_new(&c, &again), 1, "only the changed one is new");
        assert_eq!(import_found(&c, again).unwrap(), 1);
        assert_eq!(db::inbox_count(&c), 1);
        assert_eq!(key_of(&found("", &["Call the plumber"])), key_of(&found("", &["Call the plumber"])));
        assert_ne!(key_of(&found("", &["Call the plumber"])), key_of(&found("", &["Call the plumber!"])));
    }

    /// The fingerprint of a sticky note without an id is of its text as Sticky Notes keeps it,
    /// and stays the same from one version of Dobo to the next (one recorded must still match).
    #[test]
    fn a_sticky_notes_fingerprint_never_changes() {
        let text = "\\id=1 Trip\n\\id=2 Tickets";
        assert_eq!(fingerprint(""), "text:cbf29ce484222325");
        assert_eq!(fingerprint(text), "text:fc11cc883d9f4802");
        // Only its line ends and the space around it don't count.
        assert_eq!(fingerprint("  \\id=1 Trip\r\n\\id=2 Tickets\r\n"), fingerprint(text));
        // What Dobo reads in it doesn't either: the same words written otherwise are another text.
        assert_ne!(fingerprint("\\id=1 Trip\n\\id=2 - Tickets"), fingerprint(text));
        let f = Found { id: String::new(), text: text.into(), note: shape(vec!["Trip".into(), "Tickets".into()]).unwrap() };
        assert_eq!(key_of(&f), "text:fc11cc883d9f4802");
        assert_eq!(key_of(&Found { id: "n1".into(), ..f }), "n1");
    }

    #[test]
    fn a_board_imported_before_ids_were_kept_gets_no_duplicates_either() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        // What an early version left: the notes, no ids.
        let hub = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        db::create_item(&c, db::NewItem { note_id: hub.id, kind: "task", text: "Weekly", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        let inbox = crate::ensure_inbox(&c, "work").unwrap();
        db::create_item(&c, db::NewItem { note_id: inbox, kind: "task", text: "Book of concord", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        db::set_setting(&c, "sticky_imported", &serde_json::json!(2)).unwrap();
        let old = vec![found("h1", &["Hub", "Weekly"]), found("b2", &["Book of concord"])];
        assert_eq!(import_found(&c, old).unwrap(), 0);
        assert_eq!(db::note_count(&c), 2);
        // Recognised once by title and text, they are known by id from now on.
        let ids: std::collections::HashSet<String> = serde_json::from_value(db::get_setting(&c, "sticky_ids").unwrap()).unwrap();
        assert!(ids.contains("h1") && ids.contains("b2"));
        assert_eq!(import_found(&c, vec![found("h1", &["Hub", "Weekly"])]).unwrap(), 0);
    }

    #[test]
    fn a_sticky_note_named_like_one_of_your_notes_still_comes_in() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        // Your own "Groceries" note; Sticky Notes has another, never imported.
        db::create_note(&c, "Groceries", "work", "soft", "Sage").unwrap();
        crate::ensure_inbox(&c, "work").unwrap();
        db::create_item(&c, db::NewItem { note_id: crate::ensure_inbox(&c, "work").unwrap(), kind: "task", text: "Call mum", due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }).unwrap();
        let sticky = vec![found("g1", &["Groceries", "Milk"]), found("m1", &["Call mum"])];
        assert_eq!(count_new(&c, &sticky), 2);
        assert_eq!(import_found(&c, sticky).unwrap(), 2);
        let all = db::load_all(&c).unwrap();
        assert_eq!(all.notes.iter().filter(|n| n.title == "Groceries").count(), 2);
        assert_eq!(all.items.iter().filter(|i| i.text == "Call mum").count(), 2);
        assert!(all.items.iter().any(|i| i.text == "Milk"));
    }

    #[test]
    fn counting_writes_nothing_and_an_import_leaves_nothing_to_count() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        crate::ensure_inbox(&c, "work").unwrap();
        let sticky = || vec![found("a1", &["Logitech", "Migrate"]), found("b2", &["Book of concord"]), found("c3", &["Hub", "Weekly"])];
        assert_eq!(count_new(&c, &sticky()), 3);
        // Asked twice, nothing changed: the board, and no record of an import.
        assert_eq!(count_new(&c, &sticky()), 3);
        assert_eq!(db::note_count(&c), 1);
        assert!(db::get_setting(&c, "sticky_ids").is_none() && db::get_setting(&c, "sticky_imported").is_none());
        // Brought in: they are recorded (and the board offers their times once).
        assert_eq!(import_found(&c, sticky().into_iter().take(2).collect()).unwrap(), 2);
        assert_eq!(db::get_setting(&c, "sticky_imported"), Some(serde_json::json!(2)));
        assert_eq!(count_new(&c, &sticky()), 1, "only the one not brought in yet");
        assert_eq!(import_found(&c, sticky()).unwrap(), 1);
        assert_eq!(count_new(&c, &sticky()), 0);
        assert_eq!(import_found(&c, sticky()).unwrap(), 0);
    }

    #[test]
    fn nothing_to_count_on_a_computer_without_sticky_notes() {
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        let none = std::env::temp_dir().join(format!("dobo-sticky-none-{}-{:?}", db::now_ms(), std::thread::current().id()));
        let found = read_from(&none, &none.join("copy")).unwrap();
        assert!(found.is_empty());
        assert_eq!(count_new(&c, &found), 0);
        assert!(!none.exists(), "nothing made when there is nothing to read");
    }

    /// A Sticky Notes folder as Windows keeps it: plum.sqlite with its Note table.
    fn fixture(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        let c = Connection::open(dir.join("plum.sqlite")).unwrap();
        c.execute_batch("CREATE TABLE Note (Id TEXT PRIMARY KEY, Text TEXT, DeletedAt INTEGER, CreatedAt INTEGER)").unwrap();
        for (id, text, deleted, created) in [
            ("n2", "\\id=1 Groceries\n\\id=2 - Milk\n\\id=3 Bread", None, 20),
            ("n1", "\\id=4 Call the plumber", None, 10),
            ("n3", "\\id=5 Thrown away", Some(1_700_000_000_i64), 30),
            ("n4", "\\id=6 ", None, 40),
        ] {
            c.execute("INSERT INTO Note VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id, text, deleted, created]).unwrap();
        }
    }

    #[test]
    fn reads_a_copy_of_sticky_notes_and_leaves_the_original_as_it_was() {
        let root = std::env::temp_dir().join(format!("dobo-sticky-read-{}-{:?}", db::now_ms(), std::thread::current().id()));
        let src = root.join("LocalState");
        let tmp = root.join("cache").join("sticky-import");
        fixture(&src);
        let before = std::fs::read(src.join("plum.sqlite")).unwrap();
        let modified = std::fs::metadata(src.join("plum.sqlite")).unwrap().modified().unwrap();

        let found = read_from(&src, &tmp).unwrap();
        // In the order they were made; the deleted one and the empty one left out.
        assert_eq!(found.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), vec!["n1", "n2"]);
        assert_eq!(found[0].note, Imported { title: String::new(), lines: vec!["Call the plumber".into()] });
        assert_eq!(found[1].note, Imported { title: "Groceries".into(), lines: vec!["Milk".into(), "Bread".into()] });
        // The original untouched, the copy gone.
        assert_eq!(std::fs::read(src.join("plum.sqlite")).unwrap(), before);
        assert_eq!(std::fs::metadata(src.join("plum.sqlite")).unwrap().modified().unwrap(), modified);
        assert!(!tmp.exists());

        // The count the first start asks with, then the import it makes when told to.
        let c = db::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(count_new(&c, &found), 2);
        assert_eq!(import_found(&c, found).unwrap(), 2);
        assert_eq!(count_new(&c, &read_from(&src, &tmp).unwrap()), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn shapes_titles() {
        let one = shape(vec!["Book of concord".into()]).unwrap();
        assert_eq!(one.title, "");
        let list = shape(vec!["Material rework :".into(), "AO".into()]).unwrap();
        assert_eq!(list.title, "Material rework");
        assert_eq!(list.lines, vec!["AO"]);
        let long = shape(vec!["Agrandir la height du perso dans phylantropie".into(), "x".into()]).unwrap();
        assert!(long.title.ends_with('…'));
        assert_eq!(long.lines.len(), 2);
    }
}
