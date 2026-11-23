// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Where Dobo keeps its notes: one decision, made once at start, that every path goes through
//! (the database, pictures, link previews, backgrounds, backups, the notification secret and
//! icon, temporary files, and WebView2's own folder).
//!
//! The same Dobo.exe runs two ways:
//! - **Installed** (the installer put a `.dobo-installed` file next to it): the app's usual data
//!   folder (%APPDATA%\com.sookera.ztodo on Windows). WebView2 keeps its own data where it always
//!   does (%LOCALAPPDATA%\com.sookera.ztodo\EBWebView).
//! - **Portable** (no such file): a `data` folder next to the exe, WebView2's data inside it too
//!   (`data\webview`), so the folder can go on a USB stick and leaves nothing behind. When that
//!   folder can't be written (Program Files, a read-only drive), the usual data folder instead,
//!   but never silently: the start asks first (`blocked`; the recovery screen, recovery.rs), so
//!   notes are never split without a word.
//!
//! **Notes already on this computer are never left behind nor split.** Before the portable
//! mode existed, every copy kept its notes in the usual data folder. So a portable copy whose
//! `data` folder has no notes yet, on a computer where the usual folder has some, keeps using
//! the usual folder (and WebView2's usual folder): nothing is moved, and the choice holds by
//! itself (it is made again the same way at every start, from what is there). A `data` folder
//! that already holds notes is always the one used.
//!
//! **The usual folder keeps the app's first name.** The app was called ZToDo while it was being
//! made, and its bundle identifier, `com.sookera.ztodo` (tauri.conf.json), names the usual data
//! folder (%APPDATA%\com.sookera.ztodo) and WebView2's (%LOCALAPPDATA%\com.sookera.ztodo). The
//! notes people already have are there, so the identifier stays as it is: renaming it would
//! open an empty board and leave their notes behind. The notes file keeps its name too
//! (`ztodo.sqlite`), and so do the saved settings' keys.
//!
//! Debug builds use `<project>/data` (WebView2's data in it too), so a development run never
//! opens, changes or migrates the real notes; `data/` is ignored by git.

// (A debug build always uses `<project>/data`: the decision is compiled for release builds and
// the tests.)
#![cfg_attr(debug_assertions, allow(dead_code))]

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file the installer writes next to Dobo.exe: this copy is installed, not portable.
pub const INSTALLED_MARKER: &str = ".dobo-installed";
/// In the usual data folder: the portable copies that use the notes there (paths that keep using
/// them, one per line), so uninstalling the installed copy with "Also delete my notes" keeps them
/// (install.rs `uninstall_cleanup`).
pub const PORTABLE_MARKER: &str = ".dobo-portable-copies";
/// The notes' database (its presence is what "this folder holds notes" means). Named before the
/// app was called Dobo; it keeps that name so the notes already there are found.
pub const DB_FILE: &str = "ztodo.sqlite";
/// The portable data folder's name, next to the exe.
const DATA: &str = "data";
/// The bundle identifier (tauri.conf.json, checked by a test): it names the usual data folder.
pub const IDENTIFIER: &str = match option_env!("DOBO_TEST_IDENTIFIER") {
    // A renamed test copy only (see APP_NAME in lib.rs): its notes in a folder of its own.
    Some(i) => i,
    None => "com.sookera.ztodo",
};

/// The usual data folder (%APPDATA%\com.sookera.ztodo on Windows), worked out the way Tauri's
/// `app_data_dir` does (the system's data folder + the identifier), so it is known before Tauri
/// starts: for an update put in place at the very start, and for Setup's `--install-choices` and
/// the uninstaller's `--uninstall-cleanup`, which run without the app.
pub fn usual_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(IDENTIFIER))
}

/// WebView2's usual folder for the installed copy (%LOCALAPPDATA%\com.sookera.ztodo, Tauri's
/// default: the system's local data folder + the identifier).
pub fn usual_webview_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join(IDENTIFIER))
}

/// Decides the data folder now (once) when the usual one is known, before Tauri: the same
/// decision `init` makes in setup, which then finds it made.
pub fn init_early() -> Option<&'static Place> {
    usual_dir().map(|u| init(&u))
}

/// This copy was installed by Dobo Setup.
pub fn is_installed() -> bool {
    place().is_some_and(|p| p.kind == Kind::Installed)
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Installed,
    Portable,
    /// A development build (`<project>/data`).
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    Dev,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub kind: Kind,
    /// The data folder.
    pub dir: PathBuf,
    /// WebView2's own folder, when it isn't the system's usual one (a portable `data` folder or
    /// a development build). None: the usual one, as before.
    pub webview: Option<PathBuf>,
    /// Portable, but using the notes this computer already had in the usual data folder.
    pub kept_existing: bool,
    /// Portable, but the folder next to the exe can't be written: the usual data folder.
    pub fallback: bool,
    /// That folder next to the exe (asked about at start before the usual one is used).
    pub blocked: Option<PathBuf>,
}

fn has_marker(dir: &Path) -> bool {
    dir.join(INSTALLED_MARKER).is_file()
}

fn has_notes(dir: &Path) -> bool {
    dir.join(DB_FILE).is_file()
}

/// The decision itself (no side effects of its own; `writable` may create the folder it is
/// asked about). `exe_dir`: the exe's folder; `usual`: the app's usual data folder.
pub fn decide(exe_dir: Option<&Path>, usual: &Path, writable: impl Fn(&Path) -> bool) -> Place {
    let at_usual = |kept_existing: bool, fallback: bool, kind: Kind| Place { kind, dir: usual.to_path_buf(), webview: None, kept_existing, fallback, blocked: None };
    let Some(exe) = exe_dir else { return at_usual(false, true, Kind::Portable) };
    // Installed: the usual folder, and nothing is ever created next to the exe.
    if has_marker(exe) {
        return at_usual(false, false, Kind::Installed);
    }
    let next = exe.join(DATA);
    let portable = || Place { kind: Kind::Portable, webview: Some(next.join("webview")), dir: next.clone(), kept_existing: false, fallback: false, blocked: None };
    let blocked = || Place { blocked: Some(next.clone()), ..at_usual(false, true, Kind::Portable) };
    // A portable data folder that already holds notes is the one; when it can't be written (a
    // locked USB stick), the usual folder only once asked (never a half-working one).
    if has_notes(&next) {
        return if writable(&next) { portable() } else { blocked() };
    }
    // Notes already on this computer (from before the portable mode): kept where they are.
    if has_notes(usual) {
        return at_usual(true, false, Kind::Portable);
    }
    if writable(&next) { portable() } else { blocked() }
}

/// Can files be written in this folder? Creates it when missing; a folder it had to create
/// and couldn't use is removed again (nothing left behind next to a read-only exe).
pub fn is_writable(dir: &Path) -> bool {
    let existed = dir.is_dir();
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    if !ok && !existed {
        let _ = std::fs::remove_dir(dir);
    }
    ok
}

/// Debug builds: `<project>/data`.
#[cfg(debug_assertions)]
fn dev_place() -> Place {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.parent().unwrap_or(&manifest).join(DATA);
    Place { kind: Kind::Dev, webview: Some(dir.join("webview")), dir, kept_existing: false, fallback: false, blocked: None }
}

static PLACE: OnceLock<Place> = OnceLock::new();

/// Works out the data folder (once, at start, before any window) and creates it.
#[cfg_attr(debug_assertions, allow(unused_variables))]
pub fn init(usual: &Path) -> &'static Place {
    PLACE.get_or_init(|| {
        #[cfg(debug_assertions)]
        let p = dev_place();
        #[cfg(not(debug_assertions))]
        let p = {
            let exe = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
            decide(exe.as_deref(), usual, is_writable)
        };
        let _ = std::fs::create_dir_all(&p.dir);
        p
    })
}

/// A portable copy (`exe`) that uses the usual data folder (`place`: the notes this computer
/// already had, or the usual folder chosen at a start): noted there for the uninstaller, once,
/// with when it was last seen (each start of it: a "# seen <seconds> <path>" line, which Dobo
/// 1.0 reads as a remark).
pub fn note_portable_copy(place: &Place, exe: &Path) {
    if place.kind != Kind::Portable || place.webview.is_some() {
        return;
    }
    note_seen(&place.dir.join(PORTABLE_MARKER), exe, unix_now());
}

fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// What the line `l` says when it says when a copy was last seen: (seconds, the copy).
fn seen_line(l: &str) -> Option<(u64, &str)> {
    let (secs, exe) = l.strip_prefix("# seen ")?.split_once(' ')?;
    Some((secs.parse().ok()?, exe.trim()))
}

/// How often a copy's "seen" line is written again: once a day is enough to count 180 days.
const SEEN_EVERY: u64 = 24 * 3600;

/// `exe` noted in `file` (once), seen at `now`. Not written again when it was seen less than a
/// day ago (`SEEN_EVERY`); else written whole beside it, then put in its place (the uninstaller
/// reads it: never half-written).
fn note_seen(file: &Path, exe: &Path, now: u64) {
    let had = std::fs::read_to_string(file).unwrap_or_default();
    let line = exe.to_string_lossy();
    let line = line.trim();
    let noted = had.lines().any(|l| l.trim().eq_ignore_ascii_case(line));
    let lately = had.lines().filter_map(seen_line).any(|(t, e)| e.eq_ignore_ascii_case(line) && t <= now && now - t < SEEN_EVERY);
    if noted && lately {
        return;
    }
    let mut out = String::new();
    if had.trim().is_empty() {
        out.push_str("# Portable copies of Dobo that use the notes in this folder: uninstalling the installed Dobo keeps them.\r\n");
    }
    let mut listed = false;
    for l in had.lines() {
        // (Its own line saying when it was seen: written again below.)
        if seen_line(l).is_some_and(|(_, e)| e.eq_ignore_ascii_case(line)) || l.trim().is_empty() {
            continue;
        }
        listed |= l.trim().eq_ignore_ascii_case(line);
        out.push_str(l);
        out.push_str("\r\n");
    }
    if !listed {
        out.push_str(&format!("{line}\r\n"));
    }
    out.push_str(&format!("# seen {now} {line}\r\n"));
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = file.with_file_name(format!("{name}.tmp"));
    if std::fs::write(&tmp, out).and_then(|()| std::fs::rename(&tmp, file)).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// How long a copy noted (`note_portable_copy`) but not found still counts, when its drive is gone
/// or can be taken out (a USB stick in a drawer): 180 days since it was last seen.
const STILL_COUNTS: u64 = 180 * 24 * 3600;

/// Is the drive of `path` gone (a USB stick pulled out, a share not there), or one that can be
/// taken out?
fn drive_away_or_removable(path: &Path) -> bool {
    let Some(std::path::Component::Prefix(prefix)) = path.components().next() else { return false };
    let mut root = PathBuf::from(prefix.as_os_str());
    root.push("\\");
    if !root.exists() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: a NUL-terminated path.
        let kind = unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr()) };
        kind == windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOVABLE
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// The portable copies noted in `dir` (`note_portable_copy`) that still use its notes: the copy
/// is still there, isn't the installed one, and hasn't a data folder with notes of its own. One
/// that isn't found counts all the same when its drive is gone or can be taken out (a USB stick
/// not plugged in now), until 180 days after it was last seen.
pub fn portable_copies_using(dir: &Path) -> Vec<PathBuf> {
    portable_copies_using_at(dir, unix_now(), drive_away_or_removable)
}

fn portable_copies_using_at(dir: &Path, now: u64, away: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let file = dir.join(PORTABLE_MARKER);
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    // (A copy noted before the "seen" lines: when the file was last written.)
    let written = std::fs::metadata(&file).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let seen = |exe: &str| text.lines().filter_map(seen_line).filter(|(_, e)| e.eq_ignore_ascii_case(exe)).map(|(s, _)| s).max().unwrap_or(written);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| {
            let exe = Path::new(l);
            if exe.is_file() {
                return exe.parent().is_some_and(|d| !has_marker(d) && !has_notes(&d.join(DATA)));
            }
            away(exe) && now.saturating_sub(seen(l)) < STILL_COUNTS
        })
        .map(PathBuf::from)
        .collect()
}

/// Where the data is (set by `init` at start).
pub fn place() -> Option<&'static Place> {
    PLACE.get()
}

/// The data folder: the database, pictures, previews, backgrounds, backups (by default).
pub fn data_dir() -> Result<PathBuf, String> {
    place().map(|p| p.dir.clone()).ok_or_else(|| "The data folder isn’t known yet.".to_string())
}

/// A folder inside the data folder ("pictures", "previews", "backgrounds", "backups").
pub fn sub(name: &str) -> Result<PathBuf, String> {
    data_dir().map(|d| d.join(name))
}

/// Short-lived files (reading Windows Sticky Notes): inside the data folder when it has its own
/// WebView2 folder too (portable: nothing written outside it), else the app's usual cache.
pub fn temp_dir(usual_cache: Result<PathBuf, String>) -> Result<PathBuf, String> {
    match place() {
        Some(p) if p.webview.is_some() => Ok(p.dir.join("cache")),
        _ => usual_cache,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dobo-paths-{tag}-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// An exe folder and the usual data folder, side by side in a temp folder.
    fn setup(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = temp(tag);
        let exe = root.join("Apps").join("Dobo");
        let usual = root.join("Roaming").join("com.sookera.ztodo");
        std::fs::create_dir_all(&exe).unwrap();
        (root, exe, usual)
    }

    fn put_notes(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(DB_FILE), b"notes").unwrap();
    }

    #[test]
    fn installed_uses_the_usual_folder_and_never_touches_the_exe_folder() {
        let (root, exe, usual) = setup("installed");
        std::fs::write(exe.join(INSTALLED_MARKER), b"Installed by Dobo Setup.").unwrap();
        let p = decide(Some(&exe), &usual, |_| panic!("an installed copy must not try the exe folder"));
        assert_eq!(p, Place { kind: Kind::Installed, dir: usual.clone(), webview: None, kept_existing: false, fallback: false, blocked: None });
        // Old notes there: the same (that's where they already are).
        put_notes(&usual);
        assert_eq!(decide(Some(&exe), &usual, |_| panic!()).dir, usual);
        assert!(!exe.join(DATA).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_named_like_the_marker_is_not_the_marker() {
        let (root, exe, usual) = setup("marker-dir");
        std::fs::create_dir_all(exe.join(INSTALLED_MARKER)).unwrap();
        assert_eq!(decide(Some(&exe), &usual, is_writable).kind, Kind::Portable);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_new_portable_copy_on_a_clean_computer_uses_data_next_to_the_exe() {
        let (root, exe, usual) = setup("fresh");
        let p = decide(Some(&exe), &usual, is_writable);
        assert_eq!(p.kind, Kind::Portable);
        assert_eq!(p.dir, exe.join(DATA));
        assert_eq!(p.webview, Some(exe.join(DATA).join("webview")));
        assert!(!p.kept_existing && !p.fallback);
        assert!(exe.join(DATA).is_dir());
        assert!(!usual.exists(), "nothing is written in the usual folder");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_portable_copy_that_cannot_write_next_to_itself_uses_the_usual_folder() {
        let (root, exe, usual) = setup("readonly");
        let p = decide(Some(&exe), &usual, |_| false);
        assert_eq!(p, Place { kind: Kind::Portable, dir: usual.clone(), webview: None, kept_existing: false, fallback: true, blocked: Some(exe.join(DATA)) });
        // Really unwritable: "data" can't be made where a file of that name stands.
        std::fs::write(exe.join(DATA), b"a file, not a folder").unwrap();
        assert!(!is_writable(&exe.join(DATA)));
        assert_eq!(decide(Some(&exe), &usual, is_writable).dir, usual);
        assert!(exe.join(DATA).is_file(), "left as it was");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn notes_already_on_this_computer_are_kept_in_use_not_moved_nor_split() {
        let (root, exe, usual) = setup("kept");
        put_notes(&usual);
        let p = decide(Some(&exe), &usual, is_writable);
        assert_eq!(p, Place { kind: Kind::Portable, dir: usual.clone(), webview: None, kept_existing: true, fallback: false, blocked: None });
        assert!(!exe.join(DATA).exists(), "no empty data folder is made beside them");
        assert!(usual.join(DB_FILE).is_file(), "nothing is moved");
        // The next start decides the same.
        assert_eq!(decide(Some(&exe), &usual, is_writable), p);
        // An empty data folder made by hand doesn't take over either.
        std::fs::create_dir_all(exe.join(DATA)).unwrap();
        assert_eq!(decide(Some(&exe), &usual, is_writable).dir, usual);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The copy most people have today: ZToDo.exe (the app's working name) alone in a folder of
    /// its own (no marker, no `data`), and the notes (with their WAL, backups and link secret) in
    /// the usual folder. Dobo.exe put in its place, or in a new folder of its own, opens them
    /// where they are, with WebView2's usual folder (None: Tauri's default, as before), without
    /// even trying to write next to the exe.
    #[test]
    fn a_copy_run_before_the_portable_mode_keeps_every_path_it_had() {
        let root = temp("packaged");
        let exe = root.join("Apps").join("Packaged").join("Dobo");
        std::fs::create_dir_all(&exe).unwrap();
        std::fs::write(exe.join("Dobo.exe"), b"MZ").unwrap();
        let usual = root.join("AppData").join("Roaming").join("com.sookera.ztodo");
        put_notes(&usual);
        for f in ["ztodo.sqlite-wal", "ztodo.sqlite-shm", "link-secret", "notification-icon.png"] {
            std::fs::write(usual.join(f), b"x").unwrap();
        }
        std::fs::create_dir_all(usual.join("backups")).unwrap();
        let p = decide(Some(&exe), &usual, |d| panic!("must not try to write in {d:?}"));
        assert_eq!(p, Place { kind: Kind::Portable, dir: usual.clone(), webview: None, kept_existing: true, fallback: false, blocked: None });
        assert_eq!(p.dir.join(DB_FILE), usual.join("ztodo.sqlite"));
        for (sub, want) in [("pictures", "pictures"), ("previews", "previews"), ("backgrounds", "backgrounds"), ("backups", "backups")] {
            assert_eq!(p.dir.join(sub), usual.join(want));
        }
        let left: Vec<_> = std::fs::read_dir(&exe).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left, vec![std::ffi::OsString::from("Dobo.exe")], "nothing is made next to the exe");
        // The old folder, with ZToDo.exe still in it and Dobo.exe beside it: the same.
        let old = root.join("Apps").join("Packaged").join("ZToDo");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("ZToDo.exe"), b"MZ").unwrap();
        std::fs::write(old.join("Dobo.exe"), b"MZ").unwrap();
        assert_eq!(decide(Some(&old), &usual, |d| panic!("must not try to write in {d:?}")), p);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The usual folder is still the one named by the identifier the app had before its name:
    /// the notes are there (the identifier is in tauri.conf.json, which must not change it).
    #[test]
    fn the_usual_folder_keeps_the_identifier_where_the_notes_are() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], "com.sookera.ztodo");
        assert_eq!(conf["identifier"], IDENTIFIER);
        assert_eq!(conf["mainBinaryName"], "Dobo");
        // Known before Tauri starts, the way Tauri's app_data_dir works it out.
        if let Some(u) = usual_dir() {
            assert!(u.ends_with(IDENTIFIER));
        }
        // Dobo Setup's marker and data folders say the same (installer/app.nsh).
        let nsh = include_str!("../installer/app.nsh");
        assert!(nsh.contains(&format!("!define APP_MARKER \"{INSTALLED_MARKER}\"")));
        assert!(nsh.contains(&format!("!define APP_IDENTIFIER \"{IDENTIFIER}\"")));
        assert!(nsh.contains(&format!("!define APP_DATA_DIR \"$APPDATA\\{IDENTIFIER}\"")));
        assert!(nsh.contains(&format!("!define APP_WEBVIEW_DIR \"$LOCALAPPDATA\\{IDENTIFIER}\"")));
        assert_eq!(DB_FILE, "ztodo.sqlite");
        assert_eq!(INSTALLED_MARKER, ".dobo-installed");
        // Nothing else moves them: Tauri's folders follow the identifier alone (no override), and
        // WebView2's is Tauri's default (%LOCALAPPDATA%\<identifier>) unless a portable or
        // development data folder gives its own (window.rs), never one set in the config.
        assert!(conf["app"]["appDirectoriesOverride"].is_null());
        for w in conf["app"]["windows"].as_array().unwrap() {
            assert!(w["dataDirectory"].is_null(), "{w}");
        }
    }

    /// A marker from the working name (`.ztodo-installed`) never shipped: it doesn't make a copy
    /// installed (it is portable, and finds the notes this computer has, as any portable copy).
    #[test]
    fn only_the_dobo_marker_means_installed() {
        let (root, exe, usual) = setup("oldmarker");
        std::fs::write(exe.join(".ztodo-installed"), b"x").unwrap();
        put_notes(&usual);
        let p = decide(Some(&exe), &usual, is_writable);
        assert_eq!(p, Place { kind: Kind::Portable, dir: usual.clone(), webview: None, kept_existing: true, fallback: false, blocked: None });
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_data_folder_that_holds_notes_is_always_the_one() {
        let (root, exe, usual) = setup("existing");
        put_notes(&exe.join(DATA));
        let p = decide(Some(&exe), &usual, is_writable);
        assert_eq!(p.dir, exe.join(DATA));
        assert_eq!(p.webview, Some(exe.join(DATA).join("webview")));
        // Even with notes in the usual folder too (a portable folder brought from another PC).
        put_notes(&usual);
        assert_eq!(decide(Some(&exe), &usual, is_writable).dir, exe.join(DATA));
        // Unless it can't be written (a read-only stick): the usual folder, never a half-working
        // one, and only once asked (`blocked`: the notes would be split otherwise).
        let ro = decide(Some(&exe), &usual, |_| false);
        assert_eq!((ro.dir, ro.fallback, ro.kept_existing, ro.blocked), (usual.clone(), true, false, Some(exe.join(DATA))));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unknown_exe_folder_uses_the_usual_folder() {
        let (root, _exe, usual) = setup("noexe");
        let p = decide(None, &usual, |_| panic!());
        assert_eq!(p.dir, usual);
        assert!(p.fallback);
        assert_eq!(p.blocked, None, "no folder next to it to speak of");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A portable copy using the usual folder leaves its path there for the uninstaller; one
    /// that is gone, installed since, or has notes of its own next to it no longer counts.
    #[test]
    fn portable_copies_using_the_usual_folder_are_noted_for_the_uninstaller() {
        let (root, exe, usual) = setup("marker");
        put_notes(&usual);
        let dobo = exe.join("Dobo.exe");
        std::fs::write(&dobo, b"MZ").unwrap();
        let p = decide(Some(&exe), &usual, is_writable);
        assert!(p.kept_existing);
        note_portable_copy(&p, &dobo);
        note_portable_copy(&p, &dobo);
        let text = std::fs::read_to_string(usual.join(PORTABLE_MARKER)).unwrap();
        assert_eq!(text.lines().filter(|l| !l.starts_with('#')).count(), 1, "once");
        assert_eq!(portable_copies_using(&usual), vec![dobo.clone()]);
        // A portable copy with its own data folder, or a development build: nothing noted.
        let own = Place { kind: Kind::Portable, dir: exe.join(DATA), webview: Some(exe.join(DATA).join("webview")), kept_existing: false, fallback: false, blocked: None };
        note_portable_copy(&own, &root.join("Other").join("Dobo.exe"));
        assert!(!usual.join(DATA).join(PORTABLE_MARKER).exists() && !exe.join(DATA).join(PORTABLE_MARKER).exists());
        // Notes of its own next to it since: no longer.
        put_notes(&exe.join(DATA));
        assert!(portable_copies_using(&usual).is_empty());
        std::fs::remove_dir_all(exe.join(DATA)).unwrap();
        // Installed there since, or gone: no longer.
        std::fs::write(exe.join(INSTALLED_MARKER), b"x").unwrap();
        assert!(portable_copies_using(&usual).is_empty());
        std::fs::remove_file(exe.join(INSTALLED_MARKER)).unwrap();
        assert_eq!(portable_copies_using(&usual).len(), 1);
        std::fs::remove_file(&dobo).unwrap();
        assert!(portable_copies_using(&usual).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A copy noted that isn't found counts when its drive is gone or can be taken out (a USB
    /// stick in a drawer), for 180 days after it was last seen; on a fixed drive, not at all.
    #[test]
    fn a_portable_copy_on_a_stick_that_is_out_still_counts_for_a_while() {
        let root = temp("stick");
        let usual = root.join("usual");
        std::fs::create_dir_all(&usual).unwrap();
        let file = usual.join(PORTABLE_MARKER);
        let (stick, fixed) = (Path::new(r"Q:\Dobo\Dobo.exe"), root.join("Gone").join("Dobo.exe"));
        let day = 24 * 3600;
        note_seen(&file, stick, 1_000 * day);
        note_seen(&file, &fixed, 1_000 * day);
        note_seen(&file, stick, 1_010 * day);
        // Seen again within the day: not written again (nor ever half-written: no .tmp left).
        let before = std::fs::read_to_string(&file).unwrap();
        note_seen(&file, stick, 1_010 * day + 3600);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
        assert!(!file.with_file_name(format!("{PORTABLE_MARKER}.tmp")).exists());
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(text.lines().filter(|l| !l.starts_with('#')).count(), 2, "{text}");
        assert_eq!(text.lines().filter(|l| l.starts_with("# seen ")).count(), 2, "one each, the last: {text}");
        let on_stick = |p: &Path| p.starts_with("Q:\\");
        assert_eq!(portable_copies_using_at(&usual, 1_100 * day, on_stick), vec![stick.to_path_buf()]);
        assert_eq!(portable_copies_using_at(&usual, 1_189 * day, on_stick), vec![stick.to_path_buf()]);
        assert!(portable_copies_using_at(&usual, 1_191 * day, on_stick).is_empty(), "not seen for 180 days");
        // (The temp folder's own drive, fixed: never away; a path with no drive neither.)
        assert!(!drive_away_or_removable(&root.join("x.exe")));
        assert!(!drive_away_or_removable(Path::new("relative.exe")));
        // A drive letter with no drive: away (when Q: isn't one here).
        if !Path::new("Q:\\").exists() {
            assert!(drive_away_or_removable(stick));
        }
        // Dobo 1.0's file (no "seen" lines): counts from when the file was last written, for 180
        // days.
        std::fs::write(&file, format!("# Portable copies\r\n{}\r\n", stick.display())).unwrap();
        assert_eq!(portable_copies_using_at(&usual, unix_now(), on_stick), vec![stick.to_path_buf()]);
        assert_eq!(portable_copies_using_at(&usual, unix_now() + 179 * day, on_stick), vec![stick.to_path_buf()]);
        assert!(portable_copies_using_at(&usual, unix_now() + 181 * day, on_stick).is_empty(), "written 180 days ago");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_write_test_leaves_nothing_behind() {
        let root = temp("probe");
        let d = root.join("new");
        assert!(is_writable(&d));
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
