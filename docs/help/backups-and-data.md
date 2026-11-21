# Backups and your data

Your notes live on your PC, in one folder, and Dobo backs them up every day. No account, nothing to sign in to.

## Where your notes are

**Settings › About** shows which Dobo you have and where its notes are:

- **Installed** (with Dobo-Setup.exe): "Installed on this computer. Updating or reinstalling keeps your notes." The notes are in your Windows user folder, `%APPDATA%\com.sookera.ztodo`.
- **Portable** (Dobo.exe on its own): "Portable: notes, pictures and backups stay in the data folder next to Dobo.exe. Move or copy the whole folder." A portable Dobo uses `%APPDATA%\com.sookera.ztodo` instead when its own folder can't be written to (in Program Files, on a read-only drive), or when notes from an installed Dobo are already there and its `data` folder has none yet. See [Installing and updating](install-and-update.md#dobo-portable).

**Data folder** shows the folder itself; **Open folder** opens it in File Explorer. Inside are your notes, the pictures you added, the link cards' pictures, your background picture and, unless you chose another folder, your backups.

## Daily backups

**Settings › Backups**:

- **Back up every day** (on at first): a copy of your notes and pictures, once a day. The last 14 are kept.
- **Folder**: where the backups go. At first, a `backups` folder inside the data folder. **Choose…** picks another one (a synced or external drive, say); **Default** goes back. Uninstalling Dobo doesn't remove a folder you chose: it's yours, delete it yourself if you want.
- **Back up now** makes one right away.
- **Your backups** lists them, newest first, with why each was made (**Daily**, **By hand**, **Before a restore**) and its size.

A backups folder outside the data folder is safer: it stays even if the data folder is deleted.

## Restore a backup

In **Your backups**, click **Restore…** next to one, then **Restore this** (or **Keep mine** to change your mind). Before restoring, Dobo backs up the notes you have now, so nothing is lost: "Restored. Your notes from before are kept as a backup too."

## Move your notes to another PC

1. On the old PC, click **Back up now**, then copy the backups folder (or just the newest backup inside it) to a USB stick or a shared drive.
2. On the new PC, install Dobo, open **Settings › Backups › Folder › Choose…** and pick the folder you copied.
3. Click **Restore…** next to the backup, then **Restore this**. Your notes and pictures come back as they were.

A portable Dobo is simpler: copy its whole folder, `Dobo.exe` and `data` together.

Your look (themes, styles, mixes, colours) can also travel on its own as a `.ztheme` file: see [Style and themes](style-and-themes.md#share-a-look-or-take-it-to-another-pc).

## Windows Sticky Notes

On the very first start, while there are no notes yet, Dobo copies in your notes from Windows Sticky Notes by itself, without asking. It reads Sticky Notes' database on this PC, from a copy: your Sticky Notes are never changed, and nothing leaves your PC. Bring them in again any time from **Settings › Start › Bring in the notes from Windows Sticky Notes again** (**Import**). Notes already on the board aren't added twice.
