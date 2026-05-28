# Pencil Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a freehand `pencil` tool to the overlay editor so users can draw arbitrary smooth strokes alongside the existing fixed-geometry tools.

**Architecture:** Each stroke is a new `Shape` whose `geometry` is `{ kind: "pencil", points: number[] }`. Rendering uses `Konva.Line` with `tension: 0.5` for built-in smoothing. The drafting branch in `Stage.vue` mutates the preview node's `points` array in place during `mousemove` instead of rebuilding the shape each frame. Selection allows translate-only (no resize); drag offsets are folded back into the points array on `dragend`.

**Tech Stack:** Vue 3 + TypeScript + Konva.js (canvas, already in use). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-05-28-pencil-tool-design.md`

---

## File Map

| File | Change |
|------|--------|
| `src/shared/types.ts` | Add `"pencil"` to `ToolType`; add `{ kind: "pencil"; points: number[] }` variant to `ShapeGeometry` |
| `src/shared/editor/canvas/drawTools.ts` | Add `case "pencil"` in `renderShape` with `tension`, `lineCap`, `lineJoin`, `hitStrokeWidth` |
| `src/shared/editor/canvas/Stage.vue` | Add `"pencil"` to `DRAW_TOOLS`; extend `Drafting` with optional `points`; pencil branches in `mousedown`/`mousemove`/`mouseup`; transformer resize-toggle in click handler; pencil branch in `dragend transformend` |
| `src/shared/editor/ui/Toolbar.vue` | Insert pencil button between `move` and `line` |

No new files. No new dependencies.

---

## Task 1: Add pencil to type system and render case

**Why these go together:** `renderShape` in `drawTools.ts` has a runtime-throwing default case. Adding `"pencil"` to the union without adding the render case would crash on rerender (which fires on every shape add/undo/redo). Both must land in one commit.

**Files:**
- Modify: `src/shared/types.ts:27`, `src/shared/types.ts:40-45`
- Modify: `src/shared/editor/canvas/drawTools.ts:38-101`

- [ ] **Step 1: Add `"pencil"` to `ToolType`**

In `src/shared/types.ts`, change line 27 from:

```ts
export type ToolType = "move" | "line" | "rect" | "arrow" | "mosaic" | "text";
```

to:

```ts
export type ToolType = "move" | "pencil" | "line" | "rect" | "arrow" | "mosaic" | "text";
```

- [ ] **Step 2: Add pencil variant to `ShapeGeometry`**

In `src/shared/types.ts`, change lines 40-45 from:

```ts
export type ShapeGeometry =
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "rect"; x: number; y: number; w: number; h: number }
  | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "mosaic"; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: "text"; x: number; y: number; w: number; h: number };
```

to:

```ts
export type ShapeGeometry =
  | { kind: "pencil"; points: number[] }
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "rect"; x: number; y: number; w: number; h: number }
  | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "mosaic"; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: "text"; x: number; y: number; w: number; h: number };
```

- [ ] **Step 3: Add pencil case to `renderShape`**

In `src/shared/editor/canvas/drawTools.ts`, inside the `switch (shape.geometry.kind)` in `renderShape`, add this case **before** the existing `case "line":` (line 42):

```ts
    case "pencil": {
      const g = shape.geometry;
      return new Konva.Line({
        points: g.points,
        stroke,
        strokeWidth: width,
        tension: 0.5,
        lineCap: "round",
        lineJoin: "round",
        hitStrokeWidth: Math.max(width, 10),
        id: shape.id,
        draggable: true,
      });
    }
```

- [ ] **Step 4: Verify build**

Run: `npm run build`
Expected: succeeds with no TypeScript errors. (vue-tsc runs as part of the build.)

- [ ] **Step 5: Commit**

```bash
git add src/shared/types.ts src/shared/editor/canvas/drawTools.ts
git commit -m "feat(editor): add pencil type and render case"
```

---

## Task 2: Register pencil as a draw tool and add toolbar button

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue:33`
- Modify: `src/shared/editor/ui/Toolbar.vue:6-13`

- [ ] **Step 1: Register pencil in `DRAW_TOOLS`**

In `src/shared/editor/canvas/Stage.vue`, change line 33 from:

```ts
const DRAW_TOOLS = ["line", "rect", "arrow", "mosaic"] as const;
```

to:

```ts
const DRAW_TOOLS = ["pencil", "line", "rect", "arrow", "mosaic"] as const;
```

This makes `isDrawTool()` recognize pencil so `mousedown` will start a draft for it.

- [ ] **Step 2: Insert pencil button in toolbar**

In `src/shared/editor/ui/Toolbar.vue`, change the `tools` array (lines 6-13) from:

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

to:

```ts
const tools: { key: ToolType; label: string }[] = [
  { key: "move", label: "✥" },
  { key: "pencil", label: "✎" },
  { key: "line", label: "／" },
  { key: "rect", label: "▭" },
  { key: "arrow", label: "↗" },
  { key: "mosaic", label: "▦" },
  { key: "text", label: "T" },
];
```

- [ ] **Step 3: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 4: Visual verify in dev mode**

Run: `npm run tauri dev`

In the overlay, confirm:
- The `✎` button appears between `✥` (move) and `／` (line).
- Clicking it highlights as active (blue background).
- No drawing happens yet on the canvas — that comes in Task 3. The button just switches state.

Press Ctrl+C in the terminal to stop the dev server when done.

- [ ] **Step 5: Commit**

```bash
git add src/shared/editor/canvas/Stage.vue src/shared/editor/ui/Toolbar.vue
git commit -m "feat(editor): add pencil button to toolbar"
```

---

## Task 3: Implement pencil drawing flow

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue:26-31` (Drafting type)
- Modify: `src/shared/editor/canvas/Stage.vue:209-298` (mousedown / mousemove / mouseup)

**Approach:** Pencil bypasses `buildDraftShape` / `renderPreview` (which are designed for two-point shapes) and manages its preview node directly. Points are appended in place; the existing destroy-and-rebuild path stays untouched for other tools.

- [ ] **Step 1: Extend the `Drafting` type to carry pencil points**

In `src/shared/editor/canvas/Stage.vue`, change lines 26-30 from:

```ts
type Drafting = {
  startX: number;
  startY: number;
  node: Konva.Node | null;
};
```

to:

```ts
type Drafting = {
  startX: number;
  startY: number;
  node: Konva.Node | null;
  points?: number[]; // pencil only — flat [x1, y1, x2, y2, ...]
};
```

- [ ] **Step 2: Add a pencil branch to `mousedown`**

In `src/shared/editor/canvas/Stage.vue`, locate the `mousedown` handler. After the existing line:

```ts
    if (!isDrawTool(editorState.tool)) return;
```

and **before** the existing line:

```ts
    drafting = { startX: pos.x, startY: pos.y, node: null };
```

insert this pencil branch:

```ts
    if (editorState.tool === "pencil") {
      const pencilNode = new Konva.Line({
        points: [pos.x, pos.y],
        stroke: COLOR_HEX[editorState.color],
        strokeWidth: STROKE_WIDTH[editorState.thickness],
        tension: 0.5,
        lineCap: "round",
        lineJoin: "round",
      });
      previewLayer!.add(pencilNode);
      previewLayer!.batchDraw();
      drafting = {
        startX: pos.x,
        startY: pos.y,
        node: pencilNode,
        points: [pos.x, pos.y],
      };
      return;
    }
```

Then add this import line at the top of the `<script setup>` block (near the existing imports). The existing line is:

```ts
import { MOSAIC_BLOCK, FONT_SIZE } from "../../colors";
```

Change it to:

```ts
import { MOSAIC_BLOCK, FONT_SIZE, COLOR_HEX, STROKE_WIDTH } from "../../colors";
```

- [ ] **Step 3: Add a pencil branch to `mousemove`**

In the `mousemove` handler, replace the entire body. Current body:

```ts
  stage.on("mousemove", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
    const draft = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (drafting.node) drafting.node.destroy();
    const node = renderPreview(draft);
    if (!node) return;
    drafting.node = node;
    previewLayer!.destroyChildren();
    previewLayer!.add(node as Konva.Shape);
    previewLayer!.batchDraw();
  });
```

Replace with:

```ts
  stage.on("mousemove", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
    if (!pos) return;
    if (editorState.tool === "pencil" && drafting.points && drafting.node) {
      const pts = drafting.points;
      const lastX = pts[pts.length - 2];
      const lastY = pts[pts.length - 1];
      const dx = pos.x - lastX;
      const dy = pos.y - lastY;
      // Skip points closer than 2px (squared distance, no sqrt).
      if (dx * dx + dy * dy < 4) return;
      pts.push(pos.x, pos.y);
      (drafting.node as Konva.Line).points(pts);
      previewLayer!.batchDraw();
      return;
    }
    const draft = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (drafting.node) drafting.node.destroy();
    const node = renderPreview(draft);
    if (!node) return;
    drafting.node = node;
    previewLayer!.destroyChildren();
    previewLayer!.add(node as Konva.Shape);
    previewLayer!.batchDraw();
  });
```

- [ ] **Step 4: Add a pencil branch to `mouseup`**

Replace the entire `mouseup` handler. Current body:

```ts
  stage.on("mouseup", () => {
    if (!drafting) return;
    const pos = stage!.getPointerPosition();
    previewLayer!.destroyChildren();
    previewLayer!.batchDraw();
    if (!pos) {
      drafting = null;
      return;
    }
    const final = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (shapeIsTooSmall(final)) {
      drafting = null;
      return;
    }
    final.id = nanoid(10);
    editorState.shapes.push(final);
    commitChange();
    drafting = null;
  });
```

Replace with:

```ts
  stage.on("mouseup", () => {
    if (!drafting) return;
    if (editorState.tool === "pencil" && drafting.points) {
      const pts = drafting.points;
      previewLayer!.destroyChildren();
      previewLayer!.batchDraw();
      if (pts.length < 4) {
        drafting = null;
        return;
      }
      const shape: Shape = {
        id: nanoid(10),
        tool: "pencil",
        color: editorState.color,
        thickness: editorState.thickness,
        geometry: { kind: "pencil", points: pts.slice() },
      };
      editorState.shapes.push(shape);
      commitChange();
      drafting = null;
      return;
    }
    const pos = stage!.getPointerPosition();
    previewLayer!.destroyChildren();
    previewLayer!.batchDraw();
    if (!pos) {
      drafting = null;
      return;
    }
    const final = buildDraftShape(
      drafting.startX,
      drafting.startY,
      pos.x,
      pos.y,
    );
    if (shapeIsTooSmall(final)) {
      drafting = null;
      return;
    }
    final.id = nanoid(10);
    editorState.shapes.push(final);
    commitChange();
    drafting = null;
  });
```

`pts.slice()` is important — we hand the committed shape its own copy so any future mutation of the drafting reference (there shouldn't be any, but safety) can't bleed into the stored shape.

- [ ] **Step 5: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 6: Manual test — drawing works**

Run: `npm run tauri dev`

In the overlay:
1. Trigger a capture (use the configured hotkey).
2. Pick the `✎` pencil tool.
3. Press and drag a curved line. While dragging, a smooth stroke should appear under the cursor.
4. Release. The stroke remains visible.
5. Draw two more curves. All three exist as independent strokes.
6. Switch thickness / color and draw again — each new stroke uses the current settings.
7. Single-click (no drag) — no stroke is created.
8. Press Ctrl+Z three times — strokes disappear one at a time. Ctrl+Y three times — restored.

Stop the dev server when done.

- [ ] **Step 7: Commit**

```bash
git add src/shared/editor/canvas/Stage.vue
git commit -m "feat(editor): implement pencil drawing flow"
```

---

## Task 4: Disable resize on pencil selection

**Why:** `Konva.Transformer` defaults to 8 resize handles. Scaling a pencil stroke would distort the point cloud strangely — easier and more correct to allow translate-only.

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue:154-164` (annLayer click handler)

- [ ] **Step 1: Toggle `resizeEnabled` based on shape kind**

In `src/shared/editor/canvas/Stage.vue`, locate the existing `annLayer.on("click", ...)` handler (lines 154-164). Current body:

```ts
  annLayer.on("click", (e) => {
    if (editorState.tool === "text" || editorState.tool === "mosaic") return;
    const target = e.target;
    if (target === stage || !target.id()) {
      transformer!.nodes([]);
      editorState.selectedId = null;
      return;
    }
    transformer!.nodes([target as Konva.Shape]);
    editorState.selectedId = target.id();
  });
```

Replace with:

```ts
  annLayer.on("click", (e) => {
    if (editorState.tool === "text" || editorState.tool === "mosaic") return;
    const target = e.target;
    if (target === stage || !target.id()) {
      transformer!.nodes([]);
      editorState.selectedId = null;
      return;
    }
    transformer!.nodes([target as Konva.Shape]);
    const selected = editorState.shapes.find((s) => s.id === target.id());
    transformer!.resizeEnabled(selected?.geometry.kind !== "pencil");
    editorState.selectedId = target.id();
  });
```

Setting `resizeEnabled(false)` hides all anchors automatically; the user can still drag the shape itself.

- [ ] **Step 2: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 3: Manual test — selection behaviour**

Run: `npm run tauri dev`. Capture, then:
1. Draw a pencil stroke.
2. Draw a rectangle.
3. Switch to move (`✥`).
4. Click the rectangle → resize handles appear.
5. Click the pencil stroke → no resize handles, but you can drag the stroke.
6. Click the rectangle again → resize handles return.

Stop the dev server when done.

- [ ] **Step 4: Commit**

```bash
git add src/shared/editor/canvas/Stage.vue
git commit -m "feat(editor): disable resize for pencil selection"
```

---

## Task 5: Fold drag offset back into pencil points

**Why:** When a `Konva.Line` with absolute `points` is dragged, `node.x()` / `node.y()` accumulate the drag delta but the `points` themselves stay at their original coordinates. On the next rerender (e.g. after undo/redo) the line would jump back to its original spot. We must absorb the delta into the points and reset the node position to (0, 0). This mirrors how the existing line/arrow branch in the same handler works.

**Files:**
- Modify: `src/shared/editor/canvas/Stage.vue:173-207` (dragend transformend handler)

- [ ] **Step 1: Add pencil branch to dragend handler**

In `src/shared/editor/canvas/Stage.vue`, locate the existing `annLayer.on("dragend transformend", ...)` handler. Current body:

```ts
  annLayer.on("dragend transformend", (e) => {
    const node = e.target;
    const id = node.id();
    const shape = editorState.shapes.find((s) => s.id === id);
    if (!shape) return;
    const g = shape.geometry;
    if (g.kind === "rect" || g.kind === "mosaic" || g.kind === "text") {
      shape.geometry = {
        ...g,
        x: node.x(),
        y: node.y(),
        w: Math.max(1, node.width() * node.scaleX()),
        h: Math.max(1, node.height() * node.scaleY()),
      };
      node.scale({ x: 1, y: 1 });
    } else if (g.kind === "line" || g.kind === "arrow") {
      const pts = (node as Konva.Line).points();
      const newPts = [
        pts[0] + node.x(),
        pts[1] + node.y(),
        pts[2] + node.x(),
        pts[3] + node.y(),
      ];
      shape.geometry = {
        ...g,
        x1: newPts[0],
        y1: newPts[1],
        x2: newPts[2],
        y2: newPts[3],
      };
      (node as Konva.Line).points(newPts);
      node.position({ x: 0, y: 0 });
    }
    commitChange();
  });
```

Replace with:

```ts
  annLayer.on("dragend transformend", (e) => {
    const node = e.target;
    const id = node.id();
    const shape = editorState.shapes.find((s) => s.id === id);
    if (!shape) return;
    const g = shape.geometry;
    if (g.kind === "rect" || g.kind === "mosaic" || g.kind === "text") {
      shape.geometry = {
        ...g,
        x: node.x(),
        y: node.y(),
        w: Math.max(1, node.width() * node.scaleX()),
        h: Math.max(1, node.height() * node.scaleY()),
      };
      node.scale({ x: 1, y: 1 });
    } else if (g.kind === "line" || g.kind === "arrow") {
      const pts = (node as Konva.Line).points();
      const newPts = [
        pts[0] + node.x(),
        pts[1] + node.y(),
        pts[2] + node.x(),
        pts[3] + node.y(),
      ];
      shape.geometry = {
        ...g,
        x1: newPts[0],
        y1: newPts[1],
        x2: newPts[2],
        y2: newPts[3],
      };
      (node as Konva.Line).points(newPts);
      node.position({ x: 0, y: 0 });
    } else if (g.kind === "pencil") {
      const dx = node.x();
      const dy = node.y();
      const newPts = g.points.map((v, i) => v + (i % 2 === 0 ? dx : dy));
      shape.geometry = { kind: "pencil", points: newPts };
      (node as Konva.Line).points(newPts);
      node.position({ x: 0, y: 0 });
    }
    commitChange();
  });
```

The `i % 2 === 0` test picks x coordinates (even indices) vs y coordinates (odd indices) in the flat array.

- [ ] **Step 2: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 3: Manual test — pencil drag persists**

Run: `npm run tauri dev`. Capture, then:
1. Draw a pencil stroke in the upper-left of the capture.
2. Switch to move.
3. Click the stroke → drag it to the lower-right → release.
4. The stroke should stay where you released it (not snap back).
5. Press Ctrl+Z → stroke returns to original upper-left location.
6. Press Ctrl+Y → stroke moves back to lower-right.
7. Save the capture (Save button). Open the saved PNG → stroke is at the dragged location.

Stop the dev server when done.

- [ ] **Step 4: Commit**

```bash
git add src/shared/editor/canvas/Stage.vue
git commit -m "feat(editor): fold pencil drag offset into points"
```

---

## Task 6: Full validation pass

**Why:** Per-task checks confirmed local behaviour. This task runs the 10-item spec checklist plus a production build, end-to-end, to catch interactions between features.

- [ ] **Step 1: Run the 10-item manual checklist**

Run: `npm run tauri dev`. Trigger a capture and confirm each item from `docs/superpowers/specs/2026-05-28-pencil-tool-design.md` §5:

1. Basic stroke — pick pencil, draw a curve, smooth stroke appears.
2. Multiple strokes — draw 3, each is independent (select each individually).
3. Thickness × color — thin/medium/thick × red/orange/yellow/green/blue produce visibly distinct strokes.
4. Tap discard — single click without drag → no shape.
5. Select & drag — covered in Task 5. Re-confirm.
6. Delete — select a stroke, press Delete, it disappears. (Confirm Delete key is wired; if not, this is a pre-existing limitation, not pencil-specific — note it but don't fix in this plan.)
7. Undo / Redo — draw 3 strokes, undo×3 → empty, redo×3 → all back.
8. Pass-through — draw a pencil stroke, switch to line, draw a line crossing the stroke. The line must register (the stroke does not block input).
9. Export — Save. Open the saved PNG. Stroke is present, smoothing matches on-screen.
10. Tool reset — close the capture (finish or cancel). Trigger another capture. The toolbar shows `move` selected, not `pencil`.

If any item fails, stop and address it before continuing. Add a fix-up commit if needed:

```bash
git add <files>
git commit -m "fix(editor): <what was wrong>"
```

- [ ] **Step 2: Edge case — long stroke responsiveness**

In the dev session, draw a single very long, dense pencil stroke (drag continuously in tight back-and-forth motion for ~5 seconds). The stroke should remain responsive (no perceptible lag, no frame drops). The 2px point filter should keep the array bounded.

- [ ] **Step 3: Edge case — minimum-length stroke**

Draw an extremely short stroke (drag ~3-4px and release). Either the stroke appears (≥2 points → 4 numbers) or is discarded (`< 4`). Neither should crash.

- [ ] **Step 4: Production build**

Stop the dev server. Run:

```bash
npx tauri build
```

Expected: build succeeds, produces an installer in `src-tauri/target/release/bundle/`.

**Important:** Use `npx tauri build`, **never** `cargo build --release` alone — the latter silently falls back to the dev URL and ships a broken binary.

- [ ] **Step 5: Smoke-test the production binary**

Run the built `minipaste.exe` from `src-tauri/target/release/`. Before launching, **kill any existing `minipaste.exe` process** (Task Manager or `taskkill /F /IM minipaste.exe`) — otherwise the global hotkey stays registered to the old PID and the new instance's hotkey will silently no-op.

Trigger a capture, draw with pencil, save. Confirm the pencil tool works in the packaged build.

- [ ] **Step 6: No-op commit if all green**

If no fix-up commits were needed during this task, there is nothing to commit. The feature is done — recent commits (Tasks 1-5) cover all changes.

If a CHANGELOG or version bump is required by project convention, do that now in a single commit:

```bash
git add <files>
git commit -m "chore: <version bump or changelog note>"
```

Otherwise, leave the tree clean.

---

## Done

After Task 6, the pencil tool is fully shipped: type-safe, rendered with smoothing, drawable, selectable, draggable, persisting through undo/redo and PNG export, and working in the production build.
