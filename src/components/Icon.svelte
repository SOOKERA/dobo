<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  let { name, size = 18, stroke = 1.8 }: { name: string; size?: number; stroke?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={stroke} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === "search"}
    <circle cx="11" cy="11" r="6.5" /><path d="M16 16l4 4" />
  {:else if name === "pin"}
    <path d="M12 17v5" /><path d="M9 10.8V4h6v6.8l3 3.2v3H6v-3z" />
  {:else if name === "settings"}
    <path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" />
  {:else if name === "hide-right"}
    <path d="M9 6l6 6-6 6" />
  {:else if name === "hide-left"}
    <path d="M15 6l-6 6 6 6" />
  {:else if name === "chev-right"}
    <path d="M9 6l6 6-6 6" />
  {:else if name === "chev-left"}
    <path d="M15 6l-6 6 6 6" />
  {:else if name === "ontop"}
    <rect x="7" y="9" width="13" height="11" rx="2" /><path d="M4 15V6a2 2 0 0 1 2-2h9" />
  {:else if name === "minimize"}
    <path d="M6 12h12" />
  {:else if name === "help"}
    <circle cx="12" cy="12" r="8.5" /><path d="M9.6 9.6a2.5 2.5 0 0 1 4.9.7c0 1.7-2.5 2.2-2.5 3.7" /><circle cx="12" cy="16.9" r="0.6" fill="currentColor" stroke="none" />
  {:else if name === "close"}
    <path d="M7 7l10 10M17 7L7 17" />
  {:else if name === "more"}
    <circle cx="5" cy="12" r="1.3" fill="currentColor" stroke="none" /><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none" /><circle cx="19" cy="12" r="1.3" fill="currentColor" stroke="none" />
  {:else if name === "check"}
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  {:else if name === "plus"}
    <path d="M12 5v14M5 12h14" />
  {:else if name === "clock"}
    <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" />
  {:else if name === "flip"}
    <path d="M7 4v16" /><path d="M3 8l4-4 4 4" /><path d="M17 20V4" /><path d="M21 16l-4 4-4-4" />
  {:else if name === "trash"}
    <path d="M4 7h16" /><path d="M10 11v6M14 11v6" /><path d="M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12" /><path d="M9 7V4h6v3" />
  {:else if name === "arrow-down"}
    <path d="M12 5v14M6 13l6 6 6-6" />
  {:else if name === "hourglass"}
    <path d="M6 2h12M6 22h12" /><path d="M7 2v4a5 5 0 0 0 10 0V2" /><path d="M7 22v-4a5 5 0 0 1 10 0v4" />
  {:else if name === "bell"}
    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" /><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
  {:else if name === "inbox"}
    <path d="M3 13l3-8h12l3 8v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" /><path d="M3 13h5l1 3h6l1-3h5" />
  {:else if name === "link"}
    <path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1" /><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1" />
  {:else if name === "file"}
    <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /><path d="M14 3v5h5" />
  {:else if name === "image"}
    <rect x="3" y="4" width="18" height="16" rx="2" /><circle cx="9" cy="10" r="1.6" /><path d="M21 16l-5-5-9 9" />
  {:else if name === "copy"}
    <rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
  {:else if name === "external"}
    <path d="M14 4h6v6" /><path d="M20 4l-9 9" /><path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4" />
  {:else if name === "step"}
    <path d="M6 4v8a3 3 0 0 0 3 3h9" /><path d="M14 11l4 4-4 4" />
  {:else if name === "note-plus"}
    <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v9l-6 6H5a1 1 0 0 1-1-1z" /><path d="M14 20v-5a1 1 0 0 1 1-1h5" /><path d="M9 9v5M6.5 11.5h5" />
  {:else if name === "sunset"}
    <path d="M4 18h16" /><path d="M7 18a5 5 0 0 1 10 0" /><path d="M12 5v3M5.6 9.6l1.8 1.8M18.4 9.6l-1.8 1.8M8 21h8" />
  {:else if name === "repeat"}
    <path d="M17 2l4 4-4 4" /><path d="M3 11v-1a4 4 0 0 1 4-4h14" /><path d="M7 22l-4-4 4-4" /><path d="M21 13v1a4 4 0 0 1-4 4H3" />
  {:else if name === "chev-up"}
    <path d="M6 15l6-6 6 6" />
  {:else if name === "chev-down"}
    <path d="M6 9l6 6 6-6" />
  {:else if name === "wide"}
    <rect x="3" y="5" width="18" height="14" rx="2" /><path d="M9 12h6M7 10l-2 2 2 2M17 10l2 2-2 2" />
  {:else if name === "paper"}
    <path d="M6 3h9l4 4v14H6z" /><path d="M9 11h7M9 14h7M9 17h5" />
  {:else if name === "camera"}
    <path d="M4 8a2 2 0 0 1 2-2h2l1.5-2h5L16 6h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /><circle cx="12" cy="12.5" r="3.5" />
  {:else if name === "template"}
    <rect x="4" y="4" width="16" height="16" rx="2" /><path d="M4 9h16M9 9v11" />
  {:else if name === "brush"}
    <path d="M18.4 2.6a2 2 0 0 1 2.9 2.9L12 14.8 9.2 12z" /><path d="M9.2 12c-2.3 0-4.2 1.9-4.2 4.2 0 1.9-1.5 3.3-3 3.8 1.5 1 3.5 1.5 5.2 1.5 3.5 0 5.8-2.3 5.8-5.8z" />
  {:else if name === "refresh"}
    <path d="M20 11a8 8 0 1 0-2.3 5.7" /><path d="M20 4v7h-7" />
  {:else if name === "grid"}
    <rect x="3" y="4" width="7" height="7" rx="2" /><rect x="14" y="4" width="7" height="7" rx="2" /><rect x="3" y="14" width="7" height="7" rx="2" /><rect x="14" y="14" width="7" height="7" rx="2" />
  {:else if name === "sun"}
    <circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  {:else if name === "moon"}
    <path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" />
  {:else if name === "download"}
    <path d="M12 4v11M7 10l5 5 5-5M5 20h14" />
  {:else if name === "upload"}
    <path d="M12 20V9M7 14l5-5 5 5M5 4h14" />
  {:else if name === "rename"}
    <path d="M4 20h4L19 9l-4-4L4 16z" />
  {:else if name === "lines"}
    <path d="M4 7h16M4 12h10M4 17h7" />
  {:else if name === "undo"}
    <path d="M9 14L4 9l5-5" /><path d="M4 9h10a6 6 0 0 1 0 12h-3" />
  {:else if name === "tour"}
    <circle cx="12" cy="12" r="8.5" /><path d="M15.2 8.8l-1.9 4.5-4.5 1.9 1.9-4.5z" />
  {:else if name === "keyboard"}
    <rect x="3" y="6" width="18" height="12" rx="2.2" /><path d="M7 10h.01M10.3 10h.01M13.7 10h.01M17 10h.01M8 14h8" />
  {:else if name === "archive"}
    <rect x="3.5" y="4.5" width="17" height="4.5" rx="1.3" /><path d="M5.5 9v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2V9" /><path d="M10 13h4" />
  {/if}
</svg>
