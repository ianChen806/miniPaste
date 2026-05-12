import Konva from "konva";
import { COLOR_HEX, STROKE_WIDTH } from "../../../shared/colors";
import type { Shape } from "../../../shared/types";

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
    case "text":
      throw new Error("text must use renderText");
    default:
      throw new Error(
        `renderShape: ${(shape.geometry as { kind: string }).kind} not yet supported`,
      );
  }
}
