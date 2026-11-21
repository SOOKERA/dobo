<p align="center">
  <a href="https://dobo.sookera.com"><img src="https://dobo.sookera.com/dobo.svg" width="96" height="96" alt="Dobo"></a>
</p>

<h1 align="center">Dobo</h1>

<p align="center">
  To-dos &amp; Idea board: tasks and ideas on a board of notes,<br>
  like sticky notes that remember times.
</p>

<p align="center">
  <a href="https://dobo.sookera.com"><b>Download for Windows</b></a> ·
  <a href="https://dobo.sookera.com/help">Help</a> ·
  <a href="https://dobo.sookera.com/help/whats-new">What's new</a> ·
  <a href="https://github.com/sookera/dobo/discussions">Discussions</a> ·
  <a href="https://sookera.com/terms">Terms</a> ·
  <a href="https://sookera.com/privacy">Privacy</a>
</p>

Free for Windows 10 and 11. No account, no ads, no tracking: your notes stay on your computer.

## What it does

- **Write it now, sort it later.** Type a line and press Enter, or press **Ctrl+Shift+D** in any app for the
  quick dump bar. What has no place yet waits in the Inbox, and **Tidy** helps you put it away one card at a time.
- **Times in plain words.** "Call Anna tomorrow 3pm", "dentist friday 9am", "water the plants every Monday":
  Dobo understands English and French, and reminds you with Windows notifications you can answer (Done, Later…).
- **Drag a task onto the clock.** Carry a line onto today's or tomorrow's hours to give it a time.
- **Notes that hold tasks and ideas.** Tasks you tick, ideas you keep, steps inside a task with a ring that fills,
  `#note` to file a line as you type it, pictures and files dropped in.
- **One shortcut away.** Docked at the edge of the screen or floating, it opens and hides with
  **Ctrl+Shift+Space** and waits in the tray.
- **A board that feels like yours.** Paper, cork, linen, terrazzo, watercolour and more, your own styles, light and
  dark themes that switch with Windows or by the hour, and themes you can share.
- **Look back, search and archive.** A short look at your day, search across every note, and an archive to put
  notes away without deleting them.
- **Your notes are safe.** A backup every day, restored in one click. Windows Sticky Notes come in with one click too
  (their notes are never changed).
- **English and French.**

## Get it

From [dobo.sookera.com](https://dobo.sookera.com) or the
[latest release](https://github.com/sookera/dobo-releases/releases/latest):

- **Dobo-Setup.exe** installs Dobo for you only, without admin rights (in `%LOCALAPPDATA%\Programs\Dobo`), with a
  Start menu entry and, if you like, a desktop shortcut and Start with Windows.
- **Dobo.exe** is the portable version: one file that runs from any folder and keeps its notes in a `data` folder
  beside it.

Both update themselves: once a day Dobo looks for a new version, checks its signature, SHA-256 and size, and uses it
the next time it starts (or now, with **Restart now** in Settings › Updates). A new version that doesn't start
properly is replaced by the one before. Your notes are never touched. The daily check can be turned off in
Settings › Updates.

The first time, Windows may say it protected your PC: choose **More info**, then **Run anyway**.

Dobo goes online only to fetch the title and picture of a web page you dump, and once a day to check for updates.

## This repository

It holds a copy of each release's source code. Contributions aren't accepted (pull requests and issues are off).
Ideas and questions are welcome in [Discussions](https://github.com/sookera/dobo/discussions), or write to
support@sookera.com.

<details>
<summary><b>Building it yourself</b></summary>

You need Node.js, Rust and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for Windows.

```
npm install
npm run dev                         # the interface alone in a browser, with sample notes
npx tauri dev                       # the real app, while working on it
npx tauri build --no-bundle         # Dobo.exe in src-tauri/target/release
npx tauri build                     # Dobo.exe and Dobo Setup (src-tauri/installer, from Sookera's installer template)
npx vitest run                      # tests (interface)
cd src-tauri && cargo test          # tests (notes, backups, reminders, Windows parts)
```

A development build keeps its notes in a `data` folder at the project's root, never with your real notes.

</details>

## Licence

Dobo is free software under the **GNU General Public License version 3** (`LICENSE`), with additional
terms (`NOTICE.md`):

- keep the credit "Dobo by Sookera" (a changed version says "Based on Dobo by Sookera");
- mark changed versions as changed, never as the original or as made by Sookera;
- the names "Dobo" and "Sookera" and their logos aren't part of the licence: a version you share uses
  its own name and icon.

See also the [terms of use](TERMS.md). Copyright (C) 2026 Sookera.

<p align="center"><sub>Dobo by <a href="https://sookera.com">Sookera</a></sub></p>
