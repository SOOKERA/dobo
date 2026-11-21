// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
#[macro_use]
mod i18n;
mod backup;
mod browser;
mod clock;
mod consent;
mod db;
mod fullscreen;
pub mod install;
mod look;
mod navkeys;
mod notify;
mod paths;
mod preview;
pub mod release;
mod reminders;
mod rename;
mod repeat;
mod sticky;
mod terms;
mod tray;
pub mod updates;
mod window;

use db::{Everything, Item, Note, Picture};
use rusqlite::Connection;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
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

pub struct AppState {
    pub db: Mutex<Connection>,
    /// When the floating window last moved or resized (ms); saved once it stays still.
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
fn temp_dir(app: &AppHandle) -> R<std::path::PathBuf> {
    paths::temp_dir(app.path().app_cache_dir().map_err(|e| e.to_string()))
}

#[tauri::command]
fn load_all(window: tauri::WebviewWindow, state: State<AppState>) -> R<Everything> {
    // The main window has loaded and reached the notes: a new version on trial is up.
    if window.label() == "main" {
        updates::app_ready();
    }
    db::load_all(&state.db.lock().unwrap())
}

#[tauri::command]
fn create_note(state: State<AppState>, title: String, space: String, palette: String, color: String) -> R<Note> {
    db::create_note(&state.db.lock().unwrap(), &title, &space, &palette, &color)
}

/// Clears the notifications of these lines (their buttons would do nothing any more).
fn clear_items(app: &AppHandle, ids: &[i64]) {
    for id in ids {
        notify::clear(app, &format!("i{id}"));
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

/// Undo after a delete: notes, lines and pictures back as they were.
#[tauri::command]
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
    let (it, next) = db::update_item_full(&state.db.lock().unwrap(), &item)?;
    if it.ring_state.is_empty() || it.done_at.is_some() {
        notify::clear(&app, &format!("i{}", it.id));
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

/// Picture files in Dobo's folder that no note uses any more and are older than a day (kept
/// that long so Undo can bring a deleted note back with its pictures).
fn purge_unused_pictures(app: &AppHandle) {
    let Ok(dir) = paths::data_dir() else { return };
    let used: std::collections::HashSet<String> = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        // Note pictures, and link previews' pictures (a preview no line uses any more goes too).
        db::picture_paths(&conn).into_iter().chain(db::preview_images(&conn).into_iter().map(|(_, p)| p)).collect()
    };
    let day_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3600);
    for folder in ["pictures", "previews"] {
        let Ok(entries) = std::fs::read_dir(dir.join(folder)) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let old = e.metadata().and_then(|m| m.modified()).map(|t| t < day_ago).unwrap_or(false);
            if old && p.is_file() && !used.contains(&p.to_string_lossy().to_string()) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

#[tauri::command]
fn set_setting(app: AppHandle, state: State<AppState>, key: String, value: serde_json::Value) -> R<()> {
    if key == "edge_open" {
        state.edge_open.store(value.as_bool().unwrap_or(false), Ordering::Relaxed);
    }
    if key == "edge_peek" {
        state.edge_peek.store(value.as_bool().unwrap_or(false), Ordering::Relaxed);
    }
    let conn = state.db.lock().unwrap();
    db::set_setting(&conn, &key, &value)?;
    if key == "dark_mode" {
        look::changed(&app, &conn);
    }
    if key == "language" {
        // Notifications, the tray menu and dialogs follow at once; so does the quick dump bar.
        i18n::set(value.as_str().unwrap_or("system"));
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
}

#[tauri::command]
fn language_now(state: State<AppState>) -> LanguageNow {
    LanguageNow { choice: i18n::choice(&state.db.lock().unwrap()), system: i18n::system() }
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

#[tauri::command]
fn import_sticky_notes(app: AppHandle, state: State<AppState>) -> R<usize> {
    let tmp = temp_dir(&app)?.join("sticky-import");
    sticky::import(&state.db.lock().unwrap(), &tmp)
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
    db::set_setting(&state.db.lock().unwrap(), "float_on_top", &serde_json::json!(on))?;
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

#[tauri::command]
fn show_dump(app: AppHandle) {
    window::show_dump(&app);
}

#[tauri::command]
fn hide_dump(app: AppHandle) {
    window::hide_dump(&app);
}

/// A line dumped from the quick bar: into the Inbox of the space showing (Work when All).
#[tauri::command]
fn dump_item(app: AppHandle, state: State<AppState>, text: String, link: Option<serde_json::Value>, file: Option<String>) -> R<(Item, i64)> {
    let (item, waiting) = {
        let conn = state.db.lock().unwrap();
        let view = db::get_setting(&conn, "view").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        let space = if view == "personal" { "personal" } else { "work" };
        let inbox = ensure_inbox(&conn, space)?;
        let item = db::create_item(
            &conn,
            db::NewItem { note_id: inbox, kind: "task", text: &text, due_at: None, parent_id: None, alarm: "reminder", link: link.as_ref(), file: file.as_deref() },
        )?;
        (item, db::inbox_count(&conn))
    };
    let _ = app.emit_to("main", "reload", ());
    Ok((item, waiting))
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

/// Files that would run something when opened (programs, scripts, shortcuts, installers).
fn runs_something(path: &std::path::Path) -> bool {
    const RISKY: &[&str] = &[
        "exe", "com", "scr", "pif", "bat", "cmd", "ps1", "psm1", "psd1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "msi", "msp",
        "mst", "msix", "msixbundle", "appx", "appxbundle", "appinstaller", "appref-ms", "application", "cpl", "msc", "reg", "inf",
        "scf", "sct", "lnk", "url", "website", "jar", "chm", "gadget", "library-ms", "searchconnector-ms", "settingcontent-ms",
        "diagcab", "xll", "py", "pyw", "pl", "rb", "sh", "bash", "zsh", "command", "tool", "app", "pkg", "desktop", "appimage", "run",
    ];
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    RISKY.contains(&ext.as_str()) || executable_file(path)
}

/// Mac and Linux: a file marked as a program (any name).
#[cfg(unix)]
fn executable_file(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(not(unix))]
fn executable_file(_path: &std::path::Path) -> bool {
    false
}

/// A file card was clicked: documents open with their app; a program or script is never
/// started from here, it is shown in its folder instead.
#[tauri::command]
fn open_path(app: AppHandle, path: String) -> R<()> {
    if runs_something(std::path::Path::new(&path)) {
        return app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string());
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
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
#[tauri::command]
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
    if window.label() != "main" {
        let _ = app.emit_to("main", "reload", ());
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
#[tauri::command]
fn add_picture_file(state: State<AppState>, note_id: i64, item_id: Option<i64>, path: String) -> R<Picture> {
    let src = std::path::Path::new(&path);
    let meta = std::fs::metadata(src).map_err(|_| tr!("That picture can't be read.", "Cette image ne peut pas être lue."))?;
    if !meta.is_file() {
        return Err(tr!("That isn't a picture file.", "Ce n’est pas un fichier image."));
    }
    if meta.len() > PICTURE_MAX {
        return Err(too_big());
    }
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dest = new_picture_path(ext)?;
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    db::add_picture(&state.db.lock().unwrap(), note_id, item_id, &dest.to_string_lossy())
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

/// "Save as a picture": into Pictures › Dobo. Returns the file.
#[tauri::command]
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
    let dir = app.path().picture_dir().or_else(|_| app.path().download_dir()).map_err(|e| e.to_string())?.join(APP_NAME);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { ' ' }).collect::<String>().trim().to_string();
    let safe = if safe.is_empty() { "note".to_string() } else { safe };
    let mut path = dir.join(format!("{safe}.png"));
    let mut k = 2;
    while path.exists() {
        path = dir.join(format!("{safe} {k}.png"));
        k += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

/// Shows a file in its folder (Explorer / Finder / the file manager).
#[tauri::command]
fn reveal(app: AppHandle, path: String) -> R<()> {
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn backup_now(app: AppHandle) -> R<backup::BackupInfo> {
    backup::make(&app, "manual")
}

#[tauri::command]
fn backup_list(app: AppHandle) -> R<Vec<backup::BackupInfo>> {
    backup::list(&app)
}

#[tauri::command]
fn backup_restore(app: AppHandle, path: String) -> R<()> {
    backup::restore(&app, &path)
}

#[tauri::command]
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
    let p = paths::place().ok_or("The data folder isn't known yet.")?;
    Ok(DataInfo { kind: p.kind, dir: p.dir.to_string_lossy().into(), kept_existing: p.kept_existing, fallback: p.fallback, version: app.package_info().version.to_string() })
}

/// Opens the data folder itself (Explorer / Finder / the file manager).
#[tauri::command]
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
    Shortcut::from_str(accel).map_err(|_| tr!("“{}” isn't a shortcut Dobo understands.", "« {} » n’est pas un raccourci que Dobo comprend.", i18n::keys(accel)))
}

/// Changes one of the shortcuts ("toggle", "dump" or "search"); refuses ones already used by
/// Dobo or by another app.
#[tauri::command]
fn set_shortcut(app: AppHandle, state: State<AppState>, which: String, accel: String) -> R<()> {
    let new = parse_keys(&accel)?;
    if new.mods.is_empty() {
        return Err(tr!("Add Ctrl, Alt or Shift so it doesn't fire while you type.", "Ajoutez Ctrl, Alt ou Maj pour qu’il ne se déclenche pas quand vous écrivez."));
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
        // Fixed keys inside the app.
        ("", "Ctrl+Shift+S", tr!("the Style panel", "le panneau Style")),
        ("", "Ctrl+Shift+P", tr!("Settings", "les Paramètres")),
        ("", "Ctrl+Comma", tr!("Settings", "les Paramètres")),
    ];
    for (name, keys, what) in names {
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
        return Err(tr!("Another app already uses {}. Try another one.", "Une autre app utilise déjà {}. Essayez-en un autre.", i18n::keys(&accel)));
    }
    {
        let mut k = state.keys.lock().unwrap();
        if which == "toggle" { k.toggle = accel.clone() } else { k.dump = accel.clone() }
    }
    db::set_setting(&state.db.lock().unwrap(), &format!("shortcut_{which}"), &serde_json::json!(accel))?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> R<()> {
    let a = app.autolaunch();
    if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
}

// ---- Terms of use and Dobo's own updates ----

/// "Agree and continue" on the terms (TERMS.md): saved at once, and Dobo may go online from now
/// on (update checks, link previews), no restart.
/// Gives the date saved (the terms' "Last updated" date, as Dobo Setup saves it).
#[tauri::command]
fn accept_terms(state: State<AppState>) -> R<String> {
    let date = terms::version().ok_or("Dobo's terms of use have no date.")?;
    db::set_setting(&state.db.lock().unwrap(), "terms_accepted", &serde_json::json!(date))?;
    consent::open();
    Ok(date)
}

/// "Quit Dobo" on the terms.
#[tauri::command]
fn quit_app(app: AppHandle) {
    window::hide(&app);
    app.exit(0);
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
                let _ = app.emit_to("main", "focus-item", id);
                // "Later…" from a notification: the Now card opens its choices.
                if query(q, "later").is_some() {
                    let _ = app.emit_to("main", "later-item", id);
                }
            }
        }
        "lookback" => {
            window::show(app);
            let _ = app.emit_to("main", "open-lookback", ());
        }
        "tidy" => reminders::tidy_answer(app, &query(q, "a").unwrap_or_else(|| "now".into())),
        _ => window::show(app),
    }
}

fn first_run(app: &AppHandle) -> R<()> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    if db::get_setting(&conn, "first_run_done").is_none() {
        if db::note_count(&conn) == 0 {
            let tmp = temp_dir(app)?.join("sticky-import");
            match sticky::import(&conn, &tmp) {
                Ok(n) => {
                    let _ = db::set_setting(&conn, "sticky_imported", &serde_json::json!(n));
                }
                Err(e) => eprintln!("Sticky Notes import failed: {e}"),
            }
        }
        ensure_inbox(&conn, "work")?;
        db::set_setting(&conn, "first_run_done", &serde_json::json!(db::now_ms()))?;
    }
    Ok(())
}

/// Keys Dobo uses inside its own window. A global shortcut on one of them would win over it
/// everywhere (the panel would hide instead of opening Style).
const IN_APP_KEYS: [&str; 3] = ["Ctrl+Shift+S", "Ctrl+Shift+P", "Ctrl+Comma"];

/// Is this one of the in-app keys (however it is written)?
fn in_app_key(accel: &str) -> bool {
    let Ok(s) = parse_keys(accel) else { return false };
    IN_APP_KEYS.iter().any(|k| parse_keys(k).ok() == Some(s))
}

/// The global shortcuts as saved. One set before the in-app keys were fixed (Ctrl+Shift+S / P,
/// Ctrl+,) goes back to its default, and the app is told once (`shortcut_moved`) so it can say so.
fn stored_keys(conn: &Connection) -> Keys {
    let get = |k: &str, d: &str| db::get_setting(conn, k).and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| d.into());
    let mut keys = Keys { toggle: get("shortcut_toggle", DEFAULT_TOGGLE), dump: get("shortcut_dump", DEFAULT_DUMP) };
    let mut moved = Vec::new();
    for (which, cur, default) in [("toggle", &mut keys.toggle, DEFAULT_TOGGLE), ("dump", &mut keys.dump, DEFAULT_DUMP)] {
        if in_app_key(cur) {
            moved.push(serde_json::json!({ "which": which, "from": cur.clone(), "to": default }));
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
        moved.push(serde_json::json!({ "which": which, "from": cur.clone(), "to": default }));
        *cur = default.to_string();
        let _ = db::set_setting(conn, &format!("shortcut_{which}"), &serde_json::json!(default));
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
    let (edge, peek, new) = {
        let conn = state.db.lock().unwrap();
        // An older backup may come from before the styles: it gets their defaults too.
        let _ = look::migrate(&conn);
        look::changed(app, &conn);
        (
            db::get_setting(&conn, "edge_open").and_then(|v| v.as_bool()).unwrap_or(false),
            db::get_setting(&conn, "edge_peek").and_then(|v| v.as_bool()).unwrap_or(false),
            stored_keys(&conn),
        )
    };
    state.edge_open.store(edge, Ordering::Relaxed);
    state.edge_peek.store(peek, Ordering::Relaxed);
    let gs = app.global_shortcut();
    let mut guard = state.keys.lock().unwrap();
    let k = &mut *guard;
    for (cur, want) in [(&mut k.toggle, new.toggle), (&mut k.dump, new.dump)] {
        if *cur == want {
            continue;
        }
        let (Ok(old), Ok(next)) = (parse_keys(cur), parse_keys(&want)) else { continue };
        let _ = gs.unregister(old);
        if gs.register(next).is_ok() {
            *cur = want;
        } else {
            let _ = gs.register(old);
        }
    }
}

pub fn run() {
    // Where the notes are, decided before anything else (an update put in place right below
    // keeps its notes there). Setup finds it decided.
    paths::init_early();
    // A downloaded update goes in place before anything else starts (then it starts instead).
    if updates::apply_at_start() {
        return;
    }
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(url) = args.iter().find(|a| a.starts_with(&format!("{}:", notify::SCHEME))) {
                handle_url(app, url);
            } else {
                window::show(app);
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
        .plugin(tauri_plugin_dialog::init())
        // "Start with Windows": an entry named after the app ("Dobo"), set here rather than taken
        // from the product name, so it can't change by itself. (The old "ZToDo" one: below.)
        .plugin(tauri_plugin_autostart::Builder::new().app_name(APP_NAME).arg("--hidden").build())
        .setup(move |app| {
            // Where the notes are (installed, portable, or the notes this computer already had):
            // decided once, here, before any window (WebView2's folder depends on it).
            let place = paths::init(&app.path().app_data_dir()?);
            let dir = place.dir.clone();
            std::fs::create_dir_all(&dir)?;
            // Pictures in a portable data folder show too (the config allows $APPDATA's).
            for folder in ["pictures", "previews", "backgrounds"] {
                let _ = app.asset_protocol_scope().allow_directory(dir.join(folder), true);
            }
            // The main window (hidden until shown below), as early as before it was made here:
            // WebView2 starts loading the page while the notes open. (Its calls wait for the end
            // of setup, when everything they use is ready.)
            window::create_main(app.handle())?;
            let conn = db::open(&dir.join(paths::DB_FILE)).map_err(std::io::Error::other)?;
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
            // Online only once the terms of use are accepted (an installed copy: by Setup).
            consent::start_with(&conn);
            let edge = db::get_setting(&conn, "edge_open").and_then(|v| v.as_bool()).unwrap_or(false);
            let peek = db::get_setting(&conn, "edge_peek").and_then(|v| v.as_bool()).unwrap_or(false);
            let keys = stored_keys(&conn);
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
            });
            first_run(app.handle()).map_err(std::io::Error::other)?;
            purge_unused_pictures(app.handle());
            // "Start with Windows" set under the working name (ZToDo) becomes Dobo's: the old
            // entry would start a ZToDo.exe that is gone. (Release builds only: a development
            // run never changes the real one.)
            #[cfg(not(debug_assertions))]
            rename::take_old_start(|| {
                app.autolaunch().enable().map_err(|e| eprintln!("Start with Windows couldn't be carried over to Dobo: {e}")).is_ok()
            });

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

            window::create_dump(app.handle())?;
            // The mouse's Back / Forward buttons when mouse software sends them as app commands.
            if let Some(w) = app.get_webview_window("main") {
                navkeys::hook(&w);
            }

            let menu = tray_menu(app.handle(), &keys)?;
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
            tray::watch(app.handle().clone());
            // (A change between the icon above and the watch: caught here.)
            tray::refresh(app.handle());

            reminders::start(app.handle().clone());
            window::start_edge_watch(app.handle().clone());
            // Installed copy: Windows Apps shows the version really installed (Dobo's own
            // updates replace Dobo.exe without the installer).
            install::sync_apps_entry();
            start_update_checks(app.handle().clone());

            let args: Vec<String> = std::env::args().collect();
            if let Some(url) = args.iter().find(|a| a.starts_with(&format!("{}:", notify::SCHEME))) {
                handle_url(app.handle(), url);
            } else if !args.iter().any(|a| a == "--hidden") {
                window::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|win, event| {
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
                WindowEvent::ScaleFactorChanged { .. } | WindowEvent::ThemeChanged(_) => tray::refresh(win.app_handle()),
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    let app = win.app_handle().clone();
                    let st = app.state::<AppState>();
                    st.moves.store(db::now_ms().max(0) as u64, Ordering::SeqCst);
                    // One saver for a whole drag: it waits until the window stays still 600 ms.
                    if st.saving.swap(true, Ordering::SeqCst) {
                        return;
                    }
                    std::thread::spawn(move || {
                        let st = app.state::<AppState>();
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            let last = st.moves.load(Ordering::SeqCst) as i64;
                            if db::now_ms() - last >= 600 {
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
            import_sticky_notes,
            window_info,
            hide_panel,
            apply_window,
            set_float_on_top,
            list_monitors,
            answer_ring,
            tidy_answer,
            rhythm_presets,
            show_dump,
            hide_dump,
            dump_item,
            fetch_preview,
            set_item_link,
            open_path,
            open_url,
            save_picture,
            add_picture_file,
            delete_picture,
            release_picture,
            set_shortcut,
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
            update_restart
        ])
        .build(tauri::generate_context!())
        .expect("error while running Dobo");
    app.run(|_app, event| {
        // Quitting normally: a new version on trial whose window loaded worked.
        if let tauri::RunEvent::Exit = event {
            updates::quit_normally();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_fixed_in_app_keys_are_understood() {
        for k in ["Ctrl+Shift+S", "Ctrl+Shift+P", "Ctrl+Comma"] {
            assert!(parse_keys(k).is_ok(), "{k}");
        }
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
    fn programs_are_never_started_from_a_file_card() {
        for p in ["C:/x/setup.EXE", "C:/x/run.bat", "C:/x/a.lnk", "C:/x/s.ps1", "C:/x/i.msi", "C:/x/w.url", "/Applications/A.app", "/x/y.sh"] {
            assert!(runs_something(Path::new(p)), "{p}");
        }
        for p in ["C:/x/report.pdf", "C:/x/notes.txt", "C:/x/photo.JPG", "C:/x/folder", "C:/x/sheet.xlsx"] {
            assert!(!runs_something(Path::new(p)), "{p}");
        }
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
