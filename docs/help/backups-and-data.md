# Backups and your data

Your notes live on your PC, in one folder, and Dobo backs them up every day. No account, nothing to sign in to.

## Where your notes are

**Settings › About** shows which Dobo you have and where its notes are:

- **Installed** (with Dobo-Setup.exe): "Installed on this computer. Updating or reinstalling keeps your notes." The notes are in your Windows user folder, `%APPDATA%\com.sookera.ztodo`.
- **Portable** (Dobo.exe on its own): "Portable: notes, pictures and backups stay in the data folder next to Dobo.exe. Move or copy the whole folder." A portable Dobo uses `%APPDATA%\com.sookera.ztodo` instead when notes from an installed Dobo are already there and its `data` folder has none yet. When its own folder can't be written to (in Program Files, on a read-only drive), it asks first: see [Installing and updating](install-and-update.md#dobo-portable).

**Data folder** shows the folder itself; **Open folder** opens it in File Explorer. Inside are your notes, the pictures you added, the link cards' pictures, your background picture and, unless you chose another folder, your backups. The `pictures\thumbs` folder holds smaller copies of your pictures, which the board shows so it opens quickly; Dobo makes them again whenever they are missing.

## Daily backups

**Settings › Backups**:

- **Back up every day** (on at first): a copy of your notes and pictures, once a day. The last 14 are kept. (The smaller copies in `pictures\thumbs` aren't in backups: after a restore, Dobo makes them again, a few at a time.)
- **Folder**: where the backups go. At first, a `backups` folder inside the data folder, and a quiet line under it says so (as it does for any folder on the same drive or network share as your notes): "Kept with your notes, on the same drive. A folder on another drive keeps them safe if this one fails." **Choose…** picks another one (a synced or external drive, say); **Default** goes back. Uninstalling Dobo doesn't remove a folder you chose: it's yours, delete it yourself if you want.
- **Back up now** makes one right away.
- **Your backups** lists them by kind, newest first, each with its date and size. A backup made before an update that couldn't go into the folder you chose (see [Install and update](install-and-update.md)) is listed too, "in the data folder".

Each kind keeps its own last few, so making backups by hand never pushes the daily ones out:

| Kind | Made | Kept |
|---|---|---|
| **Daily** | once a day | the last 14 |
| **By hand** | with **Back up now** | the last 10 |
| **Before a restore** | by **Restore…**, of the notes you had | the last 5 |
| **Before an update** | when a new version of Dobo first opens your notes, before it adds anything to them ("For Dobo 1.1.0", with the version's number) | the last 3 |

A backups folder outside the data folder is safer: it stays even if the data folder is deleted, or the drive fails.

## Restore a backup

In **Your backups**, click **Restore…** next to one, then **Restore this** (or **Keep mine** to change your mind). Before restoring, Dobo backs up the notes you have now, so nothing is lost: "Restored. Your notes from before are kept as a backup too."

The backup brings back its notes, pictures and settings, not its days: an evening's look back you already closed stays closed, and **Tidy** doesn't ring again for today. Neither does it bring back the computers you allowed file cards to open from (see [Tasks and ideas](tasks-and-ideas.md#pictures-and-files)), nor its backups folder (a backup from another PC keeps this one's), nor what it said about backups in its time.

If you type while a restore runs, what you typed goes into the "Before a restore" backup too. Should that backup's drive be gone just then, Settings › Backups says where those changes are kept.

## If your notes don't open

Rarely, the notes file can't be opened when Dobo starts: a disk error damaged it, another program holds it, or it can't be written. Dobo then shows a calm screen saying what happened, instead of not starting:

- **Your notes file is damaged.** Nothing is renamed until you choose. Dobo first tries a repair: when everything it can still read goes into a new file, **Use the repaired notes** comes first, with how many notes and lines it holds (a repair may miss some, so compare it with the backup's). Then **Try again** reads the file again (a drive that answered badly once may read it whole now), **Put back the newest backup** (its date and how many notes and lines it holds are shown: the newest one that is whole), or **Start with an empty board** (your settings come along when they can still be read). Each of these first puts the damaged file aside, as it was, as `broken-<date>.sqlite` in the data folder: nothing is deleted, and the pictures it names are kept while it's there. If the choice can't be done, the file goes back in its place. **Quit Dobo** leaves it all as it is, and the screen comes back at the next start.
- **Your notes can be put back**: an earlier start put your notes aside because they couldn't be read, and they read whole now. **Put my notes back** carries on where you left off.
- **Another program is using your notes** (often an antivirus, a sync or a backup tool): nothing is renamed. Wait a moment, then **Try again**.
- **Your notes can't be saved**: Windows lets Dobo read the file but not change it (the file or its folder is read-only). Untick **Read-only** in the file's Properties, then **Try again**.
- **Your notes didn't open**, for another reason: **Try again** first; then you can put back the newest backup or start with an empty board, and the file is put aside either way (and back in its place if that can't be done).
- **The board couldn't start**: what you chose worked, but the board didn't come in. **Try again**; if it still doesn't start, quit Dobo and open it again. Your notes are kept as they are.

**Open the data folder** shows the folder in File Explorer. **Up** / **Down** move between the choices, **Enter** picks one.

## Move your notes to another PC

1. On the old PC, click **Back up now**, then copy the backups folder (or just the newest backup inside it) to a USB stick or a shared drive.
2. On the new PC, install Dobo, open **Settings › Backups › Folder › Choose…** and pick the folder you copied.
3. Click **Restore…** next to the backup, then **Restore this**. Your notes and pictures come back as they were.

A portable Dobo is simpler: copy its whole folder, `Dobo.exe` and `data` together.

Your look (themes, styles, mixes, colours) can also travel on its own as a `.ztheme` file: see [Style and themes](style-and-themes.md#share-a-look-or-take-it-to-another-pc).

## Windows Sticky Notes

On the first start, if Windows Sticky Notes has notes, the welcome window asks: "Bring in your 4 Windows Sticky Notes?" Nothing comes in until you click **Import**. **Not now** leaves them where they are, and Dobo doesn't ask again. Either way, you can bring them in any time from **Settings › Start › Bring in the notes from Windows Sticky Notes** (**Import**). Dobo reads Sticky Notes' database on this PC, from a copy: your Sticky Notes are never changed, and nothing leaves your PC. Notes already on the board aren't added twice.
