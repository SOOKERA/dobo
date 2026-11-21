// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! A pinned docked panel steps aside for full-screen apps (a video, a game, a presentation,
//! screen sharing) on its screen, and comes back when full screen ends — automatically.
//!
//! Only watched while it matters (docked, pinned, and shown or stepped aside), about once a
//! second; the watcher stops by itself otherwise. Reminders and notifications are unaffected.
//! Windows asks the shell (SHQueryUserNotificationState) and looks at the window in front;
//! other systems: nothing yet.

use crate::{AppState, window};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

/// What the system says about the window in front.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Front {
    /// One of our own windows (the panel, the dump bar…).
    pub ours: bool,
    /// The desktop or the taskbar (Progman, WorkerW, Shell_TrayWnd).
    pub shell: bool,
    /// On the same screen as the panel.
    pub same_screen: bool,
    /// Covers its whole screen (not just the work area: the taskbar is hidden).
    pub covers_screen: bool,
    /// Covers almost all of it (at least 95%: borderless "full screen" windows).
    pub almost_covers: bool,
    /// The shell says a full-screen app, a Direct3D game or presentation mode is on.
    pub shell_busy: bool,
}

/// Another app is full screen on the panel's screen.
pub fn other_app_full_screen(f: &Front) -> bool {
    !f.ours && !f.shell && f.same_screen && (f.covers_screen || (f.shell_busy && f.almost_covers))
}

#[derive(Debug, PartialEq)]
pub enum Step {
    /// Step aside (and remember why).
    Hide,
    /// Full screen ended: come back.
    Show,
    /// Forget why it was hidden (unpinned, floating, or opened again by hand).
    Forget,
    Nothing,
}

/// One look: what to do now.
pub fn step(stepped_aside: bool, visible: bool, pinned: bool, docked: bool, full: bool) -> Step {
    if stepped_aside {
        if !pinned || !docked || visible {
            return Step::Forget;
        }
        return if full { Step::Nothing } else { Step::Show };
    }
    if visible && pinned && docked && full { Step::Hide } else { Step::Nothing }
}

/// Worth watching: pinned and docked, and shown or stepped aside.
pub fn worth_watching(stepped_aside: bool, visible: bool, pinned: bool, docked: bool) -> bool {
    pinned && docked && (visible || stepped_aside)
}

fn state_now(app: &AppHandle) -> (bool, bool, bool, bool) {
    let st = app.state::<AppState>();
    let docked = window::info(app).mode == "docked";
    let visible = app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false) && !st.peeking.load(Ordering::SeqCst);
    (st.fs_hidden.load(Ordering::SeqCst), visible, st.pinned.load(Ordering::SeqCst), docked)
}

/// Starts watching (once) when it is worth it.
pub fn watch(app: &AppHandle) {
    if !cfg!(windows) {
        return;
    }
    let st = app.state::<AppState>();
    if st.fs_watching.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let (aside, visible, pinned, docked) = state_now(&app);
            if !worth_watching(aside, visible, pinned, docked) {
                let st = app.state::<AppState>();
                st.fs_hidden.store(false, Ordering::SeqCst);
                st.fs_watching.store(false, Ordering::SeqCst);
                return;
            }
            let Some(win) = app.get_webview_window("main") else { continue };
            let full = imp::front(&win).is_some_and(|f| other_app_full_screen(&f));
            let st = app.state::<AppState>();
            match step(aside, visible, pinned, docked, full) {
                Step::Hide => {
                    st.fs_hidden.store(true, Ordering::SeqCst);
                    let _ = win.hide();
                }
                Step::Show => {
                    st.fs_hidden.store(false, Ordering::SeqCst);
                    window::show_quietly(&app);
                }
                Step::Forget => st.fs_hidden.store(false, Ordering::SeqCst),
                Step::Nothing => {}
            }
        }
    });
}

#[cfg(windows)]
mod imp {
    use super::Front;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
    use windows::Win32::UI::Shell::{QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId};

    pub fn front(win: &tauri::WebviewWindow) -> Option<Front> {
        let ours: HWND = win.hwnd().ok()?;
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            let mut buf = [0u16; 64];
            let n = GetClassNameW(fg, &mut buf).max(0) as usize;
            let class = String::from_utf16_lossy(&buf[..n]);
            let screen = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let mut r = RECT::default();
            if !GetMonitorInfoW(screen, &mut mi).as_bool() || GetWindowRect(fg, &mut r).is_err() {
                return None;
            }
            let m = mi.rcMonitor;
            let covers = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
            let area = |x: &RECT| ((x.right - x.left).max(0) as i64) * ((x.bottom - x.top).max(0) as i64);
            let inter = RECT { left: r.left.max(m.left), top: r.top.max(m.top), right: r.right.min(m.right), bottom: r.bottom.min(m.bottom) };
            let almost = area(&m) > 0 && area(&inter) * 100 >= area(&m) * 95;
            let busy = SHQueryUserNotificationState().is_ok_and(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE);
            Some(Front {
                ours: pid == std::process::id(),
                shell: ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"].contains(&class.as_str()),
                same_screen: screen == MonitorFromWindow(ours, MONITOR_DEFAULTTONEAREST),
                covers_screen: covers,
                almost_covers: almost,
                shell_busy: busy,
            })
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn front(_win: &tauri::WebviewWindow) -> Option<super::Front> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIDEO: Front = Front { ours: false, shell: false, same_screen: true, covers_screen: true, almost_covers: true, shell_busy: true };

    #[test]
    fn only_another_app_full_screen_on_this_screen_counts() {
        assert!(other_app_full_screen(&VIDEO));
        assert!(!other_app_full_screen(&Front { same_screen: false, ..VIDEO }), "another screen: stays");
        assert!(!other_app_full_screen(&Front { ours: true, ..VIDEO }), "our own window");
        assert!(!other_app_full_screen(&Front { shell: true, ..VIDEO }), "the desktop");
        // A maximised window leaves the taskbar: not full screen.
        let maximised = Front { covers_screen: false, almost_covers: true, shell_busy: false, ..VIDEO };
        assert!(!other_app_full_screen(&maximised));
        // A borderless game that misses a pixel, with the shell saying busy: full screen.
        assert!(other_app_full_screen(&Front { covers_screen: false, ..VIDEO }));
        // The shell busy but the window in front is small: not it.
        assert!(!other_app_full_screen(&Front { covers_screen: false, almost_covers: false, ..VIDEO }));
    }

    #[test]
    fn steps_aside_and_comes_back_only_for_that_reason() {
        // Shown, pinned, docked, full screen starts: hide.
        assert_eq!(step(false, true, true, true, true), Step::Hide);
        // Not pinned (it hides on its own when you click away): nothing.
        assert_eq!(step(false, true, false, true, true), Step::Nothing);
        // Floating: nothing.
        assert_eq!(step(false, true, true, false, true), Step::Nothing);
        // Hidden by hand (not by us): never brought back.
        assert_eq!(step(false, false, true, true, false), Step::Nothing);
        // Stepped aside, still full screen: wait.
        assert_eq!(step(true, false, true, true, true), Step::Nothing);
        // Full screen ended: back.
        assert_eq!(step(true, false, true, true, false), Step::Show);
        // Unpinned or opened by hand meanwhile: forget it.
        assert_eq!(step(true, false, false, true, false), Step::Forget);
        assert_eq!(step(true, true, true, true, true), Step::Forget);
    }

    #[test]
    fn the_watch_stops_when_it_no_longer_matters() {
        assert!(worth_watching(false, true, true, true));
        assert!(worth_watching(true, false, true, true));
        assert!(!worth_watching(false, false, true, true), "hidden by hand");
        assert!(!worth_watching(false, true, false, true), "not pinned");
        assert!(!worth_watching(true, false, true, false), "now floating");
    }
}
