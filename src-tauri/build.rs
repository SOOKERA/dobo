// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
fn main() {
    terms_page();
    system_dlls_only();
    tauri_build::build()
}

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
