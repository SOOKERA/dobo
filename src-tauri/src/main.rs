// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Before anything else: DLLs asked for by name while Dobo runs come from Windows' System32
/// folder, never from Dobo's own folder or the folder it was started in (a file with such a name
/// next to a portable Dobo.exe, in Downloads, would otherwise be loaded and run). The DLLs
/// Dobo.exe names itself are covered by the linker setting in build.rs. (As Pako does.)
#[cfg(windows)]
fn system_dlls_only() {
    use windows_sys::Win32::System::LibraryLoader::{LOAD_LIBRARY_SEARCH_SYSTEM32, LOAD_LIBRARY_SEARCH_USER_DIRS, SetDefaultDllDirectories};
    // SAFETY: a plain call with constant flags. If Windows refuses, the search stays as it was.
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_USER_DIRS) };
}

fn main() {
    #[cfg(windows)]
    system_dlls_only();
    // (args_os: an argument that isn't valid Unicode must not stop Dobo; `args()` would panic.)
    let args: Vec<String> = std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    // Started by the uninstaller: remove what this copy registered with Windows (and the notes
    // when asked), open no window.
    if args.first().is_some_and(|a| a == dobo_lib::install::UNINSTALL_CLEANUP) {
        std::process::exit(dobo_lib::install::uninstall_cleanup(&args));
    }
    // Started by Dobo Setup's window with its choices: do them, open no window.
    if args.first().is_some_and(|a| a == dobo_lib::install::INSTALL_CHOICES) {
        std::process::exit(dobo_lib::install::install_choices(&args));
    }
    // Release tool (signing new versions): never used by the app itself.
    if args.first().is_some_and(|a| a == "--release") {
        std::process::exit(dobo_lib::release::main(args));
    }
    dobo_lib::run()
}
