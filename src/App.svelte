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
  import { updates, watchUpdates } from "./lib/updates.svelte";
  import { bundled, ownSection, planAfterStart, whatsNewStep, type Shown } from "./lib/whatsnew";
  import SizeControl from "./components/SizeControl.svelte";
  import Toast from "./components/Toast.svelte";
  import LookBack from "./components/LookBack.svelte";
  import Templates from "./components/Templates.svelte";
  import RepeatHistory from "./components/RepeatHistory.svelte";
  import Archive from "./components/Archive.svelte";
  import Report from "./components/Report.svelte";
  import WhatsNew from "./components/WhatsNew.svelte";
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
  import type { BoardPoint } from "./lib/newNote";
  import { blankItem, inTauri, type EventName } from "./lib/backend";
  import { boardCovered, closeTopLayer, layerBack, layerCount, layerForward, pushLayer } from "./lib/layers";
  import { copyOffer, menuTargetOf, selectedAt, type CopyOffer, type Point } from "./lib/contextmenu";
  import { askMenu, atPointer, fromKeys, noteMenuKey, rightClick, swallowed, takeMenu } from "./lib/rightclick.svelte";
  import { lineText } from "./lib/astext";
  import { modal } from "./lib/modal";
  import { once, type NavSource } from "./lib/nav";
  import { gridKey } from "./lib/gridkeys";
  import { boardKey, focusWords, isStrip, liveLine, liveNote, undoKey } from "./lib/keys";
  import { latinKey, sizeKey } from "./lib/keyboard";
  import { HOURS_LIFT, itemDrag, type Back, type Landing } from "./lib/itemdrag.svelte";
  import { viewer } from "./lib/viewer.svelte";
  import { fileDrop, onFileDrop } from "./lib/filedrop.svelte";
  import { warmStyle } from "./lib/warm";
  import { rehearse } from "./lib/rehearse";
  import { installWindowDrag } from "./lib/windrag";
  import { arrive, fadeQuick, pop } from "./lib/motion";
  import { i18n, keysText, t, type Key } from "./lib/i18n.svelte";
  import { untrack } from "svelte";

  /** A word from the recovery screen on what it did ("Your notes are back."), or none. */
  let { welcome = null }: { welcome?: string | null } = $props();

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");

  let shown = $state(false);
  let shownAt = 0;
  /** The shell is a layer of its own only while it slides or settles in or out: at rest it is
   * drawn with the page, so its words and the logo are crisp at 100 % (a layer kept from the
   * start of the slide, or from the floating window's 98.5 % scale, draws them soft). */
  let sliding = $state(false);
  $effect(() => {
    void shown;
    sliding = true;
    const done = setTimeout(() => (sliding = false), 600);
    return () => clearTimeout(done);
  });
  let settingsOpen = $state(false);
  /** A section Settings opens on (Help or Keyboard, from the "?" menu or F1). */
  let settingsAt = $state<{ key: Key; n: number } | null>(null);
  let searchOpen = $state(false);
  let tidyOpen = $state(false);
  let onboarding = $state(false);
  let termsOpen = $state(false);
  /** Report a problem, and where it was opened from (its Back goes there). */
  let reportOpen = $state<"board" | "settings" | null>(null);
  /** What's new: what it shows, and where it was opened from (its Back goes there). */
  let whatsNew = $state<{ from: "board" | "settings"; shown: Shown } | null>(null);
  /** The notes are read, and the first-run window and the terms are known. */
  let started = $state(false);
  // The recovery screen's word: once the board is in, and not under the first-run window or the
  // terms (after them, then).
  let welcomed = false;
  $effect(() => {
    if (welcomed || !welcome || !started || onboarding || termsOpen) return;
    welcomed = true;
    const word = welcome;
    setTimeout(() => store.showToast(word, undefined, 8000), 900);
  });
  let captureInput = $state<HTMLInputElement>();
  let scroller = $state<HTMLElement>();
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let hoverNote: number | null = null;

  let floating = $derived(store.win.mode === "floating");
  // A view over the whole board (or the first-run window, the terms): what is under it is inert,
  // so Tab, typing and clicks never reach it. Under a veiled dialog (Search, a new note) the top
  // bar stays as it is, for the window to be moved by (its Tab stays in the dialog).
  let fullCover = $derived(settingsOpen || tidyOpen || viewer.open || !!store.archiveOpen || !!reportOpen || !!whatsNew || onboarding || termsOpen);
  let covered = $derived(fullCover || searchOpen || store.templatesOpen);
  // (Made inert once the view has arrived: going inert restyles the whole board, about 150 ms on a
  // big board at a slow CPU, which would stall the view's opening. Meanwhile the view keeps the
  // keyboard itself (modal.ts) and covers the board. On close it stops at once.)
  let shellEl = $state<HTMLElement>();
  let coverSettled = $state(false);
  const arriving = new Set<Element>();
  function settleCover() {
    for (const el of arriving) if (!el.isConnected) arriving.delete(el);
    if (covered && !arriving.size) coverSettled = true;
  }
  $effect(() => {
    const shell = shellEl;
    if (!shell) return;
    // The views' own arrivals: they are the shell's children (a card or a row arriving inside
    // one doesn't count).
    const track = (e: Event) => {
      const el = e.target as Element;
      if (el.parentElement !== shell) return;
      if (e.type === "introstart") arriving.add(el);
      else {
        arriving.delete(el);
        settleCover();
      }
    };
    const types = ["introstart", "introend", "outrostart"];
    for (const ty of types) shell.addEventListener(ty, track, true);
    return () => types.forEach((ty) => shell.removeEventListener(ty, track, true));
  });
  $effect(() => {
    if (!covered) {
      coverSettled = false;
      return;
    }
    // (A frame for the arrival to start; then once it ends, or at once without one. Never later
    // than the longest arrival, whatever happens to it.)
    const frame = requestAnimationFrame(settleCover);
    const latest = setTimeout(() => (coverSettled = true), 900);
    return () => {
      cancelAnimationFrame(frame);
      clearTimeout(latest);
    };
  });
  let boardInert = $derived(covered && coverSettled);
  let chromeInert = $derived(fullCover && coverSettled);
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
    // (What was typed in Report a problem stays for its next open.)
    reportOpen = null;
    whatsNew = null;
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

  // The big views are built once off the page in calm moments (rehearse.ts), so their first open
  // is as quick as the next ones.
  $effect(() => {
    if (!store.loaded || onboarding) return;
    const none = () => {};
    const t = setTimeout(
      () =>
        void rehearse([
          [Settings, { onClose: none }],
          [StylePanel, { onClose: none }],
          [Tidy, { onClose: none }],
          [Search, { onClose: none }],
          [Archive, { from: "board", onBack: none, onClose: none }],
          [Report, { from: "board", onBack: none, onClose: none }],
          [WhatsNew, { from: "board", shown: { entries: bundled(i18n.lang).slice(0, 2), since: "1.0.0" }, onBack: none, onClose: none }],
          [Templates, { onClose: none }],
          // (A repeating task's History card, for a line that is nowhere: only built.)
          [RepeatHistory, { item: blankItem({ id: -1, note_id: -1, due_at: Date.now(), repeat: { freq: "day", interval: 1, ends: { kind: "never" } } }), onBack: none, onClose: none }],
          // (…and a weekly and a monthly one's: a year of weeks, a year of months.)
          [RepeatHistory, { item: blankItem({ id: -1, note_id: -1, due_at: Date.now(), repeat: { freq: "week", interval: 2, days: [1, 4], ends: { kind: "never" } } }), onBack: none, onClose: none }],
          [RepeatHistory, { item: blankItem({ id: -1, note_id: -1, due_at: Date.now(), repeat: { freq: "month", interval: 1, monthday: 1, ends: { kind: "never" } } }), onBack: none, onClose: none }],
        ]),
      1500,
    );
    return () => clearTimeout(t);
  });

  $effect(() => {
    store.init().then(() => {
      onboarding = !store.settings.onboarded;
      setUpBefore = !onboarding;
      // The terms of use, once, before anything goes online (an installed copy: accepted in
      // Setup). In the browser preview only with ?terms.
      termsOpen = !store.settings.terms_accepted && (inTauri || new URLSearchParams(location.search).has("terms"));
      started = true;
      watchUpdates();
      store.backend.onEvent("panel-shown", async () => {
        store.peek = false;
        await store.refreshWindow();
        appear();
      });
      store.backend.onEvent("panel-hide-request", () => hide());
      // Something asked for elsewhere (a reminder, a notification, the tray) ends the tour first:
      // it would open under its veil, and could reach the tour's sample line.
      const asked: Partial<Record<EventName, (payload: unknown) => void>> = {
        "open-tidy": () => {
          tour.close();
          settingsOpen = false;
          searchOpen = false;
          reportOpen = null;
          whatsNew = null;
          tidyOpen = true;
        },
        "focus-item": (id) => {
          tour.close();
          const it = store.items.find((i) => i.id === id);
          if (it) store.focusNote(it.note_id);
        },
        // (Only for a reminder still ringing: from an old notification, answered or ticked
        // since, the choices would wait unseen and open by themselves the next time it rings.)
        "later-item": (id) => {
          tour.close();
          if (store.ringing.some((i) => i.id === id)) store.laterFor = id as number;
        },
        "open-lookback": (day) => (tour.close(), store.openLookback(typeof day === "number" ? day : null)),
        "shortcut-taken": (list) => store.sayTakenShortcuts(list),
      };
      for (const [name, run] of Object.entries(asked)) store.backend.onEvent(name as EventName, run);
      // Dobo started by a click on one of its notifications: what it asked came before the page
      // listened, and is done now.
      void store.backend.pageReady().then((before) => before.forEach(([name, payload]) => asked[name]?.(payload)));
      store.backend.onEvent("focus-note", (id) => {
        tour.close();
        if (typeof id === "number" && id > 0) store.focusNote(id);
      });
      // The mouse's side buttons when mouse software sends them as app commands.
      store.backend.onEvent("nav", (dir) => step(dir === "forward" ? "forward" : "back", "app"));
      store.backend.onEvent("peek-shown", () => {
        tour.close();
        shown = false;
        store.peek = true;
      });
      window.addEventListener("dobo-tidy", () => (tour.close(), (tidyOpen = true), (settingsOpen = false), (searchOpen = false), (reportOpen = null), (whatsNew = null)));
      // The window closed or minimised (its × hides it without the slide-out): the tour ends too.
      document.addEventListener("visibilitychange", () => document.hidden && tour.close());
      window.addEventListener("dobo-tour", startTour);
      window.addEventListener("dobo-archive", (e) => openArchive((e as CustomEvent).detail === "settings" ? "settings" : "board"));
      window.addEventListener("dobo-report", (e) => openReport((e as CustomEvent).detail === "settings" ? "settings" : "board"));
      window.addEventListener("dobo-whatsnew", (e) => void reopenWhatsNew((e as CustomEvent).detail === "settings" ? "settings" : "board"));
      window.addEventListener("dobo-help", (e) => openHelp((e as CustomEvent).detail === "Keyboard" ? "Keyboard" : "Help"));
      if (!inTauri && new URLSearchParams(location.search).has("peek")) store.peek = true;
      store.backend.onFocusChanged((focused) => {
        if (!focused && !onboarding && store.win.mode === "docked" && !store.pinned && shown && Date.now() - shownAt > 400 && !itemDrag.item && !fileDrop.over) hide();
      });
      store.backend.onFileDrop((e) => void onFileDrop(e));
      // The first slide-in waits for the board to be laid out as it will show (its notes measured,
      // the fonts in): its frames are then the slide's alone. (Never more than a moment.)
      const cap = new Promise<void>((r) => setTimeout(r, 900));
      void Promise.race([Promise.all([store.whenBoardSettled(), document.fonts?.ready]), cap]).then(() => appear());
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
    if (onboarding || termsOpen) return;
    // (Never under another view: the picture viewer, New note and Tidy step away first.)
    closeOverBoard();
    settingsOpen = false;
    store.archiveOpen = null;
    searchOpen = true;
  }

  /** Note size: Ctrl+wheel, Ctrl + / Ctrl − / Ctrl 0. (Not behind a view over the board, where
   * the notes can't be seen changing.) */
  function stepSize(dir: -1 | 0 | 1) {
    if (store.over || onboarding || termsOpen) return;
    const i = SIZES.indexOf(store.size);
    const next = dir === 0 ? "comfortable" : SIZES[Math.max(0, Math.min(SIZES.length - 1, i + dir))];
    if (next !== store.size) store.setSetting("note_size", next);
  }

  // The wheel is listened to without holding up scrolling (a listener that may cancel it makes
  // every scroll wait for the page's script). Only while Ctrl is held down is the wheel's own
  // effect (the page zoom) cancelled; a touchpad pinch (a Ctrl+wheel with no key down) changes
  // the note size the same way, and the window's own zoom is off (Tauri's zoom hotkeys).
  $effect(() => {
    const wheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      stepSize(e.deltaY < 0 ? 1 : -1);
    };
    const hold = (e: WheelEvent) => e.ctrlKey && e.preventDefault();
    let held = false;
    const grip = (on: boolean) => {
      if (on === held) return;
      held = on;
      if (on) window.addEventListener("wheel", hold, { passive: false });
      else window.removeEventListener("wheel", hold);
    };
    const down = (e: KeyboardEvent) => e.key === "Control" && grip(true);
    const up = (e: KeyboardEvent) => e.key === "Control" && grip(false);
    const off = () => grip(false);
    window.addEventListener("wheel", wheel, { passive: true });
    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);
    window.addEventListener("blur", off);
    return () => {
      grip(false);
      window.removeEventListener("wheel", wheel);
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
      window.removeEventListener("blur", off);
    };
  });

  // ---- Back and Forward, like a browser ----
  // The mouse's side buttons, the keyboard's Browser Back / Forward keys, Alt+Left / Alt+Right
  // and Backspace (outside a text field) go one level back or forward in what is open; with
  // nothing open they leave the board alone.
  function step(dir: "back" | "forward", source: NavSource): boolean {
    // (Nor in the middle of carrying a line: nothing closes under the slip.)
    if (onboarding || termsOpen || itemDrag.item) return false;
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
  /** The keyboard is on nothing (no button, field, menu or panel has it): the board's own keys
   * (its selection, `store.focus`) are the ones that act. */
  function keyFree(): boolean {
    const a = document.activeElement;
    return !a || a === document.body || a === document.documentElement;
  }
  /** A selected board line keeps Backspace (task ↔ idea) unless the keyboard is elsewhere (in an
   * open view, a menu, on a button). */
  function boardOwnsBackspace(): boolean {
    if (!store.focus || !keyFree()) return false;
    if (!boardCovered()) return true;
    return look.panelOpen && store.focus.item != null;
  }
  // The keyboard taken elsewhere: the board's selection goes, so its keys never act on a line out
  // of sight. (Not for the line's or the note's own field, opened by Enter or F2 on it.) Taken by a
  // view, a menu, the Style panel or the toast, or by the button that opens a menu (a note's or a
  // line's ⋯), it is only put aside: once that is gone (or that menu closed again) and the
  // keyboard is back on the board (nowhere, or on that button), the board's place comes back, and
  // the arrows go on from the line you were on. Taken by a control on the board itself (Tab to a
  // note's + Add), it goes for good.
  let parked: { note: number; item: number | null } | null = null;
  /** The menu's button that put the place aside (pressed or reached), and whether its menu has
   * had the keyboard since: only then is the keyboard back on that button the menu closing. */
  let parkedOn: Element | null = null;
  let menuSeen = false;
  function park(f: typeof parked, on: Element | null = null) {
    parked = f;
    parkedOn = on;
    menuSeen = false;
  }
  // (A place given to the board meanwhile, by Search's Enter for one, is the place now.)
  $effect(() => {
    if (store.focus) park(null);
  });
  const AWAY = "[role='dialog'], [role='alertdialog'], [role='menu'], aside.panel, [data-modal-also]";
  /** The buttons that open a menu (a note's ⋯, a line's ⋯, the header's ⋯, Help). */
  const OPENER = "[aria-haspopup]:not([aria-haspopup='false'])";
  /** The keyboard on a menu's button, its menu closed again: as good as nowhere for the board's
   * place (a menu closed by Esc gives the keyboard back to its button). */
  function onClosedOpener(): HTMLElement | null {
    const a = document.activeElement as HTMLElement | null;
    return menuSeen && a?.matches?.(OPENER) && a.getAttribute("aria-expanded") !== "true" ? a : null;
  }
  /** A press on the board: its keyboard place goes (a click elsewhere), unless the press is on a
   * menu's button (a note's or a line's ⋯), or is a right-click, its menu opening: then it is put
   * aside, back once the menu closes. */
  function boardPress(e: PointerEvent) {
    const on = (e.target as Element | null)?.closest?.(OPENER);
    if (store.focus && (on || e.button === 2)) park(store.focus, on);
    store.focus = null;
  }
  function stillThere(f: { note: number; item: number | null }): boolean {
    return f.item != null ? store.items.some((i) => i.id === f.item) : isStrip(f.note) || store.notes.some((n) => n.id === f.note && !n.archived);
  }
  /** The board's place put aside comes back (the keyboard is on the board again, nothing over it). */
  function unpark(): boolean {
    const f = parked;
    const opener = onClosedOpener();
    if (!f || store.focus || !(keyFree() || opener) || boardCovered()) return false;
    park(null);
    if (!stillThere(f)) return false;
    // (Off the menu's button: the board's keys are the board's again.)
    opener?.blur();
    store.focus = f;
    return true;
  }
  $effect(() => {
    const moved = (e: FocusEvent) => {
      const el = e.target as Element | null;
      if (!el || el === document.body) return;
      const f = store.focus;
      if (!f) {
        // (In a menu or a view: still aside. Back on the menu's button from its menu, the menu
        // closing: it comes back. On the button just pressed, its menu opening: still aside. On
        // the board itself: what was put aside goes.)
        if (!parked) return;
        if (el.closest(AWAY)) menuSeen = true;
        else if (el.matches(OPENER) && menuSeen) soon();
        else if (el !== parkedOn) park(null);
        return;
      }
      const own = document.querySelector(f.item != null ? liveLine(f.item) : isStrip(f.note) ? "[data-inbox-strip]" : liveNote(f.note));
      // (Its menu's fields, a task's When, are the menu's: put aside, as in any menu.)
      if (own?.contains(el) && !el.closest(AWAY) && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || (el as HTMLElement).isContentEditable)) return;
      const opener = el.matches(OPENER);
      park(el.closest(AWAY) || opener ? f : null, opener ? el : null);
      menuSeen = !!el.closest(AWAY);
      store.focus = null;
    };
    // The keyboard back to nowhere (a view closing gives it back so), or on a menu's button with
    // its menu closed: the place comes back once the view or the menu is gone.
    let later = 0;
    function soon() {
      clearTimeout(later);
      let tries = 0;
      const look = () => {
        if (unpark() || !parked || !(keyFree() || document.activeElement?.matches(OPENER)) || ++tries > 20) return;
        later = window.setTimeout(look, 50);
      };
      later = window.setTimeout(look, 0);
    }
    const left = (e: FocusEvent) => {
      if (e.relatedTarget || !parked) return;
      soon();
    };
    document.addEventListener("focusin", moved);
    document.addEventListener("focusout", left);
    return () => {
      clearTimeout(later);
      document.removeEventListener("focusin", moved);
      document.removeEventListener("focusout", left);
    };
  });
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
    const any = settingsOpen || searchOpen || tidyOpen || look.panelOpen || viewer.open || store.templatesOpen || store.lookbackOpen || !!boardMenuAt || !!store.archiveOpen || !!reportOpen || !!whatsNew;
    settingsOpen = searchOpen = tidyOpen = false;
    store.archiveOpen = null;
    reportOpen = null;
    whatsNew = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    store.hideLookback();
    boardMenuAt = null;
    ideaMenu = null;
    store.focus = null;
    if (store.ruler) store.ruler = null;
    store.toast = null;
    // (Not over something opened meanwhile, like Help by F1.)
    setTimeout(() => {
      if (!onboarding && !settingsOpen && !searchOpen && !tidyOpen && !look.panelOpen && !viewer.open && !store.templatesOpen && !store.archiveOpen && !reportOpen && !whatsNew) tour.start();
    }, any && !reduce.matches ? 200 : 0);
  }
  // A full view over the board: the board waits to show notes put back from the archive.
  $effect(() => {
    store.over = settingsOpen || searchOpen || tidyOpen || !!store.archiveOpen || !!reportOpen || !!whatsNew || store.templatesOpen || viewer.open;
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
    ideaMenu = null;
    settingsOpen = false;
    reportOpen = null;
    whatsNew = null;
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
    reportOpen = null;
    whatsNew = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    boardMenuAt = null;
    ideaMenu = null;
    if (store.ruler?.mode === "click") store.ruler = null;
    settingsAt = { key, n: (settingsAt?.n ?? 0) + 1 };
    settingsOpen = true;
  }

  // ---- Report a problem ----
  function openReport(from: "board" | "settings") {
    if (onboarding) return;
    helpMenu.open = false;
    tour.close();
    searchOpen = tidyOpen = false;
    store.archiveOpen = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    boardMenuAt = null;
    ideaMenu = null;
    settingsOpen = false;
    whatsNew = null;
    reportOpen = from;
  }
  /** Back one level: to Settings › Help when it was opened from there, else the board. */
  function reportBack() {
    const from = reportOpen;
    reportOpen = null;
    if (from !== "settings") return;
    settingsAt = { key: "Help", n: (settingsAt?.n ?? 0) + 1 };
    settingsOpen = true;
  }

  // ---- What's new ----
  function openWhatsNew(from: "board" | "settings", shown: Shown) {
    if (onboarding || termsOpen || !shown.entries.length) return;
    helpMenu.open = false;
    tour.close();
    searchOpen = tidyOpen = false;
    store.archiveOpen = null;
    look.panelOpen = false;
    viewer.open = false;
    store.templatesOpen = false;
    boardMenuAt = null;
    ideaMenu = null;
    settingsOpen = false;
    reportOpen = null;
    whatsNew = { from, shown };
  }
  /** The running version (Rust's own, as Settings › Updates shows it). */
  async function runningVersion(): Promise<string> {
    if (updates.v?.current) return updates.v.current;
    return store.backend
      .updateStatus()
      .then((v) => v.current)
      .catch(() => "");
  }
  /** From the "?" menu or Settings › Updates: the running version's own part. */
  async function reopenWhatsNew(from: "board" | "settings") {
    const now = await runningVersion();
    openWhatsNew(from, { entries: ownSection(bundled(i18n.lang), now), since: null });
  }
  /** Back one level: to Settings › Updates when it was opened from there, else the board. */
  function whatsNewBack() {
    const from = whatsNew?.from;
    whatsNew = null;
    if (from !== "settings") return;
    settingsAt = { key: "Updates", n: (settingsAt?.n ?? 0) + 1 };
    settingsOpen = true;
  }
  // Once after an update, a moment after the panel shows: every version since the one you had
  // (whats_new_seen, whatsnew.ts). Never over the first-run window or the terms (after them,
  // then; the recovery screen comes before the board), never while the window is hidden (started
  // with Windows) or over something already opened: at the next show, then. Noted at once, so it
  // shows only once, whatever happens next; a first install only notes this version. (Only the
  // gates and the panel showing again arm it: the running version is read once, so the update
  // checks' news never brings it up later, in the middle of someone's work.)
  let whatsNewDone = false;
  let running = $derived(updates.v?.current ?? "");
  $effect(() => {
    if (whatsNewDone || !started || onboarding || termsOpen || !shown) return;
    const now = running;
    if (!now) return;
    const id = setTimeout(() => whatsNewOnce(now), reduce.matches ? 200 : 650);
    return () => clearTimeout(id);
  });
  /** Words being typed: a field with something in it has the keyboard. */
  function midTyping(): boolean {
    const a = document.activeElement as HTMLElement | null;
    if (!a) return false;
    if (a.isContentEditable) return true;
    return (a instanceof HTMLInputElement || a instanceof HTMLTextAreaElement) && a.value.trim() !== "";
  }
  function whatsNewOnce(now: string) {
    if (whatsNewDone) return;
    const plan = planAfterStart(store.settings.whats_new_seen, now, bundled(i18n.lang));
    const step = whatsNewStep(plan.kind, {
      hidden: !shown || document.hidden || store.peek,
      setup: onboarding || termsOpen,
      covered: boardCovered() || tour.open || !!itemDrag.item,
      typing: midTyping(),
    });
    if (step === "wait") return;
    whatsNewDone = true;
    if (step === "note" || step === "show") void store.setSetting("whats_new_seen", now);
    if (step === "show" && plan.kind === "show") openWhatsNew("board", plan);
  }
  /** Once, a moment after the first-run setup: a gentle offer, never started by itself. */
  function offerTour() {
    if (store.settings.tour_offered || store.settings.tour_seen) return;
    const offer = () => {
      if (tour.open || onboarding) return;
      // (After a word already showing, the recovery screen's for one: never over it.)
      if (store.toast) return void setTimeout(offer, 1000);
      void store.setSetting("tour_offered", Date.now());
      store.showToast(() => t("New here? A one-minute tour."), { label: () => t("Take the tour"), run: startTour }, 15_000);
    };
    setTimeout(offer, 1400);
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

  // (Said a moment after the selection rests: a quick run of arrows is one announcement.)
  let boardSays = $state("");
  $effect(() => {
    const words = focusWords();
    const id = setTimeout(() => (boardSays = words), words ? 150 : 0);
    return () => clearTimeout(id);
  });

  /** Tidy, New note and the picture viewer step away (each with its own closing) before Style,
   * Settings or Search opens: one would otherwise open under them, unseen, with their keys still
   * live. */
  function closeOverBoard() {
    tidyOpen = false;
    store.templatesOpen = false;
    viewer.open = false;
    // (Report a problem keeps what you typed for when it opens again.)
    reportOpen = null;
    whatsNew = null;
  }
  function toggleStyle() {
    // (Not under the first-run window or the terms: only they answer then.)
    if (onboarding || termsOpen) return;
    if (look.panelOpen) look.panelOpen = false;
    else {
      closeOverBoard();
      look.open("main");
      settingsOpen = false;
      // (Search too: the panel would open over it while its field kept the keyboard.)
      searchOpen = false;
      store.archiveOpen = null;
    }
  }
  function toggleSettings() {
    if (onboarding || termsOpen) return;
    settingsAt = null;
    settingsOpen = !settingsOpen;
    if (settingsOpen) closeOverBoard();
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
    if (e.defaultPrevented || composing(e)) return;
    // (The first-run window's rows of choices still take their arrows.)
    if (onboarding || termsOpen) {
      if (gridKey(e)) e.preventDefault();
      return;
    }
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
    // A line being carried: nothing opens or closes under the slip, nothing is undone from under
    // it (Esc, which calls the carry off, is the carry's own).
    if (itemDrag.item) return;
    // (A row or grid of choices with the keyboard on it takes its arrows first: a Style grid, the
    // note colours.)
    if (navKey(e) || gridKey(e) || scrollKey(e)) {
      e.preventDefault();
      return;
    }
    // Ctrl+Shift+S: Style · Ctrl+Shift+P: Settings ("paramètres"); Ctrl+, too, the usual
    // place for Settings. They type nothing, so they work while writing in a note too; again
    // closes.
    const ctrl = (e.ctrlKey || e.metaKey) && !e.altKey;
    if (ctrl && e.shiftKey && (latinKey(e) === "s" || latinKey(e) === "p")) {
      e.preventDefault();
      if (latinKey(e) === "s") toggleStyle();
      else toggleSettings();
      return;
    }
    if (ctrl && !e.shiftKey && e.key === ",") {
      e.preventDefault();
      toggleSettings();
      return;
    }
    const size = (e.ctrlKey || e.metaKey) && !e.altKey ? sizeKey(e) : null;
    if (size != null) {
      e.preventDefault();
      stepSize(size);
      return;
    }
    // (/ is a key of the board and the views over it, not of Tidy, the Archive, the picture viewer
    // or New note: there, with the keyboard on a choice or a card, it would close them by
    // surprise. The shortcut still opens Search from them, closing them first.)
    const slash = e.key === "/" && !typing() && !e.ctrlKey && !e.altKey && !e.metaKey && !tidyOpen && !store.archiveOpen && !reportOpen && !whatsNew && !viewer.open && !store.templatesOpen;
    if (matches(e, store.shortcuts.search) || slash) {
      e.preventDefault();
      openSearch();
      return;
    }
    if (e.key !== "Escape") {
      // The board by keyboard (only when nothing is open on top of it, and the keyboard is on the
      // board itself or the entry line, whose Down steps onto it; elsewhere, only Undo / Redo).
      // (Report a problem: its box has its own Undo, the board none.)
      const over = settingsOpen || searchOpen || tidyOpen || viewer.open || store.templatesOpen || !!reportOpen || !!whatsNew;
      const a = document.activeElement;
      // (The place put aside while a view was open, if it hasn't come back yet.)
      if (!over && !store.archiveOpen && keyFree()) unpark();
      if (!over && !store.archiveOpen && (keyFree() || a?.id === "capture" ? boardKey(e) : undoKey(e))) e.preventDefault();
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

  // Right-click on the board (a long press with a finger or a pen too): on a note or a line, its
  // own menu at the pointer (its ⋯ menu; an idea's short one, below); on an empty spot, the
  // board's: "Change background…". (Selected words and links keep the Copy menu, fields
  // WebView2's own; nothing opens while something is carried.) From the keyboard (Shift+F10, the
  // Menu key, on a button of a note or a line), at its ⋯.
  let boardMenuAt = $state.raw<Point | null>(null);
  /** Where on the board it was right-clicked (its "New note" goes there; null from the keyboard:
   * at the usual place, at the end). */
  let boardSpot: BoardPoint | null = null;
  function boardMenu(e: MouseEvent) {
    // (Selected words or a link: the Copy menu, below, took it.)
    if (e.defaultPrevented) return;
    const t = e.target as HTMLElement;
    // (No menu of Dobo's opens: the board's place, put aside by the press, goes, as on a click.
    // A right-click inside a menu already open puts nothing aside: the place that menu set aside
    // when it opened stays, to come back when it closes.)
    const none = () => void (t.closest(AWAY) || park(null));
    if (itemDrag.item || rightClick.carrying) return void (e.preventDefault(), none());
    if (tour.open) return none();
    const keys = fromKeys(e);
    const on = menuTargetOf(t);
    if (on?.kind === "none") return none();
    if (on) {
      // (Words selected under it, in the browser preview: the browser's own menu, to copy them.)
      if (!keys && selectedAt(window.getSelection(), e.clientX, e.clientY)) return none();
      e.preventDefault();
      const it = on.kind === "line" ? store.items.find((i) => i.id === on.id) : null;
      if (on.kind === "line" && !it) return none();
      // (The board's keyboard place, put aside by the press, moves to what was right-clicked.)
      if (parked) park({ note: it ? it.note_id : on.id, item: it?.id ?? null });
      askMenu(on.kind, on.id, keys ? null : { x: e.clientX, y: e.clientY });
      return;
    }
    if (t.closest(".slot, input, textarea, [contenteditable], a, button")) return none();
    e.preventDefault();
    boardSpot = keys ? null : (store.board?.point({ x: e.clientX, y: e.clientY }) ?? null);
    boardMenuAt = { x: e.clientX, y: e.clientY };
  }
  /** "New note" in the board's menu: an empty note where it was clicked, its title ready. */
  function newNoteHere() {
    const at = boardSpot;
    boardMenuAt = null;
    void store.fromTemplate(null, "", { at, fresh: true });
  }
  /** "New note from a template…": the New note window; its note goes where it was clicked. */
  function templateHere() {
    const at = boardSpot;
    boardMenuAt = null;
    store.openTemplates(at);
  }
  // The keyboard's right-click for a menu Dobo already opened from the key (Shift+F10 or the Menu
  // key on the board's selected note or line): set aside, never a second menu over it.
  $effect(() => {
    const key = (e: KeyboardEvent) => noteMenuKey(e);
    const menu = (e: MouseEvent) => {
      if (!swallowed()) return;
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener("keydown", key, true);
    window.addEventListener("keyup", key, true);
    window.addEventListener("contextmenu", menu, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("keyup", key, true);
      window.removeEventListener("contextmenu", menu, true);
    };
  });
  // An idea has no ⋯: its right-click (or Shift+F10, the Menu key) opens a short menu of its own:
  // Copy text, Make it a task, Make a new note of it, Delete. From the keyboard, under its words.
  let ideaMenu = $state.raw<{ id: number; at: Point } | null>(null);
  const ideaMotion = { y: -4 };
  $effect(() => {
    const a = rightClick.ask;
    if (a?.kind !== "line") return;
    untrack(() => {
      const it = store.items.find((i) => i.id === a.id);
      if (it?.kind !== "idea" || !takeMenu("line", it.id)) return;
      const words = document.querySelector(`${liveLine(it.id)} .idea-text`)?.getBoundingClientRect();
      const at = a.at ?? (words ? { x: words.left, y: words.bottom + 4 } : null);
      if (at) ideaMenu = { id: it.id, at };
    });
  });
  let ideaItem = $derived(ideaMenu ? store.items.find((i) => i.id === ideaMenu!.id) : undefined);
  $effect(() => {
    if (ideaMenu && !ideaItem) ideaMenu = null;
  });
  $effect(() => {
    if (!ideaMenu) return;
    return pushLayer(() => (ideaMenu = null));
  });
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
  // Right-click selected words (a folder's path in Settings, a message, a found line) or a link:
  // Copy, or Copy link (WebView2's own menu stays in fields only).
  let copyAt = $state<({ x: number; y: number } & CopyOffer) | null>(null);
  $effect(() => {
    const open = (e: MouseEvent) => {
      // (Only the words right-clicked: words selected elsewhere on the page aren't offered.)
      const offer = copyOffer(e.target, selectedAt(window.getSelection(), e.clientX, e.clientY));
      if (!offer || onboarding) return;
      e.preventDefault();
      copyAt = { ...offer, x: e.clientX, y: e.clientY };
    };
    document.addEventListener("contextmenu", open, true);
    return () => document.removeEventListener("contextmenu", open, true);
  });
  $effect(() => {
    if (!copyAt) return;
    return pushLayer(() => (copyAt = null));
  });
  /** How the board's menus come in (from above, or below when opened above the pointer). */
  const boardMotion = { y: -4, duration: 160 };
  const copyMotion = { y: -4, duration: 160 };
  async function copyOut(what: string) {
    copyAt = null;
    await store.copy(what);
  }
  /** Arrows, Home / End move through the board's menu, as in the app's other menus. */
  function ctxKeys(e: KeyboardEvent) {
    // (Tab leaves it: it closes, and Tab goes on.)
    if (e.key === "Tab") return void ((boardMenuAt = null), (copyAt = null), (ideaMenu = null));
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
    const up = async (e: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      if (frame) cancelAnimationFrame(frame);
      resizing = false;
      // (The system took the pointer: the width goes back to what it was.)
      await store.resizeDocked(e.type === "pointercancel" ? startW : want, true, true);
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
  /** The carried slip's size, as laid out, and the width its words need (up to its 260 px; more
   * than its size when narrowed over the hours). Read when the browser has them anyway, never
   * asked for. */
  function slipSize(node: HTMLElement) {
    const ro = new ResizeObserver(([e]) => {
      const b = e.borderBoxSize?.[0];
      itemDrag.slip = b ? { w: Math.round(b.inlineSize), h: Math.round(b.blockSize), full: Math.min(260, node.scrollWidth + 2) } : null;
    });
    ro.observe(node);
    return { destroy: () => (ro.disconnect(), (itemDrag.slip = null)) };
  }

  // An undo taking away a note made from a line: the line lifts off the note as a slip (its
  // words where they were), waits there as the note fades, then glides back to its place.
  $effect(() => {
    store.lineLifts = (id, after) => {
      const it = store.items.find((x) => x.id === id);
      const words = it && document.querySelector<HTMLElement>(`${liveLine(id)} ${it.kind === "idea" ? ".idea-text" : ".text"}`);
      const r = words?.getBoundingClientRect();
      if (!it || !r?.width) return;
      itemDrag.liftBack(it, { x: r.left, y: r.top + r.height / 2 }, after);
    };
    return () => (store.lineLifts = null);
  });

  function land(node: HTMLElement, l: Landing) {
    const from = `translate(${l.from.x + 14}px, ${l.from.y + 10}px) rotate(${l.tilt}deg) scale(1)`;
    // (The carried slip's size, known already: the same words in the same slip. Asking the new one
    // would lay the page out again in the middle of the drop.)
    const w = l.size?.w ?? node.offsetWidth;
    const h = l.size?.h ?? node.offsetHeight;
    const to = `translate(${l.to.x - w / 2}px, ${l.to.y - h / 2}px) rotate(0deg) scale(${l.scale ?? 0.15})`;
    node.style.transform = from;
    node.style.transformOrigin = "50% 50%";
    if (l.follow && !reduce.matches) {
      // Onto a new note: its place is looked at each frame (the board can move it as the line
      // leaves), the same path and timing as below otherwise.
      const follow = l.follow;
      const t0 = performance.now();
      const ease = (p: number) => 0.5 - Math.cos(Math.PI * p) / 2;
      let raf = 0;
      const tick = (now: number) => {
        const p = Math.min(1, (now - t0) / 300);
        const e = ease(p);
        const at = follow() ?? l.to;
        const x = l.from.x + 14 + (at.x - w / 2 - (l.from.x + 14)) * e;
        const y = l.from.y + 10 + (at.y - h / 2 - (l.from.y + 10)) * e;
        const s = 1 + ((l.scale ?? 0.15) - 1) * e;
        node.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) rotate(${(l.tilt * (1 - e)).toFixed(2)}deg) scale(${s.toFixed(3)})`;
        node.style.opacity = (1 - e).toFixed(3);
        if (p < 1) raf = requestAnimationFrame(tick);
        else itemDrag.landings = itemDrag.landings.filter((x) => x.key !== l.key);
      };
      raf = requestAnimationFrame(tick);
      return { destroy: () => cancelAnimationFrame(raf) };
    }
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

  /** A carry called off: the slip glides back onto its line's text (following the line, should the
   * board scroll meanwhile), straightens, and fades as the line shows again.
   * Reduced motion: it just fades where it is. */
  function goBack(node: HTMLElement, b: Back) {
    const h = node.offsetHeight;
    // (Lifted off a line: its words on the line's, 13 px in from its edge; else by the pointer.)
    const sx = b.onLine ? b.from.x - 13 : b.from.x + 14;
    const sy = b.onLine ? b.from.y - h / 2 : b.from.y + 10;
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
    // First it waits where it is (lifting a little and tilting, when it was lifted off a line).
    const WAIT = b.wait ?? 0;
    const LIFT = b.onLine && GO ? 4 : 0;
    const TILT = b.onLine && GO ? -1.5 : b.tilt;
    const t0 = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      if (now - t0 < WAIT) {
        const k = 1 - Math.pow(1 - (now - t0) / WAIT, 3);
        place(sx, sy - LIFT * k, b.tilt + (TILT - b.tilt) * k, 1);
        raf = requestAnimationFrame(tick);
        return;
      }
      const t = now - t0 - WAIT;
      const p = GO ? Math.min(1, t / GO) : 1;
      const e = 1 - Math.pow(1 - p, 3.4);
      const r = GO ? line()?.getBoundingClientRect() : null;
      // The slip's text sits 13 px in from its edge (its padding and border).
      const fx = sx;
      const fy = sy - LIFT;
      const tx = r && r.width ? r.left - 13 : fx;
      const ty = r && r.width ? r.top + r.height / 2 - h / 2 : fy;
      const fade = Math.max(0, Math.min(1, (t - GO) / FADE));
      if (p >= 1) arrived();
      place(fx + (tx - fx) * e, fy + (ty - fy) * e, TILT * (1 - e), 1 - fade);
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
  bind:this={shellEl}
  class:floating
  class:docked={!floating}
  class:left
  class:shown
  class:sliding
  class:preview={!inTauri}
  style:--dock-w="{store.win.width}px"
  onpointerover={onPointerOver}
  onpointerdown={boardPress}
>
  {#if store.error}
    <p class="error">{t("Couldn’t open your notes: {error}", { error: store.error })}</p>
  {:else if store.loaded}
    <BoardBackground />
    <div class="chrome" inert={chromeInert} data-cover>
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
    <div class="under" inert={boardInert} data-cover>
      <!-- svelte-ignore a11y_no_static_element_interactions (right-click: a shortcut to the Style panel, which also has its own button) -->
      <div
        class="scroll"
        bind:this={scroller}
        data-carry-scroll
        data-drop-empty
        class:file-over={fileDrop.over && fileDrop.note == null}
        class:textured={!["plain", "paper", "dotted"].includes(look.shown.bg)}
        class:dark-board={lookIsDark(look.shown)}
        class:held={heldBy(look.active) !== "none"}
        oncontextmenu={boardMenu}
      >
        <!-- The evening's look back (or today's, from its button): whole, at the top of the board,
             which makes room under it (the page scrolls). -->
        {#if store.lookback}<LookBack day={store.lookback} {scroller} />{/if}
        <div class="zoom" style:zoom={store.zoom} data-lang-later data-drop-empty>
          <!-- The Inbox: a strip across the board's width, above the notes. -->
          <InboxStrip />
          <!-- Today's hours, just under it, over the notes (taking no room): a strip while a task is
               carried, the hours when it is carried onto it or picked from a task's menu. -->
          <HoursRuler />
          <Board notes={store.boardNotes} {scroller} />
        </div>
        <!-- (While a line is carried, the "+ New note" card takes its place.) -->
        {#if !store.boardNotes.length && !itemDrag.item}
          <p class="nothing" in:arrive={{ delay: 160, duration: 280, y: 6 }} out:fadeQuick={{ duration: 120 }}>{t("No notes here yet. Type a line with #name to start one.")}</p>
        {/if}
        <!-- (The way to the notes put away sits by the note sizes, at the corner: SizeControl.) -->
      </div>
      <SizeControl />
    </div>
    <Toast />
    <!-- Where the board's keyboard is, for screen readers (said once the keys rest). -->
    <span class="visually-hidden" aria-live="polite">{boardSays}</span>
    {#if fileDrop.over}
      <div class="drop-hint" aria-hidden="true" transition:fadeQuick={{ duration: 160 }}>{fileDrop.note == null ? t("Drop to put it in the Inbox") : t("Drop on this note")}</div>
    {/if}
    {#if boardMenuAt}
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="ctx-veil" data-drag-through onpointerdown={() => (boardMenuAt = null)} oncontextmenu={(e) => (e.defaultPrevented || e.preventDefault(), (boardMenuAt = null))}></div>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="ctx" role="menu" tabindex="-1" use:atPointer={{ at: boardMenuAt, motion: boardMotion }} transition:pop|global={boardMotion} onkeydown={ctxKeys} use:ctxFocus>
        <button role="menuitem" onclick={newNoteHere}>{t("New note")}</button>
        <button role="menuitem" onclick={templateHere}>{t("New note from a template…")}</button>
        <hr />
        <button role="menuitem" onclick={() => ((boardMenuAt = null), look.open("main"), (settingsOpen = false))}>{t("Change background…")}</button>
        <button role="menuitem" onclick={() => openArchive("board")}
          ><span>{t("Archived notes…")}</span>{#if store.archived.length}<span class="count">{store.archived.length}</span>{/if}</button
        >
        <hr />
        <button role="menuitem" onclick={() => ((boardMenuAt = null), startTour())}>{t("Take the tour")}</button>
      </div>
    {/if}
    {#if ideaMenu && ideaItem}
      {@const idea = ideaItem}
      <!-- (Each choice takes the idea before it closes the menu: once closed, it is gone.) -->
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="ctx-veil" data-drag-through onpointerdown={() => (ideaMenu = null)} oncontextmenu={(e) => (e.defaultPrevented || e.preventDefault(), (ideaMenu = null))}></div>
      <!-- (The keyboard comes in, on its first line, and goes back where it was when it closes.) -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="ctx" role="menu" tabindex="-1" aria-label={t("Idea options")} use:atPointer={{ at: ideaMenu.at, motion: ideaMotion }} transition:pop|global={ideaMotion} onkeydown={ctxKeys} use:modal={{ initial: "button", trap: false }}>
        <button role="menuitem" onclick={() => (void store.copy(lineText(idea)), (ideaMenu = null))}>{t("Copy text")}</button>
        <button role="menuitem" onclick={() => {
            const it = idea;
            ideaMenu = null;
            void store.flipKind(it);
          }}><span>{t("Make it a task")}</span><span class="count">{keysText("Backspace")}</span></button>
        {#if !inMemory(idea.id)}<button role="menuitem" onclick={() => {
              const it = idea;
              ideaMenu = null;
              void itemDrag.toNewNote(it);
            }}>{t("Make a new note of it")}</button>{/if}
        <hr />
        <button role="menuitem" class="danger" onclick={() => {
            const it = idea;
            ideaMenu = null;
            void store.deleteItem(it);
          }}><span>{t("Delete")}</span><span class="count">{keysText("Del")}</span></button>
      </div>
    {/if}
    {#if look.panelOpen}
      <StylePanel onClose={() => (look.panelOpen = false)} />
    {/if}
    {#if copyAt}
      {@const offer = copyAt}
      <!-- (Over everything, the views too: what is copied may be in one of them.) -->
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="ctx-veil copy" data-drag-through onpointerdown={() => (copyAt = null)} oncontextmenu={(e) => {
          // (A right-click on words selected under it: the Copy menu, just opened there, stays.)
          if (e.defaultPrevented) return;
          e.preventDefault();
          copyAt = null;
        }}></div>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="ctx copy" role="menu" tabindex="-1" use:atPointer={{ at: offer, motion: copyMotion }} transition:pop|global={copyMotion} onkeydown={ctxKeys} use:ctxFocus>
        {#if offer.text}<button role="menuitem" onclick={() => copyOut(offer.text)}><span>{t("Copy")}</span><span class="count">Ctrl+C</span></button>{/if}
        {#if offer.link}<button role="menuitem" onclick={() => copyOut(offer.link!)}>{t("Copy link")}</button>{/if}
      </div>
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
      <Templates onClose={() => ((store.templatesOpen = false), (store.templateAt = null))} />
    {/if}
    {#if store.archiveOpen}
      <Archive from={store.archiveOpen} onBack={archiveBack} onClose={closeArchive} />
    {/if}
    {#if reportOpen}
      <Report from={reportOpen} onBack={reportBack} onClose={() => (reportOpen = null)} />
    {/if}
    {#if whatsNew}
      <WhatsNew from={whatsNew.from} shown={whatsNew.shown} onBack={whatsNewBack} onClose={() => (whatsNew = null)} />
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
  {#if !floating && store.loaded}
    <!-- The panel's inner edge: drag (or arrow keys) to change its width. A focusable separator is
         the ARIA window-splitter pattern. (Last in the panel, so Tab comes to it after the board,
         not first; it is fixed, so it shows where it always has.) -->
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
      inert={boardInert}
      data-tip={t("Drag to change the width")} data-tip-keys="Left|Right"
      onpointerdown={gripDown}
      onkeydown={gripKey}
    >
      <span></span>
    </div>
  {/if}
</div>

{#if store.peek && store.loaded}
  <Peek />
{/if}

{#if itemDrag.item}
  <!-- (On the pointer's left, it is placed by its own width: narrowed as the pointer moves, it
       stays put against the pointer, with no glide behind it.) -->
  <div
    class="slip"
    class:idea={itemDrag.item.kind === "idea"}
    class:to-new={itemDrag.target?.kind === "new"}
    style:transform="translate({itemDrag.x + 14}px, {itemDrag.y + 10}px) rotate({itemDrag.tilt}deg)"
    style:translate={itemDrag.time != null ? `${itemDrag.liftX === HOURS_LIFT.x ? `${HOURS_LIFT.x}px` : "calc(-100% - 28px)"} -${HOURS_LIFT.y}px` : itemDrag.leftSide ? "calc(-100% - 28px) 0px" : null}
    style:max-width={itemDrag.time != null && itemDrag.cap != null ? `${itemDrag.cap}px` : null}
    aria-hidden="true"
    use:slipSize
  >
    {itemDrag.item.text}
  </div>
{/if}
{#each itemDrag.backs as b (b.key)}
  <div class="slip back" class:idea={b.idea} style:max-width={b.cap ? `${b.cap}px` : null} aria-hidden="true" use:goBack={b}>
    {b.text}
  </div>
{/each}
{#each itemDrag.landings as l (l.key)}
  <div class="slip" class:idea={l.idea} style:max-width={l.size ? `${l.size.w}px` : null} aria-hidden="true" use:land={l}>
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
  /* A layer of its own while it slides (it is the whole window: nothing in it is drawn again
     during the slide), drawn with the page again once it rests, crisp. */
  .shell.sliding {
    will-change: transform, opacity;
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
    /* (Over the board under it.) */
    z-index: 3;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--ground) 94%, transparent);
    box-shadow: 0 1px 0 color-mix(in srgb, var(--ink) 7%, transparent);
  }
  /* (Only says what is inert under a view: its parts lay out in the panel as before.) */
  .under {
    display: contents;
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
  .ctx-veil {
    position: absolute;
    inset: 0;
    z-index: 60;
  }
  .ctx-veil.copy {
    z-index: 90;
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
  .ctx.copy {
    z-index: 91;
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
  .ctx button.danger {
    color: var(--late-ink);
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
    /* (Over the hours it rides a little higher: HOURS_LIFT.) */
    transition: translate 0.18s var(--ease-out);
  }
  @media (prefers-reduced-motion: reduce) {
    .slip {
      transition: none;
    }
  }
  .slip.back {
    animation: none;
  }
  /* Over "+ New note" or the board's empty space: dashed in the accent, as the card it makes. */
  .slip.to-new {
    border-style: dashed;
    border-color: var(--accent);
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
