# Toolbar 超出畫面範圍時點不到 — Design

**日期**：2026-05-15（v2 — 加上 multi-screen 根因）
**範圍**：`src/windows/overlay/toolbarPlacement.ts`、`src/windows/overlay/App.vue`、新增 `findActiveScreen` helper

## 問題

Editing 階段 floating toolbar 落在「視覺上點不到」的位置。

### Root Cause（更新）

`trigger_capture` 把 overlay 視窗鋪滿整個 **virtual desktop**（多螢幕加總，例如 4080x1925），並把 `screens: Vec<ScreenInfo>` 一起 emit 給 frontend。但 `App.vue` 沒接 `screens`，`placeToolbar` 也只看 overlay 整體 `{ w, h }`。

結果：placement 計算的座標可能落在「virtual desktop 範圍內」但「任何實體螢幕外」——例如螢幕之間的 dead zone、或 origin offset 造成的虛空間。從使用者視角這個位置「不存在」，所以按不到。

之前的 v1 spec 只 clamp y 到 `overlay.h`，在多螢幕情境完全沒幫助（clamp 到 1925 仍然超出單一實體螢幕範圍）。

### 既有資料

`CaptureReadyPayload` 已經帶：
```rust
pub struct ScreenInfo { pub x: i32, pub y: i32, pub w: u32, pub h: u32, pub scale: f32 }
pub struct CaptureReadyPayload {
    pub width: u32, pub height: u32,
    pub origin_x: i32, pub origin_y: i32,
    pub screens: Vec<ScreenInfo>,
    // ...
}
```

- `ScreenInfo.x/y` 是 **virtual desktop 座標**
- `origin_x/y` 是 virtual desktop 起點（對主螢幕的 offset，可能為負）
- 換成 overlay-local 座標：`screen_local = { x: screen.x - origin.x, y: screen.y - origin.y }`

## 解法

1. **`App.vue` 接 `screens`**，存到 `state.screens`。
2. **新增 `findActiveScreen(selection, screens, origin)`** helper：找 selection 中心點所在的螢幕，回傳 overlay-local Rect。中心不在任何螢幕（dead zone）就 fallback 整個 overlay。
3. **`placeToolbar` 簽名改為接 `bounds: Rect`**（active screen 的 overlay-local rect），不再接 `overlay: { w, h }`。所有 below/above/inside 計算與 clamp 都基於 `bounds`，不是整個 overlay。

### 新 API

```ts
// 新增 findActiveScreen.ts
export interface ScreenRect { x: number; y: number; w: number; h: number }
export function findActiveScreen(
  selection: Rect,
  screens: ScreenRect[],     // 已經換成 overlay-local 座標的螢幕陣列
  fallback: ScreenRect,      // 找不到時回傳的 bounds
): ScreenRect

// placeToolbar 改簽名
export function placeToolbar(
  selection: Rect,
  toolbar: ToolbarSize,
  bounds: ScreenRect,        // active screen 的 overlay-local rect
  gap = 8,
): ToolbarPlacement
```

`placeToolbar` 內部邏輯同原本，但所有比較對象改成 `bounds`：
- `belowY + tbar.h <= bounds.y + bounds.h` → place below
- `aboveY >= bounds.y` → place above
- inside: `y = sel.y + sel.h - tbar.h - gap`（不變）
- 最後 clamp：`x ∈ [bounds.x, bounds.x + bounds.w - tbar.w]`，`y ∈ [bounds.y, bounds.y + bounds.h - tbar.h]`

### App.vue plumbing

```ts
state.screens = p.screens.map(s => ({ x: s.x - p.origin_x, y: s.y - p.origin_y, w: s.w, h: s.h }));

const toolbarBounds = computed(() => {
  if (!state.selection) return null;
  const overlayFallback = { x: 0, y: 0, w: state.width, h: state.height };
  return findActiveScreen(state.selection, state.screens, overlayFallback);
});

// placeToolbar(state.selection, tbar, toolbarBounds.value)
```

## 不在範圍內

- **Magnifier 多螢幕**：放大鏡的可見性檢查也用 `state.width/height`，理論上同樣問題。但這次只解 toolbar；放大鏡單獨處理（使用者沒抱怨）。
- **`resizeRect` clamp**：仍保留為未來改進。
- **多 DPI 螢幕**：使用者主螢幕 DPI=96（CSS px == 物理 px），先不處理混合 DPI。`ScreenInfo.scale` 暫時不用。
- **selection 跨螢幕**：以中心點所在螢幕為主，不處理「平均」或「最大重疊」。

## 驗證

### Unit tests
1. **`findActiveScreen.test.ts`**（新檔）
   - selection 中心在某螢幕內 → 回該螢幕
   - selection 中心在 dead zone（沒有任何螢幕）→ fallback
   - selection 中心剛好在螢幕邊界 → deterministic（用 `>=` 與 `<` 嚴格判斷）

2. **`toolbarPlacement.test.ts`**（更新）
   - 既有 5 個 test：把 `overlay = { w, h }` 改成 `bounds = { x: 0, y: 0, w, h }`，行為保持
   - 新增：bounds 不從 0,0 開始（multi-screen 情境）→ toolbar 應 clamp 在 bounds 內
   - 新增：選區把 bounds 上下都塞滿 → fallback inside，y 在 bounds 範圍內

### 手動驗證
1. 在主螢幕選一個小選區 → toolbar 在主螢幕內
2. 在副螢幕選 → toolbar 在副螢幕內
3. 在主螢幕底邊附近選大選區，下面塞不下 → toolbar fallback inside，仍在主螢幕內
4. 跨螢幕拉選區（中心在主螢幕）→ toolbar 在主螢幕內

## 風險

- **中**。改動三處（helper、placement、App.vue），但介面清晰，每處都可獨立測試。
- 有原 5 個 test 鎖住單螢幕行為，新測試覆蓋多螢幕，回歸風險可控。
