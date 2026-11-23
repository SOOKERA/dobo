// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Help › Report a problem (as Pako's). Nothing goes until the person clicks: the window shows
//! everything that goes with their words first.
//!
//! - **Send report** posts it to Sookera's report service (reports.sookera.com, a Cloudflare
//!   Worker on the studio's account), which files it as an issue in a private GitHub repository.
//!   Dobo holds no token: the service does. Only after the terms are accepted (consent.rs).
//! - **Send by email** opens a new message to support@sookera.com, and only that.
//!
//! What goes with the words: Dobo's version, Setup or Portable, the Windows version, Dobo's
//! language and look, and (unless turned off) the last lines of Dobo's own problem notes
//! (crash.log and startup-error.log, failure.rs). Never a note, a title, a file name, a link or a
//! setting. Those lines are cleaned (`clean`) when they are written and again when they are read.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Sookera's report service: what it takes and answers is `report_body` and `service_answer`.
const REPORT_URL: &str = "https://reports.sookera.com/v1/report";
pub const SUPPORT_EMAIL: &str = "support@sookera.com";
/// The newest problem lines a report carries (the service takes no more).
pub const REPORT_PROBLEMS: usize = 10;
const DESCRIPTION_MAX: usize = 5000;
/// One problem line, cleaned, at most (the service's limit).
const MESSAGE_MAX: usize = 500;
/// The service refuses anything bigger than 64 KB; Dobo keeps well under.
const REPORT_MAX: usize = 32 * 1024;
/// Dobo's two problem notes (failure.rs) and what each is about.
const NOTES: [(&str, &str); 2] = [("crash.log", "crash"), ("startup-error.log", "startup")];

// ---------- Cleaning: what could say who you are or what you wrote never goes ----------

/// The Windows user folder, as Dobo sees it.
fn home() -> Option<String> {
    std::env::var("USERPROFILE").ok().or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().into_owned())).filter(|h| h.trim().len() >= 3)
}

/// A problem line with nothing personal left (see `clean_with`).
pub fn clean(text: &str) -> String {
    clean_with(text, home().as_deref())
}

/// Text-direction marks, which could make a line read otherwise than it is stored.
fn is_bidi(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Cleans one problem line, the same way however many times (cleaning a clean line changes
/// nothing):
/// - the Windows user folder reads `%USERPROFILE%`, and the user's name on its own reads `…`;
/// - anything in quotes (`"…"`, `'…'`, `‘…’`, `“…”`, `«…»`, backticks) reads `…`, so an error that
///   quotes a note's words or a file's name never shows them; only Rust's own names in backticks
///   (`Option::unwrap()`, `None`) stay;
/// - folder and file names in paths read `…`, spaces and all, and a file keeps only its type
///   (`C:\…\….txt`); Dobo's own source files (`src\db.rs:120`) and the last part of another
///   program's (`…\app.rs:12`) stay, they say where the error was;
/// - links keep only their site; email addresses, long IDs and tokens go;
/// - control and text-direction characters become spaces; at most 500 characters, cut between
///   words.
pub fn clean_with(text: &str, home: Option<&str>) -> String {
    let mut s: String = text.chars().map(|c| if c.is_control() || is_bidi(c) { ' ' } else { c }).collect();
    if let Some(h) = home.map(|h| h.trim().trim_end_matches(['\\', '/'])).filter(|h| h.len() >= 3) {
        for form in [h.to_string(), h.replace('\\', "/"), h.replace('\\', "\\\\")] {
            s = replace_folder(&s, &form);
        }
        // The user's name on its own ("Zak Smith failed"), unless it is a word Windows uses for
        // everyone.
        let name = h.rsplit(['\\', '/']).next().unwrap_or("").trim();
        if name.chars().count() >= 3 && !matches!(name.to_ascii_lowercase().as_str(), "user" | "users" | "admin" | "administrator" | "owner" | "public" | "default" | "guest") {
            s = replace_name(&s, name);
        }
    }
    let s = quoted(&s);
    let out = words(&s);
    let out = out.trim();
    let cut = if out.chars().count() <= MESSAGE_MAX {
        out.to_string()
    } else {
        // Between words, never inside one (half a cleaned path would read otherwise when
        // cleaned again), with " …" to say so.
        let head: String = out.chars().take(MESSAGE_MAX - 2).collect();
        let head = head.rfind(' ').map_or(head.as_str(), |i| &head[..i]).trim_end();
        format!("{head} …")
    };
    if cut.trim().is_empty() { "…".into() } else { cut }
}

/// `folder` (ASCII letters in any case) reads %USERPROFILE%, where it is a whole folder (not the
/// start of a longer name: `C:\Users\Al` in `C:\Users\Alice`).
fn replace_folder(s: &str, folder: &str) -> String {
    let (hay, needle) = (s.to_ascii_lowercase(), folder.to_ascii_lowercase());
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut from = 0;
    while let Some(i) = hay[from..].find(&needle).map(|i| i + from) {
        let end = i + needle.len();
        let whole = hay[end..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'));
        if whole {
            out.push_str(&s[last..i]);
            out.push_str("%USERPROFILE%");
            last = end;
        }
        from = end;
    }
    out.push_str(&s[last..]);
    out
}

/// `name` as a whole word or words (ASCII letters in any case) reads `…`.
fn replace_name(s: &str, name: &str) -> String {
    let (hay, needle) = (s.to_ascii_lowercase(), name.to_ascii_lowercase());
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut from = 0;
    while let Some(i) = hay[from..].find(&needle).map(|i| i + from) {
        let end = i + needle.len();
        if !word(hay[..i].chars().next_back()) && !word(hay[end..].chars().next()) {
            out.push_str(&s[last..i]);
            out.push('…');
            last = end;
        }
        from = end;
    }
    out.push_str(&s[last..]);
    out
}

/// What's in quotes reads `…`: double quotes (a `\"` inside doesn't end them), ‘ ’, “ ”, « »,
/// single quotes around more than one character, and backticks. Backticks stay only while every
/// pair holds one of Rust's own names; from the first pair that doesn't, everything up to the
/// last backtick reads `…` (a backtick inside the quoted words can't split them).
fn quoted(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let n = cs.len();
    let mut out = String::with_capacity(s.len());
    let mask = |out: &mut String, open: char, close: Option<char>, empty: bool| {
        out.push(open);
        if !empty {
            out.push('…');
        }
        if let Some(c) = close {
            out.push(c);
        }
    };
    let mut i = 0;
    while i < n {
        let c = cs[i];
        match c {
            '"' => {
                // The closing quote: not one escaped by an odd number of backslashes.
                let close = (i + 1..n).find(|&j| cs[j] == '"' && cs[i + 1..j].iter().rev().take_while(|&&b| b == '\\').count() % 2 == 0);
                match close {
                    Some(j) => {
                        mask(&mut out, '"', Some('"'), j == i + 1);
                        i = j + 1;
                    }
                    None => {
                        mask(&mut out, '"', None, false);
                        i = n;
                    }
                }
            }
            '`' => {
                let next = (i + 1..n).find(|&j| cs[j] == '`');
                let inner: Option<String> = next.map(|j| cs[i + 1..j].iter().collect());
                match (next, inner) {
                    (Some(j), Some(inner)) if is_code(&inner) => {
                        out.push('`');
                        out.push_str(&inner);
                        out.push('`');
                        i = j + 1;
                    }
                    _ => {
                        let last = cs.iter().rposition(|&b| b == '`').filter(|&l| l > i);
                        match last {
                            Some(l) => {
                                mask(&mut out, '`', Some('`'), false);
                                i = l + 1;
                            }
                            None => {
                                mask(&mut out, '`', None, false);
                                i = n;
                            }
                        }
                    }
                }
            }
            '‘' | '“' | '«' => {
                let close = match c {
                    '‘' => '’',
                    '“' => '”',
                    _ => '»',
                };
                match (i + 1..n).find(|&j| cs[j] == close) {
                    Some(j) => {
                        let inner: String = cs[i + 1..j].iter().collect();
                        mask(&mut out, c, Some(close), inner.trim().is_empty() && inner.is_empty());
                        i = j + 1;
                    }
                    None => {
                        mask(&mut out, c, None, false);
                        i = n;
                    }
                }
            }
            // A single quote that opens (not an apostrophe inside a word), around more than one
            // character ('é' alone, as Rust quotes a character, stays).
            '\'' if i == 0 || !cs[i - 1].is_alphanumeric() => {
                let close = (i + 2..n).find(|&j| cs[j] == '\'' && (j + 1 == n || !cs[j + 1].is_alphanumeric()));
                match close {
                    Some(j) if j > i + 2 => {
                        mask(&mut out, '\'', Some('\''), false);
                        i = j + 1;
                    }
                    _ => {
                        out.push('\'');
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A name from Rust's own messages: `Option::unwrap()`, `Result::expect()`, `None`, `Err`.
/// Never a word of the person's (one word that isn't one of Rust's, or anything with a space).
fn is_code(s: &str) -> bool {
    let name = s.strip_suffix("()").unwrap_or(s);
    let ident = |p: &str| p.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let path = name.split("::").all(ident);
    path && s.len() <= 60 && (name.contains("::") || matches!(name, "None" | "Some" | "Ok" | "Err" | "unwrap" | "expect"))
}

/// Characters that end a path's last part as they end a quoted name or a list.
fn path_stop(c: char) -> bool {
    "\"`'()<>|,;[]{}«»“”‘’".contains(c)
}

/// The words of a line, cleaned. A path is followed through the spaces in its folder and file
/// names, up to its last part.
fn words(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let n = cs.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        if cs[i] == ' ' {
            out.push(' ');
            i += 1;
            continue;
        }
        let end = (i..n).find(|&j| cs[j] == ' ').unwrap_or(n);
        let w: String = cs[i..end].iter().collect();
        let lead = w.chars().take_while(|c| "([{<".contains(*c)).count();
        let core_start = i + lead;
        let core: String = cs[core_start..end].iter().collect();
        if !core.contains("://") && !is_dobo_source(core.trim_end_matches([')', ']', '}', '>', ',', ';', '.'])) && is_path(&core) {
            out.extend(&cs[i..core_start]);
            let (text, next) = path_from(&cs, core_start);
            out.push_str(&text);
            i = next;
            continue;
        }
        out.push_str(&word(&w));
        i = end;
    }
    out
}

/// One word of a line, cleaned (its brackets and end punctuation kept around it).
fn word(w: &str) -> String {
    if w.is_empty() {
        return String::new();
    }
    let start = w.find(|c: char| !"([{<".contains(c)).unwrap_or(w.len());
    let end = w.rfind(|c: char| !")]}>,;.:".contains(c)).map_or(start, |i| i + w[i..].chars().next().map_or(1, char::len_utf8)).max(start);
    let (lead, core, tail) = (&w[..start], &w[start..end], &w[end..]);
    format!("{lead}{}{tail}", core_word(core))
}

fn core_word(w: &str) -> String {
    if w.is_empty() || w == "…" || is_dobo_source(w) {
        return w.to_string();
    }
    if let Some(i) = w.find("://") {
        // A link: its site only, never who signed in (user@) or the rest of the address.
        let scheme = &w[..i];
        let host = w[i + 3..].split(['/', '?', '#', '\\']).next().unwrap_or("");
        let host = host.rsplit('@').next().unwrap_or("");
        return if scheme.chars().all(|c| c.is_ascii_alphanumeric()) && !host.is_empty() { format!("{scheme}://{host}") } else { "…".into() };
    }
    if is_email(w) {
        return "…".into();
    }
    // A link without its scheme ("example.com/Marie-Dupont"): its site only.
    if let Some((host, _)) = w.split_once('/')
        && is_host(host)
    {
        return host.to_string();
    }
    if is_long_id(w) {
        return "…".into();
    }
    // A file's name on its own ("list.txt"): its type only.
    if let Some((name, ext)) = w.rsplit_once('.')
        && name.chars().count() > 1
        && (1..=8).contains(&ext.len())
        && ext.bytes().all(|c| c.is_ascii_alphanumeric())
        && ext.bytes().any(|c| c.is_ascii_alphabetic())
        && !is_source(w)
    {
        return format!("….{}", ext.to_ascii_lowercase());
    }
    w.to_string()
}

fn is_email(w: &str) -> bool {
    match w.split_once('@') {
        Some((user, host)) => !user.is_empty() && host.contains('.') && !host.starts_with('.'),
        None => false,
    }
}

/// "example.com", "www.site.co.uk": a host name with a top-level domain of letters.
fn is_host(h: &str) -> bool {
    let parts: Vec<&str> = h.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        && parts.last().is_some_and(|t| t.len() >= 2 && t.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Where a path starts: `C:\…`, `\\server\…`, `%USERPROFILE%`, anything with a backslash, or a
/// forward-slash path (`/home/…`, `./x`, `a/b/c`; not `and/or`).
fn is_path(w: &str) -> bool {
    let b = w.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    drive || w.contains('\\') || w.starts_with("%USERPROFILE%") || (w.starts_with('/') && w.len() > 1) || w.starts_with("./") || w.matches('/').count() >= 2
}

/// "db.rs:120" (a source file with its line and column).
fn is_source(name: &str) -> bool {
    let file = name.split(':').next().unwrap_or("");
    let numbers = name.split(':').skip(1).all(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()));
    file.len() > 3 && file.ends_with(".rs") && file.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-') && numbers
}

/// Dobo's own source (`src\db.rs:120`, where an error happened): nothing personal.
fn is_dobo_source(w: &str) -> bool {
    let Some(rest) = w.strip_prefix("src\\").or_else(|| w.strip_prefix("src/")) else { return false };
    let parts: Vec<&str> = rest.split(['\\', '/']).collect();
    parts.iter().all(|p| !p.is_empty())
        && parts.last().is_some_and(|f| is_source(f))
        && parts[..parts.len() - 1].iter().all(|p| p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
}

/// The end of a path's last part in `seg`: just after its type (".txt", with a source file's
/// ":120:5"), else at the first space. The rest is ordinary words again.
fn file_end(seg: &[char]) -> usize {
    let n = seg.len();
    for (k, &c) in seg.iter().enumerate() {
        if c != '.' || k == 0 || seg[k - 1] == ' ' {
            continue;
        }
        let mut e = k + 1;
        while e < n && seg[e].is_ascii_alphanumeric() {
            e += 1;
        }
        let ext = e - k - 1;
        let ends = e == n || matches!(seg[e], ' ' | ':' | '.');
        if (1..=8).contains(&ext) && seg[k + 1..e].iter().any(|c| c.is_ascii_alphabetic()) && ends {
            // A source file keeps its line and column (":120:5").
            let mut f = e;
            while f + 1 < n && seg[f] == ':' && seg[f + 1].is_ascii_digit() {
                f += 1;
                while f < n && seg[f].is_ascii_digit() {
                    f += 1;
                }
            }
            return f;
        }
    }
    seg.iter().position(|&c| c == ' ').unwrap_or(n)
}

/// Another path starts at `k`: `C:\`, `\\server`, `%USERPROFILE%`.
fn path_starts_at(cs: &[char], k: usize) -> bool {
    let at = |i: usize, c: char| cs.get(k + i) == Some(&c);
    let drive = cs.get(k).is_some_and(|c| c.is_ascii_alphabetic()) && at(1, ':') && (at(2, '\\') || at(2, '/'));
    drive || (at(0, '\\') && at(1, '\\')) || cs.get(k..k + 13).is_some_and(|w| w.iter().collect::<String>() == "%USERPROFILE%")
}

/// A path from `start`: its folders read `…` (spaces and all), its last part keeps only its type.
/// Returns the cleaned path and where the line goes on after it.
fn path_from(cs: &[char], start: usize) -> (String, usize) {
    let n = cs.len();
    let sep = |c: char| c == '\\' || c == '/';
    let rest: String = cs[start..].iter().take(16).collect();
    let (prefix, mut j) = if rest.starts_with("%USERPROFILE%") {
        ("%USERPROFILE%".to_string(), start + 13)
    } else if rest.starts_with("\\\\") {
        ("\\\\".to_string(), start + 2)
    } else if cs.len() > start + 1 && cs[start].is_ascii_alphabetic() && cs[start + 1] == ':' {
        (format!("{}:", cs[start]), start + 2)
    } else {
        (String::new(), start)
    };
    let mut folders = 0;
    loop {
        while j < n && sep(cs[j]) {
            j += 1;
        }
        let seg_start = j;
        // (A space before another path ends this one: "C:\a\x.txt and D:\b".)
        while j < n && !sep(cs[j]) && !path_stop(cs[j]) && !(cs[j] == ' ' && path_starts_at(cs, j + 1)) {
            j += 1;
        }
        if j < n && sep(cs[j]) {
            // A folder (or a server), whatever its spaces.
            if j > seg_start {
                folders += 1;
            }
            continue;
        }
        let seg = &cs[seg_start..j];
        let end = file_end(seg);
        let file: String = seg[..end].iter().collect();
        let last = if file.is_empty() {
            String::new()
        } else if is_source(&file) {
            file
        } else {
            let tail = file.rsplit_once('.').map(|(_, e)| e).filter(|e| (1..=8).contains(&e.len()) && e.chars().all(|c| c.is_ascii_alphanumeric()) && e.chars().any(|c| c.is_ascii_alphabetic()));
            match tail {
                Some(e) if file.chars().count() > e.len() + 1 => format!("….{}", e.to_ascii_lowercase()),
                _ => "…".into(),
            }
        };
        let mut out = prefix.clone();
        let unc = prefix == "\\\\";
        if !prefix.is_empty() && !unc && (folders > 0 || !last.is_empty()) {
            out.push('\\');
        }
        if folders > 0 {
            out.push_str("…\\");
        }
        if last.is_empty() && folders > 0 {
            out.pop();
        }
        out.push_str(&last);
        if out.is_empty() {
            out.push('…');
        }
        return (out, seg_start + end);
    }
}

/// A long run of letters and digits (an ID, a key, a hash): 24 or more, with both; or a token in
/// parts ("eyJhbGci….eyJzdWIi….abc"), any part of 16 or more.
fn is_long_id(w: &str) -> bool {
    let id = |p: &str, min: usize| {
        p.chars().count() >= min
            && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '=' || c == '+')
            && p.chars().any(|c| c.is_ascii_digit())
            && p.chars().any(|c| c.is_ascii_alphabetic())
    };
    id(w, 24) || (w.contains('.') && w.split('.').any(|p| id(p, 16)))
}

// ---------- What a report carries ----------

/// One problem as the window shows it and the service takes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Problem {
    /// "2026-11-23T15:37:00+03:00": this computer's time then.
    pub time: String,
    /// "crash" (Dobo stopped on an error) or "startup" (Dobo couldn't start).
    pub area: String,
    pub message: String,
    /// The Dobo version that had it.
    pub version: String,
}

#[derive(Serialize, Debug)]
pub struct ReportInfo {
    pub version: String,
    pub installed: bool,
    pub windows: String,
    /// The newest first, at most REPORT_PROBLEMS.
    pub problems: Vec<Problem>,
    /// How many lines the notes hold in all.
    pub total: usize,
}

/// "1.1.1", "2.0.0-beta.1", "1.1.0+build5": the service's own rule.
fn is_version(v: &str) -> bool {
    if v.is_empty() || v.len() > 32 {
        return false;
    }
    let (main, build) = match v.split_once('+') {
        Some((m, b)) => (m, Some(b)),
        None => (v, None),
    };
    let tag_ok = |t: &str| !t.is_empty() && t.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.');
    if build.is_some_and(|b| !tag_ok(b)) {
        return false;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    let mut it = main.splitn(3, '.');
    let (Some(a), Some(b), Some(rest)) = (it.next(), it.next(), it.next()) else { return false };
    let split = rest.find(['-', '.']).unwrap_or(rest.len());
    let (c, tag) = rest.split_at(split);
    digits(a) && digits(b) && digits(c) && (tag.is_empty() || tag_ok(&tag[1..]))
}

/// One line of a note ("2026-11-23_153700 Dobo 1.1.0: what happened"), read back.
fn parse_line(line: &str, area: &str, home: Option<&str>) -> Option<(i64, Problem)> {
    let (stamp, rest) = line.trim().split_once(' ')?;
    // "2026-11-23T15:37:00+03:00" (with its offset then), or the older "2026-11-23_153700" (this
    // PC's clock now says which offset).
    let (ms, time) = match stamp.parse::<jiff::Timestamp>() {
        Ok(t) if stamp.len() == 25 && stamp.as_bytes().get(10) == Some(&b'T') => (t.as_millisecond(), stamp.to_string()),
        _ => {
            let ms = crate::clock::from_stamp(stamp)?;
            (ms, crate::clock::iso_local(ms)?)
        }
    };
    let rest = rest.strip_prefix("Dobo ")?;
    let (version, message) = rest.split_once(": ")?;
    if !is_version(version) {
        return None;
    }
    Some((ms, Problem { time, area: area.into(), message: clean_with(message, home), version: version.into() }))
}

/// The lines of Dobo's notes in these folders, the newest first.
fn read_notes(dirs: &[PathBuf], home: Option<&str>) -> Vec<Problem> {
    let mut all: Vec<(i64, Problem)> = Vec::new();
    for dir in dirs {
        for (file, area) in NOTES {
            // The notes are kept small (failure.rs): a bigger file isn't one of them.
            let Ok(meta) = std::fs::metadata(dir.join(file)) else { continue };
            if !meta.is_file() || meta.len() > 256 * 1024 {
                continue;
            }
            let Ok(bytes) = std::fs::read(dir.join(file)) else { continue };
            for line in String::from_utf8_lossy(&bytes).lines() {
                if let Some(p) = parse_line(line, area, home) {
                    all.push(p);
                }
            }
        }
    }
    all.sort_by_key(|p| std::cmp::Reverse(p.0));
    // The same line twice (in both folders, say), wherever it is: once.
    let mut seen = std::collections::HashSet::new();
    all.retain(|(ms, p)| seen.insert((*ms, p.area.clone(), p.version.clone(), p.message.clone())));
    all.into_iter().map(|(_, p)| p).collect()
}

/// Where failure.rs writes its notes: the data folder, or the temp folder when it couldn't.
fn note_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = crate::paths::place().map(|p| p.dir.clone()).or_else(crate::paths::usual_dir).into_iter().collect();
    dirs.push(std::env::temp_dir().join("Dobo"));
    dirs
}

/// What a report would carry (read off the window's thread).
pub fn report_info() -> ReportInfo {
    let all = read_notes(&note_dirs(), home().as_deref());
    ReportInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        installed: crate::paths::is_installed(),
        windows: windows_version(),
        total: all.len(),
        problems: all.into_iter().take(REPORT_PROBLEMS).collect(),
    }
}

// ---------- Send report ----------

/// Why a report didn't go (the window says what to do instead).
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SendFail {
    /// No connection, or the service didn't answer in time.
    Offline,
    /// The service is down or answered something unexpected.
    Server,
    /// Too many reports from this connection: try again after this many seconds.
    #[serde(rename_all = "camelCase")]
    Busy { retry_after: u64 },
    /// The report itself can't go (too long, empty…): `message` says why.
    Refused { message: String },
}

fn too_long() -> SendFail {
    SendFail::Refused { message: tr!("The report is too long. Shorten what happened, or turn off Include recent problems.", "Le rapport est trop long. Raccourcissez ce qui s’est passé, ou désactivez Joindre les problèmes récents.") }
}

/// The JSON the service gets: what the window showed, checked again here (each line is cleaned
/// again, in case it didn't come from the notes; cleaning twice changes nothing).
pub fn report_body(description: &str, look: &str, language: &str, problems: &[Problem], version: &str, installed: bool, windows: &str) -> Result<String, SendFail> {
    let refused = |m: String| SendFail::Refused { message: m };
    let description = description.trim();
    // (Only spaces or invisible characters is nothing said: the service's own rule.)
    if !description.chars().any(|c| !c.is_whitespace() && !matches!(c, '\u{200B}'..='\u{200F}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}')) {
        return Err(refused(tr!("Say what happened first.", "Dites d’abord ce qui s’est passé.")));
    }
    if description.chars().count() > DESCRIPTION_MAX {
        return Err(refused(tr!("What happened is too long: keep it under 5,000 characters.", "Ce qui s’est passé est trop long : restez sous 5 000 caractères.")));
    }
    if !matches!(look, "light" | "dark") || !matches!(language, "en" | "fr") || problems.len() > REPORT_PROBLEMS {
        return Err(refused(tr!("Dobo couldn’t put this report together. Try again.", "Dobo n’a pas pu préparer ce rapport. Réessayez.")));
    }
    let home = home();
    let mut list = Vec::with_capacity(problems.len());
    for p in problems {
        if !is_version(&p.version) || !matches!(p.area.as_str(), "crash" | "startup") || p.time.is_empty() || p.time.len() > 40 {
            return Err(refused(tr!(
                "A problem in the list can’t be sent. Turn off Include recent problems, then try again.",
                "Un problème de la liste ne peut pas être envoyé. Désactivez Joindre les problèmes récents, puis réessayez."
            )));
        }
        list.push(serde_json::json!({ "time": p.time, "area": p.area, "message": clean_with(&p.message, home.as_deref()), "version": p.version }));
    }
    let body = serde_json::json!({
        "app": "dobo",
        "version": version,
        "install": if installed { "setup" } else { "portable" },
        "windows": windows.chars().take(120).collect::<String>(),
        "language": language,
        "look": look,
        "description": description,
        "problems": list,
    })
    .to_string();
    if body.len() > REPORT_MAX {
        return Err(too_long());
    }
    Ok(body)
}

/// What the service's answer means: the report's reference, or why it didn't go.
pub fn service_answer(status: u16, retry_header: Option<u64>, body: &str) -> Result<String, SendFail> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    match status {
        200 | 201 => {
            let r = match &json["ref"] {
                serde_json::Value::String(s) => s.trim_start_matches('#').to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            // A short, plain reference (it is shown as it is).
            let plain = !r.is_empty() && r.len() <= 40 && r.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c));
            if json["ok"] == serde_json::Value::Bool(true) && plain { Ok(r) } else { Err(SendFail::Server) }
        }
        429 => Err(SendFail::Busy { retry_after: json["retryAfter"].as_u64().or(retry_header).unwrap_or(3600).clamp(30, 86_400) }),
        413 => Err(too_long()),
        400 => Err(SendFail::Refused { message: tr!("Sookera’s report service didn’t take this report. Use Send by email instead.", "Le service de rapports de Sookera n’a pas accepté ce rapport. Utilisez plutôt Envoyer par e-mail.") }),
        _ => Err(SendFail::Server),
    }
}

/// Sends the report as the window shows it. Returns its reference ("42").
pub fn send(description: &str, look: &str, language: &str, problems: &[Problem]) -> Result<String, SendFail> {
    let version = env!("CARGO_PKG_VERSION");
    let body = report_body(description, look, language, problems, version, crate::paths::is_installed(), &windows_version())?;
    let http = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        // The report goes to this address only: a redirect (which would carry it elsewhere) is
        // an unexpected answer.
        .redirects(0)
        .https_only(true)
        .user_agent(&format!("Dobo/{version}"))
        .build();
    let res = match http.post(REPORT_URL).set("Content-Type", "application/json").send_string(&body) {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(_)) => return Err(SendFail::Offline),
    };
    let status = res.status();
    let retry = res.header("Retry-After").and_then(|v| v.trim().parse().ok());
    // The answer is small; anything bigger isn't the service's.
    let mut text = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::Read::take(res.into_reader(), 4097), &mut text);
    service_answer(status, retry, if text.len() > 4096 { "" } else { &text })
}

// ---------- Send by email ----------

/// The email link Dobo may open: a new message to support@sookera.com with a subject and a
/// body, nothing else (no other address, no copy, no attachment).
pub fn mail_link_ok(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("mailto:") else { return false };
    let (to, query) = rest.split_once('?').unwrap_or((rest, ""));
    if !to.eq_ignore_ascii_case(SUPPORT_EMAIL) || url.contains('#') {
        return false;
    }
    let mut seen = Vec::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let key = pair.split('=').next().unwrap_or("").to_ascii_lowercase();
        if !matches!(key.as_str(), "subject" | "body") || seen.contains(&key) {
            return false;
        }
        seen.push(key);
    }
    true
}

// ---------- Windows ----------

/// "Windows 11 Pro 25H2 (build 26200)", read from the registry.
pub fn windows_version() -> String {
    #[cfg(windows)]
    {
        const KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        let get = |name: &str| hklm(KEY, name).filter(|v| !v.trim().is_empty());
        let build = get("CurrentBuild").or_else(|| get("CurrentBuildNumber"));
        windows_label(&get("ProductName").unwrap_or_else(|| "Windows".into()), get("DisplayVersion").as_deref(), build.as_deref())
    }
    #[cfg(not(windows))]
    {
        std::env::consts::OS.to_string()
    }
}

/// Windows 11 still calls itself "Windows 10" in the registry: its build number tells.
fn windows_label(product: &str, display: Option<&str>, build: Option<&str>) -> String {
    let eleven = build.and_then(|b| b.trim().parse::<u32>().ok()).is_some_and(|b| b >= 22000);
    let mut s = if eleven { product.replacen("Windows 10", "Windows 11", 1) } else { product.to_string() };
    if let Some(d) = display {
        s.push(' ');
        s.push_str(d.trim());
    }
    if let Some(b) = build {
        s.push_str(&format!(" (build {})", b.trim()));
    }
    let s: String = s.chars().filter(|c| !c.is_control()).take(120).collect();
    if s.trim().is_empty() { "Windows".into() } else { s }
}

/// A text value under HKEY_LOCAL_MACHINE (read only).
#[cfg(windows)]
fn hklm(key: &str, name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (k, n) = (wide(key), wide(name));
    let mut len = 0u32;
    // SAFETY: NUL-terminated strings; first the size, then the value into a buffer that big.
    unsafe {
        if RegGetValueW(HKEY_LOCAL_MACHINE, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut len) != 0 || len > 4096 {
            return None;
        }
        let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
        let mut size = (buf.len() * 2) as u32;
        if RegGetValueW(HKEY_LOCAL_MACHINE, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size) != 0 {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }
}

/// A folder for the tests' notes (never the real data folder).
#[cfg(test)]
fn test_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dobo-report-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = r"C:\Users\Zak Smith";
    fn c(s: &str) -> String {
        clean_with(s, Some(HOME))
    }

    #[test]
    fn the_user_folder_and_paths_say_nothing_personal() {
        assert_eq!(c(r"couldn't open C:\Users\Zak Smith\AppData\Roaming\com.sookera.ztodo\ztodo.sqlite: denied"), r"couldn't open %USERPROFILE%\…\….sqlite: denied");
        assert_eq!(c(r"c:\users\zak smith\Documents\Shopping list.txt"), r"%USERPROFILE%\…\….txt");
        assert_eq!(c(r"D:\Projets secrets\plan.docx is gone"), r"D:\…\….docx is gone");
        // Names with several spaces, folders and file alike.
        assert_eq!(c(r"C:\Users\Zak Smith\Documents\Lettre à Marie Dupont.docx"), r"%USERPROFILE%\…\….docx");
        assert_eq!(c(r"D:\My Secret Stuff\plan.docx"), r"D:\…\….docx");
        assert_eq!(c(r"couldn't open E:\Marie Claire Dupont\Dobo\data\ztodo.sqlite: locked"), r"couldn't open E:\…\….sqlite: locked");
        // The user's name on its own, and the folder alone.
        assert_eq!(c("ZAK SMITH failed"), "… failed");
        assert_eq!(c("in %USERPROFILE% now"), "in %USERPROFILE% now");
        assert_eq!(c(r"\\nas\Family\photos\me.jpg"), r"\\…\….jpg");
        assert_eq!(c("/home/zak/notes/a.md"), "…\\….md");
        // Not the start of another user's folder.
        assert_eq!(clean_with(r"C:\Users\Al\x.txt and C:\Users\Alice\y.txt", Some(r"C:\Users\Al")), r"%USERPROFILE%\….txt and C:\…\….txt");
        // Debug-printed paths (doubled backslashes, quoted) and forward slashes.
        assert_eq!(c(r#"Os { code: 5, kind: PermissionDenied, message: "Access is denied." } at "C:\\Users\\Zak Smith\\x""#), r#"Os { code: 5, kind: PermissionDenied, message: "…" } at "…""#);
        assert_eq!(c("C:/Users/Zak Smith/Desktop/a.png"), r"%USERPROFILE%\…\….png");
    }

    #[test]
    fn where_the_error_was_stays() {
        assert_eq!(c("index out of bounds: the len is 3 but the index is 7 (src\\db.rs:120)"), "index out of bounds: the len is 3 but the index is 7 (src\\db.rs:120)");
        assert_eq!(c("boom (src/board/layout.rs:9:4)"), "boom (src/board/layout.rs:9:4)");
        assert_eq!(c(r"boom (C:\Users\Zak Smith\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tauri-2.9.0\src\app.rs:123:5)"), r"boom (%USERPROFILE%\…\app.rs:123:5)");
    }

    #[test]
    fn quotes_links_emails_and_ids_go() {
        assert_eq!(c("byte index 3 is not a char boundary; it is inside 'é' of `Buy milk for Zak`"), "byte index 3 is not a char boundary; it is inside 'é' of `…`");
        assert_eq!(c(r"called `Option::unwrap()` on a `None` value (src\reminders.rs:214:31)"), r"called `Option::unwrap()` on a `None` value (src\reminders.rs:214:31)");
        assert_eq!(c("called `Result::unwrap()` on an `Err` value: \"locked\""), "called `Result::unwrap()` on an `Err` value: \"…\"");
        assert_eq!(c("of `Milk` and `Zak::Smith now`"), "of `…`");
        assert_eq!(c(r#"bad title "Lettre à Marie""#), r#"bad title "…""#);
        assert_eq!(c("no end \"to this quote"), "no end \"…");
        assert_eq!(c("fetch https://user:pw@example.com/a/b?q=1 failed"), "fetch https://example.com failed");
        assert_eq!(c("write to zak@example.com now"), "write to … now");
        assert_eq!(c("token ghp_a1B2c3D4e5F6g7H8i9J0k1L2m3N4 refused"), "token … refused");
        assert_eq!(c("read/write and/or 1/2"), "read/write and/or 1/2");
        // A backtick or an escaped quote inside the quoted words never splits them.
        assert_eq!(c(r"inside 'é' of `It`s Marie Dupont's birthday list` (src\db.rs:12:5)"), r"inside 'é' of `…` (src\db.rs:12:5)");
        assert_eq!(c(r#"Error { message: "near \"Marie Dupont\": syntax error" } (src\db.rs:9)"#), r#"Error { message: "…" } (src\db.rs:9)"#);
        assert_eq!(c(r#"Json("He said \"organic milk\"")"#), r#"Json("…")"#);
        // Every kind of quote.
        assert_eq!(c("title ‘Lettre à Marie’ and “Marie” and « Marie » and 'Marie D'"), "title ‘…’ and “…” and «…» and '…'");
        assert_eq!(c("it's the user's file"), "it's the user's file");
        // Links without their scheme, tokens in parts.
        assert_eq!(c("see example.com/Marie-Dupont now"), "see example.com now");
        assert_eq!(c("jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ6YWsifQ.abcDEF123 refused"), "jwt … refused");
    }

    #[test]
    fn controls_marks_and_length() {
        assert_eq!(c("a\u{202E}b\tc\u{0}d"), "a b c d");
        let long = "x ".repeat(400);
        let out = c(&long);
        assert!(out.chars().count() <= 500);
        assert!(out.ends_with(" …"), "cut between words");
        // A cut never leaves half a cleaned path.
        let paths = r"C:\Users\Zak Smith\a\b.txt ".repeat(40);
        let out = c(&paths);
        assert_eq!(c(&out), out);
        assert_eq!(c("   "), "…");
    }

    /// Cleaning a clean line changes nothing, and nothing makes it panic (a release build stops
    /// on a panic): random text of every kind, many times.
    #[test]
    fn cleaning_is_stable_and_never_panics() {
        let long = "Lorem ipsum dolor sit amet ".repeat(4);
        let pieces = [
            "C:", "\\", "/", "\"", "`", "%USERPROFILE%", "src", ".rs", ":12", "@", ".", "://", "é", "😀", " ", "(", ")", "Users", "Zak Smith", "a1b2c3d4e5f6a1b2c3d4e5f6", "\u{202E}", "\n", "…", "x.txt", "\\\\", "\\\"", "'", "‘", "’", "“", "”", "«", "»", "D:\\a b\\", "x/y/", "Option::unwrap()", "None", ":", long.as_str(),
        ];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..40_000 {
            let mut s = String::new();
            for _ in 0..(seed % 30) {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                s.push_str(pieces[(seed % pieces.len() as u64) as usize]);
            }
            let once = c(&s);
            assert_eq!(c(&once), once, "{s:?}");
            assert!(!once.is_empty() && once.chars().count() <= 500);
            // The user folder never stays, in any of its forms.
            let low = once.to_ascii_lowercase();
            assert!(!low.contains(r"users\zak smith") && !low.contains("users/zak smith") && !low.contains(r"users\zak smith"), "{s:?} → {once:?}");
        }
    }

    #[test]
    fn versions_follow_the_services_rule() {
        for v in ["1.1.1", "2.0.0-beta.1", "1.1.0+build5", "10.20.30"] {
            assert!(is_version(v), "{v}");
        }
        for v in ["", "1.1", "1.1.1-", "1.1.1-beta-2", "1.1.1+GH-1", "v1.1.1", "1.1.1 x", &"1".repeat(33)] {
            assert!(!is_version(v), "{v}");
        }
    }

    #[test]
    fn the_notes_are_read_back_newest_first_and_cleaned() {
        let d = test_dir("read");
        std::fs::write(d.join("crash.log"), "2026-11-20_091405 Dobo 1.1.0: index out of range (src\\db.rs:120)\nnot a line\n2026-11-22_100000 Dobo 1.1.1: at C:\\Users\\Zak Smith\\x.txt\n").unwrap();
        std::fs::write(d.join("startup-error.log"), "2026-11-21_080000 Dobo 1.1.0: database is locked\n2026-02-30_080000 Dobo 1.1.0: no such day\n2026-07-01T09:00:00+02:00 Dobo 1.1.1: summer\n").unwrap();
        let list = read_notes(std::slice::from_ref(&d), Some(HOME));
        let msgs: Vec<(&str, &str)> = list.iter().map(|p| (p.area.as_str(), p.message.as_str())).collect();
        assert_eq!(msgs, vec![("crash", r"at %USERPROFILE%\….txt"), ("startup", "database is locked"), ("crash", "index out of range (src\\db.rs:120)"), ("startup", "summer")]);
        assert!(list[0].time.starts_with("2026-11-22T10:00:00"), "{}", list[0].time);
        assert_eq!(list[0].time.len(), 25);
        assert_eq!(list[0].version, "1.1.1");
        // A line written with its offset keeps it, whatever this PC's offset is now.
        let summer = list.iter().find(|p| p.message == "summer").unwrap();
        assert_eq!(summer.time, "2026-07-01T09:00:00+02:00");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_body_is_what_the_service_takes() {
        let p = Problem { time: "2026-11-22T10:00:00+03:00".into(), area: "crash".into(), message: r"at C:\Users\Zak Smith\x.txt".into(), version: "1.1.0".into() };
        let body = report_body("  The board is empty.  ", "dark", "fr", std::slice::from_ref(&p), "1.1.1", false, "Windows 11 Pro").unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        let mut keys = keys;
        keys.sort_unstable();
        assert_eq!(keys, ["app", "description", "install", "language", "look", "problems", "version", "windows"]);
        assert_eq!(v["app"], "dobo");
        assert_eq!(v["install"], "portable");
        assert_eq!(v["description"], "The board is empty.");
        assert!(!body.contains("Zak Smith") || home().is_some_and(|h| h.contains("Zak Smith")), "cleaned again");
        assert!(matches!(report_body(" ", "dark", "en", &[], "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
        assert!(matches!(report_body("\u{200B}\u{FEFF} \u{2060}", "dark", "en", &[], "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
        assert!(matches!(report_body("x", "system", "en", &[], "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
        assert!(matches!(report_body(&"x".repeat(5001), "dark", "en", &[], "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
        let bad = Problem { area: "download".into(), ..p.clone() };
        assert!(matches!(report_body("x", "dark", "en", &[bad], "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
        let eleven = vec![p; 11];
        assert!(matches!(report_body("x", "dark", "en", &eleven, "1.1.1", true, "W"), Err(SendFail::Refused { .. })));
    }

    #[test]
    fn the_services_answers() {
        assert_eq!(service_answer(201, None, r#"{"ok":true,"ref":"42"}"#), Ok("42".into()));
        assert_eq!(service_answer(201, None, r#"{"ok":true,"ref":"<b>"}"#), Err(SendFail::Server));
        assert_eq!(service_answer(201, None, "nope"), Err(SendFail::Server));
        assert_eq!(service_answer(429, Some(120), "{}"), Err(SendFail::Busy { retry_after: 120 }));
        assert_eq!(service_answer(429, None, r#"{"retryAfter":5}"#), Err(SendFail::Busy { retry_after: 30 }));
        assert!(matches!(service_answer(413, None, ""), Err(SendFail::Refused { .. })));
        assert!(matches!(service_answer(400, None, ""), Err(SendFail::Refused { .. })));
        assert_eq!(service_answer(502, None, ""), Err(SendFail::Server));
        assert_eq!(service_answer(301, None, ""), Err(SendFail::Server));
    }

    #[test]
    fn email_only_to_support() {
        assert!(mail_link_ok("mailto:support@sookera.com?subject=Dobo%201.1.1&body=Hi"));
        assert!(mail_link_ok("mailto:support@sookera.com"));
        for bad in [
            "mailto:other@example.com?subject=x",
            "mailto:support@sookera.com,other@example.com",
            "mailto:support@sookera.com?cc=other@example.com",
            "mailto:support@sookera.com?subject=a&subject=b",
            "mailto:support@sookera.com?attach=C:/x",
            "https://sookera.com",
            "mailto:support@sookera.com?body=x#y",
        ] {
            assert!(!mail_link_ok(bad), "{bad}");
        }
    }

    #[test]
    fn windows_11_says_so() {
        assert_eq!(windows_label("Windows 10 Pro", Some("25H2"), Some("26200")), "Windows 11 Pro 25H2 (build 26200)");
        assert_eq!(windows_label("Windows 10 Home", Some("22H2"), Some("19045")), "Windows 10 Home 22H2 (build 19045)");
        assert_eq!(windows_label("", None, None), "Windows");
    }
}
