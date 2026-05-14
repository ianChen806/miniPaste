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
  const ce = document.createElement("div");
  ce.className = "konva-text-editor";
  ce.contentEditable = "true";
  ce.spellcheck = false;
  Object.assign(ce.style, {
    position: "absolute",
    left: `${opts.stagePoint.x}px`,
    top: `${opts.stagePoint.y}px`,
    color: COLOR_HEX[opts.color],
    fontSize: `${FONT_SIZE[opts.thickness]}px`,
    fontFamily: "system-ui, sans-serif",
    background: "transparent",
    border: "1px dashed #9ca3af",
    caretColor: COLOR_HEX[opts.color],
    padding: "4px 6px",
    minWidth: "120px",
    minHeight: `${FONT_SIZE[opts.thickness] + 12}px`,
    width: "200px",
    whiteSpace: "pre-wrap",
    overflow: "auto",
    resize: "both",
    outline: "none",
    zIndex: "1000",
  });
  ce.innerText = opts.initial ?? "";
  opts.containerEl.appendChild(ce);
  ce.focus();

  let done = false;
  function commit() {
    if (done) return;
    done = true;
    const text = ce.innerText;
    const w = ce.offsetWidth;
    const h = ce.offsetHeight;
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
    ce.removeEventListener("keydown", onKey);
    ce.removeEventListener("mousedown", stopProp);
    document.removeEventListener("pointerdown", onOutsidePointer, true);
    ce.remove();
  }
  function onKey(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    } else if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      commit();
    }
  }
  function stopProp(e: Event) {
    e.stopPropagation();
  }
  function onOutsidePointer(e: PointerEvent) {
    if (e.target instanceof Node && ce.contains(e.target)) return;
    commit();
  }
  ce.addEventListener("keydown", onKey);
  ce.addEventListener("mousedown", stopProp);
  setTimeout(() => {
    if (!done) document.addEventListener("pointerdown", onOutsidePointer, true);
  }, 0);
}
