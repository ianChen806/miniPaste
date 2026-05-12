<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import { rectFromDrag, clampToBounds, type Point } from "./selection";

const state = reactive({
  bgUrl: "",
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },
  dragStart: null as Point | null,
  dragEnd: null as Point | null,
});

const selectionStyle = computed(() => {
  if (!state.dragStart || !state.dragEnd) return {};
  const r = rectFromDrag(state.dragStart, state.dragEnd);
  return {
    left: r.x + "px",
    top: r.y + "px",
    width: r.w + "px",
    height: r.h + "px",
  };
});

onMounted(() => {
  on<{
    thumbnail_b64: string;
    width: number;
    height: number;
    origin_x: number;
    origin_y: number;
  }>("capture-ready", (p) => {
    state.bgUrl = `data:image/png;base64,${p.thumbnail_b64}`;
    state.width = p.width;
    state.height = p.height;
    state.origin = { x: p.origin_x, y: p.origin_y };
  });
  window.addEventListener("keydown", onKey);
});

onUnmounted(() => window.removeEventListener("keydown", onKey));

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") cancel();
}

function onMouseDown(e: MouseEvent) {
  state.dragStart = { x: e.clientX, y: e.clientY };
  state.dragEnd = { x: e.clientX, y: e.clientY };
}

function onMouseMove(e: MouseEvent) {
  if (state.dragStart) state.dragEnd = { x: e.clientX, y: e.clientY };
}

async function onMouseUp() {
  if (!state.dragStart || !state.dragEnd) return;
  const local = rectFromDrag(state.dragStart, state.dragEnd);
  const clamped = clampToBounds(local, state.width, state.height);
  state.dragStart = null;
  state.dragEnd = null;
  if (clamped.w < 5 || clamped.h < 5) return;
  const rectInOsCoords = {
    x: clamped.x + state.origin.x,
    y: clamped.y + state.origin.y,
    w: clamped.w,
    h: clamped.h,
  };
  await call("selection_confirmed", { rect: rectInOsCoords });
}

async function cancel() {
  await call("selection_cancelled");
}
</script>

<template>
  <div
    class="overlay"
    :style="{ backgroundImage: `url(${state.bgUrl})` }"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
  >
    <div class="dim"></div>
    <div
      v-if="state.dragStart && state.dragEnd"
      class="selection"
      :style="selectionStyle"
    ></div>
  </div>
</template>

<style scoped src="./overlay.css"></style>
