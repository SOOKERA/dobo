// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! What the app's working name (ZToDo) left on this computer, taken over by Dobo.
//!
//! "Start with Windows" was a "ZToDo" entry in the current user's Run key, pointing at
//! ZToDo.exe. Dobo's entry is named "Dobo" and points at Dobo.exe, so the old one would start a
//! program that is no longer there (or the old copy). At start, Dobo removes the old entry (and
//! the on/off Task Manager keeps for it) and, when it was on, turns its own on instead: the
//! person's choice carries over. (The data folder, the notifications' identity and their links
//! keep the old name on purpose: see paths.rs and notify.rs.)

// (Development runs never touch the real entry: only release builds and the tests use this.)
#![cfg_attr(debug_assertions, allow(dead_code))]

/// The working name's entry.
pub const OLD_NAME: &str = "ZToDo";

/// What the old "start with Windows" entry was.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OldStart {
    /// No old entry.
    None,
    /// It was there and on: Dobo's own should be turned on.
    On,
    /// It was there but turned off in Task Manager (Startup apps): Dobo's stays off.
    Off,
}

/// Task Manager's on/off for a Run entry: 12 bytes, the first one even when on (2 or 6), odd
/// when turned off (3 or 7). No value: on.
fn approved_on(value: Option<&[u8]>) -> bool {
    value.and_then(|v| v.first()).is_none_or(|b| b & 1 == 0)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW};
    use windows::core::PCWSTR;

    /// Where Windows keeps them for the current user.
    pub const WINDOWS_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn exists(key: &str, name: &str) -> bool {
        let (k, n) = (wide(key), wide(name));
        unsafe { RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, None, None, None).is_ok() }
    }

    fn bytes(key: &str, name: &str) -> Option<Vec<u8>> {
        let (k, n) = (wide(key), wide(name));
        let mut buf = [0u8; 64];
        let mut len = buf.len() as u32;
        let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), RRF_RT_REG_BINARY, None, Some(buf.as_mut_ptr().cast()), Some(&mut len)) };
        r.is_ok().then(|| buf[..len as usize].to_vec())
    }

    /// Removes a value; one that isn't there is fine.
    fn remove(key: &str, name: &str) -> bool {
        let (k, n) = (wide(key), wide(name));
        let r = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr())) };
        r.is_ok() || r == ERROR_FILE_NOT_FOUND
    }

    /// Takes the old entry away under `base` (in the app: Windows' own key; in the tests: a
    /// key of their own), and says what it was. When it was on, `turn_on` makes Dobo's own entry
    /// first: the old one goes only once that worked (else it stays, and the next start tries
    /// again), so the person's choice is never lost on the way.
    pub fn take_old_start(base: &str, turn_on: impl FnOnce() -> bool) -> OldStart {
        let run = format!(r"{base}\Run");
        let approved = format!(r"{base}\Explorer\StartupApproved\Run");
        let was = if exists(&run, OLD_NAME) {
            if approved_on(bytes(&approved, OLD_NAME).as_deref()) { OldStart::On } else { OldStart::Off }
        } else {
            OldStart::None
        };
        if was == OldStart::On && !turn_on() {
            return OldStart::None;
        }
        // Task Manager's line for it goes too (even alone: it would list a missing app), but only
        // with the entry itself: an entry left behind (no access) keeps its on/off.
        if remove(&run, OLD_NAME) {
            remove(&approved, OLD_NAME);
        }
        was
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::System::Registry::{REG_BINARY, REG_SZ, RegDeleteKeyW, RegDeleteTreeW, RegSetKeyValueW};

        fn set(key: &str, name: &str, kind: windows::Win32::System::Registry::REG_VALUE_TYPE, data: &[u8]) {
            let (k, n) = (wide(key), wide(name));
            let r = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), kind.0, Some(data.as_ptr().cast()), data.len() as u32) };
            assert!(r.is_ok(), "{r:?}");
        }

        fn sz(s: &str) -> Vec<u8> {
            wide(s).iter().flat_map(|c| c.to_le_bytes()).collect()
        }

        const PARENT: &str = r"Software\DoboTests";

        /// A key of the test's own, like Windows' CurrentVersion, removed after.
        struct Base(String);
        impl Base {
            fn new(tag: &str) -> Self {
                Base(format!(r"{PARENT}\rename-{tag}-{}-{:?}", crate::db::now_ms(), std::thread::current().id()))
            }
        }
        impl Drop for Base {
            fn drop(&mut self) {
                let (k, parent) = (wide(&self.0), wide(PARENT));
                unsafe {
                    let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()));
                    // (Goes only once empty: another test may still be using it.)
                    let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(parent.as_ptr()));
                }
            }
        }

        #[test]
        fn an_old_entry_that_was_on_is_removed_and_said_on() {
            let b = Base::new("on");
            let (run, approved) = (format!(r"{}\Run", b.0), format!(r"{}\Explorer\StartupApproved\Run", b.0));
            set(&run, OLD_NAME, REG_SZ, &sz(r"C:\Apps\ZToDo\ZToDo.exe --hidden"));
            set(&run, "Another app", REG_SZ, &sz(r"C:\Other\other.exe"));
            set(&approved, OLD_NAME, REG_BINARY, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let turned_on = std::cell::Cell::new(0);
            let on = || {
                turned_on.set(turned_on.get() + 1);
                true
            };
            assert_eq!(take_old_start(&b.0, on), OldStart::On);
            assert_eq!(turned_on.get(), 1, "Dobo's own entry is turned on");
            assert!(!exists(&run, OLD_NAME));
            assert!(bytes(&approved, OLD_NAME).is_none());
            assert!(exists(&run, "Another app"), "other apps' entries stay");
            // Done once: the next start finds nothing, and turns nothing on.
            assert_eq!(take_old_start(&b.0, || panic!("nothing to carry over")), OldStart::None);
        }

        #[test]
        fn an_old_entry_with_no_task_manager_line_was_on() {
            let b = Base::new("plain");
            let run = format!(r"{}\Run", b.0);
            set(&run, OLD_NAME, REG_SZ, &sz(r"D:\Packaged\ZToDo\ZToDo.exe --hidden"));
            assert_eq!(take_old_start(&b.0, || true), OldStart::On);
            assert!(!exists(&run, OLD_NAME));
        }

        /// Dobo's own entry couldn't be made: the old one stays as it was (on), for the next start.
        #[test]
        fn an_old_entry_stays_while_dobos_own_cant_be_made() {
            let b = Base::new("failed");
            let (run, approved) = (format!(r"{}\Run", b.0), format!(r"{}\Explorer\StartupApproved\Run", b.0));
            let on = [2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            set(&run, OLD_NAME, REG_SZ, &sz(r"C:\Apps\ZToDo\ZToDo.exe --hidden"));
            set(&approved, OLD_NAME, REG_BINARY, &on);
            assert_eq!(take_old_start(&b.0, || false), OldStart::None);
            assert!(exists(&run, OLD_NAME));
            assert_eq!(bytes(&approved, OLD_NAME).as_deref(), Some(&on[..]));
            // The next start, when it works: carried over then.
            assert_eq!(take_old_start(&b.0, || true), OldStart::On);
            assert!(!exists(&run, OLD_NAME));
            assert!(bytes(&approved, OLD_NAME).is_none());
        }

        #[test]
        fn an_old_entry_turned_off_in_task_manager_stays_off() {
            let b = Base::new("off");
            let (run, approved) = (format!(r"{}\Run", b.0), format!(r"{}\Explorer\StartupApproved\Run", b.0));
            set(&run, OLD_NAME, REG_SZ, &sz(r"C:\Apps\ZToDo\ZToDo.exe --hidden"));
            set(&approved, OLD_NAME, REG_BINARY, &[3, 0, 0, 0, 0x10, 0x20, 0x30, 0x40, 0, 0, 0, 0]);
            assert_eq!(take_old_start(&b.0, || panic!("an entry turned off is never turned on")), OldStart::Off);
            assert!(!exists(&run, OLD_NAME));
            assert!(bytes(&approved, OLD_NAME).is_none());
        }

        #[test]
        fn no_old_entry_changes_nothing_but_a_stray_task_manager_line() {
            let b = Base::new("none");
            let (run, approved) = (format!(r"{}\Run", b.0), format!(r"{}\Explorer\StartupApproved\Run", b.0));
            set(&run, "Dobo", REG_SZ, &sz(r"C:\Apps\Dobo\Dobo.exe --hidden"));
            set(&approved, OLD_NAME, REG_BINARY, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(take_old_start(&b.0, || panic!("nothing to carry over")), OldStart::None);
            assert!(exists(&run, "Dobo"), "Dobo's own entry is never touched");
            assert!(bytes(&approved, OLD_NAME).is_none());
            // No key at all (a fresh account): nothing, no error.
            assert_eq!(take_old_start(&format!(r"{}\Missing", b.0), || panic!("nothing to carry over")), OldStart::None);
        }
    }
}

/// Removes the old "start with Windows" entry and says what it was (Windows' own Run key). When
/// it was on, `turn_on` turns Dobo's own on first (true: done).
#[cfg(windows)]
pub fn take_old_start(turn_on: impl FnOnce() -> bool) -> OldStart {
    imp::take_old_start(imp::WINDOWS_RUN, turn_on)
}

#[cfg(not(windows))]
pub fn take_old_start(_turn_on: impl FnOnce() -> bool) -> OldStart {
    OldStart::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_managers_on_and_off() {
        assert!(approved_on(None));
        assert!(approved_on(Some(&[2, 0, 0])));
        assert!(approved_on(Some(&[6, 0, 0])));
        assert!(!approved_on(Some(&[3, 0, 0])));
        assert!(!approved_on(Some(&[7, 0, 0])));
        assert!(approved_on(Some(&[])));
    }
}
