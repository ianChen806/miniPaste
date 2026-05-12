import { describe, expect, it } from "vitest";
import { rectFromDrag, clampToBounds } from "../windows/overlay/selection";

describe("rectFromDrag", () => {
  it("returns positive w/h regardless of drag direction", () => {
    expect(rectFromDrag({ x: 100, y: 100 }, { x: 50, y: 30 })).toEqual({
      x: 50,
      y: 30,
      w: 50,
      h: 70,
    });
  });
});

describe("clampToBounds", () => {
  it("clamps a rect to [0..w]x[0..h]", () => {
    expect(clampToBounds({ x: -10, y: 5, w: 100, h: 100 }, 80, 80)).toEqual({
      x: 0,
      y: 5,
      w: 80,
      h: 75,
    });
  });
});
