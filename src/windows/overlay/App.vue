<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { call, on } from "../../shared/ipc";
import { rectFromDrag, clampToBounds, type Point as SelPoint } from "./selection";
import { hitTestHandle, resizeRect, type HandleId } from "./handles";
import { placeToolbar } from "./toolbarPlacement";
import { findActiveScreen } from "./findActiveScreen";
import { renderMagnifier, MAGNIFIER_SIZE } from "./magnifier";
import Stage from "../../shared/editor/canvas/Stage.vue";
import Toolbar from "../../shared/editor/ui/Toolbar.vue";
import ActionBar from "../../shared/editor/ui/ActionBar.vue";
import { editorState, resetEditor, undo, redo, commitChange } from "../../shared/editor/state/shapes";
import Toast from "../../shared/Toast.vue";
import type { Rect, ScreenInfo } from "../../shared/types";

type Phase = "idle" | "framing" | "editing";

const state = reactive({
  phase: "idle" as Phase,
  bgUrl: "",
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },
  screens: [] as Rect[],
  dragStart: null as SelPoint | null,
  dragEnd: null as SelPoint | null,
  selection: null as Rect | null,
  cursor: null as SelPoint | null,
  activeHandle: null as HandleId | null,
  dragLast: null as SelPoint | null,
});

const bgImg = ref<HTMLImageElement | null>(null);
const magCanvas = ref<HTMLCanvasElement | null>(null);
const toolbarRef = ref<HTMLDivElement | null>(null);

const framingRectStyle = computed(() => {
  if (!state.dragStart || !state.dragEnd) return { display: "none" };
  const r = rectFromDrag(state.dragStart, state.dragEnd);
  return {
    left: r.x + "px",
    top: r.y + "px",
    width: r.w + "px",
    height: r.h + "px",
  };
});

const selectionStyle = computed(() => {
  if (!state.selection) return { display: "none" };
  const s = state.selection;
  return {
    left: s.x + "px",
    top: s.y + "px",
    width: s.w + "px",
    height: s.h + "px",
  };
});

const stageOffsetStyle = computed(() => {
  if (!state.selection) return { display: "none" };
  return {
    left: -state.selection.x + "px",
    top: -state.selection.y + "px",
  };
});

const toolbarPlacement = computed(() => {
  if (!state.selection) return null;
  const tbar = toolbarRef.value
    ? { w: toolbarRef.value.offsetWidth, h: toolbarRef.value.offsetHeight }
    : { w: 320, h: 80 };
  const fallback: Rect = { x: 0, y: 0, w: state.width, h: state.height };
  const bounds = findActiveScreen(state.selection, state.screens, fallback);
  return placeToolbar(state.selection, tbar, bounds);
});

const toolbarStyle = computed(() => {
  const p = toolbarPlacement.value;
  if (!p) return { display: "none" };
  return { left: p.x + "px", top: p.y + "px" };
});

const magnifierStyle = computed(() => {
  if (!state.cursor) return { display: "none" };
  const off = 20;
  const right = state.cursor.x + off + MAGNIFIER_SIZE > state.width;
  const bottom = state.cursor.y + off + MAGNIFIER_SIZE > state.height;
  const left = right ? state.cursor.x - off - MAGNIFIER_SIZE : state.cursor.x + off;
  const top = bottom ? state.cursor.y - off - MAGNIFIER_SIZE : state.cursor.y + off;
  return { left: left + "px", top: top + "px" };
});

function drawMagnifier() {
  if (!magCanvas.value || !bgImg.value || !state.cursor) return;
  const ctx = magCanvas.value.getContext("2d");
  if (!ctx) return;
  renderMagnifier(ctx, bgImg.value, state.cursor, 5);
}

watch(() => state.cursor, drawMagnifier);

onMounted(() => {
  on<{
    thumbnail_b64: string;
    width: number;
    height: number;
    origin_x: number;
    origin_y: number;
    screens: ScreenInfo[];
  }>("capture-ready", async (p) => {
    state.bgUrl = `data:image/png;base64,${p.thumbnail_b64}`;
    state.width = p.width;
    state.height = p.height;
    state.origin = { x: p.origin_x, y: p.origin_y };
    state.screens = (p.screens ?? []).map((s) => ({
      x: s.x - p.origin_x,
      y: s.y - p.origin_y,
      w: s.w,
      h: s.h,
    }));
    const img = new Image();
    img.src = state.bgUrl;
    await img.decode().catch(() => {});
    bgImg.value = img;
    resetEditor();
    state.phase = "framing";
    state.selection = null;
    state.dragStart = null;
    state.dragEnd = null;
    state.cursor = null;
  });

  on("capture-clear", () => {
    state.phase = "idle";
    state.bgUrl = "";
    state.selection = null;
    state.dragStart = null;
    state.dragEnd = null;
    state.cursor = null;
    state.screens = [];
    resetEditor();
  });

  window.addEventListener("keydown", onKey);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
});

function onKey(e: KeyboardEvent) {
  if (state.phase === "idle") return;
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
  if (e.target instanceof HTMLElement && e.target.isContentEditable) return;
  const key = e.key.toLowerCase();
  if (e.key === "Escape") {
    e.preventDefault();
    void cancel();
  } else if (e.key === "Enter" && state.phase === "editing") {
    e.preventDefault();
    defaultAction();
  } else if (state.phase === "editing" && e.ctrlKey && key === "z" && !e.shiftKey) {
    e.preventDefault();
    undo();
  } else if (
    state.phase === "editing" &&
    ((e.ctrlKey && key === "y") || (e.ctrlKey && e.shiftKey && key === "z"))
  ) {
    e.preventDefault();
    redo();
  } else if (
    state.phase === "editing" &&
    (e.key === "Delete" || e.key === "Backspace") &&
    editorState.selectedId
  ) {
    e.preventDefault();
    const id = editorState.selectedId;
    editorState.shapes = editorState.shapes.filter((s) => s.id !== id);
    editorState.selectedId = null;
    commitChange();
  }
}

async function cancel() {
  await call("selection_cancelled");
}

function defaultAction() {
  const hook = (window as unknown as { __overlayActionBarCopy?: () => void }).__overlayActionBarCopy;
  if (hook) hook();
}

function onMouseDown(e: MouseEvent) {
  if (state.phase === "framing") {
    state.dragStart = { x: e.clientX, y: e.clientY };
    state.dragEnd = { x: e.clientX, y: e.clientY };
  } else if (state.phase === "editing" && state.selection) {
    const target = e.target as HTMLElement | null;
    if (target?.closest(".floating-toolbar")) return;
    const pt = { x: e.clientX, y: e.clientY };
    const hit = hitTestHandle(state.selection, pt);
    if (hit === null) {
      void requestReframe();
    } else if (hit !== "move") {
      state.activeHandle = hit;
      state.dragLast = pt;
    }
  }
}

function onMouseMove(e: MouseEvent) {
  const pt = { x: e.clientX, y: e.clientY };
  if (state.phase === "framing") {
    state.cursor = pt;
    if (state.dragStart) state.dragEnd = pt;
  } else if (state.phase === "editing" && state.activeHandle && state.dragLast && state.selection) {
    const delta = { x: pt.x - state.dragLast.x, y: pt.y - state.dragLast.y };
    state.selection = resizeRect(state.selection, state.activeHandle, delta);
    state.dragLast = pt;
    state.cursor = pt;
  } else {
    state.cursor = null;
  }
}

async function onMouseUp() {
  if (state.phase === "framing") {
    if (!state.dragStart || !state.dragEnd) return;
    const local = rectFromDrag(state.dragStart, state.dragEnd);
    const clamped = clampToBounds(local, state.width, state.height);
    state.dragStart = null;
    state.dragEnd = null;
    if (clamped.w < 5 || clamped.h < 5) return;
    state.selection = clamped;
    try {
      await call("selection_confirmed");
      state.phase = "editing";
      state.cursor = null;
    } catch (err) {
      state.selection = null;
      state.phase = "framing";
      throw err;
    }
  } else if (state.phase === "editing" && state.activeHandle) {
    state.activeHandle = null;
    state.dragLast = null;
    state.cursor = null;
  }
}

async function requestReframe() {
  try {
    await call("reframe_request");
  } catch {
    /* race: phase already changed elsewhere; ignore */
  }
  state.phase = "framing";
  state.selection = null;
  resetEditor();
}

function onDblClick() {
  if (state.phase === "editing") defaultAction();
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  void cancel();
}
</script>

<template>
  <div
    class="overlay"
    :style="{ backgroundImage: `url(${state.bgUrl})` }"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDblClick"
    @contextmenu="onContextMenu"
  >
    <div class="dim"></div>

    <div v-if="state.phase === 'framing'" class="selection" :style="framingRectStyle"></div>

    <template v-if="state.phase === 'editing' && state.selection">
      <div class="selection editing-frame" :style="selectionStyle">
        <div class="handle nw"></div>
        <div class="handle n"></div>
        <div class="handle ne"></div>
        <div class="handle e"></div>
        <div class="handle se"></div>
        <div class="handle s"></div>
        <div class="handle sw"></div>
        <div class="handle w"></div>
      </div>

      <div class="stage-clip" :style="selectionStyle">
        <div class="stage-inner" :style="stageOffsetStyle">
          <Stage
            :bg-url="state.bgUrl"
            :selection="state.selection"
            :overlay-size="{ w: state.width, h: state.height }"
          />
        </div>
      </div>

      <div class="floating-toolbar" :style="toolbarStyle" ref="toolbarRef">
        <Toolbar />
        <ActionBar :crop="state.selection" />
      </div>
    </template>

    <canvas
      v-if="state.cursor"
      class="magnifier"
      :style="magnifierStyle"
      ref="magCanvas"
      width="120"
      height="120"
    ></canvas>

    <Toast />
  </div>
</template>

<style scoped src="./overlay.css"></style>
