# Toolbar 超出畫面範圍時點不到 — Design

**日期**：2026-05-15
**範圍**：`src/windows/overlay/toolbarPlacement.ts`

## 問題

Editing 階段使用者拖選區 handle 將選區延伸超出 overlay（特別是底邊）時，`placeToolbar` 的 `inside` fallback 算出來的 y 會落在視窗外，導致 floating toolbar 完全點不到。

### Root Cause

`placeToolbar` 對 x 有 clamp（`Math.max(0, Math.min(desiredX, overlay.w - toolbar.w))`），但 y 沒有對應的保護：

```ts
// 現況
} else {
  orientation = "inside";
  y = selection.y + selection.h - toolbar.h - gap;
}
```

當 `selection.y + selection.h > overlay.h`（選區底超出畫面），`y` 也會跟著超出。x 不會出這個問題是因為 `resizeRect` 的 east handle 雖然不 clamp，但 toolbar x 已經有 clamp 兜底。

`resizeRect` 不限制 selection 是否超出 overlay，所以 selection 本身可能超界——但這次只修 toolbar 端。

## 解法

對 toolbar y 加上跟 x 對稱的 clamp。

```ts
// 預期改法
const x = Math.max(0, Math.min(desiredX, overlay.w - toolbar.w));
const clampedY = Math.max(0, Math.min(y, overlay.h - toolbar.h));
return { x, y: clampedY, orientation };
```

統一在 return 前 clamp 一次，三個 orientation 都受惠（below/above 雖然條件本來就確保在範圍內，但統一處理較不易出錯，也對未來修改更安全）。

## 不在範圍內

- **`resizeRect` 加 clamp**：保留為未來改進（option B）。這次只解 toolbar 端的痛點。
- **toolbar 改為可拖 / 固定角落**：另一個設計題，不在這次範圍。
- **App.vue / CSS / 其他 overlay 邏輯**：不動。

## 驗證

### 既有測試
`src/__tests__/toolbarPlacement.test.ts` 5 個 case 須全數仍通過。

### 新增測試
1. **選區底邊超出 overlay**：`sel = { x: 100, y: 50, w: 400, h: overlay.h + 200 }` → toolbar y 應該 ≤ `overlay.h - toolbar.h`，且 ≥ 0。
2. **選區覆蓋全畫面**：`sel = { x: 0, y: 0, w: overlay.w, h: overlay.h }` → orientation 為 `inside`，toolbar y = `overlay.h - toolbar.h - 8`（gap），仍可見可點。

### 手動驗證
1. 啟動程式，hotkey 觸發 capture
2. 拉一個小選區（例如螢幕左上角 200x200）
3. 拖 SE handle 把選區拉到超出畫面右下
4. 檢查 toolbar 仍在畫面內、可點

## 風險

- **極低**。改動限縮在一個 pure function 的 return 計算，不影響 IPC、state、畫面其他元素。
- 既有測試覆蓋現有行為，新增測試覆蓋本次修補的 case。
