<script setup lang="ts">
import { editorState, undo, redo } from "../state/shapes";
import { COLOR_HEX, COLOR_ORDER } from "../../colors";
import type { ToolType, Thickness } from "../../types";

const tools: { key: ToolType; label: string }[] = [
  { key: "line", label: "／" },
  { key: "rect", label: "▭" },
  { key: "arrow", label: "↗" },
  { key: "mosaic", label: "▦" },
  { key: "text", label: "T" },
];
const thicknesses: Thickness[] = ["thin", "medium", "thick"];
</script>

<template>
  <div class="toolbar">
    <button
      v-for="t in tools"
      :key="t.key"
      :class="{ active: editorState.tool === t.key }"
      @click="editorState.tool = t.key"
    >
      {{ t.label }}
    </button>

    <span class="sep"></span>

    <button
      v-for="c in COLOR_ORDER"
      :key="c"
      class="swatch"
      :class="{ active: editorState.color === c }"
      :style="{ background: COLOR_HEX[c] }"
      @click="editorState.color = c"
    ></button>

    <span class="sep"></span>

    <button
      v-for="t in thicknesses"
      :key="t"
      :class="{ active: editorState.thickness === t }"
      @click="editorState.thickness = t"
    >
      {{ t[0].toUpperCase() }}
    </button>

    <span class="sep"></span>

    <button @click="undo">↶</button>
    <button @click="redo">↷</button>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: 6px;
  align-items: center;
  padding: 4px 8px;
  border-bottom: 1px solid #3a3a3a;
}
.toolbar button {
  padding: 4px 10px;
  background: #f3f4f6;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  cursor: pointer;
}
.toolbar button.active {
  background: #3b82f6;
  color: white;
  border-color: #2563eb;
}
.swatch {
  width: 22px;
  height: 22px;
  padding: 0 !important;
  border-radius: 50% !important;
}
.swatch.active {
  outline: 2px solid #f3f4f6;
  outline-offset: 2px;
}
.sep {
  width: 1px;
  height: 18px;
  background: #3a3a3a;
}
</style>
