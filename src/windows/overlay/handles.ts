import type { Rect } from "../../shared/types";

export type HandleId =
  | "nw" | "n" | "ne"
  | "e"
  | "se" | "s" | "sw"
  | "w"
  | "move";

export interface Point { x: number; y: number }

const HIT_RADIUS = 6;

function near(a: number, b: number, r = HIT_RADIUS): boolean {
  return Math.abs(a - b) <= r;
}

export function hitTestHandle(rect: Rect, pt: Point): HandleId | null {
  const { x, y, w, h } = rect;
  const cx = x + w / 2;
  const cy = y + h / 2;
  const r = x + w;
  const b = y + h;

  if (near(pt.x, x) && near(pt.y, y)) return "nw";
  if (near(pt.x, r) && near(pt.y, y)) return "ne";
  if (near(pt.x, r) && near(pt.y, b)) return "se";
  if (near(pt.x, x) && near(pt.y, b)) return "sw";
  if (near(pt.x, cx) && near(pt.y, y)) return "n";
  if (near(pt.x, r) && near(pt.y, cy)) return "e";
  if (near(pt.x, cx) && near(pt.y, b)) return "s";
  if (near(pt.x, x) && near(pt.y, cy)) return "w";

  if (pt.x >= x && pt.x <= r && pt.y >= y && pt.y <= b) return "move";
  return null;
}

export function cursorForHandle(h: HandleId): string {
  switch (h) {
    case "nw":
    case "se":
      return "nwse-resize";
    case "ne":
    case "sw":
      return "nesw-resize";
    case "n":
    case "s":
      return "ns-resize";
    case "e":
    case "w":
      return "ew-resize";
    case "move":
      return "move";
  }
}

export function resizeRect(
  rect: Rect,
  handle: HandleId,
  delta: Point,
  minSize = 10,
): Rect {
  let { x, y, w, h } = rect;

  if (handle === "move") {
    return { x: x + delta.x, y: y + delta.y, w, h };
  }

  if (handle.includes("w")) {
    const dx = Math.min(delta.x, w - minSize);
    x += dx;
    w -= dx;
  }
  if (handle.includes("e")) {
    w = Math.max(minSize, w + delta.x);
  }
  if (handle.includes("n")) {
    const dy = Math.min(delta.y, h - minSize);
    y += dy;
    h -= dy;
  }
  if (handle.includes("s")) {
    h = Math.max(minSize, h + delta.y);
  }
  return { x, y, w, h };
}
