// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! System notifications with answer buttons.
//!
//! Windows: real toasts with buttons. Each button opens a `ztodo:` link, which Windows hands
//! to Dobo (the running one gets it through the single-instance plugin), so the buttons work
//! even later from the notification centre. Dobo registers its name/icon (AppUserModelID)
//! and the `ztodo:` link for the current user at each start.
//! Linux: notifications with actions where the desktop supports them. macOS: plain
//! notifications; the Now card in the panel has the buttons.

use std::sync::OnceLock;
use tauri::AppHandle;

// Both keep the app's working name (ZToDo), like the bundle identifier they follow (see
// paths.rs): the AppUserModelID is what Windows files the notifications already shown under,
// and reminders waiting in the notification centre carry `ztodo:` links. Nobody sees either
// (the name shown is APP_NAME).
pub const AUMID: &str = "com.sookera.ztodo";
pub const SCHEME: &str = "ztodo";
/// Notifications' group (one per app), as the notifications already shown were given.
#[cfg_attr(not(windows), allow(dead_code))]
const GROUP: &str = "ztodo";

/// A secret made once per install and kept in the app's data folder. Every `ztodo:` link a
/// notification carries includes it, so a web page can't tick or snooze tasks with a
/// made-up `ztodo:` link (it can't know the secret).
static TOKEN: OnceLock<String> = OnceLock::new();

fn random_hex() -> String {
    use std::hash::{BuildHasher, Hasher};
    // Each RandomState is keyed from the operating system's random source.
    (0..4u64)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(i);
            h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Loads the install's secret, or makes it the first time.
pub fn init_token(dir: &std::path::Path) {
    let file = dir.join("link-secret");
    let token = std::fs::read_to_string(&file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() >= 32 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or_else(|| {
            let t = random_hex();
            let _ = std::fs::write(&file, &t);
            t
        });
    let _ = TOKEN.set(token);
}

/// A notification's action with the secret added.
pub fn with_token(action: &str) -> String {
    match TOKEN.get() {
        Some(t) => format!("{action}{}t={t}", if action.contains('?') { '&' } else { '?' }),
        None => action.to_string(),
    }
}

/// Does a link carry this install's secret? (Same time whatever matches, so it can't be guessed
/// a letter at a time.)
pub fn token_ok(given: Option<&str>) -> bool {
    let (Some(t), Some(g)) = (TOKEN.get(), given) else { return false };
    t.len() == g.len() && t.bytes().zip(g.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

pub struct Notice {
    /// Replaces the previous notification with the same tag (one per item).
    pub tag: String,
    pub title: String,
    pub body: String,
    /// (label, action) — the action is the part after "ztodo:".
    pub buttons: Vec<(String, String)>,
    /// What a click on the notification itself does.
    pub open: String,
    /// Stays on screen until answered.
    pub persistent: bool,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegSetValueExW,
    };
    use windows::core::{HSTRING, PCWSTR};

    fn esc(s: &str) -> String {
        // Control characters are not allowed in XML: one in a task's text would make the whole
        // notification fail to show (a silently missed reminder).
        let s: String = s.chars().map(|c| if c.is_control() && !matches!(c, '\t' | '\n' | '\r') { ' ' } else { c }).collect();
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
    }

    fn xml(n: &Notice) -> String {
        let mut actions = String::new();
        for (label, action) in &n.buttons {
            actions.push_str(&format!(
                r#"<action content="{}" arguments="{}" activationType="protocol"/>"#,
                esc(label),
                esc(&format!("{SCHEME}:{}", with_token(action)))
            ));
        }
        let scenario = if n.persistent && !n.buttons.is_empty() { r#" scenario="reminder""# } else { "" };
        let audio = if n.persistent { r#"<audio src="ms-winsoundevent:Notification.Reminder"/>"# } else { "" };
        format!(
            r#"<toast launch="{}" activationType="protocol"{scenario}><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual><actions>{actions}</actions>{audio}</toast>"#,
            esc(&format!("{SCHEME}:{}", with_token(&n.open))),
            esc(&n.title),
            esc(&n.body)
        )
    }

    pub fn show(n: &Notice) {
        let xml = xml(n);
        let tag = n.tag.clone();
        std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let r = (|| -> windows::core::Result<()> {
                let doc = XmlDocument::new()?;
                doc.LoadXml(&HSTRING::from(xml.as_str()))?;
                let toast = ToastNotification::CreateToastNotification(&doc)?;
                toast.SetTag(&HSTRING::from(tag.as_str()))?;
                toast.SetGroup(&HSTRING::from(GROUP))?;
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?.Show(&toast)
            })();
            if let Err(e) = r {
                eprintln!("Couldn't show the notification: {e}");
            }
        });
    }

    pub fn clear(tag: &str) {
        let tag = tag.to_string();
        std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            if let Ok(h) = ToastNotificationManager::History() {
                let _ = h.RemoveGroupedTagWithId(&HSTRING::from(tag.as_str()), &HSTRING::from(GROUP), &HSTRING::from(AUMID));
            }
        });
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn set(path: &str, name: Option<&str>, value: &str) -> bool {
        unsafe {
            let mut key = HKEY::default();
            let p = wide(path);
            if RegCreateKeyExW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_err() {
                return false;
            }
            let n = name.map(wide);
            let v = wide(value);
            let bytes = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2);
            let ok = RegSetValueExW(key, n.as_ref().map(|n| PCWSTR(n.as_ptr())).unwrap_or(PCWSTR::null()), None, REG_SZ, Some(bytes)).is_ok();
            let _ = RegCloseKey(key);
            ok
        }
    }

    /// The name and icon shown on notifications, and the `ztodo:` link the buttons use.
    pub fn register(icon_png: &std::path::Path) {
        let Ok(exe) = std::env::current_exe() else { return };
        let exe = exe.to_string_lossy().to_string();
        let base = format!(r"Software\Classes\AppUserModelId\{AUMID}");
        set(&base, Some("DisplayName"), crate::APP_NAME);
        set(&base, Some("IconUri"), &icon_png.to_string_lossy());
        let proto = format!(r"Software\Classes\{SCHEME}");
        set(&proto, None, &format!("URL:{}", crate::APP_NAME));
        set(&proto, Some("URL Protocol"), "");
        set(&format!(r"{proto}\shell\open\command"), None, &format!("\"{exe}\" \"%1\""));
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn any_task_text_makes_a_valid_notification() {
            let n = Notice {
                tag: "i1".into(),
                title: "Pay \u{1}rent & \"bills\" <today>\u{0b}".into(),
                body: "Due now · Home".into(),
                buttons: vec![("Done".into(), "ring?item=1&a=done".into())],
                open: "open?item=1".into(),
                persistent: true,
            };
            let doc = XmlDocument::new().unwrap();
            doc.LoadXml(&HSTRING::from(xml(&n).as_str())).unwrap();
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn show(app: &AppHandle, n: &Notice) {
        let title = n.title.clone();
        let body = n.body.clone();
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let buttons = n.buttons.clone();
            let open = n.open.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                let mut note = notify_rust::Notification::new();
                note.appname(crate::APP_NAME).summary(&title).body(&body);
                note.action("default", &tr!("Open", "Ouvrir"));
                for (label, action) in &buttons {
                    note.action(action, label);
                }
                if let Ok(handle) = note.show() {
                    handle.wait_for_action(|a| {
                        if a == "__closed" {
                            return;
                        }
                        let act = if a == "default" { open.clone() } else { a.to_string() };
                        crate::handle_url(&app, &format!("{SCHEME}:{}", with_token(&act)));
                    });
                }
            });
        }
        #[cfg(target_os = "macos")]
        {
            let _ = app;
            std::thread::spawn(move || {
                let _ = notify_rust::Notification::new().appname(crate::APP_NAME).summary(&title).body(&body).show();
            });
        }
    }

    pub fn clear(_tag: &str) {}
}

pub fn show(app: &AppHandle, n: &Notice) {
    #[cfg(windows)]
    {
        let _ = app;
        imp::show(n);
    }
    #[cfg(not(windows))]
    imp::show(app, n);
}

pub fn clear(_app: &AppHandle, tag: &str) {
    imp::clear(tag);
}

#[cfg(windows)]
pub fn register(icon_png: &std::path::Path) {
    imp::register(icon_png);
}

#[cfg(not(windows))]
pub fn register(_icon_png: &std::path::Path) {}

#[cfg(test)]
mod token_tests {
    use super::*;

    #[test]
    fn links_carry_the_secret_and_others_are_refused() {
        let dir = std::env::temp_dir().join(format!("dobo-token-{}", crate::db::now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        init_token(&dir);
        let t = TOKEN.get().unwrap().clone();
        assert_eq!(t.len(), 64);
        assert_eq!(with_token("ring?item=3&a=done"), format!("ring?item=3&a=done&t={t}"));
        assert_eq!(with_token("lookback"), format!("lookback?t={t}"));
        assert!(token_ok(Some(&t)));
        assert!(!token_ok(None));
        assert!(!token_ok(Some("deadbeef")));
        let mut wrong = t.clone();
        wrong.replace_range(0..1, if t.starts_with('0') { "1" } else { "0" });
        assert!(!token_ok(Some(&wrong)));
        // Made once: the same secret next start.
        assert_eq!(std::fs::read_to_string(dir.join("link-secret")).unwrap(), t);
        assert_ne!(random_hex(), random_hex());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
