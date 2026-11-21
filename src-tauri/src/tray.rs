// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The tray icon: the Dobo note drawn for the taskbar it sits on (an ink edge on a light
//! taskbar, a paper edge on a dark one, as a paper note looks on each), at the size Windows shows
//! tray icons at (16 px at 100 %, 20 at 125 %, 24 at 150 %, 32 at 200 %: each size is its own
//! drawing on the pixel grid). It changes live when the taskbar's theme or the display scale
//! changes.

use std::sync::Mutex;
use tauri::AppHandle;
use tauri::image::Image;

/// (size, for a light taskbar, for a dark taskbar)
const ICONS: [(u32, &[u8], &[u8]); 4] = [
    (16, include_bytes!("../icons/tray/tray-16-light.png"), include_bytes!("../icons/tray/tray-16-dark.png")),
    (20, include_bytes!("../icons/tray/tray-20-light.png"), include_bytes!("../icons/tray/tray-20-dark.png")),
    (24, include_bytes!("../icons/tray/tray-24-light.png"), include_bytes!("../icons/tray/tray-24-dark.png")),
    (32, include_bytes!("../icons/tray/tray-32-light.png"), include_bytes!("../icons/tray/tray-32-dark.png")),
];

/// The drawing for a tray icon of `px` pixels: the smallest that is at least that big (Windows
/// then only shrinks it a little), else the biggest.
fn pick(px: u32, light_taskbar: bool) -> &'static [u8] {
    let (_, light, dark) = ICONS.iter().find(|(s, _, _)| *s >= px).unwrap_or(&ICONS[ICONS.len() - 1]);
    if light_taskbar { light } else { dark }
}

/// What the tray shows now (light taskbar, size), so a change that doesn't concern it does
/// nothing.
static SHOWN: Mutex<Option<(bool, u32)>> = Mutex::new(None);

/// The icon for the taskbar as it is now (the one the tray is made with).
pub fn icon_now() -> Option<Image<'static>> {
    let now = (taskbar_is_light(), tray_px());
    let img = Image::from_bytes(pick(now.1, now.0)).ok()?;
    *SHOWN.lock().unwrap() = Some(now);
    Some(img)
}

/// Puts the right icon in the tray, if the taskbar's theme or the scale changed since. (Called
/// when Windows' theme settings change, when the main window's scale or theme changes, and when
/// the mouse comes over the icon: the taskbar's display may change scale while the window is on
/// another one.)
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let now = (taskbar_is_light(), tray_px());
    {
        let mut shown = SHOWN.lock().unwrap();
        if *shown == Some(now) {
            return;
        }
        *shown = Some(now);
    }
    if let Ok(img) = Image::from_bytes(pick(now.1, now.0)) {
        let _ = tray.set_icon(Some(img));
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{CloseHandle, HWND, WAIT_OBJECT_0};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_DWORD, RegCloseKey, RegGetValueW,
        RegNotifyChangeKeyValue, RegOpenKeyExW,
    };
    use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};
    use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetSystemMetrics, SM_CXSMICON};
    use windows::core::w;

    const PERSONALIZE: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

    /// Windows' "Choose your default Windows mode" (the taskbar's theme; the apps' theme is a
    /// separate setting). Missing (Windows 10 before 1903): the taskbar was dark.
    pub fn taskbar_is_light() -> bool {
        let mut v: u32 = 0;
        let mut len = std::mem::size_of::<u32>() as u32;
        let r = unsafe {
            RegGetValueW(HKEY_CURRENT_USER, PERSONALIZE, w!("SystemUsesLightTheme"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut len))
        };
        r.is_ok() && v != 0
    }

    /// The size of a tray icon on the taskbar's display, in pixels.
    pub fn tray_px() -> u32 {
        unsafe {
            let bar = FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or(HWND::default());
            let dpi = if bar.is_invalid() { 0 } else { GetDpiForWindow(bar) };
            let px = if dpi > 0 { GetSystemMetricsForDpi(SM_CXSMICON, dpi) } else { GetSystemMetrics(SM_CXSMICON) };
            px.clamp(16, 64) as u32
        }
    }

    /// Calls `changed` each time Windows' theme settings change (the taskbar's light or dark
    /// among them), on one thread of its own that sleeps until then, for as long as the app runs.
    pub fn watch(changed: impl Fn() + Send + 'static) {
        watch_key(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", changed);
    }

    /// The same for any key of the current user's (the tests use one of their own).
    fn watch_key(path: &'static str, changed: impl Fn() + Send + 'static) {
        std::thread::spawn(move || unsafe {
            let path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(path.as_ptr()), None, KEY_NOTIFY | KEY_READ, &mut key).is_err() {
                return;
            }
            let Ok(event) = CreateEventW(None, false, false, None) else {
                let _ = RegCloseKey(key);
                return;
            };
            // Windows writes the apps' and the taskbar's theme one after the other: the watch is
            // set again before the values are read, so a write landing meanwhile wakes the next
            // wait instead of being missed.
            let arm = || RegNotifyChangeKeyValue(key, false, REG_NOTIFY_CHANGE_LAST_SET, Some(event), true).is_ok();
            if arm() {
                while WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0 && arm() {
                    changed();
                }
            }
            let _ = CloseHandle(event);
            let _ = RegCloseKey(key);
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::mpsc;
        use std::time::Duration;
        use windows::Win32::System::Registry::{REG_DWORD, RegDeleteKeyW, RegDeleteTreeW, RegSetKeyValueW};
        use windows::core::PCWSTR;

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        fn set(key: &str, v: u32) {
            let (k, n) = (wide(key), wide("SystemUsesLightTheme"));
            let r = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), REG_DWORD.0, Some((&v as *const u32).cast()), 4) };
            assert!(r.is_ok(), "{r:?}");
        }

        /// Every change is seen, the second one too (the watch is set again each time), on a key
        /// of the test's own (removed after).
        #[test]
        fn each_theme_change_is_seen() {
            const PARENT: &str = r"Software\DoboTests";
            let key: &'static str = Box::leak(format!(r"{PARENT}\tray-{}", crate::db::now_ms()).into_boxed_str());
            set(key, 1);
            let (tx, rx) = mpsc::channel();
            watch_key(key, move || {
                let _ = tx.send(());
            });
            std::thread::sleep(Duration::from_millis(200));
            for v in [0, 1, 0] {
                set(key, v);
                assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok(), "the change to {v} was seen");
                while rx.try_recv().is_ok() {}
            }
            let (k, parent) = (wide(key), wide(PARENT));
            unsafe {
                let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()));
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()));
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(parent.as_ptr()));
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn taskbar_is_light() -> bool {
        false
    }
    pub fn tray_px() -> u32 {
        32
    }
    pub fn watch(_changed: impl Fn() + Send + 'static) {}
}

use imp::{taskbar_is_light, tray_px};

/// Follows the taskbar's theme from now on.
pub fn watch(app: AppHandle) {
    imp::watch(move || refresh(&app));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_scale_gets_its_own_drawing_and_each_taskbar_its_edge() {
        let size = |b: &[u8]| Image::from_bytes(b).map(|i| (i.width(), i.height())).unwrap();
        for (px, want) in [(16, 16), (20, 20), (24, 24), (30, 32), (32, 32), (40, 32), (12, 16)] {
            assert_eq!(size(pick(px, true)), (want, want), "{px} px");
            assert_eq!(size(pick(px, false)), (want, want), "{px} px");
        }
        // Light taskbar: the ink edge; dark taskbar: the paper edge (the first drawn pixel down
        // the middle).
        let edge = |b: &[u8]| {
            let i = Image::from_bytes(b).unwrap();
            let (w, x) = (i.width() as usize, (i.width() / 2) as usize);
            let px = i.rgba().chunks(4).skip(x).step_by(w).find(|p| p[3] == 255).unwrap().to_vec();
            px[0]
        };
        for px in [16, 20, 24, 32] {
            assert!(edge(pick(px, true)) < 80, "ink edge at {px} px");
            assert!(edge(pick(px, false)) > 200, "paper edge at {px} px");
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_taskbar_is_read_without_failing() {
        let _ = taskbar_is_light();
        assert!((16..=64).contains(&tray_px()));
    }
}
