import { describe, it, expect } from "vitest";
import { placeToolbar } from "../windows/overlay/toolbarPlacement";
import type { Rect } from "../shared/types";

const tbar = { w: 300, h: 36 };
const screen: Rect = { x: 0, y: 0, w: 1920, h: 1080 };

describe("placeToolbar", () => {
  it("places below when space allows", () => {
    const sel = { x: 200, y: 100, w: 400, h: 200 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.orientation).toBe("below");
    expect(p.y).toBe(sel.y + sel.h + 8);
  });

  it("places above when below is too tight", () => {
    const sel = { x: 200, y: 100, w: 400, h: 1000 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.orientation).toBe("above");
    expect(p.y).toBe(sel.y - 8 - tbar.h);
  });

  it("falls back to inside when neither fits", () => {
    const tinyScreen: Rect = { x: 0, y: 0, w: 1920, h: 50 };
    const sel = { x: 200, y: 0, w: 400, h: 50 };
    const p = placeToolbar(sel, tbar, tinyScreen);
    expect(p.orientation).toBe("inside");
  });

  it("clamps x to bounds at right edge", () => {
    const sel = { x: 1900, y: 100, w: 20, h: 50 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.x + tbar.w).toBeLessThanOrEqual(screen.x + screen.w);
  });

  it("clamps x to bounds at left edge", () => {
    const sel = { x: 0, y: 100, w: 50, h: 50 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.x).toBeGreaterThanOrEqual(screen.x);
  });

  describe("multi-screen bounds (bounds not at origin)", () => {
    // Simulates the active screen being at virtual position (1080, 434)
    // inside a larger overlay window covering 4080x1925.
    const primary: Rect = { x: 1080, y: 434, w: 1920, h: 1080 };

    it("clamps toolbar inside the active screen, not the larger overlay", () => {
      // Selection at the bottom of the primary screen.
      const sel = { x: 1500, y: 1300, w: 600, h: 200 };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.x).toBeGreaterThanOrEqual(primary.x);
      expect(p.x + tbar.w).toBeLessThanOrEqual(primary.x + primary.w);
      expect(p.y).toBeGreaterThanOrEqual(primary.y);
      expect(p.y + tbar.h).toBeLessThanOrEqual(primary.y + primary.h);
    });

    it("places below relative to the active screen's bottom edge", () => {
      // Below fits within the primary screen.
      const sel = { x: 1500, y: 600, w: 200, h: 200 };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.orientation).toBe("below");
      expect(p.y).toBe(sel.y + sel.h + 8);
    });

    it("falls back to inside when selection covers the whole active screen", () => {
      const sel = { ...primary };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.orientation).toBe("inside");
      expect(p.y).toBeGreaterThanOrEqual(primary.y);
      expect(p.y + tbar.h).toBeLessThanOrEqual(primary.y + primary.h);
    });
  });
});
