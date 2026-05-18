# Move Selection Tool — Design

**日期**：2026-05-18
**範圍**：editor toolbar 新增 Move 工具，當作預設工具，讓使用者進到 editing phase 後直接可以拖動選區框。

## 問題

進到 editing phase 時，預設工具是 `"rect"`（畫長方形）。如果使用者只是想微調選區位置（例如剛剛框得不夠精準），目前必須去拖 resize handle 一格一格調整，或重新框（reframe）。沒有「直接整塊平移選區」的方法。

## 解法

在 toolbar 加一個 Move 工具，當作預設選取的工具。Move 工具激活時，在選區內 mousedown 拖動會平移選區框；放開即定位。背景與既有註解不動，使用者看到的是「框框滑過螢幕截圖、露出不同區域」的效果。

### 行為定義

| 動作 | tool === "move" | tool === draw/text |
|------|-----------------|---------------------|
| 在選區內空白處拖 | **平移選區** | 開始畫 shape |
| 在 shape 上拖 | 拖 shape（Konva 既有） | 拖 shape（Konva 既有） |
| 在 handle 上拖 | resize 選區 | resize 選區 |
| 在選區外 click | reframe | reframe |

Move 不做邊界 clamp，與既有 resize 行為一致；選區可被拖到 active screen 外。整治留給未來。

註解不跟選區一起平移。註解是「畫在背景上」的概念，選區框離開後註解仍在原位（可能掉出框外）。

### 衝突處理

Stage（Konva）在選區內部處理 mousedown；同時 DOM 事件冒泡到 App.vue。若 tool === "move" 且點到 shape，App.vue 不該也觸發選區平移（會造成 shape 跟選區同時動）。

解法：`Stage.vue` mousedown 偵測到命中 shape（`e.target.id()` 有值）時呼叫 `e.evt.stopPropagation()`，阻止冒泡到 App.vue。

- click 空白處：Stage mousedown fires（tool != draw 所以不開始 draft，也不 stopPropagation）→ 冒泡到 App.vue → 觸發 move
- click shape：Stage mousedown fires → stopPropagation → App.vue 不 fire → 只有 Konva 的 shape drag 作用
- click handle：handle 在 `.editing-frame` 裡，不在 stage-clip 內 → Stage 不 fire → 只有 App.vue 處理 resize

### Cursor

Move tool 激活且 hover 在選區內 → cursor 改 `move`（四方箭頭）。Hover 在 handle 上仍用對應的 resize cursor（`nwse-resize` 等，由 `cursorForHandle` 提供）。

實作：在 `App.vue` 對 `.stage-clip` 動態套 cursor class，或在 mousemove 時設 `el.style.cursor`。

## 檔案改動

| 檔案 | 改動 |
|------|------|
| `src/shared/types.ts` | `ToolType` 加 `"move"` |
| `src/shared/editor/state/shapes.ts` | `editorState.tool` 預設改 `"move"` |
| `src/shared/editor/ui/Toolbar.vue` | tools 陣列最前面加 `{ key: "move", label: "✥" }` |
| `src/shared/editor/canvas/Stage.vue` | mousedown 命中 shape 時 stopPropagation |
| `src/windows/overlay/App.vue` | mousedown 新增 `hit === "move" && editorState.tool === "move"` 分支；cursor 動態切換 |

Icon `✥` 是 placeholder，視覺確認後可換。

## 不在範圍

- **Move 與 resize 的 clamp 整治** — 跟既有 resize 行為一致，先不動。未來如果決定 clamp 到 active screen，兩個 handle path 一起改。
- **註解跟選區綁定的進階模式** — 設計時討論過「shape 跟選區一起動」的選項，否決了。
- **icon 美術** — 用 placeholder 字元。

## 驗證

### Unit tests
- `handles.test.ts` 既有測試已涵蓋 `resizeRect(rect, "move", delta)` 的位置平移行為。不新增 unit test（核心邏輯沒變，只是 UI dispatch 改）。

### 手動驗證
1. **預設 tool**：trigger capture → 拉選區 → 進 editing → Toolbar 上 Move 按鈕應為 active
2. **平移選區**：在選區內空白處按住拖 → 選區框滑動，背景不動，露出不同區域
3. **平移後切 rect**：切到 rect 工具，在選區內畫框 → 應該正常畫圖，不會觸發 move
4. **拖 shape（move tool）**：先用 rect 畫一個框，切回 move tool，拖 shape → shape 自己動，選區不動
5. **Handle resize**：tool=move 時拖 handle → 仍可 resize（不被新邏輯擋掉）
6. **Reframe**：tool=move 時點選區外 → 仍進入 reframe
7. **Cursor**：tool=move + hover 選區內 → 看到 move cursor

## 風險

- **低-中**。改動跨 5 檔但每處小；最大風險是 mousedown 事件路徑的衝突，靠 stopPropagation 與 tool 條件分支處理。手動驗證涵蓋三條互動路徑（move / draw / resize）。
