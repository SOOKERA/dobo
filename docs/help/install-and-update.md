# Installing and updating

Get Dobo, keep it up to date, and remove it if you need to. Dobo runs on Windows 10 and 11.

## Dobo-Setup or Dobo Portable

Dobo comes two ways, from [dobo.sookera.com](https://dobo.sookera.com). Both are the same program and update themselves the same way.

- **Dobo-Setup.exe** installs Dobo for you only (per user), without admin rights, in `%LOCALAPPDATA%\Programs\Dobo`. It puts Dobo in the Start menu (and on the desktop if you like) and in Windows' Apps list, so it can be uninstalled like any app.
- **Dobo.exe** is the portable version: one file. Put it in a folder of its own and run it. It isn't added to the Start menu or the Apps list.

**Settings › About** shows which one you have, **Installed** or **Portable**, next to the version, and where its notes are.

### Dobo Setup's window

- **English | Français**, at the top right: the language of Setup's window. It opens in English for a first install, and in Dobo's own language over an installed Dobo (an update, a reinstall). A click (or Tab, then the arrow keys) changes every word at once and keeps what you've set. When you pick a language there, Dobo then opens in it, its welcome window too; change it any time in **Settings › Appearance › Language**. Leave it as it is and Dobo keeps its own language.
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
- **The folder next to Dobo.exe can't be written** (in Program Files, or on a read-only drive): Dobo asks first, never silently, so your notes are never split. Its start screen says where the notes would be and why they can't go there, then offers **Use the usual folder** (with that folder's path: your notes stay there from then on) or **Quit Dobo**. It asks again at each start while the folder can't be written, unless your notes are in the usual folder by then. Once they are, Dobo keeps using that folder, even after its own folder can be written again: nothing is moved without a word. **Settings › About** then says "Portable, but the folder next to Dobo.exe can't be written: the notes are in your user profile." Dobo can't update itself there either: quit, and move the Dobo folder somewhere you can change files, such as Documents.

Anyone who can open the portable folder can read the notes in it, so keep it in a folder of your own.

### The terms of use, at first start

The portable Dobo shows its terms of use the first time it starts: "Before you start, please read Dobo's terms of use. Dobo starts as soon as you agree." Click **Agree and continue**, or **Quit Dobo**. It asks once, and never again, also not after an update. Until you agree, Dobo doesn't go online at all.

An installed Dobo doesn't ask, since you accepted the terms in Setup's window, except after a silent install (see below), where nobody saw them. If you used Dobo before it had terms, it shows them once too ("Dobo now has terms of use"), and your notes stay as they are.

## "Windows protected your PC"

The first time you run Dobo-Setup.exe or Dobo.exe, Windows may say it protected your PC. Click **More info**, then **Run anyway**.

Dobo's window uses Microsoft Edge WebView2, which Windows 11 and most Windows 10 PCs already have.

## Updates

Open **Settings › Updates**, second in Settings, right after **Appearance**:

- The first line names your copy, "Dobo 1.2.0 · Installed" (or "· Portable"), and says under it where things stand: "Not checked yet." or "Last checked …", "You’re up to date.", "Checking for updates…", or that a new version is out. With the automatic check on, it adds "Dobo looks once a day." Its button, **Check for updates**, looks right away, also with the automatic check off. When a new version is downloaded and checked, the line says "Dobo 1.2.1 is ready. It starts the next time Dobo opens.", with what the new version brings under it, and **Restart now** takes the button's place. The board says it once too, with **Restart now**. When something goes wrong (no connection, a folder Dobo can't change), the reason shows under the line.
- **Check for updates automatically** (on at first): "A new version is downloaded and checked, then used the next time Dobo starts. Your notes are never touched." It looks once a day, also while Dobo stays open in the tray for days. If it can't reach the internet, it tries again an hour later.
- **What’s new**: **Read** shows what this version of Dobo brings. It also shows by itself, once, the first time Dobo opens after an update: every version since the one you had, newest first ("What’s new since Dobo 1.1.0"), with links to the guides on dobo.sookera.com. A new install shows nothing. **Got it** or **‹ Back** returns to where you came from; **×** or **Esc** goes straight to the board. It's in the **?** menu at the board's corner too.

How an update works:

- Dobo downloads the new version and checks it before anything else: it must be signed by Sookera, with the right size and checksum. Anything else is refused, and Settings says why.
- It's used the next time Dobo starts, or right away with **Restart now**.
- The new version runs on trial at first. If it doesn't start properly, the next start puts back the version you had, which keeps working, and Settings says so ("Dobo 1.1.1 didn’t start properly, so Dobo 1.1.0 was put back."). The automatic check doesn't download that version again; **Check for updates** can.
- Only Dobo.exe changes. Your notes, settings and styles are never touched.
- The first time a new version opens your notes, before it adds anything to them, it backs them up as they were: **Settings › Backups** lists that backup under **Before an update**, with the version's number (the last 3 are kept). Dobo makes it at its first start after the update (Setup leaves it to Dobo), once for each version, and quickly: the notes are copied at once; on the same drive, the pictures aren't copied again but linked (a picture file never changes); otherwise (another drive, or a USB stick that can't link files) they're copied in the background once the board is open, and if Dobo is closed before that's done, its next start finishes it. If the backups folder you chose can't be used in time (a network drive that's gone, a slow or full drive, a folder that can't be written to), that backup goes into the `backups` folder inside the data folder instead: **Settings › Backups** says where and why, and lists it with the others ("in the data folder") for as long as it's kept.

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

The uninstaller speaks Dobo's language: the one Setup was in when Dobo was installed, and, once you change it in **Settings › Appearance › Language**, that one (with **Follow Windows**, the language Windows shows). A Dobo installed with an older, English-only Dobo-Setup uninstalls in English until an up-to-date Dobo has started once, or a newer Dobo-Setup is run over it.

1. Open Windows **Settings › Apps › Installed apps**, find Dobo and click **Uninstall**.
2. Turn on **Also delete my notes** only if you want them gone too. Under it: "Your notes, pictures, settings and the backups in Dobo's own folder. A backups folder you chose is kept. This can't be undone." It's off at first, so reinstalling later picks up where you left off. If you might want your notes back one day, keep a backup outside the data folder first (see [Backups and your data](backups-and-data.md)). A backups folder you chose in **Settings › Backups** is never removed by uninstalling: delete it yourself if you want.
3. Confirm. Dobo, its shortcuts and its start with Windows are removed: "Dobo is gone from this PC."

If another Dobo is open, or a portable Dobo uses the same notes (one that kept the notes already on this PC: it leaves a small `.dobo-portable-copies` file in that folder; one on a USB stick that isn't plugged in still counts, for 180 days after it last ran), your notes are kept even with **Also delete my notes** on: "Your notes were kept: another Dobo is open, or a portable Dobo uses them too." Close every Dobo, then delete the `com.sookera.ztodo` folder in `%APPDATA%` yourself if you want them gone.

### Remove the portable Dobo

1. In Dobo, turn off **Settings › Start › Start Dobo with Windows**.
2. Quit Dobo (right-click the tray icon › **Quit Dobo**).
3. Delete Dobo.exe and its `data` folder. If **Settings › About** said its notes were in your user profile, they're in `%APPDATA%\com.sookera.ztodo` (shared with an installed Dobo, if you have one). A backups folder you chose stays until you delete it.

## From Portable to Setup

The installed Dobo keeps its notes in your user folder, not next to the portable one. To take your notes with you:

1. In the portable Dobo, open **Settings › Backups** and click **Back up now**.
2. Quit it, then install Dobo with Dobo-Setup.exe, in another folder than the portable one (Setup asks for another, so the portable's notes stay with it).
3. In the installed Dobo, open **Settings › Backups › Folder › Choose…** and pick the portable's `data\backups` folder.
4. Click **Restore…** next to the newest backup, then **Restore this**.

## Notification buttons

The buttons on Dobo's notifications (**Done**, **10 min**, **Later…**) open a `ztodo:` link that Windows hands to Dobo. The installed Dobo always registers it. A portable Dobo only does when no Dobo has it yet, or when the Dobo that has it is gone: running a portable copy once never takes the buttons from the installed one.

## Free software

Dobo is free software under the GNU GPL version 3. Its source code is on GitHub: [github.com/sookera/dobo](https://github.com/sookera/dobo).
