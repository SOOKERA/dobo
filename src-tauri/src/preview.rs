// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Link previews, like in a messaging app: the page's title, site name and picture.
//! The picture is saved next to the notes so it shows offline.

use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

#[derive(Serialize, Clone, Debug, Default)]
pub struct Preview {
    pub url: String,
    pub title: String,
    pub site: String,
    /// Local file of the page's picture, when it has one.
    pub image: Option<String>,
}

/// Addresses on the internet. Previews never reach this computer, the home or office network
/// (a router's page, a printer, a dev server) or other special addresses.
fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let [o0, o1, ..] = a.octets();
            !(a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_unspecified()
                || a.is_broadcast()
                || a.is_documentation()
                || o0 == 0
                || o0 >= 224
                || (o0 == 100 && (o1 & 0xC0) == 64)
                || (o0 == 198 && (o1 & 0xFE) == 18)
                || (o0 == 192 && o1 == 0))
        }
        IpAddr::V6(a) => {
            let seg = a.segments();
            let v4 = |hi: u16, lo: u16| IpAddr::V4(std::net::Ipv4Addr::from((u32::from(hi) << 16) | u32::from(lo)));
            // An IPv4 address inside an IPv6 one is judged as that IPv4 address: mapped
            // (::ffff:a.b.c.d), compatible (::a.b.c.d), NAT64 (64:ff9b::a.b.c.d) and 6to4
            // (2002:aabb:ccdd::), all of which can reach the IPv4 network behind them.
            if let Some(m) = a.to_ipv4_mapped() {
                return is_public(IpAddr::V4(m));
            }
            if seg[..6].iter().all(|&x| x == 0) {
                return is_public(v4(seg[6], seg[7]));
            }
            if seg[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
                return is_public(v4(seg[6], seg[7]));
            }
            if seg[0] == 0x2002 {
                return is_public(v4(seg[1], seg[2]));
            }
            let s0 = seg[0];
            !(a.is_loopback()
                || a.is_unspecified()
                || a.is_multicast()
                || (s0 & 0xfe00) == 0xfc00 // unique local
                || (s0 & 0xffc0) == 0xfe80 // link-local
                || (s0 & 0xffc0) == 0xfec0 // site-local (old)
                || (s0 == 0x64 && seg[1] == 0xff9b) // NAT64 for local use (64:ff9b:1::/48)
                || (s0 == 0x2001 && seg[1] < 0x0200) // IETF special use: Teredo, benchmarking… (2001::/23)
                || (s0 == 0x2001 && seg[1] == 0x0db8) // documentation
                || seg[..4] == [0x100, 0, 0, 0]) // discard-only (100::/64)
        }
    }
}

/// Name lookups still running, given up on or not (Windows' own lookup can't be stopped).
static LOOKUPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// At most this many at once: a DNS that never answers leaves a few threads waiting, never more.
const MAX_LOOKUPS: usize = 4;

/// A name lookup that takes too long (a slow DNS) gives up, as the connection would: Windows'
/// own lookup has no time limit of ureq's. An address written as such needs no lookup.
fn lookup(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    use std::sync::atomic::Ordering;
    if let Ok(a) = netloc.parse::<SocketAddr>() {
        return Ok(vec![a]);
    }
    let slow = || std::io::Error::new(std::io::ErrorKind::TimedOut, "the name lookup took too long");
    if LOOKUPS.fetch_add(1, Ordering::AcqRel) >= MAX_LOOKUPS {
        LOOKUPS.fetch_sub(1, Ordering::AcqRel);
        return Err(slow());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let name = netloc.to_string();
    let started = std::thread::Builder::new().name("dobo-lookup".into()).spawn(move || {
        let _ = tx.send(name.to_socket_addrs().map(|a| a.collect::<Vec<_>>()));
        LOOKUPS.fetch_sub(1, Ordering::AcqRel);
    });
    if started.is_err() {
        LOOKUPS.fetch_sub(1, Ordering::AcqRel);
        return Err(slow());
    }
    rx.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| Err(slow()))
}

/// Every connection (the page, each redirect, its picture) goes through this: only public
/// addresses are used, checked after the name lookup so a name can't point inside either.
fn public_only(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let addrs: Vec<SocketAddr> = lookup(netloc)?.into_iter().filter(|a| is_public(a.ip())).collect();
    if addrs.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "not an internet address"));
    }
    Ok(addrs)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .resolver(public_only)
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(8))
        .redirects(5)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36 Dobo")
        .build()
}

fn decode(s: &str) -> String {
    let s = s
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The attributes of one tag (`<meta property="og:title" content="…">`).
fn attrs(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'>' {
            i += 1;
        }
        let name = tag[start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            let value = if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                i += 1;
                let vs = i;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                let v = &tag[vs..i.min(tag.len())];
                i += 1;
                v
            } else {
                let vs = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                &tag[vs..i]
            };
            if !name.is_empty() {
                out.push((name, value.to_string()));
            }
        } else if name.is_empty() {
            i += 1;
        }
    }
    out
}

pub fn parse(html: &str, url: &str) -> (String, String, Option<String>) {
    let lower = html.to_ascii_lowercase();
    let mut meta = std::collections::HashMap::new();
    let mut pos = 0;
    while let Some(off) = lower[pos..].find("<meta") {
        let s = pos + off + 5;
        let Some(end) = lower[s..].find('>') else { break };
        let a = attrs(&html[s..s + end]);
        let key = a.iter().find(|(k, _)| k == "property" || k == "name").map(|(_, v)| v.to_ascii_lowercase());
        let content = a.iter().find(|(k, _)| k == "content").map(|(_, v)| decode(v));
        if let (Some(k), Some(c)) = (key, content) {
            meta.entry(k).or_insert(c);
        }
        pos = s + end;
    }
    let title_tag = lower.find("<title").and_then(|s| {
        let open = s + lower[s..].find('>')? + 1;
        let close = open + lower[open..].find("</title")?;
        Some(decode(&html[open..close]))
    });
    let title = meta.get("og:title").or(meta.get("twitter:title")).cloned().or(title_tag).unwrap_or_default();
    let host = host_of(url);
    let site = meta.get("og:site_name").cloned().unwrap_or_else(|| host.trim_start_matches("www.").to_string());
    let image = meta.get("og:image").or(meta.get("og:image:url")).or(meta.get("twitter:image")).map(|i| absolute(i, url));
    (title, site, image)
}

fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    rest.split(['/', '?', '#']).next().unwrap_or("").to_string()
}

fn absolute(src: &str, page: &str) -> String {
    if src.starts_with("http://") || src.starts_with("https://") {
        src.to_string()
    } else if let Some(r) = src.strip_prefix("//") {
        format!("https://{r}")
    } else {
        let scheme = page.split("://").next().unwrap_or("https");
        let host = host_of(page);
        if src.starts_with('/') { format!("{scheme}://{host}{src}") } else { format!("{scheme}://{host}/{src}") }
    }
}

pub fn fetch(url: &str, dir: &Path) -> Result<Preview, String> {
    let url = if url.contains("://") { url.to_string() } else { format!("https://{url}") };
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(tr!("Only web links get a preview.", "Seuls les liens web ont un aperçu."));
    }
    let a = agent();
    let resp = a.get(&url).call().map_err(|e| e.to_string())?;
    let final_url = resp.get_url().to_string();
    let is_html = resp.content_type().contains("html");
    let mut body = Vec::new();
    resp.into_reader().take(800_000).read_to_end(&mut body).map_err(|e| e.to_string())?;
    let mut p = Preview { url: url.clone(), site: host_of(&final_url).trim_start_matches("www.").to_string(), ..Default::default() };
    if !is_html {
        p.title = final_url.rsplit('/').next().unwrap_or("").chars().take(300).collect();
        return Ok(p);
    }
    let html = String::from_utf8_lossy(&body);
    let (title, site, image) = parse(&html, &final_url);
    // A page can say anything: kept to a sensible length.
    p.title = title.chars().take(300).collect();
    p.site = site.chars().take(100).collect();
    if let Some(img) = image {
        p.image = save_image(&a, &img, dir).ok();
    }
    Ok(p)
}

/// The biggest preview picture kept.
const IMAGE_MAX: u64 = 4_000_000;

/// What a picture's first bytes say it is (its file type), or None when it isn't one Dobo shows.
fn picture_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

/// The page's picture, saved next to the notes: only a real picture (said so by the server, and
/// by its own first bytes), whole (not cut at the size limit).
fn save_image(a: &ureq::Agent, src: &str, dir: &Path) -> Result<String, String> {
    let resp = a.get(src).call().map_err(|e| e.to_string())?;
    let kind = resp.content_type().to_ascii_lowercase();
    if !kind.starts_with("image/") || kind.starts_with("image/svg") {
        return Err("not a picture".into());
    }
    let mut bytes = Vec::new();
    resp.into_reader().take(IMAGE_MAX + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 200 {
        return Err("too small".into());
    }
    if bytes.len() as u64 > IMAGE_MAX {
        return Err("too big".into());
    }
    let ext = picture_kind(&bytes).ok_or("not a picture")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = image_path(dir, src, ext);
    // (Written beside, then renamed over: a file of the same name is replaced, never changed in
    // place, so a backup made before an update that links to it keeps its own, backup.rs
    // `link_files`.)
    let part = path.with_file_name(format!("{}.part", path.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::write(&part, &bytes).and_then(|_| std::fs::rename(&part, &path)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })?;
    Ok(path.to_string_lossy().to_string())
}

/// Where the picture at `src` is kept: right in `dir` (the data folder's previews), named by its
/// address.
pub(crate) fn image_path(dir: &Path, src: &str, ext: &str) -> std::path::PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    dir.join(format!("preview-{:016x}.{ext}", h.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_needs_no_lookup() {
        assert_eq!(lookup("93.184.216.34:443").unwrap(), vec!["93.184.216.34:443".parse::<SocketAddr>().unwrap()]);
        assert_eq!(lookup("[2606:2800:220:1::]:80").unwrap().len(), 1);
    }

    #[test]
    fn reads_open_graph() {
        let html = r#"<html><head><title>Fallback</title>
            <meta property="og:title" content="Hangar Y &amp; the walkthrough">
            <meta content='https://cdn.example.com/a.jpg' property='og:image' />
            <meta name="og:site_name" content="Example">
            </head></html>"#;
        let (t, s, i) = parse(html, "https://www.example.com/page");
        assert_eq!(t, "Hangar Y & the walkthrough");
        assert_eq!(s, "Example");
        assert_eq!(i.as_deref(), Some("https://cdn.example.com/a.jpg"));
    }

    #[test]
    fn never_reaches_this_computer_or_the_local_network() {
        for local in [
            "127.0.0.1:80", "10.0.0.8:80", "192.168.1.1:80", "172.16.0.1:443", "169.254.169.254:80", "0.0.0.0:80", "100.64.0.1:80",
            "[::1]:80", "[fe80::1]:80", "[fd00::1]:80", "[::ffff:192.168.1.1]:80", "localhost:80",
        ] {
            assert!(public_only(local).is_err(), "{local}");
        }
        // IPv4 inside IPv6 (NAT64, 6to4, compatible) and IPv6's own special ranges.
        for inside in [
            "64:ff9b::a00:8", "64:ff9b::7f00:1", "2002:c0a8:101::", "2002:7f00:1::1", "::7f00:1", "::a00:1", "64:ff9b:1::1", "fec0::1",
            "2001:db8::1", "100::1", "2001::1", "2001:0:4136:e378::1", "ff02::1",
        ] {
            assert!(!is_public(inside.parse().unwrap()), "{inside}");
        }
        for public in ["64:ff9b::5db8:d822", "2002:5db8:d822::1", "2606:4700::1111", "2a00:1450:4007::200e"] {
            assert!(is_public(public.parse().unwrap()), "{public}");
        }
        assert!(public_only("93.184.216.34:443").is_ok());
        assert!(public_only("[2606:4700::1111]:443").is_ok());
        assert!(fetch("http://127.0.0.1:9/", &std::env::temp_dir()).is_err());
        assert!(fetch("file:///C:/Windows/win.ini", &std::env::temp_dir()).is_err());
    }

    #[test]
    fn only_real_pictures_are_kept() {
        assert_eq!(picture_kind(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(picture_kind(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("jpg"));
        assert_eq!(picture_kind(b"GIF89a.."), Some("gif"));
        assert_eq!(picture_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(picture_kind(b"BM\0\0"), Some("bmp"));
        assert_eq!(picture_kind(b"<!doctype html><html>"), None);
        assert_eq!(picture_kind(b"RIFF\0\0\0\0WAVEfmt "), None);
        assert_eq!(picture_kind(b""), None);
    }

    #[test]
    fn title_and_host_fallbacks() {
        let html = "<head><TITLE> A page </TITLE><meta property=og:image content=/img.png></head>";
        let (t, s, i) = parse(html, "https://www.site.fr/x/y");
        assert_eq!(t, "A page");
        assert_eq!(s, "site.fr");
        assert_eq!(i.as_deref(), Some("https://www.site.fr/img.png"));
    }
}
