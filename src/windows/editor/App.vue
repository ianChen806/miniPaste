<script setup lang="ts">
import { onMounted, onUnmounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import Stage from "./canvas/Stage.vue";
import Toolbar from "./ui/Toolbar.vue";
import ActionBar from "./ui/ActionBar.vue";
import { editorState, undo, redo, commitChange } from "./state/shapes";

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
      state.imgUrl = `data:image/png;base64,${p.image_b64}`;
      state.width = p.width;
      state.height = p.height;
    },
  );
  window.addEventListener("keydown", onShortcut);
});

onUnmounted(() => window.removeEventListener("keydown", onShortcut));
</script>

<template>
  <div class="editor">
    <Toolbar />
    <Stage
      :image-url="state.imgUrl"
      :width="state.width"
      :height="state.height"
    />
    <ActionBar />
  </div>
</template>

<style scoped src="./editor.css"></style>
