// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

// Tauri expects a fixed port and must not watch src-tauri.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    // (Nor the development build's notes and WebView2 data in ./data: src-tauri/src/paths.rs.)
    watch: { ignored: ["**/src-tauri/**", `${process.cwd().replace(/\\/g, "/")}/data/**`] },
  },
  // The page goes to build/ (tauri.conf.json frontendDist), never dist/: dist/<version>/ holds
  // the release files (scripts/release.ps1), which must not end up inside Dobo.exe.
  build: {
    outDir: "build",
    target: "es2022",
    cssMinify: true,
  },
});
