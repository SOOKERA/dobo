// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! First run: bring in the notes from Windows Sticky Notes. Its database is copied first and
//! only the copy is opened, so the original is never touched.

use crate::db;
use rusqlite::Connection;

/// A sticky note turned into a Dobo note: a title and its lines.
#[derive(Debug, PartialEq)]
pub struct Imported {
    pub title: String,
    pub lines: Vec<String>,
}

/// A sticky note read from Sticky Notes: its own id (to recognise it next time) and its shape.
pub struct Found {
    pub id: String,
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

#[cfg(windows)]
fn read_sticky_notes(tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    let local = std::env::var("LOCALAPPDATA").map_err(|e| e.to_string())?;
    let src = std::path::Path::new(&local)
        .join("Packages")
        .join("Microsoft.MicrosoftStickyNotes_8wekyb3d8bbwe")
        .join("LocalState");
    let db_file = src.join("plum.sqlite");
    if !db_file.exists() {
        return Ok(Vec::new());
    }
    // A fresh folder: a journal left by an earlier try must never be mixed into this copy.
    let _ = std::fs::remove_dir_all(tmp);
    std::fs::create_dir_all(tmp).map_err(|e| e.to_string())?;
    for name in ["plum.sqlite", "plum.sqlite-wal", "plum.sqlite-shm"] {
        let from = src.join(name);
        if from.exists() {
            std::fs::copy(&from, tmp.join(name)).map_err(|e| e.to_string())?;
        }
    }
    let conn = Connection::open(tmp.join("plum.sqlite")).map_err(|e| e.to_string())?;
    let mut st = conn
        .prepare("SELECT Id, Text FROM Note WHERE DeletedAt IS NULL ORDER BY CreatedAt")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([], |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    drop(st);
    drop(conn);
    let _ = std::fs::remove_dir_all(tmp);
    Ok(rows
        .into_iter()
        .filter_map(|(id, text)| {
            let note = shape(clean_lines(text.as_deref()?))?;
            Some(Found { id: id.unwrap_or_default(), note })
        })
        .collect())
}

#[cfg(not(windows))]
fn read_sticky_notes(_tmp: &std::path::Path) -> Result<Vec<Found>, String> {
    Ok(Vec::new())
}

/// Already brought in earlier: its Sticky Notes id was recorded, or (imported before ids were
/// recorded) a note with the same title, or an Inbox line with the same text, is there.
fn already_here(conn: &Connection, seen: &std::collections::HashSet<String>, f: &Found) -> bool {
    if !f.id.is_empty() && seen.contains(&f.id) {
        return true;
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

/// Adds the sticky notes to the board. Returns how many notes were made. Importing again only
/// brings the sticky notes not already here (no duplicates).
pub fn import(conn: &Connection, tmp: &std::path::Path) -> Result<usize, String> {
    let found = read_sticky_notes(tmp)?;
    import_found(conn, found)
}

fn import_found(conn: &Connection, found: Vec<Found>) -> Result<usize, String> {
    let mut seen: std::collections::HashSet<String> = db::get_setting(conn, "sticky_ids")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let found: Vec<Found> = found.into_iter().filter(|f| !already_here(conn, &seen, f)).collect();
    // All the notes or none (a failure halfway would leave half of them, and trying again
    // would add the first half twice).
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut made = 0;
    let mut inbox: Option<i64> = None;
    for (i, f) in found.into_iter().enumerate() {
        if !f.id.is_empty() {
            seen.insert(f.id.clone());
        }
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
            made += 1;
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
    tx.commit().map_err(|e| e.to_string())?;
    Ok(made)
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
        Found { id: id.into(), note: shape(lines.iter().map(|s| s.to_string()).collect()).unwrap() }
    }

    #[test]
    fn importing_again_adds_no_duplicates() {
        let path = std::env::temp_dir().join(format!("dobo-sticky-{}-{:?}.sqlite", db::now_ms(), std::thread::current().id()));
        let c = db::open(&path).unwrap();
        let first = vec![found("a1", &["Logitech", "Migrate"]), found("b2", &["Book of concord"])];
        assert_eq!(import_found(&c, first).unwrap(), 1);
        let again = vec![found("a1", &["Logitech", "Migrate"]), found("b2", &["Book of concord"]), found("c3", &["Hub", "Weekly"])];
        assert_eq!(import_found(&c, again).unwrap(), 1);
        assert_eq!(db::note_count(&c), 3); // Logitech, Inbox, Hub
        // Imported before ids were recorded: recognised by its title / Inbox text.
        let old = vec![found("", &["Hub", "Weekly"]), found("", &["Book of concord"])];
        assert_eq!(import_found(&c, old).unwrap(), 0);
        assert_eq!(db::inbox_count(&c), 1);
        drop(c);
        let _ = std::fs::remove_file(&path);
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
