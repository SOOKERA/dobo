// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! The words Rust shows (notifications and their buttons, the tray menu, dialogs, messages) in
//! English or French. The language comes from the same setting as the app's ("system": as
//! Windows speaks, else "en" / "fr"); it is read at start and whenever Settings changes it.
//!
//! A string is written in both languages where it is used: `tr!("Due now · {}", "C'est l'heure · {}", title)`.

use std::sync::atomic::{AtomicBool, Ordering};

static FRENCH: AtomicBool = AtomicBool::new(false);

// Tests run side by side: each test thread keeps its own language (English unless it says).
#[cfg(test)]
thread_local! {
    static TEST_FRENCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// French right now?
pub fn is_fr() -> bool {
    #[cfg(test)]
    {
        TEST_FRENCH.with(|f| f.get())
    }
    #[cfg(not(test))]
    {
        FRENCH.load(Ordering::Relaxed)
    }
}

/// "en" or "fr" for a setting's value ("system", "en", "fr", or nothing yet).
pub fn resolve(choice: &str) -> &'static str {
    match choice {
        "fr" => "fr",
        "en" => "en",
        _ => system(),
    }
}

/// The language Windows shows its own words in: "fr" for any French, else "en".
pub fn system() -> &'static str {
    #[cfg(windows)]
    {
        // The primary language id is the low 10 bits; 0x0C is French (France, Canada, Belgium…).
        let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        if id & 0x3ff == 0x0c {
            return "fr";
        }
        "en"
    }
    #[cfg(not(windows))]
    {
        let lang = std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")).unwrap_or_default();
        if lang.to_ascii_lowercase().starts_with("fr") { "fr" } else { "en" }
    }
}

/// The setting as stored ("system" when never chosen).
pub fn choice(conn: &rusqlite::Connection) -> String {
    crate::db::get_setting(conn, "language").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "system".into())
}

/// Reads the setting and uses it from now on.
pub fn load(conn: &rusqlite::Connection) {
    set(&choice(conn));
}

pub fn set(choice: &str) {
    let fr = resolve(choice) == "fr";
    FRENCH.store(fr, Ordering::Relaxed);
    #[cfg(test)]
    TEST_FRENCH.with(|f| f.set(fr));
}

/// A note's title as shown: the Inbox is called "Inbox" underneath, and shows in the language.
pub fn note_title(title: &str) -> String {
    if title == "Inbox" && is_fr() { "À trier".into() } else { title.to_string() }
}

/// "3rd" / "3e" (French: "1er").
pub fn ordinal(n: i64) -> String {
    if is_fr() {
        return if n == 1 { "1er".into() } else { format!("{n}e") };
    }
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A weekday by number (0 = Sunday): "Thursday" / "jeudi".
pub fn weekday(d: u32) -> &'static str {
    const EN: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    const FR: [&str; 7] = ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"];
    let i = (d % 7) as usize;
    if is_fr() { FR[i] } else { EN[i] }
}

/// A month by number (1 = January), short: "Dec" / "déc.".
pub fn month_short(m: u32) -> &'static str {
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const FR: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    let i = (m.clamp(1, 12) - 1) as usize;
    if is_fr() { FR[i] } else { EN[i] }
}

/// "10 min", "1 h", "1 h 30" (French) / "1 h 30 min" (English).
pub fn duration(min: i64) -> String {
    if min < 60 {
        return format!("{min} min");
    }
    let (h, m) = (min / 60, min % 60);
    if m == 0 {
        format!("{h} h")
    } else if is_fr() {
        format!("{h} h {m:02}")
    } else {
        format!("{h} h {m} min")
    }
}

/// A shortcut as Windows names its keys ("Ctrl+Shift+Space" → "Ctrl+Maj+Espace" in French).
pub fn keys(accel: &str) -> String {
    if !is_fr() {
        return accel.replace("Comma", ",");
    }
    accel
        .split('+')
        .map(|k| match k {
            "Shift" => "Maj",
            "Space" => "Espace",
            "Enter" => "Entrée",
            "Esc" | "Escape" => "Échap",
            "Delete" => "Suppr",
            "Backspace" => "Retour arrière",
            "Comma" => ",",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// French typography for a whole line: a non-breaking space before : and », after «, a thin one
/// before ; ? ! (Windows shows notifications with these, like any French text).
pub fn fr_type(s: &str) -> String {
    s.replace(" :", "\u{a0}:").replace(" ;", "\u{202f};").replace(" ?", "\u{202f}?").replace(" !", "\u{202f}!").replace("« ", "«\u{a0}").replace(" »", "\u{a0}»")
}

/// A French line with its `{}` filled in order. The typography goes on our words only (the
/// template), never on what is put in: a note's title or a line keeps its spaces as typed.
pub fn fill(template: &str, args: &[String]) -> String {
    let typed = fr_type(template);
    let mut out = String::with_capacity(typed.len() + args.iter().map(|a| a.len()).sum::<usize>());
    let mut rest = typed.as_str();
    let mut next = args.iter();
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        out.push_str(next.next().map(String::as_str).unwrap_or("{}"));
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// The words in the language now: `tr!("English {}", "Français {}", arg)`. French gets its
/// typography (non-breaking spaces) added to our words.
macro_rules! tr {
    ($en:literal, $fr:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::is_fr() {
            // (Never run: the compiler checks the French words take the same arguments.)
            if false {
                let _ = format!($fr $(, $arg)*);
            }
            $crate::i18n::fill($fr, &[$(format!("{}", $arg)),*])
        } else {
            format!($en $(, $arg)*)
        }
    };
}

/// A count takes the singular ("one") form, as in the app (Intl.PluralRules): in English 1 alone,
/// in French 0 and 1 (« 0 tâche », « 1 tâche », « 2 tâches »).
pub fn is_one(n: i64) -> bool {
    if is_fr() {
        n == 0 || n == 1
    } else {
        n == 1
    }
}

/// The words for a count, in the language now and in its plural form, as the app's { one, other }
/// entries: `trn!(n, "{} task", "{} tasks", "{} tâche", "{} tâches", n)`.
macro_rules! trn {
    ($n:expr, $en_one:literal, $en_other:literal, $fr_one:literal, $fr_other:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::is_one(($n) as i64) {
            tr!($en_one, $fr_one $(, $arg)*)
        } else {
            tr!($en_other, $fr_other $(, $arg)*)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_in_both_languages() {
        set("en");
        assert_eq!(tr!("Due now · {}", "C’est l’heure · {}", "Hub"), "Due now · Hub");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(duration(90), "1 h 30 min");
        assert_eq!(note_title("Inbox"), "Inbox");
        set("fr");
        assert_eq!(tr!("Is “{}” really done?", "« {} », vraiment fini ?", "Payer"), "«\u{a0}Payer\u{a0}», vraiment fini\u{202f}?");
        assert_eq!(ordinal(1), "1er");
        assert_eq!(ordinal(3), "3e");
        assert_eq!(duration(90), "1 h 30");
        assert_eq!(duration(60), "1 h");
        assert_eq!(weekday(4), "jeudi");
        assert_eq!(note_title("Inbox"), "À trier");
        assert_eq!(note_title("Hub"), "Hub");
        // The user's own words keep their spaces; ours get the French ones.
        assert_eq!(tr!("Due now · {}", "C’est l’heure : {}", "Appel : client ?"), "C’est l’heure\u{a0}: Appel : client ?");
        assert_eq!(fill("{} et {}", &["a".into()]), "a et {}");
        // Plurals as the app's: French 0 and 1 are singular.
        let more = |n: usize| trn!(n, "and {} more", "and {} more", "et {} autre", "et {} autres", n);
        assert_eq!([more(0), more(1), more(2)], ["et 0 autre", "et 1 autre", "et 2 autres"]);
        set("en");
        let tasks = |n: usize| trn!(n, "{} task", "{} tasks", "{} tâche", "{} tâches", n);
        assert_eq!([tasks(0), tasks(1), tasks(2)], ["0 tasks", "1 task", "2 tasks"]);
        assert_eq!(resolve("fr"), "fr");
        assert_eq!(resolve("en"), "en");
        assert!(["en", "fr"].contains(&resolve("system")));
    }
}
