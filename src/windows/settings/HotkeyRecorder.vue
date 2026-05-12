<script setup lang="ts">
import { ref } from "vue";

defineProps<{ modelValue: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const recording = ref(false);

function format(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Meta");
  const k = e.key;
  if (!["Control", "Shift", "Alt", "Meta"].includes(k)) {
    parts.push(k.length === 1 ? k.toUpperCase() : k);
  }
  return parts.join("+");
}

function onKeydown(e: KeyboardEvent) {
  e.preventDefault();
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
  const formatted = format(e);
  if (formatted.includes("+")) {
    emit("update:modelValue", formatted);
    recording.value = false;
  }
}
</script>

<template>
  <input
    class="hotkey-input"
    :value="recording ? '(press combo...)' : modelValue"
    readonly
    @focus="recording = true"
    @blur="recording = false"
    @keydown="onKeydown"
  />
</template>
