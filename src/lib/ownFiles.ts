// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The board shows a picture only when its path names a file right inside one of Dobo's own
// folders (the data folder's pictures and their thumbs, previews and backgrounds: every picture
// is copied there), read from the path's text alone. Anything else (a restored backup from
// another computer, a crafted one naming a network share) is never asked for: asking for it would
// make Windows reach that place before refusing it. The same rule as src-tauri/src/thumbs.rs
// `directly_in`; the folders come from lib.rs `own_folders`.

/** Windows' names for devices, whatever the extension (thumbs.rs `is_device`). */
function isDevice(name: string): boolean {
  const stem = name.split(".")[0].replace(/[ .]+$/, "").toUpperCase();
  if (["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].includes(stem)) return true;
  return stem.length === 4 && (stem.startsWith("COM") || stem.startsWith("LPT")) && /[0-9¹²³]/.test(stem[3]);
}

/** A folder or path as compared: backslashes, lower case, no `\\?\` prefix. */
function key(s: string): string {
  let k = s.replace(/\//g, "\\");
  if (k.startsWith("\\\\?\\UNC\\")) k = "\\\\" + k.slice(8);
  else if (k.startsWith("\\\\?\\")) k = k.slice(4);
  return k.toLowerCase();
}

/** Whether `path` is a file right inside one of `folders` (by its text alone; as thumbs.rs
 * `directly_in` and `own_name`: never a name of dots or spaces only, another stream or a
 * device). */
export function ownFile(path: string, folders: readonly string[]): boolean {
  if (!path || path.length >= 1000) return false;
  const p = key(path);
  return folders.some((f) => {
    const dir = key(f).replace(/\\+$/, "");
    if (!dir || !p.startsWith(dir + "\\")) return false;
    const name = p.slice(dir.length + 1);
    return !/^[. ]*$/.test(name) && !/[\\:]/.test(name) && !isDevice(name);
  });
}

/** What the board is given for a picture it won't ask for (an empty image: no request). Each place
 * that shows pictures checks for it and shows a calm placeholder instead of an <img> (the picture
 * icon on the chip colour, `.pic-ph` in app.css), never the browser's broken picture; the Viewer
 * then can't copy it nor open it elsewhere, and a background picture gives way to the import tile
 * and a plain board (look.svelte.ts `usable`). */
export const NOT_OURS = "data:image/gif;base64,";
