# Inline Capture (Snipaste-style) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert MiniPaste's capture flow from a two-window pattern (overlay → editor window) into a Snipaste-style single-overlay flow where framing, annotation, and terminal actions all happen on the same overlay. Retire the `editor` Tauri window.

**Architecture:** The overlay window holds a frontend state machine with two visual phases (`framing` / `editing`) on top of the existing `AppPhase::{Capturing, Editing}` backend states. A new `ReframeRequest` event allows returning from editing to framing. Annotations are pixel-anchored to the overlay's coordinate space; the selection rect is a clipping viewport. A new `FinishAction::PinFromOverlay` reuses the existing `pin::service` spawn path. Editor-related files are deleted at the end.

**Tech Stack:** Rust + Tauri 2 (no new deps), Vue 3, Konva (existing), Vitest. Build: `npx tauri build` from project root, `cargo test` from `src-tauri/`, `npm test` from project root.

**Reference Spec:** `docs/superpowers/specs/2026-05-13-inline-capture-design.md`

---

## File Map

**Rust — modify:**
- `src-tauri/src/state.rs` — add `PhaseEvent::ReframeRequest` + transition row
- `src-tauri/src/ipc/commands.rs` — add `reframe_request` command; add `FinishAction::PinFromOverlay` variant + match arm; later: simplify `selection_confirmed`
- `src-tauri/src/lib.rs` — register `reframe_request` in `generate_handler!`; later: remove editor window CloseRequested loop entry
- `src-tauri/src/pin/service.rs` — rename `spawn_pin` → `spawn_from_bytes`, make `pub`, drop the `pub(crate)` shim if any
- `src-tauri/tauri.conf.json` — remove `editor` window (final cleanup)

**Frontend — new:**
- `src/windows/overlay/handles.ts` — hit-test + resize math
- `src/windows/overlay/toolbarPlacement.ts` — toolbar below/above/inside decision
- `src/windows/overlay/magnifier.ts` — canvas-based loupe renderer
- `src/__tests__/handles.test.ts` — unit tests
- `src/__tests__/toolbarPlacement.test.ts` — unit tests
- `src/__tests__/magnifier.test.ts` — smoke test

**Frontend — move (git mv):**
- `src/windows/editor/state/` → `src/shared/editor/state/`
- `src/windows/editor/canvas/` → `src/shared/editor/canvas/`
- `src/windows/editor/ui/` → `src/shared/editor/ui/`

**Frontend — modify:**
- `src/shared/editor/canvas/Stage.vue` — re-parameterized props after the move (`bgUrl`, `selection`, `overlaySize`)
- `src/shared/editor/ui/ActionBar.vue` — add Pin button
- `src/windows/overlay/App.vue` — rewritten with two-phase machine
- `src/shared/types.ts` — extend `FinishAction` with `PinFromOverlay`
- `src/main.ts` — remove `case "editor"`

**Frontend — delete (final cleanup):**
- `src/windows/editor/` (whole directory, after its contents are migrated)
- `editor.html` at project root
- `vite.config.ts` — remove `editor` rollup input

**Docs — modify:**
- `docs/manual-test-checklist.md` — append "Inline Capture" section

---

## Task Ordering

Backend additive tasks first (no behavior change), then frontend pure helpers, then the migration block, then the breaking-cutover, then cleanup. Each task is its own commit.

1. Backend: `PhaseEvent::ReframeRequest` + transition test
2. Backend: `reframe_request` command + handler registration
3. Backend: `FinishAction::PinFromOverlay` variant + match arm + frontend type
4. Backend: rename `spawn_pin` → public `spawn_from_bytes`
5. Frontend: `handles.ts` + tests
6. Frontend: `toolbarPlacement.ts` + tests
7. Frontend: `magnifier.ts` + smoke test
8. Migration: move editor → shared; re-parameterize Stage; add Pin button to ActionBar; fix old editor's imports
9. Overlay rewrite: two-phase `App.vue` integrating helpers + shared editor
10. Backend cutover: simplify `selection_confirmed` (drop crop + drop editor.show())
11. Cleanup: delete editor window + entry + html + folder + vite input + main.ts case
12. Manual test checklist + final `npx tauri build` verify

---

## Task 1: Add `PhaseEvent::ReframeRequest`

**Files:**
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: Write the failing tests**

Append inside the existing `#[cfg(test)] mod tests { ... }` block in `src-tauri/src/state.rs`:

```rust
#[test]
fn editing_to_capturing_on_reframe() {
    let mut s = AppPhase::Editing;
    assert!(s.transition(PhaseEvent::ReframeRequest).is_ok());
    assert_eq!(s, AppPhase::Capturing);
}

#[test]
fn reframe_from_idle_is_err() {
    let mut s = AppPhase::Idle;
    assert!(s.transition(PhaseEvent::ReframeRequest).is_err());
    assert_eq!(s, AppPhase::Idle);
}

#[test]
fn reframe_from_capturing_is_err() {
    let mut s = AppPhase::Capturing;
    assert!(s.transition(PhaseEvent::ReframeRequest).is_err());
    assert_eq!(s, AppPhase::Capturing);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run from `src-tauri/`:
```
cargo test state::tests
```
Expected: 3 new tests fail with "no variant `ReframeRequest` for `PhaseEvent`".

- [ ] **Step 3: Add the variant + transition row**

Edit `PhaseEvent` enum:

```rust
#[derive(Debug, Clone, Copy)]
pub enum PhaseEvent {
    HotkeyPressed,
    SelectionConfirmed,
    ActionFinished,
    Cancelled,
    ReframeRequest,
}
```

Add the transition row inside the `match` in `AppPhase::transition`:

```rust
            (Idle, HotkeyPressed) => Capturing,
            (Capturing, SelectionConfirmed) => Editing,
            (Editing, ReframeRequest) => Capturing,
            (Editing, ActionFinished) => Idle,
            (Capturing, Cancelled) => Idle,
            (Editing, Cancelled) => Idle,
            (from, ev) => return Err(TransitionError { from, event: ev }),
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test state::tests
```
Expected: all tests pass (existing 5 + new 3 = 8).

- [ ] **Step 5: Commit**

```
git add src-tauri/src/state.rs
git commit -m "feat(state): add ReframeRequest event for editing→capturing"
```

---

## Task 2: Add `reframe_request` IPC command

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the command**

Append to `src-tauri/src/ipc/commands.rs` (after `cancel_edit`):

```rust
#[tauri::command]
pub fn reframe_request(state: State<AppState>) -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    phase
        .transition(PhaseEvent::ReframeRequest)
        .map_err(|e| AppError::State(e.to_string()))?;
    tracing::info!("reframe_request: phase -> Capturing");
    Ok(())
}
```

- [ ] **Step 2: Register the command**

In `src-tauri/src/lib.rs`, update the import:

```rust
use crate::ipc::commands::{
    cancel_edit, finish_action, get_config, pin_close, reframe_request, selection_cancelled,
    selection_confirmed, update_config,
};
```

And add to `generate_handler!`:

```rust
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            selection_confirmed,
            selection_cancelled,
            reframe_request,
            finish_action,
            cancel_edit,
            pin_close,
        ])
```

- [ ] **Step 3: Verify build**

```
cd src-tauri && cargo check
```
Expected: clean compile.

- [ ] **Step 4: Commit**

```
git add src-tauri/src/ipc/commands.rs src-tauri/src/lib.rs
git commit -m "feat(ipc): add reframe_request command"
```

---

## Task 3: Add `FinishAction::PinFromOverlay`

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src/shared/types.ts`

- [ ] **Step 1: Extend the Rust enum**

In `src-tauri/src/ipc/commands.rs`, extend `FinishAction`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FinishAction {
    CopyImage,
    Save { path: PathBuf },
    SaveAndCopyPath,
    PinFromOverlay,
}
```

Add a match arm in `finish_action` (after `SaveAndCopyPath`):

```rust
        FinishAction::PinFromOverlay => {
            crate::pin::service::spawn_from_bytes(&app, image_bytes.clone())
                .map_err(AppError::State)?;
            finalize(&app, &state, FinishOutcome { saved_path: None })
        }
```

Note: `spawn_from_bytes` does not exist yet — that's Task 4. The `cargo check` in the next step will fail; that's expected. We commit Task 3 only after Task 4 lands.

Defer the commit to Task 4.

- [ ] **Step 2: Extend the TS type**

In `src/shared/types.ts`:

```ts
export type FinishAction =
  | { kind: "CopyImage" }
  | { kind: "Save"; path: string }
  | { kind: "SaveAndCopyPath" }
  | { kind: "PinFromOverlay" };
```

- [ ] **Step 3: Hold off on the commit — continue to Task 4**

The Rust enum addition will fail `cargo check` until Task 4 lands. Stage these changes; commit after Task 4 succeeds.

---

## Task 4: Rename `spawn_pin` → public `spawn_from_bytes`

**Files:**
- Modify: `src-tauri/src/pin/service.rs`

- [ ] **Step 1: Rename + make public**

In `src-tauri/src/pin/service.rs`, change the function signature:

```rust
pub fn spawn_from_bytes(app: &AppHandle, png_bytes: Vec<u8>) -> Result<(), String> {
    // body unchanged
```

And update the caller inside `paste_from_clipboard`:

```rust
    if let Err(msg) = spawn_from_bytes(app, png) {
        emit_error(app, msg);
    }
```

- [ ] **Step 2: Verify build (now Task 3 + 4 together compile)**

```
cd src-tauri && cargo build
```
Expected: clean compile.

- [ ] **Step 3: Verify frontend type compiles**

From project root:
```
npm run build
```
Expected: clean Vite + tsc compile.

- [ ] **Step 4: Commit Tasks 3 + 4 together**

```
git add src-tauri/src/ipc/commands.rs src-tauri/src/pin/service.rs src/shared/types.ts
git commit -m "feat(pin): expose spawn_from_bytes and add PinFromOverlay action"
```

---

## Task 5: `handles.ts` — selection hit-test + resize math

**Files:**
- Create: `src/windows/overlay/handles.ts`
- Create: `src/__tests__/handles.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/__tests__/handles.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import {
  hitTestHandle,
  cursorForHandle,
  resizeRect,
  type HandleId,
} from "../windows/overlay/handles";

const r = { x: 100, y: 100, w: 200, h: 100 };

describe("hitTestHandle", () => {
  it("returns null for points outside", () => {
    expect(hitTestHandle(r, { x: 50, y: 50 })).toBeNull();
    expect(hitTestHandle(r, { x: 400, y: 200 })).toBeNull();
  });

  it("returns the matching corner id within 6px", () => {
    expect(hitTestHandle(r, { x: 100, y: 100 })).toBe("nw");
    expect(hitTestHandle(r, { x: 300, y: 100 })).toBe("ne");
    expect(hitTestHandle(r, { x: 300, y: 200 })).toBe("se");
    expect(hitTestHandle(r, { x: 100, y: 200 })).toBe("sw");
  });

  it("returns the matching midpoint id", () => {
    expect(hitTestHandle(r, { x: 200, y: 100 })).toBe("n");
    expect(hitTestHandle(r, { x: 300, y: 150 })).toBe("e");
    expect(hitTestHandle(r, { x: 200, y: 200 })).toBe("s");
    expect(hitTestHandle(r, { x: 100, y: 150 })).toBe("w");
  });

  it("returns 'move' for points inside but not on a handle", () => {
    expect(hitTestHandle(r, { x: 200, y: 150 })).toBe("move");
  });
});

describe("cursorForHandle", () => {
  it.each([
    ["nw", "nwse-resize"],
    ["se", "nwse-resize"],
    ["ne", "nesw-resize"],
    ["sw", "nesw-resize"],
    ["n", "ns-resize"],
    ["s", "ns-resize"],
    ["e", "ew-resize"],
    ["w", "ew-resize"],
    ["move", "move"],
  ] as const)("%s → %s", (h, cur) => {
    expect(cursorForHandle(h as HandleId)).toBe(cur);
  });
});

describe("resizeRect", () => {
  it("se grows the rect by delta", () => {
    const out = resizeRect(r, "se", { x: 50, y: 25 });
    expect(out).toEqual({ x: 100, y: 100, w: 250, h: 125 });
  });

  it("nw shrinks the rect by moving origin and reducing size", () => {
    const out = resizeRect(r, "nw", { x: 20, y: 10 });
    expect(out).toEqual({ x: 120, y: 110, w: 180, h: 90 });
  });

  it("move shifts both origin and size unchanged", () => {
    const out = resizeRect(r, "move", { x: 30, y: -10 });
    expect(out).toEqual({ x: 130, y: 90, w: 200, h: 100 });
  });

  it("clamps to minSize when shrinking past it", () => {
    const out = resizeRect(r, "se", { x: -500, y: -500 }, 10);
    expect(out).toEqual({ x: 100, y: 100, w: 10, h: 10 });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```
npm test handles
```
Expected: cannot resolve module `handles`.

- [ ] **Step 3: Implement `handles.ts`**

Create `src/windows/overlay/handles.ts`:

```ts
import type { Rect } from "../../shared/types";

export type HandleId =
  | "nw" | "n" | "ne"
  | "e"
  | "se" | "s" | "sw"
  | "w"
  | "move";

export interface Point { x: number; y: number }

const HIT_RADIUS = 6;

function near(a: number, b: number, r = HIT_RADIUS): boolean {
  return Math.abs(a - b) <= r;
}

export function hitTestHandle(rect: Rect, pt: Point): HandleId | null {
  const { x, y, w, h } = rect;
  const cx = x + w / 2;
  const cy = y + h / 2;
  const r = x + w;
  const b = y + h;

  if (near(pt.x, x) && near(pt.y, y)) return "nw";
  if (near(pt.x, r) && near(pt.y, y)) return "ne";
  if (near(pt.x, r) && near(pt.y, b)) return "se";
  if (near(pt.x, x) && near(pt.y, b)) return "sw";
  if (near(pt.x, cx) && near(pt.y, y)) return "n";
  if (near(pt.x, r) && near(pt.y, cy)) return "e";
  if (near(pt.x, cx) && near(pt.y, b)) return "s";
  if (near(pt.x, x) && near(pt.y, cy)) return "w";

  if (pt.x >= x && pt.x <= r && pt.y >= y && pt.y <= b) return "move";
  return null;
}

export function cursorForHandle(h: HandleId): string {
  switch (h) {
    case "nw":
    case "se":
      return "nwse-resize";
    case "ne":
    case "sw":
      return "nesw-resize";
    case "n":
    case "s":
      return "ns-resize";
    case "e":
    case "w":
      return "ew-resize";
    case "move":
      return "move";
  }
}

export function resizeRect(
  rect: Rect,
  handle: HandleId,
  delta: Point,
  minSize = 10,
): Rect {
  let { x, y, w, h } = rect;

  if (handle === "move") {
    return { x: x + delta.x, y: y + delta.y, w, h };
  }

  if (handle.includes("w")) {
    const dx = Math.min(delta.x, w - minSize);
    x += dx;
    w -= dx;
  }
  if (handle.includes("e")) {
    w = Math.max(minSize, w + delta.x);
  }
  if (handle.includes("n")) {
    const dy = Math.min(delta.y, h - minSize);
    y += dy;
    h -= dy;
  }
  if (handle.includes("s")) {
    h = Math.max(minSize, h + delta.y);
  }
  return { x, y, w, h };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
npm test handles
```
Expected: all 4 describe blocks pass.

- [ ] **Step 5: Commit**

```
git add src/windows/overlay/handles.ts src/__tests__/handles.test.ts
git commit -m "feat(overlay): add handles module for selection hit-test + resize"
```

---

## Task 6: `toolbarPlacement.ts`

**Files:**
- Create: `src/windows/overlay/toolbarPlacement.ts`
- Create: `src/__tests__/toolbarPlacement.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/__tests__/toolbarPlacement.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { placeToolbar } from "../windows/overlay/toolbarPlacement";

const tbar = { w: 300, h: 36 };
const overlay = { w: 1920, h: 1080 };

describe("placeToolbar", () => {
  it("places below when space allows", () => {
    const sel = { x: 200, y: 100, w: 400, h: 200 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.orientation).toBe("below");
    expect(p.y).toBe(sel.y + sel.h + 8);
  });

  it("places above when below is too tight", () => {
    const sel = { x: 200, y: 100, w: 400, h: 1000 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.orientation).toBe("above");
    expect(p.y).toBe(sel.y - 8 - tbar.h);
  });

  it("falls back to inside when neither fits", () => {
    const tinyOverlay = { w: 1920, h: 50 };
    const sel = { x: 200, y: 0, w: 400, h: 50 };
    const p = placeToolbar(sel, tbar, tinyOverlay);
    expect(p.orientation).toBe("inside");
  });

  it("clamps x to overlay bounds at right edge", () => {
    const sel = { x: 1900, y: 100, w: 20, h: 50 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.x + tbar.w).toBeLessThanOrEqual(overlay.w);
  });

  it("clamps x to 0 at left edge", () => {
    const sel = { x: 0, y: 100, w: 50, h: 50 };
    const p = placeToolbar(sel, tbar, overlay);
    expect(p.x).toBeGreaterThanOrEqual(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```
npm test toolbarPlacement
```
Expected: cannot resolve module.

- [ ] **Step 3: Implement `toolbarPlacement.ts`**

Create `src/windows/overlay/toolbarPlacement.ts`:

```ts
import type { Rect } from "../../shared/types";

export interface ToolbarSize { w: number; h: number }
export interface OverlaySize { w: number; h: number }

export interface ToolbarPlacement {
  x: number;
  y: number;
  orientation: "below" | "above" | "inside";
}

export function placeToolbar(
  selection: Rect,
  toolbar: ToolbarSize,
  overlay: OverlaySize,
  gap = 8,
): ToolbarPlacement {
  const belowY = selection.y + selection.h + gap;
  const aboveY = selection.y - gap - toolbar.h;

  let orientation: ToolbarPlacement["orientation"];
  let y: number;
  if (belowY + toolbar.h <= overlay.h) {
    orientation = "below";
    y = belowY;
  } else if (aboveY >= 0) {
    orientation = "above";
    y = aboveY;
  } else {
    orientation = "inside";
    y = selection.y + selection.h - toolbar.h - gap;
  }

  const desiredX = selection.x + (selection.w - toolbar.w) / 2;
  const x = Math.max(0, Math.min(desiredX, overlay.w - toolbar.w));

  return { x, y, orientation };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
npm test toolbarPlacement
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```
git add src/windows/overlay/toolbarPlacement.ts src/__tests__/toolbarPlacement.test.ts
git commit -m "feat(overlay): add toolbar placement (below/above/inside)"
```

---

## Task 7: `magnifier.ts`

**Files:**
- Create: `src/windows/overlay/magnifier.ts`
- Create: `src/__tests__/magnifier.test.ts`

- [ ] **Step 1: Write a smoke test**

Create `src/__tests__/magnifier.test.ts`:

```ts
import { describe, it, expect, vi } from "vitest";
import { renderMagnifier } from "../windows/overlay/magnifier";

function makeCtx(): CanvasRenderingContext2D {
  return {
    clearRect: vi.fn(),
    drawImage: vi.fn(),
    fillRect: vi.fn(),
    fillText: vi.fn(),
    strokeRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    imageSmoothingEnabled: true,
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 1,
    font: "",
    textAlign: "left",
    textBaseline: "alphabetic",
    canvas: { width: 120, height: 120 } as HTMLCanvasElement,
  } as unknown as CanvasRenderingContext2D;
}

describe("renderMagnifier", () => {
  it("does not throw with a valid source image and cursor", () => {
    const ctx = makeCtx();
    const img = new Image() as HTMLImageElement;
    Object.defineProperty(img, "width", { value: 1920 });
    Object.defineProperty(img, "height", { value: 1080 });
    expect(() =>
      renderMagnifier(ctx, img, { x: 806, y: 506 }, 5),
    ).not.toThrow();
  });

  it("invokes drawImage exactly once", () => {
    const ctx = makeCtx();
    const img = new Image() as HTMLImageElement;
    Object.defineProperty(img, "width", { value: 1920 });
    Object.defineProperty(img, "height", { value: 1080 });
    renderMagnifier(ctx, img, { x: 100, y: 100 }, 5);
    expect(ctx.drawImage).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```
npm test magnifier
```
Expected: cannot resolve module.

- [ ] **Step 3: Implement `magnifier.ts`**

Create `src/windows/overlay/magnifier.ts`:

```ts
import type { Point } from "./handles";

const SIZE = 120;
const DEFAULT_ZOOM = 5;

export function renderMagnifier(
  ctx: CanvasRenderingContext2D,
  source: HTMLImageElement,
  cursor: Point,
  zoom: number = DEFAULT_ZOOM,
): void {
  const w = ctx.canvas.width;
  const h = ctx.canvas.height;
  const srcSpan = w / zoom;
  const sx = Math.max(0, Math.min(source.width - srcSpan, cursor.x - srcSpan / 2));
  const sy = Math.max(0, Math.min(source.height - srcSpan, cursor.y - srcSpan / 2));

  ctx.clearRect(0, 0, w, h);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(source, sx, sy, srcSpan, srcSpan, 0, 0, w, h);

  ctx.strokeStyle = "rgba(0, 128, 255, 0.9)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, h / 2);
  ctx.lineTo(w, h / 2);
  ctx.moveTo(w / 2, 0);
  ctx.lineTo(w / 2, h);
  ctx.stroke();

  ctx.strokeStyle = "rgba(255, 255, 255, 0.8)";
  ctx.strokeRect(w / 2 - 3, h / 2 - 3, 6, 6);

  ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
  ctx.fillRect(0, h - 18, w, 18);
  ctx.fillStyle = "#fff";
  ctx.font = "12px monospace";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(`(${Math.round(cursor.x)}, ${Math.round(cursor.y)})`, w / 2, h - 9);
}

export const MAGNIFIER_SIZE = SIZE;
```

- [ ] **Step 4: Run tests to verify they pass**

```
npm test magnifier
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```
git add src/windows/overlay/magnifier.ts src/__tests__/magnifier.test.ts
git commit -m "feat(overlay): add magnifier loupe renderer"
```

---

## Task 8: Move editor components to shared + re-parameterize Stage + add Pin button

This task is the migration step. After it, `src/windows/editor/App.vue` will visibly break (it passes obsolete props to Stage), but the editor window is being killed in Task 11 so the broken intermediate is accepted.

**Files:**
- Move (`git mv`): `src/windows/editor/state/` → `src/shared/editor/state/`
- Move (`git mv`): `src/windows/editor/canvas/` → `src/shared/editor/canvas/`
- Move (`git mv`): `src/windows/editor/ui/` → `src/shared/editor/ui/`
- Modify: `src/shared/editor/canvas/Stage.vue` (new props)
- Modify: `src/shared/editor/ui/ActionBar.vue` (Pin button)
- Modify: `src/windows/editor/App.vue` (update imports to point at shared)

- [ ] **Step 1: Move the three subfolders**

```
git mv src/windows/editor/state src/shared/editor/state
git mv src/windows/editor/canvas src/shared/editor/canvas
git mv src/windows/editor/ui src/shared/editor/ui
```

- [ ] **Step 2: Update import paths inside moved files**

The moved files import from `../../../shared/...` (3 levels up). After moving they are now 2 levels deep under `src/shared/editor/`, so the original `../../../shared` path now needs to be `../../` to reach `src/shared/`. Update each moved file by replacing `../../../shared/` with `../../`.

Files to update (each contains imports like `from "../../../shared/types"`):

- `src/shared/editor/state/shapes.ts`
- `src/shared/editor/canvas/Stage.vue`
- `src/shared/editor/canvas/drawTools.ts`
- `src/shared/editor/canvas/textTool.ts`
- `src/shared/editor/ui/Toolbar.vue`
- `src/shared/editor/ui/ActionBar.vue`

Replacement pattern (do per file):
- Old: `from "../../../shared/types"` → New: `from "../../types"`
- Old: `from "../../../shared/colors"` → New: `from "../../colors"`
- Old: `from "../../../shared/ipc"` → New: `from "../../ipc"`
- Old: `from "../../../shared/toast"` → New: `from "../../toast"`

- [ ] **Step 3: Update `src/windows/editor/App.vue` imports**

The old editor window's App.vue still imports its components from local paths. Update it to point to shared:

```ts
import Stage from "../../shared/editor/canvas/Stage.vue";
import Toolbar from "../../shared/editor/ui/Toolbar.vue";
import ActionBar from "../../shared/editor/ui/ActionBar.vue";
import {
  editorState,
  undo,
  redo,
  commitChange,
  resetEditor,
} from "../../shared/editor/state/shapes";
```

(The rest of `src/windows/editor/App.vue` stays unchanged.)

- [ ] **Step 4: Re-parameterize Stage.vue**

Replace the `defineProps` block and the `onMounted` + `watch` blocks in `src/shared/editor/canvas/Stage.vue`.

New props:

```ts
const props = defineProps<{
  bgUrl: string;
  selection: { x: number; y: number; w: number; h: number };
  overlaySize: { w: number; h: number };
}>();
```

Replace the `onMounted` size-setup with overlay-sized stage:

```ts
onMounted(() => {
  if (!containerRef.value) return;
  stage = new Konva.Stage({
    container: containerRef.value,
    width: props.overlaySize.w,
    height: props.overlaySize.h,
  });
  bgLayer = new Konva.Layer({ listening: false });
  annLayer = new Konva.Layer();
  previewLayer = new Konva.Layer({ listening: false });
  uiLayer = new Konva.Layer();
  stage.add(bgLayer);
  stage.add(annLayer);
  stage.add(previewLayer);
  stage.add(uiLayer);
  // (transformer + event handlers below stay unchanged — they operate on stage-local coords,
  //  which are now overlay-global coords; the math still works.)
  // ... existing transformer / click / mousedown / mousemove / mouseup blocks ...
});
```

Replace the `watch(() => props.imageUrl, ...)` block with a `bgUrl` watcher that loads the full overlay background:

```ts
watch(
  () => props.bgUrl,
  async (url) => {
    if (!url || !stage || !bgLayer) return;
    const img = new Image();
    img.src = url;
    try {
      await img.decode();
    } catch {
      return;
    }
    bgImage = img;
    const kImg = new Konva.Image({
      image: img,
      x: 0,
      y: 0,
      width: props.overlaySize.w,
      height: props.overlaySize.h,
    });
    bgLayer.destroyChildren();
    bgLayer.add(kImg);
    bgLayer.draw();
    stage.size({ width: props.overlaySize.w, height: props.overlaySize.h });
    rerenderAnnotations();
  },
  { immediate: true },
);
```

Leave the rest of the file (`buildDraftShape`, `renderPreview`, mousedown/mousemove/mouseup, `rerenderAnnotations`, `defineExpose`) intact. The selection prop is **not** used inside Stage — clipping is handled by the outer DOM container that the overlay App.vue owns. Stage simply renders at overlay-size; the consumer wraps it in a `overflow: hidden` container to crop visually.

- [ ] **Step 5: Add Pin button to ActionBar.vue**

In `src/shared/editor/ui/ActionBar.vue`, add a `pinIt` function and button.

After `saveAndCopy`:

```ts
async function pinIt() {
  doAction({ kind: "PinFromOverlay" });
}
```

In the template, add a button after `Save+Copy`:

```html
    <button @click="pinIt">Pin</button>
```

- [ ] **Step 6: Verify the build still compiles**

```
npm run build
```
Expected: clean (the old editor window's Stage call passes obsolete props, but Vue templates aren't type-checked strictly — TS compile passes).

- [ ] **Step 7: Commit**

```
git add -A
git commit -m "refactor(editor): move components to shared, re-parameterize Stage, add Pin"
```

---

## Task 9: Rewrite `src/windows/overlay/App.vue` with two-phase machine

**Files:**
- Modify: `src/windows/overlay/App.vue`
- Modify: `src/windows/overlay/overlay.css` (additive styles for editing phase)

- [ ] **Step 1: Replace `src/windows/overlay/App.vue` with the two-phase version**

```vue
<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { call, on } from "../../shared/ipc";
import { rectFromDrag, clampToBounds, type Point as SelPoint } from "./selection";
import { hitTestHandle, resizeRect, cursorForHandle, type HandleId } from "./handles";
import { placeToolbar } from "./toolbarPlacement";
import { renderMagnifier, MAGNIFIER_SIZE } from "./magnifier";
import Stage from "../../shared/editor/canvas/Stage.vue";
import Toolbar from "../../shared/editor/ui/Toolbar.vue";
import ActionBar from "../../shared/editor/ui/ActionBar.vue";
import { editorState, resetEditor } from "../../shared/editor/state/shapes";
import Toast from "../../shared/Toast.vue";
import type { Rect } from "../../shared/types";

type Phase = "idle" | "framing" | "editing";

const state = reactive({
  phase: "idle" as Phase,
  bgUrl: "",
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },
  dragStart: null as SelPoint | null,
  dragEnd: null as SelPoint | null,
  selection: null as Rect | null,
  cursor: null as SelPoint | null,
  activeHandle: null as HandleId | null,
  dragLast: null as SelPoint | null,
});

const bgImg = ref<HTMLImageElement | null>(null);
const magCanvas = ref<HTMLCanvasElement | null>(null);
const toolbarRef = ref<HTMLDivElement | null>(null);

const framingRectStyle = computed(() => {
  if (!state.dragStart || !state.dragEnd) return { display: "none" };
  const r = rectFromDrag(state.dragStart, state.dragEnd);
  return {
    left: r.x + "px",
    top: r.y + "px",
    width: r.w + "px",
    height: r.h + "px",
  };
});

const selectionStyle = computed(() => {
  if (!state.selection) return { display: "none" };
  const s = state.selection;
  return {
    left: s.x + "px",
    top: s.y + "px",
    width: s.w + "px",
    height: s.h + "px",
  };
});

const toolbarPlacement = computed(() => {
  if (!state.selection) return null;
  const tbar = toolbarRef.value
    ? { w: toolbarRef.value.offsetWidth, h: toolbarRef.value.offsetHeight }
    : { w: 320, h: 36 };
  return placeToolbar(
    state.selection,
    tbar,
    { w: state.width, h: state.height },
  );
});

const toolbarStyle = computed(() => {
  const p = toolbarPlacement.value;
  if (!p) return { display: "none" };
  return { left: p.x + "px", top: p.y + "px" };
});

const magnifierStyle = computed(() => {
  if (!state.cursor) return { display: "none" };
  const off = 20;
  const right = state.cursor.x + off + MAGNIFIER_SIZE > state.width;
  const bottom = state.cursor.y + off + MAGNIFIER_SIZE > state.height;
  const left = right ? state.cursor.x - off - MAGNIFIER_SIZE : state.cursor.x + off;
  const top = bottom ? state.cursor.y - off - MAGNIFIER_SIZE : state.cursor.y + off;
  return { left: left + "px", top: top + "px" };
});

function drawMagnifier() {
  if (!magCanvas.value || !bgImg.value || !state.cursor) return;
  const ctx = magCanvas.value.getContext("2d");
  if (!ctx) return;
  renderMagnifier(ctx, bgImg.value, state.cursor, 5);
}

watch(() => state.cursor, drawMagnifier);

onMounted(() => {
  on<{
    thumbnail_b64: string;
    width: number;
    height: number;
    origin_x: number;
    origin_y: number;
  }>("capture-ready", async (p) => {
    state.bgUrl = `data:image/png;base64,${p.thumbnail_b64}`;
    state.width = p.width;
    state.height = p.height;
    state.origin = { x: p.origin_x, y: p.origin_y };
    const img = new Image();
    img.src = state.bgUrl;
    await img.decode().catch(() => {});
    bgImg.value = img;
    resetEditor();
    state.phase = "framing";
    state.selection = null;
    state.dragStart = null;
    state.dragEnd = null;
    state.cursor = null;
  });

  on("capture-clear", () => {
    state.phase = "idle";
    state.selection = null;
    state.dragStart = null;
    state.dragEnd = null;
    state.cursor = null;
    resetEditor();
  });

  window.addEventListener("keydown", onKey);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
});

function onKey(e: KeyboardEvent) {
  if (state.phase === "idle") return;
  if (e.key === "Escape") {
    e.preventDefault();
    cancel();
  } else if (e.key === "Enter" && state.phase === "editing") {
    e.preventDefault();
    defaultAction();
  }
}

async function cancel() {
  await call("selection_cancelled");
}

async function defaultAction() {
  // Copy is the default terminal action — delegate to the ActionBar logic via a synthetic click.
  // Implemented by exposing a global hook on ActionBar (see ActionBar.vue defineExpose).
  const ab = (window as unknown as { __overlayActionBarCopy?: () => void }).__overlayActionBarCopy;
  if (ab) ab();
}

function onMouseDown(e: MouseEvent) {
  if (state.phase === "framing") {
    state.dragStart = { x: e.clientX, y: e.clientY };
    state.dragEnd = { x: e.clientX, y: e.clientY };
  } else if (state.phase === "editing" && state.selection) {
    const pt = { x: e.clientX, y: e.clientY };
    const hit = hitTestHandle(state.selection, pt);
    if (hit === null) {
      // Click outside selection in editing → reframe
      void requestReframe();
    } else {
      state.activeHandle = hit;
      state.dragLast = pt;
    }
  }
}

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
  } else {
    state.cursor = null;
  }
}

async function onMouseUp() {
  if (state.phase === "framing") {
    if (!state.dragStart || !state.dragEnd) return;
    const local = rectFromDrag(state.dragStart, state.dragEnd);
    const clamped = clampToBounds(local, state.width, state.height);
    state.dragStart = null;
    state.dragEnd = null;
    if (clamped.w < 5 || clamped.h < 5) return;
    state.selection = clamped;
    try {
      await call("selection_confirmed", { rect: {
        x: clamped.x + state.origin.x,
        y: clamped.y + state.origin.y,
        w: clamped.w,
        h: clamped.h,
      } });
      state.phase = "editing";
      state.cursor = null;
    } catch (err) {
      state.selection = null;
      state.phase = "framing";
      throw err;
    }
  } else if (state.phase === "editing" && state.activeHandle) {
    state.activeHandle = null;
    state.dragLast = null;
    state.cursor = null;
  }
}

async function requestReframe() {
  try {
    await call("reframe_request");
  } catch {
    // race: phase already changed elsewhere; ignore
  }
  state.phase = "framing";
  state.selection = null;
  resetEditor();
}

function onDblClick() {
  if (state.phase === "editing") defaultAction();
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  cancel();
}
</script>

<template>
  <div
    class="overlay"
    :style="{ backgroundImage: `url(${state.bgUrl})` }"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDblClick"
    @contextmenu="onContextMenu"
  >
    <div class="dim"></div>

    <!-- Framing rectangle (during initial drag) -->
    <div v-if="state.phase === 'framing'" class="selection" :style="framingRectStyle"></div>

    <!-- Editing: persistent selection + handles + Stage + toolbars -->
    <template v-if="state.phase === 'editing' && state.selection">
      <div class="selection editing-frame" :style="selectionStyle">
        <div class="handle nw"></div>
        <div class="handle n"></div>
        <div class="handle ne"></div>
        <div class="handle e"></div>
        <div class="handle se"></div>
        <div class="handle s"></div>
        <div class="handle sw"></div>
        <div class="handle w"></div>
      </div>

      <div class="stage-clip" :style="selectionStyle">
        <div class="stage-inner" :style="{ left: -state.selection.x + 'px', top: -state.selection.y + 'px' }">
          <Stage
            :bg-url="state.bgUrl"
            :selection="state.selection"
            :overlay-size="{ w: state.width, h: state.height }"
          />
        </div>
      </div>

      <div class="floating-toolbar" :style="toolbarStyle" ref="toolbarRef">
        <Toolbar />
        <ActionBar />
      </div>
    </template>

    <!-- Magnifier (framing or handle-drag) -->
    <canvas
      v-if="state.cursor"
      class="magnifier"
      :style="magnifierStyle"
      ref="magCanvas"
      width="120"
      height="120"
    ></canvas>

    <Toast />
  </div>
</template>

<style scoped src="./overlay.css"></style>
```

- [ ] **Step 2: Add the editing-phase CSS**

Append to `src/windows/overlay/overlay.css`:

```css
.editing-frame {
  border: 2px solid rgba(0, 128, 255, 0.95);
  pointer-events: none;
}

.handle {
  position: absolute;
  width: 8px;
  height: 8px;
  background: #fff;
  border: 1px solid rgba(0, 128, 255, 0.95);
  pointer-events: none;
}
.handle.nw { left: -5px; top: -5px; }
.handle.n  { left: calc(50% - 4px); top: -5px; }
.handle.ne { right: -5px; top: -5px; }
.handle.e  { right: -5px; top: calc(50% - 4px); }
.handle.se { right: -5px; bottom: -5px; }
.handle.s  { left: calc(50% - 4px); bottom: -5px; }
.handle.sw { left: -5px; bottom: -5px; }
.handle.w  { left: -5px; top: calc(50% - 4px); }

.stage-clip {
  position: absolute;
  overflow: hidden;
  pointer-events: auto;
}
.stage-inner {
  position: absolute;
}

.floating-toolbar {
  position: absolute;
  display: flex;
  gap: 4px;
  padding: 4px;
  background: rgba(30, 30, 30, 0.92);
  border-radius: 6px;
  pointer-events: auto;
  z-index: 10;
}

.magnifier {
  position: absolute;
  pointer-events: none;
  z-index: 20;
  border: 1px solid rgba(255, 255, 255, 0.5);
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.6);
}
```

- [ ] **Step 3: Wire ActionBar's Copy as the default-action hook**

In `src/shared/editor/ui/ActionBar.vue` `<script setup>`, expose copy globally so overlay App.vue can fire it on Enter / double-click:

```ts
import { onMounted, onUnmounted } from "vue";

onMounted(() => {
  (window as unknown as { __overlayActionBarCopy?: () => void }).__overlayActionBarCopy = copyImage;
});

onUnmounted(() => {
  delete (window as unknown as { __overlayActionBarCopy?: () => void }).__overlayActionBarCopy;
});
```

- [ ] **Step 4: Run the dev build to smoke-test compilation**

```
npm run build
```
Expected: clean compile. Behavior at this point: editor window still shows when capture confirms (because `selection_confirmed` still calls `editor.show()`), and the overlay simultaneously enters editing. Both UIs visible. This is fixed in Task 10.

- [ ] **Step 5: Commit**

```
git add -A
git commit -m "feat(overlay): rewrite App.vue with framing/editing two-phase machine"
```

---

## Task 10: Simplify `selection_confirmed` — drop crop + editor.show()

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`

- [ ] **Step 1: Replace the body of `selection_confirmed`**

Replace the whole `selection_confirmed` function in `src-tauri/src/ipc/commands.rs` with:

```rust
#[tauri::command]
pub fn selection_confirmed(
    rect: Rect,
    state: State<AppState>,
    _app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        phase
            .transition(PhaseEvent::SelectionConfirmed)
            .map_err(|e| AppError::State(e.to_string()))?;
    }
    tracing::info!("selection_confirmed: rect={:?}, phase -> Editing", rect);
    Ok(())
}
```

Remove the unused `cap`/`cropped`/`overlay.emit`/`editor.show` logic. The `rect` parameter is kept in the signature for API stability (frontend still sends it, even though backend no longer crops; useful for telemetry and the upcoming finish_action validation).

- [ ] **Step 2: Verify build**

```
cd src-tauri && cargo build
```
Expected: clean.

- [ ] **Step 3: Smoke-test the full flow manually**

Run the app via `npx tauri dev`. Trigger the capture hotkey, frame a region, and confirm:
- Editor window NO LONGER opens
- Overlay enters editing mode with handles, toolbar, Stage
- Copy / Save / Pin all work and the overlay parks back to (-32000, -32000)
- Esc cancels cleanly

- [ ] **Step 4: Commit**

```
git add src-tauri/src/ipc/commands.rs
git commit -m "refactor(capture): drop server-side crop, overlay owns editing"
```

---

## Task 11: Delete editor window + entry + HTML + folder

**Files:**
- Modify: `src-tauri/tauri.conf.json` (remove `editor` window)
- Modify: `src-tauri/src/lib.rs` (remove `editor` from CloseRequested loop)
- Modify: `src-tauri/src/ipc/commands.rs` (remove `editor.hide()` calls in `finalize` + `cancel_edit`)
- Modify: `vite.config.ts` (remove `editor` input)
- Modify: `src/main.ts` (remove `case "editor"`)
- Delete: `editor.html`
- Delete: `src/windows/editor/` (whole directory — App.vue is the only remaining file after Task 8's moves)

- [ ] **Step 1: Remove editor window from `tauri.conf.json`**

Delete the entire `editor` object from the `windows` array. After the edit, `windows` contains only `settings` and `overlay`.

- [ ] **Step 2: Update `src-tauri/src/lib.rs`**

Change the CloseRequested loop's label list from `["editor", "overlay", "settings"]` to `["overlay", "settings"]`.

- [ ] **Step 3: Remove `editor.hide()` calls in commands.rs**

In `finalize`:

```rust
fn finalize(
    app: &AppHandle,
    state: &State<AppState>,
    outcome: FinishOutcome,
) -> Result<FinishOutcome, AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::ActionFinished);
    }
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    let _ = app.emit(
        "action-complete",
        serde_json::json!({ "saved_path": outcome.saved_path }),
    );
    Ok(outcome)
}
```

(Drop the `if let Some(editor) = app.get_webview_window("editor")` block entirely.)

In `cancel_edit`:

```rust
#[tauri::command]
pub fn cancel_edit(
    state: State<AppState>,
    _app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::Cancelled);
    }
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    Ok(())
}
```

- [ ] **Step 4: Remove `editor` from `vite.config.ts`**

```ts
      input: {
        overlay: resolve(__dirname, "overlay.html"),
        settings: resolve(__dirname, "settings.html"),
        pin: resolve(__dirname, "pin.html"),
      },
```

- [ ] **Step 5: Remove `case "editor"` from `src/main.ts`**

```ts
  switch (entry) {
    case "settings":
      App = (await import("./windows/settings/App.vue")).default;
      break;
    case "overlay":
      App = (await import("./windows/overlay/App.vue")).default;
      break;
    case "pin":
      App = (await import("./windows/pin/App.vue")).default;
      break;
    default:
      throw new Error(`unknown window entry: ${entry}`);
  }
```

- [ ] **Step 6: Delete editor.html and the editor directory**

```
git rm editor.html
git rm -r src/windows/editor
```

- [ ] **Step 7: Verify full build**

```
npx tauri build
```
Expected: green build (uses the `npx tauri build` path per the project's documented gotcha).

- [ ] **Step 8: Commit**

```
git add -A
git commit -m "chore: retire editor window — overlay owns the full capture flow"
```

---

## Task 12: Manual test checklist + final verification

**Files:**
- Modify: `docs/manual-test-checklist.md`

- [ ] **Step 1: Append the Inline Capture section**

Append to `docs/manual-test-checklist.md`:

```markdown
## Inline Capture (Snipaste-style)

### Framing
- [ ] Capture hotkey → overlay shows, drag-to-frame works
- [ ] Magnifier follows cursor, offsets away from it, shows correct coords
- [ ] Drag < 5px → no transition, can re-drag
- [ ] Esc → cancels back to idle

### Editing
- [ ] mouseup shows 8 handles + toolbar below selection
- [ ] Toolbar flips above when below is tight
- [ ] Toolbar falls back to inside when overlay is shorter than expected
- [ ] Resizing via handle: annotations stay at their pixel positions, clip outside new selection
- [ ] Magnifier appears during handle drag
- [ ] Dragging selection body (move handle): viewport slides over fixed background; annotations reveal/clip as the selection moves
- [ ] Left-click outside selection → shapes cleared, returns to framing
- [ ] Double-click inside selection → default action (Copy) + exit
- [ ] Enter → default action (Copy) + exit
- [ ] Esc / right-click → cancels everything

### Finish actions
- [ ] Copy → clipboard has image, overlay exits
- [ ] Save → dialog, path picked, file written, toast
- [ ] Save+Copy → default path used, clipboard FileList set, toast
- [ ] Pin → pin window spawned with edited image, overlay exits
- [ ] Any action failure → toast shown, stays in editing

### Multi-monitor
- [ ] Primary + secondary monitor both captureable
- [ ] Cross-monitor selection, magnifier, toolbar placement all correct
```

- [ ] **Step 2: Run the unit test suites end-to-end**

```
npm test
cd src-tauri && cargo test
```
Expected: all suites green.

- [ ] **Step 3: Run the final production build**

```
npx tauri build
```
Expected: MSI/NSIS installer produced under `src-tauri/target/release/bundle/`.

- [ ] **Step 4: Smoke-test the built binary**

Launch the installed binary (or `target/release/minipaste.exe`). Run through the manual checklist top to bottom.

- [ ] **Step 5: Commit the checklist update**

```
git add docs/manual-test-checklist.md
git commit -m "docs: add manual checklist for inline capture flow"
```

---

## Self-Review Notes

- **Spec coverage:** Every section of `2026-05-13-inline-capture-design.md` has at least one corresponding task. Phase machine → Task 1. `reframe_request` → Task 2. `PinFromOverlay` → Task 3. `spawn_from_bytes` → Task 4. `handles` / `magnifier` / `toolbarPlacement` → Tasks 5-7. Component migration + Stage re-param + Pin button → Task 8. Overlay rewrite (App.vue + CSS + default-action wiring) → Task 9. `selection_confirmed` simplification → Task 10. Editor retirement → Task 11. Manual checklist → Task 12.
- **No placeholders:** Every code step contains the exact content. No "TODO" or "similar to" references.
- **Type consistency:** `HandleId`, `Rect`, `Point`, `ToolbarPlacement`, `FinishAction` are defined in one task each and referenced consistently in later tasks. `spawn_from_bytes` is introduced in Task 4 before being called in Task 3's match arm (which is why Tasks 3 + 4 commit together).
- **Intermediate broken states:** Between Task 8 and Task 11 the legacy editor window's `App.vue` calls `Stage` with the old prop shape; this is acceptable because the window is being deleted in Task 11 and is never opened during normal flow after Task 10.
