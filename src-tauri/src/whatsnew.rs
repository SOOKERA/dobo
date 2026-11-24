// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! What's new, shown once after an update (the window shows it: src/lib/whatsnew.ts,
//! WhatsNew.svelte). The notes' setting `whats_new_seen` holds the last version whose What's new
//! was shown (or that had nothing to show): after an update, the window shows every version newer
//! than it, up to the one running, then notes the running one.
//!
//! Dobo 1.1.x never noted it. So the first time the notes open without it, it is worked out here
//! from what they say, before this start writes its own version over it (`backup::OPENED_BY`,
//! the version that last opened them, noted since 1.1.0):
//! - never set up (no `onboarded`: the first-run window was never finished): a first install,
//!   nothing to show, so the running version;
//! - last opened by an older version: an update from that one (1.1.0 → shows 1.1.1 and 1.2.0);
//! - last opened by this version or a newer one: nothing new (a new install by Setup, which notes
//!   its own version, or notes made ready by this version: a backup put back, an empty board);
//! - set up, but no version noted (or one that can't be read): an update from a Dobo before 1.1,
//!   from which version can't be told: `UNKNOWN`, and the window shows the running version's part.

/// The setting: the last version whose What's new was shown.
pub const SEEN: &str = "whats_new_seen";
/// An update from a version that can't be told (before 1.1): only the running version's part.
pub const UNKNOWN: &str = "unknown";

/// What `whats_new_seen` starts as for notes that have none yet: `opened_by` is the version that
/// last opened them (before this start), `used_before` whether they were ever set up.
pub fn first_seen(opened_by: Option<&str>, used_before: bool, running: &str) -> String {
    if !used_before {
        return running.to_string();
    }
    let parse = |v: &str| semver::Version::parse(v.trim()).ok();
    match (opened_by.and_then(parse), parse(running)) {
        (Some(was), Some(now)) if was < now => was.to_string(),
        (Some(_), _) => running.to_string(),
        (None, _) => UNKNOWN.to_string(),
    }
}

/// Notes `whats_new_seen` when the notes have none yet (`opened_by`: read before this start
/// writes its own version). Never changes one already noted.
pub fn note_first(conn: &rusqlite::Connection, opened_by: Option<&str>, running: &str) {
    if crate::db::get_setting(conn, SEEN).is_some() {
        return;
    }
    let used_before = crate::db::get_setting(conn, "onboarded").is_some();
    let _ = crate::db::set_setting(conn, SEEN, &serde_json::json!(first_seen(opened_by, used_before, running)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A first install has nothing to show; an update shows what came since the version it had,
    /// or only its own part when that can't be told.
    #[test]
    fn first_install_and_updates_are_told_apart() {
        // Never set up: a first install, whatever the notes say.
        assert_eq!(first_seen(None, false, "1.2.0"), "1.2.0");
        assert_eq!(first_seen(Some("1.2.0"), false, "1.2.0"), "1.2.0", "a new install by Setup");
        assert_eq!(first_seen(Some("1.1.1"), false, "1.2.0"), "1.2.0", "set up never finished");
        // Updates from 1.1.x: what came since.
        assert_eq!(first_seen(Some("1.1.0"), true, "1.2.0"), "1.1.0");
        assert_eq!(first_seen(Some("1.1.1"), true, "1.2.0"), "1.1.1");
        assert_eq!(first_seen(Some(" 1.1.1 "), true, "1.2.0"), "1.1.1");
        // This version (notes made ready by it) or a newer one (gone back): nothing new.
        assert_eq!(first_seen(Some("1.2.0"), true, "1.2.0"), "1.2.0");
        assert_eq!(first_seen(Some("1.3.0"), true, "1.2.0"), "1.2.0");
        // From before 1.1 (no version noted), or one that can't be read: only this version's part.
        assert_eq!(first_seen(None, true, "1.2.0"), UNKNOWN);
        assert_eq!(first_seen(Some("one point oh"), true, "1.2.0"), UNKNOWN);
    }

    /// Noted once, from the notes themselves; never over a value already there.
    #[test]
    fn noted_only_when_missing() {
        let conn = crate::db::open(std::path::Path::new(":memory:")).unwrap();
        note_first(&conn, None, "1.2.0");
        assert_eq!(crate::db::get_setting(&conn, SEEN), Some(json!("1.2.0")), "a new board");
        crate::db::set_setting(&conn, SEEN, &json!("1.2.0")).unwrap();
        note_first(&conn, Some("1.0.0"), "9.0.0");
        assert_eq!(crate::db::get_setting(&conn, SEEN), Some(json!("1.2.0")), "kept");

        let conn = crate::db::open(std::path::Path::new(":memory:")).unwrap();
        crate::db::set_setting(&conn, "onboarded", &json!(true)).unwrap();
        note_first(&conn, Some("1.1.1"), "1.2.0");
        assert_eq!(crate::db::get_setting(&conn, SEEN), Some(json!("1.1.1")), "an update from 1.1.1");
    }
}
