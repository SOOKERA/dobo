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
    // An e-mail address (or anything with a sign-in before the host) is never a page's address.
    if !v.contains("://") && v.split(['/', '?', '#']).next().is_some_and(|host| host.contains('@')) {
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

/// How a browser names its address bar's own box, for the browsers known to: Chrome's, Edge's,
/// Brave's, Opera's (and Chromium's own builds') has the class "OmniboxViewViews", Firefox's the automation id
/// "urlbar-input". The others (Vivaldi, Arc…) name it otherwise: their first box, straight away
/// (asking for a name they don't use would walk the whole window for nothing first).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(not(windows), allow(dead_code))]
enum BarName {
    Class(&'static str),
    Id(&'static str),
}

#[cfg_attr(not(windows), allow(dead_code))]
fn bar_name(app: &str) -> Option<BarName> {
    match app {
        "chrome.exe" | "msedge.exe" | "brave.exe" | "opera.exe" | "chromium.exe" | "thorium.exe" => Some(BarName::Class("OmniboxViewViews")),
        "firefox.exe" => Some(BarName::Id("urlbar-input")),
        _ => None,
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    use windows::Win32::System::Variant::{VARIANT, VT_BSTR, VT_I4, VariantClear};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationValuePattern, TreeScope_Descendants, UIA_AutomationIdPropertyId, UIA_ClassNamePropertyId,
        UIA_ControlTypePropertyId, UIA_EditControlTypeId, UIA_ValuePatternId,
    };
    use windows::core::{BSTR, Interface};
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

    fn address_bar(hwnd: HWND, app: &str) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            with_time_limits(&uia);
            let root = uia.ElementFromHandle(hwnd).ok()?;
            let mut v = VARIANT::default();
            (*v.Anonymous.Anonymous).vt = VT_I4;
            (*v.Anonymous.Anonymous).Anonymous.lVal = UIA_EditControlTypeId.0;
            let box_ = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &v).ok()?;
            // The address bar itself, asked for by name (not the find bar's or a sidebar's box),
            // so the search stops there, early in the window, never walking into the page's own
            // fields. A browser that isn't known to name it so: its first box, as before.
            let named = bar_name(app).and_then(|name| {
                let (property, value) = match name {
                    BarName::Class(c) => (UIA_ClassNamePropertyId, c),
                    BarName::Id(i) => (UIA_AutomationIdPropertyId, i),
                };
                let mut value = text(value);
                let n = uia.CreatePropertyCondition(property, &value).and_then(|n| uia.CreateAndCondition(&box_, &n));
                let _ = VariantClear(&mut value);
                n.ok()
            });
            let edit = match named {
                Some(n) => root.FindFirst(TreeScope_Descendants, &n).ok().or_else(|| root.FindFirst(TreeScope_Descendants, &box_).ok())?,
                None => root.FindFirst(TreeScope_Descendants, &box_).ok()?,
            };
            // Being typed in: what it holds may be words not yet sent (a search, an e-mail
            // address), never taken as the page's address.
            if edit.CurrentHasKeyboardFocus().is_ok_and(|f| f.as_bool()) {
                return None;
            }
            let pattern: IUIAutomationValuePattern = edit.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
            let value = pattern.CurrentValue().ok()?.to_string();
            looks_like_url(&value)
        }
    }

    /// A text for a UI Automation condition (cleared with `VariantClear` after use).
    unsafe fn text(s: &str) -> VARIANT {
        let mut v = VARIANT::default();
        unsafe {
            (*v.Anonymous.Anonymous).vt = VT_BSTR;
            (*v.Anonymous.Anonymous).Anonymous.bstrVal = std::mem::ManuallyDrop::new(BSTR::from(s));
        }
        v
    }

    /// A browser that doesn't answer UI Automation quickly (busy, hung) never holds up the dump
    /// bar's thread for long.
    unsafe fn with_time_limits(uia: &IUIAutomation) {
        if let Ok(u2) = uia.cast::<IUIAutomation2>() {
            unsafe {
                let _ = u2.SetConnectionTimeout(1000);
                let _ = u2.SetTransactionTimeout(1000);
            }
        }
    }

    pub fn read(hwnd: isize) -> Source {
        let h = HWND(hwnd as *mut core::ffi::c_void);
        if h.0.is_null() {
            return Source::default();
        }
        let app = exe_of(h);
        let title = clean_title(&title_of(h), &app);
        let url = if BROWSERS.contains(&app.as_str()) { address_bar(h, &app) } else { None };
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
        // Typed, not a page: an e-mail address.
        assert!(looks_like_url("john.doe@corp.com").is_none());
        assert!(looks_like_url("me@site.fr/x").is_none());
        assert_eq!(looks_like_url("example.com/@user").as_deref(), Some("https://example.com/@user"));
        assert_eq!(clean_title("Hangar Y files - Google Chrome", "chrome.exe"), "Hangar Y files");
        assert_eq!(clean_title("Docs - Personal - Microsoft Edge", "msedge.exe"), "Docs");
        assert_eq!(clean_title("ChanelVR - Unreal Editor", "unrealeditor.exe"), "ChanelVR - Unreal Editor");
    }

    #[test]
    fn the_address_bar_is_asked_for_by_name_only_where_it_has_that_name() {
        for app in ["chrome.exe", "msedge.exe", "brave.exe", "opera.exe", "chromium.exe", "thorium.exe"] {
            assert_eq!(bar_name(app), Some(BarName::Class("OmniboxViewViews")), "{app}");
        }
        assert_eq!(bar_name("firefox.exe"), Some(BarName::Id("urlbar-input")));
        for app in ["vivaldi.exe", "arc.exe"] {
            assert_eq!(bar_name(app), None, "{app}: its first box, straight away");
        }
    }
}
