// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
// The picture viewer (double-click a picture): which pictures, which one is showing.
import type { Picture } from "./types";

class Viewer {
  pictures = $state<Picture[]>([]);
  index = $state(0);
  open = $state(false);
}

export const viewer = new Viewer();

export function openViewer(pictures: Picture[], index: number) {
  viewer.pictures = pictures;
  viewer.index = index;
  viewer.open = true;
}
