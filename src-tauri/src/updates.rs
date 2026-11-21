// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Keeping Dobo up to date (ported from Pako's updates.rs, Sookera's reference).
//!
//! New versions are published as GitHub releases in the public, releases-only repo
//! sookera/dobo-releases. Each has `latest.json` (version, SHA-256, size, notes, signature) and
//! `Dobo.exe`. The signature is made with a private key that never leaves the release PC; only
//! its public half is in the app, so a release that isn't signed with it is refused, wherever it
//! came from. Before anything is kept, the signature, the SHA-256 and the size are checked.
//!
//! The installed and the portable Dobo update the same way (Setup holds the very same Dobo.exe):
//! the new exe is downloaded next to the running one as `Dobo.new.exe` and checked. On the next
//! start (or "Restart now"), before anything else runs, the running exe is renamed to
//! `Dobo.old.exe` (Windows allows renaming a running exe), the new one takes its name, and it
//! starts "on trial". The old copy is kept until the new version's window has loaded and it has
//! run for about 20 seconds (or quit normally after its window loaded); if it stopped before
//! that, the next start puts the old version back. **The notes are never touched**: only files
//! next to Dobo.exe change.
//!
//! The update's own notes (which version is on trial, which was put back, the old copy's
//! SHA-256) live in the data folder, sealed for this Windows user (DPAPI): never next to
//! Dobo.exe, where a browser can save files. An old copy is only ever put back and run when its
//! SHA-256 is the one recorded there.
//!
//! The automatic check (Settings › Updates, on by default) looks once a day; a check that failed
//! is tried again the next hour. It goes online only once the terms are accepted (consent.rs).

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where Dobo's releases are published.
pub const RELEASES: &str = "https://github.com/sookera/dobo-releases/releases";

/// Public half of Dobo's release signing key (base64). Dobo's own key, never Pako's. The private
/// half stays on the release PC (%USERPROFILE%\.dobo-signing, see release.rs).
const PUBLIC_KEY: &str = "/Z0Oms17pPO6dSZFLQ+QTVrwePPkKdCjtzRkrqnxWJk=";

/// The single-instance plugin's lock: `<identifier>-sim` (checked by a test).
const INSTANCE_LOCK: &str = "com.sookera.ztodo-sim";

/// How long a new version must run before the old copy is deleted.
pub const TRIAL: Duration = Duration::from_secs(20);

/// The setting for Settings › Updates › "Check for updates automatically" (on unless turned off).
pub const AUTO_SETTING: &str = "update_auto";

/// What `latest.json` holds.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Manifest {
    pub version: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub notes: String,
    /// Base64 Ed25519 signature of `signed_message(…)`.
    pub signature: String,
}

/// The text a release signs: the version (so an old release can't pass for a new one), the
/// SHA-256, the size, and the notes (as their SHA-256, so nothing in them can change where a
/// field ends).
pub fn signed_message(version: &str, sha256: &str, size: u64, notes: &str) -> String {
    format!("dobo-update:{version}:{sha256}:{size}:{:x}", Sha256::digest(notes.as_bytes()))
}

fn public_key() -> Result<VerifyingKey, String> {
    let bytes: [u8; 32] = B64.decode(PUBLIC_KEY).map_err(|e| e.to_string())?.try_into().map_err(|_| "bad public key".to_string())?;
    VerifyingKey::from_bytes(&bytes).map_err(|e| e.to_string())
}

/// Checks a manifest's signature against Dobo's public key.
pub fn verify(m: &Manifest) -> Result<(), String> {
    verify_with(&public_key()?, m)
}

fn verify_with(key: &VerifyingKey, m: &Manifest) -> Result<(), String> {
    let refused = || tr!("This update isn't signed by Dobo's publisher, so it wasn't installed.", "Cette mise à jour n’est pas signée par l’éditeur de Dobo : elle n’a pas été installée.");
    // A plain version and a real checksum (both end up in file names and links).
    if parse_version(&m.version).is_none() || m.sha256.len() != 64 || !m.sha256.chars().all(|c| c.is_ascii_hexdigit()) || m.size == 0 {
        return Err(refused());
    }
    let sig = Signature::from_slice(&B64.decode(&m.signature).map_err(|_| refused())?).map_err(|_| refused())?;
    key.verify(signed_message(&m.version, &m.sha256, m.size, &m.notes).as_bytes(), &sig).map_err(|_| refused())
}

fn parse_version(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v')).ok()
}

/// True if version `a` is newer than `b` ("1.0.10" > "1.0.9", "1.0.0" > "1.0.0-beta.2").
pub fn newer(a: &str, b: &str) -> bool {
    match (parse_version(a), parse_version(b)) {
        (Some(x), Some(y)) => x.cmp_precedence(&y).is_gt(),
        _ => false,
    }
}

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// This build may update itself: a release of Dobo itself. A development build (cargo's exe,
/// with the project's own `data` folder) and a renamed test copy (DoboTest, with notes of its
/// own) never do: a release put in their place would be the real Dobo, opening the real notes.
/// Nothing is checked or downloaded for them, and nothing is put in place at their start.
pub fn enabled() -> bool {
    !cfg!(debug_assertions) && option_env!("DOBO_TEST_NAME").is_none() && option_env!("DOBO_TEST_IDENTIFIER").is_none()
}

/// Dobo's releases come from GitHub over HTTPS only: the release page itself, and the download
/// host it sends the files to (`*.githubusercontent.com`). Anything else (a redirect elsewhere)
/// is refused before a byte is kept. (The signature already guards what is kept; this keeps
/// Dobo from talking to anyone else.)
fn from_github(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // No "user@" part, no port other than HTTPS's own.
    if authority.contains('@') {
        return false;
    }
    let host = match authority.rsplit_once(':') {
        Some((h, "443")) => h,
        Some(_) => return false,
        None => authority,
    }
    .to_ascii_lowercase();
    host == "github.com" || (host.ends_with(".githubusercontent.com") && host.len() > ".githubusercontent.com".len())
}

// ---------- files next to the exe ----------

fn exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn sibling(suffix: &str) -> Option<PathBuf> {
    let exe = exe()?;
    let stem = exe.file_stem()?.to_string_lossy().to_string();
    Some(exe.with_file_name(format!("{stem}.{suffix}")))
}

fn new_exe() -> Option<PathBuf> {
    sibling("new.exe")
}

fn old_exe() -> Option<PathBuf> {
    sibling("old.exe")
}

/// A new version that didn't start properly (kept aside until the next update).
fn bad_exe() -> Option<PathBuf> {
    sibling("bad.exe")
}

/// The manifest of the downloaded update (checked again before it is put in place).
fn marker() -> Option<PathBuf> {
    sibling("update.json")
}

// ---------- the update's own notes, in the data folder ----------

fn data_file(name: &str) -> Option<PathBuf> {
    crate::paths::data_dir().ok().map(|d| d.join(name))
}

/// A new version on trial: `{ version, started }`.
fn trial_file() -> Option<PathBuf> {
    data_file("update-trial.bin")
}

/// The version that was put back because it didn't start properly.
fn rolled_back_file() -> Option<PathBuf> {
    data_file("update-rolled-back.bin")
}

/// What the copy kept as `Dobo.old.exe` must be: its SHA-256 and version, written right before
/// an update puts the new version in place.
fn old_record_file() -> Option<PathBuf> {
    data_file("update-old.bin")
}

/// When the last automatic check worked (seconds).
fn stamp_file() -> Option<PathBuf> {
    data_file("update-check.txt")
}

/// Writes a note sealed for this Windows user.
fn write_sealed(path: &Path, text: &str) {
    if let Some(blob) = seal::protect(text.as_bytes()) {
        let _ = std::fs::write(path, blob);
    }
}

/// Reads a sealed note (anything else, or one sealed for another user, reads as nothing).
fn read_sealed(path: &Path) -> Option<String> {
    let blob = std::fs::read(path).ok()?;
    String::from_utf8(seal::unprotect(&blob)?).ok()
}

/// The old copy's record: `{ sha256, version }`.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
struct OldCopy {
    sha256: String,
    version: String,
}

fn read_old_record() -> Option<OldCopy> {
    serde_json::from_str(&read_sealed(&old_record_file()?)?).ok()
}

fn write_old_record(r: &OldCopy) {
    if let Some(f) = old_record_file() {
        write_sealed(&f, &serde_json::to_string(r).unwrap_or_default());
    }
}

fn forget(file: Option<PathBuf>) {
    if let Some(f) = file {
        let _ = std::fs::remove_file(f);
    }
}

/// The old copy next to Dobo.exe is the one Dobo kept at its last update: its SHA-256 matches
/// the sealed record. Only then may it ever be put back and started.
fn old_copy_trusted(old: &Path) -> bool {
    old_trusted_by(read_old_record().as_ref(), || sha256_file(old).ok())
}

fn old_trusted_by(record: Option<&OldCopy>, hash: impl FnOnce() -> Option<String>) -> bool {
    match record {
        Some(r) if !r.sha256.is_empty() => hash().is_some_and(|h| h.eq_ignore_ascii_case(&r.sha256)),
        _ => false,
    }
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(format!("{:x}", h.finalize()))
}

/// The update downloaded and checked, waiting for a restart (its version).
fn ready_version() -> Option<String> {
    let m: Manifest = serde_json::from_str(&std::fs::read_to_string(marker()?).ok()?).ok()?;
    new_exe()?.is_file().then_some(m.version).filter(|v| newer(v, current()))
}

fn rolled_back_version() -> Option<String> {
    put_back_note(&read_sealed(&rolled_back_file()?)?, current())
}

fn write_rolled_back(version: &str) {
    if let Some(f) = rolled_back_file() {
        write_sealed(&f, version);
    }
}

/// The version a "put back" note is about, while it still says something true: a version newer
/// than this Dobo. Once this Dobo is that version or a later one, the note is out of date.
fn put_back_note(noted: &str, current: &str) -> Option<String> {
    let v = noted.trim();
    (!v.is_empty() && newer(v, current)).then(|| v.to_string())
}

/// At start: a "put back" note that is out of date goes.
fn forget_old_put_back_note() {
    let Some(f) = rolled_back_file() else { return };
    if f.is_file() && rolled_back_version().is_none() {
        let _ = std::fs::remove_file(f);
    }
}

/// Dobo's folder accepts new files (a copy in Program Files or on a read-only drive doesn't).
fn folder_writable() -> Result<(), Fail> {
    let dir = exe().and_then(|e| e.parent().map(Path::to_path_buf)).ok_or_else(|| Fail::other(tr!("Can't find where Dobo is.", "Impossible de trouver où est Dobo.")))?;
    let test = dir.join(format!(".dobo-write-test-{}-{}", std::process::id(), crate::db::now_ms()));
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&test) {
        Ok(f) => {
            drop(f);
            let _ = std::fs::remove_file(&test);
            Ok(())
        }
        Err(_) => Err(Fail::other(tr!(
            "Dobo can't save the update in its folder ({}): Windows doesn't allow changes there. Move the Dobo folder somewhere you can change files, such as Documents, then try again.",
            "Dobo ne peut pas enregistrer la mise à jour dans son dossier ({}) : Windows n’y autorise pas les modifications. Déplacez le dossier de Dobo là où vous pouvez modifier des fichiers, comme Documents, puis réessayez.",
            dir.display()
        ))),
    }
}

// ---------- state shown in Settings ----------

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct View {
    pub current: String,
    /// A newer version that exists.
    pub latest: Option<String>,
    pub notes: Option<String>,
    /// Downloaded and checked: restart to use it.
    pub ready: bool,
    pub busy: bool,
    pub error: Option<String>,
    /// A check has been made this session.
    pub checked: bool,
    /// When the last check that worked was made (ms), this session or before.
    pub last_checked: Option<i64>,
}

static STATE: Mutex<Option<View>> = Mutex::new(None);

fn first_view() -> View {
    let error = rolled_back_version().map(|v| tr!("Dobo {} didn't start properly, so Dobo {} was put back.", "Dobo {} n’a pas démarré correctement : Dobo {} a été remis.", v, current()));
    View { current: current().into(), error, last_checked: last_check().map(|s| s as i64 * 1000).filter(|&s| s > 0), ..Default::default() }
}

pub fn view() -> View {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let v = s.get_or_insert_with(first_view);
    if let Some(r) = ready_version() {
        v.ready = true;
        v.latest.get_or_insert(r);
    }
    v.clone()
}

fn set(f: impl FnOnce(&mut View)) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let v = s.get_or_insert_with(first_view);
    f(v);
}

/// A failed check: its words, and whether it may pass by itself (the update page out of reach:
/// no network yet after the PC woke up, a timeout, the site down for a moment).
#[derive(Debug)]
struct Fail {
    msg: String,
    passing: bool,
}

impl Fail {
    fn other(msg: String) -> Self {
        Fail { msg, passing: false }
    }
    fn unreachable() -> Self {
        Fail { msg: tr!("Couldn't reach Dobo's update page. Check your connection, then try again.", "Impossible de joindre la page des mises à jour de Dobo. Vérifiez votre connexion, puis réessayez."), passing: true }
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .redirects(5)
        // Never plain HTTP, not even after a redirect.
        .https_only(true)
        .user_agent(&format!("Dobo/{}", current()))
        .build()
}

/// Said when GitHub's answer came from somewhere else (`from_github`).
fn not_from_github() -> Fail {
    Fail::other(tr!(
        "Dobo's update page sent it somewhere other than GitHub, so nothing was downloaded.",
        "La page des mises à jour de Dobo l’a envoyé ailleurs que sur GitHub : rien n’a été téléchargé."
    ))
}

fn fetch_manifest(http: &ureq::Agent) -> Result<Manifest, Fail> {
    let r = match http.get(&format!("{RELEASES}/latest/download/latest.json")).timeout(Duration::from_secs(30)).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return Err(Fail::other(tr!("No Dobo release is published yet.", "Aucune version de Dobo n’est encore publiée."))),
        Err(ureq::Error::Status(code, _)) if code >= 500 => return Err(Fail::unreachable()),
        Err(ureq::Error::Status(code, _)) => return Err(Fail::other(tr!("Dobo's update page answered {}.", "La page des mises à jour de Dobo a répondu {}.", code))),
        Err(ureq::Error::Transport(_)) => return Err(Fail::unreachable()),
    };
    if !from_github(r.get_url()) {
        return Err(not_from_github());
    }
    let mut text = String::new();
    r.into_reader().take(64 * 1024).read_to_string(&mut text).map_err(|_| Fail::unreachable())?;
    let m: Manifest = serde_json::from_str(&text).map_err(|_| Fail::other(tr!("Dobo's update page sent something unexpected.", "La page des mises à jour de Dobo a envoyé quelque chose d’inattendu.")))?;
    verify(&m).map_err(Fail::other)?;
    Ok(m)
}

/// Deletes a half-downloaded file unless it was kept.
struct PartGuard(PathBuf);

impl Drop for PartGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Leftovers of earlier downloads that stopped half-way (`Dobo.new.<n>.part`).
fn remove_old_parts() {
    let (Some(exe), Some(new)) = (exe(), new_exe()) else { return };
    let Some(dir) = exe.parent() else { return };
    let prefix = new.with_extension("").file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if prefix.is_empty() {
        return;
    }
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) && name.ends_with(".part") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

fn download(http: &ureq::Agent, m: &Manifest) -> Result<(), Fail> {
    let (Some(dest), Some(mark)) = (new_exe(), marker()) else { return Err(Fail::other(tr!("Can't find where Dobo is.", "Impossible de trouver où est Dobo."))) };
    folder_writable()?;
    remove_old_parts();
    let cut = || Fail::other(tr!("The update download was cut. Try again.", "Le téléchargement de la mise à jour a été interrompu. Réessayez."));
    // A name of its own: nothing else writes into this file.
    let part = dest.with_extension(format!("{}-{}.part", std::process::id(), crate::db::now_ms()));
    let guard = PartGuard(part.clone());
    let r = match http.get(&format!("{RELEASES}/download/v{}/Dobo.exe", m.version)).call() {
        Ok(r) => r,
        Err(ureq::Error::Transport(_)) => return Err(Fail::unreachable()),
        Err(ureq::Error::Status(code, _)) => return Err(Fail::other(tr!("The update download answered {}.", "Le téléchargement de la mise à jour a répondu {}.", code))),
    };
    if !from_github(r.get_url()) {
        return Err(not_from_github());
    }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&part).map_err(|e| Fail::other(e.to_string()))?;
    let mut body = r.into_reader().take(m.size + 1);
    let mut buf = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let n = body.read(&mut buf).map_err(|_| cut())?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > m.size {
            return Err(Fail::other(tr!("The update download is bigger than its signed size, so it was stopped.", "Le téléchargement de la mise à jour dépasse sa taille signée : il a été arrêté.")));
        }
        file.write_all(&buf[..n]).map_err(|e| Fail::other(e.to_string()))?;
    }
    file.sync_all().map_err(|e| Fail::other(e.to_string()))?;
    drop(file);
    // The checksum of what is really on disk, not of what was received.
    if size != m.size || sha256_file(&part).ok().as_deref() != Some(m.sha256.as_str()) {
        return Err(Fail::other(tr!("The downloaded update doesn't match its signed checksum, so it wasn't kept.", "La mise à jour téléchargée ne correspond pas à sa somme de contrôle signée : elle n’a pas été gardée.")));
    }
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&part, &dest).map_err(|e| Fail::other(e.to_string()))?;
    std::mem::forget(guard);
    std::fs::write(mark, serde_json::to_string(m).unwrap_or_default()).map_err(|e| Fail::other(e.to_string()))?;
    Ok(())
}

/// One check at a time (two would write the same files).
static CHECKING: Mutex<()> = Mutex::new(());

/// "Check for updates": looks for a newer Dobo and, when there is one, downloads and checks it
/// (then it waits for a restart). Blocks: call it off the main thread.
pub fn check() -> View {
    run_check(false).0
}

/// The automatic check (once a day): the view, and whether it worked. Only a check that worked
/// counts for the day; anything else is tried again the next hour.
pub fn auto_check() -> (View, bool) {
    run_check(true)
}

/// What an automatic check's result does: (the day counts as checked, its error shows in
/// Settings). Only a check that worked counts; a passing failure (the update page out of reach,
/// as when the PC has just woken up) also stays out of Settings, which keeps what it showed.
fn after_auto(result: &Result<(), Fail>) -> (bool, bool) {
    match result {
        Ok(()) => (true, false),
        Err(e) => (false, !e.passing),
    }
}

/// `auto` = the daily check: a version that was put back once isn't downloaded again by itself,
/// and a passing failure isn't shown.
fn run_check(auto: bool) -> (View, bool) {
    let _one = CHECKING.lock().unwrap_or_else(|e| e.into_inner());
    let before = view();
    set(|v| {
        v.busy = true;
        v.error = None;
    });
    let http = agent();
    let result: Result<(), Fail> = (|| {
        if !enabled() {
            return Err(Fail::other(tr!(
                "This copy of Dobo is a development or test build: it never updates itself.",
                "Cette copie de Dobo est une version de développement ou de test : elle ne se met jamais à jour."
            )));
        }
        let m = fetch_manifest(&http)?;
        if !newer(&m.version, current()) {
            set(|v| {
                v.latest = None;
                v.notes = None;
            });
            return Ok(());
        }
        set(|v| {
            v.latest = Some(m.version.clone());
            v.notes = Some(m.notes.clone()).filter(|n| !n.is_empty());
        });
        // Said right away, not only once the download starts.
        folder_writable()?;
        let skipped = auto && rolled_back_version().as_deref() == Some(m.version.as_str());
        if !skipped && ready_version().as_deref() != Some(m.version.as_str()) {
            download(&http, &m)?;
            if !auto {
                forget(rolled_back_file());
            }
        }
        Ok(())
    })();
    let (worked, shown) = if auto { after_auto(&result) } else { (result.is_ok(), true) };
    if worked {
        mark_checked();
    }
    let error = result.err().map(|e| e.msg);
    set(|v| {
        v.busy = false;
        if worked {
            v.last_checked = Some(crate::db::now_ms());
        }
        if worked || shown {
            v.checked = true;
            v.error = error;
        } else {
            // Out of reach for a moment: Settings keeps what it showed before.
            v.checked = before.checked;
            v.error = before.error;
        }
    });
    (view(), worked)
}

// ---------- once a day ----------

fn now_secs() -> u64 {
    (crate::db::now_ms().max(0) / 1000) as u64
}

fn last_check() -> Option<u64> {
    std::fs::read_to_string(stamp_file()?).ok().and_then(|s| s.trim().parse().ok())
}

fn is_due(last: u64, now: u64) -> bool {
    // A clock set back (last check "in the future") must not stop the checks.
    now < last || now - last >= 24 * 3600
}

/// A day has passed since the last check that worked.
pub fn due() -> bool {
    is_due(last_check().unwrap_or(0), now_secs())
}

fn mark_checked() {
    if let Some(f) = stamp_file() {
        let _ = std::fs::write(f, now_secs().to_string());
    }
}

// ---------- putting the update in place ----------

#[derive(Serialize, Deserialize, Debug, Default)]
struct Trial {
    version: String,
    /// The new version has started once already.
    started: bool,
}

fn read_trial() -> Option<Trial> {
    serde_json::from_str(&read_sealed(&trial_file()?)?).ok()
}

fn write_trial(t: &Trial) {
    if let Some(f) = trial_file() {
        write_sealed(&f, &serde_json::to_string(t).unwrap_or_default());
    }
}

/// Another Dobo is running (it holds the single-instance lock).
pub fn another_instance_running() -> bool {
    match option_env!("DOBO_TEST_IDENTIFIER") {
        // A renamed test copy (lib.rs APP_NAME): its own lock.
        Some(id) => lock_exists(&format!("{id}-sim")),
        None => lock_exists(INSTANCE_LOCK),
    }
}

#[cfg(windows)]
fn lock_exists(lock: &str) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    let name: Vec<u16> = lock.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a NUL-terminated name; the handle is closed right away.
    unsafe {
        let h = OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr());
        if h.is_null() {
            return false;
        }
        CloseHandle(h);
        true
    }
}

#[cfg(not(windows))]
fn lock_exists(_lock: &str) -> bool {
    false
}

/// What to do with a version on trial at start.
#[derive(Debug, PartialEq)]
enum TrialStep {
    /// Nothing on trial (or a stale note: forget it).
    None,
    /// The first start of the new version: it is on trial now.
    Start,
    /// The new version started before and stopped within the trial: put the old one back.
    RollBack,
}

/// `old_trusted`: an old copy is there and it is the one Dobo recorded at its last update. One
/// Dobo didn't record is never put back: there is nothing to try.
fn trial_step(t: Option<&Trial>, current: &str, old_trusted: bool) -> TrialStep {
    if !old_trusted {
        return TrialStep::None;
    }
    match t {
        Some(t) if t.version == current && t.started => TrialStep::RollBack,
        _ => TrialStep::Start,
    }
}

/// When this Dobo started (the trial counts from here).
static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);
/// Dobo's window has loaded.
static READY: AtomicBool = AtomicBool::new(false);

/// Dobo's main window has loaded: a version on trial is kept once it has also run `TRIAL`.
pub fn app_ready() {
    if READY.swap(true, Ordering::SeqCst) {
        return;
    }
    let left = TRIAL.saturating_sub(STARTED.elapsed());
    std::thread::spawn(move || {
        std::thread::sleep(left);
        trial_passed();
    });
}

/// Quitting normally: a version on trial whose window loaded worked.
pub fn quit_normally() {
    if READY.load(Ordering::SeqCst) {
        trial_passed();
    }
}

/// The new version ran long enough (or quit normally): it is kept, the old copy goes.
fn trial_passed() {
    let Some(t) = read_trial() else { return };
    if t.version != current() {
        return;
    }
    forget(trial_file());
    forget(old_record_file());
    remove_old_copies();
}

/// Deletes the copy from the last update (it may still be closing: tried again for a while).
fn remove_old_copies() {
    for p in [old_exe(), bad_exe()].into_iter().flatten() {
        if p.is_file() && std::fs::remove_file(&p).is_err() {
            std::thread::spawn(move || {
                for _ in 0..10 {
                    std::thread::sleep(Duration::from_secs(3));
                    if std::fs::remove_file(&p).is_ok() {
                        break;
                    }
                }
            });
        }
    }
}

/// Moves `from` to `to`; if that fails, copies it (so `to` exists either way).
fn move_or_copy(from: &Path, to: &Path) -> bool {
    std::fs::rename(from, to).is_ok() || std::fs::copy(from, to).is_ok()
}

/// At the very start, before the window or anything else (the data folder is known by then,
/// `paths::init_early`): puts a downloaded update in place and starts it. Returns true when this
/// process should exit (another version is starting).
pub fn apply_at_start() -> bool {
    LazyLock::force(&STARTED);
    // A development build or a test copy never puts a release in its own place (`enabled`).
    if !enabled() || crate::paths::place().is_none() {
        return false;
    }
    forget_old_put_back_note();
    // As Windows gave them (a name that isn't valid Unicode is passed on unchanged).
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // "Restart now": wait for the Dobo that asked to be gone first.
    let mut waited = false;
    if let Some(i) = args.iter().position(|a| a == "--wait-pid")
        && let Some(pid) = args.get(i + 1).and_then(|p| p.to_str()).and_then(|p| p.parse::<u32>().ok())
    {
        wait_for_exit(pid, Duration::from_secs(20));
        // A Dobo slow to quit still holds its lock: waited for a while longer; still there,
        // this start changes nothing (it hands over as usual).
        let until = Instant::now() + Duration::from_secs(40);
        while another_instance_running() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(250));
        }
        waited = !another_instance_running();
    }
    // Another Dobo is running and uses this exe: change nothing under it (this start only hands
    // over to it). "Restart now" waited for the old one to quit.
    if !waited && another_instance_running() {
        return false;
    }
    let (Some(exe), Some(new), Some(old), Some(mark)) = (exe(), new_exe(), old_exe(), marker()) else { return false };

    let trial = read_trial();
    let old_trusted = old.is_file() && old_copy_trusted(&old);
    match trial_step(trial.as_ref(), current(), old_trusted) {
        TrialStep::Start => {
            write_trial(&Trial { version: current().into(), started: true });
            // On trial: the old copy stays until `trial_passed`.
            return false;
        }
        TrialStep::RollBack => {
            if let Some(bad) = bad_exe() {
                let _ = std::fs::remove_file(&bad);
                if std::fs::rename(&exe, &bad).is_ok() {
                    if move_or_copy(&old, &exe) {
                        write_rolled_back(current());
                        forget(old_record_file());
                        forget(trial_file());
                        // This process is the version that didn't start properly, now named
                        // `bad`: it quits even if the old one can't be started from here (the
                        // next start is the old one, under Dobo's own name).
                        let _ = std::process::Command::new(&exe).args(strip_wait_pid(args)).spawn();
                        return true;
                    }
                    // Couldn't put the old one back: keep this one.
                    let _ = move_or_copy(&bad, &exe);
                }
            }
            forget(trial_file());
        }
        TrialStep::None => {
            if trial.is_some() {
                forget(trial_file());
            }
            // An old copy Dobo didn't record goes, unrun.
            remove_old_copies();
            forget(old_record_file());
        }
    }

    if !new.is_file() {
        return false;
    }
    // Checked again here: the file sat on disk since it was downloaded.
    let checked = std::fs::read_to_string(&mark)
        .ok()
        .and_then(|s| serde_json::from_str::<Manifest>(&s).ok())
        .filter(|m| verify(m).is_ok() && newer(&m.version, current()))
        .filter(|m| std::fs::metadata(&new).map(|f| f.len()).ok() == Some(m.size))
        .filter(|m| sha256_file(&new).ok().as_deref() == Some(m.sha256.as_str()));
    let Some(m) = checked else {
        let _ = std::fs::remove_file(&new);
        let _ = std::fs::remove_file(&mark);
        return false;
    };
    let version = m.version.trim_start_matches('v').to_string();
    // Written first: the new version reads them as soon as it starts. The record says which copy
    // may be put back (this one, about to become the old copy).
    let Ok(this_sha) = sha256_file(&exe) else { return false };
    write_old_record(&OldCopy { sha256: this_sha, version: current().into() });
    write_trial(&Trial { version: version.clone(), started: false });
    match swap_and_start(&exe, &new, &old, |exe| std::process::Command::new(exe).args(strip_wait_pid(args)).spawn().is_ok()) {
        Swap::InPlace => {
            let _ = std::fs::remove_file(&mark);
            true
        }
        Swap::PutBack => {
            // Windows wouldn't start it (an antivirus or a company rule that blocks new
            // programs): this version stays, under its own name, and says so in Settings. The
            // automatic check doesn't fetch that version again; "Check for updates" tries again.
            forget(trial_file());
            let _ = std::fs::remove_file(&new);
            let _ = std::fs::remove_file(&mark);
            write_rolled_back(&version);
            forget(old_record_file());
            false
        }
        Swap::Unchanged => {
            forget(trial_file());
            forget(old_record_file());
            false
        }
    }
}

/// What `swap_and_start` did.
#[derive(Debug, PartialEq)]
enum Swap {
    /// The new version is in place: started (or waiting for the next start). This process quits.
    InPlace,
    /// It couldn't be put in place: nothing changed.
    Unchanged,
    /// It was put in place but Windows wouldn't start it: everything is back as it was.
    PutBack,
}

/// Puts `new` in place of the running `exe` (which becomes `old`) and starts it. If it can't be
/// started, the running version gets its own name back (start with Windows and the `ztodo:` links
/// point at `exe`). `exe` exists whatever happens.
fn swap_and_start(exe: &Path, new: &Path, old: &Path, start: impl FnOnce(&Path) -> bool) -> Swap {
    let _ = std::fs::remove_file(old);
    if std::fs::rename(exe, old).is_err() {
        return Swap::Unchanged;
    }
    if std::fs::rename(new, exe).is_err() {
        let _ = move_or_copy(old, exe);
        return Swap::Unchanged;
    }
    if start(exe) {
        return Swap::InPlace;
    }
    if std::fs::rename(exe, new).is_ok() {
        if std::fs::rename(old, exe).is_ok() {
            return Swap::PutBack;
        }
        // Dobo.exe must exist: the new one goes back in place.
        let _ = move_or_copy(new, exe);
    }
    Swap::InPlace
}

fn strip_wait_pid(args: Vec<OsString>) -> Vec<OsString> {
    let mut out = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--wait-pid" {
            it.next();
        } else {
            out.push(a);
        }
    }
    out
}

/// The arguments "Restart now" starts Dobo with: only the wait, and `--hidden` when Dobo's window
/// was hidden (the new Dobo stays in the tray then).
fn restart_args(pid: u32, hidden: bool) -> Vec<String> {
    let mut a = vec!["--wait-pid".to_string(), pid.to_string()];
    if hidden {
        a.push("--hidden".into());
    }
    a
}

/// "Restart now": starts this exe again, told to wait until this process has quit (the update is
/// put in place then). The caller quits right after.
pub fn restart(hidden: bool) -> Result<(), String> {
    let exe = exe().ok_or_else(|| tr!("Can't find where Dobo is.", "Impossible de trouver où est Dobo."))?;
    std::process::Command::new(exe).args(restart_args(std::process::id(), hidden)).spawn().map_err(|e| tr!("Couldn't restart Dobo: {}", "Impossible de redémarrer Dobo : {}", e))?;
    Ok(())
}

#[cfg(windows)]
fn wait_for_exit(pid: u32, limit: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    // SAFETY: plain handle use; a null handle means the process is already gone.
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, limit.as_millis() as u32);
            CloseHandle(h);
        }
    }
}

#[cfg(not(windows))]
fn wait_for_exit(_pid: u32, limit: Duration) {
    std::thread::sleep(limit.min(Duration::from_secs(2)));
}

/// Notes sealed for this Windows user (DPAPI): a copied data folder on another PC or account
/// reads them as nothing.
#[cfg(windows)]
mod seal {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData};

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        // SAFETY: the input blob points at `data` for the call; the output is copied, then freed.
        unsafe {
            let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            if CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) == 0 {
                return None;
            }
            let out = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            LocalFree(output.pbData as _);
            Some(out)
        }
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        // SAFETY: as above.
        unsafe {
            let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            if CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) == 0 {
                return None;
            }
            let out = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            LocalFree(output.pbData as _);
            Some(out)
        }
    }
}

/// (Updates are Windows-only for now: elsewhere the notes are kept as they are.)
#[cfg(not(windows))]
mod seal {
    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        Some(data.to_vec())
    }
    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        Some(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// A file named like one of Windows' DLLs next to a portable Dobo.exe must never be loaded:
    /// the linker setting covers the DLLs Dobo.exe names, `main` the ones asked for while it runs,
    /// and the release script refuses an exe without it.
    #[test]
    fn dobo_exe_takes_dlls_from_windows_only() {
        assert!(include_str!("../build.rs").contains(r#"println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");"#));
        let main = include_str!("main.rs");
        let call = main.find("    system_dlls_only();").expect("main calls system_dlls_only");
        assert!(call < main.find("std::env::args_os()").unwrap(), "before anything else");
        assert!(include_str!("../../scripts/release.ps1").contains("ToUInt16($bytes, $cfgAt + 0x4E) -ne 0x800"));
    }

    #[test]
    fn compares_versions() {
        assert!(newer("1.0.10", "1.0.9"));
        assert!(newer("v1.1.0", "1.0.9"));
        assert!(!newer("1.0.0", "1.0.0"));
        assert!(!newer("1.0.0", "1.1.0"));
        assert!(newer("1.0.0", "1.0.0-beta.2"));
        assert!(!newer("1.0.0-beta.2", "1.0.0"));
        assert!(!newer("1.0.0+build.2", "1.0.0+build.1"));
        assert!(!newer("not a version", "1.0.0"));
    }

    fn manifest(key: &SigningKey, version: &str) -> Manifest {
        let sha256 = "ab".repeat(32);
        let (size, notes) = (1234, "Fixes".to_string());
        let signature = B64.encode(key.sign(signed_message(version, &sha256, size, &notes).as_bytes()).to_bytes());
        Manifest { version: version.into(), sha256, size, notes, signature }
    }

    /// The signature covers the version, the checksum, the size and the notes; another key, or
    /// anything changed, is refused.
    #[test]
    fn verifies_signatures() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let vk = key.verifying_key();
        let m = manifest(&key, "1.0.1");
        assert!(verify_with(&vk, &m).is_ok());
        assert!(verify_with(&vk, &Manifest { size: 99, ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { notes: "Other".into(), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { version: "1.0.2".into(), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { sha256: "cd".repeat(32), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { signature: String::new(), ..m.clone() }).is_err());
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key();
        assert!(verify_with(&other, &m).is_err());
        // Odd versions or checksums are refused before anything else.
        assert!(verify_with(&vk, &Manifest { version: "../1.0.1".into(), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { sha256: "zz".into(), ..m.clone() }).is_err());
        // Read back from JSON the same.
        let back: Manifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
        // Not signed by Dobo's own key.
        assert!(verify(&m).is_err());
    }

    /// Dobo's key is its own: valid, and never Pako's.
    #[test]
    fn the_embedded_key_is_valid_and_dobos_own() {
        assert!(public_key().is_ok());
        assert_ne!(PUBLIC_KEY, "Hy25ZR31ovufrFfrE47WT3n3/hLld+F8ehrPZ83YHRw=", "Pako's key");
    }

    /// A built release, before it's published: `DOBO_MANIFEST=dist/<version>/latest.json cargo
    /// test --lib -- --ignored built_release`.
    #[test]
    #[ignore]
    fn built_release_is_signed_with_dobos_key() {
        let path = std::env::var("DOBO_MANIFEST").expect("DOBO_MANIFEST");
        let m: Manifest = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        verify(&m).expect("signed with Dobo's key");
    }

    /// A development build (as these tests are) never goes online for an update nor puts one in
    /// place, even when asked with "Check for updates".
    #[cfg(debug_assertions)]
    #[test]
    fn a_development_build_never_updates() {
        assert!(!enabled());
        assert!(!apply_at_start());
        let v = check();
        assert!(!v.busy && v.latest.is_none() && !v.ready);
        assert!(v.error.is_some_and(|e| e.contains("never updates itself")));
        assert!(v.last_checked.is_none(), "not counted as a check");
    }

    /// Only GitHub's release page and its download host, over HTTPS (after any redirect).
    #[test]
    fn downloads_only_from_github_over_https() {
        assert!(from_github("https://github.com/sookera/dobo-releases/releases/latest/download/latest.json"));
        assert!(from_github("https://release-assets.githubusercontent.com/github-production-release-asset/1/2?sp=r"));
        assert!(from_github("https://objects.githubusercontent.com/x"));
        assert!(from_github("https://GitHub.com:443/sookera"));
        assert!(!from_github("http://github.com/sookera/dobo-releases"));
        assert!(!from_github("https://github.com.example.com/x"));
        assert!(!from_github("https://example.com/?u=https://github.com/"));
        assert!(!from_github("https://github.com@example.com/x"));
        assert!(!from_github("https://github.com:8443/x"));
        assert!(!from_github("https://githubusercontent.com/x"));
        assert!(!from_github("https://evilgithubusercontent.com/x"));
        assert!(!from_github(""));
        // Where Dobo asks first.
        assert!(from_github(RELEASES));
    }

    #[test]
    fn drops_the_wait_argument() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let os = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(strip_wait_pid(os(&["--hidden", "--wait-pid", "42"])), os(&["--hidden"]));
        assert_eq!(restart_args(42, false), a(&["--wait-pid", "42"]));
        assert_eq!(restart_args(42, true), a(&["--wait-pid", "42", "--hidden"]));
    }

    fn swap_dir() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("dobo-swap-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (exe, new, old) = (dir.join("Dobo.exe"), dir.join("Dobo.new.exe"), dir.join("Dobo.old.exe"));
        std::fs::write(&exe, b"running").unwrap();
        std::fs::write(&new, b"update").unwrap();
        (dir, exe, new, old)
    }

    #[test]
    fn swaps_in_the_update() {
        let (dir, exe, new, old) = swap_dir();
        let mut started = None;
        assert_eq!(
            swap_and_start(&exe, &new, &old, |p| {
                started = Some(std::fs::read(p).unwrap());
                true
            }),
            Swap::InPlace
        );
        assert_eq!(started.as_deref(), Some(&b"update"[..]));
        assert_eq!(std::fs::read(&old).unwrap(), b"running");
        assert!(!new.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_update_that_wont_start_is_put_back() {
        let (dir, exe, new, old) = swap_dir();
        assert_eq!(swap_and_start(&exe, &new, &old, |_| false), Swap::PutBack);
        assert_eq!(std::fs::read(&exe).unwrap(), b"running");
        assert_eq!(std::fs::read(&new).unwrap(), b"update");
        assert!(!old.exists());
        std::fs::remove_file(&new).unwrap();
        assert_eq!(swap_and_start(&exe, &new, &old, |_| true), Swap::Unchanged);
        assert_eq!(std::fs::read(&exe).unwrap(), b"running");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_put_back_note_ends_once_a_later_update_worked() {
        assert_eq!(put_back_note("1.0.1\n", "1.0.0"), Some("1.0.1".into()));
        assert_eq!(put_back_note("1.0.1", "1.0.2"), None);
        assert_eq!(put_back_note("1.0.1", "1.0.1"), None);
        assert_eq!(put_back_note("  ", "1.0.0"), None);
    }

    #[test]
    fn decides_the_trial() {
        let t = |v: &str, started: bool| Trial { version: v.into(), started };
        assert_eq!(trial_step(Some(&t("1.0.1", false)), "1.0.1", true), TrialStep::Start);
        assert_eq!(trial_step(Some(&t("1.0.1", true)), "1.0.1", true), TrialStep::RollBack);
        assert_eq!(trial_step(Some(&t("1.0.1", true)), "1.0.1", false), TrialStep::None);
        assert_eq!(trial_step(None, "1.0.1", false), TrialStep::None);
        assert_eq!(trial_step(None, "1.0.1", true), TrialStep::Start);
        assert_eq!(trial_step(Some(&t("1.0.1", true)), "1.0.0", true), TrialStep::Start);
    }

    /// Only the copy Dobo recorded at its last update may be put back and run: a `Dobo.old.exe`
    /// planted next to Dobo.exe (by a browser download, an extracted archive) never is.
    #[test]
    fn only_the_recorded_old_copy_is_trusted() {
        let r = OldCopy { sha256: "AB12".into(), version: "1.0.0".into() };
        assert!(old_trusted_by(Some(&r), || Some("ab12".into())));
        assert!(!old_trusted_by(Some(&r), || Some("cd34".into())));
        assert!(!old_trusted_by(None, || Some("ab12".into())));
        assert!(!old_trusted_by(Some(&OldCopy::default()), || Some(String::new())));
        assert!(!old_trusted_by(Some(&r), || None));
    }

    #[test]
    fn a_failed_automatic_check_is_tried_again() {
        assert_eq!(after_auto(&Ok(())), (true, false));
        assert_eq!(after_auto(&Err(Fail::unreachable())), (false, false));
        assert_eq!(after_auto(&Err(Fail::other("Dobo's folder can't be written to".into()))), (false, true));
    }

    #[test]
    fn daily_check_survives_a_clock_set_back() {
        assert!(is_due(0, 100_000));
        assert!(!is_due(100_000, 100_000 + 3600));
        assert!(is_due(100_000, 100_000 + 24 * 3600));
        assert!(is_due(200_000, 100_000));
    }

    /// The single-instance lock follows the identifier (tauri.conf.json).
    #[test]
    fn instance_lock_matches_the_identifier() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(INSTANCE_LOCK, format!("{}-sim", conf["identifier"].as_str().unwrap()));
    }

    /// Sealed notes read back for this user; anything else reads as nothing.
    #[test]
    fn sealed_notes_read_back() {
        let dir = std::env::temp_dir().join(format!("dobo-seal-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("note.bin");
        write_sealed(&f, r#"{"version":"1.0.1","started":true}"#);
        assert_eq!(read_sealed(&f).as_deref(), Some(r#"{"version":"1.0.1","started":true}"#));
        #[cfg(windows)]
        {
            std::fs::write(&f, br#"{"version":"1.0.1","started":true}"#).unwrap();
            assert_eq!(read_sealed(&f), None, "a plain note planted there isn't trusted");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
