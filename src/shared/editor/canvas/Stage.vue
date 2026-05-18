<script setup lang="ts">
import Konva from "konva";
import { onMounted, onUnmounted, ref, watch } from "vue";
import { editorState, commitChange } from "../state/shapes";
import { renderShape, renderMosaic } from "./drawTools";
import { openTextEditor } from "./textTool";
import { MOSAIC_BLOCK, FONT_SIZE } from "../../colors";
import { nanoid } from "nanoid";
import type { Shape } from "../../types";

const props = defineProps<{
  bgUrl: string;
  selection: { x: number; y: number; w: number; h: number };
  overlaySize: { w: number; h: number };
}>();

const containerRef = ref<HTMLDivElement | null>(null);
let stage: Konva.Stage | null = null;
let bgLayer: Konva.Layer | null = null;
let annLayer: Konva.Layer | null = null;
let previewLayer: Konva.Layer | null = null;
let uiLayer: Konva.Layer | null = null;
let bgImage: HTMLImageElement | null = null;
let transformer: Konva.Transformer | null = null;

type Drafting = {
  startX: number;
  startY: number;
  node: Konva.Node | null;
};
let drafting: Drafting | null = null;

const DRAW_TOOLS = ["line", "rect", "arrow", "mosaic"] as const;
type DrawTool = (typeof DRAW_TOOLS)[number];

function isDrawTool(t: string): t is DrawTool {
  return (DRAW_TOOLS as readonly string[]).includes(t);
}

function buildDraftShape(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): Shape {
  const tool = editorState.tool;
  const base = {
    id: "draft",
    color: editorState.color,
    thickness: editorState.thickness,
    tool,
  };
  if (tool === "rect") {
    return {
      ...base,
      geometry: {
        kind: "rect",
        x: Math.min(x1, x2),
        y: Math.min(y1, y2),
        w: Math.abs(x1 - x2),
        h: Math.abs(y1 - y2),
      },
    } as Shape;
  }
  if (tool === "mosaic") {
    return {
      ...base,
      geometry: {
        kind: "mosaic",
        x: Math.min(x1, x2),
        y: Math.min(y1, y2),
        w: Math.abs(x1 - x2),
        h: Math.abs(y1 - y2),
        blockSize: MOSAIC_BLOCK[editorState.thickness],
      },
    } as Shape;
  }
  return {
    ...base,
    geometry: {
      kind: tool === "line" ? "line" : "arrow",
      x1,
      y1,
      x2,
      y2,
    },
  } as Shape;
}

function shapeIsTooSmall(s: Shape): boolean {
  const g = s.geometry;
  if (g.kind === "rect" || g.kind === "mosaic") return g.w < 3 || g.h < 3;
  if (g.kind === "line" || g.kind === "arrow")
    return Math.hypot(g.x2 - g.x1, g.y2 - g.y1) < 3;
  return false;
}

function renderPreview(draft: Shape): Konva.Node | null {
  if (draft.geometry.kind === "mosaic") {
    const g = draft.geometry;
    return new Konva.Rect({
      x: g.x,
      y: g.y,
      width: g.w,
      height: g.h,
      fill: "rgba(0,0,0,0.4)",
      stroke: "white",
      strokeWidth: 1,
      dash: [4, 4],
    });
  }
  try {
    return renderShape(draft);
  } catch {
    return null;
  }
}

onMounted(() => {
  if (!containerRef.value) return;
  stage = new Konva.Stage({
    container: containerRef.value,
    width: props.overlaySize.w || 800,
    height: props.overlaySize.h || 600,
  });
  (window as unknown as { __editorStage?: Konva.Stage }).__editorStage = stage;
  bgLayer = new Konva.Layer({ listening: false });
  annLayer = new Konva.Layer();
  previewLayer = new Konva.Layer({ listening: false });
  uiLayer = new Konva.Layer();
  stage.add(bgLayer);
  stage.add(annLayer);
  stage.add(previewLayer);
  stage.add(uiLayer);

  transformer = new Konva.Transformer({
    rotateEnabled: false,
    ignoreStroke: true,
  });
  uiLayer.add(transformer);

  loadBg();

  annLayer.on("click", (e) => {
    if (editorState.tool === "text" || editorState.tool === "mosaic") return;
    const target = e.target;
    if (target === stage || !target.id()) {
      transformer!.nodes([]);
      editorState.selectedId = null;
      return;
    }
    transformer!.nodes([target as Konva.Shape]);
    editorState.selectedId = target.id();
  });

  stage.on("click", (e) => {
    if (e.target === stage) {
      transformer!.nodes([]);
      editorState.selectedId = null;
    }
  });

  annLayer.on("dragend transformend", (e) => {
    const node = e.target;
    const id = node.id();
    const shape = editorState.shapes.find((s) => s.id === id);
    if (!shape) return;
    const g = shape.geometry;
    if (g.kind === "rect" || g.kind === "mosaic" || g.kind === "text") {
      shape.geometry = {
        ...g,
        x: node.x(),
        y: node.y(),
        w: Math.max(1, node.width() * node.scaleX()),
        h: Math.max(1, node.height() * node.scaleY()),
      };
      node.scale({ x: 1, y: 1 });
    } else if (g.kind === "line" || g.kind === "arrow") {
      const pts = (node as Konva.Line).points();
      const newPts = [
        pts[0] + node.x(),
        pts[1] + node.y(),
        pts[2] + node.x(),
        pts[3] + node.y(),
      ];
      shape.geometry = {
        ...g,
        x1: newPts[0],
        y1: newPts[1],
        x2: newPts[2],
        y2: newPts[3],
      };
      (node as Konva.Line).points(newPts);
      node.position({ x: 0, y: 0 });
    }
    commitChange();
  });

  stage.on("mousedown", (e) => {
    if (e.target !== stage && e.target.id()) {
      // A shape was hit. Stop the DOM event from bubbling up to App.vue,
      // otherwise App.vue would also start a "move" drag and the shape
      // and selection would both move.
      e.evt.stopPropagation();
      return;
    }
    if (e.target !== stage) return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
    if (editorState.tool === "text") {
      if (!containerRef.value) return;
      openTextEditor({
        containerEl: containerRef.value,
        stagePoint: pos,
        color: editorState.color,
        thickness: editorState.thickness,
        onCommit: (text, bounds) => {
          const shape: Shape = {
            id: nanoid(10),
            tool: "text",
            color: editorState.color,
            thickness: editorState.thickness,
            geometry: {
              kind: "text",
              x: pos.x,
              y: pos.y,
              w: bounds.w,
              h: bounds.h,
            },
            text: {
              content: text,
              fontSize: FONT_SIZE[editorState.thickness],
            },
          };
          editorState.shapes.push(shape);
          commitChange();
        },
        onCancel: () => {},
      });
      return;
    }
    if (!isDrawTool(editorState.tool)) return;
    drafting = { startX: pos.x, startY: pos.y, node: null };
  });

  stage.on("mousemove", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
    const draft = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (drafting.node) drafting.node.destroy();
    const node = renderPreview(draft);
    if (!node) return;
    drafting.node = node;
    previewLayer!.destroyChildren();
    previewLayer!.add(node as Konva.Shape);
    previewLayer!.batchDraw();
  });

  stage.on("mouseup", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
    previewLayer!.destroyChildren();
    previewLayer!.batchDraw();
    if (!pos) {
      drafting = null;
      return;
    }
    const final = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (shapeIsTooSmall(final)) {
      drafting = null;
      return;
    }
    final.id = nanoid(10);
    editorState.shapes.push(final);
    commitChange();
    drafting = null;
  });
});

function rerenderAnnotations() {
  if (!annLayer) return;
  annLayer.destroyChildren();
  editorState.shapes.forEach((s) => {
    if (s.geometry.kind === "mosaic" && bgImage) {
      annLayer!.add(
        renderMosaic(s as Shape & { geometry: typeof s.geometry }, bgImage),
      );
      return;
    }
    try {
      annLayer!.add(renderShape(s) as Konva.Shape);
    } catch {
      /* unsupported kinds skipped (text handled later) */
    }
  });
  annLayer.batchDraw();
}

async function loadBg() {
  const url = props.bgUrl;
  if (!url || !stage || !bgLayer) return;
  const img = new Image();
  img.src = url;
  try {
    await img.decode();
  } catch {
    return;
  }
  bgImage = img;
  const kImg = new Konva.Image({
    image: img,
    x: 0,
    y: 0,
    width: props.overlaySize.w,
    height: props.overlaySize.h,
  });
  bgLayer.destroyChildren();
  bgLayer.add(kImg);
  bgLayer.draw();
  stage.size({ width: props.overlaySize.w, height: props.overlaySize.h });
  rerenderAnnotations();
}

watch(() => props.bgUrl, loadBg);

watch(() => editorState.shapes.length, rerenderAnnotations);

onUnmounted(() => {
  delete (window as unknown as { __editorStage?: Konva.Stage }).__editorStage;
});

defineExpose({
  getStage: () => stage,
  getBgLayer: () => bgLayer,
  getAnnLayer: () => annLayer,
  getPreviewLayer: () => previewLayer,
  getUiLayer: () => uiLayer,
});
</script>

<template>
  <div ref="containerRef" class="stage-host"></div>
</template>
