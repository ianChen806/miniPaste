<script setup lang="ts">
import Konva from "konva";
import { onMounted, ref, watch } from "vue";

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
