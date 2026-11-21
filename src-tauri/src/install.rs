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
//!   data (%LOCALAPPDATA%\com.sookera.ztodo), unless another Dobo is open. What changes from one
//!   version to the next is done here, not in the uninstaller: a Dobo that updated itself keeps
//!   the uninstaller of the version first installed, but this exe is always the current one.
//! - `Dobo.exe --install-choices --start-with-windows=0|1 [--terms-accepted=1]`: run by Dobo
//!   Setup's window once Dobo is installed (never by a silent or passive install). Saves Setup's
//!   choices with the notes and does them now. No window. `--terms-accepted=1`: Install was
//!   clicked under "By installing, you accept the terms of use", so the first start doesn't ask.
//! - At each start of an installed copy: the Windows Apps entry shows the version really
//!   installed (Dobo's own updates replace Dobo.exe without the installer).

use std::path::{Path, PathBuf};

/// The argument the uninstaller starts Dobo.exe with.
pub const UNINSTALL_CLEANUP: &str = "--uninstall-cleanup";
/// With it: also delete the notes ("Also delete my notes").
pub const DELETE_DATA: &str = "--delete-data";
/// Exit code: another Dobo is open (a portable copy), so the notes (which may be the same
/// folder) were kept. The uninstaller then keeps the rest too and says why.
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

/// `args`: the arguments after the exe (the first is `INSTALL_CHOICES`). 0 = all done; 1 = some
/// of it couldn't be done (Setup says so; Settings can still change it).
pub fn install_choices(args: &[String]) -> i32 {
    let start = choice(args, "--start-with-windows");
    let terms = (choice(args, "--terms-accepted") == Some(true)).then(crate::terms::version).flatten();
    // The installed copy's notes: Setup wrote the marker next to this exe, so it's the usual folder.
    let Some(place) = crate::paths::init_early() else { return 1 };
    if save_choices(&place.dir, start, terms.as_deref()).is_err() {
        return 1;
    }
    match start {
        Some(on) if !set_autostart(on) => 1,
        _ => 0,
    }
}

/// Saves Setup's choices with the notes (the database is made when there's none yet: a new
/// install; its first start then finds them).
fn save_choices(dir: &Path, start: Option<bool>, terms: Option<&str>) -> Result<(), String> {
    use serde_json::json;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let conn = crate::db::open(&dir.join(crate::paths::DB_FILE))?;
    // An acceptance already saved is kept as it is.
    if let Some(date) = terms
        && crate::consent::needed(&conn)
    {
        crate::db::set_setting(&conn, "terms_accepted", &json!(date))?;
    }
    if let Some(v) = start {
        crate::db::set_setting(&conn, INSTALLER_START, &json!(v))?;
    }
    Ok(())
}

/// The same "start with Windows" entry the app's own switch makes (tauri-plugin-autostart: the
/// value "Dobo" in HKCU's Run key, "<exe> --hidden"), for this exe. Off: only this copy's entry
/// (or one whose program is gone) goes; a portable Dobo elsewhere keeps starting with Windows.
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
    let mut b = auto_launch::AutoLaunchBuilder::new();
    // (The current user's entry only, as a per-user install: never the whole PC's.)
    b.set_app_name(crate::APP_NAME)
        .set_app_path(&exe.display().to_string())
        .set_args(&["--hidden"])
        .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    b.build().and_then(|a| a.enable()).is_ok()
}

#[cfg(not(windows))]
fn set_autostart(_on: bool) -> bool {
    true
}

/// Windows' list of programs started at sign-in (current user).
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// The program a Run entry starts: `"C:\…\Dobo.exe" --hidden` or, as tauri-plugin-autostart
/// writes it, `C:\…\Dobo.exe --hidden` (unquoted).
fn entry_program(command: &str) -> Option<PathBuf> {
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
    if other_copy_open() {
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
        reg::delete_value(r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run", crate::APP_NAME);
    }
    let proto = format!(r"Software\Classes\{}", crate::notify::SCHEME);
    if reg::get(&format!(r"{proto}\shell\open\command"), None).is_some_and(|c| starts(&c, exe)) {
        reg::delete_tree(&proto);
        reg::delete_tree(&format!(r"Software\Classes\AppUserModelId\{}", crate::notify::AUMID));
    }
}

/// Same folder, whatever the case, quotes or a trailing backslash.
fn same_folder(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').trim_end_matches(['\\', '/']).to_lowercase();
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

/// The few registry calls this needs (current user only).
#[cfg(windows)]
mod reg {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegDeleteKeyW, RegDeleteTreeW, RegGetValueW, RegSetKeyValueW};

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
        assert!(hooks.contains("--uninstall-cleanup$AppUnArgs"));
        assert!(hooks.contains(r#"StrCpy $AppUnArgs " --delete-data""#));
        // The uninstaller's switch, word for word as promised.
        assert!(include_str!("../installer/English.nsh").contains(r#"LangString deleteAppData ${LANG_ENGLISH} "Also delete my notes""#));
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
    fn updates_only_this_copys_apps_entry() {
        let dir = Path::new(r"C:\Users\me\AppData\Local\Programs\Dobo");
        assert!(needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Dobo""#), Some("1.0.0"), dir, "1.0.1"));
        assert!(needs_new_version(Some(r"c:\users\me\appdata\local\programs\dobo\"), None, dir, "1.0.1"));
        assert!(!needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Dobo""#), Some("1.0.1"), dir, "1.0.1"));
        assert!(!needs_new_version(Some(r#""D:\Apps\Dobo""#), Some("1.0.0"), dir, "1.0.1"));
        assert!(!needs_new_version(None, Some("1.0.0"), dir, "1.0.1"));
    }

    /// Setup's choices land with the notes: the terms once (an earlier acceptance is kept), Start
    /// with Windows as chosen.
    #[test]
    fn setups_choices_are_saved_with_the_notes() {
        let dir = std::env::temp_dir().join(format!("dobo-install-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        save_choices(&dir, Some(false), Some("2026-11-21")).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "terms_accepted"), Some(serde_json::json!("2026-11-21")));
        assert_eq!(crate::db::get_setting(&conn, INSTALLER_START), Some(serde_json::json!(false)));
        drop(conn);
        save_choices(&dir, Some(true), Some("2027-01-01")).unwrap();
        let conn = crate::db::open(&dir.join(crate::paths::DB_FILE)).unwrap();
        assert_eq!(crate::db::get_setting(&conn, "terms_accepted"), Some(serde_json::json!("2026-11-21")), "kept");
        assert_eq!(crate::db::get_setting(&conn, INSTALLER_START), Some(serde_json::json!(true)));
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
