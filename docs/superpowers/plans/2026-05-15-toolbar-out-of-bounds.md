# Toolbar Multi-Screen Clamp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Place the floating toolbar inside the **physical screen containing the selection center**, not inside the larger virtual desktop. This prevents the toolbar from landing in dead zones between screens or off-monitor areas.

**Architecture:** Introduce `findActiveScreen` to pick the screen containing the selection center (overlay-local coords). Change `placeToolbar` to take a `bounds: Rect` instead of overlay size — all `below`/`above`/`inside` decisions and clamping happen against that rect. `App.vue` plumbs the `screens` payload through and feeds the active screen to placement.

**Tech Stack:** TypeScript, Vitest, Vue 3.

**Spec:** `docs/superpowers/specs/2026-05-15-toolbar-out-of-bounds-design.md`

---

### Task 1: Revert v1 y-clamp from working tree

The v1 fix in `toolbarPlacement.ts` and the v1 tests in `toolbarPlacement.test.ts` are now superseded. Reset both files to HEAD so we start clean.

**Files:**
- Modify: `src/windows/overlay/toolbarPlacement.ts`
- Modify: `src/__tests__/toolbarPlacement.test.ts`

- [ ] **Step 1: Reset both files to HEAD**

Run:
```bash
git checkout HEAD -- src/windows/overlay/toolbarPlacement.ts src/__tests__/toolbarPlacement.test.ts
```

Expected: working tree shows no diff for these two files (`git status` should not list them).

- [ ] **Step 2: Sanity-run tests**

Run: `npm test -- toolbarPlacement`

Expected: 5 PASS (the original test set).

---

### Task 2: Add `findActiveScreen` helper with tests

**Files:**
- Create: `src/windows/overlay/findActiveScreen.ts`
- Create: `src/__tests__/findActiveScreen.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/__tests__/findActiveScreen.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { findActiveScreen } from "../windows/overlay/findActiveScreen";
import type { Rect } from "../../shared/types";

const fallback: Rect = { x: 0, y: 0, w: 4080, h: 1925 };

describe("findActiveScreen", () => {
  const screens: Rect[] = [
    { x: 0, y: 0, w: 1080, h: 1920 },        // left portrait
    { x: 1080, y: 434, w: 1920, h: 1080 },   // primary
    { x: 3000, y: 434, w: 1080, h: 1920 },   // right portrait
  ];

  it("returns the screen containing the selection center", () => {
    const sel = { x: 1500, y: 700, w: 200, h: 200 }; // center (1600, 800) → primary
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[1]);
  });

  it("returns the left screen when center is on the left", () => {
    const sel = { x: 100, y: 100, w: 200, h: 200 }; // center (200, 200) → left
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[0]);
  });

  it("returns fallback when center is in a dead zone", () => {
    const sel = { x: 1080, y: 0, w: 200, h: 200 }; // center (1180, 100) → above primary, dead zone
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(fallback);
  });

  it("returns fallback when no screens are provided", () => {
    const sel = { x: 100, y: 100, w: 50, h: 50 };
    const r = findActiveScreen(sel, [], fallback);
    expect(r).toEqual(fallback);
  });

  it("treats screen rect as half-open: top/left inclusive, bottom/right exclusive", () => {
    const sel = { x: 1080, y: 434, w: 0, h: 0 }; // center exactly (1080, 434) → primary (inclusive)
    const r = findActiveScreen(sel, screens, fallback);
    expect(r).toEqual(screens[1]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- findActiveScreen`

Expected: import failure (`findActiveScreen` doesn't exist yet).

- [ ] **Step 3: Implement `findActiveScreen`**

Create `src/windows/overlay/findActiveScreen.ts`:

```ts
import type { Rect } from "../../shared/types";

export function findActiveScreen(
  selection: Rect,
  screens: Rect[],
  fallback: Rect,
): Rect {
  const cx = selection.x + selection.w / 2;
  const cy = selection.y + selection.h / 2;
  for (const s of screens) {
    if (cx >= s.x && cx < s.x + s.w && cy >= s.y && cy < s.y + s.h) {
      return s;
    }
  }
  return fallback;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- findActiveScreen`

Expected: 5 PASS.

---

### Task 3: Change `placeToolbar` to use `bounds: Rect`

**Files:**
- Modify: `src/windows/overlay/toolbarPlacement.ts`
- Modify: `src/__tests__/toolbarPlacement.test.ts`

- [ ] **Step 1: Update existing tests to use `bounds` and add multi-screen cases**

Replace the entire content of `src/__tests__/toolbarPlacement.test.ts` with:

```ts
import { describe, it, expect } from "vitest";
import { placeToolbar } from "../windows/overlay/toolbarPlacement";
import type { Rect } from "../../shared/types";

const tbar = { w: 300, h: 36 };
const screen: Rect = { x: 0, y: 0, w: 1920, h: 1080 };

describe("placeToolbar", () => {
  it("places below when space allows", () => {
    const sel = { x: 200, y: 100, w: 400, h: 200 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.orientation).toBe("below");
    expect(p.y).toBe(sel.y + sel.h + 8);
  });

  it("places above when below is too tight", () => {
    const sel = { x: 200, y: 100, w: 400, h: 1000 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.orientation).toBe("above");
    expect(p.y).toBe(sel.y - 8 - tbar.h);
  });

  it("falls back to inside when neither fits", () => {
    const tinyScreen: Rect = { x: 0, y: 0, w: 1920, h: 50 };
    const sel = { x: 200, y: 0, w: 400, h: 50 };
    const p = placeToolbar(sel, tbar, tinyScreen);
    expect(p.orientation).toBe("inside");
  });

  it("clamps x to bounds at right edge", () => {
    const sel = { x: 1900, y: 100, w: 20, h: 50 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.x + tbar.w).toBeLessThanOrEqual(screen.x + screen.w);
  });

  it("clamps x to bounds at left edge", () => {
    const sel = { x: 0, y: 100, w: 50, h: 50 };
    const p = placeToolbar(sel, tbar, screen);
    expect(p.x).toBeGreaterThanOrEqual(screen.x);
  });

  describe("multi-screen bounds (bounds not at origin)", () => {
    // Simulates the active screen being at virtual position (1080, 434)
    // inside a larger overlay window covering 4080x1925.
    const primary: Rect = { x: 1080, y: 434, w: 1920, h: 1080 };

    it("clamps toolbar inside the active screen, not the larger overlay", () => {
      // Selection at the bottom of the primary screen.
      const sel = { x: 1500, y: 1300, w: 600, h: 200 };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.x).toBeGreaterThanOrEqual(primary.x);
      expect(p.x + tbar.w).toBeLessThanOrEqual(primary.x + primary.w);
      expect(p.y).toBeGreaterThanOrEqual(primary.y);
      expect(p.y + tbar.h).toBeLessThanOrEqual(primary.y + primary.h);
    });

    it("places below relative to the active screen's bottom edge", () => {
      // Below fits within the primary screen.
      const sel = { x: 1500, y: 600, w: 200, h: 200 };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.orientation).toBe("below");
      expect(p.y).toBe(sel.y + sel.h + 8);
    });

    it("falls back to inside when selection covers the whole active screen", () => {
      const sel = { ...primary };
      const p = placeToolbar(sel, tbar, primary);
      expect(p.orientation).toBe("inside");
      expect(p.y).toBeGreaterThanOrEqual(primary.y);
      expect(p.y + tbar.h).toBeLessThanOrEqual(primary.y + primary.h);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- toolbarPlacement`

Expected: tests fail (current `placeToolbar` signature takes `OverlaySize`, not `Rect`). TypeScript compile errors are fine — vitest will still report them.

- [ ] **Step 3: Rewrite `placeToolbar`**

Replace the entire content of `src/windows/overlay/toolbarPlacement.ts` with:

```ts
import type { Rect } from "../../shared/types";

export interface ToolbarSize { w: number; h: number }

export interface ToolbarPlacement {
  x: number;
  y: number;
  orientation: "below" | "above" | "inside";
}

export function placeToolbar(
  selection: Rect,
  toolbar: ToolbarSize,
  bounds: Rect,
  gap = 8,
): ToolbarPlacement {
  const belowY = selection.y + selection.h + gap;
  const aboveY = selection.y - gap - toolbar.h;
  const boundsBottom = bounds.y + bounds.h;
  const boundsRight = bounds.x + bounds.w;

  let orientation: ToolbarPlacement["orientation"];
  let y: number;
  if (belowY + toolbar.h <= boundsBottom) {
    orientation = "below";
    y = belowY;
  } else if (aboveY >= bounds.y) {
    orientation = "above";
    y = aboveY;
  } else {
    orientation = "inside";
    y = selection.y + selection.h - toolbar.h - gap;
  }

  const desiredX = selection.x + (selection.w - toolbar.w) / 2;
  const x = Math.max(bounds.x, Math.min(desiredX, boundsRight - toolbar.w));
  const clampedY = Math.max(bounds.y, Math.min(y, boundsBottom - toolbar.h));

  return { x, y: clampedY, orientation };
}
```

Note: the `OverlaySize` interface is removed — call sites must pass `Rect` (with `x`, `y`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- toolbarPlacement`

Expected: 8 PASS.

- [ ] **Step 5: Run full suite**

Run: `npm test`

Expected: every test PASSES. Note that App.vue uses `placeToolbar` with the old signature — TypeScript may flag this but the runtime tests don't exercise App.vue. We fix App.vue in Task 4.

If anything else fails, stop and investigate before continuing.

---

### Task 4: Plumb `screens` through App.vue and feed active screen to placement

**Files:**
- Modify: `src/windows/overlay/App.vue`

- [ ] **Step 1: Import the helper and `ScreenInfo` type**

In `src/windows/overlay/App.vue`, change the imports near the top so the new helper and type are available. Update the import block to include:

```ts
import { placeToolbar } from "./toolbarPlacement";
import { findActiveScreen } from "./findActiveScreen";
import type { Rect, ScreenInfo } from "../../shared/types";
```

(`Rect` import line already exists — keep it; just add `ScreenInfo`.)

- [ ] **Step 2: Add `screens` to reactive state**

In the `reactive({ ... })` literal at the top of `<script setup>`, add a `screens` field initialized to an empty array. The full updated reactive block should read:

```ts
const state = reactive({
  phase: "idle" as Phase,
  bgUrl: "",
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },
  screens: [] as Rect[],
  dragStart: null as SelPoint | null,
  dragEnd: null as SelPoint | null,
  selection: null as Rect | null,
  cursor: null as SelPoint | null,
  activeHandle: null as HandleId | null,
  dragLast: null as SelPoint | null,
});
```

- [ ] **Step 3: Populate `state.screens` from the payload**

In the `on<{...}>("capture-ready", ...)` handler, after the existing `state.origin = { x: p.origin_x, y: p.origin_y };` line, add:

```ts
state.screens = (p.screens ?? []).map((s) => ({
  x: s.x - p.origin_x,
  y: s.y - p.origin_y,
  w: s.w,
  h: s.h,
}));
```

The payload type at the `on` callsite must include `screens`. Update the generic to:

```ts
on<{
  thumbnail_b64: string;
  width: number;
  height: number;
  origin_x: number;
  origin_y: number;
  screens: ScreenInfo[];
}>("capture-ready", async (p) => { ... });
```

Also add `state.screens = [];` inside the `"capture-clear"` handler (resetting alongside the other state fields).

- [ ] **Step 4: Compute toolbar bounds and pass to `placeToolbar`**

Replace the existing `toolbarPlacement` computed:

```ts
const toolbarPlacement = computed(() => {
  if (!state.selection) return null;
  const tbar = toolbarRef.value
    ? { w: toolbarRef.value.offsetWidth, h: toolbarRef.value.offsetHeight }
    : { w: 320, h: 80 };
  return placeToolbar(
    state.selection,
    tbar,
    { w: state.width, h: state.height },
  );
});
```

with the new version that resolves the active screen first:

```ts
const toolbarPlacement = computed(() => {
  if (!state.selection) return null;
  const tbar = toolbarRef.value
    ? { w: toolbarRef.value.offsetWidth, h: toolbarRef.value.offsetHeight }
    : { w: 320, h: 80 };
  const fallback: Rect = { x: 0, y: 0, w: state.width, h: state.height };
  const bounds = findActiveScreen(state.selection, state.screens, fallback);
  return placeToolbar(state.selection, tbar, bounds);
});
```

- [ ] **Step 5: Run full test suite**

Run: `npm test`

Expected: all tests PASS. (App.vue itself is not unit-tested, but the change should not break anything else.)

- [ ] **Step 6: Type-check the build**

Run: `npm run build`

Expected: `vue-tsc` passes with no errors. If TypeScript flags anything in App.vue, fix it before moving on.

---

### Task 5: Manual verification on the running app

**Files:** none (smoke test against built binary)

- [ ] **Step 1: Build the release binary**

Run: `npx tauri build`

Expected: build succeeds.

- [ ] **Step 2: Stop any existing instance and launch the new build**

Run (PowerShell):
```powershell
Get-Process minipaste -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process "D:\SideProject\MiniPaste\src-tauri\target\release\minipaste.exe"
```

Expected: minipaste running.

- [ ] **Step 3: Reproduce the original multi-screen scenario**

1. Trigger capture via hotkey
2. Make a selection on the **primary screen near its bottom edge** — the case that previously showed "工具列按不到"
3. Verify the toolbar appears INSIDE the primary screen (above or inside the selection, not in the dead zone below)

Expected: toolbar is visible and clickable on the primary screen.

- [ ] **Step 4: Cross-check secondary screens**

Repeat on each non-primary screen: small selection, large selection, selection at bottom edge. Confirm toolbar always lands within the same screen as the selection center.

- [ ] **Step 5: Single-screen sanity**

A normal small selection in the middle of the primary screen → toolbar still appears below the selection as before. No regression.

---

### Task 6: Commit

**Files:** none (git only)

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add src/windows/overlay/findActiveScreen.ts \
        src/windows/overlay/toolbarPlacement.ts \
        src/windows/overlay/App.vue \
        src/__tests__/findActiveScreen.test.ts \
        src/__tests__/toolbarPlacement.test.ts
git commit -m "fix(overlay): clamp floating toolbar to active screen, not virtual desktop"
```

Expected: clean commit with five files. No other files staged accidentally.
