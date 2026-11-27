# Tasks and ideas

A task is something to do: it has a checkbox and can have a time. An idea is something to keep. Both are lines in a note, and one can become the other any time.

## The entry line

Click the entry line at the top (it's ready as soon as Dobo opens), type, then:

- **Enter** adds a **task**;
- **Shift+Enter** adds an **idea**.

Before you press anything, the line under the field shows how Dobo reads you: task or idea, the time it found, and the note it goes into. Without a **#note**, the line goes to the Inbox.

You can write a time ("tomorrow 9am", "demain 15h"), a deadline (end with **!**) or a repeat ("every Monday") right in the line. See [Times and reminders](times-and-reminders.md).

## Send a line to a note with #

Add **#** and the start of a note's name: "Milk #shop" goes into "Shopping".

As soon as you type **#**, a small list of your notes shows under the entry line, and narrows as you type:

- The best match is lit. **↑** / **↓** choose another one.
- **Space** completes the lit note's name and closes the list (**Tab** and **Enter** do too, once you've used **↑** / **↓**).
- **Esc** closes the list and keeps your text.
- A name of several words is written with hyphens: "Grand Palais" becomes **#grand-palais**.

You don't have to use the list. Press **Enter** and Dobo still finds the note loosely: the whole name first, then a name that starts with your word, then one that contains it, ignoring capitals, accents and punctuation. When nothing matches, Dobo makes a new note with that name (the list says so: **New note "holidays"**).

To write several lines into one note, open its menu and choose **Write into this note**. The entry line shows "Into Shopping" until you click its **×**.

## + Add in a note

Every note ends with **+ Add**. Click it, or click the empty space of a note, and type:

- **Enter** adds a task, **Shift+Enter** an idea, right in that note.
- Times, repeats and **!** work as in the entry line.
- It stays open for the next line. **Esc** or a click elsewhere closes it, and what you typed is added, never lost.

## Work with a line

- **Tick a task**: click its checkbox, or select it and press **Space**. A ticked task stays in its place; to gather done tasks at the end of a note, choose **Move done tasks to the end** in its menu. Tick a note's last open task and the note says **All done** (see [The board](the-board.md)).
- **Edit a line**: double-click it (or select it and press **Enter** or **F2**). **Enter** saves, **Esc** leaves.
- **Delete a line**: its menu › **Delete**, or select it and press **Delete**.
- **Move a line**: press on its words and drag it onto another note, onto a task (it becomes a step) or onto the Inbox strip. **Esc** while carrying sends it back.
- **Make a line into a new note**: drag it onto **+ New note**, the dashed card that waits at the end of the board while you carry a line (the note grows in there), or onto any empty spot of the board (the note appears where you drop it, then settles under the note above). Or choose **Make a new note of it** in the line's menu: the same as the dashed card. The note takes the line's words as its title (cut at about 60 characters) and the line goes inside, whole: its time, steps, link and pictures. It works for any line, from the Inbox or a note, a task, an idea or a step. One **Ctrl+Z** puts the line back where it was and takes the note away.
- **Give a task a time**: while you carry it, a dashed strip, **Drop here for a time**, shows just under the Inbox. Drag the task onto it and today's hours open there, over the notes: drop it on one (see [The hours ruler](the-hours-ruler.md)).
- **A task's menu** (**⋯** on the line, or right-click it): its time, **Pick on today's hours**, **Repeat**, **Reminder**, **History…** (a repeating task's, see [Times and reminders](times-and-reminders.md#repeats)), **Add a step**, **Copy text**, **Make it an idea**, **Make a new note of it**.
- **An idea's menu** (right-click it): **Copy text**, **Make it a task**, **Make a new note of it**, **Delete**.
- **Copy text** copies the line's words, with its link's address and its file's path each on a line of their own; a short "Copied" says it's done.

## Task or idea

- On an idea, click **Make it a task**.
- On a task, open its menu › **Make it an idea**.
- With the keyboard: select the line and press **Backspace**. While editing, **Backspace** at the very start of the line does the same.

A task that becomes an idea leaves its time behind.

## Dates written in a note

When you edit a task and write a day and a time in it ("Call the client tomorrow 9am"), the date gets a dotted line and a **Remind?** offer. **Enter** adds the reminder, **Esc** leaves it. In **Settings › Notes › Dates written in a note** choose **Ask** (as described), **Automatic** (only when the date is certain, a day and an hour like "Monday 2:30pm") or **Off**.

## Pictures and files

- **Paste a picture** (**Ctrl+V**) with the mouse over a note: it joins that note's pictures. Pasted into the entry line or the dump bar, it comes in as a new line in the Inbox.
- **Drop files** from File Explorer onto a note. Pictures join the note's pictures; other files become a line with a file card. Click the card to open the file. Dobo never starts programs or scripts from a card: it shows them in their folder instead. When a file can't open (moved, deleted), the card says why.
- **A file on another computer** (a shared folder, a NAS): the first time, the card asks once for that computer, "Files on nas open from the network. Allow?" **Allow** opens it, and that computer's files open from then on; **Not now** leaves it. (Opening it sends your Windows sign-in to that computer: see [Privacy](privacy.md#files-on-another-computer).)
- A note shows one picture large, with arrows when there are more. Double-click it for the viewer: **←** / **→** to browse, **Copy picture**, **Open in Photos**, **Remove**. **Esc** closes.
- A picture whose file isn't in Dobo's own folders (from a backup that came without it, for example) isn't looked for anywhere else: Dobo never reaches out to another place on its own. A quiet picture icon stands in its place, and in the viewer **Copy picture** and **Open in Photos** are dimmed, their tip saying "This picture isn’t in Dobo’s folders". A picture whose file is gone or damaged gets the same icon and dimmed buttons, its tip saying "This picture can’t be read"; Dobo tries it again when you come back to its window, or after a restore.
