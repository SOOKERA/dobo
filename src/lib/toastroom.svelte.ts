// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// A view with buttons along its bottom (Tidy's Delete · Keep for later · Put it in…, New note's
// Create note) keeps them clear of the toast: the bar says how much room it takes from the
// panel's bottom, and the toast rises above it while the view is open.

const bars = new Map<HTMLElement, number>();

/** The top of the toast in its usual place, from the panel's bottom (Toast.svelte: 18px up and
 * one line, 40px; a narrow panel's 62px up, above the note sizes, and up to two lines, 56px), with
 * a little to spare: a bar ending higher up isn't in its way. */
const reach = (shell: HTMLElement) => (shell.clientWidth <= 620 ? 62 + 56 : 18 + 40) + 6;

/** How far above the panel's bottom the toast must sit (0: its usual place). */
export const toastRoom = $state({ lift: 0 });

function update() {
  let lift = 0;
  for (const v of bars.values()) lift = Math.max(lift, v);
  if (lift !== toastRoom.lift) toastRoom.lift = lift;
}

/** The room the bar takes from the panel's bottom (up to its top), in the laid-out page: a view
 * arriving (its slide or scale) doesn't count, so the toast goes straight to where the bar will
 * be. 0 when the bar ends higher up than the toast reaches (New note's, in a short dialog). */
function roomBelow(node: HTMLElement): number {
  const shell = node.closest<HTMLElement>(".shell");
  if (!shell) return 0;
  let y = 0;
  let e: HTMLElement | null = node;
  while (e && e !== shell) {
    const p = e.offsetParent as HTMLElement | null;
    y += e.offsetTop;
    // (Less what the boxes in between have scrolled.)
    for (let a: HTMLElement | null = e.parentElement; a && a !== p && a !== shell; a = a.parentElement) y -= a.scrollTop;
    if (p && p !== shell) y -= p.scrollTop;
    e = p;
  }
  const room = shell.clientHeight - y;
  if (room - node.offsetHeight > reach(shell)) return 0;
  return Math.max(0, Math.min(shell.clientHeight, room));
}

/** On a view's bottom bar: the toast stays above it while the bar is on the page. (Measured when
 * the bar or its view changes size, once laid out: nothing is read while it is being built.) */
export function toastAbove(node: HTMLElement) {
  const ro = new ResizeObserver(() => {
    if (!node.isConnected) return;
    bars.set(node, roomBelow(node));
    update();
  });
  ro.observe(node);
  // (And its view, the box it sits at the bottom of: a view growing moves the bar.)
  if (node.parentElement) ro.observe(node.parentElement);
  return {
    destroy() {
      ro.disconnect();
      bars.delete(node);
      update();
    },
  };
}
