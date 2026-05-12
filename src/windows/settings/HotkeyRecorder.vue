<script setup lang="ts">
// Full impl arrives in Task 15.
defineProps<{ modelValue: string }>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <input class="hotkey-input" :value="modelValue" readonly />
</template>
