# Installing and updating

Get Dobo, keep it up to date, and remove it if you need to. Dobo runs on Windows 10 and 11.

## Dobo-Setup or Dobo Portable

Dobo comes two ways, from [dobo.sookera.com](https://dobo.sookera.com). Both are the same program and update themselves the same way.

- **Dobo-Setup.exe** installs Dobo for you only (per user), without admin rights, in `%LOCALAPPDATA%\Programs\Dobo`. It puts Dobo in the Start menu (and on the desktop if you like) and in Windows' Apps list, so it can be uninstalled like any app.
- **Dobo.exe** is the portable version: one file. Put it in a folder of its own and run it. It isn't added to the Start menu or the Apps list.

**Settings › About** shows which one you have, **Installed** or **Portable**, next to the version, and where its notes are.

### Dobo Setup's window

- **Install in**: the folder, `%LOCALAPPDATA%\Programs\Dobo` at first. **Change** picks another one. It must be a folder you can change without admin rights (not Program Files), because Dobo updates itself there: if it isn't, Setup says why and holds the button until you pick another.
- **Desktop shortcut** (on at first).
- **Start Dobo with Windows** (on at first). Reminders only ring while Dobo runs.
- Click **Install**. Installing means you accept Dobo's terms of use, linked under the button ("By installing, you accept the terms of use."), so the installed Dobo doesn't ask again. When it's done, **Open Dobo** starts it.

If Dobo is open, Setup closes it first: your notes are saved as you write, so nothing is lost.

### Installing again after an uninstall

Setup remembers the folder Dobo was installed in. If you uninstalled Dobo and kept your notes, a new install shows that same folder under **Install in** (you can still **Change** it), and your notes are there as you left them. If you uninstalled with **Also delete my notes** on, that folder is forgotten too, and Setup goes back to `%LOCALAPPDATA%\Programs\Dobo`.

### Dobo Portable

The first time it starts, Dobo.exe makes a `data` folder next to itself and keeps everything there: notes, pictures, backups, and its window's own data. Move or copy the whole folder (`Dobo.exe` and `data` together) to take it elsewhere, a USB stick for example.

Two cases where it uses the usual folder in your user profile (`%APPDATA%\com.sookera.ztodo`) instead:

- **This PC already has Dobo notes there** (from an installed Dobo, or an earlier copy) and the `data` folder has none yet: the portable Dobo keeps using them, so your notes are never left behind or split in two. **Settings › About** says "Portable, using the notes already on this computer." A `data` folder that already holds notes is always the one used.
- **The folder next to Dobo.exe can't be written** (in Program Files, or on a read-only drive): "Portable, but the folder next to Dobo.exe can't be written: the notes are in your user profile." Dobo can't update itself there either: move the Dobo folder somewhere you can change files, such as Documents.

Anyone who can open the portable folder can read the notes in it, so keep it in a folder of your own.

### The terms of use, at first start

The portable Dobo shows its terms of use the first time it starts: "Before you start, please read Dobo's terms of use. Dobo starts as soon as you agree." Click **Agree and continue**, or **Quit Dobo**. It asks once, and never again, also not after an update. Until you agree, Dobo doesn't go online at all.

An installed Dobo doesn't ask, since you accepted the terms in Setup's window, except after a silent install (see below), where nobody saw them. If you used Dobo before it had terms, it shows them once too ("Dobo now has terms of use"), and your notes stay as they are.

## "Windows protected your PC"

The first time you run Dobo-Setup.exe or Dobo.exe, Windows may say it protected your PC. Click **More info**, then **Run anyway**.

Dobo's window uses Microsoft Edge WebView2, which Windows 11 and most Windows 10 PCs already have.

## Updates

Open **Settings › Updates**:

- **Check for updates automatically** (on at first): "Once a day. A new version is downloaded and checked, then used the next time Dobo starts. Your notes are never touched." It also checks while Dobo stays open in the tray for days. If it can't reach the internet, it tries again an hour later.
- **Check for updates** looks right away, also with the automatic check off.
- The line next to the button says where things stand. Under it, the version line: "Version 1.0.0 · last checked …" (or "not checked yet"). When the line next to the button only gives the version, the one under it just says "Last checked …" or "Not checked yet". When a new version is downloaded and checked, it says **Dobo {v} is ready** ("It starts the next time Dobo opens, or now with Restart now."), and **Restart now** takes the button's place. The board says it once too, with **Restart now**: "Dobo {v} is ready. It starts the next time Dobo opens."

How an update works:

- Dobo downloads the new version and checks it before anything else: it must be signed by Sookera, with the right size and checksum. Anything else is refused, and Settings says why.
- It's used the next time Dobo starts, or right away with **Restart now**.
- The new version runs on trial at first. If it doesn't start properly, the next start puts back the version you had, which keeps working, and Settings says so ("Dobo 1.0.1 didn't start properly, so Dobo 1.0.0 was put back."). The automatic check doesn't download that version again; **Check for updates** can.
- Only Dobo.exe changes. Your notes, settings and styles are never touched.

With the automatic check off, Dobo never goes online for updates unless you click **Check for updates**.

A development or test build of Dobo (one built from its source code, not downloaded from dobo.sookera.com) never updates itself: **Check for updates** says "This copy of Dobo is a development or test build: it never updates itself."

### A newer Dobo-Setup over an installed Dobo

You can also run a newer Dobo-Setup.exe over the installed Dobo: its window says "Update Dobo", and **Update** updates it in its folder and keeps your notes, settings, shortcuts and taskbar pin.

## Installing without a window

For installing on several PCs, or from a script:

- `Dobo-Setup.exe /S` installs silently, in `%LOCALAPPDATA%\Programs\Dobo` (or the folder Dobo was installed in before), with a desktop shortcut.
- `Dobo-Setup.exe /S /D=C:\Tools\Dobo` installs in another folder. `/D=` comes last, with no quotes, even if the path has spaces.
- The folder must be one this Windows user can change. If it isn't, Setup stops before changing anything, with exit code 2 and the reason in the console.
- A silent install doesn't accept the terms for you: Dobo shows them at its first start. It doesn't set **Start Dobo with Windows** either: on a PC with no Dobo notes yet, Dobo's own first-start setup offers it; otherwise turn it on in **Settings › Start**.
- `uninstall.exe /S`, in Dobo's folder, uninstalls silently. It always keeps your notes: **Also delete my notes** never applies in a silent uninstall.

## Uninstalling

### Uninstall the installed Dobo

1. Open Windows **Settings › Apps › Installed apps**, find Dobo and click **Uninstall**.
2. Turn on **Also delete my notes** only if you want them gone too. Under it: "Your notes, pictures, settings and the backups in Dobo's own folder. A backups folder you chose is kept. This can't be undone." It's off at first, so reinstalling later picks up where you left off. If you might want your notes back one day, keep a backup outside the data folder first (see [Backups and your data](backups-and-data.md)). A backups folder you chose in **Settings › Backups** is never removed by uninstalling: delete it yourself if you want.
3. Confirm. Dobo, its shortcuts and its start with Windows are removed: "Dobo is gone from this PC."

If another Dobo is open (a portable copy), your notes are kept even with **Also delete my notes** on, since it may use the same folder. Close it, then delete the `com.sookera.ztodo` folder in `%APPDATA%` yourself if you want them gone.

### Remove the portable Dobo

1. In Dobo, turn off **Settings › Start › Start Dobo with the computer**.
2. Quit Dobo (right-click the tray icon › **Quit Dobo**).
3. Delete Dobo.exe and its `data` folder. If **Settings › About** said its notes were in your user profile, they're in `%APPDATA%\com.sookera.ztodo` (shared with an installed Dobo, if you have one). A backups folder you chose stays until you delete it.

## From Portable to Setup

The installed Dobo keeps its notes in your user folder, not next to the portable one. To take your notes with you:

1. In the portable Dobo, open **Settings › Backups** and click **Back up now**.
2. Quit it, then install Dobo with Dobo-Setup.exe, in another folder than the portable one (Setup asks for another, so the portable's notes stay with it).
3. In the installed Dobo, open **Settings › Backups › Folder › Choose…** and pick the portable's `data\backups` folder.
4. Click **Restore…** next to the newest backup, then **Restore this**.

## Free software

Dobo is free software under the GNU GPL version 3. Its source code is on GitHub: [github.com/sookera/dobo](https://github.com/sookera/dobo).
