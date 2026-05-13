# Inline Capture (Snipaste-style) — Design

**Date:** 2026-05-13
**Status:** Design approved by user, pending implementation plan

## Summary

Convert MiniPaste's capture flow from a two-window pattern (overlay for framing → separate editor window for annotation) into a Snipaste-style single-overlay flow: framing, annotation, and terminal actions all happen on the same fullscreen overlay window. The selection rectangle is mutable (8 resize handles + drag-move). A magnifier loupe assists pixel-precise framing. A floating toolbar follows the selection. Annotations are clipped to the selection and rescale with it. A new `PinFromOverlay` terminal action reuses the existing pin service. The `editor` Tauri window is retired.

## Goals

- Match Snipaste's "frame → edit in place → terminal action" UX on the same overlay.
- Reuse existing Konva-based editing components by moving them into `src/shared/editor/`.
- Keep the post-flicker-fix overlay parking pattern intact (no hide/show on each capture).
- Wire a new `PinFromOverlay` terminal action that bridges capture → existing pin service.
- Phase machine stays a single source of truth: add `Editing` state + `ReframeRequest` event.

## Non-Goals

- Keeping the legacy `editor` Tauri window as a fallback or "advanced" surface (full retirement).
- Adding free-draw pen/highlighter tools (existing 5 drawing tools only; YAGNI).
- Numeric width/height micro-adjustment input on the toolbar (mouse handles are precise enough with the magnifier).
- Right-click context menu (toolbar already exposes all actions).
- Color sampling / eyedropper from the magnifier (out of scope for this iteration).
- E2E automated tests (Tauri webview automation is immature; covered by manual checklist).

## User Preferences (resolved in brainstorming)

| Decision | Choice |
|---|---|
| Component location | Shared (`src/shared/editor/`) so future pin/etc can reuse |
| Selection after initial drag | Mutable: drag to move + 8 handles to resize |
| Toolbar position | Follows selection, flips above when below is tight, hugs inside when overlay is too short; updates in real time while dragging |
| Magnifier loupe | Shown during initial drag and during handle drag; off during free annotation; 5× zoom, 120×120, offset away from cursor |
| Drawing tool set | Existing 5: line / rect / arrow / mosaic / text (no new pen) |
| Terminal actions | Existing Copy / Save / Save+Copy plus **new Pin** |
| Annotation coordinate system | Selection-relative; rescale on resize; stroke width stays constant |
| Esc / right-click | Cancel entire capture |
| Left-click outside selection | Reframe (clear annotations, clear selection, return to framing) |
| Double-click inside selection / Enter | Default terminal action = Copy |
| Editor window | Fully retired |

## Architecture

### Flow comparison

**Before:**
```
hotkey → overlay (framing) → selection_confirmed
                                  ↓
                              editor window (toolbar + canvas + actionbar)
                                  ↓
                              finish_action → both windows hide
```

**After:**
```
hotkey → overlay (framing → editing → terminal action, all in one)
                                  ↓
                              finish_action → overlay parks at (-32000, -32000)
```

### Overlay UI state machine (frontend)

```
        ┌─────────────┐
        │  Idle       │ overlay parked at (-32000, -32000)
        └──────┬──────┘
               │ capture-ready event
               ▼
        ┌─────────────┐
        │  Framing    │ drag-to-frame + magnifier
        └──────┬──────┘
               │ mouseup with w*h >= 5*5
               ▼
        ┌─────────────┐
        │  Editing    │ 8 handles + floating toolbar + canvas
        └──────┬──────┘
               │ finish_action / Esc / left-click outside
               ▼
        Idle (parked again)
```

Transitions out of `Editing`:
- Left-click outside selection → back to `Framing` (clears annotations + selection)
- Esc / right-click → back to `Idle` (cancels entire capture)
- Toolbar action (Copy/Save/Save+Copy/Pin) → back to `Idle` after backend completion

### Backend phase machine

`state::AppPhase` already has the three variants we need (`Idle`, `Capturing`, `Editing`) from the previous editor-window era. Only one **new event** is added:

```
Idle → Capturing → Editing → Idle
        │ (HotkeyPressed)        ▲
        │                        │ (ActionFinished)
        │ (SelectionConfirmed)   │
        ▼                        │
        Editing ─────────────────┘
            │
            └─ ReframeRequest → Capturing   (NEW event)

Any active phase → Cancelled → Idle  (existing, unchanged)
```

Semantic shift on `Editing`: previously meant "editor window is showing", now means "overlay is in editing mode". Same state, same transitions in/out, just different UI surface. `ReframeRequest` does **not** clear `state.capture` (the background frame is still valid); the overlay frontend clears its own annotation state.

### Frontend module layout

```
src/shared/editor/                  NEW (moved from src/windows/editor/)
├── canvas/
│   ├── Stage.vue                   Konva stage, re-parameterized (see Components)
│   ├── drawTools.ts                Per-tool pointer handlers
│   └── textTool.ts                 Text input overlay logic
├── state/
│   ├── shapes.ts                   editorState reactive, undo/redo, rescaleShapes
│   └── history.ts                  Undo/redo stack
└── ui/
    ├── Toolbar.vue                 Drawing tools + colors + thickness + undo/redo
    └── ActionBar.vue               Copy / Save / Save+Copy / Pin (new)

src/windows/overlay/
├── App.vue                          Re-written: two-phase UI + dispatcher
├── selection.ts                     Existing rect math (kept)
├── handles.ts                       NEW: 8-handle hit-test + resize math
├── magnifier.ts                     NEW: canvas-based loupe renderer
├── toolbarPlacement.ts              NEW: below/above/inside placement decision
└── overlay.css

src/windows/editor/                  DELETED
editor.html                          DELETED
vite.config.ts (editor entry)        DELETED
```

## Components

### `src/windows/overlay/App.vue` (re-written, ~300 lines)

Owns the overlay UI state machine and dispatches events into shared editor state.

Reactive state shape:

```ts
const state = reactive({
  phase: 'idle' | 'framing' | 'editing',
  bgUrl: '',
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },

  // framing
  dragStart: Point | null,
  dragEnd: Point | null,

  // editing
  selection: Rect | null,
  cursor: Point | null,         // for magnifier during handle drag
  activeHandle: HandleId | null,
});
```

Event subscriptions:
- `capture-ready` → set bg, switch to `framing`
- `capture-clear` → reset everything, back to `idle`

### `src/windows/overlay/handles.ts` (NEW, ~150 lines)

Pure functions, fully testable without DOM.

```ts
export type HandleId = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'move';

export function hitTestHandle(rect: Rect, pt: Point): HandleId | null;
export function cursorForHandle(h: HandleId): string;        // CSS cursor value
export function resizeRect(
  rect: Rect,
  handle: HandleId,
  delta: Point,
  minSize?: number,                                          // default 10
): Rect;
```

`hitTestHandle` returns the handle ID for a 12×12 hit area around each handle point, `'move'` for any point inside the rect not on a handle, or `null` for outside.

### `src/windows/overlay/magnifier.ts` (NEW, ~120 lines)

```ts
export function renderMagnifier(
  ctx: CanvasRenderingContext2D,   // 120×120 destination
  source: HTMLImageElement,         // overlay background
  cursor: Point,                    // overlay-local coords
  zoom?: number,                    // default 5
): void;
```

Crops a `(zoom_window / zoom)`-sized window from `source` around `cursor`, draws it at 5× zoom into `ctx`, overlays a crosshair, and renders coordinate text at the bottom. Placement (which corner the loupe sits in relative to the cursor) is decided by `App.vue` and applied via CSS transform.

### `src/windows/overlay/toolbarPlacement.ts` (NEW, ~80 lines)

```ts
export interface ToolbarPlacement {
  x: number;
  y: number;
  orientation: 'below' | 'above' | 'inside';
}

export function placeToolbar(
  selection: Rect,
  toolbarSize: { w: number; h: number },
  overlaySize: { w: number; h: number },
  gap?: number,                                              // default 8
): ToolbarPlacement;
```

Decision tree:
1. If `selection.y + selection.h + gap + toolbar.h <= overlaySize.h` → `below`
2. Else if `selection.y - gap - toolbar.h >= 0` → `above`
3. Else → `inside` (toolbar overlaid on selection's bottom edge, with semi-transparent background)

Horizontal x is clamped so toolbar stays inside overlay bounds.

### `src/shared/editor/canvas/Stage.vue` (moved + re-parameterized)

**Old props:** `imageUrl, width, height`
**New props:** `selection: Rect, overlaySize: { w, h }`

Background image is provided by the overlay (already loaded into `<img>` for magnifier reuse); Stage no longer fetches it. Stage absolutely positions itself at `left = selection.x, top = selection.y`, with `width = selection.w, height = selection.h`, and `overflow: hidden`.

Shape coordinates are stored relative to Stage's `(0, 0)` (i.e. selection-local).

When the `selection` prop changes, Stage distinguishes two cases:
- **Move only** (`x`/`y` change, `w`/`h` unchanged): no shape mutation needed — Stage repositions itself absolutely; shapes move with it for free because their coordinates are selection-local.
- **Resize** (`w` or `h` change): Stage calls `rescaleShapes(prevRect, newRect)` to scale all shape coordinates proportionally. Stroke widths are not scaled.

Both cases happen live during handle/body drag, not on drag-end, so the UX matches Snipaste.

### `src/shared/editor/state/shapes.ts` (moved + extended)

Adds `rescaleShapes(oldRect: Rect, newRect: Rect): void` that mutates `editorState.shapes` in place (already reactive). Coordinates scale by `newRect.w / oldRect.w` and `newRect.h / oldRect.h`; stroke widths are untouched.

### `src/shared/editor/ui/ActionBar.vue` (moved + extended)

Adds a "Pin" button. Behavior:

```ts
async function pinIt() {
  const bytes = Array.from(await exportPng());
  await call("finish_action", {
    action: { kind: "PinFromOverlay" },
    imageBytes: bytes,
  });
}
```

Same `finish_action` channel as existing actions, just a new `FinishAction` variant.

### Rust: `src-tauri/src/ipc/commands.rs`

**`FinishAction` gains a variant:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FinishAction {
    CopyImage,
    Save { path: PathBuf },
    SaveAndCopyPath,
    PinFromOverlay,                                          // NEW
}
```

`finish_action` adds a match arm:

```rust
FinishAction::PinFromOverlay => {
    crate::pin::service::spawn_from_bytes(&app, &state, &image_bytes)?;
    finalize(&app, &state, FinishOutcome { saved_path: None })
}
```

**`selection_confirmed`** is simplified: drop the "show editor window + emit editor-ready" block **and** the server-side `PlatformCapture::crop` call. In the new flow the Konva stage on overlay exports the final PNG via `stage.toDataURL`, so the server no longer needs to materialize a cropped buffer. `state.cropped` is no longer written or read — leave the field in `AppState` for now (zero cost; can be removed in a follow-up cleanup). The command transitions phase to `Editing` and emits nothing — the frontend already holds the rect locally.

**New `reframe_request` command:**

```rust
#[tauri::command]
pub fn reframe_request(state: State<AppState>) -> Result<(), AppError> {
    state.phase.lock().unwrap()
        .transition(PhaseEvent::ReframeRequest)
        .map_err(|e| AppError::State(e.to_string()))?;
    Ok(())
}
```

(No `state.cropped` clearing needed — the field is no longer used; see `selection_confirmed` note above.)

### Rust: `src-tauri/src/state.rs`

Adds `PhaseEvent::ReframeRequest` only (no new `AppPhase` variant — `Editing` already exists). Transition table gains one row:

| From | Event | To |
|---|---|---|
| `Editing` | `ReframeRequest` | `Capturing` |

Existing rows unchanged:

| From | Event | To |
|---|---|---|
| `Idle` | `HotkeyPressed` | `Capturing` |
| `Capturing` | `SelectionConfirmed` | `Editing` |
| `Editing` | `ActionFinished` | `Idle` |
| `Capturing` | `Cancelled` | `Idle` |
| `Editing` | `Cancelled` | `Idle` |

### Rust: `src-tauri/src/pin/service.rs`

Refactor: extract the internal spawn body (currently called only from `paste_from_clipboard`) into `spawn_from_bytes(app, state, png_bytes) -> Result<(), AppError>`. Both `paste_from_clipboard` and the new `finish_action` `PinFromOverlay` arm call this shared function.

### `tauri.conf.json`

Remove the `editor` window declaration. The `overlay` and `settings` windows are unchanged.

### `vite.config.ts`

Remove the `editor` rollup input entry.

### Capabilities

`src-tauri/capabilities/default.json` no longer needs to grant permissions to the `editor` window (it was already broad — review and trim if any permission was editor-only). The `pin-*` capabilities (added in the floating-pin work) are unchanged.

## Data Flow

### Happy path

```
[1] User presses capture hotkey
[2] tray://trigger-capture → capture::trigger::trigger_capture(app)
[3] PlatformCapture::virtual_desktop() captures frame
    state.capture = Some(frame)
[4] overlay.set_position(frame.origin) + emit("capture-ready", payload)
[5] overlay App.vue: state.phase = 'framing'
[6] User drags rect, mouseup
[7] call("selection_confirmed", { rect })
    Backend: phase → Editing (no crop computed)
[8] overlay App.vue: state.phase = 'editing', selection = rect
    Stage + Toolbar + ActionBar + 8 handles render
[9] User edits / drags / resizes selection (zero round-trips)
[10] User clicks Copy/Save/Save+Copy/Pin
     Frontend exports PNG via stage.toDataURL()
[11] call("finish_action", { action, imageBytes })
     Backend: executes action, phase ActionFinished → Idle
     overlay.set_position(-32000, -32000)
     emit("capture-clear", ())
[12] overlay App.vue: state.phase = 'idle', state reset
```

### Reframe path

```
[E1] User left-clicks outside selection in editing phase
[E2] App.vue: hitTestHandle returns null; cursor is outside selection rect
[E3] call("reframe_request")
     Backend: phase Editing → Capturing
[E4] App.vue: state.phase = 'framing', selection = null, shapes = []
     Awaits next mousedown
```

### Cancel path

```
[C1] User presses Esc or right-clicks
[C2] call("selection_cancelled")
     Backend: phase * → Idle, state.capture cleared
     overlay.set_position(-32000, -32000)
     emit("capture-clear", ())
[C3] App.vue: state reset, phase 'idle'
```

### Default action path (double-click / Enter)

```
[D1] User double-clicks inside selection or presses Enter in editing phase
[D2] Same as [10]: dispatch finish_action with kind = CopyImage
```

## IPC contract

### Commands (frontend → backend)

| Command | Payload | Phase transition |
|---|---|---|
| `selection_confirmed` | `{ rect: Rect }` | `Capturing → Editing` |
| `selection_cancelled` | (none) | `* → Idle` |
| `reframe_request` *(NEW)* | (none) | `Editing → Capturing` |
| `finish_action` | `{ action: FinishAction, imageBytes: number[] }` | `Editing → Idle` |

### Events (backend → frontend)

| Event | Payload | Trigger |
|---|---|---|
| `capture-ready` | `{ thumbnail_b64, width, height, origin_x, origin_y, screens }` | Capture done, overlay repositioned |
| `capture-clear` | (none) | Overlay about to park; frontend resets |
| `action-complete` | `{ saved_path: string \| null }` | `finish_action` succeeded |
| `capture-error` | `string` | Capture pipeline failure |
| `hotkey-conflict` | `{ kind, attempted, reason }` | Hotkey registration failure |

### Shared types (`src/shared/types.ts`)

```ts
export type FinishAction =
  | { kind: 'CopyImage' }
  | { kind: 'Save'; path: string }
  | { kind: 'SaveAndCopyPath' }
  | { kind: 'PinFromOverlay' };

export interface Rect { x: number; y: number; w: number; h: number }
```

## Error Handling

### Selection phase

| Scenario | Handling |
|---|---|
| Selection w or h < 5px | Frontend silently drops on mouseup; stays in framing |
| Selection past overlay bounds | `clampToBounds` (existing) |
| `selection_confirmed` arrives while phase is not `Capturing` | `phase.transition` errors → `AppError::State`; frontend toast, reset to idle |

### Editing phase

| Scenario | Handling |
|---|---|
| Resize collapses selection below minSize (10×10) | `handles.ts::resizeRect` clamps |
| Drag pushes selection outside overlay | Clamp so at least 1px stays inside |
| `placeToolbar` cannot fit toolbar below or above | Falls back to `inside` (semi-transparent background) |
| Konva stage render error | Catch + `pushToast('error', ...)`; do not tear down overlay |

### Finish-action phase

| Scenario | Handling |
|---|---|
| `stage.toDataURL()` throws | Frontend toast "匯出失敗"; stay in editing |
| `finish_action` backend I/O error (`AppError::Fs`/`Clipboard`/`Capture`) | Toast the message; **stay in editing** (transient failures are retry-friendly; user can Esc to bail) |
| `PinFromOverlay` but pin registry at 30 cap | `spawn_from_bytes` errors; toast "Pin 數量已達上限 (30)"; stay in editing |
| `Save` to unwritable path | `validate_writable_dir` error; toast; stay in editing |

### Phase-machine race conditions

| Scenario | Handling |
|---|---|
| `reframe_request` fires while phase is already Idle (double-click race) | `phase.transition` returns Err; backend logs warn, returns `AppError::State`; frontend silently swallows (state already matches the user's intent) |
| `finish_action` fires while phase is not Editing | Same: `AppError::State`; frontend toast and stay in current UI state |

### Out of scope

| Scenario | Why not handled |
|---|---|
| Multi-monitor DPI mismatch affecting magnifier coords | Capture pipeline already normalizes via virtual desktop coords; magnifier uses overlay-local coords (DPI-independent) |
| Konva slowdown on 4K-fullscreen captures | Rare workflow; if it becomes a problem, that is a Konva-level fix, not architectural |
| Concurrent capture hotkey presses | Phase machine rejects second `Capturing` while not in Idle |

## Testing

### Rust unit tests (`src-tauri/`)

| Module | What we test |
|---|---|
| `state::phase` | New `ReframeRequest` event: `Editing → Capturing` passes; `Idle → ReframeRequest` and `Capturing → ReframeRequest` error (existing `Editing → Idle` transitions are already covered) |
| `pin::service::spawn_from_bytes` | Both `paste_from_clipboard` and `finish_action(PinFromOverlay)` succeed via shared path (mock app handle); registry-at-cap rejects from both entries |
| `ipc::commands::finish_action` | New `PinFromOverlay` arm: given image_bytes, `state.cropped` cleared, phase `ActionFinished` |
| `ipc::commands::reframe_request` | In Editing → `Capturing`; in other phases → `AppError::State` |

### Frontend unit tests (Vitest, `src/__tests__/`)

| File | What we test |
|---|---|
| `handles.test.ts` *(NEW)* | `hitTestHandle` matrix (each handle, inside-is-move, outside-is-null); `resizeRect` per handle × delta with minSize clamp; `cursorForHandle` for all 8 + move |
| `magnifier.test.ts` *(NEW)* | `renderMagnifier` does not throw on valid inputs; no pixel-level golden comparison |
| `toolbarPlacement.test.ts` *(NEW)* | Below when fits; above when below tight; inside fallback; horizontal clamp at right/left edges |
| `shapes.test.ts` *(UPDATE)* | `rescaleShapes`: coordinates scale proportionally; stroke widths unchanged |
| `selection.test.ts` *(existing)* | Unchanged |

### Manual test checklist

Appended to `docs/manual-test-checklist.md` under a new "Inline Capture (Snipaste-style)" section:

```markdown
## Inline Capture (Snipaste-style)

### Framing
- [ ] capture hotkey → overlay shows, drag-to-frame works
- [ ] Magnifier follows cursor, offsets away from it, shows correct coords
- [ ] Drag < 5px → no transition, can re-drag
- [ ] Esc → cancels back to idle

### Editing
- [ ] mouseup shows 8 handles + toolbar below selection
- [ ] Toolbar flips above when below is tight
- [ ] Toolbar falls back to inside when overlay is shorter than expected
- [ ] Resizing via handle: shapes scale proportionally, stroke width stays constant
- [ ] Magnifier appears during handle drag
- [ ] Dragging selection body moves selection + shapes together
- [ ] Left-click outside selection → shapes cleared, returns to framing
- [ ] Double-click inside selection → default action (Copy) + exit
- [ ] Enter → default action (Copy) + exit
- [ ] Esc / right-click → cancels everything

### Finish actions
- [ ] Copy → clipboard has image, overlay exits
- [ ] Save → dialog, path picked, file written, toast
- [ ] Save+Copy → default path used, clipboard FileList set, toast
- [ ] **Pin (NEW)** → pin window spawned with edited image, overlay exits
- [ ] Any action failure → toast shown, stays in editing

### Multi-monitor
- [ ] Primary + secondary monitor both captureable
- [ ] Cross-monitor selection, magnifier, toolbar placement all correct
```

### Not tested

| Area | Reason |
|---|---|
| Konva golden image diff | Font anti-aliasing varies across platforms/Konva versions; high maintenance |
| End-to-end automation (Playwright on Tauri) | Tauri webview automation immature; manual checklist suffices |
| Magnifier pixel-perfect comparison | DPI + font hinting noise; non-throw check is enough |

## Migration / Cleanup Steps

1. Move `src/windows/editor/` → `src/shared/editor/`, fix imports.
2. Re-parameterize `Stage.vue` from `imageUrl/width/height` → `selection/overlaySize`.
3. Add `handles.ts`, `magnifier.ts`, `toolbarPlacement.ts` under `src/windows/overlay/`.
4. Rewrite `src/windows/overlay/App.vue` with the two-phase machine.
5. Add `Phase::Editing` + `PhaseEvent::ReframeRequest`; update transition table; add tests.
6. Add `reframe_request` command + register in `tauri::generate_handler!`.
7. Add `FinishAction::PinFromOverlay` + match arm in `finish_action`.
8. Extract `pin::service::spawn_from_bytes`; refactor `paste_from_clipboard` to call it.
9. Remove `editor` window from `tauri.conf.json` and `vite.config.ts`.
10. Delete `editor.html`, `src/windows/editor/`.
11. Trim editor-only capabilities from `default.json` (review only).
12. Append manual checklist section.

## Open Questions

None — all design questions resolved in brainstorming.
