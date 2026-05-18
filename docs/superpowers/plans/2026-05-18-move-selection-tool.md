# Move Selection Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "move" tool to the editor toolbar, make it the default. When active, dragging inside the selection translates the selection frame over the captured background.

**Architecture:** Type-level addition (`"move"` in `ToolType`), a new Toolbar button, a default-tool change, plus two event-dispatch tweaks: App.vue starts a `"move"` drag when tool is move and a shape was NOT hit; Stage.vue stops DOM propagation on shape-hit mousedown so the two paths never both fire.

**Tech Stack:** TypeScript, Vue 3, Konva (Stage), Vitest (for the one new unit assertion).

**Spec:** `docs/superpowers/specs/2026-05-18-move-selection-tool-design.md`

---

### Task 1: Extend `ToolType` and change default tool

**Files:**
- Modify: `src/shared/types.ts`
- Modify: `src/shared/editor/state/shapes.ts`

- [ ] **Step 1: Add `"move"` to `ToolType`**

In `src/shared/types.ts`, change:

```ts
export type ToolType = "line" | "rect" | "arrow" | "mosaic" | "text";
```

to:

```ts
export type ToolType = "move" | "line" | "rect" | "arrow" | "mosaic" | "text";
```

- [ ] **Step 2: Change the default tool in editor state**

In `src/shared/editor/state/shapes.ts`, change line 14:

```ts
  tool: "rect" as ToolType,
```

to:

```ts
  tool: "move" as ToolType,
```

- [ ] **Step 3: Run the test suite**

Run: `npm test`

Expected: all tests PASS. (No tests pin the default tool; the type change is additive.)

---

### Task 2: Add the Move button to the toolbar

**Files:**
- Modify: `src/shared/editor/ui/Toolbar.vue`

- [ ] **Step 1: Add `"move"` to the tools array**

In `src/shared/editor/ui/Toolbar.vue`, change lines 6–12 so the `tools` array starts with a Move entry:

```ts
const tools: { key: ToolType; label: string }[] = [
  { key: "move", label: "✥" },
  { key: "line", label: "／" },
  { key: "rect", label: "▭" },
  { key: "arrow", label: "↗" },
  { key: "mosaic", label: "▦" },
  { key: "text", label: "T" },
];
```

`✥` is a placeholder icon (4-direction-arrow vibe). Icon polish is out of scope per the spec.

- [ ] **Step 2: Type-check the build**

Run: `npm run build`

Expected: `vue-tsc` passes (the `ToolType` change in Task 1 makes `"move"` valid).

---

### Task 3: Stop DOM propagation in Stage when a shape is hit

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue:199-237`

- [ ] **Step 1: Update the Stage mousedown handler**

In `src/shared/editor/canvas/Stage.vue`, locate the `stage.on("mousedown", (e) => { ... })` block (around lines 199–237). Add a shape-hit guard at the very start of the handler:

```ts
  stage.on("mousedown", (e) => {
    if (e.target !== stage && e.target.id()) {
      // A shape was hit. Stop the DOM event from bubbling up to App.vue,
      // otherwise App.vue would also start a "move" drag and the shape
      // and selection would both move.
      e.evt.stopPropagation();
      return;
    }
    if (e.target !== stage) return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
    if (editorState.tool === "text") {
      // ...existing text branch unchanged...
```

The rest of the handler (text branch, draft branch) is unchanged. Make sure only the leading lines change.

- [ ] **Step 2: Run the test suite**

Run: `npm test`

Expected: all tests PASS.

---

### Task 4: Dispatch move in App.vue mousedown and update the cursor

**Files:**
- Modify: `src/windows/overlay/App.vue`

- [ ] **Step 1: Import `editorState` (if not already)**

Open `src/windows/overlay/App.vue` and confirm the import already brings in `editorState`. From the existing imports near the top:

```ts
import { editorState, resetEditor, undo, redo, commitChange } from "../../shared/editor/state/shapes";
```

This line exists — no change needed. If it's missing, add `editorState` to it.

- [ ] **Step 2: Add a move branch to `onMouseDown`**

Locate `onMouseDown` (around line 193). The current `editing` branch reads:

```ts
  } else if (state.phase === "editing" && state.selection) {
    const target = e.target as HTMLElement | null;
    if (target?.closest(".floating-toolbar")) return;
    const pt = { x: e.clientX, y: e.clientY };
    const hit = hitTestHandle(state.selection, pt);
    if (hit === null) {
      void requestReframe();
    } else if (hit !== "move") {
      state.activeHandle = hit;
      state.dragLast = pt;
    }
  }
```

Replace with:

```ts
  } else if (state.phase === "editing" && state.selection) {
    const target = e.target as HTMLElement | null;
    if (target?.closest(".floating-toolbar")) return;
    const pt = { x: e.clientX, y: e.clientY };
    const hit = hitTestHandle(state.selection, pt);
    if (hit === null) {
      void requestReframe();
    } else if (hit === "move") {
      if (editorState.tool === "move") {
        state.activeHandle = "move";
        state.dragLast = pt;
      }
    } else {
      state.activeHandle = hit;
      state.dragLast = pt;
    }
  }
```

The existing `onMouseMove` handler already routes `state.activeHandle === "move"` through `resizeRect`, which translates `selection.x/y`. No change there.

- [ ] **Step 3: Add a `hoverPoint` ref and a cursor computed property**

Introduce a separate `hoverPoint` ref for cursor purposes (we don't reuse `state.cursor` because that drives the magnifier, which should remain framing/resize-only).

Just after the existing `magCanvas` / `toolbarRef` refs (around line 32-33), add:

```ts
const hoverPoint = ref<SelPoint | null>(null);
```

Then, just after `toolbarStyle` (around line 81), add a cursor computed:

```ts
const overlayCursor = computed(() => {
  if (state.phase !== "editing" || !state.selection || !hoverPoint.value) {
    return "crosshair";
  }
  const hit = hitTestHandle(state.selection, hoverPoint.value);
  if (hit && hit !== "move") return cursorForHandle(hit);
  if (hit === "move" && editorState.tool === "move") return "move";
  return "crosshair";
});
```

`cursorForHandle` lives in `./handles`. Update the existing handles import to include it:

```ts
import { hitTestHandle, resizeRect, cursorForHandle, type HandleId } from "./handles";
```

- [ ] **Step 4: Apply the cursor to the overlay**

Find the root `<div class="overlay" ...>` in the template (around line 265). Add a dynamic style binding for the cursor:

```vue
  <div
    class="overlay"
    :style="{ backgroundImage: `url(${state.bgUrl})`, cursor: overlayCursor }"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDblClick"
    @contextmenu="onContextMenu"
  >
```

The existing `cursor: crosshair` rule in `overlay.css` becomes the default; the inline binding overrides it dynamically.

- [ ] **Step 5: Update `hoverPoint` from `onMouseMove`**

Locate `onMouseMove` (around line 211). Add `hoverPoint.value = pt;` at the top of the editing branch (without touching `state.cursor`). Replace the function body with:

```ts
function onMouseMove(e: MouseEvent) {
  const pt = { x: e.clientX, y: e.clientY };
  if (state.phase === "framing") {
    state.cursor = pt;
    if (state.dragStart) state.dragEnd = pt;
  } else if (state.phase === "editing" && state.activeHandle && state.dragLast && state.selection) {
    const delta = { x: pt.x - state.dragLast.x, y: pt.y - state.dragLast.y };
    state.selection = resizeRect(state.selection, state.activeHandle, delta);
    state.dragLast = pt;
    state.cursor = pt;
    hoverPoint.value = pt;
  } else if (state.phase === "editing") {
    hoverPoint.value = pt;
    state.cursor = null;
  } else {
    state.cursor = null;
    hoverPoint.value = null;
  }
}
```

The diff vs. the original: a new `else if (state.phase === "editing")` branch that updates `hoverPoint` only (not `state.cursor`, so the magnifier stays hidden in editing-idle hover); plus `hoverPoint.value = pt;` in the active-handle branch so the cursor stays correct mid-drag.

- [ ] **Step 6: Type-check and run tests**

Run: `npm test && npm run build`

Expected: tests PASS, `vue-tsc` passes.

---

### Task 5: Manual verification on the running app

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

Expected: minipaste process running.

- [ ] **Step 3: Verify default tool and move behaviour**

1. Hotkey → frame any region → enter editing
2. Toolbar: Move button (`✥`) should appear leftmost and be **active** (highlighted)
3. In selection-interior empty space, press-drag-release → selection frame slides over the captured background; background does not move
4. Cursor while hovering inside selection (tool=move) → `move` (4-arrow) cursor
5. Cursor while hovering over a resize handle → resize cursor (`nwse-resize` etc.)

- [ ] **Step 4: Verify tool switching and conflict handling**

1. Click `▭` (rect) → tool becomes rect
2. Draw a rectangle inside the selection → drawing works as before, selection does NOT move
3. Switch back to Move (`✥`)
4. Click on the previously drawn rectangle → it gets selected (Konva transformer appears); drag it → the shape moves, the selection frame does **not**
5. Click on empty space again → selection moves, the existing shape stays put

- [ ] **Step 5: Verify reframe and resize still work**

1. Tool=move, click outside the selection → triggers reframe (returns to framing phase)
2. Tool=move, drag a corner handle → still resizes (move dispatch must not capture handle clicks)

---

### Task 6: Commit

**Files:** none (git only)

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add src/shared/types.ts \
        src/shared/editor/state/shapes.ts \
        src/shared/editor/ui/Toolbar.vue \
        src/shared/editor/canvas/Stage.vue \
        src/windows/overlay/App.vue
git commit -m "feat(editor): add Move tool for translating the selection frame"
```

Expected: clean commit with five files, nothing else pulled in.
