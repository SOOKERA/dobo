// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The mouse's Back / Forward buttons and the keyboard's Browser Back / Forward keys.
//!
//! WebView2 hands the page the side buttons as ordinary mouse buttons (and the page handles
//! them), but some mouse software sends them as WM_APPCOMMAND messages instead, which reach our
//! window, not the page. Those are caught here and passed on as a "nav" event ("back" /
//! "forward"); the page treats both ways the same (and ignores a double within a moment).

#[cfg(windows)]
pub fn hook(win: &tauri::WebviewWindow) {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let Ok(hwnd) = win.hwnd() else { return };
    // Kept while the window lives; given back (and the subclass taken off) when it is destroyed.
    let data = Box::into_raw(Box::new(win.clone())) as usize;
    unsafe {
        if !SetWindowSubclass(hwnd, Some(proc), SUBCLASS_ID, data).as_bool() {
            // Not hooked: nothing will free it otherwise.
            drop(Box::from_raw(data as *mut tauri::WebviewWindow));
        }
    }
}

#[cfg(windows)]
const SUBCLASS_ID: usize = 0x5A70_4E41;

#[cfg(not(windows))]
pub fn hook(_win: &tauri::WebviewWindow) {}

/// What an app command asks for, if it is Back or Forward (GET_APPCOMMAND_LPARAM).
pub fn command(lparam: isize) -> Option<&'static str> {
    match ((lparam >> 16) & 0x0FFF) as u32 {
        1 => Some("back"),    // APPCOMMAND_BROWSER_BACKWARD
        2 => Some("forward"), // APPCOMMAND_BROWSER_FORWARD
        _ => None,
    }
}

#[cfg(windows)]
unsafe extern "system" fn proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wp: windows::Win32::Foundation::WPARAM,
    lp: windows::Win32::Foundation::LPARAM,
    _id: usize,
    data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use tauri::Emitter;
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
    const WM_APPCOMMAND: u32 = 0x0319;
    const WM_NCDESTROY: u32 = 0x0082;
    if msg == WM_NCDESTROY {
        // The window is going: take the subclass off and free what `hook` kept for it.
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(proc), SUBCLASS_ID);
            drop(Box::from_raw(data as *mut tauri::WebviewWindow));
            return DefSubclassProc(hwnd, msg, wp, lp);
        }
    }
    if msg == WM_APPCOMMAND
        && let Some(dir) = command(lp.0)
    {
        // SAFETY: `data` is the boxed window set in `hook`, freed only on WM_NCDESTROY.
        let win = unsafe { &*(data as *const tauri::WebviewWindow) };
        let _ = win.emit_to(win.label(), "nav", dir);
        return LRESULT(1);
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

#[cfg(test)]
mod tests {
    use super::command;

    #[test]
    fn reads_back_and_forward() {
        // HIWORD = command | device bits (FAPPCOMMAND_MOUSE = 0x8000).
        assert_eq!(command((1 | 0x8000) << 16), Some("back"));
        assert_eq!(command((2 | 0x8000) << 16), Some("forward"));
        assert_eq!(command(1 << 16), Some("back"));
        assert_eq!(command(8 << 16), None); // volume mute
    }
}
