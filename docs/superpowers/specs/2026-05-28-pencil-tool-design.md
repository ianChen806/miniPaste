# Pencil Tool (Freehand Drawing) — Design

**Date:** 2026-05-28
**Status:** Approved
**Scope:** Add a freehand pencil tool to the overlay editor so users can draw arbitrary strokes, not just fixed geometric shapes.

---

## Goal

Add a new `pencil` tool to the editor toolbar that lets the user draw freehand strokes on a capture. Each press-drag-release produces one independent stroke object that obeys the existing color, thickness, undo/redo, selection, and export mechanisms.

## Non-Goals

- Pressure-sensitive (velocity-modulated) stroke width
- Multi-stroke merging (Shift-continue, time-based grouping)
- Vector path representation (SVG `<path>` / Bezier smoothing)
- Eraser tool
- Reshaping individual points after a stroke is committed
- Resize/scale of pencil strokes (only translate is supported)

---

## Approach

Add `pencil` as a first-class `ToolType` alongside `move`, `line`, `rect`, `arrow`, `mosaic`, `text`. Each stroke is stored as a flat point array and rendered via `Konva.Line` with `tension: 0.5` for built-in smoothing. The implementation follows the same pattern as the existing two-point tools (`line`, `arrow`) — just with an array of points instead of a start/end pair.

This was chosen over (a) overloading `line` with a Shift modifier (hidden affordance) and (b) using `Konva.Path` with SVG path strings (YAGNI, inconsistent with current data model).

---

## §1 Data Model

In `src/shared/types.ts`:

```ts
export type ToolType =
  | "move" | "pencil" | "line" | "rect" | "arrow" | "mosaic" | "text";

type ShapeGeometry =
  | { kind: "pencil"; points: number[] }   // flat [x1, y1, x2, y2, ...]
  | { kind: "line";   x1: number; y1: number; x2: number; y2: number }
  | { kind: "rect";   x: number; y: number; w: number; h: number }
  | { kind: "arrow";  x1: number; y1: number; x2: number; y2: number }
  | { kind: "mosaic"; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: "text";   x: number; y: number; w: number; h: number };
```

**Rationale**

- Flat `number[]` is Konva's native `points` format — no conversion needed at render time.
- No bounding box stored; Konva computes client rect on demand for selection/drag.

---

## §2 Drawing Flow (`Stage.vue`)

Pencil takes a dedicated branch in `mousedown / mousemove / mouseup`.

### mousedown (when `editorState.tool === "pencil"`)

- Create a `Konva.Line` with:
  - `points: [x, y]`
  - `tension: 0.5`
  - `lineCap: "round"`
  - `lineJoin: "round"`
  - `stroke`, `strokeWidth` from current `editorState.color` / `thickness`
- Add it to `previewLayer`.
- Store `drafting = { startX, startY, node, points: [x, y] }`. The `points` field extends the existing drafting struct; other tools ignore it.

### mousemove

- If the new position is **closer than 2px** to the last point (squared-distance compare, no sqrt), drop it.
- Otherwise push `[x, y]` into `drafting.points` and call `drafting.node.points(drafting.points)`.
- Do not destroy/recreate the node — mutating `points` in place is enough for Konva to redraw.

### mouseup

- If `drafting.points.length < 4` (fewer than 2 actual points) → discard the preview node, do not create a Shape. Mirrors the existing `min 3px` discard rule for line/rect.
- Otherwise:
  - Build a `Shape` with `geometry: { kind: "pencil", points: drafting.points }`.
  - Push into `editorState.shapes`, call `commitChange()`.
  - Remove the preview node from `previewLayer`.

### Render (`drawTools.ts` — `renderShape`)

Add a case to the geometry switch:

```ts
case "pencil":
  return new Konva.Line({
    points: geometry.points,
    tension: 0.5,
    lineCap: "round",
    lineJoin: "round",
    stroke: COLOR_HEX[shape.color],
    strokeWidth: STROKE_WIDTH[shape.thickness],
    hitStrokeWidth: Math.max(STROKE_WIDTH[shape.thickness], 10),
  });
```

Do **not** set `closed: true` — that would auto-connect last point to first.

---

## §3 Selection / Editing / Export

### Selection (move tool)

- Layer-level `annLayer.listening(true)` already exposes pencil shapes to pointer events.
- When a pencil shape is selected, the `Konva.Transformer` must disable resize handles:
  - `transformer.enabledAnchors([])` and `transformer.resizeEnabled(false)` if `shape.geometry.kind === "pencil"`.
  - Only translation (drag) is allowed.

### Hit testing

- A 2px line is very hard to click. Use `hitStrokeWidth: Math.max(strokeWidth, 10)` so hit area is always ≥10px while the rendered stroke stays at its real width. This is a built-in Konva property.

### Drag end (translate-into-geometry)

On `dragend`:

- Read `node.x()` / `node.y()` (the translation delta).
- Apply the delta to every point in `geometry.points` (i.e. `points[i] += dx`, `points[i+1] += dy`).
- Reset `node.position({ x: 0, y: 0 })`.
- Commit to history.

This mirrors how `line` / `arrow` fold the drag offset back into their endpoint coordinates so position and data never drift apart.

### Undo / Redo

No changes. The existing `commitChange` snapshots the entire `editorState.shapes` array, so pencil shapes are covered for free.

### Export (PNG)

No changes. `stage.toDataURL()` rasterizes all layers including the pencil's `Konva.Line`.

---

## §4 UI / Toolbar

`src/shared/editor/ui/Toolbar.vue`:

```ts
const tools = [
  { key: "move",   label: "✥" },
  { key: "pencil", label: "✎" },   // U+270E LOWER RIGHT PENCIL
  { key: "line",   label: "／" },
  { key: "rect",   label: "▭" },
  { key: "arrow",  label: "↗" },
  { key: "mosaic", label: "▦" },
  { key: "text",   label: "T" },
];
```

- Icon `✎` matches the emoji-style visual language of existing buttons — no icon font needed.
- Color and thickness selectors apply to pencil unchanged — no conditional rendering.
- No keyboard shortcut (none of the existing tools have shortcuts; stay consistent).

### Helper updates

- `DRAW_TOOLS` array in `src/shared/editor/canvas/Stage.vue:33` must include `"pencil"` so `isDrawTool()` recognizes it.
- `resetEditor()` already sets `editorState.tool = "move"`, which covers pencil — no change.
- `renderShape` in `drawTools.ts` has a runtime-throwing `default` case (not a compile-time exhaustive check), so adding pencil to types alone will not fail compilation; the pencil render case must be added in the same task as the type to avoid runtime errors on rerender.

---

## §5 Validation

The project has no automated test framework currently. Verification is via compile checks + manual testing.

### Compile

- `npm run build` (includes `vue-tsc`) — any unhandled `pencil` branch in an exhaustive switch is a compile error.
- `npx tauri build` (per the memory rule: never `cargo build` alone — silently falls back to dev URL).

### Manual checklist

1. **Basic stroke** — capture → pick pencil → draw a curve → release → smooth stroke appears.
2. **Multiple strokes** — draw 3 strokes; each is an independent shape.
3. **Thickness / color matrix** — try `thin`/`medium`/`thick` × 5 colors.
4. **Tap discard** — single click without drag → no shape created.
5. **Select & drag** — switch to move → click stroke → drag → release → stroke follows, position correctly folded into geometry (single undo restores original spot).
6. **Delete** — select stroke → Delete key → gone.
7. **Undo / Redo** — draw 3 strokes → undo×3 to empty → redo×3 to restore all.
8. **Pass-through** — draw pencil, then switch to line and draw on top of the stroke; the line goes through without being blocked.
9. **Export** — Save → exported PNG contains the pencil stroke with smoothing matching on-screen.
10. **Tool reset** — finish a capture → next capture opens with `move` selected (not stuck on pencil).

### Edge cases

- Long stroke (>1000 points) — confirm mousemove stays responsive.
- Minimum-length stroke (exactly 4 floats / 2 points) — does not crash.

---

## Files Touched

| File | Change |
|------|--------|
| `src/shared/types.ts` | Add `"pencil"` to `ToolType`; add `{ kind: "pencil"; points: number[] }` to `ShapeGeometry` |
| `src/shared/editor/canvas/drawTools.ts` | Add `pencil` case in `renderShape` (with `hitStrokeWidth`) |
| `src/shared/editor/canvas/Stage.vue` | Add `"pencil"` to `DRAW_TOOLS`; pencil branch in `mousedown`/`mousemove`/`mouseup`; transformer resize-disable on pencil select; dragend translate-into-points |
| `src/shared/editor/ui/Toolbar.vue` | Insert pencil button between `move` and `line` |

No new files. No new dependencies.
