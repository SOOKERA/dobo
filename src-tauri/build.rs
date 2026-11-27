// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
fn main() {
    terms_page();
    system_dlls_only();
    // Each of Dobo's own commands gets a permission (allow-<command>): a window may call only the
    // ones its capability lists (capabilities/: the board all of them, the quick dump bar its few).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS))).expect("Tauri's build step failed");
}

/// Every command lib.rs hands to Tauri (`generate_handler!`; a test checks they match, and that
/// the board's capability allows each one).
const COMMANDS: &[&str] = &[
    "load_all",
    "create_note",
    "update_note",
    "delete_note",
    "reorder_notes",
    "create_item",
    "update_item",
    "forget_done",
    "undo_tick",
    "delete_item",
    "set_setting",
    "language_now",
    "inbox_for",
    "drop_empty_inbox",
    "count_sticky_notes",
    "import_sticky_notes",
    "window_info",
    "hide_panel",
    "apply_window",
    "set_float_on_top",
    "list_monitors",
    "answer_ring",
    "page_ready",
    "tidy_answer",
    "rhythm_presets",
    "show_dump",
    "hide_dump",
    "dump_item",
    "fetch_preview",
    "set_item_link",
    "open_path",
    "open_url",
    "report_info",
    "report_send",
    "report_mail",
    "save_picture",
    "add_picture_file",
    "delete_picture",
    "make_thumb",
    "release_picture",
    "set_shortcut",
    "shortcuts_taken",
    "get_autostart",
    "set_autostart",
    "restore",
    "open_note",
    "end_peek",
    "save_note_image",
    "reveal",
    "backup_now",
    "backup_list",
    "backup_restore",
    "backup_status",
    "backup_folder",
    "data_info",
    "open_data_folder",
    "own_folders",
    "dark_mode_now",
    "export_look",
    "import_look",
    "pick_background_picture",
    "resize_docked",
    "set_pinned",
    "accept_terms",
    "quit_app",
    "update_status",
    "update_check",
    "update_restart",
    "recovery_state",
    "recovery_choose",
    "recovery_continue",
];

// The Markdown-to-text conversion, shared with Dobo itself (its tests are there).
#[path = "src/terms.rs"]
mod terms;

/// Dobo Setup's terms of use (bundle.licenseFile in tauri.conf.json; Tauri needs one, though the
/// window links the terms instead of showing a page): TERMS.md as plain text in gen/terms.txt
/// (gitignored), made fresh for every build so it never drifts from TERMS.md.
fn terms_page() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("..").join("TERMS.md");
    println!("cargo:rerun-if-changed={}", source.display());
    let md = std::fs::read_to_string(&source).unwrap_or_else(|_| panic!("{} is missing: Dobo needs its terms of use.", source.display()));
    if md.trim().is_empty() {
        panic!("TERMS.md is empty: Dobo needs its terms of use.");
    }
    if terms::version_of(&md).is_none() {
        panic!("TERMS.md has no \"Last updated <day> <month> <year>\" line: Dobo saves that date when the terms are accepted.");
    }
    let text = terms::plain_text(&md);
    let out = manifest.join("gen").join("terms.txt");
    // Written only when it changes (tauri dev would see a new file each time otherwise). A text
    // that isn't plain ASCII gets a UTF-8 mark at the start, so the installer reads it right.
    let bytes = if text.is_ascii() { text.into_bytes() } else { [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat() };
    if std::fs::read(&out).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::create_dir_all(out.parent().unwrap()).expect("make src-tauri/gen");
        std::fs::write(&out, bytes).expect("write src-tauri/gen/terms.txt for Dobo Setup");
    }
}

/// Windows looks for the DLLs a program names (dwmapi.dll, uxtheme.dll…) in the program's own
/// folder first. The portable Dobo.exe often sits in Downloads, where a file with one of those
/// names would be loaded and run at Dobo's next start. This linker setting
/// (LOAD_LIBRARY_SEARCH_SYSTEM32) makes Windows take them, and what they need in turn, from its
/// own System32 folder only; `main` does the same for DLLs asked for while Dobo runs. Checked by
/// scripts/release.ps1 on the built exe. (As Pako does.)
fn system_dlls_only() {
    if std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|e| e == "msvc") {
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
}
