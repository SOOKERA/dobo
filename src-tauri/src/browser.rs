// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! What was in front when the quick dump bar opened: the window's title and, for a browser,
//! the page's address (read from the address bar through UI Automation, on Windows).

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
pub struct Source {
    /// The page's address, when the window was a browser.
    pub url: Option<String>,
    /// The page or window title.
    pub title: String,
    /// The program ("chrome.exe").
    pub app: String,
}

const BROWSERS: &[&str] = &[
    "chrome.exe", "msedge.exe", "vivaldi.exe", "brave.exe", "opera.exe", "chromium.exe", "thorium.exe", "arc.exe", "firefox.exe",
];

fn looks_like_url(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() || v.contains(char::is_whitespace) || !v.contains('.') {
        return None;
    }
    if v.starts_with("http://") || v.starts_with("https://") {
        Some(v.to_string())
    } else if v.contains("://") {
        None
    } else {
        Some(format!("https://{v}"))
    }
}

fn clean_title(title: &str, app: &str) -> String {
    let t = title.trim();
    if BROWSERS.contains(&app) {
        // "Page – Google Chrome", "Page - Personal - Microsoft Edge", "Page — Mozilla Firefox"
        for sep in [" - ", " — ", " – "] {
            if let Some(i) = t.rfind(sep) {
                let mut head = &t[..i];
                if app == "msedge.exe"
                    && let Some(j) = head.rfind(" - ") {
                        head = &head[..j];
                    }
                return head.trim().to_string();
            }
        }
    }
    t.to_string()
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    use windows::Win32::System::Variant::{VARIANT, VT_I4};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationValuePattern, TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_EditControlTypeId,
        UIA_ValuePatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    use windows::core::PWSTR;

    /// The window in front right now (taken before the dump bar shows).
    pub fn foreground() -> isize {
        unsafe { GetForegroundWindow().0 as isize }
    }

    fn exe_of(hwnd: HWND) -> String {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(h);
            if !ok {
                return String::new();
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit(['\\', '/']).next().unwrap_or("").to_ascii_lowercase()
        }
    }

    fn title_of(hwnd: HWND) -> String {
        unsafe {
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        }
    }

    fn address_bar(hwnd: HWND) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let root = uia.ElementFromHandle(hwnd).ok()?;
            let mut v = VARIANT::default();
            (*v.Anonymous.Anonymous).vt = VT_I4;
            (*v.Anonymous.Anonymous).Anonymous.lVal = UIA_EditControlTypeId.0;
            let cond = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &v).ok()?;
            let edit = root.FindFirst(TreeScope_Descendants, &cond).ok()?;
            let pattern: IUIAutomationValuePattern = edit.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
            let value = pattern.CurrentValue().ok()?.to_string();
            looks_like_url(&value)
        }
    }

    pub fn read(hwnd: isize) -> Source {
        let h = HWND(hwnd as *mut core::ffi::c_void);
        if h.0.is_null() {
            return Source::default();
        }
        let app = exe_of(h);
        let title = clean_title(&title_of(h), &app);
        let url = if BROWSERS.contains(&app.as_str()) { address_bar(h) } else { None };
        Source { url, title, app }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn foreground() -> isize {
        0
    }
    pub fn read(_hwnd: isize) -> Source {
        Source::default()
    }
}

pub use imp::{foreground, read};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_titles() {
        assert_eq!(looks_like_url("github.com/sookera").as_deref(), Some("https://github.com/sookera"));
        assert_eq!(looks_like_url("https://a.b/c").as_deref(), Some("https://a.b/c"));
        assert!(looks_like_url("search words here").is_none());
        assert_eq!(clean_title("Hangar Y files - Google Chrome", "chrome.exe"), "Hangar Y files");
        assert_eq!(clean_title("Docs - Personal - Microsoft Edge", "msedge.exe"), "Docs");
        assert_eq!(clean_title("ChanelVR - Unreal Editor", "unrealeditor.exe"), "ChanelVR - Unreal Editor");
    }
}
