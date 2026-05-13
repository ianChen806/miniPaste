import type { Rect } from "../../shared/types";

export interface ToolbarSize { w: number; h: number }
export interface OverlaySize { w: number; h: number }

export interface ToolbarPlacement {
  x: number;
  y: number;
  orientation: "below" | "above" | "inside";
}

export function placeToolbar(
  selection: Rect,
  toolbar: ToolbarSize,
  overlay: OverlaySize,
  gap = 8,
): ToolbarPlacement {
  const belowY = selection.y + selection.h + gap;
  const aboveY = selection.y - gap - toolbar.h;

  let orientation: ToolbarPlacement["orientation"];
  let y: number;
  if (belowY + toolbar.h <= overlay.h) {
    orientation = "below";
    y = belowY;
  } else if (aboveY >= 0) {
    orientation = "above";
    y = aboveY;
  } else {
    orientation = "inside";
    y = selection.y + selection.h - toolbar.h - gap;
  }

  const desiredX = selection.x + (selection.w - toolbar.w) / 2;
  const x = Math.max(0, Math.min(desiredX, overlay.w - toolbar.w));

  return { x, y, orientation };
}
