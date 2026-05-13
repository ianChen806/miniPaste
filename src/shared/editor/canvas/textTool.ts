import { COLOR_HEX, FONT_SIZE } from "../../colors";
import type { ColorKey, Thickness } from "../../types";

export interface TextEditOptions {
  containerEl: HTMLElement;
  stagePoint: { x: number; y: number };
  color: ColorKey;
  thickness: Thickness;
  initial?: string;
  onCommit: (text: string, bounds: { w: number; h: number }) => void;
  onCancel: () => void;
}

export function openTextEditor(opts: TextEditOptions) {
  const ta = document.createElement("textarea");
  ta.className = "konva-text-editor";
  Object.assign(ta.style, {
    position: "absolute",
    left: `${opts.stagePoint.x}px`,
    top: `${opts.stagePoint.y}px`,
    color: COLOR_HEX[opts.color],
    fontSize: `${FONT_SIZE[opts.thickness]}px`,
    fontFamily: "system-ui, sans-serif",
    background: "rgba(255,255,255,0.85)",
    border: "1px dashed #1f2937",
    padding: "2px 4px",
    minWidth: "60px",
    minHeight: `${FONT_SIZE[opts.thickness] + 8}px`,
    resize: "both",
    zIndex: "1000",
  });
  ta.value = opts.initial ?? "";
  opts.containerEl.appendChild(ta);
  ta.focus();

  let done = false;
  function commit() {
    if (done) return;
    done = true;
    const text = ta.value;
    const w = ta.offsetWidth;
    const h = ta.offsetHeight;
    cleanup();
    if (text.trim()) opts.onCommit(text, { w, h });
    else opts.onCancel();
  }
  function cancel() {
    if (done) return;
    done = true;
    cleanup();
    opts.onCancel();
  }
  function cleanup() {
    ta.removeEventListener("keydown", onKey);
    ta.removeEventListener("blur", commit);
    ta.remove();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    } else if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      commit();
    }
  }
  ta.addEventListener("keydown", onKey);
  // Defer blur listener so the initial mousedown's focus reshuffle does not
  // immediately trigger commit (which would remove the textarea before the
  // user can type anything).
  setTimeout(() => {
    if (!done) ta.addEventListener("blur", commit);
  }, 0);
}
