# Privacy

Dobo keeps your notes on your PC. There's no account, no sign-in, no tracking, and nothing you write leaves your computer.

## What stays on your PC

Everything you put in Dobo: notes, tasks, ideas, steps, pictures, files you drop in (Dobo keeps a link to them, they stay where they are), your styles and settings, and the backups. They're in the data folder shown in **Settings › About**, or, for the backups, in the folder you chose for them (see [Backups and your data](backups-and-data.md)). A portable Dobo keeps them in the `data` folder next to Dobo.exe, or in some cases in your user profile (see [Installing and updating](install-and-update.md#dobo-portable)).

Dobo has no analytics, no crash reports and no "send a report" button: it never sends anything about you or your notes.

## When Dobo goes online

**Settings › About** puts it in one sentence: "Dobo goes online only to fetch the title and picture of a web page you dump, and once a day to check for updates (unless you turn that off). Your notes stay on this computer."

Nothing goes online before you accept Dobo's terms of use: in Dobo Setup's window, or at the portable Dobo's first start (see [Installing and updating](install-and-update.md#the-terms-of-use-at-first-start)). Until then, no update check and no link card; everything else works as usual.

1. **Link cards for the pages you dump.** When the quick dump bar keeps the link to the web page you were reading, Dobo fetches that page once, to show its title and picture on a small card. The picture is saved next to your notes, so the card shows offline. Only that page and its picture are fetched (the picture can come from another site, as the page says): your line's words are never sent. Dobo never fetches addresses on your own network (a router, a printer). Click **Leave it out** in the bar and nothing is fetched.
2. **Updates.** Once a day, Dobo fetches `latest.json` from [github.com/sookera/dobo-releases](https://github.com/sookera/dobo-releases), where its releases are published, to see whether there's a new version. Only when there is does it fetch `Dobo.exe` from there too. It says which version of Dobo is asking, and nothing about you or your notes. You can turn this off in **Settings › Updates** (**Check for updates automatically**): then Dobo checks only when you click **Check for updates**.

That's all. Nothing else goes online: not your notes, not your searches, not your settings.

## What Dobo reads on your PC

- **Windows Sticky Notes, on the very first start.** The first time Dobo starts, while it has no notes yet, it copies in your notes from Windows Sticky Notes by itself, without asking. It reads Sticky Notes' own database on this PC, from a copy, and nothing leaves your PC. Your Sticky Notes are never changed. You can bring them in again any time with **Settings › Start › Bring in the notes from Windows Sticky Notes again** (**Import**); notes already on the board aren't added twice.
- **The window in front, when the quick dump bar opens.** Each time you open the bar (**Ctrl+Shift+D**, or **+ Dump** in the tray menu), Dobo reads the title of the window in front. When that window is a web browser, it also reads the page's address from the address bar, through Windows UI Automation (Windows' accessibility feature). This stays on your PC: it's only used to offer the link in the bar. Only a link you keep is fetched, for its title and picture (see above). **Leave it out** drops it.
- **Files you drop or paste** onto the board.

## Notifications

Reminders are shown by Windows' own notifications. They stay on your PC.

## Remove everything

- **Installed**: uninstall Dobo from Windows **Settings › Apps** with **Also delete my notes** on. If you chose a backups folder in **Settings › Backups**, uninstalling doesn't remove it: it's yours, delete it yourself if you want it gone.
- **Portable**: delete Dobo.exe and its `data` folder. If **Settings › About** says its notes are in your user profile, they're in `%APPDATA%\com.sookera.ztodo`. A backups folder you chose stays until you delete it.
