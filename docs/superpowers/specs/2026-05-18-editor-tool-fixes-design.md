# Editor Tool Fixes — Design

**日期**：2026-05-18
**範圍**：兩個 editor 互動 bug。`src/shared/editor/state/shapes.ts`、`src/shared/editor/canvas/Stage.vue`。

## 問題

### Bug 1：tool 跨 capture 持續

`editorState.tool` 是 reactive 全域，跨 capture session 不會 reset。使用者上次選了 rect，這次 capture 開起來預設仍是 rect，而非剛加的 Move tool。

預期：每次 capture 開啟時都預設 `"move"`（toolbar 最左邊）。

### Bug 2：shape 阻擋繪圖

`Stage.vue:199-206`（上次加 Move tool 時引入的 stopPropagation 區段）在 shape 命中時無條件 return，**沒判斷當前 tool**。所以：

1. 切 rect 畫了一個方框
2. 切其他 draw tool（或同 tool），想在方框 **內部** 起點畫第二個 shape
3. mousedown 命中方框 → stopPropagation+return → drafting 沒啟動 → 無法畫

附帶：annLayer.on("click") 在非 text/mosaic 時會把命中的 shape 套上 transformer，使用者「想畫卻變成選中前一個 shape」。

預期：繪圖 tool 啟用時，既有 shape 是 pass-through；只有 Move tool 啟用時才可選+可拖。

## 解法

### Bug 1
`shapes.ts` 的 `resetEditor()` 加一行 `editorState.tool = "move"`。

`resetEditor()` 在 `App.vue` 的 `capture-ready` handler 裡會被呼叫（line 118），所以每次 capture 都自動重設。

### Bug 2
用 Konva 的 layer-level `listening()` 一刀切：當前 tool 不是 "move" 時，整個 `annLayer.listening(false)`，所有 shape 自動 pass-through。

實作：
- `onMounted` 結尾，annLayer 建立後加 `annLayer.listening(editorState.tool === "move");`（初始值）
- 加 `watch(() => editorState.tool, ...)` 反映後續變化：
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
- 切離 move 時順手清掉 transformer 與 `selectedId`，避免畫面殘留 transformer handles

### 為什麼用 layer-level listening

- Stage.vue 既有的 mousedown「shape hit → stopProp+return」只在 `listening=true` 時會 fire — 自動跟 move tool 行為對齊
- `annLayer.on("click")` 的 select-shape 邏輯也只在 listening=true 時 fire — 自動跟 move tool 綁定
- 不用改 `drawTools.ts`（shape 各自的 `draggable` 屬性保持原樣）
- 不用改 `App.vue`
- 不用改 toolbar UI
- 兩個 bug 加起來只動 2 檔、約 6 行

## 檔案改動

| 檔案 | 改動 |
|------|------|
| `src/shared/editor/state/shapes.ts` | `resetEditor()` 加 `editorState.tool = "move"` |
| `src/shared/editor/canvas/Stage.vue` | onMounted 初始 listening；watch tool 切換 listening + 清 transformer |

## 不在範圍

- 改 shape 的 `draggable` 屬性 — 用 layer-level listening 更乾淨
- 改 App.vue mousedown — 不需要
- 改 toolbar UI — 不需要
- 「畫完自動切回 Move」之類的工作流變化 — 維持當前行為（畫完留在 draw tool 可連續畫）

## 驗證

### Unit tests

- 既有 41 個 test 應仍全綠（核心邏輯沒改）
- 無新增 unit test：tool reset 是 1 行 state 變更，layer listening 是 Konva 互動行為，需手動驗證

### 手動驗證

1. **Tool reset**：capture → 進編輯 → 切 rect 畫一個 → Esc 取消 → 再次 capture → toolbar 應預設 Move active
2. **疊畫**：切 rect 畫一個方框 → 在方框 **內部** 起點按住拖到框外，畫第二個 rect → 應該能畫；方框不動
3. **Move tool 拖 shape 仍可用**：畫完切回 Move → 點方框 → 可選+可拖
4. **跨 tool 切換**：Move 下選一個 shape → 切 rect → transformer handles 應該消失（無殘留）
5. **連續畫**：切 rect 連畫三個方框（每次起點都在前一個方框內）→ 全部都該畫得出來

## 風險

- **低**。改動小且局部，沒有跨檔案介面變化。
- 主要風險點：watch tool 切換時 transformer 清理時機是否正確。如果 transformer 沒清乾淨，畫面會看到孤立的 handles，但不影響功能。手動驗證涵蓋這個 case。
