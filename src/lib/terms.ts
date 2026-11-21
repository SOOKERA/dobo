// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// TERMS.md as blocks for the terms screen (Terms.svelte): the little Markdown it uses (# and ##
// headings, paragraphs, "- " lists, **bold**, [links](…)), never as HTML.

export interface Part {
  text: string;
  bold?: boolean;
  url?: string;
}
export type Block = { kind: "h1" | "h2"; text: string } | { kind: "p" | "li"; parts: Part[] };

/** One line's words: plain, **bold** and [linked](url) parts, in order. */
export function inline(s: string): Part[] {
  const out: Part[] = [];
  const re = /\*\*(.+?)\*\*|\[([^\]]+)\]\(([^)\s]+)\)/g;
  let at = 0;
  for (const m of s.matchAll(re)) {
    if (m.index! > at) out.push({ text: s.slice(at, m.index) });
    if (m[1] !== undefined) out.push({ text: m[1], bold: true });
    else out.push({ text: m[2].replace(/\*\*/g, ""), url: m[3] });
    at = m.index! + m[0].length;
  }
  if (at < s.length) out.push({ text: s.slice(at) });
  return out;
}

export function parseTerms(md: string): Block[] {
  const blocks: Block[] = [];
  let para: string[] = [];
  let item: string[] | null = null;
  const flush = () => {
    if (para.length) blocks.push({ kind: "p", parts: inline(para.join(" ")) });
    if (item) blocks.push({ kind: "li", parts: inline(item.join(" ")) });
    para = [];
    item = null;
  };
  for (const raw of md.replace(/^﻿/, "").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) {
      flush();
      continue;
    }
    const h = /^(#{1,6})\s+(.*)$/.exec(line);
    if (h) {
      flush();
      blocks.push({ kind: h[1].length === 1 ? "h1" : "h2", text: h[2] });
      continue;
    }
    if (line.startsWith("- ")) {
      flush();
      item = [line.slice(2)];
      continue;
    }
    // A line right under a list item (indented) goes on with it.
    if (item && /^\s/.test(raw)) {
      item.push(line);
      continue;
    }
    if (item) flush();
    para.push(line);
  }
  flush();
  return blocks;
}
