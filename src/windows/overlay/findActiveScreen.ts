import type { Rect } from "../../shared/types";

export function findActiveScreen(
  selection: Rect,
  screens: Rect[],
  fallback: Rect,
): Rect {
  const cx = selection.x + selection.w / 2;
  const cy = selection.y + selection.h / 2;
  for (const s of screens) {
    if (cx >= s.x && cx < s.x + s.w && cy >= s.y && cy < s.y + s.h) {
      return s;
    }
  }
  return fallback;
}
