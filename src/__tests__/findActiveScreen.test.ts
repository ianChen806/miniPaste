import { describe, it, expect } from "vitest";
import { findActiveScreen } from "../windows/overlay/findActiveScreen";
import type { Rect } from "../shared/types";

const fallback: Rect = { x: 0, y: 0, w: 4080, h: 1925 };

describe("findActiveScreen", () => {
  const screens: Rect[] = [
    { x: 0, y: 0, w: 1080, h: 1920 },        // left portrait
    { x: 1080, y: 434, w: 1920, h: 1080 },   // primary
    { x: 3000, y: 434, w: 1080, h: 1920 },   // right portrait
  ];

  it("returns the screen containing the selection center", () => {
    const sel = { x: 1500, y: 700, w: 200, h: 200 }; // center (1600, 800) → primary
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[1]);
  });

  it("returns the left screen when center is on the left", () => {
    const sel = { x: 100, y: 100, w: 200, h: 200 }; // center (200, 200) → left
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[0]);
  });

  it("returns fallback when center is in a dead zone", () => {
    const sel = { x: 1080, y: 0, w: 200, h: 200 }; // center (1180, 100) → above primary, dead zone
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(fallback);
  });

  it("returns fallback when no screens are provided", () => {
    const sel = { x: 100, y: 100, w: 50, h: 50 };
    const r = findActiveScreen(sel, [], fallback);
    expect(r).toEqual(fallback);
  });

  it("treats screen rect as half-open: top/left inclusive, bottom/right exclusive", () => {
    const sel = { x: 1080, y: 434, w: 0, h: 0 }; // center exactly (1080, 434) → primary (inclusive)
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[1]);
  });
});
