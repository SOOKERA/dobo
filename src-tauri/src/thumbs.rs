// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//! Board-size copies of the notes' pictures ("thumbs"): a photo of 4000 × 3000 shown in a box of
//! 300 × 150 costs the board its whole file read and 48 MB decoded; its thumb, a few hundred KB.
//! The board shows the thumb, the viewer the picture itself.
//!
//! - **Where:** `pictures/thumbs/<the picture's file name>.<width>x<height>.webp`, so the file
//!   names the picture it was made from and its size (the board knows whether it is sharp enough
//!   for a box without opening it). Only the pictures folder's own files get one.
//! - **Size:** enough to cover a box of `BOX_W` × `BOX_H` pixels, the biggest picture box of a
//!   note at the large note size on a 200% screen; a picture not much bigger than that keeps no
//!   thumb (`size_for`). A wider box (a wide note on a 200% screen) shows the picture itself.
//! - **What:** turned as its Exif says (as the browser shows the picture), its transparency kept, its colour profile carried over; WebP (libwebp), lossy at
//!   a high quality, sharp colour edges (`use_sharp_yuv`). A moving picture (a GIF of several
//!   frames, an animated WebP, an APNG) gets none: it keeps moving on the board (`animated`).
//! - **Once:** a picture that gets none (small enough, moving, or unreadable) is noted so beside
//!   the thumbs (`<picture>.none`, `<picture>.failed`), so it isn't read again at every start.
//! - **Never needed:** backups leave them out (they copy the pictures folder's files, not its
//!   folders), a restore makes them again little by little, and the clean-up at start removes
//!   those of pictures no longer used (`unused`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The thumbs' folder, inside the pictures folder.
pub const DIR: &str = "thumbs";
/// The box a thumb covers, in pixels: a note's picture box at the large note size (about 570 ×
/// 172 points in the widest normal note) on a 200% screen.
pub const BOX_W: u32 = 1200;
pub const BOX_H: u32 = 360;
/// A picture this close to the thumb's size (its sides at most 1/0.75 of the thumb's) keeps none:
/// the board shows it as it is.
const WORTH: f64 = 0.75;
/// WebP quality (0–100). Shown smaller than it is on the board, no trace of the encoding is seen.
const QUALITY: f32 = 88.0;
/// A picture bigger than this (in pixels) gets no thumb: decoding it would take too much memory.
const MAX_PIXELS: u64 = 100_000_000;

/// A thumb as the board uses it: its file, and its size in pixels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Thumb {
    pub path: String,
    pub w: u32,
    pub h: u32,
}

/// The thumb's size for a picture of `w` × `h` (as shown, turned): just enough to cover
/// `BOX_W` × `BOX_H`, the same shape. None: the picture is small enough as it is.
pub fn size_for(w: u32, h: u32) -> Option<(u32, u32)> {
    if w == 0 || h == 0 {
        return None;
    }
    let scale = (f64::from(BOX_W) / f64::from(w)).max(f64::from(BOX_H) / f64::from(h));
    if scale > WORTH {
        return None;
    }
    let side = |s: u32| ((f64::from(s) * scale).round() as u32).max(1);
    let (tw, th) = (side(w), side(h));
    // (WebP can't be wider or taller than that: a very long strip shows as it is.)
    (tw.max(th) <= WEBP_MAX).then_some((tw, th))
}

/// The longest side a WebP can have.
const WEBP_MAX: u32 = 16383;

/// A thumb's file name: the picture's name, then the thumb's size.
pub fn name_for(picture: &str, w: u32, h: u32) -> String {
    format!("{picture}.{w}x{h}.webp")
}

/// A thumb's file name read back: the picture's name and the thumb's size.
pub fn parse(name: &str) -> Option<(&str, u32, u32)> {
    let rest = name.strip_suffix(".webp")?;
    let (picture, size) = rest.rsplit_once('.')?;
    let (w, h) = size.split_once('x')?;
    let (w, h) = (w.parse().ok()?, h.parse().ok()?);
    (!picture.is_empty() && w > 0 && h > 0).then_some((picture, w, h))
}

/// A file's name as compared here: the name alone, in lower case (as lib.rs `file_key`).
fn key(path: &str) -> Option<String> {
    let name = path.rsplit(['\\', '/']).next()?.trim();
    (!name.is_empty()).then(|| name.to_lowercase())
}

/// The thumbs in `dir`, by their picture's name (`key`).
pub fn existing(dir: &Path) -> std::collections::HashMap<String, Thumb> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Default::default() };
    let mut out = std::collections::HashMap::new();
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(name) = name.to_str() else { continue };
        if let Some((picture, w, h)) = parse(name)
            && let Some(k) = key(picture)
        {
            out.insert(k, Thumb { path: dir.join(name).to_string_lossy().into_owned(), w, h });
        }
    }
    out
}

/// Is `path` a file right in `folder` (any letter case, any slashes), by its name? Never another
/// stream of a file (`1-0.jpg:hidden`) nor a device Windows opens whatever the folder (CON,
/// NUL, COM1…, with or without an extension).
pub fn directly_in(path: &str, folder: &Path) -> bool {
    let norm = |s: &str| s.replace('/', "\\").to_lowercase();
    let dir = norm(&folder.to_string_lossy());
    let dir = dir.trim_end_matches('\\');
    let p = norm(path);
    !dir.is_empty()
        && p.strip_prefix(dir)
            .and_then(|rest| rest.strip_prefix('\\'))
            .is_some_and(|name| !name.is_empty() && !name.contains(['\\', ':']) && name != "." && name != ".." && !is_device(name))
}

/// A name Windows takes for a device, whatever the extension: CON, PRN, AUX, NUL, CONIN$,
/// CONOUT$, COM1…COM9 and LPT1…LPT9 (COM¹ … LPT³ too; and 0, which some versions reserve).
fn is_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches([' ', '.']).to_uppercase();
    let letters: Vec<char> = stem.chars().collect();
    ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || (letters.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && (letters[3].is_ascii_digit() || "¹²³".contains(letters[3])))
}

/// `directly_in`, and a file there now (not a folder, nor nothing): a picture a thumb may be made
/// of.
pub fn is_own_picture(path: &str, folder: &Path) -> bool {
    directly_in(path, folder) && std::fs::metadata(path).is_ok_and(|m| m.is_file())
}

/// Each picture of the pictures folder (`pictures`) given its thumb, when it has one.
pub fn attach(list: &mut [crate::db::Picture], pictures: &Path) {
    let made = existing(&pictures.join(DIR));
    if made.is_empty() {
        return;
    }
    for p in list.iter_mut() {
        if directly_in(&p.path, pictures) {
            p.thumb = key(&p.path).and_then(|k| made.get(&k).cloned());
        }
    }
}

/// The thumb of `picture` (a file of the pictures folder), made now unless it was already: None
/// when the picture is small enough as it is. An error: it can't be read (not a picture, a kind
/// this can't read, damaged) or the thumb can't be written; the board shows the picture itself.
/// What a picture's reading found when it made no thumb (small enough, moving, unreadable) is
/// remembered beside the thumbs (`NONE`, `FAILED`): it is never read again at the next start.
pub fn make(picture: &Path) -> Result<Option<Thumb>, String> {
    let name = picture.file_name().and_then(|n| n.to_str()).ok_or("no file name")?;
    let dir = picture.parent().ok_or("no folder")?.join(DIR);
    if let Some(t) = key(name).and_then(|k| existing(&dir).remove(&k)) {
        return Ok(Some(t));
    }
    if let Some(said) = remembered(&dir, name) {
        return said;
    }
    // (A file that can't be opened just now, held by another program: not remembered, it is
    // tried again next time.)
    std::fs::File::open(picture).map_err(|e| e.to_string())?;
    let made = match read(picture) {
        Ok(Some(made)) => made,
        Ok(None) => {
            remember(&dir, name, None);
            return Ok(None);
        }
        Err(e) => {
            // (Only what the picture itself is: a read that failed just now is tried again.)
            if e.lasting {
                remember(&dir, name, Some(&e.said));
            }
            return Err(e.said);
        }
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = name_for(name, made.w, made.h);
    let to = dir.join(&file);
    // Written beside under a name of its own, then renamed: a thumb is whole or not there.
    let part = dir.join(format!("{file}.part"));
    std::fs::write(&part, &made.webp).and_then(|_| std::fs::rename(&part, &to)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })?;
    Ok(Some(Thumb { path: to.to_string_lossy().into_owned(), w: made.w, h: made.h }))
}

/// Beside the thumbs: "<picture>.none", the picture gets none (small enough, or it moves);
/// "<picture>.failed", it can't be read (the version of Dobo that found it, then what was said).
/// Another version reads it again: it may read that picture, or have been wrong once.
const NONE: &str = ".none";
const FAILED: &str = ".failed";

/// The first line of a `FAILED` file: the version that wrote it.
fn failed_by() -> String {
    format!("Dobo {}", env!("CARGO_PKG_VERSION"))
}

/// What an earlier reading of the picture `name` found, when it made no thumb.
fn remembered(dir: &Path, name: &str) -> Option<Result<Option<Thumb>, String>> {
    if dir.join(format!("{name}{NONE}")).is_file() {
        return Some(Ok(None));
    }
    let said = std::fs::read_to_string(dir.join(format!("{name}{FAILED}"))).ok()?;
    let (by, error) = said.split_once('\n').unwrap_or((&said, ""));
    (by.trim() == failed_by()).then(|| Err(Some(error.trim()).filter(|e| !e.is_empty()).map(String::from).unwrap_or_else(|| "it couldn’t be read".into())))
}

/// Remembers that the picture `name` gets no thumb (`error`: it can't be read). (Not written,
/// it is only read again next time.)
fn remember(dir: &Path, name: &str, error: Option<&str>) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let _ = match error {
        None => std::fs::write(dir.join(format!("{name}{NONE}")), b""),
        Some(e) => std::fs::write(dir.join(format!("{name}{FAILED}")), format!("{}\n{e}", failed_by())),
    };
}

/// Why a picture made no thumb when it was read: `lasting` when it is the picture itself (it
/// can't be decoded, a kind this can't read, too big): remembered (`FAILED`). Else (the disk or
/// memory just then, the thumb's own making) it is tried again next time.
#[derive(Debug)]
struct Unread {
    said: String,
    lasting: bool,
}

impl Unread {
    /// A failure that isn't the picture's.
    fn now(said: impl ToString) -> Unread {
        Unread { said: said.to_string(), lasting: false }
    }
}

impl From<image::ImageError> for Unread {
    /// The picture's own, but for a read that failed (the disk) or memory that ran out.
    fn from(e: image::ImageError) -> Unread {
        let lasting = !matches!(e, image::ImageError::IoError(_)) && !matches!(&e, image::ImageError::Limits(l) if l.kind() == image::error::LimitErrorKind::InsufficientMemory);
        Unread { said: e.to_string(), lasting }
    }
}

/// The picture read and made thumb-size (None: it gets none: small enough, or it moves).
fn read(picture: &Path) -> Result<Option<Made>, Unread> {
    let open = || -> Result<image::ImageReader<std::io::BufReader<std::fs::File>>, Unread> {
        let file = std::fs::File::open(picture).map_err(Unread::now)?;
        image::ImageReader::new(std::io::BufReader::new(file)).with_guessed_format().map_err(Unread::now)
    };
    // Its size first, from its header alone: a picture that says it is huge (a GIF of a few bytes
    // may say 65535 × 65535, 17 GB of pixels) is never decoded, not even to see whether it moves.
    let (w, h) = open()?.into_dimensions()?;
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(Unread { said: "too big to make smaller".into(), lasting: true });
    }
    let reader = open()?;
    // A moving picture keeps moving on the board (the owner's choice): no thumb, it shows itself.
    if animated(picture, reader.format()) {
        return Ok(None);
    }
    shrink(reader)
}

/// Whether the picture at `path` moves: a GIF of more than one frame, an animated WebP or an
/// APNG. Unreadable counts as not moving (the thumb's own reading then says why). Read within
/// the usual memory limits (`Limits::default`: 512 MB): a frame that says it is bigger than its
/// picture is refused, never made room for.
fn animated(path: &Path, format: Option<image::ImageFormat>) -> bool {
    use image::{AnimationDecoder, ImageDecoder, Limits};
    let open = || std::fs::File::open(path).ok().map(std::io::BufReader::new);
    match format {
        Some(image::ImageFormat::Gif) => open()
            .and_then(|r| image::codecs::gif::GifDecoder::new(r).ok())
            .and_then(|mut d| d.set_limits(Limits::default()).ok().map(|_| d))
            // (Frames that can't be read don't count: a refused one isn't a second frame.)
            .is_some_and(|d| d.into_frames().take(2).take_while(Result::is_ok).count() > 1),
        Some(image::ImageFormat::WebP) => {
            open().and_then(|r| image::codecs::webp::WebPDecoder::new(r).ok()).is_some_and(|d| d.has_animation())
        }
        Some(image::ImageFormat::Png) => open()
            .and_then(|r| image::codecs::png::PngDecoder::with_limits(r, Limits::default()).ok())
            .is_some_and(|d| d.is_apng().unwrap_or(false)),
        _ => false,
    }
}

/// A thumb made in memory.
struct Made {
    webp: Vec<u8>,
    w: u32,
    h: u32,
}

/// The picture `reader` reads, made thumb-size and written as WebP (None: small enough).
fn shrink<R: std::io::BufRead + std::io::Seek>(reader: image::ImageReader<R>) -> Result<Option<Made>, Unread> {
    use image::{ImageDecoder, metadata::Orientation};
    let mut decoder = reader.into_decoder()?;
    let turn = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let icc = decoder.icc_profile().ok().flatten();
    let (w, h) = decoder.dimensions();
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(Unread { said: "too big to make smaller".into(), lasting: true });
    }
    let sideways = matches!(turn, Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH);
    let (sw, sh) = if sideways { (h, w) } else { (w, h) };
    let Some((tw, th)) = size_for(sw, sh) else { return Ok(None) };
    // (An animation: its first frame.)
    let mut img = image::DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(turn);
    let (w, h) = (img.width(), img.height());
    let alpha = img.color().has_alpha();
    let (pixels, kind) = if alpha {
        let rgba = img.into_rgba8().into_raw();
        if rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255) {
            // Transparency it never uses: written without (smaller).
            (rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect(), fast_image_resize::PixelType::U8x3)
        } else {
            (rgba, fast_image_resize::PixelType::U8x4)
        }
    } else {
        (img.into_rgb8().into_raw(), fast_image_resize::PixelType::U8x3)
    };
    let small = resize(pixels, w, h, tw, th, kind).map_err(Unread::now)?;
    let webp = encode(&small, tw, th, kind == fast_image_resize::PixelType::U8x4).map_err(Unread::now)?;
    let webp = match icc {
        Some(icc) if is_rgb_profile(&icc) => with_icc(&webp, &icc, tw, th).map_err(Unread::now)?,
        _ => webp,
    };
    Ok(Some(Made { webp, w: tw, h: th }))
}

/// Lanczos3 (the sharpest of the usual filters), colours weighted by their transparency.
fn resize(pixels: Vec<u8>, w: u32, h: u32, tw: u32, th: u32, kind: fast_image_resize::PixelType) -> Result<Vec<u8>, String> {
    use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer, images::Image};
    let src = Image::from_vec_u8(w, h, pixels, kind).map_err(|e| e.to_string())?;
    let mut dst = Image::new(tw, th, kind);
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    Resizer::new().resize(&src, &mut dst, &opts).map_err(|e| e.to_string())?;
    Ok(dst.into_vec())
}

/// RGB (or RGBA) pixels as a lossy WebP; the transparency, when there is some, kept exactly.
fn encode(pixels: &[u8], w: u32, h: u32, alpha: bool) -> Result<Vec<u8>, String> {
    use libwebp_sys as webp;
    let per = if alpha { 4 } else { 3 };
    if w == 0 || h == 0 || pixels.len() != (w * h * per) as usize {
        return Err("wrong picture size".into());
    }
    // SAFETY: the config and picture are set up by libwebp's own init calls; the pixels are
    // `w * h * per` bytes (checked above), read by the import with that stride; the writer lives
    // until the picture is freed, and its memory is copied out before it is cleared.
    unsafe {
        let mut config = std::mem::MaybeUninit::<webp::WebPConfig>::uninit();
        if webp::WebPConfigInitInternal(config.as_mut_ptr(), webp::WebPPreset::WEBP_PRESET_DEFAULT, QUALITY, webp::WEBP_ENCODER_ABI_VERSION as i32) == 0 {
            return Err("libwebp: no config".into());
        }
        let mut config = config.assume_init();
        config.method = 4;
        config.use_sharp_yuv = 1;
        config.alpha_quality = 100;
        if webp::WebPValidateConfig(&config) == 0 {
            return Err("libwebp: bad config".into());
        }
        let mut pic = std::mem::MaybeUninit::<webp::WebPPicture>::uninit();
        if webp::WebPPictureInitInternal(pic.as_mut_ptr(), webp::WEBP_ENCODER_ABI_VERSION as i32) == 0 {
            return Err("libwebp: no picture".into());
        }
        let mut pic = pic.assume_init();
        pic.width = w as i32;
        pic.height = h as i32;
        // (Imported as RGB, turned into YUV by the encoder itself: the sharp way.)
        pic.use_argb = 1;
        let stride = (w * per) as i32;
        let imported = if alpha { webp::WebPPictureImportRGBA(&mut pic, pixels.as_ptr(), stride) } else { webp::WebPPictureImportRGB(&mut pic, pixels.as_ptr(), stride) };
        if imported == 0 {
            webp::WebPPictureFree(&mut pic);
            return Err("libwebp: import failed".into());
        }
        let mut writer = std::mem::MaybeUninit::<webp::WebPMemoryWriter>::uninit();
        webp::WebPMemoryWriterInit(writer.as_mut_ptr());
        let mut writer = writer.assume_init();
        pic.writer = Some(webp::WebPMemoryWrite);
        pic.custom_ptr = (&mut writer as *mut webp::WebPMemoryWriter).cast();
        let ok = webp::WebPEncode(&config, &mut pic);
        let code = pic.error_code;
        webp::WebPPictureFree(&mut pic);
        let out = if ok != 0 && !writer.mem.is_null() { std::slice::from_raw_parts(writer.mem, writer.size).to_vec() } else { Vec::new() };
        webp::WebPMemoryWriterClear(&mut writer);
        if ok == 0 || out.is_empty() {
            return Err(format!("libwebp: {code:?}"));
        }
        Ok(out)
    }
}

/// An ICC profile for RGB pixels (a profile for CMYK or grey would be wrong on the thumb's RGB).
fn is_rgb_profile(icc: &[u8]) -> bool {
    icc.len() >= 128 && &icc[16..20] == b"RGB " && &icc[36..40] == b"acsp"
}

/// A WebP chunk: its four-letter name, its data.
type Chunk<'a> = (&'a [u8], &'a [u8]);

/// The WebP's chunks after its "RIFF…WEBP" header.
fn chunks(webp: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    if webp.len() < 12 || &webp[0..4] != b"RIFF" || &webp[8..12] != b"WEBP" {
        return Err("not a WebP".into());
    }
    let mut out = Vec::new();
    let mut at = 12;
    while at + 8 <= webp.len() {
        let size = u32::from_le_bytes(webp[at + 4..at + 8].try_into().unwrap()) as usize;
        let end = at + 8 + size;
        if end > webp.len() {
            return Err("cut-short WebP".into());
        }
        out.push((&webp[at..at + 4], &webp[at + 8..end]));
        at = end + (size & 1);
    }
    Ok(out)
}

/// The WebP with the colour profile it was made from (the browser shows its colours as the
/// picture's, a wide-gamut photo included): a VP8X header saying so, then the ICCP chunk.
fn with_icc(webp: &[u8], icc: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    const ICC: u8 = 0x20;
    const ALPHA: u8 = 0x10;
    let parts = chunks(webp)?;
    let mut vp8x = match parts.iter().find(|(n, _)| *n == b"VP8X") {
        Some((_, d)) if d.len() == 10 => d.to_vec(),
        _ => {
            let mut d = vec![0u8; 10];
            d[4..7].copy_from_slice(&(w - 1).to_le_bytes()[..3]);
            d[7..10].copy_from_slice(&(h - 1).to_le_bytes()[..3]);
            if parts.iter().any(|(n, _)| *n == b"ALPH") {
                d[0] |= ALPHA;
            }
            d
        }
    };
    vp8x[0] |= ICC;
    let mut body = Vec::with_capacity(webp.len() + icc.len() + 32);
    body.extend_from_slice(b"WEBP");
    let mut put = |name: &[u8], data: &[u8]| {
        body.extend_from_slice(name);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }
    };
    put(b"VP8X", &vp8x);
    put(b"ICCP", icc);
    for (name, data) in parts.iter().filter(|(n, _)| *n != b"VP8X" && *n != b"ICCP") {
        put(name, data);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Thumbs in `dir` (the thumbs folder) no longer needed: of a picture nothing uses (`used`: the
/// `key`s of every picture file the notes and looks use) that is gone, or older than `before`
/// (the picture itself is removed then: lib.rs `purge_unused_pictures`); and pieces of a thumb
/// left half-written; and what a reading remembered of such a picture (`remember`). A thumb of a
/// picture still used is never one of them.
pub fn unused(dir: &Path, used: &std::collections::HashSet<String>, before: std::time::SystemTime) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let pictures = dir.parent();
    entries
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file()))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let old = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < before);
            if name.ends_with(".part") {
                return old;
            }
            // (What a reading remembered, `remember`: as a thumb of that picture.)
            let marked = name.strip_suffix(NONE).or_else(|| name.strip_suffix(FAILED)).filter(|p| !p.is_empty());
            let Some(picture) = marked.or_else(|| parse(&name).map(|(p, _, _)| p)) else { return false };
            let Some(k) = key(picture) else { return false };
            !used.contains(&k) && (old || pictures.is_some_and(|p| !p.join(picture).exists()))
        })
        .map(|e| e.path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage};

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dobo-thumbs-{tag}-{}-{}", std::process::id(), crate::db::now_ms()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn read(path: &str) -> image::DynamicImage {
        image::ImageReader::open(path).unwrap().with_guessed_format().unwrap().decode().unwrap()
    }

    /// Four quarters of their own colours (top left red, top right green, bottom left blue,
    /// bottom right white): which way a picture was turned shows.
    fn quarters(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, y| match (x < w / 2, y < h / 2) {
            (true, true) => Rgb([220, 30, 30]),
            (false, true) => Rgb([30, 200, 30]),
            (true, false) => Rgb([30, 30, 220]),
            (false, false) => Rgb([250, 250, 250]),
        })
    }

    fn near(a: [u8; 3], b: [u8; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 24)
    }

    fn rgb_at(img: &image::DynamicImage, x: u32, y: u32) -> [u8; 3] {
        let p = img.get_pixel(x, y).0;
        [p[0], p[1], p[2]]
    }

    fn jpeg(img: &RgbImage, exif: Option<u8>, icc: Option<&[u8]>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
        if let Some(icc) = icc {
            enc.set_icc_profile(icc.to_vec()).unwrap();
        }
        enc.write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8).unwrap();
        match exif {
            None => out,
            // An APP1 Exif segment right after the start: a TIFF header and one entry,
            // Orientation (0x0112) = `turn`.
            Some(turn) => {
                let mut tiff = b"II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
                tiff.extend_from_slice(&[turn, 0, 0, 0, 0, 0, 0, 0]);
                let mut app1 = b"Exif\0\0".to_vec();
                app1.extend_from_slice(&tiff);
                let mut with = out[..2].to_vec();
                with.extend_from_slice(&[0xFF, 0xE1]);
                with.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
                with.extend_from_slice(&app1);
                with.extend_from_slice(&out[2..]);
                with
            }
        }
    }

    fn png(img: &RgbaImage) -> Vec<u8> {
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out).write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8).unwrap();
        out
    }

    #[test]
    fn a_thumb_just_covers_the_biggest_box_and_keeps_the_shape() {
        // A photo: as wide as the box.
        assert_eq!(size_for(4000, 3000), Some((1200, 900)));
        // Standing up: as wide as the box too (the box crops it top and bottom).
        assert_eq!(size_for(3000, 4000), Some((1200, 1600)));
        // A panorama: as tall as the box.
        assert_eq!(size_for(8000, 1000), Some((2880, 360)));
        // A long strip only a little taller than the box: as it is.
        assert_eq!(size_for(12000, 400), None);
        // One whose thumb would be longer than a WebP can be: as it is.
        assert_eq!(size_for(40000, 1000), Some((14400, 360)));
        assert_eq!(size_for(50000, 1000), None);
        assert_eq!(size_for(1000, 50000), None);
        // Close to the box's size, or smaller: the picture as it is.
        assert_eq!(size_for(1400, 1000), None);
        assert_eq!(size_for(1200, 900), None);
        assert_eq!(size_for(300, 200), None);
        assert_eq!(size_for(0, 10), None);
        // Each thumb is sharp in the box: at least as wide and as tall as it.
        for (w, h) in [(4000, 3000), (3000, 4000), (8000, 1000), (1601, 1601), (12000, 900), (1700, 6000)] {
            let (tw, th) = size_for(w, h).unwrap();
            assert!(tw >= BOX_W && th >= BOX_H, "{w}x{h} -> {tw}x{th}");
            let (a, b) = (f64::from(w) / f64::from(h), f64::from(tw) / f64::from(th));
            assert!((a - b).abs() / a < 0.01, "{w}x{h} -> {tw}x{th} keeps the shape");
        }
    }

    #[test]
    fn a_thumbs_name_says_its_picture_and_size() {
        assert_eq!(name_for("1716400000000-3.jpg", 1200, 900), "1716400000000-3.jpg.1200x900.webp");
        assert_eq!(parse("1716400000000-3.jpg.1200x900.webp"), Some(("1716400000000-3.jpg", 1200, 900)));
        assert_eq!(parse("1-0.webp.1200x900.webp"), Some(("1-0.webp", 1200, 900)));
        for bad in ["1-0.jpg", "1-0.jpg.webp", ".1200x900.webp", "1-0.jpg.0x900.webp", "1-0.jpg.12x.webp", "1-0.jpg.1200x900.webp.part"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_photo_gets_a_sharp_thumb_of_its_size_once() {
        let dir = temp_dir("photo");
        let pic = dir.join("1-0.jpg");
        std::fs::write(&pic, jpeg(&quarters(1800, 1350), None, None)).unwrap();
        let t = make(&pic).unwrap().expect("a thumb");
        assert_eq!((t.w, t.h), (1200, 900));
        assert_eq!(Path::new(&t.path), dir.join(DIR).join("1-0.jpg.1200x900.webp"));
        let img = read(&t.path);
        assert_eq!(img.dimensions(), (1200, 900));
        assert!(!img.color().has_alpha(), "a photo has no transparency");
        assert!(near(rgb_at(&img, 10, 10), [220, 30, 30]) && near(rgb_at(&img, 1190, 890), [250, 250, 250]));
        // The edge between two colours stays sharp: a few pixels, not a blur.
        let row: Vec<[u8; 3]> = (590..610).map(|x| rgb_at(&img, x, 200)).collect();
        assert!(row.iter().filter(|p| !near(**p, [220, 30, 30]) && !near(**p, [30, 200, 30])).count() <= 4, "{row:?}");
        // Asked again: the one already made, nothing written.
        let at = std::fs::metadata(&t.path).unwrap().modified().unwrap();
        assert_eq!(make(&pic).unwrap(), Some(t.clone()));
        assert_eq!(std::fs::metadata(&t.path).unwrap().modified().unwrap(), at);
        assert_eq!(std::fs::read_dir(dir.join(DIR)).unwrap().count(), 1, "no piece left");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_small_picture_keeps_none_and_a_broken_one_is_an_error() {
        let dir = temp_dir("small");
        let pic = dir.join("1-0.jpg");
        std::fs::write(&pic, jpeg(&quarters(1300, 900), None, None)).unwrap();
        assert_eq!(make(&pic).unwrap(), None);
        let bad = dir.join("2-0.jpg");
        std::fs::write(&bad, b"not a picture at all").unwrap();
        assert!(make(&bad).is_err());
        assert!(make(&dir.join("3-0.png")).is_err(), "a picture that isn't there");
        assert!(existing(&dir.join(DIR)).is_empty(), "no thumb");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A picture that gets no thumb is read once: what was found is remembered beside the
    /// thumbs, never read again (even had it changed); and goes with the picture.
    #[test]
    fn a_picture_without_a_thumb_is_read_once() {
        let dir = temp_dir("once");
        let (small, bad) = (dir.join("1-0.jpg"), dir.join("2-0.jpg"));
        std::fs::write(&small, jpeg(&quarters(1300, 900), None, None)).unwrap();
        std::fs::write(&bad, b"not a picture at all").unwrap();
        assert_eq!(make(&small).unwrap(), None);
        let said = make(&bad).unwrap_err();
        assert!(dir.join(DIR).join("1-0.jpg.none").is_file() && dir.join(DIR).join("2-0.jpg.failed").is_file());
        // (Both now big enough for a thumb: what was found stands, they aren't read again.)
        for p in [&small, &bad] {
            std::fs::write(p, jpeg(&quarters(1800, 1350), None, None)).unwrap();
        }
        assert_eq!(make(&small).unwrap(), None);
        assert_eq!(make(&bad).unwrap_err(), said);
        // A picture that can't be opened just now isn't remembered.
        assert!(make(&dir.join("3-0.jpg")).is_err());
        assert!(!dir.join(DIR).join("3-0.jpg.failed").exists());
        // Found by another version (or by Dobo before the version was noted): read again, by
        // this one; here it reads now, and gets its thumb.
        std::fs::write(dir.join(DIR).join("2-0.jpg.failed"), "Dobo 0.9.0\nit couldn’t be read").unwrap();
        assert!(make(&bad).unwrap().is_some(), "read again");
        // (Its thumb gone again, the picture unreadable again; the marker as Dobo 1.0 wrote it.)
        for f in std::fs::read_dir(dir.join(DIR)).unwrap().flatten() {
            if f.file_name().to_string_lossy().starts_with("2-0.jpg.") {
                std::fs::remove_file(f.path()).unwrap();
            }
        }
        std::fs::write(&bad, b"not a picture at all").unwrap();
        std::fs::write(dir.join(DIR).join("2-0.jpg.failed"), "it couldn’t be read").unwrap();
        assert_eq!(make(&bad).unwrap_err(), said, "read again: unreadable still, and said as it is");
        assert!(std::fs::read_to_string(dir.join(DIR).join("2-0.jpg.failed")).unwrap().starts_with(&failed_by()));
        // The clean-up takes them with their pictures, never a used one's.
        let used: std::collections::HashSet<String> = ["1-0.jpg".to_string()].into();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        let names = |v: Vec<PathBuf>| {
            let mut n: Vec<String> = v.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
            n.sort();
            n
        };
        assert_eq!(names(unused(&dir.join(DIR), &used, later)), ["2-0.jpg.failed"]);
        // Not a day old: only one whose picture is gone; neither used, a day later: both.
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        assert!(unused(&dir.join(DIR), &used, long_ago).is_empty());
        std::fs::remove_file(&bad).unwrap();
        assert_eq!(names(unused(&dir.join(DIR), &used, long_ago)), ["2-0.jpg.failed"], "its picture gone");
        assert_eq!(names(unused(&dir.join(DIR), &Default::default(), later)), ["1-0.jpg.none", "2-0.jpg.failed"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Only what the picture itself is gets remembered: a read that failed just then (the disk,
    /// memory) is tried again next time.
    #[test]
    fn only_the_pictures_own_faults_are_remembered() {
        use image::error::{DecodingError, ImageFormatHint, LimitError, LimitErrorKind};
        let io: Unread = image::ImageError::IoError(std::io::Error::other("the drive is gone")).into();
        assert!(!io.lasting);
        let memory: Unread = image::ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)).into();
        assert!(!memory.lasting);
        let damaged: Unread = image::ImageError::Decoding(DecodingError::new(ImageFormatHint::Unknown, "bad")).into();
        assert!(damaged.lasting);
        let too_big: Unread = image::ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError)).into();
        assert!(too_big.lasting);
        assert!(!Unread::now("libwebp: no config").lasting);
    }

    #[test]
    fn a_turned_photo_is_turned_as_the_browser_shows_it() {
        let dir = temp_dir("turned");
        // Stored lying (2400 × 1800), Exif 6: shown turned a quarter clockwise, standing up.
        let pic = dir.join("1-0.jpg");
        std::fs::write(&pic, jpeg(&quarters(2400, 1800), Some(6), None)).unwrap();
        let t = make(&pic).unwrap().unwrap();
        assert_eq!((t.w, t.h), (1200, 1600));
        let img = read(&t.path);
        // Turned clockwise: the top left (red) is now top right, the bottom left (blue) top left.
        assert!(near(rgb_at(&img, 1190, 10), [220, 30, 30]), "{:?}", rgb_at(&img, 1190, 10));
        assert!(near(rgb_at(&img, 10, 10), [30, 30, 220]), "{:?}", rgb_at(&img, 10, 10));
        assert!(near(rgb_at(&img, 10, 1590), [250, 250, 250]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn transparency_is_kept_and_unused_transparency_dropped() {
        let dir = temp_dir("alpha");
        // Left half clear, right half solid orange.
        let clear = RgbaImage::from_fn(1800, 1350, |x, _| if x < 900 { Rgba([0, 0, 0, 0]) } else { Rgba([240, 140, 20, 255]) });
        let pic = dir.join("1-0.png");
        std::fs::write(&pic, png(&clear)).unwrap();
        let img = read(&make(&pic).unwrap().unwrap().path);
        assert!(img.color().has_alpha());
        let rgba = img.to_rgba8();
        assert_eq!(rgba.get_pixel(100, 100)[3], 0, "clear stays clear");
        assert_eq!(rgba.get_pixel(1100, 100)[3], 255, "solid stays solid");
        assert!(near([rgba.get_pixel(1100, 100)[0], rgba.get_pixel(1100, 100)[1], rgba.get_pixel(1100, 100)[2]], [240, 140, 20]));
        // Next to the edge no dark fringe (colours weighted by their transparency).
        let edge = rgba.get_pixel(601, 400);
        assert!(edge[3] == 0 || near([edge[0], edge[1], edge[2]], [240, 140, 20]), "{edge:?}");
        // A PNG with transparency it never uses: a thumb without.
        let solid = dir.join("2-0.png");
        std::fs::write(&solid, png(&RgbaImage::from_pixel(1800, 1350, Rgba([10, 120, 200, 255])))).unwrap();
        let img = read(&make(&solid).unwrap().unwrap().path);
        assert!(!img.color().has_alpha());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_moving_gif_keeps_moving_and_a_still_one_gets_a_thumb() {
        let dir = temp_dir("gif");
        let gif = |frames: &[Rgba<u8>]| {
            let mut out = Vec::new();
            {
                let mut enc = image::codecs::gif::GifEncoder::new_with_speed(&mut out, 30);
                enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
                for c in frames {
                    enc.encode_frame(image::Frame::new(RgbaImage::from_pixel(1700, 1300, *c))).unwrap();
                }
            }
            out
        };
        // Two frames: it moves, so no thumb (the board shows the GIF itself).
        let moving = dir.join("1-0.gif");
        std::fs::write(&moving, gif(&[Rgba([200, 20, 20, 255]), Rgba([20, 200, 20, 255])])).unwrap();
        assert!(make(&moving).unwrap().is_none());
        assert!(existing(&dir.join(DIR)).is_empty(), "no thumb");
        // One frame: a still picture like any other, made smaller.
        let still = dir.join("2-0.gif");
        std::fs::write(&still, gif(&[Rgba([200, 20, 20, 255])])).unwrap();
        let img = read(&make(&still).unwrap().unwrap().path);
        assert!(near(rgb_at(&img, 50, 50), [200, 20, 20]), "{:?}", rgb_at(&img, 50, 50));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A one-pixel GIF as the image crate writes it.
    fn one_pixel_gif() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = image::codecs::gif::GifEncoder::new(&mut out);
            enc.encode_frame(image::Frame::new(RgbaImage::from_pixel(1, 1, Rgba([200, 20, 20, 255])))).unwrap();
        }
        out
    }

    /// A GIF of a few bytes that says it is 65535 × 65535 (17 GB of pixels once decoded): refused
    /// from its header, never decoded (not even to see whether it moves), and at once.
    #[test]
    fn a_gif_that_says_it_is_huge_is_never_decoded() {
        let dir = temp_dir("bomb");
        let mut bytes = one_pixel_gif();
        // The logical screen's width and height, right after "GIF89a".
        bytes[6..10].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        assert!(bytes.len() < 100, "{} bytes", bytes.len());
        let pic = dir.join("1-0.gif");
        std::fs::write(&pic, &bytes).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(make(&pic), Err("too big to make smaller".into()));
        // A frame that says it is bigger than its 1 × 1 picture: never made room for either.
        let mut frame = one_pixel_gif();
        // (The image descriptor: after the header, the screen, its colour table and any
        // extension blocks.)
        let mut at = 13 + if frame[10] & 0x80 != 0 { 3 << ((frame[10] & 7) + 1) } else { 0 };
        while frame[at] == 0x21 {
            at += 2;
            while frame[at] != 0 {
                at += frame[at] as usize + 1;
            }
            at += 1;
        }
        assert_eq!(frame[at], 0x2c);
        assert_eq!(&frame[at + 5..at + 9], &[1, 0, 1, 0]);
        frame[at + 5..at + 9].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        let odd = dir.join("2-0.gif");
        std::fs::write(&odd, &frame).unwrap();
        assert!(!animated(&odd, Some(image::ImageFormat::Gif)));
        assert_eq!(make(&odd), Ok(None), "1 × 1 as it says: small enough, its frame never decoded");
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_colour_profile_comes_along() {
        let dir = temp_dir("icc");
        // (Only its header matters here: an RGB display profile.)
        let mut icc = vec![0u8; 132];
        icc[0..4].copy_from_slice(&132u32.to_be_bytes());
        icc[12..16].copy_from_slice(b"mntr");
        icc[16..20].copy_from_slice(b"RGB ");
        icc[36..40].copy_from_slice(b"acsp");
        let pic = dir.join("1-0.jpg");
        std::fs::write(&pic, jpeg(&quarters(1800, 1350), None, Some(&icc))).unwrap();
        let t = make(&pic).unwrap().unwrap();
        let bytes = std::fs::read(&t.path).unwrap();
        let mut dec = image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(image::ImageDecoder::icc_profile(&mut dec).unwrap(), Some(icc.clone()));
        assert_eq!(read(&t.path).dimensions(), (1200, 900), "still a whole picture");
        // A CMYK profile isn't the thumb's (its pixels are RGB): left out.
        let mut cmyk = icc.clone();
        cmyk[16..20].copy_from_slice(b"CMYK");
        assert!(!is_rgb_profile(&cmyk));
        // With transparency too: VP8X says both.
        let clear = RgbaImage::from_fn(40, 30, |x, _| Rgba([200, 50, 50, if x < 20 { 0 } else { 255 }]));
        let raw = encode(clear.as_raw(), 40, 30, true).unwrap();
        let both = with_icc(&raw, &icc, 40, 30).unwrap();
        let mut dec = image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(&both)).unwrap();
        assert_eq!(image::ImageDecoder::icc_profile(&mut dec).unwrap(), Some(icc));
        let img = image::load_from_memory(&both).unwrap().to_rgba8();
        assert_eq!((img.get_pixel(5, 5)[3], img.get_pixel(35, 5)[3]), (0, 255));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pictures_get_the_thumbs_of_their_own_folder_only() {
        let dir = temp_dir("attach");
        let pics = dir.join("pictures");
        std::fs::create_dir_all(pics.join(DIR)).unwrap();
        std::fs::write(pics.join(DIR).join(name_for("1-0.jpg", 1200, 900)), b"x").unwrap();
        std::fs::write(pics.join(DIR).join(name_for("2-0.png", 1200, 1600)), b"x").unwrap();
        let pic = |id: i64, path: String| crate::db::Picture { id, note_id: 1, item_id: None, path, sort: 0.0, created_at: 0, thumb: None };
        let mut list = vec![
            // Written another way (letter case, slashes): still that file.
            pic(1, pics.join("1-0.JPG").to_string_lossy().replace('\\', "/")),
            pic(2, pics.join("2-0.png").to_string_lossy().into_owned()),
            pic(3, pics.join("3-0.png").to_string_lossy().into_owned()),
            // Another folder's file of the same name: not this thumb's picture.
            pic(4, dir.join("elsewhere").join("1-0.jpg").to_string_lossy().into_owned()),
        ];
        attach(&mut list, &pics);
        assert_eq!(list[0].thumb.as_ref().map(|t| (t.w, t.h)), Some((1200, 900)));
        assert_eq!(list[1].thumb.as_ref().map(|t| (t.w, t.h)), Some((1200, 1600)));
        assert_eq!(list[1].thumb.as_ref().unwrap().path, pics.join(DIR).join("2-0.png.1200x1600.webp").to_string_lossy());
        assert!(list[2].thumb.is_none() && list[3].thumb.is_none());
        assert!(directly_in(r"C:\Data\pictures\1-0.jpg", Path::new(r"c:/data/pictures/")));
        assert!(!directly_in(r"C:\Data\pictures\thumbs\1-0.jpg", Path::new(r"C:\Data\pictures")));
        assert!(!directly_in(r"C:\Data\pictures-old\1-0.jpg", Path::new(r"C:\Data\pictures")));
        assert!(!directly_in(r"C:\Data\pictures\", Path::new(r"C:\Data\pictures")));
        // Never another stream of a file, nor a device.
        for odd in ["1-0.jpg:hidden", "1-0.jpg::$DATA", "CON", "con.jpg", "NUL.png", "Com1.gif", "lpt9", "COM¹.jpg", "conin$", "AUX .jpg", "."] {
            assert!(!directly_in(&format!(r"C:\Data\pictures\{odd}"), Path::new(r"C:\Data\pictures")), "{odd}");
        }
        for fine in ["console.jpg", "COM10.jpg", "nul-1.png", "1-0.jpg"] {
            assert!(directly_in(&format!(r"C:\Data\pictures\{fine}"), Path::new(r"C:\Data\pictures")), "{fine}");
        }
        // A thumb is made only of a file that is there.
        std::fs::write(pics.join("5-0.jpg"), b"x").unwrap();
        std::fs::create_dir_all(pics.join("6-0.jpg")).unwrap();
        assert!(is_own_picture(&pics.join("5-0.jpg").to_string_lossy(), &pics));
        assert!(!is_own_picture(&pics.join("6-0.jpg").to_string_lossy(), &pics), "a folder");
        assert!(!is_own_picture(&pics.join("7-0.jpg").to_string_lossy(), &pics), "nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_clean_up_never_takes_the_thumb_of_a_picture_still_used() {
        let dir = temp_dir("cleanup");
        let pics = dir.join("pictures");
        let thumbs = pics.join(DIR);
        std::fs::create_dir_all(&thumbs).unwrap();
        for name in ["Used.JPG", "unused-here.jpg"] {
            std::fs::write(pics.join(name), b"x").unwrap();
        }
        for name in ["Used.JPG", "unused-here.jpg", "unused-gone.jpg"] {
            std::fs::write(thumbs.join(name_for(name, 1200, 900)), b"x").unwrap();
        }
        std::fs::write(thumbs.join("Used.JPG.1200x900.webp.part"), b"x").unwrap();
        std::fs::write(thumbs.join("notes.txt"), b"x").unwrap();
        // The notes name it in another letter case: still used.
        let used: std::collections::HashSet<String> = ["used.jpg".to_string()].into();
        let names = |v: Vec<PathBuf>| {
            let mut n: Vec<String> = v.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
            n.sort();
            n
        };
        // Just made (Undo may still bring its note back): only the thumb whose picture is gone.
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        assert_eq!(names(unused(&thumbs, &used, long_ago)), ["unused-gone.jpg.1200x900.webp"]);
        // A day later: every unused one and the half-written piece; never the used one's, nor a
        // file that isn't a thumb.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        assert_eq!(names(unused(&thumbs, &used, later)), ["Used.JPG.1200x900.webp.part", "unused-gone.jpg.1200x900.webp", "unused-here.jpg.1200x900.webp"]);
        let both: std::collections::HashSet<String> = ["used.jpg".to_string(), "unused-here.jpg".to_string()].into();
        assert_eq!(names(unused(&thumbs, &both, later)), ["Used.JPG.1200x900.webp.part", "unused-gone.jpg.1200x900.webp"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
