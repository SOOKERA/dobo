// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
#[macro_use]
mod i18n;
mod backup;
mod browser;
mod clock;
mod consent;
mod db;
mod failure;
mod files;
mod fullscreen;
pub mod install;
mod look;
mod navkeys;
mod notify;
mod paths;
mod preview;
pub mod release;
mod recovery;
mod report;
mod reminders;
mod rename;
mod repeat;
mod sticky;
mod terms;
mod thumbs;
mod tray;
pub mod updates;
mod whatsnew;
mod window;

use db::{Everything, Item, Note, Picture};
use rusqlite::Connection;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
#[cfg(not(windows))]
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

/// The app's name in one place. (It was called ZToDo while it was being made: the bundle
/// identifier, com.sookera.ztodo, keeps that name on purpose, as it names the folder where the
/// notes are; see paths.rs.)
pub const APP_NAME: &str = match option_env!("DOBO_TEST_NAME") {
    // A renamed test copy (DOBO_TEST_NAME and DOBO_TEST_IDENTIFIER set at build time) for trying
    // Setup without touching the real Dobo's start-with-Windows entry or notes. Never in a release
    // (scripts/release.ps1 refuses to build with them).
    Some(n) => n,
    None => "Dobo",
};

pub const DEFAULT_TOGGLE: &str = "Ctrl+Shift+Space";
pub const DEFAULT_DUMP: &str = "Ctrl+Shift+D";
pub const DEFAULT_SEARCH: &str = "Ctrl+F";

pub struct Keys {
    pub toggle: String,
    pub dump: String,
}

/// Milliseconds on a clock that only goes forward (since Dobo started): never set back by a time
/// sync or a change of the PC's clock, unlike `db::now_ms`.
fn steady_ms() -> u64 {
    static START: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);
    START.elapsed().as_millis() as u64
}

/// The floating window has stayed still long enough to save where it is: `ms` since its last
/// move (`last`, on `steady_ms`) at `now`. A last move "after" now (it can't be) is seen as just
/// now: never a wait of hours, never a save in the middle of a drag.
fn still_for(last: u64, now: u64, ms: u64) -> bool {
    now.saturating_sub(last) >= ms
}

pub struct AppState {
    pub db: Mutex<Connection>,
    /// When the floating window last moved or resized (`steady_ms`); saved once it stays still.
    moves: AtomicU64,
    /// A saver is already waiting for the window to stay still (one at a time, not one per move).
    saving: AtomicBool,
    /// "Also open when the mouse reaches the screen edge".
    pub edge_open: AtomicBool,
    /// "Peek the note titles first" (with edge_open).
    pub edge_peek: AtomicBool,
    /// The narrow strip of titles is showing (not the whole panel).
    pub peeking: AtomicBool,
    /// "Keep the panel open" (the pin in the header).
    pub pinned: AtomicBool,
    /// The pinned panel stepped aside for a full-screen app (it comes back when that ends).
    pub fs_hidden: AtomicBool,
    /// The full-screen watcher is running.
    pub fs_watching: AtomicBool,
    pub keys: Mutex<Keys>,
    /// The window settings read all the time (window.rs `Prefs`).
    pub prefs: Mutex<window::Prefs>,
    /// The global shortcuts ("toggle", "dump") another app holds: they do nothing until changed
    /// (Settings › Keyboard says so).
    pub taken: Mutex<Vec<String>>,
}

type R<T> = Result<T, String>;

/// The Inbox note of a space (made when missing). It comes first on the board.
pub fn ensure_inbox(conn: &Connection, space: &str) -> R<i64> {
    if let Some(id) = db::inbox_of(conn, space)? {
        return Ok(id);
    }
    let mut note = db::create_note(conn, "Inbox", space, "soft", "Butter")?;
    let min: f64 = conn.query_row("SELECT COALESCE(MIN(sort), 0) FROM notes", [], |r| r.get(0)).unwrap_or(0.0);
    note.sort = min - 1.0;
    db::update_note(conn, &note)?;
    Ok(note.id)
}

/// After a change to the notes: a space whose Inbox just went (an undo archived, renamed or moved
/// it, a redo deleted it) has one again at once, the next note called "Inbox" or a new one. A
/// dump, the strip and the tray's count never find a space without its Inbox.
pub fn keep_inboxes(conn: &Connection, had: &[String]) -> R<()> {
    let now = db::inbox_spaces(conn)?;
    for space in had.iter().filter(|s| !now.contains(s)) {
        ensure_inbox(conn, space)?;
    }
    Ok(())
}

/// Saves a note (the space keeps its Inbox); also returns the lines to quiet when it was just
/// archived. All or nothing: never an Inbox gone without the one that takes its place.
fn save_note(conn: &Connection, note: &Note) -> R<(Note, Vec<i64>)> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let had = db::inbox_spaces(conn)?;
    let was_archived = db::get_note(conn, note.id).map(|n| n.archived).unwrap_or(false);
    let saved = db::update_note(conn, note)?;
    let put_away = if saved.archived && !was_archived { db::item_ids_of_note(conn, note.id) } else { vec![] };
    keep_inboxes(conn, &had)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((saved, put_away))
}

/// Deletes a note with its lines and pictures (the space keeps its Inbox, all or nothing); returns
/// its lines, to clear their notifications.
fn remove_note(conn: &Connection, id: i64) -> R<Vec<i64>> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let had = db::inbox_spaces(conn)?;
    let ids = db::item_ids_of_note(conn, id);
    db::delete_note(conn, id)?;
    keep_inboxes(conn, &had)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ids)
}

/// Short-lived files (Sticky Notes being read): see `paths::temp_dir`.
pub(crate) fn temp_dir(app: &AppHandle) -> R<std::path::PathBuf> {
    paths::temp_dir(app.path().app_cache_dir().map_err(|e| e.to_string()))
}

#[tauri::command]
fn load_all(window: tauri::WebviewWindow, state: State<AppState>) -> R<Everything> {
    // The main window has loaded and reached the notes: a new version on trial is up.
    if window.label() == "main" {
        updates::app_ready();
    }
    let mut all = db::load_all(&state.db.lock().unwrap())?;
    // (Their thumbs found after the notes are let go: a slow drive never holds them.)
    if let Ok(pictures) = paths::sub("pictures") {
        thumbs::attach(&mut all.pictures, &pictures);
    }
    Ok(all)
}

#[tauri::command]
fn create_note(state: State<AppState>, title: String, space: String, palette: String, color: String) -> R<Note> {
    db::create_note(&state.db.lock().unwrap(), &title, &space, &palette, &color)
}

/// Clears the notifications of these lines (their buttons would do nothing any more).
pub(crate) fn clear_items(app: &AppHandle, ids: &[i64]) {
    for id in ids {
        reminders::clear_ring(app, *id);
    }
}

#[tauri::command]
fn update_note(app: AppHandle, state: State<AppState>, note: Note) -> R<Note> {
    let (saved, put_away) = save_note(&state.db.lock().unwrap(), &note)?;
    // Archived: its reminders go quiet, notifications included.
    clear_items(&app, &put_away);
    Ok(saved)
}

/// Deleting keeps the picture files for a day, so Undo can bring everything back; files no
/// note uses any more are cleaned up at the next start.
#[tauri::command]
fn delete_note(app: AppHandle, state: State<AppState>, id: i64) -> R<()> {
    let ids = remove_note(&state.db.lock().unwrap(), id)?;
    clear_items(&app, &ids);
    Ok(())
}

/// Undo after a delete: notes, lines and pictures back as they were. (Off the main thread: a
/// big note's lines are written back in one go, which the windows don't wait for.)
#[tauri::command(async)]
fn restore(state: State<AppState>, notes: Vec<Note>, items: Vec<Item>, pictures: Vec<Picture>) -> R<()> {
    put_back(&state.db.lock().unwrap(), &notes, &items, &pictures)
}

/// Undo after a delete, all or nothing (the space keeps its Inbox).
fn put_back(conn: &Connection, notes: &[Note], items: &[Item], pictures: &[Picture]) -> R<()> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let had = db::inbox_spaces(conn)?;
    db::restore_rows(conn, notes, items, pictures)?;
    keep_inboxes(conn, &had)?;
    tx.commit().map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_notes(state: State<AppState>, ids: Vec<i64>) -> R<()> {
    db::reorder_notes(&mut state.db.lock().unwrap(), &ids)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn create_item(
    state: State<AppState>,
    note_id: i64,
    kind: String,
    text: String,
    due_at: Option<i64>,
    parent_id: Option<i64>,
    alarm: Option<String>,
    link: Option<serde_json::Value>,
    file: Option<String>,
) -> R<Item> {
    db::create_item(
        &state.db.lock().unwrap(),
        db::NewItem {
            note_id,
            kind: &kind,
            text: &text,
            due_at,
            parent_id,
            alarm: alarm.as_deref().unwrap_or("reminder"),
            link: link.as_ref(),
            file: file.as_deref(),
        },
    )
}

#[tauri::command]
fn update_item(app: AppHandle, window: tauri::Window, state: State<AppState>, item: Item) -> R<Item> {
    let (it, next, steps) = {
        let conn = state.db.lock().unwrap();
        let (it, next, steps) = db::update_item_full(&conn, &item)?;
        // (While the notes are held: a ring the clock makes right after, at the new time, is
        // never cleared by this.)
        if it.ring_state.is_empty() || it.done_at.is_some() {
            reminders::clear_ring(&app, it.id);
        }
        // A task ticked: its steps' reminders are paused, their notifications go.
        if it.done_at.is_some() {
            for s in &steps {
                reminders::clear_ring(&app, s.id);
            }
        }
        (it, next, steps)
    };
    // Steps paused or brought back with their task show it on the board.
    for s in steps {
        let _ = app.emit_to("main", "item-updated", s);
    }
    // A change made from the quick dump bar (a link's preview arriving) shows on the board.
    if window.label() != "main" {
        let _ = app.emit_to("main", "item-updated", it.clone());
    }
    // A repeating task was ticked: its next occurrence appears on the board.
    if let Some(n) = next {
        let _ = app.emit_to("main", "item-updated", n);
    }
    Ok(it)
}

#[tauri::command]
fn delete_item(app: AppHandle, state: State<AppState>, id: i64) -> R<()> {
    let ids = {
        let conn = state.db.lock().unwrap();
        let ids = db::subtree_ids(&conn, id);
        db::delete_item(&conn, id)?;
        ids
    };
    // The line and its steps: their notifications go too.
    clear_items(&app, &ids);
    Ok(())
}

/// A file's name as the clean-up compares it: the name alone, in lower case. The notes may name
/// the same file another way (another letter case, as when a portable Dobo is started through a
/// path typed differently, or slashes the other way): it is still in use.
fn file_key(path: &str) -> Option<String> {
    let name = path.rsplit(['\\', '/']).next()?.trim();
    (!name.is_empty()).then(|| name.to_lowercase())
}

/// Files in `folder` older than `before` that nothing uses (`used`: from `file_key`).
fn unused_files(folder: &std::path::Path, used: &std::collections::HashSet<String>, before: std::time::SystemTime) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return vec![] };
    entries
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.modified().is_ok_and(|t| t < before)))
        .map(|e| e.path())
        .filter(|p| file_key(&p.to_string_lossy()).is_some_and(|k| !used.contains(&k)))
        .collect()
}

/// Every picture file the notes and looks use (`file_key`s): note pictures, link previews'
/// pictures and the board's background pictures. Any read that fails is an error, and so is an
/// empty answer while the notes do have pictures: never "nothing is used", which would delete
/// every picture.
fn used_files(conn: &Connection) -> R<std::collections::HashSet<String>> {
    let pictures = db::picture_paths(conn)?;
    if pictures.is_empty() && db::picture_count(conn)? > 0 {
        return Err("the pictures couldn’t be listed".into());
    }
    let previews = db::preview_images(conn)?;
    let backgrounds = look::background_pictures(conn)?;
    Ok(pictures.into_iter().chain(previews.into_iter().map(|(_, p)| p)).chain(backgrounds).filter_map(|p| file_key(&p)).collect())
}

/// Picture files in Dobo's folder that no note (nor look) uses any more and are older than a day
/// (kept that long so Undo can bring a deleted note back with its pictures, or a look its
/// background picture; at start, nothing can be undone any more). When the notes can't be read
/// (locked, damaged), nothing is cleaned up this time; nor right after a choice on the recovery
/// screen (`after_recovery`: the notes just put back may be an older backup's).
fn purge_unused_pictures(app: &AppHandle, after_recovery: bool) {
    let Ok(dir) = paths::data_dir() else { return };
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    if let Err(e) = purge_at_start(&dir, &conn, after_recovery) {
        eprintln!("Pictures not cleaned up this time: {e}");
    }
}

/// `purge_unused_pictures` in the data folder `dir`, whose notes are `conn`: none right after a
/// choice on the recovery screen (`after_recovery`).
fn purge_at_start(dir: &std::path::Path, conn: &Connection, after_recovery: bool) -> R<()> {
    if after_recovery {
        return Ok(());
    }
    purge_in(dir, conn)
}

/// `purge_unused_pictures` in the data folder `dir`, whose notes are `conn`. The notes put aside
/// there (`broken-….sqlite`, recovery.rs) still name their pictures: those are kept while the
/// file is there, so it can still be recovered whole. One that can't be read at all: nothing is
/// cleaned up while it is there.
fn purge_in(dir: &std::path::Path, conn: &Connection) -> R<()> {
    let mut used = used_files(conn)?;
    for aside in recovery::put_aside_files(dir) {
        let read = Connection::open_with_flags(&aside, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| e.to_string())
            .and_then(|c| db::harden(&c).map(|_| c))
            .and_then(|c| used_files(&c));
        match read {
            Ok(names) => used.extend(names),
            Err(e) => return Err(format!("{} can’t be read ({e})", aside.display())),
        }
    }
    let day_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3600);
    for folder in ["pictures", "previews", "backgrounds"] {
        for p in unused_files(&dir.join(folder), &used, day_ago) {
            let _ = std::fs::remove_file(&p);
        }
    }
    // The thumbs of the pictures gone (never one whose picture is still used).
    for p in thumbs::unused(&dir.join("pictures").join(thumbs::DIR), &used, day_ago) {
        let _ = std::fs::remove_file(&p);
    }
    Ok(())
}

#[tauri::command]
fn set_setting(app: AppHandle, state: State<AppState>, key: String, value: serde_json::Value) -> R<()> {
    if key == "edge_open" {
        state.edge_open.store(value.as_bool().unwrap_or(false), Ordering::Relaxed);
        window::wake_edge_watch();
    }
    if key == "edge_peek" {
        state.edge_peek.store(value.as_bool().unwrap_or(false), Ordering::Relaxed);
    }
    let conn = state.db.lock().unwrap();
    db::set_setting(&conn, &key, &value)?;
    if window::PREF_KEYS.contains(&key.as_str()) {
        window::reload_prefs(&app, &conn);
    }
    if key == "dark_mode" {
        look::changed(&app, &conn);
    }
    if key == "language" {
        // Notifications, the tray menu and dialogs follow at once; so does the quick dump bar.
        let choice = value.as_str().unwrap_or("system");
        i18n::set(choice);
        // The installed copy: Dobo Setup and the uninstaller speak it too (the language in use,
        // also when following Windows).
        install::tell_setup(i18n::resolve(choice));
        drop(conn);
        refresh_tray(&app);
        let _ = app.emit("language", ());
    }
    Ok(())
}

/// The language setting ("system", "en", "fr") and the language Windows speaks ("en" / "fr").
#[derive(serde::Serialize)]
struct LanguageNow {
    choice: String,
    system: &'static str,
    /// Windows shows the time on a 12-hour clock ("3:00 PM").
    hour12: bool,
}

#[tauri::command]
fn language_now(app: AppHandle) -> LanguageNow {
    // (The recovery screen shows before the notes, and their setting, can be read: as Windows.)
    let choice = app.try_state::<AppState>().map(|s| i18n::choice(&s.db.lock().unwrap())).unwrap_or_else(|| "system".into());
    LanguageNow { choice, system: i18n::system(), hour12: clock::hour12() }
}

/// The tray's menu and tooltip, in the language and with the shortcuts as they are now.
fn tray_menu(app: &AppHandle, keys: &Keys) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", tr!("Open {}", "Ouvrir {}", APP_NAME), true, None::<&str>)?;
    let dump = MenuItem::with_id(app, "dump", tr!("+ Dump  ({})", "+ Déposer  ({})", i18n::keys(&keys.dump)), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr!("Quit {}", "Quitter {}", APP_NAME), true, None::<&str>)?;
    Menu::with_items(app, &[&open, &dump, &quit])
}

fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let keys = {
        let st = app.state::<AppState>();
        let k = st.keys.lock().unwrap();
        Keys { toggle: k.toggle.clone(), dump: k.dump.clone() }
    };
    if let Ok(menu) = tray_menu(app, &keys) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(format!("{APP_NAME}  ({})", i18n::keys(&keys.toggle))));
}

/// The dark-mode switch and whether it says dark right now (None: follow Windows).
#[tauri::command]
fn dark_mode_now(state: State<AppState>) -> look::DarkNow {
    look::dark_now(&state.db.lock().unwrap(), db::now_ms())
}

#[tauri::command]
fn inbox_for(state: State<AppState>, space: String) -> R<i64> {
    ensure_inbox(&state.db.lock().unwrap(), &space)
}

/// An Inbox made only to stand in for an old one sent back to the archive by an undo, in a space
/// that had none before that one came out: while it is still empty it goes, and the space is as
/// it was (the next line makes one again). Returns whether it went; with anything in it (a line
/// dumped meanwhile), it stays.
fn drop_stand_in(conn: &Connection, id: i64) -> R<bool> {
    use rusqlite::OptionalExtension;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let empty: bool = conn
        .query_row(
            "SELECT title = 'Inbox' AND archived = 0 AND NOT EXISTS (SELECT 1 FROM items WHERE note_id = ?1)
                AND NOT EXISTS (SELECT 1 FROM pictures WHERE note_id = ?1)
             FROM notes WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(false);
    if empty {
        db::delete_note(conn, id)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(empty)
}

#[tauri::command]
fn drop_empty_inbox(state: State<AppState>, id: i64) -> R<bool> {
    drop_stand_in(&state.db.lock().unwrap(), id)
}

/// How many Windows Sticky Notes an import would bring in (0: none, or all already here). Only
/// reads a copy; the first start asks with it, and imports only when told to.
#[tauri::command(async)]
fn count_sticky_notes(app: AppHandle, state: State<AppState>) -> R<usize> {
    let found = sticky::read(&temp_dir(&app)?.join("sticky-import"))?;
    Ok(sticky::count_new(&state.db.lock().unwrap(), &found))
}

/// Brings in the Windows Sticky Notes not already here. Returns how many came in.
#[tauri::command(async)]
fn import_sticky_notes(app: AppHandle, state: State<AppState>) -> R<usize> {
    let tmp = temp_dir(&app)?.join("sticky-import");
    // Read first, then added with the notes held (only for that).
    let found = sticky::read(&tmp)?;
    sticky::import_found(&state.db.lock().unwrap(), found)
}

#[tauri::command]
fn window_info(app: AppHandle) -> window::WindowInfo {
    window::info_with_width(&app)
}

#[tauri::command]
fn hide_panel(app: AppHandle) {
    window::hide(&app);
}

/// The docked panel's width, dragged by its inner edge or picked in Settings (live; `save`
/// remembers it for this screen). Returns the width used (clamped, maybe snapped).
#[tauri::command]
fn resize_docked(app: AppHandle, width: f64, snap: bool, save: bool) -> Option<f64> {
    window::resize_docked(&app, width, snap, save)
}

/// "Keep the panel open": while on, a docked panel also steps aside for full-screen apps.
#[tauri::command]
fn set_pinned(app: AppHandle, state: State<AppState>, on: bool) {
    state.pinned.store(on, Ordering::SeqCst);
    if !on {
        state.fs_hidden.store(false, Ordering::SeqCst);
    } else {
        fullscreen::watch(&app);
    }
}

/// After a window setting changed (mode, side, width, screen): place the window again.
#[tauri::command]
fn apply_window(app: AppHandle) {
    if app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false) {
        window::show(&app);
    }
}

#[tauri::command]
fn set_float_on_top(app: AppHandle, state: State<AppState>, on: bool) -> R<()> {
    {
        let conn = state.db.lock().unwrap();
        db::set_setting(&conn, "float_on_top", &serde_json::json!(on))?;
        window::reload_prefs(&app, &conn);
    }
    if let Some(w) = app.get_webview_window("main")
        && window::info(&app).mode == "floating" {
            let _ = w.set_always_on_top(on);
        }
    Ok(())
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<window::MonitorInfo> {
    window::list_monitors(&app)
}

// ---- Reminders ----

#[tauri::command]
fn answer_ring(app: AppHandle, item_id: i64, action: String) -> R<Item> {
    reminders::answer(&app, item_id, &action)
}

#[tauri::command]
fn tidy_answer(app: AppHandle, action: String) {
    reminders::tidy_answer(&app, &action);
}

/// The default rhythms, for Settings and the item menu.
#[tauri::command]
fn rhythm_presets() -> serde_json::Value {
    serde_json::json!({
        "gentle": reminders::Rhythm::preset("gentle"),
        "normal": reminders::Rhythm::preset("normal"),
        "insistent": reminders::Rhythm::preset("insistent"),
    })
}

// ---- Quick dump, links, files, pictures ----
//
// (`#[tauri::command(async)]`: a command that copies or writes files, or asks Windows to open
// one, runs off the main thread, so a slow drive or a big picture never freezes the windows,
// the tray and the shortcuts. Plain commands run on the main thread.)

#[tauri::command]
fn show_dump(app: AppHandle) {
    window::show_dump(&app);
}

#[tauri::command]
fn hide_dump(app: AppHandle) {
    window::hide_dump(&app);
}

/// A line dumped from the quick bar: into the Inbox of the space showing (Work when All).
/// The board is given just the new line, not everything read again: only when the space had
/// no Inbox (one was just made for it) does it read all again, to get that note too.
/// (Off the main thread, like every command that writes a little more: the windows, the tray
/// and the shortcuts never wait for the notes file.)
#[tauri::command(async)]
fn dump_item(app: AppHandle, state: State<AppState>, text: String, link: Option<serde_json::Value>, file: Option<String>) -> R<(Item, i64)> {
    let (item, waiting, made) = {
        let conn = state.db.lock().unwrap();
        let view = db::get_setting(&conn, "view").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        let space = if view == "personal" { "personal" } else { "work" };
        let made = db::inbox_of(&conn, space)?.is_none();
        let inbox = ensure_inbox(&conn, space)?;
        let item = db::create_item(
            &conn,
            db::NewItem { note_id: inbox, kind: "task", text: &text, due_at: None, parent_id: None, alarm: "reminder", link: link.as_ref(), file: file.as_deref() },
        )?;
        (item, db::inbox_count(&conn), made)
    };
    if made {
        let _ = app.emit_to("main", "reload", ());
    } else {
        let _ = app.emit_to("main", "item-updated", item.clone());
    }
    Ok((item, waiting))
}

/// Help › Report a problem: what a report would carry (reads Dobo's problem notes, off the
/// window's thread).
#[tauri::command]
async fn report_info() -> R<report::ReportInfo> {
    tauri::async_runtime::spawn_blocking(report::report_info).await.map_err(|e| e.to_string())
}

/// Sends the report as the window shows it (online: not before the terms are accepted). The
/// report's reference, or why it didn't go.
#[tauri::command]
async fn report_send(description: String, look: String, language: String, problems: Vec<report::Problem>) -> Result<String, report::SendFail> {
    consent::check().map_err(|m| report::SendFail::Refused { message: m })?;
    tauri::async_runtime::spawn_blocking(move || report::send(&description, &look, &language, &problems)).await.map_err(|_| report::SendFail::Server)?
}

/// Send by email: a new message to support@sookera.com, and only that.
#[tauri::command]
fn report_mail(app: AppHandle, url: String) -> R<()> {
    if !report::mail_link_ok(&url) {
        return Err(tr!("Only an email to support@sookera.com opens from here.", "Seul un e-mail à support@sookera.com s’ouvre d’ici."));
    }
    app.opener().open_url(url, None::<&str>).map_err(|_| {
        tr!(
            "Couldn’t open your email app. Use Copy details and paste them into an email to support@sookera.com.",
            "Impossible d’ouvrir votre messagerie. Utilisez Copier les détails et collez-les dans un e-mail à support@sookera.com."
        )
    })
}

#[tauri::command]
async fn fetch_preview(url: String) -> R<preview::Preview> {
    // Online: not before the terms of use are accepted (consent.rs).
    consent::check()?;
    let dir = paths::sub("previews")?;
    tauri::async_runtime::spawn_blocking(move || preview::fetch(&url, &dir)).await.map_err(|e| e.to_string())?
}

/// Only the link of a line (the dump bar's preview, which arrives seconds later): the rest of
/// the line is left as it is now, so a move or a tick made meanwhile is not undone.
#[tauri::command]
fn set_item_link(app: AppHandle, state: State<AppState>, id: i64, link: Option<serde_json::Value>) -> R<Item> {
    let it = db::set_link(&state.db.lock().unwrap(), id, link.as_ref())?;
    let _ = app.emit_to("main", "item-updated", it.clone());
    Ok(it)
}

/// A file card was clicked: documents open with their app; a program or script is never
/// started from here, it is shown in its folder instead. Only files on this computer, or on a
/// computer allowed (files.rs: the card asks once per computer).
#[tauri::command(async)]
fn open_path(app: AppHandle, path: String) -> R<()> {
    let data = paths::data_dir().ok();
    let hosts = files::hosts_allowed(&app.state::<AppState>().db.lock().unwrap());
    let couldnt = |e: tauri_plugin_opener::Error| tr!("Windows couldn’t open it: {}", "Windows n’a pas pu l’ouvrir : {}", e);
    match files::decide(&path, data.as_deref(), &hosts)? {
        files::Open::Reveal(p) => app.opener().reveal_item_in_dir(p).map_err(couldnt),
        files::Open::Document(p) => app.opener().open_path(p.to_string_lossy(), None::<&str>).map_err(couldnt),
    }
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> R<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(tr!("Only web links open from here.", "Seuls les liens web s’ouvrent d’ici."));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

fn picture_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().trim_start_matches('.') {
        "png" => "png",
        "gif" => "gif",
        "webp" => "webp",
        "bmp" => "bmp",
        _ => "jpg",
    }
}

fn new_picture_path(ext: &str) -> R<std::path::PathBuf> {
    let dir = paths::sub("pictures")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    static N: AtomicU64 = AtomicU64::new(0);
    Ok(dir.join(format!("{}-{}.{}", db::now_ms(), N.fetch_add(1, Ordering::Relaxed), picture_ext(ext))))
}

/// A pasted picture (the bytes come as the request body).
#[tauri::command(async)]
fn save_picture(app: AppHandle, window: tauri::Window, state: State<AppState>, request: tauri::ipc::Request) -> R<Picture> {
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(String::from);
    let note_id: i64 = header("note-id").and_then(|v| v.parse().ok()).ok_or("note-id missing")?;
    let item_id: Option<i64> = header("item-id").and_then(|v| v.parse().ok());
    let ext = header("ext").unwrap_or_else(|| "png".into());
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("no picture".into()) };
    if bytes.len() as u64 > PICTURE_MAX {
        return Err(too_big());
    }
    let path = new_picture_path(&ext)?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let p = db::add_picture(&state.db.lock().unwrap(), note_id, item_id, &path.to_string_lossy())?;
    // (From the quick dump bar: the board is given just the new picture.)
    if window.label() != "main" {
        let _ = app.emit_to("main", "picture-added", p.clone());
    }
    Ok(p)
}

/// The biggest picture kept with a note, pasted or dropped.
const PICTURE_MAX: u64 = 40_000_000;
fn too_big() -> String {
    tr!(
        "That picture is too big to keep in a note (40 MB at most). Try a smaller copy or a screenshot of it.",
        "Cette image est trop lourde pour une note (40 Mo au plus). Essayez une copie plus petite ou une capture d’écran."
    )
}

/// A picture file dropped on a note: copied next to the notes.
#[tauri::command(async)]
fn add_picture_file(state: State<AppState>, note_id: i64, item_id: Option<i64>, path: String) -> R<Picture> {
    let src = std::path::Path::new(&path);
    let meta = std::fs::metadata(src).map_err(|_| tr!("That picture can’t be read.", "Cette image ne peut pas être lue."))?;
    if !meta.is_file() {
        return Err(tr!("That isn’t a picture file.", "Ce n’est pas un fichier image."));
    }
    if meta.len() > PICTURE_MAX {
        return Err(too_big());
    }
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dest = new_picture_path(ext)?;
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    db::add_picture(&state.db.lock().unwrap(), note_id, item_id, &dest.to_string_lossy())
}

/// The thread running this at a lower priority than the windows' own, until it is dropped (a
/// thread of the pool, given back as it was).
struct BelowNormal;
impl BelowNormal {
    fn now() -> Self {
        #[cfg(windows)]
        // SAFETY: the calling thread's own pseudo handle, always valid.
        unsafe {
            use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }
        BelowNormal
    }
}
impl Drop for BelowNormal {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: as above.
        unsafe {
            use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_NORMAL};
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL);
        }
    }
}

/// The board-size copy of a picture of the pictures folder (thumbs.rs), made now unless it was
/// already; None when the picture is small enough as it is. One at a time, at a lower priority
/// than the windows: the board asks for them in calm moments, a few at a time, and when a picture
/// is added.
#[tauri::command]
async fn make_thumb(path: String) -> R<Option<thumbs::Thumb>> {
    let pictures = paths::sub("pictures")?;
    if !thumbs::is_own_picture(&path, &pictures) {
        return Err("not one of Dobo’s pictures".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
        let _calm = BelowNormal::now();
        thumbs::make(std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_picture(state: State<AppState>, id: i64) -> R<()> {
    db::delete_picture(&state.db.lock().unwrap(), id)?;
    Ok(())
}

// ---- Peeking titles, a note saved as a picture, backups ----

/// A title clicked in the peeking strip: the whole panel opens on that note.
#[tauri::command]
fn open_note(app: AppHandle, note_id: i64) {
    window::show(&app);
    let _ = app.emit_to("main", "focus-note", note_id);
}

#[tauri::command]
fn end_peek(app: AppHandle) {
    window::end_peek(&app);
}

/// A note's title as a file name Windows takes: letters, digits, spaces and dashes only, at most
/// 80 characters, no trailing dots or spaces, never a device name (CON, NUL, COM1…, which
/// Windows treats as devices whatever the extension), never empty.
fn file_stem_for(title: &str) -> String {
    let safe: String = title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { ' ' }).take(80).collect();
    let safe = safe.trim().trim_end_matches(['.', ' ']).to_string();
    if safe.is_empty() {
        return "note".into();
    }
    let upper = safe.to_ascii_uppercase();
    // (By letters, not bytes: Windows reserves COM¹ COM² COM³ and LPT¹ LPT² LPT³ too.)
    let letters: Vec<char> = upper.chars().collect();
    let device = ["CON", "PRN", "AUX", "NUL"].contains(&upper.as_str())
        || (letters.len() == 4 && (upper.starts_with("COM") || upper.starts_with("LPT")) && (letters[3].is_ascii_digit() || "¹²³".contains(letters[3])));
    if device { format!("{safe}_") } else { safe }
}

/// "Save as a picture": into Pictures › Dobo. Returns the file.
#[tauri::command(async)]
fn save_note_image(app: AppHandle, request: tauri::ipc::Request) -> R<String> {
    let raw = request.headers().get("name").and_then(|v| v.to_str().ok()).unwrap_or("note").to_string();
    // The page sends the name URL-encoded (headers are ASCII only).
    let mut bytes_name = Vec::with_capacity(raw.len());
    let rb = raw.as_bytes();
    let mut i = 0;
    while i < rb.len() {
        if rb[i] == b'%'
            && i + 2 < rb.len()
            && let Ok(b) = u8::from_str_radix(&raw[i + 1..i + 3], 16)
        {
            bytes_name.push(b);
            i += 3;
            continue;
        }
        bytes_name.push(rb[i]);
        i += 1;
    }
    let name = String::from_utf8_lossy(&bytes_name).to_string();
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("no picture".into()) };
    // (Drawn by the page: far below this; a limit all the same.)
    if bytes.len() > 200_000_000 {
        return Err(tr!("That picture is too big to save.", "Cette image est trop lourde pour être enregistrée."));
    }
    let dir = app.path().picture_dir().or_else(|_| app.path().download_dir()).map_err(|e| e.to_string())?.join(APP_NAME);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe = file_stem_for(&name);
    let mut path = dir.join(format!("{safe}.png"));
    let mut k = 2;
    while path.exists() {
        path = dir.join(format!("{safe} {k}.png"));
        k += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

/// Shows a file in its folder (Explorer / Finder / the file manager): a file card's, or a note
/// saved as a picture in Pictures › Dobo (also when Pictures is on a server).
#[tauri::command(async)]
fn reveal(app: AppHandle, path: String) -> R<()> {
    let pictures = app.path().picture_dir().ok().map(|d| d.join(APP_NAME));
    let hosts = files::hosts_allowed(&app.state::<AppState>().db.lock().unwrap());
    files::allowed(&path, paths::data_dir().ok().as_deref(), pictures.as_deref(), &hosts).map_err(files::Refused::said)?;
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn backup_now(app: AppHandle) -> R<backup::BackupInfo> {
    backup::make(&app, "manual")
}

#[tauri::command(async)]
fn backup_list(app: AppHandle) -> R<Vec<backup::BackupInfo>> {
    backup::list(&app)
}

#[tauri::command(async)]
fn backup_restore(app: AppHandle, path: String) -> R<()> {
    backup::restore(&app, &path)
}

/// Settings › Backups: whether the last automatic backup worked.
#[tauri::command]
fn backup_status(state: State<AppState>) -> backup::Status {
    backup::status(&state.db.lock().unwrap())
}

#[tauri::command(async)]
fn backup_folder(app: AppHandle) -> R<String> {
    backup::folder(&app).map(|p| p.to_string_lossy().into())
}

/// Settings › About: installed or portable, and where the notes are.
#[derive(serde::Serialize)]
struct DataInfo {
    kind: paths::Kind,
    dir: String,
    kept_existing: bool,
    fallback: bool,
    version: String,
}

#[tauri::command]
fn data_info(app: AppHandle) -> R<DataInfo> {
    let p = paths::place().ok_or("The data folder isn’t known yet.")?;
    Ok(DataInfo { kind: p.kind, dir: p.dir.to_string_lossy().into(), kept_existing: p.kept_existing, fallback: p.fallback, version: app.package_info().version.to_string() })
}

/// Opens the data folder itself (Explorer / Finder / the file manager).
#[tauri::command(async)]
fn open_data_folder(app: AppHandle) -> R<()> {
    let dir = paths::data_dir()?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn release_picture(state: State<AppState>, id: i64, note_id: i64) -> R<()> {
    db::release_picture(&state.db.lock().unwrap(), id, note_id)
}

// ---- Shortcuts and start with the computer ----

fn parse_keys(accel: &str) -> R<Shortcut> {
    Shortcut::from_str(accel).map_err(|_| tr!("“{}” isn’t a shortcut Dobo understands.", "« {} » n’est pas un raccourci que Dobo comprend.", i18n::keys(accel)))
}

/// Changes one of the shortcuts ("toggle", "dump" or "search"); refuses ones already used by
/// Dobo or by another app.
#[tauri::command]
fn set_shortcut(app: AppHandle, state: State<AppState>, which: String, accel: String) -> R<()> {
    let new = parse_keys(&accel)?;
    if menu_key(&new) {
        return Err(tr!(
            "{} is Windows’ menu key: it opens the menu in every app. Try another one.",
            "{} est la touche de menu de Windows : elle ouvre le menu dans toutes les applis. Essayez-en un autre.",
            i18n::keys(&accel)
        ));
    }
    if fires_while_typing(&new) {
        return Err(tr!(
            "Add Ctrl, Alt or Win so it doesn’t fire while you type (Shift alone isn’t enough).",
            "Ajoutez Ctrl, Alt ou Win pour qu’il ne se déclenche pas quand vous écrivez (Maj seule ne suffit pas)."
        ));
    }
    let (toggle, dump, search) = {
        let k = state.keys.lock().unwrap();
        let conn = state.db.lock().unwrap();
        let search = db::get_setting(&conn, "shortcut_search").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| DEFAULT_SEARCH.into());
        (k.toggle.clone(), k.dump.clone(), search)
    };
    let names = [
        ("toggle", toggle.as_str(), tr!("open or hide Dobo", "ouvrir ou masquer Dobo")),
        ("dump", dump.as_str(), tr!("the quick dump bar", "la barre de dépôt rapide")),
        ("search", search.as_str(), tr!("search", "la recherche")),
    ];
    // (The fixed keys inside the app, then the other shortcuts.)
    let fixed = IN_APP_KEYS.iter().map(|(keys, what)| ("", keys.as_str(), in_app_what(*what)));
    for (name, keys, what) in fixed.chain(names) {
        if name != which && parse_keys(keys).map(|s| s == new).unwrap_or(false) {
            return Err(tr!("Dobo already uses {} for {}.", "Dobo utilise déjà {} pour {}.", i18n::keys(&accel), what));
        }
    }
    if which == "search" {
        return db::set_setting(&state.db.lock().unwrap(), "shortcut_search", &serde_json::json!(accel));
    }
    let old = if which == "toggle" { toggle } else if which == "dump" { dump } else { return Err(tr!("Unknown shortcut.", "Raccourci inconnu.")) };
    let gs = app.global_shortcut();
    if let Ok(o) = parse_keys(&old) {
        let _ = gs.unregister(o);
    }
    if gs.register(new).is_err() {
        if let Ok(o) = parse_keys(&old) {
            let _ = gs.register(o);
        }
        return Err(tr!("Another app already uses {}. Try another one.", "Une autre appli utilise déjà {}. Essayez-en un autre.", i18n::keys(&accel)));
    }
    {
        let mut k = state.keys.lock().unwrap();
        if which == "toggle" { k.toggle = accel.clone() } else { k.dump = accel.clone() }
    }
    state.taken.lock().unwrap().retain(|w| *w != which);
    db::set_setting(&state.db.lock().unwrap(), &format!("shortcut_{which}"), &serde_json::json!(accel))?;
    refresh_tray(&app);
    Ok(())
}

/// Settings › Keyboard: the global shortcuts another app holds ("toggle", "dump").
#[tauri::command]
fn shortcuts_taken(state: State<AppState>) -> Vec<String> {
    state.taken.lock().unwrap().clone()
}

/// The global shortcuts that aren't Dobo's right now (another app holds them), and the board told
/// (once the page listens: at start it doesn't yet).
fn note_taken_shortcuts(app: &AppHandle) {
    let gs = app.global_shortcut();
    let state = app.state::<AppState>();
    let (toggle, dump) = {
        let k = state.keys.lock().unwrap();
        (k.toggle.clone(), k.dump.clone())
    };
    let taken: Vec<(String, String)> = [("toggle", toggle), ("dump", dump)]
        .into_iter()
        .filter(|(_, keys)| parse_keys(keys).is_ok_and(|s| !gs.is_registered(s)))
        .map(|(w, k)| (w.to_string(), k))
        .collect();
    let before = std::mem::replace(&mut *state.taken.lock().unwrap(), taken.iter().map(|(w, _)| w.clone()).collect());
    let new = newly_taken(&before, &taken);
    if !new.is_empty() {
        to_page(app, "shortcut-taken", serde_json::Value::Array(new));
    }
}

/// "Start with Windows" (Windows: install.rs writes the entry itself, its program in quotes;
/// tauri-plugin-autostart wrote it unquoted).
/// The shortcuts held by another app now that weren't before (each said once, not at every look).
fn newly_taken(before: &[String], now: &[(String, String)]) -> Vec<serde_json::Value> {
    now.iter().filter(|(w, _)| !before.contains(w)).map(|(w, k)| serde_json::json!({ "which": w, "keys": k })).collect()
}

#[tauri::command]
fn get_autostart(_app: AppHandle) -> bool {
    #[cfg(windows)]
    return install::autostart_enabled();
    #[cfg(not(windows))]
    _app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(_app: AppHandle, on: bool) -> R<()> {
    #[cfg(windows)]
    return if on { install::enable_autostart() } else { install::disable_autostart() };
    #[cfg(not(windows))]
    {
        let a = _app.autolaunch();
        if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
    }
}

// ---- Terms of use and Dobo's own updates ----

/// "Agree and continue" on the terms (TERMS.md): saved at once, and Dobo may go online from now
/// on (update checks, link previews), no restart.
/// Gives the date saved (the terms' "Last updated" date, as Dobo Setup saves it).
#[tauri::command]
fn accept_terms(state: State<AppState>) -> R<String> {
    let date = terms::version().ok_or("Dobo’s terms of use have no date.")?;
    db::set_setting(&state.db.lock().unwrap(), "terms_accepted", &serde_json::json!(date))?;
    consent::open();
    Ok(date)
}

/// "Quit Dobo" on the terms.
#[tauri::command]
fn quit_app(app: AppHandle) {
    // (From the recovery screen, the board never started: nothing to hide or save.)
    if app.try_state::<AppState>().is_some() {
        window::hide(&app);
    }
    app.exit(0);
}

// ---- The recovery screen (recovery.rs): the notes didn't open at start ----

/// What the recovery screen shows; None: the notes are open (the board shows).
#[tauri::command]
fn recovery_state(app: AppHandle) -> Option<recovery::Problem> {
    let r = app.try_state::<recovery::Recovery>()?;
    // (The window has loaded and works: a new version on trial isn't what went wrong.)
    updates::app_ready();
    match &*r.0.lock().unwrap_or_else(|e| e.into_inner()) {
        recovery::Stage::Waiting(p) => Some(p.clone()),
        _ => None,
    }
}

/// A choice on the recovery screen, done (off the main thread: a backup may be on a slow drive).
/// None: the notes are open now (`recovery_continue` starts the board); else what still stands in
/// the way. Err: it couldn't be done, nothing was changed.
#[tauri::command(async)]
fn recovery_choose(app: AppHandle, choice: recovery::Choice) -> R<Option<recovery::Problem>> {
    let r = app.try_state::<recovery::Recovery>().ok_or_else(|| tr!("The notes are open already.", "Les notes sont déjà ouvertes."))?;
    let mut stage = r.0.lock().unwrap_or_else(|e| e.into_inner());
    let recovery::Stage::Waiting(p) = &mut *stage else { return Ok(None) };
    let data = paths::data_dir()?;
    match recovery::choose(&data, p, choice, db::now_ms())? {
        recovery::Look::Open(conn) => {
            let kept = p.kept_as.clone();
            *stage = recovery::Stage::Ready(conn, kept);
            Ok(None)
        }
        recovery::Look::Trouble(next) => {
            *stage = recovery::Stage::Waiting(next.clone());
            Ok(Some(next))
        }
    }
}

/// The notes are open (`recovery_choose`): the board starts, as at any start (on the main thread,
/// where windows and the tray are made). An error then is said in a box, as at any start.
/// Returns the name the damaged file was put aside as, if it was (the board says so).
#[tauri::command(async)]
fn recovery_continue(app: AppHandle) -> R<Option<String>> {
    let r = app.try_state::<recovery::Recovery>().ok_or_else(|| tr!("The notes are open already.", "Les notes sont déjà ouvertes."))?;
    let (conn, kept) = {
        let mut stage = r.0.lock().unwrap_or_else(|e| e.into_inner());
        match std::mem::replace(&mut *stage, recovery::Stage::Started) {
            recovery::Stage::Ready(conn, kept) => (conn, kept),
            other => {
                *stage = other;
                return Err(tr!("The notes aren’t open yet.", "Les notes ne sont pas encore ouvertes."));
            }
        }
    };
    let (done, wait) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Err(e) = open_board(&handle, conn, true) {
            tauri_plugin_single_instance::destroy(&handle);
            failure::startup_failed(&e.to_string());
            failure::exiting();
            std::process::exit(1);
        }
        let _ = done.send(());
    })
    .map_err(|e| e.to_string())?;
    wait.recv().map_err(|e| e.to_string())?;
    Ok(kept)
}

/// Dobo's own update: where it stands (Settings › Updates).
#[tauri::command]
fn update_status() -> updates::View {
    updates::view()
}

/// "Check for updates": a newer Dobo is downloaded and checked, ready for the next start.
#[tauri::command]
async fn update_check(app: AppHandle) -> updates::View {
    // Not before the terms of use are accepted: what is known already, nothing asked.
    if !consent::ok() {
        return updates::view();
    }
    let _ = app.emit_to("main", "update-status", updates::View { busy: true, ..updates::view() });
    let view = tauri::async_runtime::spawn_blocking(updates::check).await.unwrap_or_else(|_| updates::view());
    let _ = app.emit_to("main", "update-status", view.clone());
    view
}

/// "Restart now": quits and starts the new version (in the tray when the window was hidden).
#[tauri::command]
fn update_restart(app: AppHandle) -> R<()> {
    let hidden = app.get_webview_window("main").map(|w| !w.is_visible().unwrap_or(true)).unwrap_or(true);
    updates::restart(hidden)?;
    window::hide(&app);
    app.exit(0);
    Ok(())
}

/// The automatic check: once a day (on unless turned off in Settings › Updates), first a little
/// after start, then, for a Dobo left running for days, whenever a day has passed, looked at
/// every hour. Only a check that worked counts for the day: with no network yet after the PC
/// woke up, it's tried again the next hour. Only once the terms are accepted.
fn start_update_checks(app: AppHandle) {
    // A development run or a test copy never looks (updates::enabled: "Check for updates" says
    // it never updates itself).
    if !updates::enabled() {
        return;
    }
    consent::after(move || {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(20));
            let mut first = true;
            loop {
                let auto = {
                    let st = app.state::<AppState>();
                    let conn = st.db.lock().unwrap();
                    db::get_setting(&conn, updates::AUTO_SETTING).and_then(|v| v.as_bool()).unwrap_or(true)
                };
                if auto && updates::due() {
                    let (view, _worked) = updates::auto_check();
                    let _ = app.emit_to("main", "update-status", view);
                } else if first {
                    // Already checked today: still tell the window about an update waiting.
                    let _ = app.emit_to("main", "update-status", updates::view());
                }
                first = false;
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        });
    });
}

// ---- `ztodo:` links (notification buttons, a second start) ----

fn query(q: &str, key: &str) -> Option<String> {
    q.split('&').find_map(|kv| kv.strip_prefix(&format!("{key}=")).map(String::from))
}

/// What a notification asks of the board, kept while the page isn't listening yet (Dobo started
/// by a click on one of its notifications: the click comes before the page has loaded). The
/// page takes them once it listens (`page_ready`).
static PAGE: Mutex<(bool, Vec<(String, serde_json::Value)>)> = Mutex::new((false, Vec::new()));

/// Tells the board, now or as soon as it listens.
pub(crate) fn to_page(app: &AppHandle, event: &str, payload: serde_json::Value) {
    let mut page = PAGE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(payload) = keep_for_page(&mut page, event, payload) {
        let _ = app.emit_to("main", event, payload);
    }
}

/// Kept while the page doesn't listen yet; once it does, handed back to be sent now.
fn keep_for_page(page: &mut (bool, Vec<(String, serde_json::Value)>), event: &str, payload: serde_json::Value) -> Option<serde_json::Value> {
    if page.0 {
        return Some(payload);
    }
    page.1.push((event.to_string(), payload));
    None
}

/// The board listens now: what was asked of it before, in order.
#[tauri::command]
fn page_ready() -> Vec<(String, serde_json::Value)> {
    take_for_page(&mut PAGE.lock().unwrap_or_else(|e| e.into_inner()))
}

fn take_for_page(page: &mut (bool, Vec<(String, serde_json::Value)>)) -> Vec<(String, serde_json::Value)> {
    page.0 = true;
    std::mem::take(&mut page.1)
}

pub fn handle_url(app: &AppHandle, url: &str) {
    let rest = url.trim().trim_start_matches(&format!("{}:", notify::SCHEME)).trim_start_matches('/');
    let (path, q) = rest.split_once('?').unwrap_or((rest, ""));
    let path = path.trim_end_matches('/');
    // Only links from Dobo's own notifications (they carry this install's secret) may change
    // anything; any other `ztodo:` link (a web page, a typed one) just opens Dobo.
    if !notify::token_ok(query(q, "t").as_deref()) {
        window::show(app);
        return;
    }
    match path {
        "ring" => {
            if let (Some(id), Some(a)) = (query(q, "item").and_then(|v| v.parse::<i64>().ok()), query(q, "a")) {
                let _ = reminders::answer(app, id, &a);
            }
        }
        "open" => {
            window::show(app);
            if let Some(id) = query(q, "item").and_then(|v| v.parse::<i64>().ok()) {
                to_page(app, "focus-item", serde_json::json!(id));
                // "Later…" from a notification: the Now card opens its choices.
                if query(q, "later").is_some() {
                    to_page(app, "later-item", serde_json::json!(id));
                }
            }
        }
        "lookback" => {
            window::show(app);
            // The day it looks back on (a notification clicked the next day opens that day's).
            let day = query(q, "day").and_then(|v| v.parse::<i64>().ok());
            to_page(app, "open-lookback", serde_json::json!(day));
        }
        "tidy" => reminders::tidy_answer(app, &query(q, "a").unwrap_or_else(|| "now".into())),
        _ => window::show(app),
    }
}

fn first_run(app: &AppHandle) -> R<()> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    first_setup(&conn)
}

/// The board's first start: its Inbox. Nothing is brought in from Windows Sticky Notes here: the
/// welcome (FirstRun) asks first, and imports only if told to.
fn first_setup(conn: &Connection) -> R<()> {
    if db::get_setting(conn, "first_run_done").is_none() {
        ensure_inbox(conn, "work")?;
        db::set_setting(conn, "first_run_done", &serde_json::json!(db::now_ms()))?;
    }
    Ok(())
}

/// A shortcut with no Ctrl, Alt or Win would be taken from every app as people use it: a key alone
/// (A, Space, F5, an arrow), or Shift with a key that types or edits (Shift+A is a capital A,
/// Shift+Left selects, Shift+Enter starts a new line). Shift with a key that never types (F1…F24,
/// Pause, Scroll Lock) is fine.
fn fires_while_typing(s: &Shortcut) -> bool {
    use tauri_plugin_global_shortcut::{Code, Modifiers};
    if s.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER | Modifiers::META) {
        return false;
    }
    let never_types = matches!(
        s.key,
        Code::F1 | Code::F2 | Code::F3 | Code::F4 | Code::F5 | Code::F6 | Code::F7 | Code::F8 | Code::F9 | Code::F10 | Code::F11 | Code::F12
            | Code::F13 | Code::F14 | Code::F15 | Code::F16 | Code::F17 | Code::F18 | Code::F19 | Code::F20 | Code::F21 | Code::F22 | Code::F23
            | Code::F24 | Code::Pause | Code::ScrollLock
    );
    !(s.mods.contains(Modifiers::SHIFT) && never_types)
}

/// Windows' menu key: Shift+F10, or the Menu key itself (no shortcut can be read with it
/// today, `parse_keys`). Taken by a shortcut, it would stop opening the right-click menu, in
/// every app and in Dobo.
fn menu_key(s: &Shortcut) -> bool {
    use tauri_plugin_global_shortcut::{Code, Modifiers};
    s.key == Code::ContextMenu || (s.key == Code::F10 && s.mods == Modifiers::SHIFT)
}

/// Why a shortcut saved earlier can't be kept at start, if it can't: Windows' menu key
/// ("menu"), one that fires while typing ("typing"), or one of the in-app keys ("in_app").
fn refused(accel: &str) -> Option<&'static str> {
    let parsed = parse_keys(accel).ok();
    if parsed.as_ref().is_some_and(menu_key) {
        Some("menu")
    } else if parsed.as_ref().is_some_and(fires_while_typing) {
        Some("typing")
    } else if in_app_key(accel) {
        Some("in_app")
    } else {
        None
    }
}

/// The keys search goes to when it can't keep its own, in order: the first that neither global
/// shortcut uses (one always is, there being only two).
const SEARCH_KEYS: [&str; 3] = [DEFAULT_SEARCH, "Ctrl+Shift+F", "Ctrl+Alt+F"];

/// What one of Dobo's fixed keys does inside its window (said when a shortcut would take it).
#[derive(Clone, Copy)]
enum InApp {
    Style,
    Settings,
    Undo,
    Size,
    Move,
    BackForward,
}

fn in_app_what(w: InApp) -> String {
    match w {
        InApp::Style => tr!("the Style panel", "le panneau Style"),
        InApp::Settings => tr!("Settings", "les Paramètres"),
        InApp::Undo => tr!("Undo and Redo", "Annuler et Rétablir"),
        InApp::Size => tr!("the note size", "la taille des notes"),
        InApp::Move => tr!("moving a line up or down", "monter ou descendre une ligne"),
        InApp::BackForward => tr!("Back and Forward", "Retour et Suivant"),
    }
}

/// Keys Dobo uses inside its own window. A global shortcut on one of them would win over it
/// everywhere (the panel would hide instead of opening Style, Ctrl+Z would stop undoing in every
/// app), and the search shortcut on one would take it from the app (undo in a field, the size,
/// moving a line). One list with the page's key handlers: src/lib/in-app-keys.json (its own test,
/// keyboard.test.ts, checks each is a key the page uses).
static IN_APP_KEYS: std::sync::LazyLock<Vec<(String, InApp)>> = std::sync::LazyLock::new(|| {
    let list: serde_json::Value = serde_json::from_str(include_str!("../../src/lib/in-app-keys.json")).expect("in-app-keys.json");
    list["keys"]
        .as_array()
        .expect("in-app-keys.json: keys")
        .iter()
        .map(|k| {
            let what = match k["what"].as_str() {
                Some("style") => InApp::Style,
                Some("settings") => InApp::Settings,
                Some("undo") => InApp::Undo,
                Some("size") => InApp::Size,
                Some("move") => InApp::Move,
                Some("back") => InApp::BackForward,
                other => panic!("in-app-keys.json: unknown what {other:?}"),
            };
            (k["accel"].as_str().expect("in-app-keys.json: accel").to_string(), what)
        })
        .collect()
});

/// Is this one of the in-app keys (however it is written)?
fn in_app_key(accel: &str) -> bool {
    let Ok(s) = parse_keys(accel) else { return false };
    IN_APP_KEYS.iter().any(|(k, _)| parse_keys(k).ok() == Some(s))
}

/// The global shortcuts as saved. One set before the in-app keys were fixed (Ctrl+Shift+S / P,
/// Ctrl+,), or one that would fire while typing (Shift+A, saved before that was refused), goes
/// back to its default, and the app is told once (`shortcut_moved`, with why) so it can say so.
fn stored_keys(conn: &Connection) -> Keys {
    let get = |k: &str, d: &str| db::get_setting(conn, k).and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| d.into());
    let mut keys = Keys { toggle: get("shortcut_toggle", DEFAULT_TOGGLE), dump: get("shortcut_dump", DEFAULT_DUMP) };
    let mut moved = Vec::new();
    for (which, cur, default) in [("toggle", &mut keys.toggle, DEFAULT_TOGGLE), ("dump", &mut keys.dump, DEFAULT_DUMP)] {
        if let Some(why) = refused(cur) {
            moved.push(serde_json::json!({ "which": which, "from": cur.clone(), "to": default, "why": why }));
            *cur = default.to_string();
            let _ = db::set_setting(conn, &format!("shortcut_{which}"), &serde_json::json!(default));
        }
    }
    // A default it went back to may be the key the other shortcut uses: that one goes back to
    // its own default too (the two defaults differ), so they never share a key.
    let same = |a: &str, b: &str| matches!((parse_keys(a), parse_keys(b)), (Ok(x), Ok(y)) if x == y);
    if !moved.is_empty() && same(&keys.toggle, &keys.dump) {
        let other = if moved[0]["which"] == "toggle" { ("dump", &mut keys.dump, DEFAULT_DUMP) } else { ("toggle", &mut keys.toggle, DEFAULT_TOGGLE) };
        let (which, cur, default) = other;
        moved.push(serde_json::json!({ "which": which, "from": cur.clone(), "to": default, "why": "in_app" }));
        *cur = default.to_string();
        let _ = db::set_setting(conn, &format!("shortcut_{which}"), &serde_json::json!(default));
    }
    // The search shortcut (inside the app only) the same way: saved before more keys were Dobo's
    // own (undo, the size keys, Back / Forward, moving a line), it would take them from the app.
    // It goes to Ctrl+F, or, when a global shortcut is on Ctrl+F (which would win: search would
    // never open with its key), to the next of `SEARCH_KEYS` that neither uses.
    if let Some(search) = db::get_setting(conn, "shortcut_search").and_then(|v| v.as_str().map(String::from))
        && let Some(why) = refused(&search)
    {
        let to = SEARCH_KEYS.into_iter().find(|k| !same(k, &keys.toggle) && !same(k, &keys.dump)).unwrap_or(DEFAULT_SEARCH);
        moved.push(serde_json::json!({ "which": "search", "from": search, "to": to, "why": why }));
        let _ = db::set_setting(conn, "shortcut_search", &serde_json::json!(to));
    }
    if !moved.is_empty() {
        let _ = db::set_setting(conn, "shortcut_moved", &serde_json::Value::Array(moved));
    }
    keys
}

/// After a restore: the restored settings take effect now (edge opening, peeking, shortcuts),
/// no restart needed. A shortcut another app took meanwhile keeps the current one.
pub fn reload_settings(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (edge, peek, new, language) = {
        let conn = state.db.lock().unwrap();
        // An older backup may come from before the styles: it gets their defaults too.
        let _ = look::migrate(&conn);
        look::changed(app, &conn);
        window::reload_prefs(app, &conn);
        (
            db::get_setting(&conn, "edge_open").and_then(|v| v.as_bool()).unwrap_or(false),
            db::get_setting(&conn, "edge_peek").and_then(|v| v.as_bool()).unwrap_or(false),
            stored_keys(&conn),
            i18n::choice(&conn),
        )
    };
    state.edge_open.store(edge, Ordering::Relaxed);
    state.edge_peek.store(peek, Ordering::Relaxed);
    window::wake_edge_watch();
    let gs = app.global_shortcut();
    // (The keys aren't held while registering: that is done on the main thread, whose shortcut
    // handler reads them. A restore runs off the main thread.)
    let (toggle, dump) = {
        let k = state.keys.lock().unwrap();
        (k.toggle.clone(), k.dump.clone())
    };
    let swap = |cur: &str, want: String| -> String {
        if cur == want {
            return want;
        }
        let (Ok(old), Ok(next)) = (parse_keys(cur), parse_keys(&want)) else { return cur.to_string() };
        let _ = gs.unregister(old);
        if gs.register(next).is_ok() {
            want
        } else {
            let _ = gs.register(old);
            cur.to_string()
        }
    };
    let (toggle, dump) = (swap(&toggle, new.toggle), swap(&dump, new.dump));
    {
        let mut k = state.keys.lock().unwrap();
        k.toggle = toggle;
        k.dump = dump;
    }
    note_taken_shortcuts(app);
    // The restored language, as when it is set (`set_setting`): notifications, the tray menu (its
    // shortcuts as kept just above) and dialogs follow at once, and Dobo Setup and the uninstaller speak it too.
    i18n::set(&language);
    install::tell_setup(i18n::resolve(&language));
    refresh_tray(app);
    let _ = app.emit("language", ());
}

/// Everything Dobo does at start, once Tauri runs (any error: `failure::startup_failed`).
fn start(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // The single-instance lock is up: another start may look at updates now (updates.rs).
    updates::release_start_lock();
    // Where the notes are (installed, portable, or the notes this computer already had):
    // decided once, here, before any window (WebView2's folder depends on it).
    let place = paths::init(&app.path().app_data_dir()?);
    let dir = place.dir.clone();
    std::fs::create_dir_all(&dir)?;
    // The clock as Windows shows it (12 or 24 hours) before anything shows a time: the recovery
    // screen shows the date of a backup.
    clock::refresh_hour12();
    // Pictures in a portable data folder show too (the config allows $APPDATA's).
    for folder in ["pictures", "previews", "backgrounds"] {
        let _ = app.asset_protocol_scope().allow_directory(dir.join(folder), true);
    }
    // The main window (hidden until shown below), as early as before it was made here:
    // WebView2 starts loading the page while the notes open. (Its calls wait for the end
    // of setup, when everything they use is ready.)
    window::create_main(app.handle())?;
    // The notes open, or the recovery screen says what stands in the way (damaged, held by
    // another program, read-only): never an exit with no word. The board starts once they open.
    // (A portable copy that can't write next to itself asks before using the usual folder.)
    let look = match &place.blocked {
        Some(blocked) => recovery::Look::Trouble(recovery::portable_problem(&dir, blocked)),
        None => recovery::first_look(&dir, db::now_ms()),
    };
    match look {
        recovery::Look::Open(conn) => open_board(app.handle(), conn, false),
        recovery::Look::Trouble(p) => {
            eprintln!("The notes didn't open ({:?}): {}", p.kind, p.error);
            app.manage(recovery::Recovery(Mutex::new(recovery::Stage::Waiting(p))));
            // (Started with Windows: it shows, without taking the keyboard from what is in front.)
            let hidden = std::env::args_os().any(|a| a == "--hidden");
            window::show_recovery(app.handle(), !hidden);
            Ok(())
        }
    }
}

/// The board starts, its notes open (`conn`): everything else Dobo does at start. Right away, or
/// once the recovery screen has opened them (`after_recovery`: the window then shows whatever
/// the start said).
fn open_board(app: &AppHandle, conn: Connection, after_recovery: bool) -> Result<(), Box<dyn std::error::Error>> {
    let place = paths::place().ok_or("The data folder isn’t known yet.")?;
    let dir = place.dir.clone();
    look::migrate(&conn).map_err(std::io::Error::other)?;
    // A portable data folder may have moved since the last start (a USB stick under
    // another drive letter): its pictures are found where they are now. (Never for the
    // usual folder, which doesn't move.)
    if place.webview.is_some()
        && let Err(e) = backup::repoint_moved(&conn, &dir)
    {
        eprintln!("Pictures couldn't be found again after the data folder moved: {e}");
    }
    i18n::load(&conn);
    // The installed copy tells Dobo Setup its language once at start, when it differs (Windows'
    // language changed while Dobo follows it, or an install from before 1.1).
    install::tell_setup(i18n::resolve(&i18n::choice(&conn)));
    clock::refresh_hour12();
    // A portable copy using the usual folder says so there: uninstalling the installed copy
    // then keeps these notes. (Release builds only: a development run has notes of its own.)
    #[cfg(not(debug_assertions))]
    if let Ok(exe) = std::env::current_exe() {
        paths::note_portable_copy(place, &exe);
    }
    // Online only once the terms of use are accepted (an installed copy: by Setup).
    consent::start_with(&conn);
    let edge = db::get_setting(&conn, "edge_open").and_then(|v| v.as_bool()).unwrap_or(false);
    let peek = db::get_setting(&conn, "edge_peek").and_then(|v| v.as_bool()).unwrap_or(false);
    let keys = stored_keys(&conn);
    let prefs = window::Prefs::read(&conn);
    app.manage(AppState {
        db: Mutex::new(conn),
        moves: AtomicU64::new(0),
        saving: AtomicBool::new(false),
        edge_open: AtomicBool::new(edge),
        edge_peek: AtomicBool::new(peek),
        peeking: AtomicBool::new(false),
        pinned: AtomicBool::new(false),
        fs_hidden: AtomicBool::new(false),
        fs_watching: AtomicBool::new(false),
        keys: Mutex::new(Keys { toggle: keys.toggle.clone(), dump: keys.dump.clone() }),
        prefs: Mutex::new(prefs),
        taken: Mutex::new(Vec::new()),
    });
    first_run(app).map_err(std::io::Error::other)?;
    purge_unused_pictures(app, after_recovery);
    // The pictures of a backup made before this version opened the notes, when they couldn't be
    // linked at once (another drive, a drive with no links): copied now, in the background.
    backup::finish_later(app);
    // "Start with Windows" set under the working name (ZToDo) becomes Dobo's: the old
    // entry would start a ZToDo.exe that is gone. (Release builds only: a development
    // run never changes the real one.)
    #[cfg(all(windows, not(debug_assertions)))]
    {
        rename::take_old_start(|| install::enable_autostart().map_err(|e| eprintln!("Start with Windows couldn't be carried over to Dobo: {e}")).is_ok());
        // Dobo 1.0's entry named its program unquoted: quoted now.
        install::quote_old_autostart();
    }

    // Notifications' name and icon, and the `ztodo:` link their buttons use. The icon is
    // written again whenever the app's own one changes; its file name changed with the
    // logo, so Windows (which keeps the picture it read for a file) shows the new one.
    let icon = dir.join("notification-icon-dobo.png");
    let png: &[u8] = include_bytes!("../icons/128x128.png");
    if std::fs::read(&icon).ok().as_deref() != Some(png) {
        let _ = std::fs::write(&icon, png);
    }
    let _ = std::fs::remove_file(dir.join("notification-icon.png"));
    notify::register(&icon);
    // The secret every notification link carries (made once, kept in the data folder).
    notify::init_token(&dir);

    for accel in [&keys.toggle, &keys.dump] {
        match parse_keys(accel) {
            Ok(s) => {
                if let Err(e) = app.global_shortcut().register(s) {
                    eprintln!("{accel} is taken by another app: {e}");
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }
    // One another app holds does nothing: the board says so, and Settings › Keyboard too.
    note_taken_shortcuts(app);

    window::create_dump(app)?;
    // The mouse's Back / Forward buttons when mouse software sends them as app commands.
    if let Some(w) = app.get_webview_window("main") {
        navkeys::hook(&w);
    }

    let menu = tray_menu(app, &keys)?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(format!("{APP_NAME}  ({})", i18n::keys(&keys.toggle)))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => window::show(app),
            "dump" => window::show_dump(app),
            "quit" => {
                window::hide(app);
                app.exit(0)
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, e| match e {
            TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => window::toggle(tray.app_handle()),
            TrayIconEvent::Enter { .. } => tray::refresh(tray.app_handle()),
            _ => {}
        });
    // The note drawn for the taskbar's theme and scale (tray.rs), following it live.
    if let Some(icon) = tray::icon_now().or_else(|| app.default_window_icon().cloned()) {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    tray::watch(app.clone());
    // (A change between the icon above and the watch: caught here.)
    tray::refresh(app);

    reminders::start(app.clone());
    window::start_edge_watch(app.clone());
    // Installed copy: Windows Apps shows the version really installed (Dobo's own
    // updates replace Dobo.exe without the installer).
    install::sync_apps_entry();
    start_update_checks(app.clone());

    // (args_os, as main.rs: an argument that isn't valid Unicode must not stop Dobo.)
    let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    if let Some(url) = args.iter().find(|a| a.starts_with(&format!("{}:", notify::SCHEME))) {
        handle_url(app, url);
    } else if after_recovery || !args.iter().any(|a| a == "--hidden") {
        window::show(app);
    }
    started(app);
    Ok(())
}

/// Second starts that came while this one was still starting (WebView2 starting runs Windows'
/// messages, the single-instance one included): none of what they ask is ready yet (the notes,
/// the notifications' secret, the window). `None` while starting.
static EARLY: Mutex<Option<Vec<Vec<String>>>> = Mutex::new(Some(Vec::new()));

/// A second start's arguments, to do now; or None: kept for when this one has started.
fn early_second_start(args: Vec<String>) -> Option<Vec<String>> {
    let mut early = EARLY.lock().unwrap_or_else(|e| e.into_inner());
    match early.as_mut() {
        Some(waiting) => {
            waiting.push(args);
            None
        }
        None => Some(args),
    }
}

/// Started: the second starts kept meanwhile are done now, in order.
fn started(app: &AppHandle) {
    let waiting = EARLY.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
    for args in waiting {
        second_start(app, &args);
    }
}

/// Another start of Dobo (a `ztodo:` link from a notification, or opened again): this one does it.
fn second_start(app: &AppHandle, args: &[String]) {
    if let Some(url) = args.iter().find(|a| a.starts_with(&format!("{}:", notify::SCHEME))) {
        handle_url(app, url);
    } else {
        window::show(app);
    }
}

pub fn run() {
    // An error Dobo can't go on from is said and noted, never a silent exit (failure.rs). The
    // messages follow Windows' language until the notes' own setting is read.
    failure::catch_panics();
    // (Before anything uses SQLite: damage it finds is noted, recovery.rs.)
    recovery::listen_for_damage();
    i18n::set("system");
    // Where the notes are, decided before anything else (an update put in place right below
    // keeps its notes there). Setup finds it decided.
    paths::init_early();
    // A downloaded update goes in place before anything else starts (then it starts instead).
    if updates::apply_at_start() {
        failure::exiting();
        return;
    }
    // A second start hands over to the Dobo already running and ends at once, inside the
    // single-instance plugin (its own exit, before any of Dobo's code runs): an error from here
    // is only noted (`failure::exiting`). Should that Dobo end meanwhile, this one goes on: said
    // again once built.
    if updates::another_instance_running() {
        failure::exiting();
    }
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Started again while the recovery screen asks: it comes to the front (what was asked
            // is done once the board has started).
            if app.try_state::<AppState>().is_none() && app.try_state::<recovery::Recovery>().is_some() {
                window::show_recovery(app, true);
            }
            // A second start while this one is still starting (a notification's button clicked
            // then): kept, and done once it has started (`started`).
            if let Some(args) = early_second_start(args) {
                second_start(app, &args);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let (toggle, dump) = {
                        let st = app.state::<AppState>();
                        let k = st.keys.lock().unwrap();
                        (parse_keys(&k.toggle).ok(), parse_keys(&k.dump).ok())
                    };
                    if toggle.as_ref() == Some(shortcut) {
                        window::toggle(app);
                    } else if dump.as_ref() == Some(shortcut) {
                        window::show_dump(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());
    // "Start with Windows": an entry named after the app ("Dobo"), set here rather than taken from
    // the product name, so it can't change by itself. (Windows: install.rs. The old "ZToDo" one:
    // below.)
    #[cfg(not(windows))]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().app_name(APP_NAME).arg(install::START_ARG).build());
    let app = builder
        .setup(|app| {
            // Any error here: said in a box (no silent exit), noted in the data folder. This
            // start lets go of being the one Dobo first: a second start meanwhile (a sign-in
            // entry, a notification clicked) then starts for itself, instead of handing over to
            // this one, which can't open, and showing nothing until the box is closed.
            if let Err(e) = start(app) {
                tauri_plugin_single_instance::destroy(app);
                failure::startup_failed(&e.to_string());
                failure::exiting();
                std::process::exit(1);
            }
            Ok(())
        })
        .on_window_event(|win, event| {
            // (Events while Dobo is still starting, before its state exists: nothing to do yet.)
            if win.app_handle().try_state::<AppState>().is_none() {
                return;
            }
            if win.label() == "dump" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window::hide_dump(win.app_handle());
                }
                return;
            }
            match event {
                // Closing only hides: Dobo keeps running in the tray (reminders need it).
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    window::hide(win.app_handle());
                }
                // The tray icon's size follows the display scale (and its theme, the taskbar's).
                WindowEvent::ThemeChanged(_) => tray::refresh(win.app_handle()),
                WindowEvent::ScaleFactorChanged { .. } => {
                    tray::refresh(win.app_handle());
                    // A docked or peeking panel showing gets its width right for the new scale
                    // (Windows rescaled it by the ratio). Done after this event, never inside it.
                    let app = win.app_handle().clone();
                    let later = app.clone();
                    let _ = app.run_on_main_thread(move || window::refit(&later));
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    let app = win.app_handle().clone();
                    let st = app.state::<AppState>();
                    st.moves.store(steady_ms(), Ordering::SeqCst);
                    // One saver for a whole drag: it waits until the window stays still 600 ms.
                    if st.saving.swap(true, Ordering::SeqCst) {
                        return;
                    }
                    std::thread::spawn(move || {
                        let st = app.state::<AppState>();
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            let last = st.moves.load(Ordering::SeqCst);
                            if still_for(last, steady_ms(), 600) {
                                break;
                            }
                        }
                        st.saving.store(false, Ordering::SeqCst);
                        if let Some(w) = app.get_webview_window("main") {
                            window::save_float_geometry(&app, &w);
                        }
                    });
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_all,
            create_note,
            update_note,
            delete_note,
            reorder_notes,
            create_item,
            update_item,
            delete_item,
            set_setting,
            language_now,
            inbox_for,
            drop_empty_inbox,
            count_sticky_notes,
            import_sticky_notes,
            window_info,
            hide_panel,
            apply_window,
            set_float_on_top,
            list_monitors,
            answer_ring,
            page_ready,
            tidy_answer,
            rhythm_presets,
            show_dump,
            hide_dump,
            dump_item,
            fetch_preview,
            report_info,
            report_send,
            report_mail,
            set_item_link,
            open_path,
            open_url,
            save_picture,
            add_picture_file,
            delete_picture,
            make_thumb,
            release_picture,
            set_shortcut,
            shortcuts_taken,
            get_autostart,
            set_autostart,
            restore,
            open_note,
            end_peek,
            save_note_image,
            reveal,
            backup_now,
            backup_list,
            backup_restore,
            backup_status,
            backup_folder,
            data_info,
            open_data_folder,
            dark_mode_now,
            look::export_look,
            look::import_look,
            look::pick_background_picture,
            resize_docked,
            set_pinned,
            accept_terms,
            quit_app,
            update_status,
            update_check,
            update_restart,
            recovery_state,
            recovery_choose,
            recovery_continue
        ])
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            failure::startup_failed(&e.to_string());
            failure::exiting();
            std::process::exit(1);
        }
    };
    // This start is the one Dobo (it didn't hand over): errors are said again.
    failure::not_exiting();
    app.run(|app, event| {
        // Quitting normally: a new version on trial whose window loaded worked; and the notes,
        // when they were open, needn't be read through at the next start (recovery.rs).
        if let tauri::RunEvent::Exit = event {
            failure::exiting();
            updates::quit_normally();
            if let Some(state) = app.try_state::<AppState>()
                && let Ok(dir) = paths::data_dir()
            {
                recovery::ended_normally(&dir, &state.db.lock().unwrap_or_else(|e| e.into_inner()));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The clean-up at start keeps the pictures the notes put aside (recovery.rs) still name, so
    /// they can be recovered whole; one that can't be read at all: nothing is cleaned up.
    #[test]
    fn the_clean_up_keeps_the_pictures_of_notes_put_aside() {
        let dir = std::env::temp_dir().join(format!("dobo-purge-{}-{:?}", db::now_ms(), std::thread::current().id()));
        let pictures = dir.join("pictures");
        std::fs::create_dir_all(&pictures).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 24 * 3600);
        for name in ["mine.png", "aside.png", "nobodys.png"] {
            let f = std::fs::File::create(pictures.join(name)).unwrap();
            f.set_modified(old).unwrap();
        }
        let with_picture = |file: &Path, name: &str| {
            let conn = db::open(file).unwrap();
            let note = db::create_note(&conn, "N", "work", "soft", "Butter").unwrap();
            db::add_picture(&conn, note.id, None, &pictures.join(name).to_string_lossy()).unwrap();
            conn
        };
        let conn = with_picture(&dir.join(paths::DB_FILE), "mine.png");
        // A file put aside (the recovery screen's Restore or Empty) names its own.
        drop(with_picture(&dir.join("broken-2026-11-20.sqlite"), "aside.png"));
        // Right after a choice on the recovery screen (the notes may be an older backup's):
        // nothing goes this time.
        purge_at_start(&dir, &conn, true).unwrap();
        assert!(pictures.join("nobodys.png").is_file(), "not after the recovery screen");
        purge_at_start(&dir, &conn, false).unwrap();
        let left = |n: &str| pictures.join(n).is_file();
        assert!(left("mine.png") && left("aside.png"), "both kept");
        assert!(!left("nobodys.png"), "the unused one goes");
        // One put aside that can't be read at all: nothing goes while it is there.
        let f = std::fs::File::create(pictures.join("other.png")).unwrap();
        f.set_modified(old).unwrap();
        std::fs::write(dir.join("broken-2026-11-21.sqlite"), b"not a database, not at all, not even a little").unwrap();
        assert!(purge_in(&dir, &conn).is_err());
        assert!(left("other.png") && left("aside.png"));
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_note_saved_as_a_picture_gets_a_name_windows_takes() {
        assert_eq!(file_stem_for("Trip to Lyon: plan"), "Trip to Lyon  plan");
        for (title, file) in [("Con", "Con_"), ("nul", "nul_"), ("AUX", "AUX_"), ("Com1", "Com1_"), ("lpt9", "lpt9_"), ("Prn", "Prn_")] {
            assert_eq!(file_stem_for(title), file);
        }
        assert_eq!(file_stem_for("Console"), "Console");
        assert_eq!(file_stem_for("COM10"), "COM10");
        assert_eq!(file_stem_for("COM¹"), "COM¹_");
        assert_eq!(file_stem_for("lpt³"), "lpt³_");
        assert_eq!(file_stem_for("COM⁴"), "COM⁴");
        assert_eq!(file_stem_for("???"), "note");
        assert_eq!(file_stem_for(""), "note");
        assert_eq!(file_stem_for(&"é".repeat(500)).chars().count(), 80);
        assert!(!file_stem_for(&format!("{} x", "a".repeat(79))).ends_with(' '));
    }

    /// What is asked of the board before it listens is kept, in order, and handed over once;
    /// after that, sent at once.
    #[test]
    fn the_board_gets_what_was_asked_before_it_listened_in_order() {
        let mut page = (false, Vec::new());
        assert_eq!(keep_for_page(&mut page, "focus-item", serde_json::json!(3)), None);
        assert_eq!(keep_for_page(&mut page, "later-item", serde_json::json!(3)), None);
        let kept = take_for_page(&mut page);
        assert_eq!(kept, vec![("focus-item".to_string(), serde_json::json!(3)), ("later-item".to_string(), serde_json::json!(3))]);
        assert!(take_for_page(&mut page).is_empty(), "handed over once");
        assert_eq!(keep_for_page(&mut page, "open-lookback", serde_json::json!(null)), Some(serde_json::json!(null)), "sent now");
        assert!(page.1.is_empty());
    }

    /// A shortcut another app holds is said once, not at every look; one taken again later is
    /// said again.
    #[test]
    fn a_taken_shortcut_is_said_once() {
        let now = vec![("toggle".to_string(), "Ctrl+Alt+D".to_string())];
        assert_eq!(newly_taken(&[], &now), vec![serde_json::json!({ "which": "toggle", "keys": "Ctrl+Alt+D" })]);
        assert!(newly_taken(&["toggle".into()], &now).is_empty(), "already said");
        let both = vec![("toggle".to_string(), "Ctrl+Alt+D".to_string()), ("dump".to_string(), "Alt+Q".to_string())];
        assert_eq!(newly_taken(&["toggle".into()], &both), vec![serde_json::json!({ "which": "dump", "keys": "Alt+Q" })]);
        assert!(newly_taken(&["toggle".into()], &[]).is_empty());
    }

    #[test]
    fn a_second_start_while_starting_waits_until_started() {
        // (EARLY: this test alone touches it.)
        assert_eq!(early_second_start(vec!["Dobo.exe".into(), "ztodo://ring?item=1&a=done".into()]), None, "kept");
        assert_eq!(early_second_start(vec!["Dobo.exe".into()]), None);
        let waiting = EARLY.lock().unwrap().take().unwrap();
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0][1], "ztodo://ring?item=1&a=done", "in order");
        // Started: done at once from then on.
        assert_eq!(early_second_start(vec!["Dobo.exe".into()]), Some(vec!["Dobo.exe".to_string()]));
    }

    /// Every command is in build.rs's list (its permission exists), the board may call each one,
    /// and the quick dump bar only its own few.
    #[test]
    fn each_window_may_call_only_its_commands() {
        let names = |src: &str, start: &str, end: char| -> Vec<String> {
            let a = src.find(start).unwrap() + start.len();
            let b = a + src[a..].find(end).unwrap();
            src[a..b].split(',').map(|x| x.trim().trim_matches('"').rsplit("::").next().unwrap().to_string()).filter(|x| !x.is_empty()).collect()
        };
        let lib = include_str!("lib.rs");
        let mut handled = names(lib, &["tauri::generate_handler", "!["].concat(), ']');
        let mut listed = names(include_str!("../build.rs"), "const COMMANDS: &[&str] = &[", ']');
        handled.sort();
        listed.sort();
        assert_eq!(handled, listed, "build.rs COMMANDS = generate_handler!");
        let allowed = |file: &str| -> Vec<String> {
            let v: serde_json::Value = serde_json::from_str(file).unwrap();
            v["permissions"].as_array().unwrap().iter().filter_map(|p| p.as_str()?.strip_prefix("allow-").map(|c| c.replace('-', "_"))).collect()
        };
        let main = allowed(include_str!("../capabilities/main.json"));
        for c in &handled {
            assert!(main.contains(c), "the board may call {c}");
        }
        let dump = allowed(include_str!("../capabilities/dump.json"));
        assert_eq!(dump, ["language_now", "dark_mode_now", "dump_item", "save_picture", "fetch_preview", "set_item_link", "hide_dump"]);
        // Nothing broad: no core:default (trays, menus, images from any path, devtools).
        for file in [include_str!("../capabilities/main.json"), include_str!("../capabilities/dump.json")] {
            assert!(!file.contains("core:default") && !file.contains(":default\""));
        }
    }

    #[test]
    fn the_steady_clock_only_goes_forward() {
        let a = steady_ms();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let b = steady_ms();
        assert!(b >= a + 15, "{a} then {b}");
    }

    #[test]
    fn the_window_is_saved_once_still_for_600_ms() {
        assert!(!still_for(1_000, 1_599, 600), "still moving");
        assert!(still_for(1_000, 1_600, 600));
        assert!(still_for(1_000, 90_000, 600));
        // A last move "in the future" (it can't be): seen as just now, never a wait of hours.
        assert!(!still_for(10_000, 1_000, 600));
        assert!(still_for(10_000, 10_600, 600));
    }

    #[test]
    fn the_fixed_in_app_keys_are_understood() {
        for (k, _) in IN_APP_KEYS.iter() {
            assert!(parse_keys(k).is_ok(), "{k}");
        }
        // Moving a line, and AZERTY's minus and underscore keys (Ctrl+6, Ctrl+8) for the size.
        for k in ["Ctrl+Up", "Ctrl+Down", "Ctrl+Shift+Down", "Ctrl+6", "Ctrl+8"] {
            assert!(in_app_key(k), "{k}");
        }
        // However the recorder writes them, undo, the size keys and Back / Forward are reserved.
        for k in ["Ctrl+Z", "Shift+Ctrl+Z", "Ctrl+Y", "Ctrl+=", "Ctrl+-", "Ctrl+0", "Ctrl+Num0", "Alt+Left", "Alt+Right"] {
            assert!(in_app_key(k), "{k}");
        }
        assert!(!in_app_key("Ctrl+Alt+Z"));
        assert!(!in_app_key("Alt+Up"));
        assert_eq!(parse_keys("Ctrl+Shift+S").unwrap(), parse_keys("Shift+Ctrl+S").unwrap());
        // What the Settings recorder sends for Ctrl+, must match the reserved one.
        assert_eq!(parse_keys("Ctrl+,").ok(), parse_keys("Ctrl+Comma").ok());
    }

    #[test]
    fn a_global_shortcut_on_an_in_app_key_goes_back_to_its_default() {
        assert!(in_app_key("Shift+Ctrl+S"));
        assert!(in_app_key("Ctrl+,"));
        assert!(!in_app_key(DEFAULT_TOGGLE));
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Ctrl+Shift+S")).unwrap();
        db::set_setting(&conn, "shortcut_dump", &serde_json::json!("Alt+Q")).unwrap();
        let k = stored_keys(&conn);
        assert_eq!(k.toggle, DEFAULT_TOGGLE);
        assert_eq!(k.dump, "Alt+Q");
        assert_eq!(db::get_setting(&conn, "shortcut_toggle"), Some(serde_json::json!(DEFAULT_TOGGLE)));
        let moved = db::get_setting(&conn, "shortcut_moved").unwrap();
        assert_eq!(moved[0]["which"], "toggle");
        assert_eq!(moved[0]["from"], "Ctrl+Shift+S");
    }

    #[test]
    fn a_shortcut_that_would_fire_while_typing_is_refused() {
        for k in ["Shift+A", "Shift+1", "A", "Shift+Space", "F5", "Shift+Left", "Shift+Enter", "Shift+Tab", "Pause"] {
            assert!(fires_while_typing(&parse_keys(k).unwrap()), "{k}");
        }
        // Shift with a key that never types is fine.
        for k in ["Ctrl+A", "Alt+Shift+A", "Super+D", "Ctrl+Shift+Space", DEFAULT_DUMP, "Shift+F5", "Shift+F12", "Shift+F24", "Shift+Pause", "Shift+ScrollLock"] {
            assert!(!fires_while_typing(&parse_keys(k).unwrap()), "{k}");
        }
        // One saved before that was refused goes back to its default, said once with why.
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Shift+A")).unwrap();
        let k = stored_keys(&conn);
        assert_eq!(k.toggle, DEFAULT_TOGGLE);
        assert_eq!(k.dump, DEFAULT_DUMP);
        let moved = db::get_setting(&conn, "shortcut_moved").unwrap();
        assert_eq!(moved[0]["why"], "typing");
        assert_eq!(moved[0]["from"], "Shift+A");
    }

    /// Windows' menu key (Shift+F10, the Menu key) is never a shortcut: one saved earlier goes back
    /// to its default, said once with why.
    #[test]
    fn windows_menu_key_is_never_a_shortcut() {
        assert!(menu_key(&parse_keys("Shift+F10").unwrap()));
        assert_eq!(refused("Shift+F10"), Some("menu"));
        // The Menu key itself isn't a key a shortcut can have (nor one the recorder writes).
        assert!(parse_keys("ContextMenu").is_err() && parse_keys("Ctrl+ContextMenu").is_err());
        for k in ["F10", "Ctrl+Shift+F10", "Alt+F10", "Shift+F11"] {
            assert!(!menu_key(&parse_keys(k).unwrap()), "{k}");
        }
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_dump", &serde_json::json!("Shift+F10")).unwrap();
        db::set_setting(&conn, "shortcut_search", &serde_json::json!("Shift+F10")).unwrap();
        let k = stored_keys(&conn);
        assert_eq!(k.dump, DEFAULT_DUMP);
        let moved = db::get_setting(&conn, "shortcut_moved").unwrap();
        assert_eq!((moved[0]["which"].as_str(), moved[0]["why"].as_str()), (Some("dump"), Some("menu")));
        assert_eq!((moved[1]["which"].as_str(), moved[1]["why"].as_str()), (Some("search"), Some("menu")));
    }

    /// Search sent back to Ctrl+F at start while a global shortcut is on Ctrl+F: it goes to a
    /// key neither global shortcut uses, and says so.
    #[test]
    fn search_sent_back_never_lands_on_a_global_shortcut() {
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Ctrl+F")).unwrap();
        db::set_setting(&conn, "shortcut_search", &serde_json::json!("Ctrl+0")).unwrap();
        stored_keys(&conn);
        assert_eq!(db::get_setting(&conn, "shortcut_search"), Some(serde_json::json!("Ctrl+Shift+F")));
        assert_eq!(db::get_setting(&conn, "shortcut_moved").unwrap()[0]["to"], "Ctrl+Shift+F");
        // Both taken: the next one.
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Ctrl+Shift+F")).unwrap();
        db::set_setting(&conn, "shortcut_dump", &serde_json::json!("Ctrl+F")).unwrap();
        db::set_setting(&conn, "shortcut_search", &serde_json::json!("Ctrl+Z")).unwrap();
        stored_keys(&conn);
        assert_eq!(db::get_setting(&conn, "shortcut_search"), Some(serde_json::json!("Ctrl+Alt+F")));
    }

    /// A 1.0 Shift+F-key shortcut stays as it is at the next start.
    #[test]
    fn a_shift_f_key_shortcut_is_kept() {
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Shift+F12")).unwrap();
        assert_eq!(stored_keys(&conn).toggle, "Shift+F12");
        assert!(db::get_setting(&conn, "shortcut_moved").is_none());
    }

    /// A search shortcut saved on a key Dobo now uses inside (Ctrl+0: the note size) goes back to
    /// Ctrl+F, said once like the others.
    #[test]
    fn a_search_shortcut_on_an_in_app_key_goes_back_to_its_default() {
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_search", &serde_json::json!("Ctrl+0")).unwrap();
        let k = stored_keys(&conn);
        assert_eq!((k.toggle.as_str(), k.dump.as_str()), (DEFAULT_TOGGLE, DEFAULT_DUMP));
        assert_eq!(db::get_setting(&conn, "shortcut_search"), Some(serde_json::json!(DEFAULT_SEARCH)));
        let moved = db::get_setting(&conn, "shortcut_moved").unwrap();
        assert_eq!(moved[0]["which"], "search");
        assert_eq!(moved[0]["from"], "Ctrl+0");
        assert_eq!(moved[0]["to"], DEFAULT_SEARCH);
        // One that is fine stays.
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_search", &serde_json::json!("Ctrl+Shift+F")).unwrap();
        stored_keys(&conn);
        assert_eq!(db::get_setting(&conn, "shortcut_search"), Some(serde_json::json!("Ctrl+Shift+F")));
        assert!(db::get_setting(&conn, "shortcut_moved").is_none());
    }

    #[test]
    fn a_shortcut_sent_back_to_its_default_never_lands_on_the_other_one() {
        let conn = db::open(Path::new(":memory:")).unwrap();
        db::set_setting(&conn, "shortcut_toggle", &serde_json::json!("Ctrl+Shift+S")).unwrap();
        db::set_setting(&conn, "shortcut_dump", &serde_json::json!(DEFAULT_TOGGLE)).unwrap();
        let k = stored_keys(&conn);
        assert_eq!(k.toggle, DEFAULT_TOGGLE);
        assert_eq!(k.dump, DEFAULT_DUMP);
        let moved = db::get_setting(&conn, "shortcut_moved").unwrap();
        assert_eq!(moved[1]["which"], "dump");
    }

    #[test]
    fn the_first_start_makes_the_inbox_and_brings_in_nothing_by_itself() {
        let conn = db::open(Path::new(":memory:")).unwrap();
        first_setup(&conn).unwrap();
        assert!(db::get_setting(&conn, "first_run_done").is_some());
        assert_eq!(db::note_count(&conn), 1, "the Inbox only");
        assert_eq!(db::inbox_count(&conn), 0, "with nothing in it");
        // No import was made, nor recorded: the welcome asks first.
        assert!(db::get_setting(&conn, "sticky_imported").is_none());
        assert!(db::get_setting(&conn, "sticky_ids").is_none());
        // A second start changes nothing.
        first_setup(&conn).unwrap();
        assert_eq!(db::note_count(&conn), 1);
    }

    /// A file in `dir`, written `days` ago.
    fn old_file(dir: &Path, name: &str, days: u64) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, b"png").unwrap();
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 24 * 3600);
        std::fs::File::options().write(true).open(&p).unwrap().set_modified(t).unwrap();
        p
    }

    #[test]
    fn a_picture_still_in_use_is_never_cleaned_up_however_its_path_is_written() {
        let dir = std::env::temp_dir().join(format!("dobo-purge-{}-{:?}", db::now_ms(), std::thread::current().id()));
        let pics = dir.join("pictures");
        std::fs::create_dir_all(&pics).unwrap();
        let kept = old_file(&pics, "1700000000000-0.png", 3);
        let upper = old_file(&pics, "1700000000000-1.png", 3);
        let gone = old_file(&pics, "1700000000000-2.png", 3);
        let recent = old_file(&pics, "1700000000000-3.png", 0);
        // The notes name them as another start of a portable copy wrote them (another letter
        // case, other slashes): the same files.
        let c = db::open(Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        db::add_picture(&c, n.id, None, &kept.to_string_lossy().to_uppercase()).unwrap();
        db::add_picture(&c, n.id, None, &upper.to_string_lossy().replace('\\', "/")).unwrap();
        let used = used_files(&c).unwrap();
        let day_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3600);
        let mut out = unused_files(&pics, &used, day_ago);
        out.sort();
        assert_eq!(out, vec![gone.clone()], "only the old one nothing uses goes");
        assert!(!out.contains(&recent));
        // Their thumbs (a folder of their own, never taken for a picture): the used ones kept,
        // however the notes write their picture's path.
        let made = pics.join(thumbs::DIR);
        std::fs::create_dir_all(&made).unwrap();
        let kept_thumb = old_file(&made, &thumbs::name_for("1700000000000-0.png", 1200, 900), 3);
        let upper_thumb = old_file(&made, &thumbs::name_for("1700000000000-1.png", 1200, 900), 3);
        let gone_thumb = old_file(&made, &thumbs::name_for("1700000000000-2.png", 1200, 900), 3);
        assert_eq!(unused_files(&pics, &used, day_ago), vec![gone.clone()]);
        let out = thumbs::unused(&made, &used, day_ago);
        assert_eq!(out, vec![gone_thumb]);
        assert!(!out.contains(&kept_thumb) && !out.contains(&upper_thumb));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The clean-up never runs on a read that failed: it would take it for "nothing is used".
    #[test]
    fn nothing_counts_as_unused_when_the_notes_cant_be_read() {
        let c = db::open(Path::new(":memory:")).unwrap();
        let n = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        db::add_picture(&c, n.id, None, "C:/d/pictures/1-0.png").unwrap();
        assert!(used_files(&c).unwrap().contains("1-0.png"));
        // A look that isn't JSON any more (a damaged row): its background is unknown.
        c.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('look_board', '{not json')", []).unwrap();
        assert!(used_files(&c).is_err());
        c.execute("DELETE FROM settings WHERE key = 'look_board'", []).unwrap();
        // A table that can't be read (here: gone).
        c.execute_batch("PRAGMA foreign_keys = OFF; DROP TABLE pictures;").unwrap();
        assert!(used_files(&c).is_err());
    }

    fn line(note_id: i64, text: &str) -> db::NewItem<'_> {
        db::NewItem { note_id, kind: "task", text, due_at: None, parent_id: None, alarm: "reminder", link: None, file: None }
    }

    fn archived(n: &Note, on: bool) -> Note {
        Note { archived: on, ..n.clone() }
    }

    #[test]
    fn a_space_never_stays_without_its_inbox() {
        let c = db::open(Path::new(":memory:")).unwrap();
        // An Inbox archived before 1.0 (the lower id: then, any note called "Inbox" was one) and
        // today's.
        let old = db::create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        db::update_note(&c, &archived(&old, true)).unwrap();
        let today = db::create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        db::create_item(&c, line(today.id, "Waiting")).unwrap();
        assert_eq!(ensure_inbox(&c, "work").unwrap(), today.id);
        // Put back: the old one is the Inbox again, today's a plain card that can be archived.
        let back = save_note(&c, &archived(&old, false)).unwrap().0;
        assert_eq!(db::inbox_of(&c, "work").unwrap(), Some(old.id));
        db::create_item(&c, line(old.id, "Dumped meanwhile")).unwrap();
        save_note(&c, &archived(&today, true)).unwrap();
        assert_eq!(db::inbox_of(&c, "work").unwrap(), Some(old.id));
        // The restore taken back: the space gets a new Inbox at once, and only one.
        save_note(&c, &archived(&back, true)).unwrap();
        let made = db::inbox_of(&c, "work").unwrap().expect("the space keeps an Inbox");
        assert!(made != old.id && made != today.id);
        assert_eq!(ensure_inbox(&c, "work").unwrap(), made);
        let live: i64 = c.query_row("SELECT COUNT(*) FROM notes WHERE title = 'Inbox' AND space = 'work' AND archived = 0", [], |r| r.get(0)).unwrap();
        assert_eq!(live, 1);
        // Nothing lost: both archived notes keep their lines; a dump lands in the new Inbox.
        assert_eq!(db::item_ids_of_note(&c, old.id).len(), 1);
        assert_eq!(db::item_ids_of_note(&c, today.id).len(), 1);
        assert_eq!(db::inbox_count(&c), 0);
        db::create_item(&c, line(ensure_inbox(&c, "work").unwrap(), "New")).unwrap();
        assert_eq!(db::inbox_count(&c), 1);
    }

    #[test]
    fn the_next_note_called_inbox_takes_the_place_of_one_that_goes() {
        let c = db::open(Path::new(":memory:")).unwrap();
        let first = db::create_note(&c, "Inbox", "personal", "soft", "Butter").unwrap();
        let second = db::create_note(&c, "Inbox", "personal", "soft", "Butter").unwrap();
        db::create_item(&c, line(second.id, "Here")).unwrap();
        // Renamed by an undo: the second one is promoted, nothing new is made.
        save_note(&c, &Note { title: "Trip".into(), ..first.clone() }).unwrap();
        assert_eq!(db::inbox_of(&c, "personal").unwrap(), Some(second.id));
        assert_eq!(db::note_count(&c), 2);
        assert_eq!(db::inbox_count(&c), 1);
        // Moved to the other space: Personal gets a new one, Work had none and still has none.
        let moved = save_note(&c, &Note { space: "work".into(), ..second.clone() }).unwrap().0;
        assert_eq!(db::inbox_of(&c, "work").unwrap(), Some(moved.id));
        let made = db::inbox_of(&c, "personal").unwrap().expect("Personal keeps an Inbox");
        assert!(made != first.id && made != second.id);
        // A space that never had one isn't given one by a change elsewhere.
        let c = db::open(Path::new(":memory:")).unwrap();
        let w = db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        save_note(&c, &archived(&w, true)).unwrap();
        assert!(db::inbox_spaces(&c).unwrap().is_empty());
    }

    #[test]
    fn a_stand_in_inbox_goes_only_while_empty() {
        let c = db::open(Path::new(":memory:")).unwrap();
        let hub = db::create_note(&c, "Hub", "personal", "soft", "Sage").unwrap();
        let stand = ensure_inbox(&c, "personal").unwrap();
        assert!(!drop_stand_in(&c, hub.id).unwrap(), "not an Inbox");
        db::create_item(&c, line(stand, "Dumped meanwhile")).unwrap();
        assert!(!drop_stand_in(&c, stand).unwrap(), "holds a line");
        let c = db::open(Path::new(":memory:")).unwrap();
        let stand = ensure_inbox(&c, "personal").unwrap();
        assert!(drop_stand_in(&c, stand).unwrap());
        assert!(db::inbox_spaces(&c).unwrap().is_empty());
        assert!(!drop_stand_in(&c, stand).unwrap(), "already gone");
    }

    #[test]
    fn deleting_or_restoring_keeps_each_space_its_inbox() {
        let c = db::open(Path::new(":memory:")).unwrap();
        let inbox = db::create_note(&c, "Inbox", "work", "soft", "Butter").unwrap();
        db::create_item(&c, line(inbox.id, "Waiting")).unwrap();
        let waiting = db::load_all(&c).unwrap().items;
        db::create_note(&c, "Hub", "work", "soft", "Sage").unwrap();
        assert_eq!(remove_note(&c, inbox.id).unwrap().len(), 1);
        let made = db::inbox_of(&c, "work").unwrap().unwrap();
        assert_ne!(made, inbox.id);
        // Restored over the one made meanwhile: the lower id is the Inbox again, one live Inbox
        // each for the strip (the other is a plain card); its line back with it.
        put_back(&c, std::slice::from_ref(&inbox), &waiting, &[]).unwrap();
        assert_eq!(db::item_ids_of_note(&c, inbox.id).len(), 1);
        assert_eq!(db::note_count(&c), 3);
        assert_eq!(db::inbox_of(&c, "work").unwrap(), Some(inbox.id));
        assert_eq!(db::inbox_spaces(&c).unwrap(), vec!["work".to_string()]);
    }
}
