import Konva from "konva";
import { COLOR_HEX, STROKE_WIDTH, FONT_SIZE } from "../../../shared/colors";
import type { Shape, ShapeGeometry } from "../../../shared/types";

type MosaicGeometry = Extract<ShapeGeometry, { kind: "mosaic" }>;
type MosaicShape = Shape & { geometry: MosaicGeometry };

export function renderMosaic(
  shape: MosaicShape,
  bgImage: HTMLImageElement,
): Konva.Image {
  const { x, y, w, h, blockSize } = shape.geometry;
  const off = document.createElement("canvas");
  off.width = w;
  off.height = h;
  const ctx = off.getContext("2d")!;
  ctx.drawImage(bgImage, x, y, w, h, 0, 0, w, h);
  const small = document.createElement("canvas");
  small.width = Math.max(1, Math.floor(w / blockSize));
  small.height = Math.max(1, Math.floor(h / blockSize));
  const sctx = small.getContext("2d")!;
  sctx.imageSmoothingEnabled = false;
  sctx.drawImage(off, 0, 0, small.width, small.height);
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, w, h);
  ctx.drawImage(small, 0, 0, small.width, small.height, 0, 0, w, h);
  return new Konva.Image({
    x,
    y,
    image: off,
    width: w,
    height: h,
    id: shape.id,
  });
}

export function renderShape(shape: Shape): Konva.Node {
  const stroke = COLOR_HEX[shape.color];
  const width = STROKE_WIDTH[shape.thickness];
  switch (shape.geometry.kind) {
    case "line": {
      const g = shape.geometry;
      return new Konva.Line({
        points: [g.x1, g.y1, g.x2, g.y2],
        stroke,
        strokeWidth: width,
        lineCap: "round",
        id: shape.id,
      });
    }
    case "rect": {
      const g = shape.geometry;
      return new Konva.Rect({
        x: g.x,
        y: g.y,
        width: g.w,
        height: g.h,
        stroke,
        strokeWidth: width,
        id: shape.id,
      });
    }
    case "arrow": {
      const g = shape.geometry;
      return new Konva.Arrow({
        points: [g.x1, g.y1, g.x2, g.y2],
        stroke,
        fill: stroke,
        strokeWidth: width,
        pointerLength: width * 3,
        pointerWidth: width * 3,
        id: shape.id,
      });
    }
    case "mosaic":
      throw new Error("mosaic must use renderMosaic with bg image");
    case "text": {
      const g = shape.geometry;
      return new Konva.Text({
        x: g.x,
        y: g.y,
        width: g.w,
        height: g.h,
        text: shape.text?.content ?? "",
        fill: stroke,
        fontSize: shape.text?.fontSize ?? FONT_SIZE[shape.thickness],
        fontFamily: "system-ui, sans-serif",
        id: shape.id,
      });
    }
    default:
      throw new Error(
        `renderShape: ${(shape.geometry as { kind: string }).kind} not yet supported`,
      );
  }
}
