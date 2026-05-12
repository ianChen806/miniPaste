import type { ColorKey, Thickness } from "./types";

export const COLOR_HEX: Record<ColorKey, string> = {
  red: "#ef4444",
  orange: "#f97316",
  yellow: "#eab308",
  green: "#22c55e",
  blue: "#3b82f6",
};

export const COLOR_ORDER: ColorKey[] = ["red", "orange", "yellow", "green", "blue"];

export const STROKE_WIDTH: Record<Thickness, number> = {
  thin: 2,
  medium: 4,
  thick: 8,
};

export const MOSAIC_BLOCK: Record<Thickness, number> = {
  thin: 8,
  medium: 16,
  thick: 24,
};

export const FONT_SIZE: Record<Thickness, number> = {
  thin: 16,
  medium: 24,
  thick: 36,
};
