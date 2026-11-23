# Times and reminders

Write a time the way you'd say it, and Dobo reminds you. End a line with **!** and it becomes a deadline that starts early and insists a little more as it gets close.

## A time in plain words

Write it in the line, in English or French (Dobo understands both, whatever its own language): "Call Anna tomorrow 4pm", "dentist friday 9am", "rapport lundi 14h30", "3pm", "demain 10h".

- The line under the field shows how Dobo read it before you press **Enter**: "Reminds you at the time · end with ! for a deadline" (in a narrow window, just "Deadline: end with !").
- The time is taken out of your words and shown as a small pill on the task.
- A day without an hour ("tomorrow") means 09:00. A part of the day keeps its hour: "tonight", "this afternoon", "demain soir", "ce soir" (today's, even once it has started).
- "at 3" alone (no am or pm, no minutes) means 15:00, from 1 to 6. With an "h" it is as written: "à 3h" is 03:00.
- French "midi", "minuit" and "après-demain" work too.
- Ideas don't take a time. Make it a task first if you want one.

## A deadline: end with **!**

"Send the report friday 5pm!" is a deadline. The hint says "Deadline: rings early, then more often", and its pill has a small hourglass.

A deadline follows a **rhythm**. **Normal**, the first one, starts 60 minutes before, rings again at 30, 15 and 5 minutes before, then every 5 minutes after the time, 6 times at most. **Gentle** is calmer, **Insistent** stronger, and **Custom** is yours.

- Change the default in **Settings › Reminders › Default rhythm for deadlines**.
- Change one task: its menu › **Kind of reminder**: **Rings at the time** or **The rhythm**, and the rhythm's minutes.

A plain reminder rings at the time, then every 5 minutes until you answer, 6 times at most.

## Set or change a time later

- **The task's menu** (**⋯**): type a **New time** ("tomorrow 10am", "Friday 3pm") and click **Set**, or **Remove the time**.
- **The hours ruler**: drag the task onto an hour, or **Pick on today's hours** in its menu (see [The hours ruler](the-hours-ruler.md)).
- **Tidy**: the **When?** field.
- **While editing**: see "Dates written in a note" in [Tasks and ideas](tasks-and-ideas.md).

## Repeats

Write it in the line: "water the plants every Monday", "gym every Mon and Thu 7pm", "stand-up every weekday 9h30", "pay rent every 1st", "every 2 weeks", "every other week", "tous les jours 8h". The first one is set for the next matching day, and the pill shows a repeat mark.

Or use the task's menu › **Repeat**: **Never**, **Day**, **Weekday**, **Week** (pick the days) or **Month** (a **Day of the month**: the 29th, 30th or 31st falls on the last day of shorter months); **Every how many**; and when it **Ends**: **Never**, **After** a number of times, or **On a date** (its **Last day**). Ticking a repeating task makes the next one, at the same time of day. Ticked late, it goes to the next one still to come.

A repeat keeps its own time when the clocks change: one at 02:30 rings at 02:30 again after the spring change. On the night 02:30 doesn't exist, it rings at 03:00, the first minute after it; on the night 02:30 comes twice, it rings at the first one.

## Answer a reminder

A reminder comes as a Windows notification, and shows on the **Now card** at the top of Dobo. Answer either one:

- **Done** ticks the task.
- **10 min** (or **1 hour**, on a plain reminder) rings again later.
- **Later…** opens Dobo with more choices: **1 hour**, **This evening**, **Tomorrow morning**, **Next Monday**, or **Pick a time** ("tomorrow 2pm", "Friday 9am").
- **I'm on it** (a deadline): Dobo stops ringing, and a little later asks "Is "Send the report" really done?" with **Yes, done** and **Not yet**.

When several ring at once (or came due while the computer slept), Windows shows one notification for all of them: "3 reminders", with the first ones' words and **Open Dobo**. Answer each one on the Now card, **10 min** and **Later…** included: it shows **2 more**, click it to see the next one. A reminder that rings alone keeps its own notification and buttons.

Unticking a task whose time has passed (or undoing **Done**) waits a minute before it rings again, so it never rings the moment you change your mind.

"This evening" and "Tomorrow morning" follow **Settings › My day** (08:00 to 22:00 at first). Still up after midnight, "Tomorrow morning" is the morning coming, a few hours away.

## Steps

A step with a time rings like any task. Tick the task it belongs to, and its steps stop ringing; their times are kept. Untick the task, and they ring again, except a step whose time came while the task was done: it stays quiet until you give it a new time. See [Steps](steps.md).

## 12 or 24 hours

Dobo shows times the way Windows does: "15:00", or "3:00 PM" when Windows uses a 12-hour clock (**Windows Settings › Time & language › Language & region › Regional format**). That covers the board, the Now card, the hours ruler, the time pills, Settings, the look back and the notifications, and it follows a change in Windows at once. The words stay in Dobo's own language. Typed times work the same either way: "15h", "3pm", "tomorrow 9am".

## Reminders only ring while Dobo runs

Closing the window keeps Dobo in the tray, so reminders still ring. **Quit Dobo** stops them. Keep **Settings › Start › Start Dobo with Windows** on so it's always there, and let Windows show Dobo's notifications.

If the computer was asleep or off at the time, the reminder rings once when Dobo is back (not once for each ring it missed), and says when it was due: "Was due yesterday at 22:00". When several were due meanwhile, they come together in one notification.

A reminder from the other space (Work or Personal) still rings: the board switches to it for that reminder, with a button to go back. An archived note's reminders stay quiet until you restore it.
