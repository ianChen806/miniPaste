# Editor Tool Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** (1) Reset the editor tool to `"move"` on each capture. (2) Make existing shapes pass-through clicks when a draw tool is active, so the user can draw new shapes on top of old ones without accidentally grabbing them.

**Architecture:** Bug 1 is a one-line addition in `resetEditor()`. Bug 2 uses Konva's `Layer.listening()` flag: when the current tool isn't `"move"`, the annotations layer ignores all pointer events and the shapes behave as if they weren't there. A Vue `watch` keeps `annLayer.listening()` in sync with `editorState.tool` and clears the transformer when leaving move mode.

**Tech Stack:** TypeScript, Vue 3 (reactive + watch), Konva (Layer listening).

**Spec:** `docs/superpowers/specs/2026-05-18-editor-tool-fixes-design.md`

---

### Task 1: Reset the editor tool on `resetEditor()`

**Files:**
- Modify: `src/shared/editor/state/shapes.ts`

- [ ] **Step 1: Add the tool reset**

In `src/shared/editor/state/shapes.ts`, change the existing `resetEditor()`:

```ts
export function resetEditor() {
  editorState.shapes = [];
  editorState.selectedId = null;
  history = createHistory();
  history.push([]);
}
```

to:

```ts
export function resetEditor() {
  editorState.shapes = [];
  editorState.selectedId = null;
  editorState.tool = "move";
  history = createHistory();
  history.push([]);
}
```

- [ ] **Step 2: Run the test suite**

Run: `npm test`

Expected: all tests PASS. No existing test pins the post-reset tool value.

---

### Task 2: Wire `annLayer.listening` to the current tool in Stage.vue

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue`

- [ ] **Step 1: Add `watch` to the existing Vue imports**

Open `src/shared/editor/canvas/Stage.vue` and find the existing imports block. The Vue import currently reads:

```ts
import { onMounted, onUnmounted, ref, watch } from "vue";
```

`watch` is already imported — no change. (If it ever gets removed, re-add it.)

- [ ] **Step 2: Initialise `annLayer.listening` after the layer is added**

In `onMounted` (around line 119), after the four layers are added to the stage and the transformer is created (i.e. after the existing `uiLayer.add(transformer);` line), add one line that sets the initial listening state:

```ts
  annLayer.listening(editorState.tool === "move");
```

For reference, this lands just before the existing `loadBg();` call inside `onMounted`.

- [ ] **Step 3: Add a `watch` that keeps listening in sync and clears the transformer when leaving move**

Inside `onMounted`, immediately after the line you added in Step 2, add the watch:

```ts
  watch(() => editorState.tool, (t) => {
    if (!annLayer || !transformer) return;
    annLayer.listening(t === "move");
    if (t !== "move") {
      transformer.nodes([]);
      editorState.selectedId = null;
    }
  });
```

The watch is registered inside `onMounted` so the closure captures the just-created `annLayer` and `transformer`. It does NOT need `immediate: true` because Step 2 already handled the initial state.

- [ ] **Step 4: Run the test suite**

Run: `npm test`

Expected: all tests PASS. No existing test exercises Konva layer listening.

- [ ] **Step 5: Type-check the build**

Run: `npm run build`

Expected: `vue-tsc` passes.

---

### Task 3: Manual verification on the running app

**Files:** none (smoke test against built binary)

- [ ] **Step 1: Build the release binary**

Run: `npx tauri build`

Expected: build succeeds. (Per `memory/tauri-build-trap.md`, never use `cargo build --release` alone.)

- [ ] **Step 2: Stop any existing instance and launch the new build**

Run (PowerShell):

```powershell
Get-Process minipaste -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process "D:\SideProject\MiniPaste\src-tauri\target\release\minipaste.exe"
```

Expected: minipaste running. (Per `memory/hotkey-multi-instance.md`, kill old PID before launching.)

- [ ] **Step 3: Verify Bug 1 — tool resets per capture**

1. Hotkey → frame anything → enter editing → toolbar Move (`✥`) should be active
2. Click `▭` (rect) → tool becomes rect
3. Esc to cancel
4. Hotkey again → frame → enter editing
5. Expected: Move (`✥`) is active again, NOT rect

- [ ] **Step 4: Verify Bug 2 — draw on top of existing shapes**

1. Hotkey → frame → editing
2. Click `▭`, draw a rectangle inside the selection
3. Without changing tool, press-drag-release starting **inside that rectangle** (e.g. its centre)
4. Expected: a SECOND rectangle starts drawing from the press point. The first rectangle does NOT get grabbed and does NOT get a transformer.
5. Repeat with a line and arrow tool on top of the rectangle. All should draw.

- [ ] **Step 5: Verify Move tool still lets you drag/select**

1. Switch to Move (`✥`)
2. Click on one of the rectangles → transformer handles appear; shape becomes selected
3. Drag it → shape moves; selection frame does NOT move
4. Click empty space inside selection → transformer clears; subsequent drag moves the selection frame (existing Move-tool behaviour)

- [ ] **Step 6: Verify cross-tool transformer cleanup**

1. Move tool → click a shape → transformer handles visible
2. Switch to `▭`
3. Expected: transformer handles disappear immediately (no orphan handles left behind)

- [ ] **Step 7: Verify continuous drawing on top**

1. Rect tool → draw three rectangles where each new one **starts inside** the previous rectangle
2. Expected: all three draw correctly; no rectangle ever gets grabbed/moved by the next mousedown

---

### Task 4: Commit

**Files:** none (git only)

- [ ] **Step 1: Stage and commit**

Run:

```bash
git add src/shared/editor/state/shapes.ts src/shared/editor/canvas/Stage.vue
git commit -m "fix(editor): reset tool per capture; make shapes pass-through for draw tools"
```

Expected: clean commit with two files, nothing else pulled in.
