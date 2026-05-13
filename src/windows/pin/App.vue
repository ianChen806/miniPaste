<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
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

async function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  e.preventDefault();
  try {
    await getCurrentWebviewWindow().startDragging();
  } catch (err) {
    console.error("startDragging failed", err);
  }
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

function onDblClick(e: MouseEvent) {
  e.preventDefault();
  closePin();
}

onMounted(() => {
  if (!data) {
    console.error("pin window mounted without __pinData");
  }
});
</script>

<template>
  <div
    v-if="ready && data"
    class="pin-root"
    @mousedown="onMouseDown"
    @contextmenu="onContextMenu"
    @dblclick="onDblClick"
  >
    <img
      :src="`data:image/png;base64,${data.image_b64}`"
      alt=""
      draggable="false"
      class="pin-image"
    />
  </div>
</template>

<style scoped src="./pin.css"></style>
