// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Dobo goes online only after its terms of use are accepted (TERMS.md, see terms.rs), as Pako.
//!
//! Dobo Setup's window says "By installing, you accept the terms of use" and tells the app so
//! (`--install-choices --terms-accepted=1`, install.rs): an installed copy never asks. A portable
//! copy (or a silent install, which nobody looked at) shows the terms once at its first start,
//! with "Agree and continue" or "Quit Dobo". Until then nothing goes online: no update check,
//! no link preview. The notes, reminders and everything else on this computer work as always,
//! and everything starts the moment the terms are accepted, no restart.
//!
//! Asked once per copy of the notes: `terms_accepted` (the terms' date, "2026-11-21") is saved at
//! once and never asked again, also after updates or a newer TERMS.md (the terms say continued
//! use means acceptance).

use std::sync::Mutex;

/// People who used Dobo before it had terms (their notes have no `terms_accepted`) are asked
/// once too, with the same screen (as Pako).
pub const ASK_EXISTING_USERS: bool = true;

/// The saved value still has to accept the terms: only a missing (or blank) date asks.
pub fn needed_value(saved: Option<&serde_json::Value>, used_before: bool) -> bool {
    match saved.and_then(|v| v.as_str()).map(str::trim).filter(|d| !d.is_empty()) {
        None => ASK_EXISTING_USERS || !used_before,
        Some(_) => false,
    }
}

/// These notes still have to accept the terms.
pub fn needed(conn: &rusqlite::Connection) -> bool {
    let used_before = crate::db::get_setting(conn, "onboarded").is_some();
    needed_value(crate::db::get_setting(conn, "terms_accepted").as_ref(), used_before)
}

type Waiting = Vec<Box<dyn FnOnce() + Send>>;

/// Closed until the terms are accepted: what waits behind it starts when it opens (once).
pub struct Gate {
    state: Mutex<(bool, Waiting)>,
}

impl Gate {
    pub const fn new() -> Self {
        Self { state: Mutex::new((false, Vec::new())) }
    }

    pub fn is_open(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0
    }

    /// Runs `f` now if the gate is open, else the moment it opens.
    pub fn after(&self, f: impl FnOnce() + Send + 'static) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.0 {
            drop(st);
            f();
        } else {
            st.1.push(Box::new(f));
        }
    }

    /// Opens it and starts what waited, in order (outside the lock: what starts may ask).
    pub fn open(&self) {
        let waiting = {
            let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
            st.0 = true;
            std::mem::take(&mut st.1)
        };
        for f in waiting {
            f();
        }
    }
}

static GATE: Gate = Gate::new();

/// The terms are accepted (or didn't need to be): Dobo may go online.
pub fn ok() -> bool {
    GATE.is_open()
}

/// For a command that goes online: refused while the terms wait.
pub fn check() -> Result<(), String> {
    if ok() { Ok(()) } else { Err(tr!("Dobo needs you to accept its terms first.", "Dobo a besoin que vous acceptiez d’abord ses conditions.")) }
}

/// Starts `f` once the terms are accepted (now if they already are).
pub fn after(f: impl FnOnce() + Send + 'static) {
    GATE.after(f)
}

/// The terms were accepted: everything waiting starts.
pub fn open() {
    GATE.open()
}

/// At start, from the notes just opened.
pub fn start_with(conn: &rusqlite::Connection) {
    if !needed(conn) {
        open();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Asked only while the date is missing; a recorded date is never asked again.
    #[test]
    fn asked_only_when_the_date_is_missing() {
        assert!(needed_value(None, false), "a new copy is asked");
        assert_eq!(needed_value(None, true), ASK_EXISTING_USERS, "notes from before the terms");
        assert_eq!(needed_value(Some(&json!("  ")), true), ASK_EXISTING_USERS);
        assert!(!needed_value(Some(&json!("2026-11-21")), true));
        assert!(!needed_value(Some(&json!("2026-11-21")), false));
        let conn = crate::db::open(std::path::Path::new(":memory:")).unwrap();
        assert!(needed(&conn));
        crate::db::set_setting(&conn, "terms_accepted", &json!("2026-11-21")).unwrap();
        assert!(!needed(&conn));
    }

    /// What waits behind the gate (the update checks) starts once when it opens, and later ones
    /// at once.
    #[test]
    fn services_start_only_after_acceptance() {
        let gate = Gate::new();
        let started = Arc::new(AtomicU32::new(0));
        for _ in 0..3 {
            let s = started.clone();
            gate.after(move || {
                s.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert!(!gate.is_open());
        assert_eq!(started.load(Ordering::SeqCst), 0);
        gate.open();
        assert_eq!(started.load(Ordering::SeqCst), 3);
        gate.open();
        assert_eq!(started.load(Ordering::SeqCst), 3, "only once");
        let s = started.clone();
        gate.after(move || {
            s.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(started.load(Ordering::SeqCst), 4);
    }
}
