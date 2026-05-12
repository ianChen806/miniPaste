<script setup lang="ts">
import Konva from "konva";
import { onMounted, ref, watch } from "vue";
import { editorState, commitChange } from "../state/shapes";
import { renderShape } from "./drawTools";
import { nanoid } from "nanoid";
import type { Shape } from "../../../shared/types";

const props = defineProps<{
  imageUrl: string;
  width: number;
  height: number;
}>();

const containerRef = ref<HTMLDivElement | null>(null);
let stage: Konva.Stage | null = null;
let bgLayer: Konva.Layer | null = null;
let annLayer: Konva.Layer | null = null;
let previewLayer: Konva.Layer | null = null;
let uiLayer: Konva.Layer | null = null;

type Drafting = {
  startX: number;
  startY: number;
  node: Konva.Node | null;
};
let drafting: Drafting | null = null;

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
  if (g.kind === "rect") return g.w < 3 || g.h < 3;
  if (g.kind === "line" || g.kind === "arrow")
    return Math.hypot(g.x2 - g.x1, g.y2 - g.y1) < 3;
  return false;
}

onMounted(() => {
  if (!containerRef.value) return;
  stage = new Konva.Stage({
    container: containerRef.value,
    width: props.width || 800,
    height: props.height || 600,
  });
  bgLayer = new Konva.Layer({ listening: false });
  annLayer = new Konva.Layer();
  previewLayer = new Konva.Layer({ listening: false });
  uiLayer = new Konva.Layer();
  stage.add(bgLayer);
  stage.add(annLayer);
  stage.add(previewLayer);
  stage.add(uiLayer);

  stage.on("mousedown", () => {
    const tool = editorState.tool;
    if (tool !== "line" && tool !== "rect" && tool !== "arrow") return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
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
    let node: Konva.Node;
    try {
      node = renderShape(draft);
    } catch {
      return;
    }
    drafting.node = node;
    previewLayer!.destroyChildren();
    previewLayer!.add(node as Konva.Shape);
    previewLayer!.batchDraw();
  });

  stage.on("mouseup", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
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
    previewLayer!.destroyChildren();
    previewLayer!.batchDraw();
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

watch(
  () => props.imageUrl,
  (url) => {
    if (!url || !stage || !bgLayer) return;
    Konva.Image.fromURL(url, (img) => {
      img.x(0);
      img.y(0);
      img.width(props.width);
      img.height(props.height);
      bgLayer!.destroyChildren();
      bgLayer!.add(img);
      bgLayer!.draw();
      stage!.size({ width: props.width, height: props.height });
    });
  },
);

watch(
  () => editorState.shapes.length,
  () => {
    if (!annLayer) return;
    annLayer.destroyChildren();
    editorState.shapes.forEach((s) => {
      try {
        annLayer!.add(renderShape(s) as Konva.Shape);
      } catch {
        /* unsupported shape kinds skipped (mosaic/text handled later) */
      }
    });
    annLayer.batchDraw();
  },
);

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
