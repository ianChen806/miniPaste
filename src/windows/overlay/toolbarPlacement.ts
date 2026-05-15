import type { Rect } from "../../shared/types";

export interface ToolbarSize { w: number; h: number }

export interface ToolbarPlacement {
  x: number;
  y: number;
  orientation: "below" | "above" | "inside";
}

export function placeToolbar(
  selection: Rect,
  toolbar: ToolbarSize,
  bounds: Rect,
  gap = 8,
): ToolbarPlacement {
  const belowY = selection.y + selection.h + gap;
  const aboveY = selection.y - gap - toolbar.h;
  const boundsBottom = bounds.y + bounds.h;
  const boundsRight = bounds.x + bounds.w;

  let orientation: ToolbarPlacement["orientation"];
  let y: number;
  if (belowY + toolbar.h <= boundsBottom) {
    orientation = "below";
    y = belowY;
  } else if (aboveY >= bounds.y) {
    orientation = "above";
    y = aboveY;
  } else {
    orientation = "inside";
    y = selection.y + selection.h - toolbar.h - gap;
  }

  const desiredX = selection.x + (selection.w - toolbar.w) / 2;
  const x = Math.max(bounds.x, Math.min(desiredX, boundsRight - toolbar.w));
  const clampedY = Math.max(bounds.y, Math.min(y, boundsBottom - toolbar.h));

  return { x, y: clampedY, orientation };
}
