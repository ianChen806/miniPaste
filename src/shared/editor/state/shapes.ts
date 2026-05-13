import { reactive } from "vue";
import { customAlphabet } from "nanoid";
import type {
  Shape,
  ToolType,
  ColorKey,
  Thickness,
} from "../../types";
import { createHistory } from "./history";

const nid = customAlphabet("abcdefghijklmnopqrstuvwxyz0123456789", 10);

export const editorState = reactive({
  tool: "rect" as ToolType,
  color: "red" as ColorKey,
  thickness: "medium" as Thickness,
  shapes: [] as Shape[],
  selectedId: null as string | null,
});

let history = createHistory();
history.push([]);

export function commitChange() {
  history.push(editorState.shapes);
}

export function resetEditor() {
  editorState.shapes = [];
  editorState.selectedId = null;
  history = createHistory();
  history.push([]);
}

export function undo() {
  history.undo();
  editorState.shapes = history.current().map((s) => ({ ...s }));
}

export function redo() {
  history.redo();
  editorState.shapes = history.current().map((s) => ({ ...s }));
}

export function newShape(partial: Omit<Shape, "id">): Shape {
  return { id: nid(), ...partial };
}
