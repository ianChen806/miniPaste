<script setup lang="ts">
import { toastState } from "./toast";
</script>

<template>
  <div class="toast-host">
    <div
      v-for="t in toastState.list"
      :key="t.id"
      :class="['toast', t.level]"
    >
      {{ t.msg }}
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  bottom: 20px;
  right: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 9999;
}
.toast {
  padding: 8px 14px;
  border-radius: 4px;
  color: white;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}
.toast.error {
  background: #b91c1c;
}
.toast.success {
  background: #15803d;
}
.toast.info {
  background: #1f2937;
}
</style>
