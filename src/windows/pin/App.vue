<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { call } from "../../shared/ipc";

interface PinData {
  label: string;
  image_b64: string;
  width: number;
  height: number;
}

declare global {
  interface Window {
    __pinData?: PinData;
  }
}

const data = window.__pinData;
const ready = ref(!!data);

const ZOOM_STEP = 1.1;
const MIN_EDGE = 40;
const MAX_EDGE = 6000;
let curW = data?.width ?? 100;
let curH = data?.height ?? 100;

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  getCurrentWebviewWindow()
    .startDragging()
    .catch((err) => console.error("startDragging failed", err));
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  e.stopPropagation();
  console.log("[pin] wheel deltaY=", e.deltaY, "curSize=", curW, curH);
  const factor = e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
  let nw = curW * factor;
  let nh = curH * factor;
  const minSide = Math.min(nw, nh);
  const maxSide = Math.max(nw, nh);
  if (minSide < MIN_EDGE) {
    const k = MIN_EDGE / minSide;
    nw *= k;
    nh *= k;
  }
  if (maxSide > MAX_EDGE) {
    const k = MAX_EDGE / maxSide;
    nw *= k;
    nh *= k;
  }
  if (Math.abs(nw - curW) < 0.5) return;
  curW = nw;
  curH = nh;
  getCurrentWebviewWindow()
    .setSize(new LogicalSize(Math.round(curW), Math.round(curH)))
    .catch((err) => console.error("setSize failed", err));
}

async function closePin() {
  if (!data) return;
  try {
    await call<void>("pin_close", { label: data.label });
  } catch (err) {
    console.error("pin_close failed", err);
  }
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  closePin();
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    closePin();
  }
}

onMounted(async () => {
  if (!data) {
    console.error("pin window mounted without __pinData");
    return;
  }
  // Listener registered with passive:false so preventDefault is honored.
  // Vue template @wheel may default to passive:true on some setups.
  window.addEventListener("wheel", onWheel, { passive: false });
  window.addEventListener("keydown", onKeyDown);
  try {
    const win = getCurrentWebviewWindow();
    const phys = await win.innerSize();
    const scale = await win.scaleFactor();
    curW = phys.width / scale;
    curH = phys.height / scale;
    console.log("[pin] initSize", curW, curH);
  } catch (err) {
    console.error("innerSize read failed", err);
  }
});

onUnmounted(() => {
  window.removeEventListener("wheel", onWheel);
  window.removeEventListener("keydown", onKeyDown);
});
</script>

<template>
  <div
    v-if="ready && data"
    class="pin-root"
    @mousedown="onMouseDown"
    @contextmenu="onContextMenu"
  >
    <img
      :src="`data:image/png;base64,${data.image_b64}`"
      alt=""
      draggable="false"
      class="pin-image"
    />
  </div>
</template>

<style src="./pin.css"></style>
