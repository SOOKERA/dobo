// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// For tests only: the store, over the preview's mock backend (its sample notes), in Node. The
// browser's globals it reads are stood in for, in memory.
import { vi } from "vitest";

export async function testStore() {
  const mem: Record<string, string> = {};
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => mem[k] ?? null,
    setItem: (k: string, v: string) => void (mem[k] = String(v)),
    removeItem: (k: string) => void delete mem[k],
  });
  vi.stubGlobal("location", { search: "" });
  vi.stubGlobal("window", { addEventListener() {}, removeEventListener() {}, matchMedia: () => ({ matches: false, addEventListener() {} }) });
  vi.stubGlobal("document", { documentElement: {}, querySelector: () => null, hasFocus: () => true });
  const { store } = await import("./store.svelte");
  const { backendPromise } = await import("./backend");
  const api = await backendPromise;
  (store as unknown as { api: typeof api }).api = api;
  await store.reload();
  return { store, api };
}
