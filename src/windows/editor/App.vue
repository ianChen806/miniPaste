<script setup lang="ts">
import { onMounted, reactive } from "vue";
import { on } from "../../shared/ipc";
import Stage from "./canvas/Stage.vue";
import Toolbar from "./ui/Toolbar.vue";
import ActionBar from "./ui/ActionBar.vue";

const state = reactive({
  imgUrl: "",
  width: 0,
  height: 0,
});

onMounted(() => {
  on<{ image_b64: string; width: number; height: number }>(
    "editor-ready",
    (p) => {
      state.imgUrl = `data:image/png;base64,${p.image_b64}`;
      state.width = p.width;
      state.height = p.height;
    },
  );
});
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
