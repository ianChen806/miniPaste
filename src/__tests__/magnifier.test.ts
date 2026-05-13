import { describe, it, expect, vi } from "vitest";
import { renderMagnifier } from "../windows/overlay/magnifier";

function makeCtx(): CanvasRenderingContext2D {
  return {
    clearRect: vi.fn(),
    drawImage: vi.fn(),
    fillRect: vi.fn(),
    fillText: vi.fn(),
    strokeRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    imageSmoothingEnabled: true,
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 1,
    font: "",
    textAlign: "left",
    textBaseline: "alphabetic",
    canvas: { width: 120, height: 120 } as HTMLCanvasElement,
  } as unknown as CanvasRenderingContext2D;
}

describe("renderMagnifier", () => {
  it("does not throw with a valid source image and cursor", () => {
    const ctx = makeCtx();
    const img = new Image() as HTMLImageElement;
    Object.defineProperty(img, "width", { value: 1920 });
    Object.defineProperty(img, "height", { value: 1080 });
    expect(() =>
      renderMagnifier(ctx, img, { x: 806, y: 506 }, 5),
    ).not.toThrow();
  });

  it("invokes drawImage exactly once", () => {
    const ctx = makeCtx();
    const img = new Image() as HTMLImageElement;
    Object.defineProperty(img, "width", { value: 1920 });
    Object.defineProperty(img, "height", { value: 1080 });
    renderMagnifier(ctx, img, { x: 100, y: 100 }, 5);
    expect(ctx.drawImage).toHaveBeenCalledTimes(1);
  });
});
