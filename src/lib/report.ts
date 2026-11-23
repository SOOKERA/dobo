// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// Help › Report a problem (as Pako's): the report's words, exactly as Copy details copies them
// and Send by email puts them in the message, in the language Dobo shows (the person sending it
// reads it in their email app too). What Send report sends is the same, as fields (report.rs).
import { APP_NAME } from "./app";
import { clockTime, dayMonthYear, t, type Key } from "./i18n.svelte";

export const SUPPORT_EMAIL = "support@sookera.com";
export const DISCUSSIONS_URL = "https://github.com/sookera/dobo/discussions";
/** The service takes no longer description. */
export const DESCRIPTION_MAX = 5000;
/** Longer mailto: links are cut off by some email apps (and Windows). */
const MAILTO_MAX = 2000;

/** Something said: not only spaces or invisible characters (the service's own rule). */
export const hasWords = (s: string): boolean => /[^\s\u200B-\u200F\u2060-\u2064\uFEFF]/u.test(s);

/** One of Dobo's problem notes, as report.rs reads it back (cleaned). */
export interface ReportProblem {
  /** "2026-11-22T10:00:00+03:00": the PC's own time then. */
  time: string;
  area: "crash" | "startup";
  message: string;
  version: string;
}

export interface ReportInfo {
  version: string;
  installed: boolean;
  windows: string;
  /** The newest first, at most 10. */
  problems: ReportProblem[];
  /** How many the notes hold in all. */
  total: number;
}

/** Why Send report didn't go (report.rs `SendFail`). */
export type SendFail = { kind: "offline" } | { kind: "server" } | { kind: "busy"; retryAfter: number } | { kind: "refused"; message: string };

export interface ReportParts {
  info: ReportInfo;
  dark: boolean;
  include: boolean;
}

const AREA: Record<ReportProblem["area"], Key> = { crash: "Stopped (problem)", startup: "Couldn’t start (problem)" };
export const areaLabel = (area: ReportProblem["area"]): string => t(AREA[area] ?? AREA.crash);

/** "UTC+03:00": the PC's offset at that moment, read from the problem's own time. */
export function zoneOf(iso: string): string {
  const m = /([+-]\d{2}:\d{2}|Z)$/.exec(iso);
  return m ? (m[1] === "Z" ? "UTC" : `UTC${m[1]}`) : "";
}

/** "UTC+03:00" now. */
export function zoneNow(d = new Date()): string {
  const m = -d.getTimezoneOffset();
  const a = Math.abs(m);
  return `UTC${m < 0 ? "-" : "+"}${String(Math.floor(a / 60)).padStart(2, "0")}:${String(a % 60).padStart(2, "0")}`;
}

/** The problem's time as the PC's clock said it: "2026-11-22 10:00" (the words of the report). */
export function plainTime(iso: string): string {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(iso) ? `${iso.slice(0, 10)} ${iso.slice(11, 16)}` : iso;
}

/** The same time as Dobo shows dates: "22 Nov 2026, 10:00" (the window's list). */
export function shownTime(iso: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(iso);
  if (!m) return iso;
  const [y, mo, d, h, mi] = m.slice(1).map(Number);
  return t("{date}, {time}", { date: dayMonthYear(new Date(y, mo - 1, d)), time: clockTime(h, mi) });
}

/** The message as the report shows it: another version's problem says which. */
export function problemLine(p: ReportProblem, version: string): string {
  return p.version && p.version !== version ? `${p.message} (${APP_NAME} ${p.version})` : p.message;
}

/** "1.1.1 (Setup)" or "1.1.1 (Portable)". */
export const versionLine = (info: ReportInfo): string => t(info.installed ? "{v} (Setup)" : "{v} (Portable)", { v: info.version });
export const lookLine = (dark: boolean): string => t(dark ? "Dark (look)" : "Light (look)");
export const languageLine = (): string => t("English (the language)");

/** The problems Send report sends: the ones shown, while they're included. */
export function sentProblems({ info, include }: ReportParts): ReportProblem[] {
  return include ? info.problems : [];
}

/** Everything under the description: Dobo, Windows, the language and look, the problems. The
 * time zone only with problems: it goes only inside their times. */
export function details(parts: ReportParts): string {
  const { info, dark } = parts;
  const list = sentProblems(parts);
  const lines = [`${APP_NAME} ${versionLine(info)}`, info.windows, t("Language: {language}", { language: languageLine() }), t("Look: {look}", { look: lookLine(dark) })];
  const zone = list.length ? zoneOf(list[0].time) || zoneNow() : "";
  if (list.length) lines.push(t("Time zone: {zone}", { zone }));
  lines.push("");
  if (!parts.include) lines.push(t("Recent problems: not included."));
  else if (!list.length) lines.push(t("Recent problems: none noted."));
  else {
    lines.push(t("Recent problems (newest first, {n} of {total}):", { n: list.length, total: info.total }));
    for (const p of list) {
      // (A time in another offset than the newest one's says which: a clock change since.)
      const z = zoneOf(p.time);
      const when = z && z !== zone ? `${plainTime(p.time)} (${z})` : plainTime(p.time);
      lines.push(`${when}  ${t("{area}: {message}", { area: areaLabel(p.area), message: problemLine(p, info.version) })}`);
    }
  }
  return lines.join("\r\n");
}

/** The whole report: what happened, then the details. */
export function reportText(description: string, parts: ReportParts): string {
  return `${t("What happened:")}\r\n${description.trim()}\r\n\r\n${details(parts)}\r\n`;
}

export const subject = (version: string): string => t("{app} {v}: problem report", { app: APP_NAME, v: version });

function mailto(subjectText: string, body: string): string {
  return `mailto:${SUPPORT_EMAIL}?subject=${encodeURIComponent(subjectText)}&body=${encodeURIComponent(body)}`;
}

/** The email to open. When the whole report doesn't fit in one mailto: link, `clipboard` is what
 * goes on the clipboard first (the details, or everything if even the words are too long) and
 * the email says to paste it. */
export function mailPlan(description: string, parts: ReportParts): { url: string; clipboard: string | null } {
  const subj = subject(parts.info.version);
  const full = mailto(subj, reportText(description, parts));
  if (full.length <= MAILTO_MAX) return { url: full, clipboard: null };
  const paste = t("The details are on your clipboard: paste them here.");
  const short = mailto(subj, `${t("What happened:")}\r\n${description.trim()}\r\n\r\n${paste}\r\n`);
  if (short.length <= MAILTO_MAX) return { url: short, clipboard: details(parts) };
  return { url: mailto(subj, `${paste}\r\n`), clipboard: reportText(description, parts) };
}

/** What to say when Send report didn't go: the words stay, and email is always there. */
export function sendFailText(e: unknown): string {
  const f = (typeof e === "object" && e ? e : { kind: "other" }) as Partial<SendFail> & { kind?: string };
  switch (f.kind) {
    case "offline":
      return t("Couldn’t reach Sookera. Check your connection and try again, or use Send by email.");
    case "busy": {
      const n = Math.max(1, Math.ceil(Number((f as { retryAfter?: number }).retryAfter ?? 3600) / 60));
      return t("Too many reports from this connection. Try again in {n} minutes, or use Send by email.", { n });
    }
    case "refused":
      return String((f as { message?: string }).message ?? "");
    case "server":
      return t("Sookera’s report service isn’t answering right now. Try again later, or use Send by email.");
    default:
      return t("Couldn’t send the report. Use Send by email instead.");
  }
}
