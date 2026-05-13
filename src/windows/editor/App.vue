<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref, nextTick } from "vue";
import { call, on } from "../../shared/ipc";
import Stage from "./canvas/Stage.vue";
import Toolbar from "./ui/Toolbar.vue";
import ActionBar from "./ui/ActionBar.vue";
import Toast from "../../shared/Toast.vue";
import {
  editorState,
  undo,
  redo,
  commitChange,
  resetEditor,
} from "./state/shapes";

type StageExpose = {
  getStage: () => unknown;
};
const stageRef = ref<StageExpose | null>(null);

const state = reactive({
  imgUrl: "",
  width: 0,
  height: 0,
});

function onShortcut(e: KeyboardEvent) {
  if (e.target instanceof HTMLTextAreaElement) return;
  const key = e.key.toLowerCase();
  if (e.ctrlKey && key === "z" && !e.shiftKey) {
    e.preventDefault();
    undo();
  } else if (
    (e.ctrlKey && key === "y") ||
    (e.ctrlKey && e.shiftKey && key === "z")
  ) {
    e.preventDefault();
    redo();
  } else if (e.key === "Escape") {
    e.preventDefault();
    call("cancel_edit");
  } else if (
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

onMounted(() => {
  on<{ image_b64: string; width: number; height: number }>(
    "editor-ready",
    (p) => {
      resetEditor();
      state.imgUrl = `data:image/png;base64,${p.image_b64}`;
      state.width = p.width;
      state.height = p.height;
    },
  );
  window.addEventListener("keydown", onShortcut);
  nextTick(() => {
    (window as unknown as { __editorStage?: unknown }).__editorStage =
      stageRef.value?.getStage();
  });
});

onUnmounted(() => window.removeEventListener("keydown", onShortcut));
</script>

<template>
  <div class="editor">
    <Toolbar />
    <Stage
      ref="stageRef"
      :image-url="state.imgUrl"
      :width="state.width"
      :height="state.height"
    />
    <ActionBar />
    <Toast />
  </div>
</template>

<style scoped src="./editor.css"></style>
