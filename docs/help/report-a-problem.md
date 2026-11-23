# Report a problem

When something goes wrong in Dobo, tell Sookera in a few words. Nothing goes until you click, and you see everything that goes with your words first.

## Sending a report

1. Open **Report a problem**: in the **?** menu at the board's corner (**F1**), or with **Report…** in **Settings › Help**.
2. In **What happened?**, say what you did, what you expected, and what happened instead.
3. Check **Sent with your report**. It shows everything that goes with your words:
   - Dobo's version, and whether it's installed (with Dobo-Setup.exe) or portable;
   - your Windows version;
   - the language Dobo shows, and its look (light or dark);
   - while **Include recent problems** is on, the last problems Dobo noted (up to 10), each with its time and your PC's time zone then.
4. Pick how to send it:
   - **Send report** (or **Ctrl+Enter**): sends what's shown to Sookera's report service. When it's sent, Dobo shows the report's number (like **#42**): mention it if you write to support@sookera.com about the same problem. If it can't go (no connection, too many reports from your connection, or the service isn't answering), Dobo says so and keeps your words: try again later, or send it by email. You can close the page while it sends: a message then says whether it went.
   - **Send by email**: your email app opens a new message to support@sookera.com with everything filled in. Check it, then send it. If the report is too long for one email link, Dobo puts the details on your clipboard first, and the message says where to paste them.
   - **Copy details**: for email in your browser (Gmail, Outlook on the web). Paste them into a new email to support@sookera.com.

**Send report** works once you've accepted Dobo's terms of use (see [Installing and updating](install-and-update.md#the-terms-of-use-at-first-start)). **Send by email** and **Copy details** always work.

## Where a report goes

**Send report** goes to Sookera's report service (reports.sookera.com), which files it in a private place on GitHub that only Sookera can read. Reports are used only to fix Dobo. Your IP address isn't stored: to stop anyone from flooding the service, it counts reports with a code that changes every day and can't be turned back into your address.

A report never carries your notes, their titles, your pictures or files, your links or your settings.

## The recent problems

Dobo writes a short line in two small files in its data folder (see [Backups and your data](backups-and-data.md#where-your-notes-are)) when it has to stop because of an error (`crash.log`), or when it can't start (`startup-error.log`). Each line says when (with your PC's time zone then), which version of Dobo, and what the error was. These files stay on your PC: a line leaves it only in a report you send yourself, and only while **Include recent problems** is on.

Before a line is written, and again before it's sent, Dobo takes out what could say who you are or what you wrote:

- your Windows user folder reads `%USERPROFILE%`, and your user name on its own reads `…`;
- folder and file names read `…`, and a file keeps only its type (`%USERPROFILE%\…\….txt`);
- anything in quotes (straight, curly or « », and backticks) reads `…`, so an error that quotes a note's words never shows them; only Rust's own names, like `Option::unwrap()`, stay;
- links keep only their site (`https://example.com`); email addresses and long codes are removed.

What stays says where the error happened in Dobo itself (like `src\db.rs:120`).

## Ideas and questions

For an idea, a wish or a question rather than a problem, **Share it on GitHub Discussions** at the bottom of the page opens [github.com/sookera/dobo/discussions](https://github.com/sookera/dobo/discussions). What you write there is public.
