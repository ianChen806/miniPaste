# Floating Pin Feature — Design

**Date:** 2026-05-13
**Status:** Design approved by user, pending implementation plan

## Summary

Add a Snipaste-style "paste pin" feature to MiniPaste. When the user presses a dedicated hotkey, MiniPaste reads the clipboard, materializes its contents as an image, and spawns an always-on-top floating window at the cursor position. The pin window is draggable, OS-native resizable, and closable via right-click or double-click. Multiple pins coexist as independent OS windows. Supported clipboard sources: bitmap images, plain text (rendered to image), and file paths pointing to image files.

## Goals

- Provide one-key clipboard-to-pin workflow comparable to Snipaste's `F3`.
- Reuse MiniPaste's existing multi-window pattern; no new dependencies on heavyweight libs.
- Keep Rust as the single source of truth for clipboard read, content classification, and window lifecycle.
- Ship MVP without restart-restore or rich text formatting.

## Non-Goals

- Restart-restore of previously pinned content (deferred — would require persistent storage and image cache).
- Pin annotation / drawing (the existing editor flow already covers that for capture sessions).
- Multi-select clipboard handling (file paths: first entry only).
- Cross-OS golden-image text rendering tests.

## User Preferences (resolved in brainstorming)

| Decision | Choice |
|---|---|
| Trigger | Dedicated hotkey, configurable in Settings |
| Content types | Image (CF_DIB / PNG), plain text → image, file path if image file |
| Pin behaviors | Always-on-top, draggable, resizable, multiple coexisting, right-click/double-click to close |
| Restart restore | Not in MVP |
| Spawn position | At cursor |
| Architecture | One Tauri webview window per pin, independent bundle |
| Text-to-image renderer | Rust-side with `ab_glyph` and an embedded font |

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                       Rust (lib.rs / tray)                    │
│                                                               │
│   hotkey "paste-pin" ──▶ pin_service::paste_from_clipboard()  │
│                              │                                │
│        ┌─────────────────────┴───────────────────────┐        │
│        ▼                                             ▼        │
│   clipboard::read_paste_content()         text_to_image::render()  │
│   (image / text / file path)              (text → PNG bytes)  │
│        │                                                      │
│        ▼                                                      │
│   pin_service::spawn_pin(png_bytes, cursor_pos)               │
│        │                                                      │
│        ▼ WebviewWindowBuilder::new(label="pin-{n}")           │
│   ┌────────────────────────────────────────┐                  │
│   │  Pin Window N (always-on-top,          │                  │
│   │   decorations: false, transparent)     │                  │
│   │  loads pin.html ───▶ shows <img>       │                  │
│   │  IPC: pin_close                        │                  │
│   └────────────────────────────────────────┘                  │
└───────────────────────────────────────────────────────────────┘
```

**Principles**

- **Rust is single source of truth.** Clipboard read, decode, text rasterization, window lifecycle all live in Rust. Frontend pin window renders one `<img>` plus minimal interaction wiring.
- **Pin windows are created dynamically**, not declared in `tauri.conf.json`. Label format: `pin-{counter}` (counter from `pin::registry`).
- **Image payload is delivered via `initialization_script`**, not via `emit` after window creation. This sidesteps the listener-not-yet-registered race the project hit during the capture-pipeline ACL bug.
- **Each pin is its own webview window** for OS-level always-on-top, drag, resize guarantees and isolation between pins. The complexity of per-region click-through on a shared overlay is not worth the RAM savings for the expected 1-3 typical pins.

## Components

### Rust modules

```
src-tauri/src/
├── pin/                            NEW
│   ├── mod.rs                      module surface
│   ├── service.rs                  paste_from_clipboard, spawn_pin
│   ├── text_to_image.rs            text → PNG bytes
│   └── registry.rs                 active pin labels, id counter
├── clipboard/
│   ├── mod.rs                      Clipboard trait: ADD read_paste_content()
│   └── windows.rs                  impl read (image / text / FileList[0])
├── ipc/
│   └── commands.rs                 ADD pin_close
├── config/model.rs                 Config: ADD paste_pin_hotkey
└── lib.rs                          setup: register paste-pin hotkey + dispatch
```

| Module | Responsibility |
|---|---|
| `clipboard::read_paste_content` | Returns enum `PasteContent::{Image(Vec<u8>), Text(String), FilePath(PathBuf), Empty}`. Tries image → text → FileList[0] in that order on Windows. |
| `pin::service::paste_from_clipboard` | Reads clipboard, branches by content type, calls `text_to_image::render` or `fs::read` as needed, derives final PNG bytes, queries cursor position, calls `spawn_pin`. |
| `pin::service::spawn_pin` | Parses PNG dimensions, clamps window size to 80% of containing screen, picks label from `registry::next_id()`, encodes bytes as base64, injects `window.__pinData` via `initialization_script`, calls `WebviewWindowBuilder.build()` with `always_on_top: true`, `decorations: false`, `transparent: true`, `skip_taskbar: true`, `resizable: true`. |
| `pin::text_to_image::render(text, defaults)` | Renders text to PNG using `ab_glyph`. Font strategy: on Windows, load Microsoft YaHei (`C:\Windows\Fonts\msyh.ttc`) at startup for full CJK + Latin coverage; on other platforms, fall back to an embedded DejaVu Sans (~300 KB). Defaults: dark background `#1f1f1f`, foreground `#e5e5e5`, font size 14 px, padding 12 px, no wrap, line-cap at 200 lines. Returns PNG bytes. |
| `pin::registry` | `next_id() -> u32`, `insert(label)`, `remove(label)`, `len() -> usize`. Atomic counter + Mutex<HashSet<String>>. Caps active pins at 30. |

### Frontend

```
src/windows/pin/                    NEW
├── App.vue                         <img>, drag / close handlers
└── pin.css                         transparent body, cursor styling
pin.html                            entry, data-window="pin"
src/main.ts                         add case "pin": import pin/App.vue
```

Pin window behavior:

1. On mount, read `window.__pinData` and bind to `<img src="data:image/png;base64,...">`.
2. `mousedown` on body → `getCurrentWebviewWindow().startDragging()` (OS-native window drag).
3. `dblclick` or `contextmenu` on body → `invoke('pin_close', { label: window.__pinData.label })`.
4. Resize is delegated to the OS window chrome; no JS involvement.

### Config and Settings

```rust
pub struct Config {
    pub schema_version: u32,
    pub hotkey: String,
    pub paste_pin_hotkey: String,    // NEW, default "Ctrl+Shift+V"
    pub default_save_path: PathBuf,
    pub image_format: ImageFormat,
    pub jpeg_quality: u8,
}
```

- `schema_version` stays at `1`. The new field is additive with a default; older configs deserialize cleanly via Serde's `#[serde(default)]`.
- Settings UI gains one `HotkeyRecorder` row for "Paste pin hotkey". The same conflict-detection path as the existing capture hotkey applies.

### Capabilities

Add to `src-tauri/capabilities/default.json` `windows` array: `"pin-*"` glob if Tauri ACL supports it, otherwise update to `"main"` capability with pre-registered windows plus a dedicated `pin` capability file that uses `windows: ["pin-*"]`. Verify glob support during implementation; if unsupported, fall back to widening permissions to all windows by omitting `windows` field.

## Data Flow

```
[1] User presses Ctrl+Shift+V (paste-pin hotkey)
       │
[2] hotkey/listener.rs background thread receives event;
    distinguishes capture vs paste-pin by HotKey id
       │
[3] handle_paste_pin_hotkey(&app):
       app.emit("internal://paste-pin", ())
       │
[4] lib.rs setup listener invokes:
       pin::service::paste_from_clipboard(&app)
       │
[5] clipboard.read_paste_content() → PasteContent
       │
       ├── Image(png_bytes)      → use as-is
       ├── Text(s)               → text_to_image::render(s) → png_bytes
       ├── FilePath(p)           → ext check → fs::read(p) → decode → re-encode PNG
       └── Empty                 → emit "pin-error"("剪貼簿是空的"), stop
       │
[6] spawn_pin(app, png_bytes, cursor_pos)
       - parse dimensions, clamp to 80% of screen
       - label = "pin-{registry.next_id()}"
       - inject window.__pinData via initialization_script
       - WebviewWindowBuilder.build()
       │
[7] Pin window loads pin.html → App.vue mounts → reads __pinData → renders <img>
       │
[8] User interactions:
       - mousedown → startDragging()
       - corner drag → OS-native resize
       - dblclick / contextmenu → invoke("pin_close", { label })
       │
[9] commands::pin_close → app.get_webview_window(label).close()
       - CloseRequested fires; lib.rs intercept allows close for labels starting "pin-"
       - registry.remove(label)
```

### IPC contract

| Direction | Channel | Payload | Purpose |
|---|---|---|---|
| Rust → frontend | event `pin-error` | `{ reason: string }` | Toast on clipboard empty / unsupported / file missing. Delivered to `editor` window's Toast component. |
| Frontend → Rust | command `pin_close` | `{ label: string }` | Pin self-closes on dblclick / right-click. |
| Init script | `window.__pinData` | `{ image_b64: string, width: number, height: number, label: string }` | One-shot payload to a freshly created pin window. |

### Why initialization_script instead of emit

Pin windows are created on demand. There is a 50-200 ms window between `WebviewWindowBuilder.build()` returning and the webview having registered its `listen` handlers. An immediate `emit("pin-ready", payload)` after build would be dropped if the listener isn't ready yet. This race is exactly the class of bug that broke the capture flow (see `git log -1 d82ed56`). `initialization_script` runs before any HTML parses, so `window.__pinData` is guaranteed present when `App.vue` mounts.

## Error Handling

| Scenario | Handling |
|---|---|
| Clipboard empty / unsupported format | `PasteContent::Empty` → `pin-error("剪貼簿是空的")` toast, no pin |
| File path points to non-image extension | `pin-error("不是圖片：xxx.txt")`, no pin |
| File path does not exist / read fails | `pin-error("找不到檔案：path")`, no pin |
| Image decode failure (corrupt PNG) | `pin-error("圖片格式無法解析")`, no pin |
| Text trims to empty | Same as clipboard empty |
| Content larger than screen × 0.8 | Clamp window size to that bound |
| Content larger than 50 MP | Reject: `pin-error("內容過大")` |
| Text longer than 200 lines | Truncate to 200, append "⋯（截斷）" to last line. Not an error |
| `WebviewWindowBuilder.build()` fails | `tracing::error!` + `pin-error("無法建立視窗")`. No retry |
| Active pin count exceeds 30 | Reject before spawn: `pin-error("Pin 上限 30 個")` |
| Hotkey conflict at startup | Reuse existing `hotkey-conflict` path. Side fix: also log it via `tracing::warn!` so the failure is visible without DevTools |
| `cursor_position()` fails | Fallback to primary screen center |
| Pin window dragged off-screen by user | Not clamped. User's choice. |

`pin-error` is delivered to the `editor` window's Toast (already wired). MVP does not introduce a dedicated notification window; revisit if it becomes intrusive.

## Testing

### Rust unit tests

| Module | Coverage |
|---|---|
| `clipboard::read_paste_content` | Mock backend trait, verify all four enum branches |
| `pin::text_to_image::render` | Output is non-empty PNG, decoded dimensions match expected bounds, line-cap fires for >200-line input |
| `pin::registry` | next_id monotonic, insert/remove/len, 30-pin cap enforced |
| `pin::service::paste_from_clipboard` | Not unit-tested. Touches AppHandle too heavily; covered by manual checklist |

### Frontend unit tests (vitest)

| File | Coverage |
|---|---|
| `windows/pin/App.test.ts` | Mount with mocked `window.__pinData`, assert `<img>` src is data URL and dimensions applied |
| dblclick / contextmenu | Mock `invoke('pin_close')`, assert called with correct label |

### Manual checklist additions to `docs/manual-test-checklist.md`

```
## Paste Pin
- [ ] Hotkey (default Ctrl+Shift+V) pins clipboard image
- [ ] Hotkey pins clipboard text as image
- [ ] Hotkey pins clipboard file path when file is PNG/JPG/GIF/BMP
- [ ] Empty clipboard → toast, no pin
- [ ] Non-image file path → toast, no pin
- [ ] Pin spawns at cursor position
- [ ] Pin is always-on-top
- [ ] Drag pin by body
- [ ] Resize pin by OS corner drag
- [ ] Right-click closes pin
- [ ] Double-click closes pin
- [ ] Multiple pins coexist; closing one leaves others alone
- [ ] Spawn 5+ pins → all responsive, RAM < 300 MB total
- [ ] Settings: change paste-pin hotkey → new hotkey works, old does not
```

### Out of scope

- Playwright E2E: dynamic Tauri webview windows are not in Playwright's reach.
- Cross-OS golden image: skipped; anti-aliasing variance not worth chasing.
- Stress tests beyond the 30-pin registry cap.

## Open Questions

None blocking implementation. Items deferred to follow-up:
- Restart restore for pins (would need persistent metadata + image cache).
- Snipaste's opacity adjustment (F + scroll) is not in MVP; revisit if requested.
- Whether to expose pin defaults (bg color, font size, padding) as user-configurable settings. MVP locks them.
