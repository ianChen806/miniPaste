import type { Point } from "./handles";

const SIZE = 120;
const DEFAULT_ZOOM = 5;

export function renderMagnifier(
  ctx: CanvasRenderingContext2D,
  source: HTMLImageElement,
  cursor: Point,
  zoom: number = DEFAULT_ZOOM,
): void {
  const w = ctx.canvas.width;
  const h = ctx.canvas.height;
  const srcSpan = w / zoom;
  const sx = Math.max(0, Math.min(source.width - srcSpan, cursor.x - srcSpan / 2));
  const sy = Math.max(0, Math.min(source.height - srcSpan, cursor.y - srcSpan / 2));

  ctx.clearRect(0, 0, w, h);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(source, sx, sy, srcSpan, srcSpan, 0, 0, w, h);

  ctx.strokeStyle = "rgba(0, 128, 255, 0.9)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, h / 2);
  ctx.lineTo(w, h / 2);
  ctx.moveTo(w / 2, 0);
  ctx.lineTo(w / 2, h);
  ctx.stroke();

  ctx.strokeStyle = "rgba(255, 255, 255, 0.8)";
  ctx.strokeRect(w / 2 - 3, h / 2 - 3, 6, 6);

  ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
  ctx.fillRect(0, h - 18, w, 18);
  ctx.fillStyle = "#fff";
  ctx.font = "12px monospace";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(`(${Math.round(cursor.x)}, ${Math.round(cursor.y)})`, w / 2, h - 9);
}

export const MAGNIFIER_SIZE = SIZE;
