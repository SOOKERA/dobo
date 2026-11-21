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
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let s0 = a.segments()[0];
            !(a.is_loopback() || a.is_unspecified() || a.is_multicast() || (s0 & 0xfe00) == 0xfc00 || (s0 & 0xffc0) == 0xfe80)
        }
    }
}

/// Every connection (the page, each redirect, its picture) goes through this: only public
/// addresses are used, checked after the name lookup so a name can't point inside either.
fn public_only(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let addrs: Vec<SocketAddr> = netloc.to_socket_addrs()?.filter(|a| is_public(a.ip())).collect();
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
        p.title = final_url.rsplit('/').next().unwrap_or("").to_string();
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

fn save_image(a: &ureq::Agent, src: &str, dir: &Path) -> Result<String, String> {
    let resp = a.get(src).call().map_err(|e| e.to_string())?;
    let ext = match resp.content_type() {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => return Err("svg skipped".into()),
        _ => "jpg",
    };
    let mut bytes = Vec::new();
    resp.into_reader().take(4_000_000).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 200 {
        return Err("too small".into());
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("preview-{:016x}.{ext}", h.finish()));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(public_only("93.184.216.34:443").is_ok());
        assert!(public_only("[2606:4700::1111]:443").is_ok());
        assert!(fetch("http://127.0.0.1:9/", &std::env::temp_dir()).is_err());
        assert!(fetch("file:///C:/Windows/win.ini", &std::env::temp_dir()).is_err());
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
