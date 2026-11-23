// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
import { describe, expect, it } from "vitest";
import { judgeAudit, readAuditOutput, SEVERITIES } from "./check-npm-audit.mjs";

// An npm audit --json report (version 2) listing these packages, its counts made to match.
function report(vulnerabilities = {}) {
  const counts = Object.fromEntries(SEVERITIES.map((s) => [s, 0]));
  for (const v of Object.values(vulnerabilities)) counts[v.severity]++;
  counts.total = Object.keys(vulnerabilities).length;
  return { auditReportVersion: 2, vulnerabilities, metadata: { vulnerabilities: counts, dependencies: {} } };
}
const advisory = (name, severity, ghsa) => ({
  name,
  severity,
  isDirect: false,
  via: [{ name, dependency: name, title: `${name} advisory`, url: `https://github.com/advisories/${ghsa}`, severity, range: "<2" }],
  effects: [],
  range: "<2",
  nodes: [`node_modules/${name}`],
  fixAvailable: true,
});

// What npm audit said of Dobo's development tools with vitest 3.2.7 (trimmed): tinypool critical,
// @vitest/mocker moderate, and vitest critical through both. Nothing in what ships.
const vitest3 = report({
  "@vitest/mocker": { ...advisory("@vitest/mocker", "moderate", "GHSA-82fw-gwwq-j7x9"), range: "<=4.1.11" },
  tinypool: { ...advisory("tinypool", "critical", "GHSA-5gmw-xhrv-c9v3"), range: "<=2.1.1" },
  vitest: { ...advisory("vitest", "critical", "GHSA-82fw-gwwq-j7x9"), via: ["@vitest/mocker", "tinypool"], isDirect: true },
});

describe("judging npm audit's reports", () => {
  it("passes a clean tree, with nothing to report", () => {
    expect(judgeAudit(report(), report())).toEqual({ ok: true, unreadable: false, failures: [], warnings: [] });
  });

  it("stops on high or critical in the development tools, and only reports moderate", () => {
    const v = judgeAudit(vitest3, report());
    expect(v.ok).toBe(false);
    expect(v.unreadable).toBe(false);
    expect(v.failures).toHaveLength(2);
    expect(v.failures[0]).toMatch(/^tinypool \(critical, development tools only, versions <=2\.1\.1\): .*GHSA-5gmw-xhrv-c9v3/);
    expect(v.failures[1]).toMatch(/^vitest \(critical, development tools only.*through @vitest\/mocker; through tinypool$/);
    expect(v.warnings).toHaveLength(1);
    expect(v.warnings[0]).toMatch(/^@vitest\/mocker \(moderate, development tools only/);
  });

  it("stops on a high one in the development tools", () => {
    const tools = report({ rollup: advisory("rollup", "high", "GHSA-aaaa") });
    expect(judgeAudit(tools, report()).ok).toBe(false);
  });

  it("only reports moderate, low and info in the development tools", () => {
    const tools = report({
      a: advisory("a", "moderate", "GHSA-a"),
      b: advisory("b", "low", "GHSA-b"),
      c: advisory("c", "info", "GHSA-c"),
    });
    const v = judgeAudit(tools, report());
    expect(v.ok).toBe(true);
    expect(v.failures).toEqual([]);
    expect(v.warnings).toHaveLength(3);
  });

  it("stops on any vulnerability in what ships, even low or info", () => {
    for (const severity of SEVERITIES) {
      const ships = report({ "chrono-node": advisory("chrono-node", severity, "GHSA-b") });
      const v = judgeAudit(ships, ships);
      expect(v.ok, severity).toBe(false);
      expect(v.failures, severity).toEqual([expect.stringMatching(new RegExp(`^chrono-node \\(${severity}, ships with Dobo`))]);
      expect(v.warnings, severity).toEqual([]);
    }
  });

  it("names a package that ships once, as shipping, not again as a development tool", () => {
    const pkg = advisory("html-to-image", "moderate", "GHSA-c");
    const v = judgeAudit(report({ "html-to-image": pkg, tinybench: advisory("tinybench", "low", "GHSA-d") }), report({ "html-to-image": pkg }));
    expect(v.failures).toEqual([expect.stringMatching(/^html-to-image \(moderate, ships with Dobo/)]);
    expect(v.warnings).toEqual([expect.stringMatching(/^tinybench \(low, development tools only/)]);
  });

  it("refuses a report that isn't version 2, in either run", () => {
    for (const version of [1, 3, "2", undefined]) {
      const other = { ...report(), auditReportVersion: version };
      for (const [full, prod] of [[other, report()], [report(), other]]) {
        const v = judgeAudit(full, prod);
        expect(v.ok, String(version)).toBe(false);
        expect(v.unreadable, String(version)).toBe(true);
        expect(v.failures.join(" "), String(version)).toMatch(/only version 2 is read/);
      }
    }
  });

  it("refuses a report that doesn't add up, never reading it as clean", () => {
    const tinypool = advisory("tinypool", "critical", "GHSA-5gmw-xhrv-c9v3");
    const broken = [
      null,
      [],
      { auditReportVersion: 2 },
      { auditReportVersion: 2, vulnerabilities: {} },
      { ...report({ tinypool }), vulnerabilities: {} },
      { ...report(), vulnerabilities: { tinypool } },
      report({ tinypool: { ...tinypool, severity: "severe" } }),
      report({ tinypool: "critical" }),
      { ...report({ tinypool }), metadata: { vulnerabilities: { ...report({ tinypool }).metadata.vulnerabilities, total: 0 } } },
    ];
    for (const r of broken) {
      const v = judgeAudit(r, report());
      expect(v, JSON.stringify(r)).toMatchObject({ ok: false, unreadable: true });
      expect(judgeAudit(report(), r), JSON.stringify(r)).toMatchObject({ ok: false, unreadable: true });
    }
  });

  it("keeps control characters from the advisories out of the console", () => {
    const sneaky = { ...advisory("evil", "low", "GHSA-e"), via: [{ title: "a\u001b[2Jb\r\nc", url: "https://x" }] };
    const [line] = judgeAudit(report({ evil: sneaky }), report({ evil: sneaky })).failures;
    expect(line).toBe("evil (low, ships with Dobo, versions <2): a [2Jb c https://x");
  });
});

describe("reading what npm audit printed", () => {
  it("reads a report", () => {
    const text = JSON.stringify(vitest3, null, 2);
    expect(readAuditOutput(text)).toEqual({ kind: "report", report: vitest3 });
  });

  it("hands on a report of another version, for judgeAudit to refuse", () => {
    expect(readAuditOutput('{"auditReportVersion": 3}')).toEqual({ kind: "report", report: { auditReportVersion: 3 } });
  });

  it("knows a registry that can't be reached", () => {
    // What npm 11 prints offline (connection refused, then no DNS).
    const refused = {
      message: "request to http://127.0.0.1:9/-/npm/v1/security/advisories/bulk failed, reason: connect ECONNREFUSED 127.0.0.1:9",
      error: { summary: "", detail: "" },
    };
    const noDns = { message: "request to https://registry.npmjs.org/-/npm/v1/security/advisories/bulk failed, reason: getaddrinfo ENOTFOUND registry.npmjs.org", error: { summary: "", detail: "" } };
    expect(readAuditOutput(JSON.stringify(refused))).toEqual({ kind: "offline", message: refused.message });
    expect(readAuditOutput(JSON.stringify(noDns)).kind).toBe("offline");
    expect(readAuditOutput(JSON.stringify({ error: { code: "ETIMEDOUT", summary: "timed out" } })).kind).toBe("offline");
  });

  it("tells another failure from being offline", () => {
    expect(readAuditOutput(JSON.stringify({ message: "audit endpoint returned an error", error: { code: "E500" } }))).toEqual({
      kind: "error",
      message: "audit endpoint returned an error E500",
    });
    expect(readAuditOutput(JSON.stringify({ error: {} }))).toEqual({ kind: "error", message: "npm audit failed, saying nothing more." });
    expect(readAuditOutput("").kind).toBe("error");
    expect(readAuditOutput("npm ERR! something").kind).toBe("error");
    expect(readAuditOutput("[]").kind).toBe("error");
    expect(readAuditOutput("null").kind).toBe("error");
  });
});
