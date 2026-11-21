<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { composing } from "../lib/compose";
  import { tick, untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import ViewHeader, { type Crumb } from "./ViewHeader.svelte";
  import RhythmFields from "./RhythmFields.svelte";
  import { store, defaultShortcuts, SIZES, type Shortcuts, type WorkHours } from "../lib/store.svelte";
  import { inTauri, type BackupInfo, type DataInfo } from "../lib/backend";
  import { pushLayer } from "../lib/layers";
  import { rhythmPresets } from "../lib/rhythm";
  import { APP_NAME } from "../lib/app";
  import { DOCK_PRESETS, presetName } from "../lib/dock";
  import { look } from "../lib/look.svelte";
  import type { DarkSwitch } from "../lib/looks";
  import type { MonitorInfo, PresetName, Rhythm } from "../lib/types";
  import { arrive, crumbIn, crumbOut, easeHeight, fadeQuick, fold, leave, sheet, reduced } from "../lib/motion";
  import { groupsOf, highlight, matchSettings } from "../lib/settingsFind";
  import { askTour } from "../lib/tour.svelte";
  import { checkNow, restartNow, updates } from "../lib/updates.svelte";
  import { dateFmt, dayName, fileSize, i18n, isKey, keysText, readChoice, switchLanguage, t, type Key, type LangChoice, words, type Words } from "../lib/i18n.svelte";

  // `at`: a section to open on (the "?" at the board's corner and F1 open Help); `n` changes on
  // each ask, so asking again while Settings is open goes there again.
  let { onClose, at = null }: { onClose: () => void; at?: { key: Key; n: number } | null } = $props();

  // Esc closes Settings; Back (the mouse's back button, Alt+Left, Backspace) leaves the search
  // first, and Forward brings that search back.
  $effect(() => pushLayer(onClose, { back: () => onBack(), forward: () => onForward() }));

  // ---- Where you are: the section under the header as you scroll, or the search ----
  let sheetEl = $state<HTMLElement>();
  let topEl = $state<HTMLElement>();
  let findEl = $state<HTMLInputElement>();
  let q = $state("");
  // The section's English name (its key); shown in the current language.
  let section = $state<Key>(untrack(() => at?.key) ?? "Appearance");
  function onScroll() {
    if (!sheetEl || !topEl) return;
    const line = topEl.getBoundingClientRect().bottom + 12;
    let cur = "";
    for (const sec of sheetEl.querySelectorAll<HTMLElement>("section:not(.gone)")) {
      if (sec.getBoundingClientRect().top <= line || !cur) cur = sec.dataset.key ?? cur;
      else break;
    }
    // At the very bottom, the last section is the one you're in even if it's short.
    if (sheetEl.scrollTop + sheetEl.clientHeight >= sheetEl.scrollHeight - 2) {
      const all = sheetEl.querySelectorAll<HTMLElement>("section:not(.gone)");
      if (all.length) cur = all[all.length - 1].dataset.key ?? cur;
    }
    if (cur && cur !== section && isKey(cur)) section = cur;
  }
  function toTop() {
    q = "";
    sheetEl?.scrollTo({ top: 0, behavior: reduced() ? "auto" : "smooth" });
  }
  let crumbs = $derived<Crumb[]>([{ label: t("Settings"), go: toTop }, { label: q.trim() ? t("Search: {q}", { q: q.trim() }) : t(section) }]);
  // Back: out of the search first (one level), else back to the board.
  let left = "";
  function onBack() {
    if (!q.trim()) return onClose();
    left = q;
    q = "";
  }
  function onForward() {
    if (left && !q.trim()) q = left;
  }

  /** Straight to a section: under the header at once when Settings opens there (its opening
   * motion plays where it lands), else a smooth scroll. */
  let opened = false;
  // What loads a moment later above it (the backups list) would push the section down: it is
  // kept in place until then, unless you scroll yourself.
  let pinned: { key: Key; until: number } | null = null;
  function place(key: Key, smooth: boolean) {
    const sec = sheetEl?.querySelector<HTMLElement>(`section[data-key="${key}"]`);
    if (!sheetEl || !sec || !topEl) return;
    sheetEl.scrollTo({ top: sec.offsetTop - topEl.offsetHeight, behavior: smooth ? "smooth" : "auto" });
  }
  function goTo(key: Key) {
    q = "";
    place(key, opened && !reduced());
    section = key;
    pinned = { key, until: performance.now() + 1500 };
  }
  $effect(() => {
    const a = at;
    if (a) untrack(() => goTo(a.key));
    opened = true;
  });
  $effect(() => {
    if (!sheetEl) return;
    const el = sheetEl;
    const ro = new ResizeObserver(() => {
      if (pinned && performance.now() < pinned.until) place(pinned.key, false);
    });
    for (const s of el.querySelectorAll("section")) ro.observe(s);
    const free = () => (pinned = null);
    el.addEventListener("wheel", free, { passive: true });
    el.addEventListener("pointerdown", free);
    el.addEventListener("keydown", free);
    return () => {
      ro.disconnect();
      el.removeEventListener("wheel", free);
      el.removeEventListener("pointerdown", free);
      el.removeEventListener("keydown", free);
    };
  });
  function seeKeys() {
    goTo("Keyboard");
  }

  // Ctrl+F while Settings is open: its own search field (not while a shortcut is being recorded:
  // then every key is the new shortcut's).
  $effect(() => {
    const key = (e: KeyboardEvent) => {
      if (recording) return;
      if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        e.stopPropagation();
        findEl?.focus();
        findEl?.select();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  });
  function findKey(e: KeyboardEvent) {
    // First Esc empties the field; the next one closes Settings. (An Esc that cancels a composed
    // character does neither.)
    if (e.key === "Escape" && q && !composing(e)) {
      e.preventDefault();
      e.stopPropagation();
      q = "";
    }
  }

  // Filtering: what doesn't match is set aside; what stays glides to its new place.
  let none = $state(false);
  function filter(text: string) {
    const root = sheetEl;
    if (!root) return;
    const secs = Array.from(root.querySelectorAll<HTMLElement>("section"));
    const groups = secs.map((s) => ({ s, title: s.querySelector("h3")?.textContent ?? "", key: s.dataset.key, gs: groupsOf(s) }));
    const flat = groups.flatMap((g) => g.gs.map((els) => ({ g, els, section: g.title, key: g.key, text: els.map((e) => e.textContent ?? "").join(" ") })));
    const found = matchSettings(text, flat);
    const moving = [...secs, ...flat.flatMap((f) => f.els)];
    const before = new Map(moving.map((el) => [el, el.classList.contains("gone") ? null : el.getBoundingClientRect()]));
    flat.forEach((f, i) => f.els.forEach((el) => el.classList.toggle("gone", !found.show[i])));
    for (const g of groups) g.s.classList.toggle("gone", !flat.some((f, i) => f.g === g && found.show[i]));
    none = !found.any;
    highlight(root, found.marks);
    if (reduced()) return;
    for (const el of moving) {
      if (el.classList.contains("gone")) continue;
      const a = before.get(el);
      const b = el.getBoundingClientRect();
      if (!a) el.animate([{ opacity: 0, transform: "translateY(6px)" }, { opacity: 1, transform: "none" }], { duration: 240, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
      else if (el.tagName === "SECTION" && Math.abs(a.top - b.top) > 1) el.animate([{ transform: `translateY(${a.top - b.top}px)` }, { transform: "none" }], { duration: 260, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
    }
  }
  $effect(() => {
    const text = q;
    tick().then(() => {
      filter(text);
      onScroll();
    });
  });
  // Lines that appear while searching (a switch turned on) follow the search too.
  $effect(() => {
    if (!sheetEl) return;
    let raf = 0;
    const mo = new MutationObserver(() => {
      if (!q.trim() || raf) return;
      raf = requestAnimationFrame(() => ((raf = 0), filter(q)));
    });
    mo.observe(sheetEl, { childList: true, subtree: true });
    return () => (mo.disconnect(), cancelAnimationFrame(raf), highlight(sheetEl!, []));
  });

  let importMsg = $state<Words>("");
  let width = $derived(store.win.width);
  let preset = $derived(presetName(width));
  let monitors = $state<MonitorInfo[]>([]);
  let presets = $state<Record<string, Rhythm> | null>(null);
  let autostart = $state(false);
  let recording = $state<keyof Shortcuts | null>(null);
  let keyError = $state<{ which: keyof Shortcuts; msg: string } | null>(null);

  $effect(() => {
    store.backend.listMonitors().then((m) => (monitors = m));
    rhythmPresets().then((p) => (presets = p));
    store.backend
      .getAutostart()
      .then((on) => (autostart = on))
      .catch(() => {});
  });

  // ---- Window ----
  async function setMode(mode: "docked" | "floating") {
    await store.setSetting("mode", mode);
    await store.refreshWindow();
    await store.backend.applyWindow();
  }
  async function setSide(side: "right" | "left") {
    await store.setSetting("side", side);
    await store.refreshWindow();
    await store.backend.applyWindow();
  }
  /** Narrow / Medium / Wide: shortcuts for this screen's width (it can also be dragged). */
  async function setWidth(w: number) {
    await store.resizeDocked(w, false, true);
  }
  let monitor = $derived((store.settings.monitor as string) || "mouse");
  async function setMonitor(key: string) {
    await store.setSetting("monitor", key);
    await store.backend.applyWindow();
  }
  // The drawing of the screens, as they are arranged.
  let layout = $derived.by(() => {
    if (!monitors.length) return null;
    const minX = Math.min(...monitors.map((m) => m.x));
    const minY = Math.min(...monitors.map((m) => m.y));
    const maxX = Math.max(...monitors.map((m) => m.x + m.width));
    const maxY = Math.max(...monitors.map((m) => m.y + m.height));
    const W = 320;
    const s = W / (maxX - minX);
    return {
      h: (maxY - minY) * s,
      boxes: monitors.map((m, i) => ({ m, n: i + 1, left: (m.x - minX) * s, top: (m.y - minY) * s, w: m.width * s, h: m.height * s })),
    };
  });

  // ---- Work hours ----
  let wh = $derived(store.workHours);
  const DAYS = [1, 2, 3, 4, 5, 6, 0] as const;
  function setWh(p: Partial<WorkHours>) {
    store.setWorkHours({ ...wh, ...p });
  }

  // ---- Reminders ----
  const PRESET_NAME = { gentle: "Gentle", normal: "Normal", insistent: "Insistent", custom: "Custom" } as const;
  const SIZE_NAME = { compact: "Small (size)", comfortable: "Normal (size)", large: "Large (size)" } as const;
  let defaultPreset = $derived(((store.settings.deadline_preset as PresetName) || "normal") as PresetName);
  let customDefault = $derived((store.settings.deadline_custom as Rhythm | undefined) ?? presets?.normal ?? null);
  interface Tidy {
    daily: boolean;
    time: string;
    when_many: boolean;
    many: number;
  }
  let tidy = $derived({ daily: true, time: "17:30", when_many: true, many: 10, ...((store.settings.tidy as Partial<Tidy>) ?? {}) });
  function setTidy(p: Partial<Tidy>) {
    store.setSetting("tidy", { ...tidy, ...p });
  }

  // ---- Keyboard ----
  const KEY_NAMES: Record<string, string> = { Space: "Space", Enter: "Enter", Backquote: "`", Slash: "/", Period: ".", Comma: ",", Minus: "-", Equal: "=" };
  function accelOf(e: KeyboardEvent): string | null {
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;
    let key: string | null = null;
    if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
    else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
    else if (/^F\d{1,2}$/.test(e.code)) key = e.code;
    else key = KEY_NAMES[e.code] ?? null;
    if (!key) return null;
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    return [...mods, key].join("+");
  }
  $effect(() => {
    const which = recording;
    if (!which) return;
    const pop = pushLayer(() => (recording = null));
    const onKey = async (e: KeyboardEvent) => {
      if (e.key === "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      const accel = accelOf(e);
      if (!accel) return;
      recording = null;
      try {
        await store.backend.setShortcut(which, accel);
        store.settings = { ...store.settings, [`shortcut_${which}`]: accel };
        keyError = null;
      } catch (err) {
        keyError = { which, msg: String(err) };
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      pop();
      window.removeEventListener("keydown", onKey, true);
    };
  });
  let keyRows = $derived<[keyof Shortcuts, string][]>([
    ["toggle", t("Open or hide {app}, from anywhere", { app: APP_NAME })],
    ["dump", t("Quick dump into the Inbox, from anywhere")],
    ["search", t("Search, inside the panel (also /)")],
  ]);
  /** A key in the lists below: our words for a few, else the key's own name. */
  const kx = (k: string) => (/^[w+]+$/.test(k) || !isKey(k) ? keysText(k) : t(k));

  async function setAuto(on: boolean) {
    autostart = on;
    try {
      await store.backend.setAutostart(on);
    } catch {
      autostart = !on;
    }
  }

  // ---- My day ----
  let day = $derived(store.day);
  function setDay(p: Partial<typeof day>) {
    store.setSetting("day", { ...day, ...p });
  }

  // ---- Backups ----
  let backups = $state<BackupInfo[]>([]);
  let backupDir = $state("");
  let backupMsg = $state<Words>("");
  let confirmRestore = $state<string | null>(null);
  async function loadBackups() {
    try {
      backups = await store.backend.backupList();
      backupDir = await store.backend.backupFolder();
    } catch (e) {
      backupMsg = String(e);
    }
  }
  $effect(() => {
    loadBackups();
  });
  async function backupNow() {
    backupMsg = () => t("Backing up…");
    try {
      const b = await store.backend.backupNow();
      backupMsg = () => t("Backed up ({size}).", { size: fileSize(b.bytes) });
      await loadBackups();
    } catch (e) {
      backupMsg = () => t("Couldn't back up: {error}", { error: String(e) });
    }
  }
  async function chooseFolder() {
    const p = await store.backend.pickFolder();
    if (!p) return;
    await store.setSetting("backup_dir", p);
    await loadBackups();
  }
  async function restore(path: string) {
    confirmRestore = null;
    backupMsg = () => t("Restoring…");
    try {
      await store.backend.backupRestore(path);
      // Undo steps recorded before point at notes and lines that were just replaced.
      store.history.clear();
      await store.reload();
      backupMsg = () => t("Restored. Your notes from before are kept as a backup too.");
      await loadBackups();
    } catch (e) {
      backupMsg = () => t("Couldn't restore: {error}", { error: String(e) });
    }
  }
  function when(ms: number) {
    return dateFmt({ weekday: "short", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" }).format(new Date(ms));
  }
  const reasonName: Record<string, Key> = { daily: "Daily", manual: "By hand", "before-restore": "Before a restore" };

  async function importAgain() {
    importMsg = () => t("Reading Sticky Notes…");
    try {
      const n = await store.backend.importStickyNotes();
      await store.reload();
      importMsg = () => (n ? t("Added {n} notes from Sticky Notes.", { n }) : t("No sticky notes found on this computer."));
    } catch (e) {
      importMsg = () => t("Couldn't read Sticky Notes: {error}", { error: String(e) });
    }
  }

  // ---- Appearance (the same setting as Style › Dark mode) ----
  let dm = $derived(look.darkMode);
  const APPEARANCE: [DarkSwitch, Key, Key][] = [
    ["light", "Light", "Always light"],
    ["dark", "Dark", "Always dark"],
    ["windows", "Follow Windows", "Light or dark like Windows"],
    ["hours", "By hours", "Dark between two times"],
  ];
  const APPEARANCE_NOTE: Record<DarkSwitch, Key> = {
    light: "Always light.",
    dark: "Always dark.",
    windows: "Light or dark, like Windows.",
    hours: "Dark between these two times, light the rest of the day.",
  };

  // ---- About: this copy (installed or portable) and where its notes are ----
  let info = $state<DataInfo | null>(null);
  $effect(() => {
    store.backend
      .dataInfo()
      .then((d) => (info = d))
      .catch(() => {});
  });
  const KIND_NAME: Record<DataInfo["kind"], Key> = { installed: "Installed", portable: "Portable", dev: "Development build" };
  function kindNote(d: DataInfo): string {
    if (d.kind === "installed") return t("Installed on this computer. Updating or reinstalling keeps your notes.");
    if (d.kind === "dev") return t("A development build, with notes of its own in the project folder.");
    if (d.kept_existing) return t("Portable, using the notes already on this computer.");
    if (d.fallback) return t("Portable, but the folder next to {app}.exe can’t be written: the notes are in your user profile.", { app: APP_NAME });
    return t("Portable: notes, pictures and backups stay in the data folder next to {app}.exe. Move or copy the whole folder.", { app: APP_NAME });
  }
  let openMsg = $state<Words>("");
  async function openData() {
    try {
      openMsg = "";
      await store.backend.openDataFolder();
    } catch (e) {
      openMsg = () => t("Couldn’t open the folder: {error}", { error: String(e) });
    }
  }

  // ---- Updates (src/lib/updates.svelte.ts) ----
  let upd = $derived(updates.v);
  // The line says what is known; when that's only the version, the words under it don't say it
  // again.
  let updVersionOnly = $derived(!upd || !((upd.ready && upd.latest) || upd.busy || upd.latest || (upd.checked && !upd.error)));
  let updLine = $derived.by(() => {
    if (!upd) return t("Version {version}", { version: info?.version ?? "" });
    if (upd.ready && upd.latest) return t("{app} {version} is ready", { app: APP_NAME, version: upd.latest });
    if (upd.busy) return t("Checking for updates…");
    if (upd.latest) return t("{app} {version} is out", { app: APP_NAME, version: upd.latest });
    if (upd.checked && !upd.error) return t("{app} is up to date", { app: APP_NAME });
    return t("Version {version}", { version: upd.current });
  });
  let updSub = $derived.by(() => {
    if (upd?.ready) return t("It starts the next time {app} opens, or now with Restart now.", { app: APP_NAME });
    const last = upd?.last_checked;
    if (updVersionOnly) return last ? t("Last checked {when}", { when: when(last) }) : t("Not checked yet");
    const version = upd?.current ?? info?.version ?? "";
    return last ? t("Version {version} · last checked {when}", { version, when: when(last) }) : t("Version {version} · not checked yet", { version });
  });

  // ---- Language (Follow Windows · English · Français): each name in its own language ----
  let langChoice = $derived(readChoice(store.settings.language));
  const LANGS: [LangChoice, string | null][] = [
    ["system", null],
    ["en", "English"],
    ["fr", "Français"],
  ];
  function setLanguage(c: LangChoice) {
    if (c === langChoice) return;
    void switchLanguage(c, () => store.setSetting("language", c), sheetEl);
  }
</script>

<div class="sheet" role="dialog" aria-label={t("Settings")} bind:this={sheetEl} onscroll={onScroll} transition:sheet|global>
  <div class="top" bind:this={topEl} data-drag-area={store.win.mode === "floating" || undefined}>
    <ViewHeader {crumbs} big={24} {onBack} {onClose} />
    <!-- (The whole field is a control: its icon and edges never move the window.) -->
    <div class="find-field" data-no-drag>
      <Icon name="search" size={16} />
      <label for="settings-find" class="visually-hidden">{t("Search settings")}</label>
      <input id="settings-find" bind:this={findEl} bind:value={q} onkeydown={findKey} placeholder={t("Search settings · Ctrl+F")} data-tip={t("Search settings")} data-tip-keys="Ctrl+F" autocomplete="off" spellcheck="false" />
      {#if q}
        <button class="clear" transition:fadeQuick aria-label={t("Clear the search")} data-tip={t("Clear")} data-tip-keys="Esc" onclick={() => ((q = ""), findEl?.focus())}><Icon name="close" size={14} stroke={2} /></button>
      {/if}
    </div>
  </div>
  {#if none}
    <p class="nothing" transition:sheet>{t("Nothing in Settings matches “{q}”. Try one word, like “dark”, “shortcut”, “backup” or “rappel”.", { q: q.trim() })}</p>
  {/if}

  <!-- The same switch as Style › Dark mode and the sun/moon in the header (one setting). -->
  <section data-key="Appearance">
    <h3>{t("Appearance")}</h3>
    <div class="tabs appear" role="radiogroup" aria-label={t("Appearance")}>
      {#each APPEARANCE as [k, label, title] (k)}
        <button role="radio" aria-checked={dm.mode === k} class:on={dm.mode === k} data-tip={t(title)} onclick={() => look.setDarkMode({ ...dm, mode: k })}>{t(label)}</button>
      {/each}
    </div>
    {#if dm.mode === "hours"}
      <div class="hours appear-hours" transition:fold={{ duration: 260 }}>
        <label>{t("Dark from")} <input type="time" aria-label={t("Dark from")} value={dm.from} onchange={(e) => e.currentTarget.value && look.setDarkMode({ ...dm, from: e.currentTarget.value })} /></label>
        <label>{t("until")} <input type="time" aria-label={t("Dark until")} value={dm.until} onchange={(e) => e.currentTarget.value && look.setDarkMode({ ...dm, until: e.currentTarget.value })} /></label>
      </div>
    {/if}
    {@render noteSwap(dm.mode, `${t(APPEARANCE_NOTE[dm.mode])} ${t("Dark mode darkens the app and the notes; the board keeps its own look unless your theme has a Dark style (Style › Themes).")}`)}
    <div class="line lang">
      <span>{t("Language / Langue")}{#if langChoice === "system"}<small transition:fold={{ y: 0, duration: 240 }}>{i18n.system === "fr" ? t("Now in French, like Windows.") : t("Now in English, like Windows.")}</small>{/if}</span>
      <div class="seg" role="radiogroup" aria-label={t("Language")}>
        {#each LANGS as [k, name] (k)}
          <button role="radio" aria-checked={langChoice === k} class:on={langChoice === k} lang={k === "system" ? undefined : k} onclick={() => setLanguage(k)}>{name ?? t("Follow Windows")}</button>
        {/each}
      </div>
    </div>
  </section>

  <section data-key="Window">
    <h3>{t("Window")}</h3>
    <div class="modes">
      <button class="mode" class:on={store.win.chosen === "docked"} aria-pressed={store.win.chosen === "docked"} onclick={() => setMode("docked")} disabled={!store.win.docked_possible}>
        <span class="pic docked" aria-hidden="true"><span></span></span>
        <span class="mode-name">{t("Docked to the edge")}</span>
        <span class="mode-text">{store.win.docked_possible ? t("Slides in with the shortcut, hides when you click away.") : t("Not possible on this desktop (Wayland).")}</span>
      </button>
      <button class="mode" class:on={store.win.chosen === "floating"} aria-pressed={store.win.chosen === "floating"} onclick={() => setMode("floating")}>
        <span class="pic floating" aria-hidden="true"><span></span></span>
        <span class="mode-name">{t("A floating window")}</span>
        <span class="mode-text">{t("Put it anywhere, any size; keep it on top if you like.")}</span>
      </button>
    </div>
    {#if store.win.mode === "floating"}
      <label class="line check" transition:fold>
        <span>
          {t("Always on top")}
          <small>{t("Stays above your other windows (also the button in the header).")}</small>
        </span>
        <input type="checkbox" checked={store.win.float_on_top} onchange={(e) => store.setFloatOnTop(e.currentTarget.checked)} />
        <span class="sw" aria-hidden="true"></span>
      </label>
    {/if}
    {#if store.win.mode === "docked"}
      <div class="line" transition:fold>
        <span>{t("Side of the screen")}</span>
        <div class="seg" role="group" aria-label={t("Side")}>
          <button class:on={store.win.side === "left"} aria-pressed={store.win.side === "left"} onclick={() => setSide("left")}>{t("Left")}</button>
          <button class:on={store.win.side === "right"} aria-pressed={store.win.side === "right"} onclick={() => setSide("right")}>{t("Right")}</button>
        </div>
      </div>
      <div class="line" transition:fold>
        <span>
          {t("Width")}
          <span class="small-swap">
            {#key !!preset}<small in:crumbIn out:crumbOut>{preset ? t("Or drag the panel’s inner edge.") : t("Custom · {n} px", { n: width })}</small>{/key}
          </span>
        </span>
        <div class="seg" role="group" aria-label={t("Width")}>
          {#each DOCK_PRESETS as [w, label] (w)}
            <button class:on={preset === label} aria-pressed={preset === label} onclick={() => setWidth(w)}>{t(label)}</button>
          {/each}
        </div>
      </div>
      <label class="line check" transition:fold>
        <span>
          {store.win.side === "left" ? t("Also open when the mouse reaches the left edge") : t("Also open when the mouse reaches the right edge")}
          <small>{t("Push the pointer against the edge for a moment.")}</small>
        </span>
        <input type="checkbox" checked={!!store.settings.edge_open} onchange={(e) => store.setSetting("edge_open", e.currentTarget.checked)} />
        <span class="sw" aria-hidden="true"></span>
      </label>
      {#if store.settings.edge_open}
        <label class="line check sub" transition:fold>
          <span>
            {t("Peek at the titles first")}
            <small>{t("Your notes’ titles slide out like index cards; click one to open it there. Stay at the edge to open everything.")}</small>
          </span>
          <input type="checkbox" checked={!!store.settings.edge_peek} onchange={(e) => store.setSetting("edge_peek", e.currentTarget.checked)} />
          <span class="sw" aria-hidden="true"></span>
        </label>
      {/if}
    {/if}
  </section>

  <section data-key="Notes">
    <h3>{t("Notes")}</h3>
    <div class="line">
      <span>{t("Note size")} <small>{t("Also Ctrl+wheel, or the A A A at the board’s corner.")}</small></span>
      <div class="seg" role="group" aria-label={t("Note size")}>
        {#each SIZES as s (s)}
          <button class:on={store.size === s} aria-pressed={store.size === s} onclick={() => store.setSetting("note_size", s)}>{t(SIZE_NAME[s])}</button>
        {/each}
      </div>
    </div>
    <div class="line">
      <span>{t("Dates written in a note")} <small>{t("“Call the client demain 9h”: the date gets a dotted line and can become a reminder.")}</small></span>
      <div class="seg" role="group" aria-label={t("Dates written in a note")}>
        {#each [["ask", "Ask"], ["auto", "Automatic"], ["off", "Off"]] as const as [k, label] (k)}
          <button class:on={store.smartDates === k} aria-pressed={store.smartDates === k} onclick={() => store.setSetting("smart_dates", k)}>{t(label)}</button>
        {/each}
      </div>
    </div>
    <p class="note">{t("Automatic adds the reminder only when the date is certain (a day and an hour, like “lundi 14h30”).")}</p>
  </section>

  <section data-key="Archive">
    <h3>{t("Archive")}</h3>
    <div class="line">
      <span
        >{t("Archived notes")}
        <small>{store.archived.length ? t("{n} notes put away, their reminders quiet. Restore or delete them there.", { n: store.archived.length }) : t("None yet. “Archive note” in a note’s menu puts it away without deleting it.")}</small></span
      >
      <button class="small" onclick={() => store.openArchive("settings")}>{t("Open the archive")}</button>
    </div>
  </section>

  <section data-key="My day">
    <h3>{t("My day")}</h3>
    <div class="hours">
      <label>{t("Starts at")} <input type="time" value={day.start} onchange={(e) => setDay({ start: e.currentTarget.value || "08:00" })} /></label>
      <label>{t("ends at")} <input type="time" value={day.end} onchange={(e) => setDay({ end: e.currentTarget.value || "22:00" })} /></label>
    </div>
    <p class="note">{t("Used for “Later… this evening / tomorrow morning”, the hours shown when you drag a task onto a time, and the evening look back (at the end of work hours on work days, else at the end of your day).")}</p>
  </section>

  <section data-key="Screen">
    <h3>{t("Screen")}</h3>
    <div class="line">
      <span>{t("Show on")}</span>
      <div class="seg" role="group" aria-label={t("Show on")}>
        <button class:on={monitor === "mouse"} aria-pressed={monitor === "mouse"} onclick={() => setMonitor("mouse")}>{t("Screen with the mouse")}</button>
        <button class:on={monitor !== "mouse"} aria-pressed={monitor !== "mouse"} onclick={() => monitors[0] && setMonitor(monitor !== "mouse" ? monitor : (monitors.find((m) => m.primary) ?? monitors[0]).key)}
          >{t("One screen")}</button
        >
      </div>
    </div>
    {#if layout}
      <div class="screens" style:height="{layout.h + 8}px" role="radiogroup" aria-label={t("Screens")}>
        {#each layout.boxes as b (b.m.key)}
          <button
            class="screen"
            class:on={monitor === b.m.key}
            class:dim={monitor === "mouse"}
            role="radio"
            aria-checked={monitor === b.m.key}
            aria-label={t("Screen {n}, {w} by {h}", { n: b.n, w: b.m.width, h: b.m.height })}
            style:left="{b.left}px"
            style:top="{b.top}px"
            style:width="{b.w - 4}px"
            style:height="{b.h - 4}px"
            onclick={() => setMonitor(b.m.key)}
          >
            <span class="num">{b.n}</span>
            <span class="res">{b.m.width}×{b.m.height}{b.m.primary ? ` · ${t("main")}` : ""}</span>
          </button>
        {/each}
      </div>
      {@render noteSwap(monitor === "mouse", monitor === "mouse" ? t("Opens on whichever screen the mouse is on. Click a screen to always use it.") : t("Always opens on the highlighted screen while it is connected."))}
    {/if}
  </section>

  <section data-key="Work hours">
    <h3>{t("Work hours")}</h3>
    <label class="line check">
      <span>{t("Switch between Work and Personal by the clock")}</span>
      <input type="checkbox" checked={wh.on} onchange={(e) => setWh({ on: e.currentTarget.checked })} />
      <span class="sw" aria-hidden="true"></span>
    </label>
    {#if wh.on}
      <div transition:fold>
      <div class="days" role="group" aria-label={t("Days")}>
        {#each DAYS as d (d)}
          {@const on = wh.days.includes(d)}
          <button class:on aria-pressed={on} onclick={() => setWh({ days: on ? wh.days.filter((x) => x !== d) : [...wh.days, d] })}>{dayName(d)}</button>
        {/each}
      </div>
      <div class="hours">
        <label>{t("Work from")} <input type="time" value={wh.from} onchange={(e) => setWh({ from: e.currentTarget.value || "09:00" })} /></label>
        <label>{t("until")} <input type="time" value={wh.until} onchange={(e) => setWh({ until: e.currentTarget.value || "19:00" })} /></label>
        <span class="soft">{t("then Personal")}</span>
      </div>
      <p class="note"><Icon name="bell" size={14} /> {t("Reminders always ring. When one from the other space rings, the board switches to it. You can still switch by hand; the clock takes over again at its next change.")}</p>
      </div>
    {/if}
  </section>

  <section data-key="Reminders">
    <h3>{t("Reminders")}</h3>
    <div class="line">
      <span>{t("Default rhythm for deadlines")} <small>{t("End a line with ! to make a deadline.")}</small></span>
    </div>
    <div class="tabs" role="tablist" aria-label={t("Default rhythm")}>
      {#each ["gentle", "normal", "insistent", "custom"] as const as p (p)}
        <button role="tab" class:on={defaultPreset === p} aria-selected={defaultPreset === p} onclick={() => store.setSetting("deadline_preset", p)}>{t(PRESET_NAME[p])}</button>
      {/each}
    </div>
    {#if defaultPreset === "custom" && customDefault}
      <div transition:fold><RhythmFields value={customDefault} onSave={(r) => store.setSetting("deadline_custom", r)} /></div>
    {:else if presets}
      {@const r = presets[defaultPreset]}
      <div transition:fold>
      {@render noteSwap(defaultPreset, t("Starts {start} min before, again at {pre} min before, then every {every} min after the time, at most {max} times. After “I'm on it”, asks “Really done?” {checkin} min later; after “Not yet”, {notyet} min later.", { start: r.start, pre: r.pre.join(" · "), every: r.every, max: r.max, checkin: r.checkin, notyet: r.notyet }))}
      </div>
    {/if}
    <p class="note">{t("A simple reminder rings at the time, then again every 5 min until you answer, at most 6 times.")}</p>

    <h4>{t("Tidy the inbox")}</h4>
    <label class="line check">
      <span>{t("Every day at")} <input class="inline" type="time" value={tidy.time} onchange={(e) => setTidy({ time: e.currentTarget.value || "17:30" })} onclick={(e) => e.stopPropagation()} /></span>
      <input type="checkbox" checked={tidy.daily} onchange={(e) => setTidy({ daily: e.currentTarget.checked })} />
      <span class="sw" aria-hidden="true"></span>
    </label>
    <label class="line check">
      <span>{t("When")} <input class="inline num" type="number" min="2" max="99" value={tidy.many} onchange={(e) => setTidy({ many: Math.max(2, Number(e.currentTarget.value) || 10) })} onclick={(e) => e.stopPropagation()} /> {t("things wait")}</span>
      <input type="checkbox" checked={tidy.when_many} onchange={(e) => setTidy({ when_many: e.currentTarget.checked })} />
      <span class="sw" aria-hidden="true"></span>
    </label>
  </section>

  <section data-key="Keyboard">
    <h3>{t("Keyboard")}</h3>
    {#each keyRows as [which, label] (which)}
      <div class="line">
        <span>{label}</span>
        <span class="keyset">
          <span class="keys" class:rec={recording === which}><span class="tswap">{#key recording === which}<span in:crumbIn out:crumbOut>{recording === which ? t("Press the new keys…") : keysText(store.shortcuts[which])}</span>{/key}</span></span>
          <button class="small" onclick={() => ((recording = recording === which ? null : which), (keyError = null))}><span class="tswap">{#key recording === which}<span in:crumbIn out:crumbOut>{recording === which ? t("Cancel") : t("Change")}</span>{/key}</span></button>
          {#if store.shortcuts[which] !== defaultShortcuts[which] && recording !== which}
            <button class="link" transition:fadeQuick onclick={async () => {
              try {
                await store.backend.setShortcut(which, defaultShortcuts[which]);
                store.settings = { ...store.settings, [`shortcut_${which}`]: defaultShortcuts[which] };
              } catch (e) {
                keyError = { which, msg: String(e) };
              }
            }}>{t("Reset")}</button>
          {/if}
        </span>
      </div>
      {#if keyError?.which === which}<p class="error" transition:fold>{keyError.msg}</p>{/if}
    {/each}
    <div class="line">
      <span>{t("Add a task · an idea · a deadline")}</span>
      <span class="keys">{t("Enter · Shift+Enter · end with !")}</span>
    </div>
    <h4>{t("On the board")}</h4>
    <div class="keymap">
      {#each [["Down", "from the empty entry line: onto the board"], ["Up / Down", "move between lines"], ["Left / Right", "move between notes"], ["Space", "tick the task"], ["Enter · F2", "edit the line (on a note: rename it; double-click works too)"], ["Backspace", "task ↔ idea (also at the start of a line you’re editing)"], ["Delete", "delete the line"], ["Ctrl+Up / Down", "move the line"], ["F", "fold or unfold the note"], ["Ctrl+Z · Ctrl+Y", "undo · redo"], ["Ctrl + / Ctrl − / Ctrl 0", "note size"], ["Page Up / Down · Home / End", "scroll the board"], ["Esc", "leave the board, then hide"]] as const as [k, what] (k)}
        <span class="keys">{kx(k)}</span><span class="what">{t(what)}</span>
      {/each}
    </div>
    <h4>{t("Everywhere")}</h4>
    <div class="keymap">
      {#each [["Alt+Left · Backspace · mouse back", "back one level (a page, then out of the view)"], ["Alt+Right · mouse forward", "forward again to where you just were"], ["Esc", "close what is open, straight back to the board"], ["Ctrl+Shift+S", "Style (again: close it)"], ["Ctrl+Shift+P · Ctrl+,", "Settings (again: close them)"], ["Ctrl+F (in Settings)", "search the settings"], ["Arrows · Home / End", "move between cards, colours and choices; Enter or Space picks"], ["F2 · double-click", "rename your own style, theme or mix"], ["F1", "help and the quick tour"]] as const as [k, what] (k)}
        <span class="keys">{kx(k)}</span><span class="what">{t(what)}</span>
      {/each}
    </div>
  </section>

  <section data-key="Backups">
    <h3>{t("Backups")}</h3>
    <label class="line check">
      <span>{t("Back up every day")} <small>{t("A copy of your notes and pictures; the last 14 are kept.")}</small></span>
      <input type="checkbox" checked={store.settings.backup_auto !== false} onchange={(e) => store.setSetting("backup_auto", e.currentTarget.checked)} />
      <span class="sw" aria-hidden="true"></span>
    </label>
    <div class="line">
      <span>{t("Folder")} <small class="path" data-tip={backupDir}>{backupDir}</small></span>
      <span class="keyset">
        <button class="small" onclick={chooseFolder}>{t("Choose…")}</button>
        {#if store.settings.backup_dir}<button class="link" transition:fadeQuick onclick={async () => (await store.setSetting("backup_dir", ""), await loadBackups())}>{t("Default")}</button>{/if}
      </span>
    </div>
    <div class="line">
      <span>{t("Make one now")}</span>
      <button class="small" onclick={backupNow}>{t("Back up now")}</button>
    </div>
    {#if backupMsg}<p class="note" transition:fold>{words(backupMsg)}</p>{/if}
    {#if backups.length}
      <div transition:fold>
      <h4>{t("Your backups")}</h4>
      <div class="backups">
        {#each backups as b (b.path)}
          <div class="backup" transition:fold>
            <span class="b-when">{when(b.at)}</span>
            <span class="b-why">{reasonName[b.reason] ? t(reasonName[b.reason]) : b.reason} · {fileSize(b.bytes)}</span>
            <span class="aswap">
            {#if confirmRestore === b.path}
              <span class="keyset" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>
                <button class="small" onclick={() => (confirmRestore = null)}>{t("Keep mine")}</button>
                <button class="small strong" onclick={() => restore(b.path)}>{t("Restore this")}</button>
              </span>
            {:else}
              <span class="keyset" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}><button class="small" onclick={() => (confirmRestore = b.path)}>{t("Restore…")}</button></span>
            {/if}
            </span>
          </div>
        {/each}
      </div>
      <p class="note">{t("Restoring first backs up the notes you have now, so nothing is lost.")}</p>
      </div>
    {/if}
  </section>

  <section data-key="Start">
    <h3>{t("Start")}</h3>
    <label class="line check">
      <span>{t("Start {app} with the computer", { app: APP_NAME })} <small>{t("Reminders only ring while it runs; it waits in the tray.")}</small></span>
      <input type="checkbox" checked={autostart} onchange={(e) => setAuto(e.currentTarget.checked)} />
      <span class="sw" aria-hidden="true"></span>
    </label>
    {#if inTauri}
      <div class="line">
        <span>{t("Bring in the notes from Windows Sticky Notes again")}</span>
        <button class="small" onclick={importAgain}>{t("Import")}</button>
      </div>
      {#if importMsg}<p class="note" transition:fold>{words(importMsg)}</p>{/if}
    {/if}
  </section>

  <section data-key="Updates">
    <h3>{t("Updates")}</h3>
    <label class="line check">
      <span>{t("Check for updates automatically")} <small>{t("Once a day. A new version is downloaded and checked, then used the next time {app} starts. Your notes are never touched.", { app: APP_NAME })}</small></span>
      <input type="checkbox" checked={store.settings.update_auto !== false} onchange={(e) => store.setSetting("update_auto", e.currentTarget.checked)} />
      <span class="sw" aria-hidden="true"></span>
    </label>
    <div class="line">
      <span>{updLine} <small>{updSub}</small></span>
      <span class="aswap">
        {#if upd?.ready}
          <span class="keyset" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}><button class="small strong" onclick={restartNow}>{t("Restart now")}</button></span>
        {:else}
          <span class="keyset" in:arrive={{ delay: 80, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}><button class="small" disabled={!!upd?.busy} onclick={checkNow}>{upd?.busy ? t("Checking…") : t("Check for updates")}</button></span>
        {/if}
      </span>
    </div>
    {#if upd?.error}<p class="error" transition:fold>{upd.error}</p>{/if}
    {#if upd?.ready && upd.notes}<p class="note" transition:fold>{upd.notes}</p>{/if}
  </section>

  <section data-key="Help">
    <h3>{t("Help")}</h3>
    <div class="line">
      <span>{t("Quick tour")} <small>{t("Eight short steps, under a minute: dump, sort, give a time, pick a style.")}</small></span>
      <button class="small strong" onclick={askTour}>{t("Take the tour")}</button>
    </div>
    <h4>{t("Keys to know")}</h4>
    <div class="keymap">
      {#each [[store.shortcuts.toggle, t("show or hide {app}, from any app", { app: APP_NAME })], [store.shortcuts.dump, t("quick dump into the Inbox, from any app")], ["Enter · Shift+Enter", t("add a task · an idea")], ["!", t("at the end: a deadline that rings early")], ["Ctrl+Z · Ctrl+Y", t("undo · redo")], ["Ctrl+Shift+S · Ctrl+Shift+P", t("Style · Settings")], ["F1", t("help and the quick tour")]] as [k, what] (k)}
        <span class="keys">{k.split(" · ").map(keysText).join(" · ")}</span><span class="what">{what}</span>
      {/each}
      <h4 class="span">{t("Gestures")}</h4>
      {#each [["Click the Inbox strip", "opens or folds it; its pin keeps it open"], ["Drag a line out of the Inbox", "files it in the note you drop it on"], ["Drag a task onto the hours", "gives it a time"], ["Drag a line onto a task", "makes it one of its steps"], ["Drag a line onto a note", "moves it there"], ["Drag a note", "rearranges the board"], ["Drag a note’s right edge", "makes it wide"], ["Drop a picture or a file on a note", "keeps it there"], ["Right-click the board", "changes the background"], ["Ctrl + wheel", "note size"]] as const as [g, what] (g)}
        <span class="gesture">{t(g)}</span><span class="what">{t(what)}</span>
      {/each}
    </div>
    <p class="note">{t("Every key is under Keyboard.")} <button class="link" onclick={seeKeys}>{t("See them")}</button></p>
  </section>

  <section data-key="About">
    <h3>{t("About")}</h3>
    <div class="line">
      <span><span class="about-name"><Logo word={false} size={20} />{APP_NAME} {info?.version ?? ""}</span>{#if info}<small transition:fold={{ y: 0, duration: 240 }}>{kindNote(info)}</small>{/if}</span>
      {#if info}<span class="badge" transition:fadeQuick>{t(KIND_NAME[info.kind])}</span>{/if}
    </div>
    <div class="line">
      <span>{t("Data folder")} <small class="path" data-tip={info?.dir ?? ""}>{info?.dir ?? ""}</small></span>
      <button class="small" onclick={openData} disabled={!info}>{t("Open folder")}</button>
    </div>
    {#if openMsg}<p class="error" transition:fold>{words(openMsg)}</p>{/if}
    <p class="note">{t("{app} goes online only to fetch the title and picture of a web page you dump, and once a day to check for updates (unless you turn that off). Your notes stay on this computer.", { app: APP_NAME })}</p>
    <!-- The credit the licence's additional terms ask for (NOTICE.md, term 1). -->
    <p class="note credit">
      {t("{app} by Sookera. Free software under the GNU General Public License version 3, with the additional terms in its NOTICE.md file.", { app: APP_NAME })}
      <button class="link" onclick={() => void store.backend.openUrl("https://sookera.com")}>sookera.com</button>
    </p>
  </section>
</div>


<!-- A note whose words follow a choice: the old words fade out of the way, the new ones fade in,
     and the room eases to the new height. -->
{#snippet noteSwap(key: unknown, text: string)}
  <div class="note-room" use:easeHeight>
    <div class="note-stack">
      {#key key}
        <p class="note" in:arrive={{ delay: 90, duration: 220, y: 0 }} out:leave={{ duration: 110, y: 0 }}>{text}</p>
      {/key}
    </div>
  </div>
{/snippet}
<style>
  .sheet {
    position: absolute;
    inset: 0;
    z-index: 40;
    overflow-y: auto;
    padding: 0 var(--s6) var(--s6);
    background: var(--ground);
    /* What is scrolled to (a focused field) lands under the header, not behind it. */
    scroll-padding-top: 116px;
  }
  /* The header and the search stay in view while the sections scroll under them. */
  .top {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 calc(-1 * var(--s6));
    padding: var(--s3) var(--s6) var(--s3);
    background: var(--ground);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .find-field {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 38px;
    padding: 0 6px 0 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--card);
    color: var(--icon);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .find-field:focus-within {
    border-color: color-mix(in srgb, var(--ink) 35%, var(--line));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .find-field input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--t-base);
  }
  .find-field input::placeholder {
    color: var(--muted);
  }
  .clear {
    flex: none;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--icon);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .clear:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .nothing {
    margin: var(--s5) 0;
    font-family: var(--serif);
    font-style: italic;
    font-size: 16px;
    line-height: 1.45;
    color: var(--muted);
  }
  .sheet :global(.gone) {
    display: none !important;
  }
  section {
    padding: var(--s4) 0;
    border-top: 1px solid var(--line);
  }
  h3 {
    margin: 0 0 var(--s3);
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
  }
  h4 {
    margin: var(--s4) 0 var(--s1);
    font-size: var(--t-md);
    font-weight: 600;
    color: var(--soft);
  }
  .modes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s3);
    margin-bottom: var(--s2);
  }
  .mode {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--s3);
    text-align: left;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--card);
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .mode.on {
    border-color: var(--ink);
    box-shadow: 0 0 0 1px var(--ink);
  }
  .mode:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .pic {
    position: relative;
    height: 64px;
    border-radius: var(--r-md);
    background: linear-gradient(160deg, #6a7b86, #3f4c57);
    overflow: hidden;
    margin-bottom: 4px;
  }
  .pic span {
    position: absolute;
    border-radius: 5px;
    background: #f2f0eb;
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
  }
  .pic.docked span {
    right: 5px;
    top: 5px;
    bottom: 5px;
    width: 34%;
  }
  .pic.floating span {
    left: 38%;
    top: 14px;
    width: 40%;
    height: 38px;
  }
  .mode-name {
    font-family: var(--serif);
    font-size: 16px;
    font-weight: 600;
  }
  .mode-text {
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--soft);
  }
  .line {
    position: relative;
    min-height: 44px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    font-size: var(--t-base);
  }
  .line > span:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .about-name {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  /* A row with choices: when the words and the choices can't share the width (a narrow panel,
     longer French words), the choices go under the words instead of squeezing them. */
  .line:has(> .seg),
  .line:has(> .keyset),
  .line:has(> .aswap),
  .line:has(> .keys) {
    flex-wrap: wrap;
    row-gap: var(--s2);
    padding-block: 6px;
  }
  .line:has(> .seg) > span:first-child,
  .line:has(> .keyset) > span:first-child,
  .line:has(> .aswap) > span:first-child,
  .line:has(> .keys) > span:first-child {
    flex: 1 1 200px;
  }
  /* (Updates' Check for updates / Restart now: the button keeps its width, never over the words.) */
  .line > .aswap {
    flex-shrink: 0;
  }
  .line.lang {
    margin-top: var(--s3);
  }
  /* "Every day at [17:30]", "When [10] things wait": the field sits in the sentence. */
  .line > span:first-child:has(> .inline) {
    flex-direction: row;
    align-items: center;
    flex-wrap: wrap;
  }
  small {
    font-size: var(--t-sm);
    color: var(--muted);
  }
  .check {
    cursor: pointer;
  }
  .check input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .sw {
    width: 42px;
    height: 26px;
    flex-shrink: 0;
    border-radius: 13px;
    background: var(--edge);
    position: relative;
    transition: background-color 0.2s;
  }
  .sw::after {
    content: "";
    position: absolute;
    left: 3px;
    top: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
    transition: transform 0.25s var(--ease-out);
  }
  .check input:checked + .sw {
    background: var(--ink);
  }
  .check input:checked + .sw::after {
    transform: translateX(16px);
  }
  .check input:focus-visible + .sw {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .keys {
    font-size: var(--t-sm);
    font-weight: 500;
    color: var(--soft);
    padding: 3px 8px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    background: var(--card);
    white-space: nowrap;
  }
  .keys.rec {
    border-color: var(--ink);
    color: var(--ink);
    animation: rec 1.2s ease-in-out infinite;
  }
  @keyframes rec {
    50% {
      opacity: 0.55;
    }
  }
  .keyset {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
    flex-shrink: 0;
  }
  .seg button {
    height: 28px;
    padding: 0 var(--s3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-md);
    color: var(--soft);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  /* Each choice as wide as its words, the spare room shared out: "Follow Windows" stays on one
     line even in a narrow panel. */
  .tabs.appear {
    max-width: none;
    grid-template-columns: repeat(4, auto);
  }
  .tabs.appear button {
    padding: 0 var(--s2);
    white-space: nowrap;
  }
  .appear-hours {
    margin-top: var(--s3);
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(4, minmax(min-content, 1fr));
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--chip);
    max-width: 420px;
  }
  .tabs button {
    height: 30px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: var(--t-md);
    color: var(--soft);
    white-space: nowrap;
    cursor: pointer;
  }
  .tabs button.on {
    background: var(--card);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
  .screens {
    position: relative;
    width: 328px;
    margin: var(--s2) 0 var(--s1);
  }
  .screen {
    position: absolute;
    border: 1.5px solid var(--box);
    border-radius: 6px;
    background: var(--card);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    cursor: pointer;
    transition: border-color 0.15s, background-color 0.15s, opacity 0.15s;
  }
  .screen.dim {
    opacity: 0.7;
  }
  .screen.on {
    border: 2px solid var(--ink);
    background: var(--chip);
  }
  .screen .num {
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 600;
    line-height: 1;
  }
  .screen .res {
    font-size: 10px;
    color: var(--muted);
  }
  .days {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin: var(--s2) 0;
  }
  .days button {
    width: 46px;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    font-size: var(--t-md);
    color: var(--soft);
    cursor: pointer;
  }
  .days button.on {
    border-color: var(--ink);
    background: var(--ink);
    color: var(--card);
    font-weight: 500;
  }
  .hours {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3);
    font-size: var(--t-base);
  }
  .hours label {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  input[type="time"],
  .inline {
    height: 34px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    font: inherit;
    font-size: var(--t-base);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }
  .inline {
    height: 30px;
    margin: 0 4px;
  }
  .inline.num {
    width: 58px;
    text-align: center;
  }
  .soft {
    color: var(--muted);
  }
  /* The swapping note keeps the note's spacing on its box (so it still meets its neighbours'
     margins the same way). */
  .note-room {
    margin: var(--s2) 0 0;
  }
  .note-stack {
    position: relative;
    display: grid;
  }
  .note-stack > .note {
    grid-area: 1 / 1;
    margin: 0;
  }
  .small-swap,
  .tswap,
  .aswap {
    position: relative;
    display: grid;
  }
  .aswap {
    justify-items: end;
  }
  .aswap > * {
    justify-content: flex-end;
  }
  .small-swap > *,
  .tswap > *,
  .aswap > * {
    grid-area: 1 / 1;
  }
  .note {
    margin: var(--s2) 0 0;
    display: flex;
    gap: 6px;
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--muted);
  }
  /* The credit: one paragraph, its link in the flow of the words. */
  .note.credit {
    display: block;
  }
  .error {
    margin: 0 0 var(--s2);
    font-size: var(--t-sm);
    color: var(--late-ink);
  }
  .small {
    height: 30px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--card);
    font-size: var(--t-md);
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
  }
  .link {
    padding: 2px;
    border: 0;
    background: transparent;
    font-size: var(--t-sm);
    color: var(--soft);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .small.strong {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  /* Installed · Portable (Settings › About). */
  .badge {
    flex-shrink: 0;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 13px;
    background: var(--card);
    display: grid;
    place-items: center;
    font-size: var(--t-sm);
    font-weight: 600;
    color: var(--soft);
  }
  .line.check.sub {
    padding-left: var(--s4);
    border-left: 2px solid var(--line);
  }
  .path {
    display: block;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .keymap {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px 12px;
    align-items: center;
    margin-top: var(--s2);
  }
  .keymap .keys {
    justify-self: start;
    white-space: normal;
  }
  /* Narrow: each key above what it does (the long ones would squeeze the words to a column). */
  /* The smallest floating window: the four light / dark choices ("Selon l’heure"…) still fit. */
  @container shell (max-width: 420px) {
    .tabs.appear button {
      padding: 0 5px;
    }
  }
  @container shell (max-width: 520px) {
    .keymap {
      grid-template-columns: minmax(0, 1fr);
      gap: 4px;
    }
    .keymap .what {
      margin-bottom: 6px;
    }
  }
  .keymap .what {
    font-size: var(--t-md);
    color: var(--soft);
  }
  .keymap h4.span {
    grid-column: 1 / -1;
    margin: var(--s3) 0 0;
  }
  .gesture {
    font-size: var(--t-md);
    font-weight: 500;
    color: var(--ink);
  }
  .note .link {
    font-size: inherit;
  }
  .backups {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: var(--s1);
  }
  .backup {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-height: 40px;
    padding: 0 var(--s2);
    border-radius: var(--r-md);
  }
  .backup:hover {
    background: var(--hover);
  }
  .b-when {
    font-size: var(--t-base);
    font-variant-numeric: tabular-nums;
  }
  .b-why {
    flex: 1;
    font-size: var(--t-sm);
    color: var(--muted);
  }
</style>
