<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { composing } from "./lib/compose";
  import Header from "./components/Header.svelte";
  import Capture from "./components/Capture.svelte";
  import NowCard from "./components/NowCard.svelte";
  import SpaceBanner from "./components/SpaceBanner.svelte";
  import NextLine from "./components/NextLine.svelte";
  import HoursRuler from "./components/HoursRuler.svelte";
  import Board from "./components/Board.svelte";
  import InboxStrip from "./components/InboxStrip.svelte";
  import Settings from "./components/Settings.svelte";
  import Search from "./components/Search.svelte";
  import Tidy from "./components/Tidy.svelte";
  import Viewer from "./components/Viewer.svelte";
  import FirstRun from "./components/FirstRun.svelte";
  import Terms from "./components/Terms.svelte";
  import { watchUpdates } from "./lib/updates.svelte";
  import SizeControl from "./components/SizeControl.svelte";
  import Toast from "./components/Toast.svelte";
  import LookBack from "./components/LookBack.svelte";
  import Templates from "./components/Templates.svelte";
  import Archive from "./components/Archive.svelte";
  import Peek from "./components/Peek.svelte";
  import StickyDates from "./components/StickyDates.svelte";
  import BoardBackground from "./components/style/BoardBackground.svelte";
  import StylePanel from "./components/style/StylePanel.svelte";
  import Tour from "./components/Tour.svelte";
  import { helpMenu, tour } from "./lib/tour.svelte";
  import { inMemory } from "./lib/sample";
  import { look } from "./lib/look.svelte";
  import { heldBy, lookIsDark } from "./lib/looks";
  import { store, SIZES } from "./lib/store.svelte";
  import { inTauri } from "./lib/backend";
  import { boardCovered, closeTopLayer, layerBack, layerCount, layerForward, pushLayer } from "./lib/layers";
  import { once, type NavSource } from "./lib/nav";
  import { gridKey } from "./lib/gridkeys";
  import { boardKey, liveLine, undoKey } from "./lib/keys";
  import { itemDrag, type Back, type Landing } from "./lib/itemdrag.svelte";
  import { viewer } from "./lib/viewer.svelte";
  import { fileDrop, onFileDrop } from "./lib/filedrop.svelte";
  import { warmStyle } from "./lib/warm";
  import { installWindowDrag } from "./lib/windrag";
  import { arrive, fadeQuick, pop, rise } from "./lib/motion";
  import Icon from "./components/Icon.svelte";
  import { t, type Key } from "./lib/i18n.svelte";

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");

  let shown = $state(false);
  let shownAt = 0;
  let settingsOpen = $state(false);
  /** A section Settings opens on (Help or Keyboard, from the "?" menu or F1). */
  let settingsAt = $state<{ key: Key; n: number } | null>(null);
  let searchOpen = $state(false);
  let tidyOpen = $state(false);
  let onboarding = $state(false);
  let termsOpen = $state(false);
  let captureInput = $state<HTMLInputElement>();
  let scroller = $state<HTMLElement>();
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let hoverNote: number | null = null;

  let floating = $derived(store.win.mode === "floating");
  let left = $derived(store.win.side === "left");

  function appear() {
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = null;
    shown = false;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        shown = true;
        shownAt = Date.now();
        tourNews();
        // (Not while the tour shows: the keyboard stays in its bubble.)
        if (!onboarding && !searchOpen && !tidyOpen && !tour.open) setTimeout(() => !tour.open && captureInput?.focus(), reduce.matches ? 0 : 120);
      }),
    );
  }

  function hide() {
    if (!shown) return;
    tour.close();
    shown = false;
    helpMenu.open = false;
    settingsOpen = false;
    searchOpen = false;
    store.archiveOpen = null;
    viewer.open = false;
    hideTimer = setTimeout(() => store.backend.hidePanel(), reduce.matches ? 0 : 240);
  }

  // The window is moved by the top of whatever is open (floating, and the first-run window):
  // the views' click-away handlers tell a move from a click with isWindowDrag.
  $effect(() => installWindowDrag(() => store.backend.startDragging()));

  // The Style panel's pictures are made ahead in idle moments (first open as smooth as any),
  // and again when what it shows changes.
  $effect(() => {
    if (!store.loaded) return;
    void [look.styles, look.themes, look.mixes, look.wc, look.board, look.boardW, look.boardH];
    warmStyle();
  });

  $effect(() => {
    store.init().then(() => {
      onboarding = !store.settings.onboarded;
      setUpBefore = !onboarding;
      // The terms of use, once, before anything goes online (an installed copy: accepted in
      // Setup). In the browser preview only with ?terms.
      termsOpen = !store.settings.terms_accepted && (inTauri || new URLSearchParams(location.search).has("terms"));
      watchUpdates();
      store.backend.onEvent("panel-shown", async () => {
        store.peek = false;
        await store.refreshWindow();
        appear();
      });
      store.backend.onEvent("panel-hide-request", () => hide());
      // Something asked for elsewhere (a reminder, a notification, the tray) ends the tour first:
      // it would open under its veil, and could reach the tour's sample line.
      store.backend.onEvent("open-tidy", () => {
        tour.close();
        settingsOpen = false;
        searchOpen = false;
        tidyOpen = true;
      });
      store.backend.onEvent("focus-item", (id) => {
        tour.close();
        const it = store.items.find((i) => i.id === id);
        if (it) store.focusNote(it.note_id);
      });
      store.backend.onEvent("focus-note", (id) => {
        tour.close();
        if (typeof id === "number" && id > 0) store.focusNote(id);
      });
      store.backend.onEvent("later-item", (id) => (tour.close(), (store.laterFor = id as number)));
      // The mouse's side buttons when mouse software sends them as app commands.
      store.backend.onEvent("nav", (dir) => step(dir === "forward" ? "forward" : "back", "app"));
      store.backend.onEvent("open-lookback", () => (tour.close(), (store.lookbackOpen = true)));
      store.backend.onEvent("peek-shown", () => {
        tour.close();
        shown = false;
        store.peek = true;
      });
      window.addEventListener("dobo-tidy", () => (tour.close(), (tidyOpen = true), (settingsOpen = false), (searchOpen = false)));
      // The window closed or minimised (its × hides it without the slide-out): the tour ends too.
      document.addEventListener("visibilitychange", () => document.hidden && tour.close());
      window.addEventListener("dobo-tour", startTour);
      window.addEventListener("dobo-archive", (e) => openArchive((e as CustomEvent).detail === "settings" ? "settings" : "board"));
      window.addEventListener("dobo-help", (e) => openHelp((e as CustomEvent).detail === "Keyboard" ? "Keyboard" : "Help"));
      if (!inTauri && new URLSearchParams(location.search).has("peek")) store.peek = true;
      store.backend.onFocusChanged((focused) => {
        if (!focused && !onboarding && store.win.mode === "docked" && !store.pinned && shown && Date.now() - shownAt > 400 && !itemDrag.item && !fileDrop.over) hide();
      });
      store.backend.onFileDrop((e) => void onFileDrop(e));
      appear();
    });
  });

  function typing(): boolean {
    const a = document.activeElement as HTMLElement | null;
    return !!a && (a.tagName === "INPUT" || a.tagName === "TEXTAREA" || a.isContentEditable);
  }

  /** "Ctrl+Shift+F" matches the key pressed. */
  function matches(e: KeyboardEvent, accel: string): boolean {
    const parts = accel.split("+").map((p) => p.trim().toLowerCase());
    const key = parts[parts.length - 1];
    const want = (m: string) => parts.includes(m);
    if (want("ctrl") !== e.ctrlKey || want("shift") !== e.shiftKey || want("alt") !== e.altKey || (want("super") || want("cmd")) !== e.metaKey) return false;
    const code = e.code.replace(/^Key|^Digit/, "").toLowerCase();
    return code === key || e.key.toLowerCase() === key;
  }

  function openSearch() {
    if (onboarding) return;
    settingsOpen = false;
    tidyOpen = false;
    store.archiveOpen = null;
    searchOpen = true;
  }

  /** Note size: Ctrl+wheel, Ctrl + / Ctrl − / Ctrl 0. */
  function stepSize(dir: -1 | 0 | 1) {
    const i = SIZES.indexOf(store.size);
    const next = dir === 0 ? "comfortable" : SIZES[Math.max(0, Math.min(SIZES.length - 1, i + dir))];
    if (next !== store.size) store.setSetting("note_size", next);
  }

  $effect(() => {
    const wheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      stepSize(e.deltaY < 0 ? 1 : -1);
    };
    window.addEventListener("wheel", wheel, { passive: false });
    return () => window.removeEventListener("wheel", wheel);
  });

  // ---- Back and Forward, like a browser ----
  // The mouse's side buttons, the keyboard's Browser Back / Forward keys, Alt+Left / Alt+Right
  // and Backspace (outside a text field) go one level back or forward in what is open; with
  // nothing open they leave the board alone.
  function step(dir: "back" | "forward", source: NavSource): boolean {
    if (onboarding || termsOpen) return false;
    if (!once(source, dir, performance.now())) return true;
    return dir === "back" ? layerBack() : layerForward();
  }
  $effect(() => {
    // WebView2 would treat the side buttons as history navigation: the page takes them instead.
    const side = (e: MouseEvent) => {
      if (e.button !== 3 && e.button !== 4) return;
      e.preventDefault();
      if (e.type === "mouseup") step(e.button === 3 ? "back" : "forward", "mouse");
    };
    for (const t of ["mousedown", "mouseup", "auxclick"]) window.addEventListener(t, side as EventListener, true);
    return () => {
      for (const t of ["mousedown", "mouseup", "auxclick"]) window.removeEventListener(t, side as EventListener, true);
    };
  });
  /** A selected board line keeps Backspace (task ↔ idea) unless the keyboard is in an open view. */
  function boardOwnsBackspace(): boolean {
    if (!store.focus) return false;
    if (!boardCovered()) return true;
    if (!look.panelOpen || store.focus.item == null) return false;
    const a = document.activeElement;
    return !a || a === document.body || !!scroller?.contains(a);
  }
  function navKey(e: KeyboardEvent): boolean {
    // (A held key's repeats take no more steps.)
    if (e.key === "BrowserBack" || e.key === "BrowserForward") {
      if (!e.repeat) step(e.key === "BrowserBack" ? "back" : "forward", "key");
      return true;
    }
    if (typing() || e.ctrlKey || e.metaKey || e.shiftKey) return false;
    if (e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
      if (!e.repeat) step(e.key === "ArrowLeft" ? "back" : "forward", "key");
      return true;
    }
    // Backspace on the board keeps its own meaning (task ↔ idea); it goes back only in what is open
    // (a card beside the board, like Look back, leaves a focused board line its Backspace; so does
    // the Style panel when a board line is selected and the keyboard isn't inside the panel).
    if (e.key === "Backspace" && !e.altKey && layerCount() > 0 && !boardOwnsBackspace()) {
      if (!e.repeat) step("back", "key");
      return true;
    }
    return false;
  }

  // ---- Help and the quick tour ----
  /** Closes whatever is open, then the tour starts (once what closed has mostly gone). */
  function startTour() {
    if (onboarding || tour.open) return;
    const any = settingsOpen || searchOpen || tidyOpen || look.panelOpen || viewer.open || store.templatesOpen || store.lookbackOpen || !!boardMenuAt || !!store.archiveOpen;
    settingsOpen = searchOpen = tidyOpen = false;
    store.archiveOpen = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    store.lookbackOpen = false;
    boardMenuAt = null;
    store.focus = null;
    if (store.ruler) store.ruler = null;
    store.toast = null;
    // (Not over something opened meanwhile, like Help by F1.)
    setTimeout(() => {
      if (!onboarding && !settingsOpen && !searchOpen && !tidyOpen && !look.panelOpen && !viewer.open && !store.templatesOpen && !store.archiveOpen) tour.start();
    }, any && !reduce.matches ? 200 : 0);
  }
  // A full view over the board: the board waits to show notes put back from the archive.
  $effect(() => {
    store.over = settingsOpen || searchOpen || tidyOpen || !!store.archiveOpen || store.templatesOpen || viewer.open;
  });

  // ---- The Archive ----
  function openArchive(from: "board" | "settings") {
    if (onboarding) return;
    tour.close();
    searchOpen = tidyOpen = false;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    boardMenuAt = null;
    settingsOpen = false;
    // ("See archive" has nothing left to say once it is open.)
    if (store.toast?.more) store.toast = null;
    store.archiveOpen = from;
  }
  /** Back to the board. Notes just put back that belong to the other space: the board shows
   * their space, so their cards are seen settling in. */
  function closeArchive() {
    store.archiveOpen = null;
    const back = store.arriving.map((id) => store.notes.find((n) => n.id === id)).filter((n) => !!n);
    if (back.length && store.view !== "all" && !back.some((n) => n.space === store.view)) void store.setView(back[0].space);
  }
  /** Back one level: to Settings › Archive when it was opened from there, else the board. */
  function archiveBack() {
    if (store.archiveOpen !== "settings") return closeArchive();
    store.archiveOpen = null;
    settingsAt = { key: "Archive", n: (settingsAt?.n ?? 0) + 1 };
    settingsOpen = true;
  }

  /** Settings › Help (or › Keyboard). */
  function openHelp(key: "Help" | "Keyboard" = "Help") {
    if (onboarding) return;
    helpMenu.open = false;
    tour.close();
    searchOpen = tidyOpen = false;
    store.archiveOpen = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    boardMenuAt = null;
    if (store.ruler?.mode === "click") store.ruler = null;
    settingsAt = { key, n: (settingsAt?.n ?? 0) + 1 };
    settingsOpen = true;
  }
  /** Once, a moment after the first-run setup: a gentle offer, never started by itself. */
  function offerTour() {
    if (store.settings.tour_offered || store.settings.tour_seen) return;
    setTimeout(() => {
      if (tour.open || onboarding) return;
      void store.setSetting("tour_offered", Date.now());
      store.showToast(() => t("New here? A one-minute tour."), { label: () => t("Take the tour"), run: startTour }, 15_000);
    }, 1400);
  }

  /** Once, for someone set up before the tour came: "New: a one-minute tour", on a free board a
   * moment after the panel shows (never over something open, never taking the keyboard). Shown
   * once, it never comes back, taken or not. */
  let setUpBefore = false;
  let newsTimer: ReturnType<typeof setTimeout> | null = null;
  function tourNews() {
    if (!setUpBefore || newsTimer || store.settings.tour_offered || store.settings.tour_seen) return;
    newsTimer = setTimeout(() => {
      newsTimer = null;
      if (!shown || onboarding || termsOpen || tour.open || store.toast || boardCovered() || store.ruler || itemDrag.item) return;
      // One ask at a time: not beside the evening's Look back card, a reminder's "later" choices or
      // the Sticky Notes dates offer (it comes on a later show).
      if (store.lookback || store.laterFor != null || (inTauri && store.settings.sticky_imported && !store.settings.sticky_dates_offered)) return;
      if (store.settings.tour_offered || store.settings.tour_seen) return;
      void store.setSetting("tour_offered", Date.now());
      store.showToast(() => t("New: a one-minute tour."), { label: () => t("Take the tour"), run: startTour }, 15_000, { label: () => t("Not now"), run: () => {} });
    }, reduce.matches ? 900 : 1600);
  }

  function toggleStyle() {
    if (look.panelOpen) look.panelOpen = false;
    else {
      look.open("main");
      settingsOpen = false;
      store.archiveOpen = null;
    }
  }
  function toggleSettings() {
    settingsAt = null;
    settingsOpen = !settingsOpen;
    searchOpen = false;
    look.panelOpen = false;
    store.archiveOpen = null;
  }

  /** Page Up / Page Down / Home / End move through the board (nothing open, not typing). */
  function scrollKey(e: KeyboardEvent): boolean {
    if (!scroller || typing() || boardCovered() || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return false;
    const h = scroller.clientHeight * 0.85;
    const behavior: ScrollBehavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
    if (e.key === "PageDown") scroller.scrollBy({ top: h, behavior });
    else if (e.key === "PageUp") scroller.scrollBy({ top: -h, behavior });
    else if (e.key === "Home" && !store.focus) scroller.scrollTo({ top: 0, behavior });
    else if (e.key === "End" && !store.focus) scroller.scrollTo({ top: scroller.scrollHeight, behavior });
    else return false;
    return true;
  }

  function onKey(e: KeyboardEvent) {
    // (A key that finishes a composed character in a field, its Esc included, is the field's.)
    // (Nor while the terms of use wait: only their two buttons answer then.)
    if (e.defaultPrevented || onboarding || termsOpen || composing(e)) return;
    // F1: the "?" menu at the board's corner (the tour first); again, or with a view open over
    // the board, Help itself.
    if (e.key === "F1" && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
      e.preventDefault();
      // (Not in the middle of carrying a line: nothing opens under the slip.)
      if (itemDrag.item) return;
      if (!e.repeat && !helpMenu.open && !boardCovered() && !tour.open) helpMenu.open = true;
      else if (!e.repeat) openHelp();
      return;
    }
    if (navKey(e) || scrollKey(e) || (!store.focus && gridKey(e))) {
      e.preventDefault();
      return;
    }
    // Ctrl+Shift+S: Style · Ctrl+Shift+P: Settings ("paramètres"); Ctrl+, too, the usual
    // place for Settings. They type nothing, so they work while writing in a note too; again
    // closes.
    const ctrl = (e.ctrlKey || e.metaKey) && !e.altKey;
    if (ctrl && e.shiftKey && (e.key.toLowerCase() === "s" || e.key.toLowerCase() === "p")) {
      e.preventDefault();
      if (e.key.toLowerCase() === "s") toggleStyle();
      else toggleSettings();
      return;
    }
    if (ctrl && !e.shiftKey && e.key === ",") {
      e.preventDefault();
      toggleSettings();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && !e.altKey && ["=", "+", "-", "_", "0"].includes(e.key)) {
      e.preventDefault();
      stepSize(e.key === "0" ? 0 : e.key === "-" || e.key === "_" ? -1 : 1);
      return;
    }
    if (matches(e, store.shortcuts.search) || (e.key === "/" && !typing() && !e.ctrlKey && !e.altKey && !e.metaKey)) {
      e.preventDefault();
      openSearch();
      return;
    }
    if (e.key !== "Escape") {
      // The board by keyboard (only when nothing is open on top of it).
      const over = settingsOpen || searchOpen || tidyOpen || viewer.open || store.templatesOpen;
      if (!over && !store.archiveOpen && boardKey(e)) e.preventDefault();
      // The Archive page: only Undo / Redo (a deleted note comes back with Ctrl+Z).
      else if (!over && store.archiveOpen && undoKey(e)) e.preventDefault();
      return;
    }
    // Esc closes the topmost thing; then it leaves the board's keyboard focus; with nothing
    // open, a docked panel slides away. The entry line keeps what you typed.
    if (closeTopLayer()) {
      e.preventDefault();
      return;
    }
    if (store.focus) {
      store.focus = null;
      e.preventDefault();
      return;
    }
    if (store.win.mode === "docked" && !onboarding) hide();
  }

  // A pasted picture: onto the note under the mouse, else as a new line in the Inbox (or the
  // note the entry line writes into).
  async function onPaste(e: ClipboardEvent) {
    const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith("image/"));
    if (!files.length || onboarding || tidyOpen) return;
    e.preventDefault();
    const capturing = document.activeElement === captureInput;
    if (hoverNote != null && !inMemory(hoverNote) && !capturing) {
      for (const f of files) await store.addPictureBlob(hoverNote, null, f);
      return;
    }
    const noteId = store.target != null && !inMemory(store.target) ? store.target : await store.inboxId(store.space);
    const text = captureInput?.value.trim() || t("Picture");
    const it = await store.backend.createItem({ noteId, kind: "task", text });
    store.upsertItem(it);
    for (const f of files) await store.addPictureBlob(noteId, it.id, f);
    if (captureInput && capturing) {
      captureInput.value = "";
      captureInput.dispatchEvent(new Event("input", { bubbles: true }));
    }
  }

  function onPointerOver(e: PointerEvent) {
    const card = (e.target as HTMLElement).closest<HTMLElement>("[data-drop-note]");
    hoverNote = card ? Number(card.dataset.dropNote) : null;
  }

  // Right-click an empty spot of the board: "Change background…".
  let boardMenuAt = $state<{ x: number; y: number } | null>(null);
  function boardMenu(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t.closest(".slot, input, textarea, [contenteditable], a, button")) return;
    e.preventDefault();
    const shell = (e.currentTarget as HTMLElement).closest(".shell")!.getBoundingClientRect();
    boardMenuAt = { x: Math.min(e.clientX - shell.left, shell.width - 200), y: Math.min(e.clientY - shell.top, shell.height - 136) };
  }
  // The menu is the top layer while it shows: Esc (or Back) closes just it, not what is under it.
  $effect(() => {
    if (!boardMenuAt) return;
    return pushLayer(() => (boardMenuAt = null));
  });
  /** The board's menu takes the keyboard while it shows (its first line), and gives it back to
   * where it was when it closes with nothing else taking it. */
  function ctxFocus(node: HTMLElement) {
    const was = document.activeElement as HTMLElement | null;
    node.querySelector<HTMLElement>("button")?.focus({ preventScroll: true });
    return {
      destroy() {
        const a = document.activeElement;
        if (was?.isConnected && was !== document.body && (!a || a === document.body || node.contains(a))) was.focus({ preventScroll: true });
      },
    };
  }
  /** Arrows, Home / End move through the board's menu, as in the app's other menus. */
  function ctxKeys(e: KeyboardEvent) {
    const list = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("button")];
    const i = list.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") list[(i + 1) % list.length]?.focus();
    else if (e.key === "ArrowUp") list[(i - 1 + list.length) % list.length]?.focus();
    else if (e.key === "Home") list[0]?.focus();
    else if (e.key === "End") list[list.length - 1]?.focus();
    else return;
    e.preventDefault();
  }

  // ---- The docked panel's width, dragged by its inner edge ----
  let resizing = $state(false);
  /** The board's "Archive (n)" link is fading out (its room stays until it is gone). */
  let archiveLeaving = $state(false);
  function gripDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const startX = e.screenX;
    const startW = store.win.width;
    // Docked right, the handle is on the left: dragging left makes it wider.
    const sign = left ? 1 : -1;
    let want = startW;
    let sent = startW;
    let busy = false;
    let frame = 0;
    resizing = true;
    // One resize per frame, never two at once (the window follows smoothly).
    const send = async () => {
      frame = 0;
      if (busy || want === sent) return;
      busy = true;
      sent = want;
      await store.resizeDocked(want, true, false);
      busy = false;
      if (want !== sent && !frame) frame = requestAnimationFrame(send);
    };
    const move = (ev: PointerEvent) => {
      want = Math.round(startW + sign * (ev.screenX - startX));
      if (!frame) frame = requestAnimationFrame(send);
    };
    const up = async () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      if (frame) cancelAnimationFrame(frame);
      resizing = false;
      await store.resizeDocked(want, true, true);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  }
  function gripKey(e: KeyboardEvent) {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    e.stopPropagation();
    const wider = left ? e.key === "ArrowRight" : e.key === "ArrowLeft";
    void store.resizeDocked(store.win.width + (wider ? 20 : -20), false, true);
  }

  const edges = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"];

  /** A slip dropped on an hour shrinks into that hour's dot (DragAnim). */
  function land(node: HTMLElement, l: Landing) {
    const from = `translate(${l.from.x + 14}px, ${l.from.y + 10}px) rotate(${l.tilt}deg) scale(1)`;
    const w = node.offsetWidth;
    const h = node.offsetHeight;
    const to = `translate(${l.to.x - w / 2}px, ${l.to.y - h / 2}px) rotate(0deg) scale(0.15)`;
    node.style.transform = from;
    node.style.transformOrigin = "50% 50%";
    const a = node.animate(
      [
        { transform: from, opacity: 1 },
        { transform: to, opacity: 0 },
      ],
      { duration: reduce.matches ? 1 : 300, easing: "cubic-bezier(.4,0,.6,1)", fill: "forwards" },
    );
    a.onfinish = () => {
      itemDrag.landings = itemDrag.landings.filter((x) => x.key !== l.key);
    };
  }

  /** A carry called off: the slip glides back onto its line's text (following the line while the
   * board settles back under the folding ruler), straightens, and fades as the line shows again.
   * Reduced motion: it just fades where it is. */
  function goBack(node: HTMLElement, b: Back) {
    const h = node.offsetHeight;
    const sx = b.from.x + 14;
    const sy = b.from.y + 10;
    const place = (x: number, y: number, tilt: number, o: number) => {
      node.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) rotate(${tilt.toFixed(2)}deg)`;
      node.style.opacity = o.toFixed(3);
    };
    place(sx, sy, b.tilt, 1);
    const id = b.id;
    let away = true;
    // On its line: the line shows again as the slip fades (one text, not two).
    const arrived = () => {
      if (!away) return;
      away = false;
      const i = itemDrag.away.indexOf(id);
      if (i >= 0) itemDrag.away = itemDrag.away.filter((_, k) => k !== i);
    };
    const line = () => document.querySelector<HTMLElement>(`${liveLine(id)} ${b.idea ? ".idea-text" : ".text"}`);
    const GO = reduce.matches ? 0 : 300;
    const FADE = reduce.matches ? 140 : 130;
    const t0 = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const t = now - t0;
      const p = GO ? Math.min(1, t / GO) : 1;
      const e = 1 - Math.pow(1 - p, 3.4);
      const r = GO ? line()?.getBoundingClientRect() : null;
      // The slip's text sits 13 px in from its edge (its padding and border).
      const tx = r && r.width ? r.left - 13 : sx;
      const ty = r && r.width ? r.top + r.height / 2 - h / 2 : sy;
      const fade = Math.max(0, Math.min(1, (t - GO) / FADE));
      if (p >= 1) arrived();
      place(sx + (tx - sx) * e, sy + (ty - sy) * e, b.tilt * (1 - e), 1 - fade);
      if (fade < 1) raf = requestAnimationFrame(tick);
      else itemDrag.backs = itemDrag.backs.filter((x) => x.key !== b.key);
    };
    raf = requestAnimationFrame(tick);
    return {
      destroy() {
        cancelAnimationFrame(raf);
        arrived();
      },
    };
  }
</script>

<svelte:window onkeydown={onKey} onpaste={onPaste} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="shell"
  class:floating
  class:docked={!floating}
  class:left
  class:shown
  class:preview={!inTauri}
  style:--dock-w="{store.win.width}px"
  onpointerover={onPointerOver}
  onpointerdown={() => (store.focus = null)}
>
  {#if !floating && store.loaded}
    <!-- The panel's inner edge: drag (or arrow keys) to change its width. A focusable separator is the ARIA window-splitter pattern. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="dock-grip"
      style:--dock-w="{store.win.width}px"
      class:left
      class:dragging={resizing}
      role="separator"
      aria-orientation="vertical"
      aria-label={t("Panel width, {n} pixels. Drag, or use the arrow keys.", { n: store.win.width })}
      aria-valuenow={store.win.width}
      aria-valuemin={420}
      aria-valuemax={1200}
      tabindex="0"
      data-tip={t("Drag to change the width")} data-tip-keys="Left|Right"
      onpointerdown={gripDown}
      onkeydown={gripKey}
    >
      <span></span>
    </div>
  {/if}
  {#if store.error}
    <p class="error">{t("Couldn’t open your notes: {error}", { error: store.error })}</p>
  {:else if store.loaded}
    <BoardBackground />
    <div class="chrome">
      <Header
        onHide={hide}
        onSettings={toggleSettings}
        onSearch={openSearch}
        onTidy={() => ((tidyOpen = true), (settingsOpen = false), (searchOpen = false))}
        onStyle={toggleStyle}
      />
      <Capture bind:input={captureInput} />
      <SpaceBanner />
      <NowCard />
      {#if inTauri && store.settings.sticky_imported && !store.settings.sticky_dates_offered}<StickyDates />{/if}
      <NextLine />
    </div>
    <!-- Today's hours while a task is carried: a card of its own under the top bar. -->
    <HoursRuler />
    <!-- svelte-ignore a11y_no_static_element_interactions (right-click: a shortcut to the Style panel, which also has its own button) -->
    <div
      class="scroll"
      bind:this={scroller}
      class:file-over={fileDrop.over && fileDrop.note == null}
      class:textured={!["plain", "paper", "dotted"].includes(look.active.bg)}
      class:dark-board={lookIsDark(look.active)}
      class:held={heldBy(look.active) !== "none"}
      class:to-archive-room={store.archived.length > 0 || archiveLeaving}
      oncontextmenu={boardMenu}
    >
      <!-- The evening's look back (or today's, from its button): whole, at the top of the board,
           which makes room under it (the page scrolls). -->
      {#if store.lookback}<LookBack day={store.lookback} {scroller} />{/if}
      <div class="zoom" style:zoom={store.zoom}>
        <!-- The Inbox: a strip across the board's width, above the notes. -->
        <InboxStrip />
        <Board notes={store.boardNotes} {scroller} />
      </div>
      {#if !store.boardNotes.length}
        <p class="nothing" in:arrive={{ delay: 160, duration: 280, y: 6 }} out:fadeQuick={{ duration: 120 }}>{t("No notes here yet. Type a line with #name to start one.")}</p>
      {/if}
      <!-- Under the last notes, at the left (clear of the note sizes and "?" at the right corner):
           the way to the notes put away, while there are some. -->
      {#if store.archived.length}
        <div class="to-archive" class:alone={!store.boardNotes.length} transition:rise={{ y: 6, duration: 240 }} onoutrostart={() => (archiveLeaving = true)} onoutroend={() => (archiveLeaving = false)} onintrostart={() => (archiveLeaving = false)}>
          <button data-tip={t("Open the archive")} onclick={() => openArchive("board")}
            ><Icon name="archive" size={14} stroke={1.8} /><span>{t("Archive ({n})", { n: store.archived.length })}</span></button
          >
        </div>
      {/if}
    </div>
    <!-- While a task is carried to the hours (or the hours are picked from its menu), the board
         dims a touch so the ruler stands out; it fades in and out with the ruler. -->
    <div class="board-dim" class:on={itemDrag.item?.kind === "task" || !!store.ruler || itemDrag.landings.length > 0} aria-hidden="true"></div>
    <SizeControl />
    <Toast />
    {#if fileDrop.over}
      <div class="drop-hint" aria-hidden="true" transition:fadeQuick={{ duration: 160 }}>{fileDrop.note == null ? t("Drop to put it in the Inbox") : t("Drop on this note")}</div>
    {/if}
    {#if boardMenuAt}
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="ctx-veil" data-drag-through onpointerdown={() => (boardMenuAt = null)} oncontextmenu={(e) => (e.preventDefault(), (boardMenuAt = null))}></div>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="ctx" role="menu" tabindex="-1" transition:pop|global={{ duration: 160 }} style:left="{boardMenuAt.x}px" style:top="{boardMenuAt.y}px" onkeydown={ctxKeys} use:ctxFocus>
        <button role="menuitem" onclick={() => ((boardMenuAt = null), look.open("main"), (settingsOpen = false))}>{t("Change background…")}</button>
        <button role="menuitem" onclick={() => openArchive("board")}
          ><span>{t("Archived notes…")}</span>{#if store.archived.length}<span class="count">{store.archived.length}</span>{/if}</button
        >
        <hr />
        <button role="menuitem" onclick={() => ((boardMenuAt = null), startTour())}>{t("Take the tour")}</button>
      </div>
    {/if}
    {#if look.panelOpen}
      <StylePanel onClose={() => (look.panelOpen = false)} />
    {/if}
    {#if settingsOpen}
      <Settings at={settingsAt} onClose={() => ((settingsOpen = false), (settingsAt = null))} />
    {/if}
    {#if tidyOpen}
      <Tidy onClose={() => (tidyOpen = false)} />
    {/if}
    {#if searchOpen}
      <Search onClose={() => (searchOpen = false)} />
    {/if}
    {#if viewer.open}
      <Viewer />
    {/if}
    {#if store.templatesOpen}
      <Templates onClose={() => (store.templatesOpen = false)} />
    {/if}
    {#if store.archiveOpen}
      <Archive from={store.archiveOpen} onBack={archiveBack} onClose={closeArchive} />
    {/if}
    {#if tour.open}
      <Tour />
    {/if}
    {#if onboarding}
      <FirstRun
        onDone={async () => {
          // The setup window becomes the real panel: Rust places it (docked or floating) and says
          // "panel-shown", which plays the slide-in.
          onboarding = false;
          await store.refreshWindow();
          if (inTauri) await store.backend.applyWindow();
          else appear();
          offerTour();
        }} />
    {/if}
    {#if termsOpen}
      <Terms onDone={() => (termsOpen = false)} />
    {/if}
  {/if}
</div>

{#if store.peek && store.loaded}
  <Peek />
{/if}

{#if itemDrag.item}
  <div class="slip" class:idea={itemDrag.item.kind === "idea"} style:transform="translate({itemDrag.x + 14}px, {itemDrag.y + 10}px) rotate({itemDrag.tilt}deg)" aria-hidden="true">
    {itemDrag.item.text}
  </div>
{/if}
{#each itemDrag.backs as b (b.key)}
  <div class="slip back" class:idea={b.idea} aria-hidden="true" use:goBack={b}>
    {b.text}
  </div>
{/each}
{#each itemDrag.landings as l (l.key)}
  <div class="slip" class:idea={l.idea} aria-hidden="true" use:land={l}>
    {l.text}
  </div>
{/each}

{#if floating}
  {#each edges as dir (dir)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="grip {dir}" onpointerdown={(e) => e.button === 0 && store.backend.startResize(dir)}></div>
  {/each}
{/if}

<style>
  .shell {
    position: absolute;
    inset: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: var(--r-xl);
    background: var(--ground);
    box-shadow: var(--shell-shadow);
    contain: layout paint;
    container: shell / inline-size;
  }
  /* Docked: slides in from its edge. */
  .shell.docked {
    transform: translateX(calc(100% + 24px));
    transition: transform 0.24s cubic-bezier(0.4, 0, 1, 1);
  }
  .shell.docked.left {
    transform: translateX(calc(-100% - 24px));
  }
  .shell.docked.shown {
    transform: translateX(0);
    transition: transform 0.42s cubic-bezier(0.2, 0.85, 0.25, 1);
  }
  /* Floating: settles in. */
  .shell.floating {
    opacity: 0;
    transform: scale(0.985);
    transition: opacity 0.16s ease-in, transform 0.16s ease-in;
  }
  .shell.floating.shown {
    opacity: 1;
    transform: none;
    transition: opacity 0.22s ease-out, transform 0.28s cubic-bezier(0.2, 0.85, 0.25, 1);
  }
  @media (prefers-reduced-motion: reduce) {
    .shell,
    .shell.shown {
      transition: none !important;
    }
  }

  /* Browser preview: a docked panel at the right of the page, a floating window in the middle. */
  .shell.preview.docked {
    left: auto;
    width: min(var(--dock-w), calc(100% - 24px));
  }
  .shell.preview.docked.left {
    left: 12px;
    right: auto;
  }
  .shell.preview.floating {
    inset: auto;
    left: 50%;
    top: 50%;
    width: min(760px, calc(100% - 24px));
    height: min(820px, calc(100% - 24px));
    translate: -50% -50%;
  }

  /* The top (header, entry line, timeline) sits on the app's own paper over the board. */
  .chrome {
    position: relative;
    /* (Over the hours ruler under it, which slides up under the bar as it folds away.) */
    z-index: 3;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--ground) 94%, transparent);
    box-shadow: 0 1px 0 color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .scroll {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--s5) var(--s6) var(--s6);
    scrollbar-gutter: stable;
    transition: background-color 0.2s;
  }
  .scroll.file-over {
    background: var(--hover);
  }
  .zoom {
    padding-bottom: 44px;
  }
  /* Notes stay opaque cards; on a texture or a dark board their shadow grows to lift them off it. */
  .scroll.textured {
    --card-shadow: 0 2px 4px rgba(40, 24, 8, 0.16), 0 8px 20px rgba(40, 24, 8, 0.14);
  }
  .scroll.dark-board {
    --card-shadow: 0 2px 5px rgba(0, 0, 0, 0.32), 0 12px 26px rgba(0, 0, 0, 0.32);
  }
  /* Room for the pins and tape above the first row. */
  .scroll.held {
    padding-top: calc(var(--s5) + 8px);
  }
  .board-dim {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    background: rgb(28 22 14 / 0.07);
    opacity: 0;
    transition: opacity 0.26s var(--ease-glide);
    will-change: opacity;
  }
  :global(:root[data-mode="dark"]) .board-dim {
    background: rgb(0 0 0 / 0.24);
  }
  .board-dim.on {
    opacity: 1;
    transition-duration: 0.32s;
  }
  .ctx-veil {
    position: absolute;
    inset: 0;
    z-index: 60;
  }
  .ctx {
    position: absolute;
    z-index: 61;
    min-width: 190px;
    padding: 6px;
    border-radius: var(--r-lg);
    background: var(--card);
    box-shadow: var(--pop-shadow);
  }
  .ctx button {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    text-align: left;
    font-size: var(--t-md);
    color: var(--ink);
    cursor: pointer;
  }
  .ctx button:hover,
  .ctx button:focus-visible {
    background: var(--hover);
  }
  .ctx button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
  }
  .ctx hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 4px 6px;
  }
  .ctx .count {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .drop-hint {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    z-index: 30;
    padding: 6px 12px;
    border-radius: 9px;
    background: var(--ink);
    color: var(--card);
    font-size: var(--t-sm);
    font-weight: 600;
    pointer-events: none;
  }
  /* 12 px under the notes. It takes the room kept there for the corner controls, and the board
     ends 20 px under it: scrolled to the end, it sits in one row with the note sizes and "?". */
  .scroll.to-archive-room {
    padding-bottom: 20px;
  }
  .scroll.to-archive-room .zoom {
    padding-bottom: 0;
  }
  .to-archive {
    margin-top: 12px;
    display: flex;
  }
  .to-archive.alone {
    margin-top: var(--s4);
  }
  .to-archive button {
    height: 28px;
    padding: 0 11px 0 9px;
    border: 0;
    border-radius: var(--r-md);
    background: color-mix(in srgb, var(--ground) 88%, transparent);
    color: var(--muted);
    font-size: var(--t-sm);
    display: flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }
  .to-archive button:hover,
  .to-archive button:focus-visible {
    background: var(--card);
    color: var(--ink);
  }
  .nothing {
    width: fit-content;
    margin-inline: auto !important;
    padding: 6px 14px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--ground) 88%, transparent);
  }
  .nothing,
  .error {
    margin: var(--s4) 0 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
    color: var(--muted);
    text-align: center;
  }
  .error {
    padding: var(--s6);
  }

  /* The line being carried: a paper slip that follows the pointer. */
  .slip {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 200;
    max-width: 260px;
    padding: 7px 12px;
    border-radius: 8px;
    background: var(--card);
    border: 1px solid var(--line);
    box-shadow: var(--lift-shadow);
    font-size: var(--t-base);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
    transform-origin: 0 0;
    animation: lift 0.16s var(--ease-out);
  }
  .slip.back {
    animation: none;
  }
  .slip.idea {
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
  }
  @keyframes lift {
    from {
      opacity: 0;
      scale: 0.96;
    }
  }

  .grip {
    position: fixed;
    z-index: 100;
  }
  /* The docked panel's inner edge (a subtle grip on hover). */
  .dock-grip {
    position: fixed;
    top: 24px;
    bottom: 24px;
    left: 6px;
    width: 12px;
    z-index: 100;
    cursor: ew-resize;
    display: grid;
    place-items: center;
    outline: none;
    touch-action: none;
  }
  .dock-grip.left {
    left: auto;
    right: 6px;
  }
  .dock-grip span {
    width: 4px;
    height: 44px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--ink) 30%, transparent);
    opacity: 0;
    transform: scaleY(0.7);
    transition: opacity 0.18s, transform 0.18s var(--ease-out), background-color 0.18s;
  }
  .dock-grip:hover span,
  .dock-grip:focus-visible span,
  .dock-grip.dragging span {
    opacity: 1;
    transform: none;
  }
  .dock-grip.dragging span,
  .dock-grip:focus-visible span {
    background: var(--ink);
  }
  /* Browser preview: the panel sits at the right of the page. */
  :global(html.preview) .dock-grip:not(.left) {
    left: calc(100% - min(var(--dock-w), 100% - 24px) - 18px);
  }
  :global(html.preview) .dock-grip.left {
    right: auto;
    left: calc(min(var(--dock-w), 100% - 24px) + 6px);
  }
  .grip.North,
  .grip.South {
    left: 24px;
    right: 24px;
    height: 10px;
    cursor: ns-resize;
  }
  .grip.North {
    top: 7px;
  }
  .grip.South {
    bottom: 7px;
  }
  .grip.East,
  .grip.West {
    top: 24px;
    bottom: 24px;
    width: 10px;
    cursor: ew-resize;
  }
  .grip.East {
    right: 7px;
  }
  .grip.West {
    left: 7px;
  }
  .grip.NorthEast,
  .grip.NorthWest,
  .grip.SouthEast,
  .grip.SouthWest {
    width: 20px;
    height: 20px;
  }
  .grip.NorthEast {
    top: 7px;
    right: 7px;
    cursor: nesw-resize;
  }
  .grip.SouthWest {
    bottom: 7px;
    left: 7px;
    cursor: nesw-resize;
  }
  .grip.NorthWest {
    top: 7px;
    left: 7px;
    cursor: nwse-resize;
  }
  .grip.SouthEast {
    bottom: 7px;
    right: 7px;
    cursor: nwse-resize;
  }
</style>
