// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
//
// The npm dependencies' known vulnerabilities (npm audit: GitHub's advisories, fetched fresh), for
// scripts/release.ps1, next to cargo audit and cargo deny:
//   - in what ships with Dobo (dependencies, npm audit --omit=dev), ANY vulnerability stops a
//     release, whatever its severity;
//   - in the development tools only (devDependencies), high and critical stop it; moderate, low
//     and info are only reported.
// npm audit's report must be version 2 (auditReportVersion): another one is refused, never guessed.
//
//   node scripts/check-npm-audit.mjs      (Node 24.2 or newer: import.meta.main; from any folder)
//
// Exit code: 0 passed (perhaps with warnings), 1 a vulnerability stops the release, 2 a report
// that can't be read or judged (or npm failed), 3 the npm registry couldn't be reached (offline?),
// 4 Node is older than 24.2.
// readAuditOutput and judgeAudit are pure (scripts/check-npm-audit.test.mjs).
import { spawnSync } from "node:child_process";
import { join } from "node:path";

export const SEVERITIES = ["info", "low", "moderate", "high", "critical"];
// In the development tools only, these stop a release; the others are only reported.
const STOPS_IN_DEV = new Set(["high", "critical"]);
// The error codes of a registry that can't be reached: offline, no DNS, refused, timed out.
const OFFLINE = /\b(ENOTFOUND|EAI_AGAIN|ECONNREFUSED|ECONNRESET|ECONNABORTED|ETIMEDOUT|ESOCKETTIMEDOUT|ENETUNREACH|ENETDOWN|EHOSTUNREACH|ENOTCACHED)\b/;

const isObject = (v) => typeof v === "object" && v !== null && !Array.isArray(v);
// The report's words come from the advisory database: no control characters (escape sequences)
// reach the console.
const clean = (v) => String(v ?? "").replace(/[\u0000-\u001f\u007f-\u009f]+/g, " ").trim();

/**
 * What `npm audit --json` printed: a report, a registry that couldn't be reached, or an error.
 * @param {string} text
 * @returns {{kind: "report", report: object} | {kind: "offline", message: string} | {kind: "error", message: string}}
 */
export function readAuditOutput(text) {
  if (!String(text ?? "").trim()) return { kind: "error", message: "npm audit printed nothing." };
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    return { kind: "error", message: "npm audit's output isn't JSON." };
  }
  if (!isObject(json)) return { kind: "error", message: "npm audit's output isn't a report." };
  if (!("auditReportVersion" in json) && ("error" in json || "message" in json)) {
    const err = isObject(json.error) ? json.error : {};
    const words = [json.message, err.code, err.summary, err.detail].map(clean).filter(Boolean).join(" ");
    const message = words || "npm audit failed, saying nothing more.";
    return OFFLINE.test(words) ? { kind: "offline", message } : { kind: "error", message };
  }
  return { kind: "report", report: json };
}

// Why a report can't be judged, or "" when it can.
function unreadable(report) {
  if (!isObject(report)) return "isn't a report.";
  if (report.auditReportVersion !== 2) {
    return `is version ${JSON.stringify(report.auditReportVersion) ?? "undefined"} (auditReportVersion), and only version 2 is read: check what changed in npm audit --json, then update scripts/check-npm-audit.mjs.`;
  }
  if (!isObject(report.vulnerabilities)) return "has no list of vulnerabilities.";
  const counts = isObject(report.metadata) ? report.metadata.vulnerabilities : undefined;
  if (!isObject(counts)) return "has no vulnerability counts (metadata.vulnerabilities).";
  const found = Object.fromEntries(SEVERITIES.map((s) => [s, 0]));
  for (const [name, v] of Object.entries(report.vulnerabilities)) {
    if (!isObject(v) || !SEVERITIES.includes(v.severity)) return `has "${clean(name)}" with no known severity.`;
    found[v.severity]++;
  }
  for (const s of SEVERITIES) {
    if (counts[s] !== found[s]) return `counts ${JSON.stringify(counts[s])} ${s} but lists ${found[s]}.`;
  }
  const total = Object.keys(report.vulnerabilities).length;
  if (counts.total !== total) return `counts ${JSON.stringify(counts.total)} in all but lists ${total}.`;
  return "";
}

// One vulnerable package, on one line: its advisories, or the packages it is vulnerable through.
function describe(name, v, where) {
  const via = (Array.isArray(v.via) ? v.via : []).map((x) =>
    isObject(x) ? [clean(x.title), clean(x.url)].filter(Boolean).join(" ") : `through ${clean(x)}`,
  );
  const range = v.range ? `, versions ${clean(v.range)}` : "";
  return `${clean(name)} (${v.severity}, ${where}${range})${via.length ? ": " + via.join("; ") : ""}`;
}

/**
 * Judges the two reports of the same tree: `npm audit --json --include=dev` (everything) and
 * `npm audit --json --omit=dev` (only what ships).
 * @param {unknown} full
 * @param {unknown} prod
 * @returns {{ok: boolean, unreadable: boolean, failures: string[], warnings: string[]}}
 */
export function judgeAudit(full, prod) {
  const failures = [];
  for (const [label, report] of [["npm audit's report", full], ["npm audit --omit=dev's report", prod]]) {
    const why = unreadable(report);
    if (why) failures.push(`${label} ${why}`);
  }
  if (failures.length) return { ok: false, unreadable: true, failures, warnings: [] };
  const warnings = [];
  const ships = prod.vulnerabilities;
  for (const [name, v] of Object.entries(ships)) failures.push(describe(name, v, "ships with Dobo"));
  for (const [name, v] of Object.entries(full.vulnerabilities)) {
    if (Object.hasOwn(ships, name)) continue;
    (STOPS_IN_DEV.has(v.severity) ? failures : warnings).push(describe(name, v, "development tools only"));
  }
  return { ok: failures.length === 0, unreadable: false, failures, warnings };
}

// Runs npm audit --json in the project (whatever the current folder), and reads what it printed.
function runAudit(include) {
  // (One command line, no argument list: npm is npm.cmd on Windows, run through the shell.)
  const run = spawnSync(`npm audit --json ${include}`, {
    cwd: join(import.meta.dirname, ".."),
    shell: true,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    windowsHide: true,
  });
  if (run.error) return { kind: "error", message: `Couldn't run npm: ${clean(run.error.message)}` };
  // (npm audit's exit code is 1 whenever it finds anything: the report says what.)
  const read = readAuditOutput(run.stdout);
  if (read.kind === "error" && run.stderr.trim()) {
    read.message += " npm said: " + clean(run.stderr.trim().split(/\r?\n/).slice(-3).join(" "));
  }
  return read;
}

function main() {
  console.log("Checking the npm dependencies for known vulnerabilities (npm audit)...");
  const full = runAudit("--include=dev");
  const prod = full.kind === "report" ? runAudit("--omit=dev") : full;
  for (const read of [full, prod]) {
    if (read.kind === "offline") {
      console.error(`npm audit couldn't reach the npm registry (offline?): ${read.message}`);
      return 3;
    }
    if (read.kind === "error") {
      console.error(`npm audit didn't give a report: ${read.message}`);
      return 2;
    }
  }
  const verdict = judgeAudit(full.report, prod.report);
  for (const w of verdict.warnings) console.log(`  reported (doesn't stop a release): ${w}`);
  if (verdict.unreadable) {
    for (const f of verdict.failures) console.error(f);
    return 2;
  }
  if (!verdict.ok) {
    for (const f of verdict.failures) console.error(`  stops the release: ${f}`);
    console.error(
      `npm audit: ${verdict.failures.length} known ${verdict.failures.length === 1 ? "vulnerability stops" : "vulnerabilities stop"} the release (any in what ships with Dobo; high or critical in the development tools).`,
    );
    return 1;
  }
  console.log(
    verdict.warnings.length
      ? `npm audit: nothing that stops a release (${verdict.warnings.length} moderate or low in the development tools only, above).`
      : "npm audit: no known vulnerability in the npm dependencies.",
  );
  return 0;
}

// import.meta.main says whether this file was run (not imported): Node 24.2 or newer. Older ones
// leave it undefined, and this script refuses rather than guess (comparing import.meta.url with
// process.argv[1] goes wrong through a junction).
if (import.meta.main === undefined) {
  console.error(`scripts/check-npm-audit.mjs needs Node 24.2 or newer (this is ${process.version}): update Node.`);
  process.exitCode = 4;
} else if (import.meta.main) {
  process.exitCode = main();
}
