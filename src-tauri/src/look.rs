// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The board's look (Milestone 4): the settings it keeps, the dark-mode switch "By hours"
//! (decided on this clock, so both windows flip together), the .ztheme files (export and a
//! strict import: a file from elsewhere is untrusted) and your picture as a background.
//!
//! The styles themselves are settings (JSON) written by the page; this side makes sure an old
//! data folder gets sensible defaults once, and that nothing malformed comes in from a file.

use crate::{AppState, db};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

pub const LOOK_VERSION: i64 = 1;
/// A look file is a few kilobytes; anything bigger is refused before reading.
pub const ZTHEME_MAX: u64 = 512 * 1024;
/// Your picture as a background (the same limit as pictures in notes).
const PICTURE_MAX: u64 = 40_000_000;

// ---- Settings: first run of Milestone 4 on an existing folder ----

fn hex_to_rgb(hex: &str) -> Option<String> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(format!("{},{},{}", (n >> 16) & 255, (n >> 8) & 255, n & 255))
}

/// The soft palette's own dots: a custom note colour equal to one of them isn't "yours".
const SOFT_DOTS: [&str; 10] = ["#B5B0A5", "#D9A928", "#E08A55", "#C9727A", "#8D7BCB", "#5D8BC2", "#4F98A6", "#6F9C67", "#B48A57", "#7E9596"];

/// Once per data folder: the dark-mode switch starts on Follow Windows (as before), and the
/// note colours you made yourself become the first of "Your colours". Everything else is kept
/// as it is; a damaged value is put back to its default. Safe to run again.
pub fn migrate(conn: &Connection) -> Result<(), String> {
    let version = db::get_setting(conn, "look_version").and_then(|v| v.as_i64()).unwrap_or(0);
    // Damaged values (not the shape they should have) go back to their default.
    for (key, want_array) in [("look_styles", true), ("look_themes", true), ("look_mixes", true), ("your_colours", true), ("look_board", false), ("dark_mode", false)] {
        if let Some(v) = db::get_setting(conn, key) {
            let ok = if want_array { v.is_array() } else { v.is_object() };
            if !ok {
                conn.execute("DELETE FROM settings WHERE key = ?1", [key]).map_err(|e| e.to_string())?;
            }
        }
    }
    if version >= LOOK_VERSION {
        return Ok(());
    }
    if db::get_setting(conn, "dark_mode").is_none() {
        db::set_setting(conn, "dark_mode", &serde_json::json!({ "mode": "windows", "from": "19:00", "until": "07:00" }))?;
    }
    if db::get_setting(conn, "your_colours").is_none() {
        let mut colours: Vec<serde_json::Value> = Vec::new();
        if let Some(serde_json::Value::Array(list)) = db::get_setting(conn, "custom_palette") {
            let now = db::now_ms();
            for (i, c) in list.iter().enumerate() {
                let Some(dot) = c.get("dot").and_then(|d| d.as_str()) else { continue };
                if SOFT_DOTS.iter().any(|s| s.eq_ignore_ascii_case(dot)) {
                    continue;
                }
                if let Some(rgb) = hex_to_rgb(dot)
                    && !colours.iter().any(|x| x["rgb"] == rgb)
                {
                    colours.push(serde_json::json!({ "rgb": rgb, "at": now - i as i64 }));
                }
            }
        }
        db::set_setting(conn, "your_colours", &serde_json::Value::Array(colours))?;
    }
    db::set_setting(conn, "look_version", &serde_json::json!(LOOK_VERSION))
}

// ---- Dark mode ----

pub(crate) fn valid_hhmm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5 && b[2] == b':' && s[..2].parse::<u8>().is_ok_and(|h| h < 24) && s[3..].parse::<u8>().is_ok_and(|m| m < 60)
}

/// "HH:MM" inside [from, until); the window may wrap past midnight (19:00–07:00).
pub fn in_hours(now: &str, from: &str, until: &str) -> bool {
    if from == until {
        return false;
    }
    if from < until { now >= from && now < until } else { now >= from || now < until }
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct DarkNow {
    pub mode: String,
    pub from: String,
    pub until: String,
    /// None: follow Windows (the page knows that one).
    pub dark: Option<bool>,
}

pub fn dark_now(conn: &Connection, now_ms: i64) -> DarkNow {
    let v = db::get_setting(conn, "dark_mode").unwrap_or(serde_json::Value::Null);
    let get = |k: &str, d: &str| v.get(k).and_then(|x| x.as_str()).filter(|s| k == "mode" || valid_hhmm(s)).unwrap_or(d).to_string();
    let mode = get("mode", "windows");
    let from = get("from", "19:00");
    let until = get("until", "07:00");
    let dark = match mode.as_str() {
        "light" => Some(false),
        "dark" => Some(true),
        "hours" => Some(in_hours(&crate::clock::local_hhmm(now_ms), &from, &until)),
        _ => None,
    };
    DarkNow { mode, from, until, dark }
}

/// 0 unknown, 1 follow Windows, 2 light, 3 dark: what was last told to the windows.
static LAST: AtomicU8 = AtomicU8::new(0);

fn code(d: &DarkNow) -> u8 {
    match d.dark {
        None => 1,
        Some(false) => 2,
        Some(true) => 3,
    }
}

/// From the clock (every few seconds): when the hours cross a boundary, both windows flip.
pub fn tick(app: &AppHandle, now_ms: i64) {
    let d = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        dark_now(&conn, now_ms)
    };
    if LAST.swap(code(&d), Ordering::Relaxed) != code(&d) {
        let _ = app.emit("dark-mode", d);
    }
}

/// The switch was changed: tell both windows now (not at the next tick).
pub fn changed(app: &AppHandle, conn: &Connection) {
    let d = dark_now(conn, db::now_ms());
    LAST.store(code(&d), Ordering::Relaxed);
    let _ = app.emit("dark-mode", d);
}

// ---- .ztheme files ----

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Wc {
    colours: Vec<String>,
    pattern: String,
    seed: i64,
    paper: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Look {
    bg: String,
    #[serde(default)]
    colour: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    custom: Option<String>,
    held: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wc: Option<Wc>,
    /// Never taken from a file, never written into one.
    #[serde(default, skip_serializing)]
    picture: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    id: String,
    name: String,
    light: Look,
    #[serde(default)]
    dark: Option<Look>,
    #[serde(default)]
    at: f64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Style {
    id: String,
    name: String,
    look: Look,
    #[serde(default)]
    at: f64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Mix {
    id: String,
    name: String,
    colours: Vec<String>,
    pattern: String,
    paper: String,
    #[serde(default)]
    at: f64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Colour {
    rgb: String,
    #[serde(default)]
    at: f64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DarkMode {
    mode: String,
    from: String,
    until: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ZTheme {
    ztheme: u32,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    #[serde(default)]
    themes: Vec<Theme>,
    #[serde(default)]
    styles: Vec<Style>,
    #[serde(default)]
    mixes: Vec<Mix>,
    #[serde(default)]
    colours: Vec<Colour>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dark_mode: Option<DarkMode>,
}

const BACKGROUNDS: [&str; 12] = ["plain", "paper", "dotted", "cork", "kraft", "terrazzo", "watercolour", "blueprint", "felt", "linen", "plaster", "picture"];
const HELD: [&str; 4] = ["auto", "pins", "tape", "none"];
const PATTERNS: [&str; 4] = ["Scattered", "Sweep", "Corners", "One patch"];
const PAPERS: [&str; 2] = ["white", "dark"];

fn bad(what: &str) -> String {
    tr!("That file isn’t a Dobo look ({}).", "Ce fichier n’est pas un style Dobo ({}).", what)
}

fn plain_text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(|c| c.is_control())
}

fn rgb_ok(s: &str) -> bool {
    let parts: Vec<&str> = s.split(',').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u16>().is_ok_and(|n| n <= 255))
}

fn hex_ok(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn check_look(l: &mut Look) -> Result<(), String> {
    if !BACKGROUNDS.contains(&l.bg.as_str()) {
        return Err(bad(&tr!("an unknown background", "un fond inconnu")));
    }
    if !HELD.contains(&l.held.as_str()) {
        return Err(bad(&tr!("an unknown way to hold notes", "une façon inconnue de fixer les notes")));
    }
    if l.colour.chars().count() > 40 || l.colour.chars().any(|c| c.is_control()) {
        return Err(bad(&tr!("a colour name", "un nom de couleur")));
    }
    if let Some(c) = &l.custom
        && !hex_ok(c)
    {
        return Err(bad(&tr!("a colour", "une couleur")));
    }
    if let Some(w) = &l.wc {
        if w.colours.is_empty() || w.colours.len() > 3 || !w.colours.iter().all(|c| rgb_ok(c)) {
            return Err(bad(&tr!("watercolour colours", "les couleurs d’aquarelle")));
        }
        if !PATTERNS.contains(&w.pattern.as_str()) || !PAPERS.contains(&w.paper.as_str()) || !(1..2147483647).contains(&w.seed) {
            return Err(bad(&tr!("a watercolour painting", "une peinture à l’aquarelle")));
        }
        if let Some(n) = &w.name
            && !plain_text(n, 60)
        {
            return Err(bad(&tr!("a mix name", "un nom de mélange")));
        }
    }
    // A path from another computer means nothing here (and is never trusted).
    l.picture = None;
    Ok(())
}

/// Reads a .ztheme strictly: every field known, every value checked, sizes bounded. Returns the
/// cleaned file (as JSON) — pictures never come in.
pub fn validate(text: &str) -> Result<ZTheme, String> {
    if text.len() as u64 > ZTHEME_MAX {
        return Err(tr!("That file is too big to be a Dobo look.", "Ce fichier est trop gros pour être un style Dobo."));
    }
    let mut z: ZTheme = serde_json::from_str(text).map_err(|_| bad(&tr!("it can’t be read", "il ne peut pas être lu")))?;
    if z.ztheme != 1 {
        return Err(tr!("That look was made by another version of Dobo.", "Ce style vient d’une autre version de Dobo."));
    }
    if !["theme", "themes", "everything"].contains(&z.kind.as_str()) {
        return Err(bad(&tr!("an unknown kind", "un type inconnu")));
    }
    if z.themes.len() > 200 || z.styles.len() > 500 || z.mixes.len() > 200 || z.colours.len() > 500 {
        return Err(bad(&tr!("too many things in it", "trop de choses dedans")));
    }
    if let Some(a) = &z.app
        && !plain_text(a, 40)
    {
        return Err(bad(&tr!("its maker", "son auteur")));
    }
    let id_ok = |s: &str| !s.is_empty() && s.len() <= 80 && !s.chars().any(|c| c.is_control());
    for t in &mut z.themes {
        if !id_ok(&t.id) || !plain_text(&t.name, 60) || !t.at.is_finite() {
            return Err(bad(&tr!("a theme", "un thème")));
        }
        check_look(&mut t.light)?;
        if let Some(d) = &mut t.dark {
            check_look(d)?;
        }
    }
    for s in &mut z.styles {
        if !id_ok(&s.id) || !plain_text(&s.name, 60) || !s.at.is_finite() {
            return Err(bad(&tr!("a style", "un style")));
        }
        check_look(&mut s.look)?;
    }
    for m in &z.mixes {
        if !id_ok(&m.id) || !plain_text(&m.name, 60) || !m.at.is_finite() {
            return Err(bad(&tr!("a mix", "un mélange")));
        }
        if m.colours.is_empty() || m.colours.len() > 3 || !m.colours.iter().all(|c| rgb_ok(c)) || !PATTERNS.contains(&m.pattern.as_str()) || !PAPERS.contains(&m.paper.as_str()) {
            return Err(bad(&tr!("a mix", "un mélange")));
        }
    }
    for c in &z.colours {
        if !rgb_ok(&c.rgb) || !c.at.is_finite() {
            return Err(bad(&tr!("a colour", "une couleur")));
        }
    }
    if let Some(d) = &z.dark_mode
        && (!["windows", "hours", "light", "dark"].contains(&d.mode.as_str()) || !valid_hhmm(&d.from) || !valid_hhmm(&d.until))
    {
        return Err(bad(&tr!("dark-mode hours", "les heures du mode sombre")));
    }
    if z.themes.is_empty() && z.styles.is_empty() && z.mixes.is_empty() && z.colours.is_empty() {
        return Err(tr!("There is nothing to add in that file.", "Il n’y a rien à ajouter dans ce fichier."));
    }
    Ok(z)
}

fn read_checked(path: &std::path::Path) -> Result<String, String> {
    let ext_ok = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("ztheme"));
    if !ext_ok {
        return Err(tr!("Only .ztheme files can be added here.", "Seuls les fichiers .ztheme peuvent être ajoutés ici."));
    }
    let meta = std::fs::metadata(path).map_err(|_| tr!("That file can’t be opened.", "Ce fichier ne peut pas être ouvert."))?;
    if !meta.is_file() || meta.len() > ZTHEME_MAX {
        return Err(tr!("That file is too big to be a Dobo look.", "Ce fichier est trop gros pour être un style Dobo."));
    }
    let text = std::fs::read_to_string(path).map_err(|_| bad(&tr!("it can’t be read", "il ne peut pas être lu")))?;
    let z = validate(&text)?;
    serde_json::to_string(&z).map_err(|e| e.to_string())
}

/// Saves a look file where you choose (the system's Save dialog). Only a valid look is written.
#[tauri::command]
pub async fn export_look(app: AppHandle, content: String, name: String) -> Result<Option<String>, String> {
    let z = validate(&content)?;
    let text = serde_json::to_string_pretty(&z).map_err(|e| e.to_string())?;
    let safe: String = name.chars().filter(|c| !"\\/:*?\"<>|".contains(*c) && !c.is_control()).take(80).collect();
    let picked = app
        .dialog()
        .file()
        .add_filter(tr!("Dobo look", "Style Dobo"), &["ztheme"])
        .set_file_name(if safe.trim().is_empty() { tr!("My look.ztheme", "Mon style.ztheme") } else { safe })
        .blocking_save_file();
    let Some(p) = picked else { return Ok(None) };
    let mut path = p.into_path().map_err(|e| e.to_string())?;
    if path.extension().is_none() {
        path.set_extension("ztheme");
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Reads a look file: the one dropped on the board, or one picked in the system's Open dialog.
/// Returns the checked contents (nothing is added until you say so on the page).
#[tauri::command]
pub async fn import_look(app: AppHandle, path: Option<String>) -> Result<Option<(String, String)>, String> {
    let path = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let picked = app.dialog().file().add_filter(tr!("Dobo look", "Style Dobo"), &["ztheme"]).blocking_pick_file();
            match picked {
                Some(p) => p.into_path().map_err(|e| e.to_string())?,
                None => return Ok(None),
            }
        }
    };
    let text = read_checked(&path)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Some((name, text)))
}

// ---- Your picture ----

const PICTURE_EXT: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Picks a picture for the board and keeps a copy next to the notes (the original stays where
/// it is). Pictures no style uses any more are cleared away.
#[tauri::command]
pub async fn pick_background_picture(app: AppHandle) -> Result<Option<String>, String> {
    let picked = app.dialog().file().add_filter(tr!("Pictures", "Images"), &PICTURE_EXT).blocking_pick_file();
    let Some(p) = picked else { return Ok(None) };
    let src = p.into_path().map_err(|e| e.to_string())?;
    let ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    if !PICTURE_EXT.contains(&ext.as_str()) {
        return Err(tr!("That file isn’t a picture Dobo can show.", "Ce fichier n’est pas une image que Dobo peut afficher."));
    }
    let len = std::fs::metadata(&src).map(|m| m.len()).map_err(|_| tr!("That picture can’t be opened.", "Cette image ne peut pas être ouverte."))?;
    if len > PICTURE_MAX {
        return Err(tr!("That picture is too big for the board (40 MB at most).", "Cette image est trop lourde pour le tableau (40 Mo au plus)."));
    }
    let dir = crate::paths::sub("backgrounds")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(format!("{}.{}", db::now_ms(), if ext == "jpeg" { "jpg" } else { &ext }));
    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}

/// The background pictures the looks use (the board's, saved styles' and themes'): each file's
/// name, for the clean-up at start (lib.rs), which keeps the others a day. (Never cleaned when a
/// picture is picked: Undo may still bring back the look that used the one before.) A setting
/// that can't be read is an error, never "no picture used".
pub fn background_pictures(conn: &Connection) -> Result<Vec<String>, String> {
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    match x.as_str() {
                        Some(s) if k == "picture" && !s.is_empty() => out.push(s.to_string()),
                        _ => walk(x, out),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = vec![];
    for key in ["look_board", "look_styles", "look_themes"] {
        if let Some(v) = db::get_setting_strict(conn, key)? {
            walk(&v, &mut out);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let dir = std::env::temp_dir().join(format!("dobo-look-{}-{:?}", db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        db::open(&dir.join("t.sqlite")).unwrap()
    }

    #[test]
    fn an_old_folder_gets_the_new_settings_once_and_keeps_its_own() {
        let c = conn();
        db::set_setting(&c, "note_size", &serde_json::json!("large")).unwrap();
        db::set_setting(
            &c,
            "custom_palette",
            &serde_json::json!([{ "name": "Paper", "dot": "#B5B0A5" }, { "name": "Mine", "dot": "#C8643C" }, { "name": "Again", "dot": "#c8643c" }]),
        )
        .unwrap();
        migrate(&c).unwrap();
        assert_eq!(db::get_setting(&c, "note_size"), Some(serde_json::json!("large")));
        assert_eq!(db::get_setting(&c, "dark_mode").unwrap()["mode"], "windows");
        let yours = db::get_setting(&c, "your_colours").unwrap();
        assert_eq!(yours.as_array().unwrap().len(), 1, "the soft palette's own dot isn't yours; a repeat comes once");
        assert_eq!(yours[0]["rgb"], "200,100,60");
        assert_eq!(db::get_setting(&c, "look_version"), Some(serde_json::json!(LOOK_VERSION)));
        // Running again changes nothing the person chose since.
        db::set_setting(&c, "dark_mode", &serde_json::json!({ "mode": "hours", "from": "20:00", "until": "06:30" })).unwrap();
        migrate(&c).unwrap();
        assert_eq!(db::get_setting(&c, "dark_mode").unwrap()["mode"], "hours");
    }

    #[test]
    fn damaged_look_settings_go_back_to_their_default() {
        let c = conn();
        db::set_setting(&c, "look_styles", &serde_json::json!("oops")).unwrap();
        db::set_setting(&c, "look_board", &serde_json::json!([1, 2])).unwrap();
        db::set_setting(&c, "look_themes", &serde_json::json!([{ "id": "t1" }])).unwrap();
        migrate(&c).unwrap();
        assert_eq!(db::get_setting(&c, "look_styles"), None);
        assert_eq!(db::get_setting(&c, "look_board"), None);
        // An array stays (the page reads each entry carefully).
        assert!(db::get_setting(&c, "look_themes").is_some());
    }

    #[test]
    fn saved_styles_and_themes_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("dobo-restart-{}", db::now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.sqlite");
        let style = serde_json::json!([{ "id": "s1", "name": "My cork", "look": { "bg": "cork", "colour": "Honey", "held": "auto" }, "at": 1 }]);
        {
            let c = db::open(&file).unwrap();
            migrate(&c).unwrap();
            db::set_setting(&c, "look_styles", &style).unwrap();
        }
        let c = db::open(&file).unwrap();
        migrate(&c).unwrap();
        assert_eq!(db::get_setting(&c, "look_styles"), Some(style));
        drop(c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_background_picture_a_look_uses_is_found() {
        let c = conn();
        db::set_setting(&c, "look_board", &serde_json::json!({ "light": { "bg": "picture", "picture": "C:/d/backgrounds/1.jpg" }, "dark": { "bg": "cork" } })).unwrap();
        db::set_setting(&c, "look_styles", &serde_json::json!([{ "id": "s1", "look": { "bg": "picture", "picture": "C:/d/backgrounds/2.png" } }, { "id": "s2", "look": { "picture": "" } }])).unwrap();
        db::set_setting(&c, "look_themes", &serde_json::json!([{ "id": "t1", "light": { "bg": "cork" }, "dark": { "bg": "picture", "picture": "C:/d/backgrounds/3.webp" } }])).unwrap();
        let mut found = background_pictures(&c).unwrap();
        found.sort();
        assert_eq!(found, vec!["C:/d/backgrounds/1.jpg", "C:/d/backgrounds/2.png", "C:/d/backgrounds/3.webp"]);
    }

    #[test]
    fn dark_by_hours_wraps_past_midnight() {
        assert!(in_hours("21:40", "19:00", "07:00"));
        assert!(in_hours("03:00", "19:00", "07:00"));
        assert!(!in_hours("07:00", "19:00", "07:00"));
        assert!(!in_hours("12:00", "19:00", "07:00"));
        assert!(in_hours("13:00", "12:00", "14:00"));
        assert!(!in_hours("14:00", "12:00", "14:00"));
        assert!(!in_hours("10:00", "10:00", "10:00"));
    }

    #[test]
    fn the_switch_is_read_with_care() {
        let c = conn();
        assert_eq!(dark_now(&c, 0).dark, None);
        db::set_setting(&c, "dark_mode", &serde_json::json!({ "mode": "dark", "from": "x", "until": "07:00" })).unwrap();
        let d = dark_now(&c, 0);
        assert_eq!(d.dark, Some(true));
        assert_eq!(d.from, "19:00", "a damaged time falls back");
        db::set_setting(&c, "dark_mode", &serde_json::json!({ "mode": "light" })).unwrap();
        assert_eq!(dark_now(&c, 0).dark, Some(false));
    }

    const GOOD: &str = r#"{"ztheme":1,"kind":"everything",
      "themes":[{"id":"t1","name":"Cork & kraft","light":{"bg":"cork","colour":"Natural","held":"auto"},
                 "dark":{"bg":"watercolour","colour":"","held":"none","wc":{"colours":["60,175,190","232,125,115"],"pattern":"Sweep","seed":42,"paper":"dark","name":"Teal sunset"}},"at":5}],
      "styles":[{"id":"s1","name":"Picture day","look":{"bg":"picture","colour":"","held":"tape","picture":"C:\\Users\\someone\\secret.jpg"},"at":1}],
      "mixes":[{"id":"m1","name":"Mine","colours":["216,96,150"],"pattern":"Corners","paper":"white","at":2}],
      "colours":[{"rgb":"216,96,150","at":3}],
      "dark_mode":{"mode":"hours","from":"19:00","until":"07:00"}}"#;

    #[test]
    fn a_good_file_is_read_and_its_picture_paths_dropped() {
        let z = validate(GOOD).unwrap();
        assert_eq!(z.themes.len(), 1);
        assert_eq!(z.styles[0].look.picture, None);
        let back = serde_json::to_string(&z).unwrap();
        assert!(!back.contains("secret.jpg"));
        // The cleaned file reads again (export then import gives the same).
        let again = validate(&back).unwrap();
        assert_eq!(serde_json::to_string(&again).unwrap(), back);
    }

    #[test]
    fn untrusted_files_are_refused() {
        let cases = [
            ("not json", "{"),
            ("an array", "[]"),
            ("another version", r#"{"ztheme":2,"kind":"theme","colours":[{"rgb":"1,2,3"}]}"#),
            ("an unknown field", r#"{"ztheme":1,"kind":"theme","colours":[{"rgb":"1,2,3"}],"run":"calc.exe"}"#),
            ("an unknown field in a look", r#"{"ztheme":1,"kind":"theme","themes":[{"id":"a","name":"x","light":{"bg":"cork","colour":"Natural","held":"auto","onload":"x"}}]}"#),
            ("an unknown background", r#"{"ztheme":1,"kind":"theme","themes":[{"id":"a","name":"x","light":{"bg":"<script>","colour":"","held":"auto"}}]}"#),
            ("a bad rgb", r#"{"ztheme":1,"kind":"theme","colours":[{"rgb":"300,2,3"}]}"#),
            ("a bad custom colour", r#"{"ztheme":1,"kind":"theme","themes":[{"id":"a","name":"x","light":{"bg":"cork","colour":"Custom","custom":"red","held":"auto"}}]}"#),
            ("four colours", r#"{"ztheme":1,"kind":"theme","mixes":[{"id":"m","name":"x","colours":["1,2,3","1,2,3","1,2,3","1,2,3"],"pattern":"Sweep","paper":"white"}]}"#),
            ("a control character", "{\"ztheme\":1,\"kind\":\"theme\",\"themes\":[{\"id\":\"a\",\"name\":\"x\\u0007\",\"light\":{\"bg\":\"cork\",\"colour\":\"\",\"held\":\"auto\"}}]}"),
            ("an empty name", r#"{"ztheme":1,"kind":"theme","themes":[{"id":"a","name":"  ","light":{"bg":"cork","colour":"","held":"auto"}}]}"#),
            ("a bad seed", r#"{"ztheme":1,"kind":"theme","themes":[{"id":"a","name":"x","light":{"bg":"watercolour","colour":"","held":"auto","wc":{"colours":["1,2,3"],"pattern":"Sweep","seed":0,"paper":"white"}}}]}"#),
            ("bad hours", r#"{"ztheme":1,"kind":"everything","colours":[{"rgb":"1,2,3"}],"dark_mode":{"mode":"hours","from":"25:00","until":"07:00"}}"#),
            ("nothing in it", r#"{"ztheme":1,"kind":"everything"}"#),
        ];
        for (why, text) in cases {
            assert!(validate(text).is_err(), "{why} should be refused");
        }
        let huge = format!(r#"{{"ztheme":1,"kind":"theme","app":"{}"}}"#, "a".repeat(ZTHEME_MAX as usize));
        assert!(validate(&huge).is_err(), "too big");
        let many = format!(r#"{{"ztheme":1,"kind":"everything","colours":[{}]}}"#, vec![r#"{"rgb":"1,2,3"}"#; 501].join(","));
        assert!(validate(&many).is_err(), "too many");
    }

    #[test]
    fn only_ztheme_files_are_read() {
        let dir = std::env::temp_dir().join(format!("dobo-z-{}", db::now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("Lea's look.ztheme");
        std::fs::write(&good, GOOD).unwrap();
        assert!(read_checked(&good).is_ok());
        let other = dir.join("notes.txt");
        std::fs::write(&other, GOOD).unwrap();
        assert!(read_checked(&other).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
