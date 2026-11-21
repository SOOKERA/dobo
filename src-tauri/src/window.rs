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

fn setting_str(app: &AppHandle, key: &str, default: &str) -> String {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    db::get_setting(&conn, key).and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| default.into())
}

pub fn info(app: &AppHandle) -> WindowInfo {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    let chosen = db::get_setting(&conn, "mode").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "docked".into());
    let side = db::get_setting(&conn, "side").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "right".into());
    let float_on_top = db::get_setting(&conn, "float_on_top").and_then(|v| v.as_bool()).unwrap_or(false);
    let possible = docked_possible();
    let mode = if chosen == "floating" || !possible { "floating" } else { "docked" }.to_string();
    WindowInfo { mode, chosen, docked_possible: possible, side, float_on_top, width: DEFAULT_DOCKED_WIDTH }
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

/// Size and place in one move (the screen-edge side stays put while the width changes).
fn place(win: &WebviewWindow, x: i32, y: i32, w: u32, h: u32) {
    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};
        let handle: HWND = hwnd;
        if unsafe { SetWindowPos(handle, None, x, y, w as i32, h as i32, SWP_NOZORDER | SWP_NOACTIVATE) }.is_ok() {
            return;
        }
    }
    let _ = win.set_size(PhysicalSize::new(w, h));
    let _ = win.set_position(PhysicalPosition::new(x, y));
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
    let area = mon.work_area();
    let w_logical = docked_width(width, area.size.width as f64 / scale, snap);
    if win.is_visible().unwrap_or(false) {
        let w = (((w_logical + 2.0 * MARGIN) * scale).round() as u32).min(area.size.width);
        let x = if info.side == "left" { area.position.x } else { area.position.x + area.size.width as i32 - w as i32 };
        place(&win, x, area.position.y, w, area.size.height);
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
    let chosen = setting_str(app, "monitor", "mouse");
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
    let scale = mon.scale_factor();
    let area = mon.work_area();
    let w = ((PEEK_W + 2.0 * MARGIN) * scale).round() as u32;
    let x = if info.side == "left" { area.position.x } else { area.position.x + area.size.width as i32 - w as i32 };
    let _ = win.set_resizable(false);
    let _ = win.set_always_on_top(true);
    let _ = win.set_skip_taskbar(true);
    let _ = win.set_size(PhysicalSize::new(w, area.size.height));
    let _ = win.set_position(PhysicalPosition::new(x, area.position.y));
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
        place(win, x, y, w, h);
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
        let width_logical = width_on(app, &mon);
        let w = ((width_logical + 2.0 * MARGIN) * scale).round() as u32;
        let w = w.min(area.size.width);
        let h = area.size.height;
        let x = if info.side == "left" { area.position.x } else { area.position.x + area.size.width as i32 - w as i32 };
        let _ = win.set_resizable(false);
        let _ = win.set_always_on_top(true);
        let _ = win.set_skip_taskbar(true);
        place(&win, x, area.position.y, w, h);
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
                let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
                let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
            }
            None => {
                let w = (((DEFAULT_FLOAT.0 + 2.0 * MARGIN) * scale) as u32).min(area.size.width);
                let h = (((DEFAULT_FLOAT.1 + 2.0 * MARGIN) * scale) as u32).min(area.size.height);
                let x = area.position.x + (area.size.width as i32 - w as i32) / 2;
                let y = area.position.y + (area.size.height as i32 - h as i32) / 2;
                let _ = win.set_size(PhysicalSize::new(w, h));
                let _ = win.set_position(PhysicalPosition::new(x, y));
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

/// The main window, as tauri.conf.json describes it (`create: false` there: it is made here, so
/// WebView2 can be given the portable data folder's own folder before it starts).
pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }
    let Some(conf) = app.config().app.windows.iter().find(|w| w.label == "main").cloned() else { return Ok(()) };
    let mut b = tauri::WebviewWindowBuilder::from_config(app, &conf)?;
    if let Some(dir) = crate::paths::place().and_then(|p| p.webview.clone()) {
        b = b.data_directory(dir);
    }
    b.build()?;
    Ok(())
}

pub fn create_dump(app: &AppHandle) -> tauri::Result<()> {
    let mut b = tauri::WebviewWindowBuilder::new(app, "dump", tauri::WebviewUrl::App("index.html".into()));
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
        .build()?;
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
        let _ = win.set_size(PhysicalSize::new(w, h));
        let _ = win.set_position(PhysicalPosition::new(x, y));
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

pub fn start_edge_watch(app: AppHandle) {
    std::thread::spawn(move || {
        let mut held = 0u32;
        let mut armed = true;
        let mut away = 0u32;
        let mut monitors: Vec<tauri::Monitor> = Vec::new();
        let mut refreshed = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let state = app.state::<AppState>();
            if !state.edge_open.load(Ordering::Relaxed) {
                held = 0;
                continue;
            }
            let Some(win) = main_window(&app) else { continue };
            // Peeking: staying against the edge opens the whole panel; leaving the strip ends it.
            if state.peeking.load(Ordering::SeqCst) {
                let (Ok(p), Ok(pos), Ok(size)) = (app.cursor_position(), win.outer_position(), win.outer_size()) else { continue };
                let inside = p.x >= pos.x as f64 && p.x < (pos.x + size.width as i32) as f64 && p.y >= pos.y as f64 && p.y < (pos.y + size.height as i32) as f64;
                let side_left = info(&app).side == "left";
                let edge_x = if side_left { pos.x as f64 } else { (pos.x + size.width as i32) as f64 - 1.0 };
                if inside && (p.x - edge_x).abs() < 1.5 {
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
            if win.is_visible().unwrap_or(false) {
                held = 0;
                armed = false;
                continue;
            }
            let info = info(&app);
            if info.mode != "docked" {
                continue;
            }
            if refreshed == 0 || monitors.is_empty() {
                monitors = win.available_monitors().ok().unwrap_or_default();
            }
            refreshed = (refreshed + 1) % 40;
            let Ok(p) = app.cursor_position() else { continue };
            let chosen = setting_str(&app, "monitor", "mouse");
            let at_edge = monitors.iter().filter(|m| chosen == "mouse" || monitor_key(m) == chosen).any(|m| {
                let (pos, size) = (m.position(), m.size());
                let inside_y = p.y >= pos.y as f64 && p.y < (pos.y + size.height as i32) as f64;
                let edge_x = if info.side == "left" { pos.x as f64 } else { (pos.x + size.width as i32) as f64 - 1.0 };
                inside_y && (p.x - edge_x).abs() < 1.5
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
    use super::{docked_width, fit_rect, saved_width, saves_float};

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
        // A second screen at negative coordinates.
        assert_eq!(fit_rect([-100, 900, 400, 400], (-1280, 0, 1280, 1000)), [-400, 600, 400, 400]);
    }
}
