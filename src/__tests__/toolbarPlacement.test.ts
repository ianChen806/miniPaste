import { describe, it, expect } from "vitest";
import { placeToolbar } from "../windows/overlay/toolbarPlacement";

const tbar = { w: 300, h: 36 };
const overlay = { w: 1920, h: 1080 };

describe("placeToolbar", () => {
  it("places below when space allows", () => {
    const sel = { x: 200, y: 100, w: 400, h: 200 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.orientation).toBe("below");
    expect(p.y).toBe(sel.y + sel.h + 8);
  });

  it("places above when below is too tight", () => {
    const sel = { x: 200, y: 100, w: 400, h: 1000 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.orientation).toBe("above");
    expect(p.y).toBe(sel.y - 8 - tbar.h);
  });

  it("falls back to inside when neither fits", () => {
    const tinyOverlay = { w: 1920, h: 50 };
    const sel = { x: 200, y: 0, w: 400, h: 50 };
    const p = placeToolbar(sel, tbar, tinyOverlay);
    expect(p.orientation).toBe("inside");
  });

  it("clamps x to overlay bounds at right edge", () => {
    const sel = { x: 1900, y: 100, w: 20, h: 50 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.x + tbar.w).toBeLessThanOrEqual(overlay.w);
  });

  it("clamps x to 0 at left edge", () => {
    const sel = { x: 0, y: 100, w: 50, h: 50 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.x).toBeGreaterThanOrEqual(0);
  });
});
