// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! When Dobo can't start (its notes locked or damaged, a full or read-only disk, no WebView2), or
//! stops on an error it can't go on from: a plain Windows message box (no webview needed) and a
//! short note in the data folder, never a silent exit. The notes are saved as they are written,
//! so nothing typed is lost; the box says where they and their backups are.

use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Notes kept small: a file past this size starts again.
const NOTE_MAX: u64 = 64 * 1024;

/// The data folder, or the usual one when it isn't decided yet.
fn data_folder() -> Option<PathBuf> {
    crate::paths::place().map(|p| p.dir.clone()).or_else(crate::paths::usual_dir)
}

/// Adds a dated line to `name` in `dir` (only the error and where it happened, cleaned: never a
/// note's words, a file's name or the user folder). Returns the file.
fn write_note(dir: &Path, name: &str, text: &str) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    let path = dir.join(name);
    let fresh = std::fs::metadata(&path).is_ok_and(|m| m.len() > NOTE_MAX);
    let mut f = std::fs::OpenOptions::new().create(true).append(!fresh).write(true).truncate(fresh).open(&path).ok()?;
    // Cleaned as it is written (report.rs): nothing that could say who you are, even here.
    let line = crate::report::clean(text);
    // The time with its offset then ("2026-11-23T15:37:00+03:00"), so a report says it as it
    // was, across clock changes and travel (older lines have "2026-11-23_153700").
    let now = crate::db::now_ms();
    let when = crate::clock::iso_local(now).unwrap_or_else(|| crate::clock::stamp(now));
    writeln!(f, "{when} Dobo {}: {line}", env!("CARGO_PKG_VERSION")).ok()?;
    Some(path)
}

/// A note in the data folder, or in the temp folder when the data folder can't be written.
fn note(name: &str, text: &str) -> Option<PathBuf> {
    data_folder().and_then(|d| write_note(&d, name, text)).or_else(|| write_note(&std::env::temp_dir().join("Dobo"), name, text))
}

/// What the box says when Dobo couldn't start.
fn startup_message(error: &str, data: Option<&Path>, log: Option<&Path>) -> String {
    let mut m = tr!("Dobo couldn’t start:\n{}", "Dobo n’a pas pu démarrer :\n{}", error.trim());
    if error.to_ascii_lowercase().contains("webview") {
        m += &tr!(
            "\n\nDobo needs Microsoft Edge WebView2, which isn’t on this PC. It can be installed from https://go.microsoft.com/fwlink/p/?LinkId=2124703",
            "\n\nDobo a besoin de Microsoft Edge WebView2, absent de ce PC. Il s’installe depuis https://go.microsoft.com/fwlink/p/?LinkId=2124703"
        );
    }
    if let Some(d) = data {
        m += &tr!(
            "\n\nYour notes are in {}, and their backups in its “backups” folder (unless you chose another one in Settings).",
            "\n\nVos notes sont dans {}, et leurs sauvegardes dans son dossier « backups » (sauf si vous en avez choisi un autre dans les Paramètres).",
            d.display()
        );
    }
    if let Some(l) = log {
        m += &tr!("\n\nThis was written down in {}.", "\n\nCeci a été noté dans {}.", l.display());
    }
    if data.is_some() {
        m += &tr!("\n\nOpen the folder of your notes?", "\n\nOuvrir le dossier de vos notes ?");
    }
    m
}

/// How long the thread of the box saying Dobo stopped may take to start. Starting a thread
/// never takes this long, except as Windows ends the process (it then holds its loader lock, and
/// no thread starts until it lets go, which it never does while Dobo waits): no box then, only
/// the note.
const BOX_START: std::time::Duration = std::time::Duration::from_secs(2);
/// The same for the box saying Dobo couldn't start (nothing is ending then: a box is due, never
/// a silent exit).
const STARTUP_BOX_START: std::time::Duration = std::time::Duration::from_secs(10);

/// Shows the box on a thread of its own (its own message loop: the app's windows aren't run
/// meanwhile), if that thread starts within `start` (`on_own_thread`). With `ask`, Yes / No;
/// returns whether Yes was clicked.
fn show(text: &str, ask: bool, start: std::time::Duration) -> bool {
    let text = text.to_string();
    on_own_thread(start, move |go_on| go_on() && show_here(&text, ask))
}

/// `job` on a thread of its own, waited for (returns its result; false if it panicked), unless
/// that thread hasn't started within `start`: then false at once, never waiting for it. `job` is
/// given `go_on`, to call first: whether it is still waited for (a thread that starts too late
/// does nothing).
fn on_own_thread(start: std::time::Duration, job: impl FnOnce(&dyn Fn() -> bool) -> bool + Send + 'static) -> bool {
    use std::sync::{Arc, Condvar, Mutex};
    #[derive(PartialEq)]
    enum Start {
        Waiting,
        Started,
        GivenUp,
    }
    let state = Arc::new((Mutex::new(Start::Waiting), Condvar::new()));
    let theirs = state.clone();
    let spawned = std::thread::Builder::new().name("dobo-box".into()).spawn(move || {
        let go_on = || {
            let (m, c) = &*theirs;
            let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
            if *s != Start::Waiting {
                return false;
            }
            *s = Start::Started;
            c.notify_all();
            true
        };
        job(&go_on)
    });
    let Ok(thread) = spawned else { return false };
    let (m, c) = &*state;
    let s = m.lock().unwrap_or_else(|e| e.into_inner());
    let (mut s, _) = c.wait_timeout_while(s, start, |s| *s == Start::Waiting).unwrap_or_else(|e| e.into_inner());
    if *s == Start::Started {
        drop(s);
        return thread.join().unwrap_or(false);
    }
    // (Its thread is let go: should it start later, it does nothing.)
    *s = Start::GivenUp;
    false
}

/// The box, on this very thread.
fn show_here(text: &str, ask: bool) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW};
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (t, c) = (wide(text), wide(crate::APP_NAME));
        // In front, over Dobo's own windows too (the panel and the floating notes stay on top).
        let style = MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST | if ask { MB_YESNO } else { MB_OK };
        // SAFETY: NUL-terminated strings that live through the call; no owner window.
        unsafe { MessageBoxW(None, windows::core::PCWSTR(t.as_ptr()), windows::core::PCWSTR(c.as_ptr()), style) == IDYES }
    }
    #[cfg(not(windows))]
    {
        let _ = ask;
        eprintln!("{text}");
        false
    }
}

/// Dobo couldn't start: the box, the note, and the folder of the notes opened when asked. The
/// caller then ends Dobo.
pub fn startup_failed(error: &str) {
    eprintln!("Dobo couldn't start: {error}");
    let log = note("startup-error.log", error);
    let data = data_folder();
    if show(&startup_message(error, data.as_deref(), log.as_deref()), data.is_some(), STARTUP_BOX_START)
        && let Some(d) = data
    {
        let _ = std::fs::create_dir_all(&d);
        #[cfg(windows)]
        let _ = std::process::Command::new("explorer.exe").arg(&d).spawn();
    }
}

/// Dobo is ending (`exiting`): a panic then is only noted, no box.
static EXITING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Dobo is ending: from here on, an error is noted without a box. A box would need a thread of
/// its own, which can't start while Windows holds its loader lock as the process ends (waiting
/// for one would hang Dobo instead of letting it stop).
pub fn exiting() {
    EXITING.store(true, std::sync::atomic::Ordering::Release);
}

/// This start goes on after all: `exiting` was said for a start that was to hand over to another
/// Dobo, which ended meanwhile.
pub fn not_exiting() {
    EXITING.store(false, std::sync::atomic::Ordering::Release);
}

/// Where the box for an error Dobo can't go on from is shown (`catch_panics`).
#[derive(Debug, PartialEq)]
enum Shown {
    /// No box, only the note.
    NoteOnly,
    /// From a thread of its own, waited for (`show`).
    OwnThread,
    /// On the very thread of the error (`show_here`).
    Here,
}

/// `release`: a release build; `exiting`: Dobo is ending; `on_main`: the error is on the main
/// thread; `first`: no such box was shown yet (asked last, only when a box may be shown: it
/// counts the box as shown).
fn shown(release: bool, exiting: bool, on_main: bool, first: impl FnOnce() -> bool) -> Shown {
    if !release || exiting || !first() {
        Shown::NoteOnly
    } else if on_main {
        Shown::OwnThread
    } else {
        Shown::Here
    }
}

/// An error Dobo can't go on from (a panic: release builds then stop at once): a note of where it
/// happened, and a box saying so, before it stops. The box only in release builds (a debug build
/// goes on after a panic on another thread: no box saying it stopped), once, and never as Dobo
/// ends (`exiting`). On the main thread, the box is shown from a thread of its own, waited for:
/// its message loop there would run the windows' (the board's calls among them) while the
/// panicking code still holds the notes' lock, and hang. (Should that thread not start, as when
/// Windows is ending the process, no box: `BOX_START`.) On any other thread, on that thread.
pub fn catch_panics() {
    std::panic::set_hook(Box::new(|info| {
        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let place = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        let what: String = what.chars().take(400).collect();
        eprintln!("Dobo stopped: {what}{place}");
        let log = note("crash.log", &format!("{what}{place}"));
        let mut m = tr!(
            "Dobo stopped because of an error it couldn’t go on from. Your notes are saved as you write, so nothing is lost: start Dobo again.",
            "Dobo s’est arrêté à cause d’une erreur. Vos notes sont enregistrées au fil de l’écriture, rien n’est perdu : relancez Dobo."
        );
        if let Some(l) = log {
            m += &tr!("\n\nThis was written down in {}.", "\n\nCeci a été noté dans {}.", l.display());
        }
        static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let exiting = EXITING.load(std::sync::atomic::Ordering::Acquire);
        let on_main = std::thread::current().name() == Some("main");
        match shown(cfg!(not(debug_assertions)), exiting, on_main, || !SHOWN.swap(true, std::sync::atomic::Ordering::AcqRel)) {
            Shown::OwnThread => {
                show(&m, false, BOX_START);
            }
            Shown::Here => {
                show_here(&m, false);
            }
            Shown::NoteOnly => {}
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_says_what_happened_and_where_the_notes_are() {
        let data = Path::new(r"C:\Users\Jo\AppData\Roaming\com.sookera.ztodo");
        let log = data.join("startup-error.log");
        let m = startup_message("database disk image is malformed", Some(data), Some(&log));
        assert!(m.contains("database disk image is malformed"));
        assert!(m.contains(&data.display().to_string()) && m.contains("backups"));
        assert!(m.contains("startup-error.log"));
        assert!(m.ends_with("Open the folder of your notes?"));
        assert!(startup_message("WebView2 Runtime not found", None, None).contains("WebView2, which isn’t on this PC"));
        assert!(!startup_message("disk full", None, None).contains("Open the folder"));
    }

    #[test]
    fn a_note_stays_small_and_on_one_line() {
        let dir = std::env::temp_dir().join(format!("dobo-failure-{}-{:?}", crate::db::now_ms(), std::thread::current().id()));
        let p = write_note(&dir, "crash.log", "first\nsecond").unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert!(text.contains("first second"));
        // Its time carries its offset, and reads back as written.
        let stamp = text.split(' ').next().unwrap();
        assert!(stamp.len() == 25 && stamp.as_bytes()[10] == b'T', "{stamp}");
        std::fs::write(&p, vec![b'x'; (NOTE_MAX + 1) as usize]).unwrap();
        write_note(&dir, "crash.log", "again").unwrap();
        assert!(std::fs::metadata(&p).unwrap().len() < 200, "started again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Where the box for an error goes: none in a debug build or as Dobo ends (never counted as
    /// shown then), once, from a thread of its own on the main thread, else right there.
    #[test]
    fn an_error_is_shown_once_and_never_as_dobo_ends() {
        let asked = std::cell::Cell::new(false);
        let first = || {
            asked.set(true);
            true
        };
        assert_eq!(shown(false, false, true, first), Shown::NoteOnly);
        assert!(!asked.get(), "a debug build counts no box as shown");
        assert_eq!(shown(true, true, true, first), Shown::NoteOnly);
        assert!(!asked.get(), "nor does an error as Dobo ends");
        assert_eq!(shown(true, false, true, first), Shown::OwnThread);
        assert!(asked.get());
        assert_eq!(shown(true, false, false, || true), Shown::Here);
        assert_eq!(shown(true, false, true, || false), Shown::NoteOnly, "once");
        assert_eq!(shown(true, false, false, || false), Shown::NoteOnly);
        // (No test here runs the hook, but the flag it reads is the one these set.)
        exiting();
        assert!(EXITING.load(std::sync::atomic::Ordering::Acquire));
        not_exiting();
        assert!(!EXITING.load(std::sync::atomic::Ordering::Acquire));
    }

    /// The box's thread: waited for once it has started, however long the box stays; never
    /// waited for when it doesn't start in time, and then it does nothing.
    #[test]
    fn a_box_whose_thread_doesnt_start_is_never_waited_for() {
        use std::time::{Duration, Instant};
        assert!(on_own_thread(Duration::from_secs(2), |go_on| go_on()));
        let t = Instant::now();
        assert!(on_own_thread(Duration::from_millis(50), |go_on| {
            let on = go_on();
            std::thread::sleep(Duration::from_millis(300));
            on
        }));
        assert!(t.elapsed() >= Duration::from_millis(300), "the box open: waited for");
        let (tell, told) = std::sync::mpsc::channel();
        let t = Instant::now();
        assert!(!on_own_thread(Duration::from_millis(50), move |go_on| {
            // (A thread that starts late, as when Windows ends the process.)
            std::thread::sleep(Duration::from_millis(600));
            let on = go_on();
            let _ = tell.send(on);
            on
        }));
        assert!(t.elapsed() < Duration::from_millis(450), "not waited for");
        assert_eq!(told.recv_timeout(Duration::from_secs(5)), Ok(false), "and it shows nothing");
        assert!(!on_own_thread(Duration::from_secs(2), |go_on| {
            assert!(go_on());
            panic!("the box failed")
        }));
    }
}
