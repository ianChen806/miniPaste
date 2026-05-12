<script setup lang="ts">
import { call } from "../../../shared/ipc";
import { pushToast } from "../../../shared/toast";
import type { FinishAction, FinishOutcome } from "../../../shared/types";
import type Konva from "konva";

interface EditorStageGlobal {
  __editorStage?: Konva.Stage;
}

async function exportPng(): Promise<Uint8Array> {
  const w = window as unknown as EditorStageGlobal;
  const stage = w.__editorStage;
  if (!stage) throw new Error("editor stage not ready");
  const dataUrl: string = stage.toDataURL({ pixelRatio: 1 });
  const res = await fetch(dataUrl);
  const buf = await res.arrayBuffer();
  return new Uint8Array(buf);
}

async function doAction(action: FinishAction) {
  try {
    const bytes = Array.from(await exportPng());
    const outcome = await call<FinishOutcome>("finish_action", {
      action,
      imageBytes: bytes,
    });
    if (outcome.saved_path) {
      pushToast("success", `Saved: ${outcome.saved_path}`);
    } else {
      pushToast("success", "Copied to clipboard");
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    pushToast("error", msg);
  }
}

async function copyImage() {
  doAction({ kind: "CopyImage" });
}

async function saveAs() {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({
    defaultPath: "screenshot.png",
    filters: [{ name: "Image", extensions: ["png", "jpg"] }],
  });
  if (path) doAction({ kind: "Save", path });
}

async function saveAndCopy() {
  doAction({ kind: "SaveAndCopyPath" });
}
</script>

<template>
  <div class="action-bar">
    <button @click="copyImage">Copy</button>
    <button @click="saveAs">Save...</button>
    <button @click="saveAndCopy">Save+Copy</button>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  justify-content: center;
  gap: 12px;
  padding: 8px;
  border-top: 1px solid #3a3a3a;
}
.action-bar button {
  padding: 8px 20px;
  background: #3b82f6;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}
.action-bar button:hover {
  background: #2563eb;
}
</style>
