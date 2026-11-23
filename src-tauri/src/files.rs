// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! File cards: which files open from Dobo (documents, with their app) and which are only shown
//! in their folder (programs, scripts, shortcuts, installers). A file card's path comes from the
//! page: a file dropped by the person, but also an Undo or a backup restored from another
//! computer, so it is never trusted as it is written.
//!
//! Files on another computer (`\\server\share\…`, a NAS, an office's shares, a redirected
//! Documents or Pictures folder) open once the person has allowed that computer, asked once per
//! computer ("Files on nas open from the network. Allow?"; the setting `file_hosts`): opening
//! one sends the Windows sign-in to that computer, so never for a computer nobody chose. Until
//! then, nothing is touched on it. Device paths (`\\.\`, `\\?\`) and streams are never opened.

use std::path::{Path, PathBuf};

/// What a click on a file card does.
#[derive(Debug, PartialEq)]
pub enum Open {
    /// A document: opened with its app (the file as Windows really names it).
    Document(PathBuf),
    /// Something that would run: shown in its folder instead, never started.
    Reveal(PathBuf),
}

/// Files that would run something when opened (programs, scripts, shortcuts, installers), or
/// connect or mount something (a remote desktop, a theme fetching its pictures from a share, a
/// disk image). Windows' own list (`AssocIsDangerous`) is asked too.
const RISKY: &[&str] = &[
    "exe", "com", "scr", "pif", "bat", "cmd", "ps1", "psm1", "psd1", "ps1xml", "ps2", "ps2xml", "psc1", "psc2", "vb", "vbs", "vbe",
    "js", "jse", "ws", "wsc", "wsf", "wsh", "msh", "msh1", "msh2", "mshxml", "msh1xml", "msh2xml", "hta", "msi", "msp", "mst", "msu",
    "msix", "msixbundle", "appx", "appxbundle", "appinstaller", "appref-ms", "application", "xbap", "jnlp", "cpl", "msc", "reg",
    "inf", "ins", "isp", "scf", "sct", "shb", "shs", "lnk", "url", "website", "rdp", "theme", "themepack", "deskthemepack", "iso",
    "img", "vhd", "vhdx", "jar", "chm", "hlp", "gadget", "library-ms", "search-ms", "searchconnector-ms", "settingcontent-ms",
    "diagcab", "xll", "py", "pyw", "pl", "rb", "sh", "bash", "zsh", "command", "tool", "app", "pkg", "desktop", "appimage", "run",
];

/// The extension Windows goes by: the name's trailing dots and spaces dropped first (Windows
/// drops them too, so "setup.exe." and "setup.exe " are setup.exe), in lower case.
fn extension(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let name = name.trim_end_matches(['.', ' ']);
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Would opening this file run something?
pub fn runs_something(path: &Path) -> bool {
    let ext = extension(path);
    RISKY.contains(&ext.as_str()) || (!ext.is_empty() && dangerous_to_windows(&ext)) || executable_file(path)
}

/// Windows' own list of file types that are dangerous to open (kept up to date by Windows).
#[cfg(windows)]
fn dangerous_to_windows(ext: &str) -> bool {
    let wide: Vec<u16> = format!(".{ext}").encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a NUL-terminated UTF-16 string that lives through the call.
    unsafe { windows::Win32::UI::Shell::AssocIsDangerous(windows::core::PCWSTR(wide.as_ptr())).as_bool() }
}

#[cfg(not(windows))]
fn dangerous_to_windows(_ext: &str) -> bool {
    false
}

/// Mac and Linux: a file marked as a program (any name).
#[cfg(unix)]
fn executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(not(unix))]
fn executable_file(_path: &Path) -> bool {
    false
}

/// A file on this computer, named plainly: `C:\…` (never `\\server\share\…`, which would send
/// the Windows sign-in to that server, nor a device path `\\.\…` / `\\?\…`, nor a stream
/// `file.exe::$DATA`).
pub fn plain_local(path: &str) -> bool {
    #[cfg(windows)]
    {
        let b = path.as_bytes();
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && !path[2..].contains(':')
    }
    #[cfg(not(windows))]
    {
        path.starts_with('/') && !path.starts_with("//")
    }
}

/// Inside Dobo's own data folder (its pictures), wherever that is (a redirected AppData may be
/// on a server: those files are still Dobo's own).
fn in_data_folder(path: &str, data: Option<&Path>) -> bool {
    let Some(data) = data else { return false };
    let norm = |s: &str| s.replace('/', "\\").to_lowercase();
    let dir = norm(&data.to_string_lossy());
    let dir = dir.trim_end_matches('\\');
    let p = norm(path);
    !dir.is_empty() && p.strip_prefix(dir).is_some_and(|rest| rest.starts_with('\\')) && !p.contains("\\..")
}

/// The computer a `\\server\share\…` path is on (in lower case), when it names a file in a share
/// plainly: never a device path (`\\.\`, `\\?\`), nor a stream (`file.pdf:x`), nor `..`.
pub fn share_host(path: &str) -> Option<String> {
    let rest = path.strip_prefix(r"\\").or_else(|| path.strip_prefix("//"))?;
    if rest.starts_with(['?', '.']) || rest.contains(':') {
        return None;
    }
    let mut parts = rest.split(['\\', '/']);
    let host = parts.next().filter(|h| !h.is_empty())?;
    let share = parts.next().filter(|s| !s.is_empty())?;
    let rest: Vec<&str> = parts.collect();
    if share == ".." || rest.contains(&"..") || rest.last().is_none_or(|p| p.is_empty()) {
        return None;
    }
    Some(host.to_lowercase())
}

/// Why a path isn't opened.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// Not a file on this computer, nor on a computer that may be asked about.
    NotHere,
    /// On another computer (this one, in lower case) not allowed yet: the card asks once.
    AskHost(String),
}

impl Refused {
    /// What the page is told: a card asks about `ask-host:<computer>`; anything else is said.
    pub fn said(self) -> String {
        match self {
            Refused::AskHost(host) => format!("{ASK_HOST}{host}"),
            Refused::NotHere => tr!("Dobo only opens files on this computer, or on a computer you allowed.", "Dobo n’ouvre que les fichiers de cet ordinateur, ou d’un ordinateur que vous avez autorisé."),
        }
    }
}

/// How the page is told that a file is on a computer to ask about (FileCard.svelte).
pub const ASK_HOST: &str = "ask-host:";

/// May this path be opened or shown at all (before anything touches it)? On this computer, in
/// Dobo's own data folder or `own` (a folder Dobo writes to: Pictures › Dobo), or on a computer
/// allowed (`hosts`, in lower case).
pub fn allowed(path: &str, data: Option<&Path>, own: Option<&Path>, hosts: &[String]) -> Result<(), Refused> {
    if plain_local(path) || in_data_folder(path, data) || in_data_folder(path, own) {
        return Ok(());
    }
    match share_host(path) {
        Some(h) if hosts.iter().any(|a| a.eq_ignore_ascii_case(&h)) => Ok(()),
        Some(h) => Err(Refused::AskHost(h)),
        None => Err(Refused::NotHere),
    }
}

/// The computers file cards may open files from (the setting `file_hosts`, in lower case).
pub fn hosts_allowed(conn: &rusqlite::Connection) -> Vec<String> {
    crate::db::get_setting(conn, "file_hosts")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|h| h.as_str().map(|s| s.trim().to_lowercase()))
        .filter(|h| !h.is_empty())
        .collect()
}

/// The file as Windows really names it: trailing dots and spaces dropped, short names made long,
/// links followed; without the `\\?\` prefix.
fn real_path(path: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(path).ok()?;
    let s = c.to_str()?;
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        // (A drive letter that names a server's folder: the person's own mapped drive.)
        return Some(PathBuf::from(format!(r"\\{rest}")));
    }
    Some(PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(s)))
}

/// What a click on a file card at `path` does.
pub fn decide(path: &str, data: Option<&Path>, hosts: &[String]) -> Result<Open, String> {
    allowed(path, data, None, hosts).map_err(Refused::said)?;
    let real = real_path(Path::new(path)).ok_or_else(|| tr!("Couldn’t find that file: moved or deleted?", "Fichier introuvable : déplacé ou supprimé ?"))?;
    Ok(if runs_something(&real) || runs_something(Path::new(path)) { Open::Reveal(real) } else { Open::Document(real) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_read_from_the_settings() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        assert!(hosts_allowed(&conn).is_empty());
        crate::db::set_setting(&conn, "file_hosts", &serde_json::json!(["NAS", " files.office ", 3, ""])).unwrap();
        assert_eq!(hosts_allowed(&conn), ["nas", "files.office"]);
        assert_eq!(share_host(r"\\Files.Office\Team\plan.docx").as_deref(), Some("files.office"));
        assert_eq!(share_host(r"\\host@SSL\DavWWWRoot\a.pdf").as_deref(), Some("host@ssl"));
        assert_eq!(share_host(r"C:\a.pdf"), None);
    }

    #[test]
    fn programs_are_never_started_from_a_file_card() {
        for p in [
            "C:/x/setup.EXE", "C:/x/run.bat", "C:/x/a.lnk", "C:/x/s.ps1", "C:/x/i.msi", "C:/x/w.url", "/Applications/A.app", "/x/y.sh",
            "C:/x/setup.exe.", "C:/x/setup.exe ", "C:/x/setup.exe. . ", "C:/x/a.rdp", "C:/x/b.theme", "C:/x/d.iso", "C:/x/e.vhdx",
            "C:/x/f.wsc", "C:/x/g.search-ms",
        ] {
            assert!(runs_something(Path::new(p)), "{p}");
        }
        for p in ["C:/x/report.pdf", "C:/x/notes.txt", "C:/x/photo.JPG", "C:/x/folder", "C:/x/sheet.xlsx"] {
            assert!(!runs_something(Path::new(p)), "{p}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_own_list_is_asked() {
        assert!(dangerous_to_windows("exe"));
        assert!(!dangerous_to_windows("txt"));
    }

    #[cfg(windows)]
    #[test]
    fn shares_devices_and_streams_are_refused_before_anything_is_touched() {
        for p in [r"\\attacker.example\share\report.pdf", r"\\host@SSL\dav\a.pdf", "//host/share/a.pdf", r"\\.\C:\a.pdf", r"\\?\C:\a.pdf", r"\\?\UNC\h\s\a.pdf", r"C:\x\a.exe::$DATA", "x/a.pdf", r"\x\a.pdf"] {
            assert!(!plain_local(p), "{p}");
            assert!(decide(p, None, &[]).is_err(), "{p}");
        }
        // A share: asked about, by its computer, before anything is touched on it.
        assert_eq!(decide(r"\\attacker.example\share\report.pdf", None, &[]), Err(format!("{ASK_HOST}attacker.example")));
        assert_eq!(decide("//NAS/projects/brief.pdf", None, &[]), Err(format!("{ASK_HOST}nas")));
        // Never a device path, a stream or a way out of the share, even on a computer allowed.
        let nas = ["nas".to_string()];
        for p in [r"\\?\UNC\nas\s\a.pdf", r"\\.\nas\a.pdf", r"\\nas\s\a.pdf:x", r"\\nas\s\..\c$\a.pdf", r"\\nas\s", r"\\nas\s\", r"\\nas", r"\\\s\a.pdf"] {
            assert_eq!(allowed(p, None, None, &nas), Err(Refused::NotHere), "{p}");
        }
        assert!(plain_local(r"C:\Users\Jo\report.pdf") && plain_local("d:/a b/c.txt"));
        // Dobo's own pictures in a data folder on a server (redirected AppData) still open.
        let data = Path::new(r"\\server\home\Jo\AppData\Roaming\com.sookera.ztodo");
        assert!(allowed(r"\\server\home\Jo\AppData\Roaming\com.sookera.ztodo\pictures\1-0.png", Some(data), None, &[]).is_ok());
        assert!(allowed(r"\\SERVER\home\jo\appdata\roaming\com.sookera.ztodo/pictures/1-0.png", Some(data), None, &[]).is_ok());
        assert!(allowed(r"\\server\home\Jo\AppData\Roaming\com.sookera.ztodo-other\a.png", Some(data), None, &[]).is_err());
        assert!(allowed(r"\\server\home\Jo\AppData\Roaming\com.sookera.ztodo\..\..\x.pdf", Some(data), None, &[]).is_err());
        // "Save as a picture" in a Pictures folder on a server (folder redirection): shown.
        let pictures = Path::new(r"\\server\home\Jo\Pictures\Dobo");
        assert!(allowed(r"\\server\home\Jo\Pictures\Dobo\Trip.png", None, Some(pictures), &[]).is_ok());
        assert!(allowed(r"\\server\home\Jo\Pictures\Other\x.png", None, Some(pictures), &[]).is_err());
        // A computer allowed: its files open (any letter case).
        let allowed_nas = ["nas".to_string()];
        assert!(allowed(r"\\NAS\projects\brief.pdf", None, None, &allowed_nas).is_ok());
        assert_eq!(allowed(r"\\other\projects\brief.pdf", None, None, &allowed_nas), Err(Refused::AskHost("other".into())));
    }

    #[cfg(windows)]
    #[test]
    fn the_file_is_judged_as_windows_names_it() {
        let dir = std::env::temp_dir().join(format!("dobo-files-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("setup.exe"), b"MZ").unwrap();
        std::fs::write(dir.join("report.pdf"), b"%PDF").unwrap();
        let at = |name: &str| format!("{}\\{name}", dir.display());
        for name in ["setup.exe", "setup.exe.", "setup.exe ", "SETUP.EXE"] {
            assert!(matches!(decide(&at(name), None, &[]), Ok(Open::Reveal(p)) if p.file_name().unwrap() == "setup.exe"), "{name}");
        }
        let Ok(Open::Document(p)) = decide(&at("report.pdf"), None, &[]) else { panic!("a document opens") };
        assert!(!p.to_string_lossy().starts_with(r"\\?\"), "no device prefix: {}", p.display());
        assert!(decide(&at("missing.pdf"), None, &[]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
