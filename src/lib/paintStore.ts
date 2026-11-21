// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Finished watercolour paintings kept across launches (IndexedDB, in the app's own web data), so
// after a restart a painting shows at once instead of being painted again. Keyed by everything
// that makes the picture (colours, pattern, seed, paper, layout size, pixel size) plus the
// recipe's version; the oldest-used go first past a size cap. Any failure (no IndexedDB, quota,
// private mode) simply means painting as before.

/** Bump when the painting recipe changes: older pictures are then never used (and pruned). */
export const RECIPE = 1;
// (Named before the app was called Dobo; kept, so the paintings already kept are found.)
const DB = "ztodo-paintings";
const MAX_BYTES = 48 * 1024 * 1024;

interface Meta {
  size: number;
  used: number;
}

let opening: Promise<IDBDatabase | null> | null = null;
function db(): Promise<IDBDatabase | null> {
  if (opening) return opening;
  opening = new Promise((resolve) => {
    try {
      const req = indexedDB.open(DB, 1);
      req.onupgradeneeded = () => {
        req.result.createObjectStore("blob");
        req.result.createObjectStore("meta");
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => resolve(null);
      req.onblocked = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
  return opening;
}

function done<T>(req: IDBRequest<T>): Promise<T | undefined> {
  return new Promise((r) => {
    req.onsuccess = () => r(req.result);
    req.onerror = () => r(undefined);
  });
}

const k = (key: string) => `${RECIPE}~${key}`;

/** What the kept paintings weigh, as of the last clean-up plus what was kept since (null: not
 * known yet, or a clean-up is on its way). Past the cap, a clean-up runs in a quiet moment, not
 * only at the next launch. */
let total: number | null = null;
const later = (fn: () => void) => ("requestIdleCallback" in window ? requestIdleCallback(fn, { timeout: 4000 }) : setTimeout(fn, 200));
let warned = false;
/** The cache failing (full disk, quota) is said once in the log; painting goes on as before. */
function warnOnce(e: unknown) {
  if (warned) return;
  warned = true;
  console.warn("Dobo: the paintings cache could not keep a picture (it will be painted again next time).", e);
}

/** Is this painting kept (without loading it)? */
export async function isStored(key: string): Promise<boolean> {
  const d = await db();
  if (!d) return false;
  try {
    return (await done(d.transaction("meta").objectStore("meta").count(k(key)))) === 1;
  } catch {
    return false;
  }
}

/** The kept painting, ready to draw (decoded off the main thread), or null. */
export async function loadStored(key: string): Promise<ImageBitmap | null> {
  const d = await db();
  if (!d) return null;
  try {
    const blob = await done(d.transaction("blob").objectStore("blob").get(k(key)));
    if (!(blob instanceof Blob)) return null;
    const bmp = await createImageBitmap(blob);
    // Remember it was used (for the clean-up), without waiting.
    const tx = d.transaction("meta", "readwrite");
    tx.objectStore("meta").put({ size: blob.size, used: Date.now() } satisfies Meta, k(key));
    return bmp;
  } catch {
    return null;
  }
}

/** Keeps a finished painting, encoded in a quiet moment. */
export function store(key: string, canvas: HTMLCanvasElement) {
  later(() =>
    canvas.toBlob(
      async (blob) => {
        if (!blob) return;
        const d = await db();
        if (!d) return;
        try {
          const tx = d.transaction(["blob", "meta"], "readwrite");
          tx.objectStore("blob").put(blob, k(key));
          tx.objectStore("meta").put({ size: blob.size, used: Date.now() } satisfies Meta, k(key));
          tx.oncomplete = () => {
            if (total == null) return;
            total += blob.size;
            if (total > MAX_BYTES) {
              total = null;
              later(() => void prune());
            }
          };
          tx.onabort = () => warnOnce(tx.error);
        } catch (e) {
          // Full or unavailable: it is only a cache.
          warnOnce(e);
        }
      },
      "image/webp",
      0.94,
    ),
  );
}

/** Which keys to drop: other recipes first, then the least recently used past the cap (pure). */
export function toDrop(entries: { key: string; size: number; used: number }[], maxBytes = MAX_BYTES, recipe = RECIPE): string[] {
  const drop = entries.filter((e) => !e.key.startsWith(`${recipe}~`)).map((e) => e.key);
  const keep = entries.filter((e) => e.key.startsWith(`${recipe}~`)).sort((a, b) => b.used - a.used);
  let total = 0;
  for (const e of keep) {
    total += e.size;
    if (total > maxBytes) drop.push(e.key);
  }
  return drop;
}

/** The clean-up: run once in a quiet moment after launch. */
export async function prune() {
  const d = await db();
  if (!d) return;
  try {
    const os = d.transaction("meta").objectStore("meta");
    const [keys, vals] = await Promise.all([done(os.getAllKeys()), done(os.getAll())]);
    if (!keys || !vals) return;
    const entries = keys.map((key, i) => ({ key: String(key), size: (vals[i] as Meta).size, used: (vals[i] as Meta).used }));
    const drop = toDrop(entries);
    const dropped = new Set(drop);
    total = entries.reduce((sum, e) => sum + (dropped.has(e.key) ? 0 : e.size), 0);
    if (!drop.length) return;
    const tx = d.transaction(["blob", "meta"], "readwrite");
    for (const key of drop) {
      tx.objectStore("blob").delete(key);
      tx.objectStore("meta").delete(key);
    }
  } catch {
    /* nothing to clean */
  }
}
