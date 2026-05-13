import { describe, it, expect } from "vitest";
import {
  hitTestHandle,
  cursorForHandle,
  resizeRect,
  type HandleId,
} from "../windows/overlay/handles";

const r = { x: 100, y: 100, w: 200, h: 100 };

describe("hitTestHandle", () => {
  it("returns null for points outside", () => {
    expect(hitTestHandle(r, { x: 50, y: 50 })).toBeNull();
    expect(hitTestHandle(r, { x: 400, y: 200 })).toBeNull();
  });

  it("returns the matching corner id within 6px", () => {
    expect(hitTestHandle(r, { x: 100, y: 100 })).toBe("nw");
    expect(hitTestHandle(r, { x: 300, y: 100 })).toBe("ne");
    expect(hitTestHandle(r, { x: 300, y: 200 })).toBe("se");
    expect(hitTestHandle(r, { x: 100, y: 200 })).toBe("sw");
  });

  it("returns the matching midpoint id", () => {
    expect(hitTestHandle(r, { x: 200, y: 100 })).toBe("n");
    expect(hitTestHandle(r, { x: 300, y: 150 })).toBe("e");
    expect(hitTestHandle(r, { x: 200, y: 200 })).toBe("s");
    expect(hitTestHandle(r, { x: 100, y: 150 })).toBe("w");
  });

  it("returns 'move' for points inside but not on a handle", () => {
    expect(hitTestHandle(r, { x: 200, y: 150 })).toBe("move");
  });
});

describe("cursorForHandle", () => {
  it.each([
    ["nw", "nwse-resize"],
    ["se", "nwse-resize"],
    ["ne", "nesw-resize"],
    ["sw", "nesw-resize"],
    ["n", "ns-resize"],
    ["s", "ns-resize"],
    ["e", "ew-resize"],
    ["w", "ew-resize"],
    ["move", "move"],
  ] as const)("%s -> %s", (h, cur) => {
    expect(cursorForHandle(h as HandleId)).toBe(cur);
  });
});

describe("resizeRect", () => {
  it("se grows the rect by delta", () => {
    const out = resizeRect(r, "se", { x: 50, y: 25 });
    expect(out).toEqual({ x: 100, y: 100, w: 250, h: 125 });
  });

  it("nw shrinks the rect by moving origin and reducing size", () => {
    const out = resizeRect(r, "nw", { x: 20, y: 10 });
    expect(out).toEqual({ x: 120, y: 110, w: 180, h: 90 });
  });

  it("move shifts both origin and size unchanged", () => {
    const out = resizeRect(r, "move", { x: 30, y: -10 });
    expect(out).toEqual({ x: 130, y: 90, w: 200, h: 100 });
  });

  it("clamps to minSize when shrinking past it", () => {
    const out = resizeRect(r, "se", { x: -500, y: -500 }, 10);
    expect(out).toEqual({ x: 100, y: 100, w: 10, h: 10 });
  });
});
