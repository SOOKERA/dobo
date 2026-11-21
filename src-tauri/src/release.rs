// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Release tool (run from a terminal by scripts/release.ps1, never by users):
//!   Dobo.exe --release sign EXE VERSION [NOTES]   writes latest.json next to EXE
//!   Dobo.exe --release public                      prints the public key of the signing key
//!
//! The private key lives in %USERPROFILE%\.dobo-signing\dobo-signing.key (or DOBO_SIGNING_DIR):
//! never in the project. Its backup is in the Sookera vault (secrets/dobo/dobo-signing.key.age).
//! Losing it means Dobo can't take updates any more: users would have to install a new version
//! by hand.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

use crate::updates::{Manifest, signed_message};

fn key_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("DOBO_SIGNING_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".dobo-signing")
}

fn key_file() -> PathBuf {
    key_dir().join("dobo-signing.key")
}

pub fn main(args: Vec<String>) -> i32 {
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let r = match rest.as_slice() {
        ["public"] => signing_key().map(|k| B64.encode(k.verifying_key().to_bytes())),
        ["sign", exe, version] => sign(Path::new(exe), version, ""),
        ["sign", exe, version, notes] => sign(Path::new(exe), version, notes),
        _ => Err("Use: --release public | --release sign EXE VERSION [NOTES]".into()),
    };
    match r {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn signing_key() -> Result<SigningKey, String> {
    let text = std::fs::read_to_string(key_file()).map_err(|e| format!("No signing key at {} ({e}).", key_file().display()))?;
    let seed: [u8; 32] = B64
        .decode(text.trim_start_matches('\u{feff}').trim())
        .map_err(|_| "The signing key file is damaged.".to_string())?
        .try_into()
        .map_err(|_| "The signing key file is damaged.".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn sign(exe: &Path, version: &str, notes: &str) -> Result<String, String> {
    let key = signing_key()?;
    let bytes = std::fs::read(exe).map_err(|e| format!("Can't read {}: {e}", exe.display()))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let size = bytes.len() as u64;
    let signature = B64.encode(key.sign(signed_message(version, &sha256, size, notes).as_bytes()).to_bytes());
    let m = Manifest { version: version.into(), sha256, size, notes: notes.into(), signature };
    crate::updates::verify(&m).map_err(|e| format!("Signed, but the app's public key doesn't match this key: {e}"))?;
    let out = exe.with_file_name("latest.json");
    std::fs::write(&out, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
    Ok(format!("Wrote {}", out.display()))
}
