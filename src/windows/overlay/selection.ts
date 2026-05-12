export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function rectFromDrag(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const w = Math.abs(a.x - b.x);
  const h = Math.abs(a.y - b.y);
  return { x, y, w, h };
}

export function clampToBounds(r: Rect, maxW: number, maxH: number): Rect {
  const x = Math.max(0, r.x);
  const y = Math.max(0, r.y);
  const w = Math.min(maxW - x, r.w - (x - r.x));
  const h = Math.min(maxH - y, r.h - (y - r.y));
  return { x, y, w: Math.max(0, w), h: Math.max(0, h) };
}
