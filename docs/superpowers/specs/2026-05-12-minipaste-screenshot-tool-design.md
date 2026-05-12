# minipaste — 螢幕截圖工具設計

**Status**: Draft (awaiting user review)
**Date**: 2026-05-12
**Platform scope**: Windows 10/11 x64 (MVP). 架構保留 Mac 擴充口，但 Mac 實作占位 `unimplemented!()`。

---

## 1. 目標與範圍

一個常駐桌面的截圖工具，提供：

1. 全域快捷鍵觸發
2. 跨虛擬桌面拖選範圍
3. 對選取結果用線條 / 方形 / 箭頭 / 馬賽克 / 文字標註
4. Undo / Redo、5 色色板、3 階粗細
5. 三種輸出動作：**Copy**（圖片進剪貼簿）/ **Save**（彈窗選位置）/ **Save + Copy**（存到預設路徑且檔案路徑進剪貼簿）
6. 背景常駐，Windows 縮到 system tray；左鍵單擊開設定，右鍵叫出 menu

非範圍（明確不做）：

- 截圖歷史 / 縮圖回顧
- 開機自啟動（之後可以加，不在 MVP）
- OCR、QR code 解碼、滾動截圖、錄影
- 雲端上傳
- 多語系（先繁中）

---

## 2. 技術棧

| 角色 | 選擇 |
|---|---|
| Shell | Tauri 2.x |
| 後端 | Rust |
| 前端 | Vue 3 + TypeScript + Vite |
| Canvas | Konva.js |
| 樣式 | 不指定，預期 CSS modules 或 Tailwind（plan 階段決定）|
| 測試 | `cargo test` / Vitest / Playwright (Tauri WebDriver) |

**關鍵 crate**：
- `tauri` — shell + tray
- `global-hotkey` — 全域快捷鍵
- `screenshots` — 跨平台截圖
- `arboard` — image clipboard
- `clipboard-win` — Windows `CF_HDROP`（檔案路徑剪貼簿，Win cfg 區塊）
- `tracing` + `tracing-appender` — 日誌
- `serde` / `serde_json` — 設定檔
- `thiserror` — 錯誤型別

---

## 3. 高層架構

### 3.1 四個窗口

| 窗口 | 角色 | 生命週期 |
|---|---|---|
| tray host | 主程序、tray 圖示、hotkey 監聽、協調其他窗口 | 程式啟動到結束 |
| overlay | 全虛擬桌面、無邊框、半透明、滑鼠拖選 | hotkey 觸發到選取完成 |
| editor | 顯示截圖 + Konva canvas + toolbar + 動作列 | 選取完成到動作執行 |
| settings | 一般窗口、三項設定 | tray 點擊到使用者關閉 |

### 3.2 Rust 模組 (`src-tauri/src/`)

```
src/
├── main.rs                 # tauri::Builder 入口
├── state.rs                # tray host state machine: Idle | Capturing | Editing
├── hotkey/
│   ├── mod.rs              # trait HotkeyService
│   ├── windows.rs          # global-hotkey impl
│   └── macos.rs            # unimplemented!()
├── capture/
│   ├── mod.rs              # trait Capture { fn virtual_desktop() }
│   ├── windows.rs          # screenshots crate impl
│   └── macos.rs            # unimplemented!()
├── clipboard/
│   ├── mod.rs              # trait Clipboard { write_image, write_file_paths }
│   ├── windows.rs          # arboard + clipboard-win
│   └── macos.rs            # unimplemented!()
├── tray/
│   ├── mod.rs              # trait TrayService
│   ├── windows.rs          # Tauri tray impl
│   └── macos.rs            # unimplemented!()
├── fs/
│   └── save.rs             # 路徑驗證 + 寫檔 + 檔名 generator
├── config/
│   ├── model.rs            # Config struct, schema_version
│   ├── store.rs            # 讀 / 寫 / migration
│   └── defaults.rs
└── ipc/
    └── commands.rs         # Tauri command 集中
```

**平台抽象規則**：所有平台敏感模組對外只暴露 trait，呼叫端不可 `cfg(target_os)`。cfg 只能出現在 `mod.rs` 的 `pub use ::windows::*` 那行。

### 3.3 前端模組 (`src/`)

```
src/
├── windows/
│   ├── overlay/
│   │   ├── App.vue
│   │   └── selection.ts        # 拖選邏輯
│   ├── editor/
│   │   ├── App.vue
│   │   ├── canvas/             # Konva 整合
│   │   │   ├── Stage.vue
│   │   │   ├── shapes/         # 五種 shape 各一檔
│   │   │   └── transformer.ts
│   │   ├── state/
│   │   │   ├── shapes.ts       # Shape[] reactive store
│   │   │   └── history.ts      # Undo/Redo snapshot stack
│   │   └── ui/
│   │       ├── Toolbar.vue
│   │       └── ActionBar.vue
│   └── settings/
│       ├── App.vue
│       └── HotkeyRecorder.vue
├── shared/
│   ├── ipc.ts                  # invoke / listen wrapper, typed
│   ├── types.ts                # Shape, Config, FinishAction 等
│   └── colors.ts               # 5 色 / 3 粗細 mapping 表
└── main.ts                     # 依 window label 載對應 App
```

---

## 4. 主流程 Data Flow

```
[使用者按 hotkey]
     │
     ▼
Rust hotkey 監聽 → state.transition(Idle → Capturing)
     │
     ▼
capture::virtual_desktop() → RGBA + 各螢幕邊界 + DPI
     │
     ▼
開 overlay 窗口, emit "capture-ready" (含 base64 thumbnail + 螢幕資訊)
     │
     ▼
[使用者拖選] mousedown → mousemove → mouseup
     │
     ▼
invoke "selection_confirmed" (rect)
     │
     ▼
Rust 裁切原圖 → 存進 in-flight capture slot (Mutex<Option<Cropped>>) → state.transition(Capturing → Editing)
     │
     ▼
關 overlay, 開 editor, emit "editor-ready" (含 cropped base64)
     │
     ▼
[使用者編輯] 工具 / Undo/Redo / 顏色粗細
     │
     ▼
[使用者按動作鈕]
     │
     ├── Copy        → stage.toCanvas → blob → invoke "finish_action" {kind: CopyImage, bytes}
     ├── Save        → invoke "pick_save_path" → 取得路徑 → invoke "finish_action" {kind: Save, bytes, path}
     └── Save + Copy → 直接 invoke "finish_action" {kind: SaveAndCopyPath, bytes}（路徑後端產）
     │
     ▼
Rust 執行對應 IO → 關 editor → state.transition(Editing → Idle)
```

### 4.1 截圖時機

在 overlay **開啟前**抓完整個 virtual desktop——避免 overlay 自己進到截圖。

### 4.2 裁切位置

在 Rust 端裁切，避免大圖 base64 來回傳。Overlay 收 thumbnail（縮過），editor 收全解析度。

### 4.3 DPI 處理

`screenshots` 回傳的螢幕資訊含 scale factor；前端用邏輯座標，Rust 用實體座標，IPC 邊界統一換算。

### 4.4 取消路徑

- overlay `Esc` → emit `selection_cancelled` → 關 overlay, 不開 editor
- editor `Esc` → 直接關 editor, 不執行動作
- 重複按 hotkey：非 `Idle` state 忽略

---

## 5. 編輯器內部（Konva）

### 5.1 Stage 結構

```
Stage
├── Layer "background"   ← 背景截圖, 鎖死不動
├── Layer "annotations"  ← Shape[] 1:1 渲染
├── Layer "preview"      ← 拖曳中的暫態 shape
└── Layer "ui"           ← Transformer 手柄、選取框
```

### 5.2 Shape Data Model

```ts
type ToolType = 'line' | 'rect' | 'arrow' | 'mosaic' | 'text'
type Color = 'red' | 'orange' | 'yellow' | 'green' | 'blue'
type Thickness = 'thin' | 'medium' | 'thick'  // 2px / 4px / 8px

interface Shape {
  id: string                    // nanoid
  tool: ToolType
  color: Color
  thickness: Thickness
  geometry: ShapeGeometry       // 依 tool 變化
  text?: { content: string; fontSize: number }
}

type ShapeGeometry =
  | { kind: 'line';   x1: number; y1: number; x2: number; y2: number }
  | { kind: 'rect';   x: number; y: number; w: number; h: number }
  | { kind: 'arrow';  x1: number; y1: number; x2: number; y2: number }
  | { kind: 'mosaic'; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: 'text';   x: number; y: number; w: number; h: number }
```

狀態源是 `Shape[]`，Konva 物件是 derived；Undo/Redo 操作的是這個陣列。

### 5.3 Tool State Machine

```
Idle ──選工具──> ToolSelected ──mousedown──> Drawing
   ▲                   ▲                       │
   │                   └────── mouseup ────────┤
   │                          (commit shape)   │
   │                                           │
   └──── 選工具列空 / Esc ─── Editing(shape) ◄──┘
                              (點已存在 shape)
```

`Editing` 顯示 Transformer 手柄，可拖、可改顏色粗細。

### 5.4 文字工具行為

選文字工具後：
1. 在 canvas 點擊一個位置 → 出現一個 HTML `<textarea>` 蓋在該座標（Konva 內含 `Konva.Text` 並不可編輯，所以用 HTML 元素疊上去）。
2. 輸入內容；`Enter` 換行、`Ctrl+Enter` 或點 textarea 外部 → commit。
3. commit 時：textarea 內容轉成 `Konva.Text`，textarea 移除，shape 進 `annotations` layer。
4. 雙擊已存在的 text shape → 重開 textarea 編輯該 shape。
5. 字型大小：thickness 對應 `thin=16px / medium=24px / thick=36px`；字色 = 工具列選的顏色。

### 5.5 馬賽克實作

不是覆蓋色塊，是「同位置低解析度 reblit」：
1. 從 `background` layer 對應區域 sample。
2. 縮成 `1/blockSize` 後再放大回原尺寸（最近鄰）。
3. 畫進 `annotations` layer 的 cached Group。

改大小 → 重 sample。`blockSize` 由 thickness 決定（thin=8、medium=16、thick=24）。

### 5.6 Undo / Redo

- `history: Shape[][]`，加 `pointer: number`。
- 每次 commit / delete / 屬性變更 → push snapshot, pointer 前進。
- `Ctrl+Z` pointer--；`Ctrl+Y` / `Ctrl+Shift+Z` pointer++。
- 上限 50 步。

### 5.7 UI Layout

```
┌──────────────────────────────────────────────────┐
│ [線][方][箭][馬][字]  ●●●●●  [細][中][粗]  ↶ ↷  │
├──────────────────────────────────────────────────┤
│                                                  │
│              (Konva canvas, 背景截圖)             │
│                                                  │
├──────────────────────────────────────────────────┤
│              [Copy]  [Save...]  [Save+Copy]      │
└──────────────────────────────────────────────────┘
```

### 5.8 動作按鈕行為

| 按鈕 | 行為 |
|---|---|
| Copy | 圖片 bytes 進剪貼簿，關 editor |
| Save... | 開系統檔案對話框（initial dir = 上次 Save 成功的目錄；首次用設定的預設路徑），檔名預設 `screenshot-YYYYMMDD-HHMMSS.{ext}`；確認後寫檔，關 editor |
| Save + Copy | **不彈窗**，直接寫到設定的預設路徑、檔名自動產生，路徑 (`PathBuf`) 進剪貼簿 (`CF_HDROP`)，關 editor |

`Save + Copy` 若預設路徑不存在 / 無寫權限 → toast「請到設定指定預設儲存路徑」，不執行、editor 不關。

---

## 6. 設定面板與 IPC

### 6.1 設定檔 (`%APPDATA%/minipaste/config.json`)

```json
{
  "schema_version": 1,
  "hotkey": "Ctrl+Shift+S",
  "default_save_path": "C:\\Users\\xxx\\Pictures",
  "image_format": "png",
  "jpeg_quality": 90
}
```

`schema_version` 給未來破壞性 migration 用。讀檔流程：
- 不存在 → 用 defaults 寫一份新檔
- 解析失敗 → 備份為 `config.broken.json`，用 defaults 重生，開設定面板提示
- `schema_version` 比目前高 → 用 defaults + 警告

`default_save_path` 預設為 `dirs::picture_dir()` (Windows: `~/Pictures`)。

### 6.2 設定面板 UI

```
┌─ Settings ────────────────────────────┐
│ Hotkey          [ Ctrl+Shift+S    ] ⌨ │
│ Default folder  [ C:\...\Pictures ] 📁 │
│ Format          ( ● PNG  ○ JPEG )      │
│ JPEG quality    [─────●────] 90        │
│                                        │
│ [Cancel]  [Save & Apply]               │
└────────────────────────────────────────┘
```

- Hotkey 欄位：點擊進錄製模式，捕鍵盤事件直到放開所有鍵；顯示組合或「衝突 / 無效」警告。
- 預設資料夾：點 📁 開資料夾選擇對話框。
- JPEG quality slider 只在 Format=JPEG 時顯示。
- 按 `Save & Apply` 才寫檔。Hotkey 改變後立即重新註冊；衝突 → 不寫 config + 紅字提示。

### 6.3 Tauri Commands

```rust
// 設定
#[tauri::command] fn get_config() -> Config;
#[tauri::command] fn update_config(new: Config) -> Result<(), ConfigError>;

// 截圖流程 (state machine 確保同時只有一個 capture in-flight, 不需要 ID)
#[tauri::command] fn selection_confirmed(rect: Rect) -> Result<(), CaptureError>;
#[tauri::command] fn selection_cancelled();

// 編輯動作
#[tauri::command] fn finish_action(action: FinishAction, image_bytes: Vec<u8>) -> Result<FinishOutcome, ActionError>;
#[tauri::command] fn cancel_edit();

// 對話框
#[tauri::command] fn pick_save_path(default_dir: PathBuf, default_name: String) -> Option<PathBuf>;
```

```rust
enum FinishAction {
    CopyImage,
    Save { path: PathBuf },
    SaveAndCopyPath,  // 路徑後端產, 不從前端帶
}

struct FinishOutcome {
    saved_path: Option<PathBuf>,
}
```

### 6.4 Tauri Events (Rust → 前端)

| 事件 | 對象 | Payload |
|---|---|---|
| `capture-ready` | overlay | `{ thumbnail: base64, screens: [{x, y, w, h, scale}] }` |
| `editor-ready` | editor | `{ image: base64, original_size: {w, h} }` |
| `hotkey-conflict` | settings | `{ attempted: string, reason: string }` |
| `action-complete` | tray host | `{ kind: 'copied' | 'saved' | 'save+copy' }` |

### 6.5 Tray

Windows：右下 system tray icon。
- 左鍵單擊 → 開 Settings 窗口（已開則 focus）
- 右鍵 → menu：

```
─────────────────────
 Capture        Ctrl+Shift+S
 Settings...
─────────────────────
 Quit
─────────────────────
```

Mac：menu bar icon（trait 接口已備，實作 `unimplemented!()`）。

---

## 7. 錯誤處理

### 7.1 分層

| 層 | 範圍 | 處理 |
|---|---|---|
| Rust `Result` | 後端模組 | `thiserror` enum, `?` 向上 |
| IPC 邊界 | command return | `Result<T, AppError>`，序列化成 `{ code, message, recoverable }` |
| 前端 toast | 使用者可見 | 統一 toast 元件，紅 = 失敗、綠 = 成功 |
| 致命錯誤 | tray host crash | Tauri panic handler 寫 log + 對話框 |

### 7.2 關鍵情境

| 情境 | 偵測時機 | 處理 |
|---|---|---|
| Hotkey 衝突 | 啟動 / 設定變更 | 設定面板紅字, tray icon 灰加問號 overlay |
| 預設路徑無效 | Save+Copy 觸發 | toast 提示, 不執行 |
| 截圖失敗 | capture call | retry 1 次, 失敗 toast + 取消流程 |
| 剪貼簿被佔用 | clipboard call | retry 3 次 (50ms 間隔), 失敗 toast |
| Config 損毀 | 啟動 / 讀檔 | 備份 → defaults 重生 → 提示 |
| 磁碟寫滿 | Save / Save+Copy | toast + OS error |

### 7.3 日誌

`tracing` + `tracing-appender`，寫到 `%APPDATA%/minipaste/logs/minipaste.log`，rolling daily，保留 7 天。Release build 預設 INFO。

---

## 8. 測試策略

| 層 | 範圍 | 工具 |
|---|---|---|
| Rust 單元 | `config`, `fs::save`, 純函式 | `cargo test` |
| Rust 整合 | trait mock 後測流程編排 (state.rs) | `cargo test` + mock trait |
| 前端單元 | Shape model, history stack, color/thickness mapping | Vitest |
| 前端元件 | Toolbar, Settings, HotkeyRecorder 互動 | Vitest + Vue Test Utils |
| E2E | overlay → editor → action 全流程 | Playwright + Tauri WebDriver |

### 8.1 手動測試 Checklist (每次發版前)

- [ ] 跨螢幕拖選（兩螢幕 + 不同 DPI）
- [ ] 5 工具 × 5 顏色 × 3 粗細 各畫一次
- [ ] 馬賽克在純色 / 文字 / 圖片背景的效果
- [ ] Undo/Redo 50 步邊界
- [ ] Hotkey 改成已被佔用的組合（如 `Win+E`）
- [ ] Save 對話框取消 / 選不存在的資料夾
- [ ] 預設路徑刪掉後 Save+Copy 的錯誤提示
- [ ] tray icon 在 Windows 重啟後是否還在啟動清單

---

## 9. Platform Scope 詳述

### 9.1 Windows-first 規則

- MVP build target 只打 `x86_64-pc-windows-msvc`
- CI 只跑 Windows runner
- 所有平台敏感 Rust 模組都拆 trait，Mac impl 占位 `unimplemented!()`
- 路徑用 `PathBuf` 而非 `String`
- Windows-only crate（`clipboard-win`）封進 `#[cfg(target_os = "windows")]`

### 9.2 留給 Mac 的擴充點

| 模組 | Mac 要做的事 |
|---|---|
| `hotkey/macos.rs` | 用 `global-hotkey` 的 macOS backend, 處理 Accessibility 權限請求 |
| `capture/macos.rs` | `screenshots` crate macOS, 處理 Screen Recording 權限 |
| `clipboard/macos.rs` | `arboard` (image) + `objc` 操作 `NSPasteboardTypeFileURL` |
| `tray/macos.rs` | menu bar icon, 不是 dock |
| 安裝包 | 處理 codesign / notarization |

---

## 10. 開放議題（待 plan 階段決定）

- Toolbar / Settings 的視覺風格（Tailwind vs CSS modules）
- 截圖 / tray icon 的視覺資產（誰畫、什麼風格）
- 是否使用 Tauri sidecar binary 還是純 in-process
- 安裝包格式（MSI / MSIX / portable exe）
