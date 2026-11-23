// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Dobo Setup's side of things (the installed copy; the portable one never needs these). From the
//! Sookera installer template's app-side/install.rs, with Dobo's own data folders (paths.rs).
//!
//! - `Dobo.exe --uninstall-cleanup [--delete-data]`: run by the uninstaller before it removes
//!   Dobo's files. Removes what this copy registered with Windows: start with Windows, and the
//!   `ztodo:` links its notifications use (with their name and icon). No window. With
//!   `--delete-data` (the uninstaller's "Also delete my notes", off by default and never in a
//!   silent uninstall) it also deletes the notes (%APPDATA%\com.sookera.ztodo) and WebView2's
//!   data (%LOCALAPPDATA%\com.sookera.ztodo), unless another Dobo is open or a portable copy uses
//!   these notes (paths.rs `PORTABLE_MARKER`). What changes from one
//!   version to the next is done here, not in the uninstaller: a Dobo that updated itself keeps
//!   the uninstaller of the version first installed, but this exe is always the current one.
//! - `Dobo.exe --install-choices --start-with-windows=0|1 [--terms-accepted=1] [--lang=en|fr]`:
//!   run by Dobo Setup's window once Dobo is installed (never by a silent or passive install).
//!   Saves Setup's choices with the notes and does them now. No window. `--terms-accepted=1`:
//!   Install was clicked under "By installing, you accept the terms of use", so the first start
//!   doesn't ask. `--lang=en|fr` (template 1.1.0): the language picked in Setup's window, saved
//!   as Dobo's language so Dobo and its first-start setup open in it. Sent only when a language
//!   was picked there (template 1.1.1): without it the setting stays as it is (following Windows
//!   until changed in Dobo, or what a portable copy over the same notes already chose).
//! - At each start of an installed copy: the Windows Apps entry shows the version really
//!   installed (Dobo's own updates replace Dobo.exe without the installer).
//! - The installed copy tells Dobo Setup the language it speaks (`tell_setup`): once at start and
//!   whenever it changes, so the uninstaller and the next Setup over it speak it too.

use std::path::{Path, PathBuf};

/// The argument the uninstaller starts Dobo.exe with.
pub const UNINSTALL_CLEANUP: &str = "--uninstall-cleanup";
/// With it: also delete the notes ("Also delete my notes").
pub const DELETE_DATA: &str = "--delete-data";
/// Exit code: another Dobo is open (a portable copy), or a portable copy uses these notes (it
/// noted so in the folder), so the notes were kept. The uninstaller then keeps the rest too and
/// says why.
pub const KEPT_FOR_OTHER_COPY: i32 = 2;
/// Exit code: some of the data couldn't be deleted (the uninstaller tries again itself).
pub const NOT_ALL_DELETED: i32 = 3;

/// The argument Dobo Setup starts Dobo.exe with for its choices.
pub const INSTALL_CHOICES: &str = "--install-choices";

/// The setting `--install-choices` saves: Setup's "Start Dobo with Windows", which Dobo's first
/// start then shows as it is (FirstRun.svelte) instead of its own default.
pub const INSTALLER_START: &str = "installer_start";

/// `--name=1` → Some(true), `--name=0` → Some(false), missing → None.
fn choice(args: &[String], name: &str) -> Option<bool> {
    let prefix = format!("{name}=");
    args.iter().find_map(|a| a.strip_prefix(&prefix)).map(|v| v.trim() == "1")
}

/// `--lang=en` / `--lang=fr` → Some("en") / Some("fr"); missing or anything else → None (an older
/// Setup, or one that leaves the language as it is: Dobo's own setting stays).
fn lang(args: &[String]) -> Option<&'static str> {
    match args.iter().find_map(|a| a.strip_prefix("--lang="))?.trim() {
        "en" => Some("en"),
        "fr" => Some("fr"),
        _ => None,
    }
}

/// `args`: the arguments after the exe (the first is `INSTALL_CHOICES`). 0 = done; 1 = "Start
/// Dobo with Windows" couldn't be set (what Setup then says: APP_CHOICES_FAILED). Choices that
/// couldn't be saved with the notes (notes that won't open, left for the app's own start to say)
/// are asked again at the first start (the terms) or left as they are (the language): Setup
/// doesn't say so, its words being about start with Windows.
pub fn install_choices(args: &[String]) -> i32 {
    let start = choice(args, "--start-with-windows");
    let terms = (choice(args, "--terms-accepted") == Some(true)).then(crate::terms::version).flatten();
    let language = lang(args);
    // The installed copy's notes: Setup wrote the marker next to this exe, so it's the usual folder.
    if let Some(place) = crate::paths::init_early()
        && let Err(e) = save_choices(&place.dir, start, terms.as_deref(), language)
    {
        eprintln!("Setup's choices weren't saved with the notes: {e}");
    }
    // (Done whatever became of the rest: it is what Setup's message is about.)
    match start {
        Some(on) if !set_autostart(on) => 1,
        _ => 0,
    }
}

/// Saves Setup's choices with the notes (the database is made when there's none yet: a new
/// install; its first start then finds them). Quick, whatever the notes (Setup waits for it): the
/// notes are opened as they are, never read through, backed up or changed but for these settings.
/// The new version's backup before it first opens them (backup.rs `before_update`), the columns
/// it adds and any trouble with them are left to the app's own start, which has its board and its
/// recovery screen. (Nor is `.clean-exit` written: recovery.rs.)
fn save_choices(dir: &Path, start: Option<bool>, terms: Option<&str>, language: Option<&str>) -> Result<(), String> {
    use serde_json::json;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = dir.join(crate::paths::DB_FILE);
    let conn = if file.exists() {
        crate::recovery::open_as_is(&file)?
    } else if crate::recovery::needs_a_look(dir) {
        // (Notes an earlier start put aside, or a journal left alone: never a new file in their
        // place without the recovery screen.)
        return Err("left for the app's start".into());
    } else {
        // A new install: new notes, opened by this version (nothing to back up before it does).
        let conn = crate::db::open(&file)?;
        crate::db::set_setting(&conn, crate::backup::OPENED_BY, &json!(env!("CARGO_PKG_VERSION")))?;
        conn
    };
    // An acceptance already saved is kept as it is.
    if let Some(date) = terms
        && crate::consent::needed(&conn)
    {
        crate::db::set_setting(&conn, "terms_accepted", &json!(date))?;
    }
    if let Some(v) = start {
        crate::db::set_setting(&conn, INSTALLER_START, &json!(v))?;
    }
    // The language Setup spoke: Dobo's own setting ("system", "en", "fr"; i18n.rs), the one its
    // first-start setup starts from (FirstRun.svelte) and Settings shows.
    if let Some(l) = language {
        crate::db::set_setting(&conn, "language", &json!(l))?;
    }
    Ok(())
}

/// Setup's "Start with Windows": the same entry the app's own switch makes (`enable_autostart`).
/// Off: only this copy's entry (or one whose program is gone) goes; a portable Dobo elsewhere
/// keeps starting with Windows.
#[cfg(windows)]
fn set_autostart(on: bool) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    if !on {
        let entry = reg::get(RUN_KEY, Some(crate::APP_NAME));
        if may_remove_autostart(entry.as_deref(), &exe) {
            reg::delete_value(RUN_KEY, crate::APP_NAME);
        }
        return true;
    }
    enable_autostart().is_ok()
}

#[cfg(not(windows))]
fn set_autostart(_on: bool) -> bool {
    true
}

/// The argument the entry starts Dobo with (in the tray, no window). Setup and the uninstaller
/// look for it too (app.nsh: APP_START_ARG).
pub const START_ARG: &str = "--hidden";

/// The program as a Run entry should name it: in quotes. Unquoted, Windows tries
/// `C:\Users\Jo\Downloads\Dobo.exe` first for `C:\Users\Jo\Downloads\Dobo portable\Dobo.exe`.
fn quoted(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// "Start with Windows" for this exe: the value "Dobo" in the current user's Run key,
/// `"<exe>" --hidden`, and Task Manager's own on/off for it set back to on (as
/// tauri-plugin-autostart did, through the same crate).
#[cfg(windows)]
pub fn enable_autostart() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut b = auto_launch::AutoLaunchBuilder::new();
    // (The current user's entry only, as a per-user install: never the whole PC's.)
    b.set_app_name(crate::APP_NAME)
        .set_app_path(&quoted(&exe))
        .set_args(&[START_ARG])
        .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    b.build().and_then(|a| a.enable()).map_err(|e| e.to_string())
}

/// The app's switch turned off: Dobo's entry goes, whichever copy it starts (the switch showed
/// it on).
#[cfg(windows)]
pub fn disable_autostart() -> Result<(), String> {
    reg::delete_value(RUN_KEY, crate::APP_NAME);
    match reg::get(RUN_KEY, Some(crate::APP_NAME)) {
        Some(_) => Err(tr!("Start with Windows couldn’t be turned off.", "Le démarrage avec Windows n’a pas pu être désactivé.")),
        None => Ok(()),
    }
}

/// Does Dobo start with Windows (what the app's switch shows)? Its entry is there, and Task
/// Manager › Startup apps (or Settings › Apps › Startup) hasn't turned it off.
#[cfg(windows)]
pub fn autostart_enabled() -> bool {
    autostart_enabled_in(RUN_KEY, APPROVED_KEY)
}

#[cfg(windows)]
fn autostart_enabled_in(run_key: &str, approved_key: &str) -> bool {
    reg::get(run_key, Some(crate::APP_NAME)).is_some() && approved(reg::get_binary(approved_key, crate::APP_NAME).as_deref())
}

/// Task Manager's own on/off for a Run entry: none (never changed there) is on; otherwise its
/// first byte says it, bit 0 set being off (02 or 06 on, 03 or 07 off), whatever the time after
/// it (zero too when a tool other than Task Manager turned it off).
#[cfg_attr(not(windows), allow(dead_code))]
fn approved(value: Option<&[u8]>) -> bool {
    value.and_then(|v| v.first()).is_none_or(|&b| b & 1 == 0)
}

/// Where Task Manager keeps its on/off for the current user's Run entries.
#[cfg(windows)]
const APPROVED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// An entry this copy wrote before 1.1, unquoted (`C:\…\Dobo.exe --hidden`), quoted now. Another
/// copy's is left to that copy.
#[cfg(windows)]
pub fn quote_old_autostart() {
    if let Ok(exe) = std::env::current_exe() {
        quote_entry(RUN_KEY, &exe);
    }
}

#[cfg(windows)]
fn quote_entry(run_key: &str, exe: &Path) {
    if let Some(now) = reg::get(run_key, Some(crate::APP_NAME))
        && let Some(fixed) = requoted(&now, exe)
    {
        reg::set(run_key, crate::APP_NAME, &fixed);
    }
}

/// The entry with its program quoted, when it is this exe's and isn't quoted yet (its arguments
/// kept as they are).
fn requoted(command: &str, exe: &Path) -> Option<String> {
    let c = command.trim();
    if c.starts_with('"') || !starts(c, exe) {
        return None;
    }
    let program = entry_program(c)?;
    let rest = c[program.as_os_str().len()..].trim();
    Some(if rest.is_empty() { quoted(&program) } else { format!("{} {rest}", quoted(&program)) })
}

/// Windows' list of programs started at sign-in (current user).
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// The program a Run entry starts: `"C:\…\Dobo.exe" --hidden` or, as Dobo 1.0 wrote it (through
/// tauri-plugin-autostart), `C:\…\Dobo.exe --hidden` (unquoted).
pub(crate) fn entry_program(command: &str) -> Option<PathBuf> {
    let c = command.trim();
    if let Some(rest) = c.strip_prefix('"') {
        return rest.split_once('"').map(|(p, _)| PathBuf::from(p));
    }
    let at = c.to_ascii_lowercase().find(".exe")?;
    Some(PathBuf::from(&c[..at + 4]))
}

/// The entry starts this exe.
fn starts(command: &str, exe: &Path) -> bool {
    entry_program(command).is_some_and(|p| same_folder(&p.to_string_lossy(), exe))
}

/// This copy may take the entry away: it's its own, or its program is gone.
fn may_remove_autostart(entry: Option<&str>, exe: &Path) -> bool {
    let Some(entry) = entry else { return false };
    starts(entry, exe) || entry_program(entry).is_none_or(|p| !p.exists())
}

/// The uninstall entry Dobo Setup writes (per user: HKEY_CURRENT_USER only). Tauri names it after
/// the product name (checked by a test).
#[cfg_attr(not(windows), allow(dead_code))]
const APPS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Dobo";

/// `args`: the arguments after the exe (the first is `UNINSTALL_CLEANUP`).
pub fn uninstall_cleanup(args: &[String]) -> i32 {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        remove_registrations_if_mine(&exe);
    }
    let data = args.iter().any(|a| a == DELETE_DATA);
    if !data {
        return 0;
    }
    if other_copy_open() || crate::paths::usual_dir().is_some_and(|d| !crate::paths::portable_copies_using(&d).is_empty()) {
        return KEPT_FOR_OTHER_COPY;
    }
    let mut all = true;
    for dir in installed_data_folders() {
        if dir.exists() && std::fs::remove_dir_all(&dir).is_err() {
            all = false;
        }
    }
    if all { 0 } else { NOT_ALL_DELETED }
}

/// The installed copy's folders: the notes (with their pictures, backups and settings) and
/// WebView2's data. (A backups folder chosen in Settings stays: it's the person's own.)
fn installed_data_folders() -> Vec<PathBuf> {
    [crate::paths::usual_dir(), crate::paths::usual_webview_dir()].into_iter().flatten().collect()
}

/// Another Dobo runs (a portable copy). The installed one was just closed by the uninstaller:
/// its lock goes with its process, so this waits a moment for that.
fn other_copy_open() -> bool {
    for _ in 0..12 {
        if !crate::updates::another_instance_running() {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    true
}

/// Start with Windows, and the `ztodo:` links of the notifications (with the name and icon
/// Windows shows on them), where they point at this copy. Another copy's stay.
#[cfg(windows)]
fn remove_registrations_if_mine(exe: &Path) {
    if reg::get(RUN_KEY, Some(crate::APP_NAME)).is_some_and(|c| starts(&c, exe)) {
        reg::delete_value(RUN_KEY, crate::APP_NAME);
        reg::delete_value(APPROVED_KEY, crate::APP_NAME);
    }
    let proto = format!(r"Software\Classes\{}", crate::notify::SCHEME);
    if reg::get(&format!(r"{proto}\shell\open\command"), None).is_some_and(|c| starts(&c, exe)) {
        reg::delete_tree(&proto);
        reg::delete_tree(&format!(r"Software\Classes\AppUserModelId\{}", crate::notify::AUMID));
    }
}

/// Same folder, whatever the case, quotes, / or \ or a trailing one (as the template's, 1.1.1).
pub(crate) fn same_folder(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').replace('/', "\\").trim_end_matches('\\').to_lowercase();
    norm(a) == norm(&b.to_string_lossy())
}

/// The Apps entry belongs to this copy (its InstallLocation is this exe's folder) and shows
/// another version.
fn needs_new_version(install_location: Option<&str>, shown: Option<&str>, exe_dir: &Path, current: &str) -> bool {
    install_location.is_some_and(|l| same_folder(l, exe_dir)) && shown.map(str::trim) != Some(current)
}

/// At start, installed copies only: Windows Apps shows the version that is really installed.
pub fn sync_apps_entry() {
    #[cfg(windows)]
    {
        if !crate::paths::is_installed() {
            return;
        }
        let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else { return };
        let location = reg::get(APPS_KEY, Some("InstallLocation"));
        let shown = reg::get(APPS_KEY, Some("DisplayVersion"));
        let current = env!("CARGO_PKG_VERSION");
        if needs_new_version(location.as_deref(), shown.as_deref(), &dir, current) {
            reg::set(APPS_KEY, "DisplayVersion", current);
        }
    }
}

// ---------- Dobo Setup and the uninstaller follow Dobo's language (template 1.1.1) ----------
//
// Setup keeps the language it ran in as "Installer Language" (Windows' language number as text,
// the way NSIS writes it: "1033" English, "1036" French, REG_SZ) under its own key for Dobo,
// HKCU\Software\Sookera\Dobo (installer.nsi: MANUPRODUCTKEY), whose default value is the folder
// it installed Dobo in. The uninstaller speaks that language, and Setup opens in it over an
// installed Dobo (ui.nsh: APP_UN_LANG_INIT, AppGuiInit). So that both follow a language changed
// in Dobo later, the INSTALLED copy writes it there too: once at start and whenever it changes.
// Only the copy Setup installed: its marker file (paths.rs INSTALLED_MARKER) AND the key's
// default value is its own folder. A portable copy (no marker), or an installed-looking copy in
// another folder than the one Setup last installed, never writes it.

/// Dobo Setup's key for Dobo: Software\<publisher>\<productName> (tauri.conf.json; checked by a
/// test). Per user, as Dobo installs (`installMode: currentUser`).
#[cfg_attr(not(windows), allow(dead_code))]
const SETUP_KEY: &str = r"Software\Sookera\Dobo";
#[cfg_attr(not(windows), allow(dead_code))]
const SETUP_VALUE: &str = "Installer Language";

/// The value for a language: "1036" for French, "1033" for anything else.
fn setup_lcid(lang: &str) -> &'static str {
    if lang.trim().eq_ignore_ascii_case("fr") { "1036" } else { "1033" }
}

/// What to write, if anything: for the installed copy only (`installed_here`), and only when the
/// saved value says something else (also when it is missing: an install from before 1.1).
fn setup_value_to_write(lang: &str, saved: Option<&str>, installed_here: bool) -> Option<&'static str> {
    let want = setup_lcid(lang);
    (installed_here && saved.map(str::trim) != Some(want)).then_some(want)
}

/// Tells Dobo Setup the language Dobo speaks now, "en" or "fr": the language in use, so with the
/// setting on "follow Windows" it is Windows' (i18n::resolve). Called once at start (after the
/// notes' setting is read) and whenever the language changes (Settings, the first-start setup).
/// Does nothing in a portable copy or a development build (no marker next to the exe).
pub fn tell_setup(lang: &str) {
    #[cfg(windows)]
    {
        if !crate::paths::is_installed() {
            return;
        }
        let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else { return };
        tell_setup_in(SETUP_KEY, lang, &dir);
    }
    #[cfg(not(windows))]
    let _ = lang;
}

/// `tell_setup` under a given key (tests use a key of their own). True when it wrote the value.
#[cfg(windows)]
fn tell_setup_in(key: &str, lang: &str, exe_dir: &Path) -> bool {
    // The key's default value: the folder Setup installed Dobo in (missing: never installed).
    let here = reg::get(key, None).is_some_and(|d| same_folder(&d, exe_dir));
    let saved = reg::get(key, Some(SETUP_VALUE));
    match setup_value_to_write(lang, saved.as_deref(), here) {
        Some(value) => reg::set(key, SETUP_VALUE, value),
        None => false,
    }
}

/// The few registry calls this needs (current user only).
#[cfg(windows)]
pub(crate) mod reg {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegDeleteKeyW, RegDeleteTreeW, RegGetValueW, RegSetKeyValueW};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// A text value (None: the key's default value).
    pub fn get(key: &str, name: Option<&str>) -> Option<String> {
        let k = wide(key);
        let n = name.map(wide);
        let np = n.as_ref().map_or(std::ptr::null(), |n| n.as_ptr());
        let mut len = 0u32;
        // SAFETY: NUL-terminated strings; first the size, then the value into a buffer that big.
        unsafe {
            if RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), np, RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut len) != 0 {
                return None;
            }
            let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
            let mut size = (buf.len() * 2) as u32;
            if RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), np, RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size) != 0 {
                return None;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some(String::from_utf16_lossy(&buf[..end]))
        }
    }

    /// A binary value.
    pub fn get_binary(key: &str, name: &str) -> Option<Vec<u8>> {
        let (k, n) = (wide(key), wide(name));
        let mut len = 0u32;
        // SAFETY: NUL-terminated strings; first the size, then the value into a buffer that big.
        unsafe {
            if RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), RRF_RT_REG_BINARY, std::ptr::null_mut(), std::ptr::null_mut(), &mut len) != 0 {
                return None;
            }
            let mut buf = vec![0u8; len as usize];
            let mut size = len;
            if RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), RRF_RT_REG_BINARY, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size) != 0 {
                return None;
            }
            buf.truncate(size as usize);
            Some(buf)
        }
    }

    #[cfg(test)]
    pub fn set_binary(key: &str, name: &str, value: &[u8]) -> bool {
        let (k, n) = (wide(key), wide(name));
        // SAFETY: NUL-terminated strings; the size is the value's length in bytes.
        unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), windows_sys::Win32::System::Registry::REG_BINARY, value.as_ptr().cast(), value.len() as u32) == 0 }
    }

    pub fn set(key: &str, name: &str, value: &str) -> bool {
        let (k, n, v) = (wide(key), wide(name), wide(value));
        // SAFETY: NUL-terminated strings; the size in bytes includes the NUL.
        unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) == 0 }
    }

    pub fn delete_value(key: &str, name: &str) {
        let (k, n) = (wide(key), wide(name));
        // SAFETY: NUL-terminated strings. "Not there" is fine.
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr()) };
    }

    pub fn delete_tree(key: &str) {
        let k = wide(key);
        // SAFETY: a NUL-terminated string. The key's values and subkeys go, then the key itself
        // (empty by then). "Not there" is fine.
        unsafe {
            RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr());
            RegDeleteKeyW(HKEY_CURRENT_USER, k.as_ptr());
        }
    }

    /// A test's own key under HKCU\Software (`Software\DoboTests-<tag>-…`), never a real one:
    /// deleted when the test ends, also when it fails.
    #[cfg(test)]
    pub struct TestKey(pub String);

    #[cfg(test)]
    impl TestKey {
        pub fn new(tag: &str) -> Self {
            TestKey(format!(r"Software\DoboTests-{tag}-{}-{:?}", crate::db::now_ms(), std::thread::current().id()))
        }
    }

    #[cfg(test)]
    impl Drop for TestKey {
        fn drop(&mut self) {
            // (Only a test's own key, ever.)
            if self.0.starts_with(r"Software\DoboTests-") {
                delete_tree(&self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_setups_choices() {
        let a: Vec<String> = ["--install-choices", "--start-with-windows=0", "--terms-accepted=1"].map(String::from).to_vec();
        assert_eq!(choice(&a, "--start-with-windows"), Some(false));
        assert_eq!(choice(&a, "--terms-accepted"), Some(true));
        let silent: Vec<String> = ["--install-choices", "--start-with-windows=1"].map(String::from).to_vec();
        assert_eq!(choice(&silent, "--start-with-windows"), Some(true));
        assert_eq!(choice(&silent, "--terms-accepted"), None, "never accepted without Setup's window");
        assert_eq!(lang(&a), None, "no --lang: Dobo's language stays as it is");
        let fr: Vec<String> = ["--install-choices", "--start-with-windows=1", "--terms-accepted=1", "--lang=fr"].map(String::from).to_vec();
        assert_eq!(lang(&fr), Some("fr"));
        let en: Vec<String> = ["--install-choices", "--lang=en"].map(String::from).to_vec();
        assert_eq!(lang(&en), Some("en"));
        let odd: Vec<String> = ["--install-choices", "--lang=system"].map(String::from).to_vec();
        assert_eq!(lang(&odd), None, "only the two languages Setup speaks");
    }

    /// Dobo Setup names its Apps entry after the product name (Tauri's UNINSTKEY), and passes the
    /// arguments these functions read.
    #[test]
    fn apps_entry_and_arguments_match_setup() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let product = conf["productName"].as_str().unwrap();
        assert_eq!(APPS_KEY, format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product}"));
        assert_eq!(product, crate::APP_NAME);
        let hooks = include_str!("../installer/hooks.nsh");
        assert!(hooks.contains("--install-choices --start-with-windows=$AppStart --terms-accepted=1"));
        assert!(hooks.contains("StrCpy $1 '$1 --lang=$AppLang'"));
        assert!(hooks.contains("--uninstall-cleanup$AppUnArgs"));
        assert!(hooks.contains(r#"StrCpy $AppUnArgs " --delete-data""#));
        // The uninstaller's switch, word for word as promised.
        assert!(include_str!("../installer/English.nsh").contains(r#"!insertmacro AppEn deleteAppData "Also delete my notes""#));
        assert!(include_str!("../installer/French.nsh").contains(r#"!insertmacro AppFr deleteAppData "Supprimer aussi mes notes""#));
    }

    #[test]
    fn reads_the_program_of_a_start_entry() {
        assert_eq!(entry_program(r"C:\Users\me\AppData\Local\Programs\Dobo\Dobo.exe --hidden"), Some(PathBuf::from(r"C:\Users\me\AppData\Local\Programs\Dobo\Dobo.exe")));
        assert_eq!(entry_program(r#""D:\Apps\My Dobo\Dobo.exe" --hidden"#), Some(PathBuf::from(r"D:\Apps\My Dobo\Dobo.exe")));
        assert_eq!(entry_program(r"D:\Apps\Dobo\DOBO.EXE"), Some(PathBuf::from(r"D:\Apps\Dobo\DOBO.EXE")));
        assert_eq!(entry_program("nothing"), None);
        let exe = Path::new(r"C:\Users\me\AppData\Local\Programs\Dobo\Dobo.exe");
        assert!(starts(r"c:\users\me\appdata\local\programs\dobo\dobo.exe --hidden", exe));
        assert!(!starts(r"D:\Portable\Dobo\Dobo.exe --hidden", exe));
        // Another copy's entry whose program is there stays; one whose program is gone may go.
        let here = std::env::current_exe().unwrap();
        assert!(!may_remove_autostart(Some(&format!("{} --hidden", here.display())), exe));
        assert!(may_remove_autostart(Some(r"Z:\Gone\Dobo.exe --hidden"), exe));
        assert!(may_remove_autostart(Some(&format!("{} --hidden", exe.display())), exe));
        assert!(!may_remove_autostart(None, exe));
    }

    #[test]
    fn an_unquoted_entry_of_this_copy_is_quoted() {
        let exe = Path::new(r"C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe");
        assert_eq!(quoted(exe), r#""C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe""#);
        assert_eq!(
            requoted(r"C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe --hidden", exe).as_deref(),
            Some(r#""C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe" --hidden"#)
        );
        assert_eq!(requoted(r"c:\users\john smith\downloads\dobo portable\dobo.exe", exe).as_deref(), Some(r#""c:\users\john smith\downloads\dobo portable\dobo.exe""#));
        // Already quoted, or another copy's: left as it is.
        assert_eq!(requoted(r#""C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe" --hidden"#, exe), None);
        assert_eq!(requoted(r"D:\Other\Dobo.exe --hidden", exe), None);
        // A quoted entry is still read as this copy's.
        assert!(starts(r#""C:\Users\John Smith\Downloads\Dobo portable\Dobo.exe" --hidden"#, exe));
        // What the uninstaller and Setup look for (hooks.nsh, ui.nsh; APP_START_ARG in app.nsh).
        assert!(include_str!("../installer/app.nsh").contains(&format!(r#"!define APP_START_ARG "{START_ARG}""#)));
        let hooks = include_str!("../installer/hooks.nsh");
        assert!(hooks.contains(r#"${OrIf} $AppOtherRun == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" ${APP_START_ARG}""#));
        assert!(hooks.contains(r#"${OrIf} $AppOtherRun == "$INSTDIR\${MAINBINARYNAME}.exe ${APP_START_ARG}""#));
    }

    #[test]
    fn task_managers_off_is_read() {
        assert!(approved(None), "never changed in Task Manager");
        assert!(approved(Some(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), "turned on there");
        assert!(!approved(Some(&[3, 0, 0, 0, 0x5c, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0xd9, 0x01])), "turned off there (with the time it was)");
        assert!(approved(Some(&[6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), "turned on (another way of saying it)");
        assert!(!approved(Some(&[3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), "turned off, with no time");
        assert!(!approved(Some(&[7, 0, 0, 0, 0x5c, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0xd9, 0x01])), "turned off (another way of saying it)");
        assert!(!approved(Some(&[3])), "short, still off");
        assert!(approved(Some(&[])), "empty: nothing to say, on");
    }

    /// Turned off in Task Manager › Startup apps: the switch shows it off (under test keys of
    /// their own, never the real ones).
    #[cfg(windows)]
    #[test]
    fn the_switch_follows_task_manager() {
        let key = reg::TestKey::new("autostart");
        let root = &key.0;
        let (run, ok) = (format!(r"{root}\Run"), format!(r"{root}\Approved"));
        assert!(!autostart_enabled_in(&run, &ok), "no entry");
        assert!(reg::set(&run, crate::APP_NAME, r#""C:\Apps\Dobo\Dobo.exe" --hidden"#));
        assert!(autostart_enabled_in(&run, &ok), "an entry, never changed in Task Manager");
        assert!(reg::set_binary(&ok, crate::APP_NAME, &[3, 0, 0, 0, 0x5c, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0xd9, 0x01]));
        assert!(!autostart_enabled_in(&run, &ok), "turned off in Task Manager");
        assert!(reg::set_binary(&ok, crate::APP_NAME, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert!(autostart_enabled_in(&run, &ok), "turned back on there");
    }

    /// A test's own registry key goes when the test ends, also when it fails.
    #[cfg(windows)]
    #[test]
    fn a_test_key_is_deleted_even_when_the_test_fails() {
        let name = std::sync::Mutex::new(String::new());
        let failed = std::panic::catch_unwind(|| {
            let key = reg::TestKey::new("guard");
            *name.lock().unwrap() = key.0.clone();
            assert!(reg::set(&format!(r"{}\Inner", key.0), "v", "x"));
            panic!("a test that fails");
        });
        assert!(failed.is_err());
        let name = name.into_inner().unwrap();
        assert!(name.starts_with(r"Software\DoboTests-guard-"));
        assert_eq!(reg::get(&format!(r"{name}\Inner"), Some("v")), None);
    }

    /// Under a test key of its own (never the real Run key).
    #[cfg(windows)]
    #[test]
    fn an_old_entry_is_quoted_in_the_registry() {
        // (A key of its own straight under Software, as every registry test makes.)
        let test_key = reg::TestKey::new("install");
        let key = test_key.0.clone();
        let exe = Path::new(r"C:\My Apps\Dobo\Dobo.exe");
        assert!(reg::set(&key, crate::APP_NAME, r"C:\My Apps\Dobo\Dobo.exe --hidden"));
        quote_entry(&key, exe);
        assert_eq!(reg::get(&key, Some(crate::APP_NAME)).as_deref(), Some(r#""C:\My Apps\Dobo\Dobo.exe" --hidden"#));
        quote_entry(&key, exe);
        assert_eq!(reg::get(&key, Some(crate::APP_NAME)).as_deref(), Some(r#""C:\My Apps\Dobo\Dobo.exe" --hidden"#), "once");
    }

    #[test]
    fn updates_only_this_copys_apps_entry() {
        let dir = Path::new(r"C:\Users\me\AppData\Local\Programs\Dobo");
        assert!(needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Dobo""#), Some("1.0.0"), dir, "1.0.1"));
        assert!(needs_new_version(Some(r"c:\users\me\appdata\local\programs\dobo\"), None, dir, "1.0.1"));
        assert!(!needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Dobo""#), Some("1.0.1"), dir, "1.0.1"));
        assert!(!needs_new_version(Some(r#""D:\Apps\Dobo""#), Some("1.0.0"), dir, "1.0.1"));
        assert!(!needs_new_version(None, Some("1.0.0"), dir, "1.0.1"));
    }

    #[test]
    fn setup_follows_dobos_language() {
        assert_eq!(setup_lcid("fr"), "1036");
        assert_eq!(setup_lcid("en"), "1033");
        assert_eq!(setup_lcid(" FR "), "1036");
        // Only the installed copy, only when it differs (or is missing).
        assert_eq!(setup_value_to_write("fr", Some("1033"), true), Some("1036"));
        assert_eq!(setup_value_to_write("en", Some("1036"), true), Some("1033"));
        assert_eq!(setup_value_to_write("fr", None, true), Some("1036"), "an install from before 1.1");
        assert_eq!(setup_value_to_write("en", None, true), Some("1033"));
        assert_eq!(setup_value_to_write("fr", Some("1036"), true), None, "already said");
        assert_eq!(setup_value_to_write("fr", Some(" 1036 "), true), None);
        assert_eq!(setup_value_to_write("fr", Some("1033"), false), None, "never a portable copy");
        assert_eq!(setup_value_to_write("fr", None, false), None);
        // "Follow Windows": the language in use (Windows'), never "system" taken for English.
        assert_eq!(setup_value_to_write(crate::i18n::resolve("system"), None, true), Some(setup_lcid(crate::i18n::system())));
        let dir = Path::new(r"C:\Users\x\AppData\Local\Programs\Dobo");
        assert!(same_folder(r"c:\users\x\appdata\local\programs\dobo\", dir));
        assert!(same_folder(r#""C:\Users\x\AppData\Local\Programs\Dobo""#, dir));
        assert!(same_folder("C:/Users/x/AppData/Local/Programs/Dobo/", dir));
        assert!(!same_folder(r"D:\Portable\Dobo", dir));
    }

    /// The key Dobo writes is the one Dobo Setup reads: Software\<publisher>\<productName> under
    /// HKCU (installer.nsi's MANUPRODUCTKEY and its "Installer Language"), whose default value is
    /// the folder Setup installed Dobo in.
    #[test]
    fn the_language_goes_where_setup_keeps_it() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let publisher = conf["bundle"]["publisher"].as_str().unwrap();
        let product = conf["productName"].as_str().unwrap();
        assert_eq!(SETUP_KEY, format!(r"Software\{publisher}\{product}"));
        assert_eq!(conf["bundle"]["windows"]["nsis"]["installMode"], "currentUser", "the key is under HKCU");
        assert!(include_str!("../installer/app.nsh").contains(&format!(r#"!define APP_PUBLISHER "{publisher}""#)));
        let nsi = include_str!("../installer/installer.nsi");
        assert!(nsi.contains(r#"!define MANUKEY "Software\${MANUFACTURER}""#));
        assert!(nsi.contains(r#"!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}""#));
        assert!(nsi.contains(r#"!define MUI_LANGDLL_REGISTRY_ROOT "HKCU""#));
        assert!(nsi.contains(r#"!define MUI_LANGDLL_REGISTRY_KEY "${MANUPRODUCTKEY}""#));
        assert!(nsi.contains(&format!(r#"!define MUI_LANGDLL_REGISTRY_VALUENAME "{SETUP_VALUE}""#)));
        assert!(nsi.contains(r#"WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR"#));
    }

    /// Setup's window over an installed Dobo opens in its saved language, and Dobo gets --lang
    /// only when a language was picked in the window (template 1.1.1).
    #[test]
    fn setup_sends_lang_only_when_picked() {
        let hooks = include_str!("../installer/hooks.nsh");
        let at = hooks.find("StrCpy $1 '$1 --lang=$AppLang'").unwrap();
        let before = &hooks[..at];
        let cond = before.rfind("${If} ").unwrap();
        assert_eq!(before[cond..].trim(), "${If} $AppLangPicked = 1", "only a pick sends --lang");
        assert!(!hooks.contains("${OrIf} $2 == \"\""), "nothing saved yet isn't a pick (template 1.1.0 sent English)");
        let ui = include_str!("../installer/ui.nsh");
        let init = &ui[ui.find("Function AppGuiInit").unwrap()..];
        let init = &init[..init.find("FunctionEnd").unwrap()];
        assert!(init.contains(r#"${If} $AppMode != "install""#), "over an installed Dobo only");
        assert!(init.contains(r#"ReadRegStr $AppTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}""#));
        assert!(init.contains("StrCpy $AppLang \"fr\""));
        // The pick itself: --lang=en / fr and nothing else; empty or odd: Dobo's own stays.
        let a = |s: &str| -> Vec<String> { vec![INSTALL_CHOICES.into(), "--start-with-windows=1".into(), s.into()] };
        assert_eq!(lang(&a("--lang= fr ")), Some("fr"));
        assert_eq!(lang(&a("--lang=")), None);
        assert_eq!(lang(&a("--lang=FR")), None);
        assert_eq!(lang(&a("--language=fr")), None);
    }

    /// Under a key of its own (never Setup's real one): only the copy in the folder Setup
    /// installed Dobo in writes, and only when the value says something else.
    #[cfg(windows)]
    #[test]
    fn only_the_installed_copy_tells_setup() {
        let key = reg::TestKey::new("setup-lang");
        let k = &key.0;
        let installed = Path::new(r"C:\Users\x\AppData\Local\Programs\Dobo");
        let portable = Path::new(r"D:\Portable\Dobo");
        // Never installed (no key): nothing written, and no key made.
        assert!(!tell_setup_in(k, "fr", installed));
        assert_eq!(reg::get(k, Some(SETUP_VALUE)), None);
        // Setup installed Dobo (the key's default value, as installer.nsi writes it), with English.
        assert!(reg::set(k, "", r"C:\Users\x\AppData\Local\Programs\Dobo"));
        assert!(reg::set(k, SETUP_VALUE, "1033"));
        // A portable copy elsewhere switched to French: Setup's language stays.
        assert!(!tell_setup_in(k, "fr", portable));
        assert_eq!(reg::get(k, Some(SETUP_VALUE)).as_deref(), Some("1033"));
        // The installed copy (the folder written another way): French now.
        assert!(tell_setup_in(k, "fr", Path::new(r"c:\users\x\appdata\local\programs\dobo\")));
        assert_eq!(reg::get(k, Some(SETUP_VALUE)).as_deref(), Some("1036"));
        assert!(!tell_setup_in(k, "fr", installed), "once: the same language isn't written again");
        assert!(tell_setup_in(k, "en", installed));
        assert_eq!(reg::get(k, Some(SETUP_VALUE)).as_deref(), Some("1033"));
        // Installed before 1.1 (no language saved): written at the first start.
        reg::delete_value(k, SETUP_VALUE);
        assert!(tell_setup_in(k, "fr", installed));
        assert_eq!(reg::get(k, Some(SETUP_VALUE)).as_deref(), Some("1036"));
        // The default value still names the folder.
        assert_eq!(reg::get(k, None).as_deref(), Some(r"C:\Users\x\AppData\Local\Programs\Dobo"));
    }

    /// Setup's choices land with the notes: the terms once (an earlier acceptance is kept), Start
    /// with Windows as chosen.
    #[test]
    fn setups_choices_are_saved_with_the_notes() {
        let dir = std::env::temp_dir().join(format!("dobo-install-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        save_choices(&dir, Some(false), Some("2026-11-21"), None).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "terms_accepted"), Some(serde_json::json!("2026-11-21")));
        assert_eq!(crate::db::get_setting(&conn, INSTALLER_START), Some(serde_json::json!(false)));
        assert_eq!(crate::db::get_setting(&conn, "language"), None, "no --lang: still following Windows");
        drop(conn);
        save_choices(&dir, Some(true), Some("2027-01-01"), None).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "terms_accepted"), Some(serde_json::json!("2026-11-21")), "kept");
        assert_eq!(crate::db::get_setting(&conn, INSTALLER_START), Some(serde_json::json!(true)));
        drop(conn);
        assert!(!dir.join(".clean-exit").exists(), "never said to have ended normally: only the app says that");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Over an older version's notes, Setup's choices are saved quickly: no backup, nothing added
    /// to the notes; the app's own start then backs them up as they were. New notes are this
    /// version's (no backup of an empty board). Notes put aside on the recovery screen: nothing
    /// made in their place.
    #[test]
    fn setups_choices_leave_the_backup_to_the_apps_start() {
        let dir = std::env::temp_dir().join(format!("dobo-install-old-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(crate::paths::DB_FILE);
        let cols = |c: &rusqlite::Connection| -> Vec<String> { c.prepare("SELECT name FROM pragma_table_info('notes')").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect() };
        {
            let conn = crate::db::open(&file).unwrap();
            crate::db::create_note(&conn, "From 1.0", "work", "soft", "Butter").unwrap();
            conn.execute_batch("ALTER TABLE notes DROP COLUMN archived_at").unwrap();
            crate::db::set_setting(&conn, crate::backup::OPENED_BY, &serde_json::json!("0.1.0")).unwrap();
        }
        save_choices(&dir, Some(true), Some("2026-11-21"), Some("fr")).unwrap();
        {
            let conn = rusqlite::Connection::open(&file).unwrap();
            assert!(!cols(&conn).contains(&"archived_at".to_string()), "nothing added to the notes");
            assert_eq!(crate::db::get_setting(&conn, crate::backup::OPENED_BY), Some(serde_json::json!("0.1.0")));
            assert_eq!(crate::db::get_setting(&conn, "language"), Some(serde_json::json!("fr")));
        }
        assert!(!dir.join("backups").exists(), "no backup made by Setup");
        let crate::recovery::Look::Open(conn) = crate::recovery::first_look(&dir, crate::db::now_ms()) else { panic!("the app's start") };
        drop(conn);
        let made: Vec<String> = std::fs::read_dir(dir.join("backups")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert!(made.len() == 1 && made[0].ends_with(&format!("-before-update-{}", env!("CARGO_PKG_VERSION"))), "{made:?}");
        let copy = rusqlite::Connection::open(dir.join("backups").join(&made[0]).join(crate::paths::DB_FILE)).unwrap();
        assert!(!cols(&copy).contains(&"archived_at".to_string()), "the notes as they were");
        drop(copy);
        let _ = std::fs::remove_dir_all(&dir);
        // A new install: no backup at the first start.
        let fresh = std::env::temp_dir().join(format!("dobo-install-new-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        save_choices(&fresh, Some(false), None, None).unwrap();
        assert!(matches!(crate::recovery::first_look(&fresh, crate::db::now_ms()), crate::recovery::Look::Open(_)));
        assert!(!fresh.join("backups").exists(), "nothing to back up");
        let _ = std::fs::remove_dir_all(&fresh);
        // Only notes put aside: left to the recovery screen.
        let aside = std::env::temp_dir().join(format!("dobo-install-aside-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&aside).unwrap();
        std::fs::write(aside.join("broken-2026-11-20.sqlite"), b"x").unwrap();
        assert!(save_choices(&aside, Some(true), None, None).is_err());
        assert!(!aside.join(crate::paths::DB_FILE).exists());
        let _ = std::fs::remove_dir_all(&aside);
    }

    /// Setup's language becomes Dobo's: Dobo (and its first-start setup) opens in it. Without
    /// --lang the setting is left alone, also one saved before.
    #[test]
    fn setups_language_is_saved_with_the_notes() {
        let dir = std::env::temp_dir().join(format!("dobo-install-lang-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        let args: Vec<String> = ["--install-choices", "--start-with-windows=1", "--terms-accepted=1", "--lang=fr"].map(String::from).to_vec();
        save_choices(&dir, Some(true), None, lang(&args)).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::i18n::choice(&conn), "fr");
        drop(conn);
        // An update that leaves the language as it was: no --lang, the French stays.
        save_choices(&dir, Some(true), None, None).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::i18n::choice(&conn), "fr");
        drop(conn);
        // English picked in Setup over a saved French.
        save_choices(&dir, None, None, Some("en")).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::i18n::choice(&conn), "en");
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
