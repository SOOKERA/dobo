# Dobo's icons

Made by `node art/icons.mjs` (from the repo root; it needs Playwright's Chromium:
`npx playwright install chromium`). Don't edit them by hand: change the drawings in `art/logo`
and run it again. Each size is drawn from the vector drawing at that size, nothing scaled down.

The owner's choice (2026-11-23): the true logo from 24 px up, the compact note at 16 and 20 px.

| File | Sizes | Drawn from |
|---|---|---|
| `icon.ico` (Dobo.exe, the window, the taskbar, Alt+Tab, the Start menu, Explorer, Setup) | 16, 20 | `art/logo/icon-16.svg`, `icon-20.svg`: the compact note, drawn on whole pixels |
| | 24, 32, 40, 48, 64 (32-bit bitmaps), 256 (PNG) | `art/logo/mark.svg`: the true logo, the block note alone (no tile), cropped to the note so it fills the icon |
| `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png` (256), `icon.png` (512) | | `art/logo/mark.svg` (note only) |
| `Square30x30Logo.png` … `Square310x310Logo.png`, `StoreLogo.png` (50) | | `art/logo/mark.svg` (note only) |
| `icon.icns` (macOS; Dobo ships for Windows only) | 16 | `art/logo/icon-16.svg` |
| | 32 to 1024 | `art/logo/mark.svg` (note only) |

`128x128.png` is also the notifications' icon (`lib.rs`: copied to the data folder as
`notification-icon-dobo.png`, the AppUserModelID's `IconUri`).

`tray/` is not made by this script: the tray's notes (16, 20, 24, 32 px, for light and dark
taskbars) are the compact drawings, from the logo's design files (`tray.rs` picks one).
