export type ImageFormat = "png" | "jpeg";

export interface Config {
  schema_version: number;
  hotkey: string;
  default_save_path: string;
  image_format: ImageFormat;
  jpeg_quality: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ScreenInfo {
  x: number;
  y: number;
  w: number;
  h: number;
  scale: number;
}

export type ToolType = "line" | "rect" | "arrow" | "mosaic" | "text";
export type ColorKey = "red" | "orange" | "yellow" | "green" | "blue";
export type Thickness = "thin" | "medium" | "thick";

export interface Shape {
  id: string;
  tool: ToolType;
  color: ColorKey;
  thickness: Thickness;
  geometry: ShapeGeometry;
  text?: { content: string; fontSize: number };
}

export type ShapeGeometry =
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "rect"; x: number; y: number; w: number; h: number }
  | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "mosaic"; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: "text"; x: number; y: number; w: number; h: number };

export type FinishAction =
  | { kind: "CopyImage" }
  | { kind: "Save"; path: string }
  | { kind: "SaveAndCopyPath" };

export interface FinishOutcome {
  saved_path: string | null;
}

export interface AppError {
  code: string;
  message: string;
}
