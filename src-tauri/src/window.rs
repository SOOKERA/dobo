// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The Dobo windows.
//!
//! The main window, in its two modes:
//! - docked: slides in at the edge of a screen (the one under the mouse, or the one chosen in
//!   Settings), full height of the work area, no taskbar button, hides when it loses focus
//!   (the page decides, unless pinned);
//! - floating: a normal frameless window the person places and sizes, remembered per screen.
//!
//! The quick dump bar: a small separate window in the middle of the screen.
//!
//! The visible card has a 12 px transparent margin around it inside the window (its shadow
//! lives there), so a docked card sits 12 px from the screen edges.

use crate::{AppState, db};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const MARGIN: f64 = 12.0;
const DEFAULT_DOCKED_WIDTH: f64 = 720.0;
/// The docked panel's width (logical pixels, the card itself): dragged by its inner edge or
/// picked in Settings, remembered per screen.
pub const DOCK_MIN: f64 = 420.0;
const DOCK_MAX: f64 = 1200.0;
const DOCK_PRESETS: [f64; 3] = [560.0, 720.0, 880.0];
const DOCK_SNAP: f64 = 16.0;

/// A wanted width made sensible for a screen whose work area is `work_w` logical pixels wide:
/// a gentle snap to Narrow / Medium / Wide (when dragging), then at least 420 and at most the
/// smaller of 1200 or 70% of the screen.
pub fn docked_width(raw: f64, work_w: f64, snap: bool) -> f64 {
    let raw = if raw.is_finite() { raw } else { DEFAULT_DOCKED_WIDTH };
    let mut w = raw;
    if snap && let Some(p) = DOCK_PRESETS.iter().find(|p| (raw - **p).abs() <= DOCK_SNAP) {
        w = *p;
    }
    let max = DOCK_MAX.min(work_w * 0.7).max(DOCK_MIN);
    w.clamp(DOCK_MIN, max).round()
}

/// The width remembered for a screen, else the last one chosen anywhere, else Medium.
pub fn saved_width(widths: Option<&serde_json::Value>, key: &str, global: Option<f64>) -> f64 {
    let ok = |v: &f64| v.is_finite() && *v > 0.0;
    widths.and_then(|w| w.get(key)).and_then(|v| v.as_f64()).filter(ok).or(global.filter(ok)).unwrap_or(DEFAULT_DOCKED_WIDTH)
}
const DEFAULT_FLOAT: (f64, f64) = (760.0, 820.0);
const DUMP_SIZE: (f64, f64) = (720.0, 176.0);

#[derive(Serialize, Clone)]
pub struct WindowInfo {
    /// "docked" or "floating" (what is really used: docked can't be placed on Wayland).
    pub mode: String,
    /// What the person chose.
    pub chosen: String,
    pub docked_possible: bool,
    pub side: String,
    pub float_on_top: bool,
    /// The docked width on the screen the panel uses (logical pixels).
    pub width: f64,
}

#[derive(Serialize, Clone)]
pub struct MonitorInfo {
    pub key: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

/// Wayland doesn't let an app put its window at a screen edge: docked becomes floating there.
pub fn docked_possible() -> bool {
    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var("XDG_SESSION_TYPE").map(|v| v.eq_ignore_ascii_case("wayland")).unwrap_or(false)
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        return !wayland;
    }
    #[allow(unreachable_code)]
    true
}

/// The window settings read all the time (every 120 ms by the edge watcher): kept here, not read
/// from the notes each time. Refreshed whenever one of them is saved (`set_setting`,
/// `set_float_on_top`) and after a restore (`reload_settings`).
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    /// "docked" or "floating", as chosen.
    pub mode: String,
    pub side: String,
    pub float_on_top: bool,
    /// The screen chosen in Settings ("mouse": the one under the mouse).
    pub monitor: String,
}

/// The settings `Prefs` keeps.
pub const PREF_KEYS: [&str; 4] = ["mode", "side", "float_on_top", "monitor"];

impl Prefs {
    pub fn read(conn: &rusqlite::Connection) -> Prefs {
        let text = |k: &str, d: &str| db::get_setting(conn, k).and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| d.into());
        Prefs {
            mode: text("mode", "docked"),
            side: text("side", "right"),
            float_on_top: db::get_setting(conn, "float_on_top").and_then(|v| v.as_bool()).unwrap_or(false),
            monitor: text("monitor", "mouse"),
        }
    }
}

/// Reads `Prefs` again from the notes (one of them was just saved, or a backup came back).
pub fn reload_prefs(app: &AppHandle, conn: &rusqlite::Connection) {
    let now = Prefs::read(conn);
    *app.state::<AppState>().prefs.lock().unwrap() = now;
}

fn prefs(app: &AppHandle) -> Prefs {
    app.state::<AppState>().prefs.lock().unwrap().clone()
}

pub fn info(app: &AppHandle) -> WindowInfo {
    let p = prefs(app);
    let possible = docked_possible();
    let mode = if p.mode == "floating" || !possible { "floating" } else { "docked" }.to_string();
    WindowInfo { mode, chosen: p.mode, docked_possible: possible, side: p.side, float_on_top: p.float_on_top, width: DEFAULT_DOCKED_WIDTH }
}

/// The docked width for a screen (as remembered, made sensible for it).
fn width_on(app: &AppHandle, mon: &tauri::Monitor) -> f64 {
    let (widths, global) = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        (db::get_setting(&conn, "docked_widths"), db::get_setting(&conn, "docked_width").and_then(|v| v.as_f64()))
    };
    let work_w = mon.work_area().size.width as f64 / mon.scale_factor();
    docked_width(saved_width(widths.as_ref(), &monitor_key(mon), global), work_w, false)
}

/// The window's info with the docked width of the screen it is on (or will open on).
pub fn info_with_width(app: &AppHandle) -> WindowInfo {
    let mut i = info(app);
    if let Some(win) = main_window(app) {
        let here = if win.is_visible().unwrap_or(false) { win.current_monitor().ok().flatten() } else { None };
        if let Some(m) = here.or_else(|| target_monitor(app, &win)) {
            i.width = width_on(app, &m);
        }
    }
    i
}

/// The docked panel's (or the peeking strip's) place on a screen whose work area is `area`
/// (physical: x, y, width, height) at `scale`: full height, against the `left` or right edge,
/// `width` logical pixels for the card plus its margin, never wider than the screen.
pub(crate) fn edge_rect(area: (i32, i32, u32, u32), scale: f64, width: f64, left: bool) -> (i32, i32, u32, u32) {
    let (ax, ay, aw, ah) = area;
    let w = (((width + 2.0 * MARGIN) * scale).round().max(1.0) as u32).min(aw);
    let x = if left { ax } else { ax + aw as i32 - w as i32 };
    (x, ay, w, ah)
}

/// A screen's work area as `edge_rect` takes it.
fn work_area(mon: &tauri::Monitor) -> (i32, i32, u32, u32) {
    let a = mon.work_area();
    (a.position.x, a.position.y, a.size.width, a.size.height)
}

/// Does the window need to reach the screen first, before its size is set? When it moves to a
/// screen of another scale, Windows rescales it on the way (WM_DPICHANGED: 1116 px at 150 %
/// would become 1674): a size set in the same move would be undone.
fn other_scale(win_scale: Option<f64>, mon_scale: f64) -> bool {
    win_scale.is_some_and(|s| (s - mon_scale).abs() > 0.001)
}

/// Sizes and places the window on `mon` (physical pixels for that screen). When it comes from
/// a screen of another scale, it is first moved onto `mon` (its scale changes there), then
/// sized: the size lands as meant. On the same screen, size and place in one move (the
/// screen-edge side stays put while the width changes).
fn place(win: &WebviewWindow, mon: &tauri::Monitor, x: i32, y: i32, w: u32, h: u32) {
    let first_onto = other_scale(win.scale_factor().ok(), mon.scale_factor());
    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos};
        let handle: HWND = hwnd;
        if first_onto {
            // Onto the screen's top-left corner, as big as it is now: mostly on that screen,
            // whose scale it takes (the window's thread handles that within this call).
            let (ax, ay, ..) = work_area(mon);
            let _ = unsafe { SetWindowPos(handle, None, ax, ay, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
        }
        if unsafe { SetWindowPos(handle, None, x, y, w as i32, h as i32, SWP_NOZORDER | SWP_NOACTIVATE) }.is_ok() {
            return;
        }
    }
    // (Elsewhere: placed first, so a change of scale happens before the size is set.)
    let _ = first_onto;
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let _ = win.set_size(PhysicalSize::new(w, h));
}

/// After the scale of the screen under a docked (or peeking) panel changed while it shows (a
/// new display scale in Windows' settings): its width made right for that scale again (Windows
/// rescaled it by the ratio). Nothing when it already is.
pub fn refit(app: &AppHandle) {
    let Some(win) = main_window(app) else { return };
    if !win.is_visible().unwrap_or(false) || !onboarded(app) {
        return;
    }
    let peeking = app.state::<AppState>().peeking.load(Ordering::SeqCst);
    let info = info(app);
    if !peeking && info.mode != "docked" {
        return;
    }
    let Some(mon) = win.current_monitor().ok().flatten() else { return };
    let width = if peeking { PEEK_W } else { width_on(app, &mon) };
    let (x, y, w, h) = edge_rect(work_area(&mon), mon.scale_factor(), width, info.side == "left");
    let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) else { return };
    if (pos.x, pos.y, size.width, size.height) != (x, y, w, h) {
        place(&win, &mon, x, y, w, h);
    }
}

/// The docked panel dragged by its inner edge (or a width picked in Settings): resized live on
/// the screen it is on; `save` remembers it for that screen. Returns the width used.
pub fn resize_docked(app: &AppHandle, width: f64, snap: bool, save: bool) -> Option<f64> {
    let win = main_window(app)?;
    let info = info(app);
    if info.mode != "docked" {
        return None;
    }
    let mon = win.current_monitor().ok().flatten().or_else(|| target_monitor(app, &win))?;
    let scale = mon.scale_factor();
    let w_logical = docked_width(width, mon.work_area().size.width as f64 / scale, snap);
    if win.is_visible().unwrap_or(false) {
        let (x, y, w, h) = edge_rect(work_area(&mon), scale, w_logical, info.side == "left");
        place(&win, &mon, x, y, w, h);
    }
    if save {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let mut widths = db::get_setting(&conn, "docked_widths").filter(|g| g.is_object()).unwrap_or_else(|| serde_json::json!({}));
        widths[monitor_key(&mon)] = serde_json::json!(w_logical);
        let _ = db::set_setting(&conn, "docked_widths", &widths);
        let _ = db::set_setting(&conn, "docked_width", &serde_json::json!(w_logical));
    }
    Some(w_logical)
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

fn monitor_key(m: &tauri::Monitor) -> String {
    let p = m.position();
    format!("{}@{},{}", m.name().cloned().unwrap_or_default(), p.x, p.y)
}

/// Is where the window is now the floating panel's place to remember? Floating, set up (the
/// first-launch window is another window), and on screen.
pub(crate) fn saves_float(mode: &str, onboarded: bool, visible: bool, minimized: bool) -> bool {
    mode == "floating" && onboarded && visible && !minimized
}

/// A saved window `[x, y, w, h]` fitted whole inside a screen's work area `(x, y, w, h)`: no
/// bigger than it (at least 200 × 200 when it fits), and moved in when it hangs off an edge.
pub(crate) fn fit_rect(rect: [i64; 4], area: (i64, i64, i64, i64)) -> [i64; 4] {
    let (ax, ay, aw, ah) = area;
    // A screen that says it has no room (one being disconnected, a virtual one): nothing to fit
    // into, left as it was (the clamps below need a work area at least 1 px wide and high).
    if aw <= 0 || ah <= 0 {
        return rect;
    }
    let w = rect[2].max(200).min(aw.max(1));
    let h = rect[3].max(200).min(ah.max(1));
    let x = rect[0].clamp(ax, ax + aw - w);
    let y = rect[1].clamp(ay, ay + ah - h);
    [x, y, w, h]
}

fn contains(m: &tauri::Monitor, x: f64, y: f64) -> bool {
    let (pos, size) = (m.position(), m.size());
    x >= pos.x as f64 && x < (pos.x + size.width as i32) as f64 && y >= pos.y as f64 && y < (pos.y + size.height as i32) as f64
}

/// The monitor under the mouse (or the window's, or the primary one).
fn monitor_under_mouse(app: &AppHandle, win: &WebviewWindow) -> Option<tauri::Monitor> {
    let monitors = win.available_monitors().ok().unwrap_or_default();
    if let Ok(p) = app.cursor_position()
        && let Some(m) = monitors.iter().find(|m| contains(m, p.x, p.y)) {
            return Some(m.clone());
        }
    win.current_monitor().ok().flatten().or_else(|| win.primary_monitor().ok().flatten()).or_else(|| monitors.into_iter().next())
}

/// The screen the panel uses: the one chosen in Settings when it is connected, else the
/// one under the mouse.
fn target_monitor(app: &AppHandle, win: &WebviewWindow) -> Option<tauri::Monitor> {
    let chosen = prefs(app).monitor;
    if chosen != "mouse"
        && let Some(m) = win.available_monitors().ok().unwrap_or_default().into_iter().find(|m| monitor_key(m) == chosen) {
            return Some(m);
        }
    monitor_under_mouse(app, win)
}

pub fn list_monitors(app: &AppHandle) -> Vec<MonitorInfo> {
    let Some(win) = main_window(app) else { return vec![] };
    let primary = win.primary_monitor().ok().flatten().map(|m| monitor_key(&m));
    win.available_monitors()
        .ok()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let key = monitor_key(m);
            MonitorInfo {
                primary: primary.as_deref() == Some(key.as_str()),
                name: m.name().cloned().unwrap_or_default(),
                key,
                x: m.position().x,
                y: m.position().y,
                width: m.size().width,
                height: m.size().height,
                scale: m.scale_factor(),
            }
        })
        .collect()
}

/// Width of the peeking strip (note titles sliding out of the edge), logical pixels.
const PEEK_W: f64 = 248.0;

/// Peeking titles: a narrow strip at the edge with the notes' titles, like index cards
/// sliding out of a drawer. It doesn't take the keyboard focus.
pub fn show_peek(app: &AppHandle) {
    let Some(win) = main_window(app) else { return };
    let info = info(app);
    let Some(mon) = target_monitor(app, &win) else { return };
    let (x, y, w, h) = edge_rect(work_area(&mon), mon.scale_factor(), PEEK_W, info.side == "left");
    let _ = win.set_resizable(false);
    let _ = win.set_always_on_top(true);
    let _ = win.set_skip_taskbar(true);
    place(&win, &mon, x, y, w, h);
    app.state::<AppState>().peeking.store(true, Ordering::SeqCst);
    let _ = app.emit("peek-shown", info.side.clone());
    let _ = win.show();
}

/// Ends peeking without opening the panel.
pub fn end_peek(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.peeking.swap(false, Ordering::SeqCst)
        && let Some(win) = main_window(app)
    {
        let _ = win.hide();
    }
}

/// Places and shows the window for its mode, then tells the page (it plays the slide-in).
pub fn show(app: &AppHandle) {
    show_with(app, true);
}

/// Back after a full-screen app ended: shown where it was, without taking the keyboard from
/// what you are doing.
pub fn show_quietly(app: &AppHandle) {
    show_with(app, false);
}

/// Size of the first-launch setup window, logical pixels (the card inside adds MARGIN around).
const SETUP_SIZE: (f64, f64) = (760.0, 680.0);

fn onboarded(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    db::get_setting(&conn, "onboarded").is_some()
}

/// Size of the recovery screen's window, logical pixels (the card inside adds MARGIN around).
const RECOVERY_SIZE: (f64, f64) = (620.0, 640.0);
/// Its height when it offers four choices (damaged notes with a repaired copy: Use the repaired
/// notes, Try again, Put back, Start empty), so Quit shows without scrolling.
const RECOVERY_TALL: f64 = 720.0;

/// The notes didn't open at start (recovery.rs): the recovery screen, in a small ordinary window
/// in the middle of the main screen, as the first-launch setup. (Nothing of the board is ready:
/// no settings are read.) `focus`: it takes the keyboard (not at a start with Windows, where it
/// only shows, in the taskbar too).
pub fn show_recovery(app: &AppHandle, focus: bool) {
    let Some(win) = main_window(app) else { return };
    let mon = win.primary_monitor().ok().flatten().or_else(|| win.current_monitor().ok().flatten());
    if let Some(mon) = mon
        && !win.is_visible().unwrap_or(false)
    {
        let scale = mon.scale_factor();
        let area = mon.work_area();
        // (Never waits for a choice being made: then it keeps the size it has.)
        let four = app.try_state::<crate::recovery::Recovery>().is_some_and(|r| r.0.try_lock().is_ok_and(|s| matches!(&*s, crate::recovery::Stage::Waiting(p) if p.repaired)));
        let tall = if four { RECOVERY_TALL } else { RECOVERY_SIZE.1 };
        let w = (((RECOVERY_SIZE.0 + 2.0 * MARGIN) * scale).round() as u32).min(area.size.width);
        let h = (((tall + 2.0 * MARGIN) * scale).round() as u32).min(area.size.height);
        let x = area.position.x + (area.size.width as i32 - w as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - h as i32) / 2;
        let _ = win.set_resizable(false);
        let _ = win.set_always_on_top(false);
        let _ = win.set_skip_taskbar(false);
        place(&win, &mon, x, y, w, h);
    }
    let _ = win.show();
    let _ = win.unminimize();
    if focus {
        let _ = win.set_focus();
    }
}

/// First launch: a small, ordinary window in the middle of the main screen, where you set
/// everything up. Once you press Start, the page asks for `apply_window` and the panel goes
/// to its designed place.
fn show_setup(app: &AppHandle, win: &WebviewWindow) {
    let mon = win.primary_monitor().ok().flatten().or_else(|| win.current_monitor().ok().flatten());
    if let Some(mon) = mon {
        let scale = mon.scale_factor();
        let area = mon.work_area();
        let w = (((SETUP_SIZE.0 + 2.0 * MARGIN) * scale).round() as u32).min(area.size.width);
        let h = (((SETUP_SIZE.1 + 2.0 * MARGIN) * scale).round() as u32).min(area.size.height);
        let x = area.position.x + (area.size.width as i32 - w as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - h as i32) / 2;
        let _ = win.set_resizable(false);
        let _ = win.set_always_on_top(false);
        let _ = win.set_skip_taskbar(false);
        place(win, &mon, x, y, w, h);
    }
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
    let _ = app.emit("panel-shown", "setup");
}

fn show_with(app: &AppHandle, focus: bool) {
    let Some(win) = main_window(app) else { return };
    app.state::<AppState>().peeking.store(false, Ordering::SeqCst);
    if !onboarded(app) {
        show_setup(app, &win);
        return;
    }
    let info = info(app);
    let Some(mon) = target_monitor(app, &win) else {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    };
    let scale = mon.scale_factor();
    let area = mon.work_area();
    if info.mode == "docked" {
        let (x, y, w, h) = edge_rect(work_area(&mon), scale, width_on(app, &mon), info.side == "left");
        let _ = win.set_resizable(false);
        let _ = win.set_always_on_top(true);
        let _ = win.set_skip_taskbar(true);
        place(&win, &mon, x, y, w, h);
    } else {
        let saved = {
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            db::get_setting(&conn, "float_geom").and_then(|g| g.get(monitor_key(&mon)).cloned())
        };
        let _ = win.set_resizable(true);
        let _ = win.set_skip_taskbar(false);
        let _ = win.set_always_on_top(info.float_on_top);
        match saved.and_then(|g| serde_json::from_value::<[i64; 4]>(g).ok()) {
            Some(rect) => {
                // A screen that changed (resolution, layout) can leave it partly off-screen:
                // brought back whole inside the screen's work area.
                let a = (area.position.x as i64, area.position.y as i64, area.size.width as i64, area.size.height as i64);
                let [x, y, w, h] = fit_rect(rect, a);
                place(&win, &mon, x as i32, y as i32, w as u32, h as u32);
            }
            None => {
                let w = (((DEFAULT_FLOAT.0 + 2.0 * MARGIN) * scale) as u32).min(area.size.width);
                let h = (((DEFAULT_FLOAT.1 + 2.0 * MARGIN) * scale) as u32).min(area.size.height);
                let x = area.position.x + (area.size.width as i32 - w as i32) / 2;
                let y = area.position.y + (area.size.height as i32 - h as i32) / 2;
                place(&win, &mon, x, y, w, h);
            }
        }
    }
    if focus {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    } else {
        show_no_activate(&win);
    }
    let _ = app.emit("panel-shown", info.mode.clone());
    if info.mode == "docked" && app.state::<AppState>().pinned.load(Ordering::SeqCst) {
        crate::fullscreen::watch(app);
    }
}

fn show_no_activate(win: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        return;
    }
    let _ = win.show();
}

/// The shortcut and the tray click: open, bring to the front, or (when it is in front) ask the
/// page to close it with its slide-out.
pub fn toggle(app: &AppHandle) {
    let Some(win) = main_window(app) else { return };
    if app.state::<AppState>().peeking.load(Ordering::SeqCst) {
        show(app);
        return;
    }
    let visible = win.is_visible().unwrap_or(false) && !win.is_minimized().unwrap_or(false);
    let focused = win.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = app.emit("panel-hide-request", ());
    } else if visible {
        let _ = win.set_focus();
    } else {
        show(app);
    }
}

pub fn hide(app: &AppHandle) {
    app.state::<AppState>().peeking.store(false, Ordering::SeqCst);
    if let Some(win) = main_window(app) {
        save_float_geometry(app, &win);
        let _ = win.hide();
    }
}

/// Remembers where the floating window is, for the screen it is on. (Not the first-launch setup
/// window: it can be moved too, by its top strip, but its place and size aren't the panel's.)
pub fn save_float_geometry(app: &AppHandle, win: &WebviewWindow) {
    if !saves_float(&info(app).mode, onboarded(app), win.is_visible().unwrap_or(false), win.is_minimized().unwrap_or(false)) {
        return;
    }
    let (Ok(pos), Ok(size), Ok(Some(mon))) = (win.outer_position(), win.outer_size(), win.current_monitor()) else { return };
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    let mut geom = db::get_setting(&conn, "float_geom").filter(|g| g.is_object()).unwrap_or_else(|| serde_json::json!({}));
    geom[monitor_key(&mon)] = serde_json::json!([pos.x, pos.y, size.width, size.height]);
    let _ = db::set_setting(&conn, "float_geom", &geom);
}

// ---- The quick dump bar ----

/// Dobo's own page (the built page, or the development server in a development build): the
/// only thing its windows may show. A link or a dropped address never loads another site inside
/// Dobo's frameless, always-on-top window.
pub(crate) fn own_page(url: &tauri::Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        // (An empty page a web view may start from.)
        "about" => url.as_str() == "about:blank",
        "http" | "https" if url.host_str() == Some("tauri.localhost") => true,
        "http" if cfg!(debug_assertions) => matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")),
        _ => false,
    }
}

/// The main window, as tauri.conf.json describes it (`create: false` there: it is made here, so
/// WebView2 can be given the portable data folder's own folder before it starts).
pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }
    let Some(conf) = app.config().app.windows.iter().find(|w| w.label == "main").cloned() else { return Ok(()) };
    let mut b = tauri::WebviewWindowBuilder::from_config(app, &conf)?.on_navigation(own_page);
    if let Some(dir) = crate::paths::place().and_then(|p| p.webview.clone()) {
        b = b.data_directory(dir);
    }
    app_keys_only(&b.build()?);
    Ok(())
}

/// WebView2's own browser keys off: Ctrl+P (print), Ctrl+R and F5 (reload), Ctrl+F and F3 (find
/// on page), F12 and Ctrl+Shift+I (developer tools), the browser's Back / Forward keys. Dobo is an
/// app, not a page; the page still gets those keys (Ctrl+F is its Search). The editing keys stay
/// (cut, copy, paste, select all, undo in a field). Left on in a development build, for its
/// developer tools. (Tauri has no switch for it: set on the web view once it is made.)
fn app_keys_only(win: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        let _ = win.with_webview(|w| {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use windows::core::Interface;
            // SAFETY: WebView2's COM objects, used on the thread with_webview runs this on (the
            // window's own); each call is checked.
            unsafe {
                let Ok(core) = w.controller().CoreWebView2() else { return };
                let Ok(settings) = core.Settings() else { return };
                if let Ok(s) = settings.cast::<ICoreWebView2Settings3>() {
                    let _ = s.SetAreBrowserAcceleratorKeysEnabled(false);
                }
            }
        });
    }
    #[cfg(not(windows))]
    let _ = win;
}

pub fn create_dump(app: &AppHandle) -> tauri::Result<()> {
    let mut b = tauri::WebviewWindowBuilder::new(app, "dump", tauri::WebviewUrl::App("index.html".into())).on_navigation(own_page);
    // The same WebView2 folder as the main window (both share one browser process).
    if let Some(dir) = crate::paths::place().and_then(|p| p.webview.clone()) {
        b = b.data_directory(dir);
    }
    b
        .title(tr!("{} quick dump", "{} · dépôt rapide", crate::APP_NAME))
        .inner_size(DUMP_SIZE.0, DUMP_SIZE.1)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(true)
        .build()
        .map(|w| app_keys_only(&w))?;
    Ok(())
}

/// Opens the bar in the middle of the screen under the mouse, then reads what was in front
/// (a browser's page) and tells the bar.
pub fn show_dump(app: &AppHandle) {
    let Some(win) = app.get_webview_window("dump") else { return };
    if win.is_visible().unwrap_or(false) {
        let _ = win.set_focus();
        return;
    }
    let front = crate::browser::foreground();
    if let Some(mon) = monitor_under_mouse(app, &win) {
        let scale = mon.scale_factor();
        let area = mon.work_area();
        let w = (DUMP_SIZE.0 * scale) as u32;
        let h = (DUMP_SIZE.1 * scale) as u32;
        let x = area.position.x + (area.size.width as i32 - w as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - h as i32) / 2;
        place(&win, &mon, x, y, w, h);
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = app.emit_to("dump", "dump-shown", ());
    let app2 = app.clone();
    std::thread::spawn(move || {
        let src = crate::browser::read(front);
        let _ = app2.emit_to("dump", "dump-source", src);
    });
}

pub fn hide_dump(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("dump") {
        let _ = win.hide();
    }
}

// ---- Opening when the mouse reaches the screen edge (Settings, off by default) ----

/// The mouse, in physical pixels. On Windows straight from Windows (the watcher asks every
/// 120 ms: no trip through the main thread each time).
fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    #[cfg(windows)]
    {
        let _ = app;
        let mut p = windows::Win32::Foundation::POINT::default();
        // SAFETY: a plain call writing into a POINT of ours.
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut p) }.ok()?;
        Some((p.x as f64, p.y as f64))
    }
    #[cfg(not(windows))]
    {
        app.cursor_position().ok().map(|p| (p.x, p.y))
    }
}

/// The main window's handle, for the watcher's own quick questions (shown? where?).
#[cfg(windows)]
struct Handle(windows::Win32::Foundation::HWND);
// SAFETY: a window handle is only a number; IsWindowVisible and GetWindowRect may be asked from
// any thread.
#[cfg(windows)]
unsafe impl Send for Handle {}

/// Is the main window showing, and where (physical x, y, width, height)?
fn shown_rect(win: &WebviewWindow, #[cfg(windows)] handle: &Option<Handle>) -> (bool, Option<(i32, i32, u32, u32)>) {
    #[cfg(windows)]
    if let Some(Handle(h)) = handle {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsWindowVisible};
        let mut r = windows::Win32::Foundation::RECT::default();
        // SAFETY: plain questions about a window of ours, answered into a RECT of ours.
        let visible = unsafe { IsWindowVisible(*h) }.as_bool();
        let rect = unsafe { GetWindowRect(*h, &mut r) }.ok().map(|_| (r.left, r.top, (r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32));
        return (visible, rect);
    }
    let visible = win.is_visible().unwrap_or(false);
    let rect = match (win.outer_position(), win.outer_size()) {
        (Ok(p), Ok(s)) => Some((p.x, p.y, s.width, s.height)),
        _ => None,
    };
    (visible, rect)
}

/// A thread that sleeps while something is off and is woken when it is turned on, whichever comes
/// first: turned on before the thread is known, it never sleeps; woken before it sleeps, it
/// doesn't (a thread's wake-up is kept until it next sleeps).
pub(crate) struct Sleeper(std::sync::OnceLock<std::thread::Thread>);

impl Sleeper {
    pub(crate) const fn new() -> Self {
        Self(std::sync::OnceLock::new())
    }
    /// The thread that sleeps: this one (before it first looks at the setting).
    pub(crate) fn here(&self) {
        let _ = self.0.set(std::thread::current());
    }
    /// Turned on: the thread looks again.
    pub(crate) fn wake(&self) {
        if let Some(t) = self.0.get() {
            t.unpark();
        }
    }
    /// On the thread itself: asleep until woken, if `off` (then the caller looks again).
    pub(crate) fn sleep_if(&self, off: bool) -> bool {
        if off {
            std::thread::park();
        }
        off
    }
}

/// The watcher's thread: asleep while the setting is off, woken when it is turned on.
static EDGE_WATCH: Sleeper = Sleeper::new();

/// "Open at the screen edge" may have been turned on: the watcher looks again.
pub fn wake_edge_watch() {
    EDGE_WATCH.wake();
}

pub fn start_edge_watch(app: AppHandle) {
    std::thread::spawn(move || {
        // (Known before the setting is first looked at: turned on from now, it wakes us.)
        EDGE_WATCH.here();
        let mut held = 0u32;
        let mut armed = true;
        let mut away = 0u32;
        let mut monitors: Vec<tauri::Monitor> = Vec::new();
        let mut refreshed = 0u32;
        #[cfg(windows)]
        let mut handle: Option<Handle> = None;
        loop {
            let state = app.state::<AppState>();
            // Off: no waking up every 120 ms for nothing; asleep until it is turned on.
            if EDGE_WATCH.sleep_if(!state.edge_open.load(Ordering::Relaxed)) {
                held = 0;
                continue;
            }
            std::thread::sleep(std::time::Duration::from_millis(120));
            if !state.edge_open.load(Ordering::Relaxed) {
                continue;
            }
            let Some(win) = main_window(&app) else { continue };
            #[cfg(windows)]
            if handle.is_none() {
                handle = win.hwnd().ok().map(Handle);
            }
            #[cfg(windows)]
            let (visible, rect) = shown_rect(&win, &handle);
            #[cfg(not(windows))]
            let (visible, rect) = shown_rect(&win);
            // Peeking: staying against the edge opens the whole panel; leaving the strip ends it.
            if state.peeking.load(Ordering::SeqCst) {
                let (Some((px, py)), Some((x, y, w, h))) = (cursor(&app), rect) else { continue };
                let inside = px >= x as f64 && px < (x + w as i32) as f64 && py >= y as f64 && py < (y + h as i32) as f64;
                let side_left = prefs(&app).side == "left";
                let edge_x = if side_left { x as f64 } else { (x + w as i32) as f64 - 1.0 };
                if inside && (px - edge_x).abs() < 1.5 {
                    held += 1;
                    away = 0;
                    if held >= 8 {
                        held = 0;
                        armed = false;
                        show(&app);
                    }
                } else if inside {
                    held = 0;
                    away = 0;
                } else {
                    away += 1;
                    if away >= 3 {
                        away = 0;
                        held = 0;
                        end_peek(&app);
                    }
                }
                continue;
            }
            if visible {
                held = 0;
                armed = false;
                continue;
            }
            let p = prefs(&app);
            if p.mode == "floating" || !docked_possible() {
                continue;
            }
            // The screens, looked at again every few seconds (one may be plugged in).
            if refreshed == 0 || monitors.is_empty() {
                monitors = win.available_monitors().ok().unwrap_or_default();
            }
            refreshed = (refreshed + 1) % 40;
            let Some((px, py)) = cursor(&app) else { continue };
            let at_edge = monitors.iter().filter(|m| p.monitor == "mouse" || monitor_key(m) == p.monitor).any(|m| {
                let (pos, size) = (m.position(), m.size());
                let inside_y = py >= pos.y as f64 && py < (pos.y + size.height as i32) as f64;
                let edge_x = if p.side == "left" { pos.x as f64 } else { (pos.x + size.width as i32) as f64 - 1.0 };
                inside_y && (px - edge_x).abs() < 1.5
            });
            if !at_edge {
                held = 0;
                armed = true;
                continue;
            }
            held += 1;
            // About a quarter of a second at the edge, once per visit.
            if armed && held >= 2 {
                armed = false;
                if state.edge_peek.load(Ordering::Relaxed) {
                    held = 2;
                    show_peek(&app);
                } else {
                    show(&app);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{MARGIN, PREF_KEYS, Prefs, docked_width, edge_rect, fit_rect, other_scale, saved_width, saves_float};

    /// "Open at the screen edge" turned on before the watcher's thread exists, or after it fell
    /// asleep: either way it looks again at once.
    #[test]
    fn the_edge_watcher_wakes_whenever_the_setting_is_turned_on() {
        use super::Sleeper;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;
        let run = |on_before: bool| {
            let sleeper = Arc::new(Sleeper::new());
            let on = Arc::new(AtomicBool::new(on_before));
            // (Woken before the thread is known: nothing to wake, and it must not matter.)
            sleeper.wake();
            let (tx, rx) = std::sync::mpsc::channel();
            let (s2, on2) = (sleeper.clone(), on.clone());
            let t = std::thread::spawn(move || {
                s2.here();
                let _ = tx.send("known");
                while s2.sleep_if(!on2.load(Ordering::SeqCst)) {}
                let _ = tx.send("looking");
            });
            assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok("known"));
            if !on_before {
                assert!(rx.recv_timeout(Duration::from_millis(150)).is_err(), "asleep while off");
                on.store(true, Ordering::SeqCst);
                sleeper.wake();
            }
            assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok("looking"));
            t.join().unwrap();
        };
        run(true);
        run(false);
    }

    #[test]
    fn the_docked_panel_is_sized_for_the_screen_it_lands_on() {
        // 720 + 2 x 12 logical, on a 1920 x 1040 work area at 100 % and a 2880 x 1560 one at 150 %
        // to its right: the same card, 744 and 1116 px wide, against the edge.
        assert_eq!(edge_rect((0, 0, 1920, 1040), 1.0, 720.0, false), (1176, 0, 744, 1040));
        assert_eq!(edge_rect((1920, 0, 2880, 1560), 1.5, 720.0, false), (1920 + 2880 - 1116, 0, 1116, 1560));
        assert_eq!(edge_rect((1920, 0, 2880, 1560), 1.5, 720.0, true), (1920, 0, 1116, 1560));
        // 125 % on a screen at negative coordinates; never wider than the screen.
        assert_eq!(edge_rect((-1600, 0, 1600, 860), 1.25, 720.0, false), (-930, 0, 930, 860));
        assert_eq!(edge_rect((0, 0, 800, 600), 2.0, 720.0, false), (0, 0, 800, 600));
        assert_eq!(edge_rect((0, 0, 1920, 1040), 1.0, 248.0, true).2, (248.0 + 2.0 * MARGIN) as u32);
    }

    #[test]
    fn the_windows_show_only_dobos_own_page() {
        let ok = |u: &str| super::own_page(&tauri::Url::parse(u).unwrap());
        assert!(ok("http://tauri.localhost/index.html") && ok("https://tauri.localhost/") && ok("tauri://localhost/") && ok("about:blank"));
        assert!(!ok("https://example.com/") && !ok("http://tauri.localhost.example.com/") && !ok("file:///C:/x.html") && !ok("javascript:alert(1)"));
        // The development server, in development builds only.
        assert_eq!(ok("http://127.0.0.1:1430/"), cfg!(debug_assertions));
    }

    #[test]
    fn the_window_settings_kept_aside_match_the_notes() {
        let c = crate::db::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(Prefs::read(&c), Prefs { mode: "docked".into(), side: "right".into(), float_on_top: false, monitor: "mouse".into() });
        for (k, v) in [("mode", serde_json::json!("floating")), ("side", serde_json::json!("left")), ("float_on_top", serde_json::json!(true)), ("monitor", serde_json::json!("DISPLAY2@1920,0"))] {
            assert!(PREF_KEYS.contains(&k));
            crate::db::set_setting(&c, k, &v).unwrap();
        }
        assert_eq!(Prefs::read(&c), Prefs { mode: "floating".into(), side: "left".into(), float_on_top: true, monitor: "DISPLAY2@1920,0".into() });
    }

    #[test]
    fn a_move_to_a_screen_of_another_scale_goes_there_first() {
        assert!(other_scale(Some(1.0), 1.5));
        assert!(other_scale(Some(1.5), 1.25));
        assert!(!other_scale(Some(1.5), 1.5));
        assert!(!other_scale(None, 1.5), "unknown: one move, as before");
    }

    #[test]
    fn the_setup_window_moved_is_not_the_floating_place() {
        assert!(saves_float("floating", true, true, false));
        assert!(!saves_float("floating", false, true, false), "first launch: the setup window, moved by its top");
        assert!(!saves_float("docked", true, true, false));
        assert!(!saves_float("floating", true, false, false));
        assert!(!saves_float("floating", true, true, true));
    }

    #[test]
    fn docked_width_snaps_gently_and_stays_sensible() {
        // A 1920-wide screen: at most 70% (1344), capped at 1200.
        assert_eq!(docked_width(812.0, 1920.0, true), 812.0, "any width in between is fine");
        assert_eq!(docked_width(708.0, 1920.0, true), 720.0, "within 16 px of Medium: snaps");
        assert_eq!(docked_width(708.0, 1920.0, false), 708.0, "keys and Settings don't snap");
        assert_eq!(docked_width(895.0, 1920.0, true), 880.0);
        assert_eq!(docked_width(300.0, 1920.0, true), 420.0, "at least 420");
        assert_eq!(docked_width(5000.0, 1920.0, true), 1200.0, "at most 1200");
        assert_eq!(docked_width(900.0, 1280.0, true), 896.0, "at most 70% of a small screen");
        assert_eq!(docked_width(900.0, 500.0, false), 420.0, "a tiny screen still gets the minimum");
        assert_eq!(docked_width(f64::NAN, 1920.0, false), 720.0);
    }

    #[test]
    fn each_screen_remembers_its_width() {
        let w = serde_json::json!({ "DISPLAY1@0,0": 812.0, "DISPLAY2@1920,0": "bad" });
        assert_eq!(saved_width(Some(&w), "DISPLAY1@0,0", Some(560.0)), 812.0);
        assert_eq!(saved_width(Some(&w), "DISPLAY2@1920,0", Some(560.0)), 560.0, "a damaged value: the last width chosen");
        assert_eq!(saved_width(Some(&w), "NEW@0,1080", None), 720.0);
        assert_eq!(saved_width(None, "X", Some(880.0)), 880.0);
    }

    #[test]
    fn a_window_off_screen_comes_back_whole() {
        let area = (0, 0, 1920, 1040);
        // Saved on a screen that was to the right (now unplugged): pulled back in.
        assert_eq!(fit_rect([2500, 100, 600, 700], area), [1320, 100, 600, 700]);
        // Hanging off the top-left: moved in.
        assert_eq!(fit_rect([-300, -50, 600, 700], area), [0, 0, 600, 700]);
        // Bigger than the screen (a smaller resolution now): shrunk to fit.
        assert_eq!(fit_rect([10, 10, 2560, 1400], area), [0, 0, 1920, 1040]);
        // Already fine: unchanged.
        assert_eq!(fit_rect([100, 100, 600, 700], area), [100, 100, 600, 700]);
        // A screen reporting an empty work area: left as it is, never a crash.
        assert_eq!(fit_rect([100, 100, 600, 700], (0, 0, 0, 1040)), [100, 100, 600, 700]);
        assert_eq!(fit_rect([100, 100, 600, 700], (0, 0, 1920, 0)), [100, 100, 600, 700]);
        assert_eq!(fit_rect([100, 100, 600, 700], (0, 0, -5, -5)), [100, 100, 600, 700]);
        // A second screen at negative coordinates.
        assert_eq!(fit_rect([-100, 900, 400, 400], (-1280, 0, 1280, 1000)), [-400, 600, 400, 400]);
    }
}
