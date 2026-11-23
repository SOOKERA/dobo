// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The first open of a big view (Settings, the Style panel, Tidy…) must be as smooth as the next
// ones. What makes it slower the first time is that nothing of it was ever built: its HTML is
// read for the first time, its code runs for the first time. So in calm moments after launch
// (calm.ts), each one is built once, off the page, and dropped at once, in the same moment.
//
// Only built: in a detached element (never laid out, painted, focused or heard), and dropped
// before anything it would do once shown can run (its effects, actions, transitions, the keyboard
// it takes, the Esc it listens for). The first real open then finds it all ready.
import { mount, unmount, type Component } from "svelte";
import { calmMoment } from "./calm";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Rehearsal = [Component<any>, Record<string, unknown>];

let done = false;

/** Builds each view once, one per calm moment (in order: the most used first). Once per launch. */
export async function rehearse(views: Rehearsal[], stop?: () => boolean) {
  if (done) return;
  done = true;
  for (const [view, props] of views) {
    // (A moment with room to spare: building a big view takes a few tens of ms.)
    await calmMoment(stop, 20);
    if (stop?.()) return;
    const host = document.createElement("div");
    try {
      // (Mounted and unmounted in the same task: nothing scheduled by it ever runs.)
      unmount(mount(view, { target: host, props, intro: false }));
    } catch {
      // A view that can't be built off the page is just built on its first open, as before.
    }
  }
}
