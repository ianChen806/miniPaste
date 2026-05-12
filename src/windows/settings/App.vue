<script setup lang="ts">
import { onMounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import type { Config } from "../../shared/types";
import HotkeyRecorder from "./HotkeyRecorder.vue";

const state = reactive({
  loaded: false,
  config: null as Config | null,
  error: "" as string,
});

onMounted(async () => {
  try {
    state.config = await call<Config>("get_config");
    state.loaded = true;
  } catch (e: unknown) {
    state.error = errorMessage(e);
  }
  on<{ attempted: string; reason: string }>("hotkey-conflict", (p) => {
    state.error = `Hotkey "${p.attempted}" 衝突：${p.reason}`;
  });
});

async function pickFolder() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    directory: true,
    defaultPath: state.config?.default_save_path,
  });
  if (picked && typeof picked === "string" && state.config) {
    state.config.default_save_path = picked;
  }
}

async function apply() {
  if (!state.config) return;
  try {
    await call<void>("update_config", { new: state.config });
    state.error = "";
  } catch (e: unknown) {
    state.error = errorMessage(e);
  }
}

function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return String(e);
}
</script>

<template>
  <div class="settings" v-if="state.loaded && state.config">
    <h2>Settings</h2>

    <label>
      Hotkey
      <HotkeyRecorder v-model="state.config.hotkey" />
    </label>

    <label>
      Default folder
      <div class="row">
        <input :value="state.config.default_save_path" readonly />
        <button type="button" @click="pickFolder">📁</button>
      </div>
    </label>

    <label>
      Format
      <div class="row">
        <label><input type="radio" value="png" v-model="state.config.image_format" /> PNG</label>
        <label><input type="radio" value="jpeg" v-model="state.config.image_format" /> JPEG</label>
      </div>
    </label>

    <label v-if="state.config.image_format === 'jpeg'">
      JPEG quality
      <div class="row">
        <input type="range" min="1" max="100" v-model.number="state.config.jpeg_quality" />
        <span>{{ state.config.jpeg_quality }}</span>
      </div>
    </label>

    <p class="error" v-if="state.error">{{ state.error }}</p>

    <div class="actions">
      <button type="button" @click="apply">Save &amp; Apply</button>
    </div>
  </div>
  <div v-else-if="!state.loaded">Loading...</div>
</template>

<style scoped src="./settings.css"></style>
