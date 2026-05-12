# minipaste Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立一個 Windows 桌面截圖工具：全域 hotkey 觸發、跨虛擬桌面拖選、線/方/箭/馬賽克/文字標註、三種輸出動作（Copy / Save / Save+Copy），背景常駐於 system tray。

**Architecture:** Tauri 2.x 殼 + Rust 後端 + Vue 3 前端，四個窗口（tray host / overlay / editor / settings）。Rust 端用 trait 抽象平台敏感模組（hotkey / capture / clipboard / tray），Windows 實作填滿、Mac 占位 `unimplemented!()`。前端 editor 用 Konva.js 管理 canvas 圖形。

**Tech Stack:** Tauri 2.x · Rust · Vue 3 · TypeScript · Vite · Konva.js · `global-hotkey` · `screenshots` · `arboard` · `clipboard-win` · `tracing` · `thiserror` · Vitest · Playwright

**Spec reference:** `docs/superpowers/specs/2026-05-12-minipaste-screenshot-tool-design.md`

---

## File Structure

### Rust (`src-tauri/`)

```
src-tauri/
├── Cargo.toml
├── tauri.conf.json              # 4 個 windows 設定
├── build.rs
├── icons/                       # tray + window icons
└── src/
    ├── main.rs                  # tauri::Builder, 註冊 commands/events/tray
    ├── state.rs                 # AppState { Mutex<Stage>, Mutex<Option<Cropped>>, settings cache }
    ├── error.rs                 # AppError (thiserror), 序列化成 IPC payload
    ├── hotkey/
    │   ├── mod.rs               # trait HotkeyService, pub use platform::*
    │   ├── windows.rs           # global-hotkey 註冊與回呼
    │   └── macos.rs             # unimplemented!()
    ├── capture/
    │   ├── mod.rs               # trait Capture { virtual_desktop, crop }
    │   ├── windows.rs           # screenshots crate
    │   └── macos.rs             # unimplemented!()
    ├── clipboard/
    │   ├── mod.rs               # trait Clipboard { write_image, write_file_paths }
    │   ├── windows.rs           # arboard + clipboard-win (CF_HDROP)
    │   └── macos.rs             # unimplemented!()
    ├── tray/
    │   ├── mod.rs               # trait TrayService, build_tray()
    │   ├── windows.rs           # TrayIconBuilder + menu
    │   └── macos.rs             # unimplemented!()
    ├── fs/
    │   ├── mod.rs
    │   ├── save.rs              # write_image_file, validate_writable_dir
    │   └── filename.rs          # screenshot-YYYYMMDD-HHMMSS.{ext}
    ├── config/
    │   ├── mod.rs
    │   ├── model.rs             # Config struct, ImageFormat, schema_version
    │   ├── store.rs             # load / save / migrate / backup-on-corrupt
    │   └── defaults.rs
    ├── ipc/
    │   ├── mod.rs
    │   └── commands.rs          # 全部 #[tauri::command]
    └── logging.rs               # tracing subscriber
```

### Frontend (`src/`)

```
src/
├── main.ts                      # 依 window.location.pathname 載對應 App
├── windows/
│   ├── overlay/
│   │   ├── App.vue
│   │   ├── selection.ts         # 拖選邏輯, 純函式優先
│   │   └── overlay.css
│   ├── editor/
│   │   ├── App.vue
│   │   ├── canvas/
│   │   │   ├── Stage.vue        # Konva 容器
│   │   │   ├── LineShape.ts
│   │   │   ├── RectShape.ts
│   │   │   ├── ArrowShape.ts
│   │   │   ├── MosaicShape.ts
│   │   │   ├── TextShape.ts
│   │   │   └── transformer.ts
│   │   ├── state/
│   │   │   ├── shapes.ts        # reactive Shape[] store
│   │   │   └── history.ts       # undo/redo snapshot stack
│   │   ├── ui/
│   │   │   ├── Toolbar.vue
│   │   │   ├── ActionBar.vue
│   │   │   └── Toast.vue
│   │   └── editor.css
│   └── settings/
│       ├── App.vue
│       ├── HotkeyRecorder.vue
│       └── settings.css
├── shared/
│   ├── ipc.ts                   # invoke<T>, listen<T> typed wrappers
│   ├── types.ts                 # Config, Shape, FinishAction, ScreenInfo
│   └── colors.ts                # 5-color palette + 3 thickness mapping
└── index.html                   # 共用模板, main.ts 動態 mount
```

### Frontend HTML entry points (Vite multi-page)

```
overlay.html → loads windows/overlay
editor.html  → loads windows/editor
settings.html → loads windows/settings
```

### Tests

```
src-tauri/src/**/*.rs            # #[cfg(test)] mod tests, 純函式單元測試
src-tauri/tests/                 # 整合測試 (trait mock)
src/__tests__/                   # Vitest, 與被測檔案就近
tests/e2e/                       # Playwright + Tauri WebDriver
```

---

## Phase A — Foundation (Rust skeleton + config)

### Task 1: Initialize Tauri 2 project with Vue + TypeScript

**Files:**
- Create: 整個專案根目錄結構

- [ ] **Step 1: Run Tauri create script**

```bash
cd /mnt/d/sideproject/minipaste
npm create tauri-app@latest . -- --template vue-ts --identifier dev.minipaste.app --manager npm
```

Choose: App name `minipaste`, project name `minipaste`, identifier `dev.minipaste.app`, frontend `Vue`, language `TypeScript`, package manager `npm`.

- [ ] **Step 2: Verify it builds**

```bash
npm install
npm run tauri dev
```

Expected: dev window opens showing the Tauri+Vue welcome page.

- [ ] **Step 3: Configure 4 windows in `src-tauri/tauri.conf.json`**

Edit `tauri.conf.json` `app.windows` to define four windows. `tray host` does not appear here (it's not a webview); the host is the main Rust process. Replace the default single-window entry with:

```json
"windows": [
  {
    "label": "settings",
    "title": "Minipaste Settings",
    "url": "settings.html",
    "width": 480,
    "height": 360,
    "resizable": false,
    "visible": false,
    "center": true,
    "decorations": true
  },
  {
    "label": "overlay",
    "title": "",
    "url": "overlay.html",
    "fullscreen": false,
    "decorations": false,
    "transparent": true,
    "alwaysOnTop": true,
    "visible": false,
    "skipTaskbar": true,
    "resizable": false
  },
  {
    "label": "editor",
    "title": "Minipaste Editor",
    "url": "editor.html",
    "width": 1000,
    "height": 700,
    "visible": false,
    "center": true,
    "decorations": true
  }
]
```

- [ ] **Step 4: Configure Vite multi-page entries**

Edit `vite.config.ts`:

```ts
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    rollupOptions: {
      input: {
        overlay: resolve(__dirname, "overlay.html"),
        editor: resolve(__dirname, "editor.html"),
        settings: resolve(__dirname, "settings.html"),
      },
    },
  },
});
```

Create three HTML entry points (`overlay.html`, `editor.html`, `settings.html`) each loading the corresponding bootstrap script via `<script type="module" src="/src/main.ts">`.

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: bootstrap Tauri 2 + Vue 3 + TS project with 4 windows"
```

---

### Task 2: Add Rust dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Update `[dependencies]`**

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
tracing-appender = "0.2"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
global-hotkey = "0.6"
screenshots = "0.8"
arboard = "3"
image = "0.25"
dirs = "5"
chrono = "0.4"
nanoid = "0.4"

[target.'cfg(windows)'.dependencies]
clipboard-win = "5"

[dev-dependencies]
mockall = "0.13"
tempfile = "3"
```

- [ ] **Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check
```

Expected: clean compile.

- [ ] **Step 3: Add `tauri-plugin-dialog` to JS**

```bash
cd .. && npm install @tauri-apps/plugin-dialog
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json
git commit -m "feat: add Rust + JS dependencies"
```

---

### Task 3: Scaffold platform-abstraction module structure

**Files:**
- Create: `src-tauri/src/hotkey/{mod.rs,windows.rs,macos.rs}`
- Create: `src-tauri/src/capture/{mod.rs,windows.rs,macos.rs}`
- Create: `src-tauri/src/clipboard/{mod.rs,windows.rs,macos.rs}`
- Create: `src-tauri/src/tray/{mod.rs,windows.rs,macos.rs}`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing test for module presence**

`src-tauri/tests/module_presence.rs`:

```rust
#[test]
fn platform_modules_export_aliases() {
    // Smoke test: each module re-exports a platform-specific implementation type.
    use minipaste::{hotkey, capture, clipboard, tray};
    let _: Option<hotkey::PlatformHotkey> = None;
    let _: Option<capture::PlatformCapture> = None;
    let _: Option<clipboard::PlatformClipboard> = None;
    let _: Option<tray::PlatformTray> = None;
}
```

- [ ] **Step 2: Run test, expect fail**

```bash
cd src-tauri && cargo test --test module_presence
```

Expected: `unresolved import` for crate `minipaste`.

- [ ] **Step 3: Set up lib crate**

`src-tauri/Cargo.toml` add:

```toml
[lib]
name = "minipaste"
path = "src/lib.rs"
```

`src-tauri/src/lib.rs`:

```rust
pub mod capture;
pub mod clipboard;
pub mod config;
pub mod error;
pub mod fs;
pub mod hotkey;
pub mod ipc;
pub mod logging;
pub mod state;
pub mod tray;
```

(`config`, `error`, `fs`, `ipc`, `logging`, `state` modules created as empty `mod.rs` placeholders for now — filled in later tasks.)

- [ ] **Step 4: Implement module skeletons**

For each of `hotkey`, `capture`, `clipboard`, `tray`, create a `mod.rs` like:

```rust
// src-tauri/src/hotkey/mod.rs
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::WindowsHotkey as PlatformHotkey;
#[cfg(target_os = "macos")]
pub use macos::MacosHotkey as PlatformHotkey;

pub trait HotkeyService: Send + Sync {
    fn register(&mut self, combo: &str) -> Result<(), HotkeyError>;
    fn unregister(&mut self);
}

#[derive(thiserror::Error, Debug)]
pub enum HotkeyError {
    #[error("invalid hotkey combo: {0}")]
    Invalid(String),
    #[error("hotkey already in use")]
    Conflict,
    #[error("backend error: {0}")]
    Backend(String),
}
```

Platform impls start as empty structs implementing the trait with `unimplemented!()` (Mac) or `Ok(())` returning struct (Windows; real impl in later task).

```rust
// src-tauri/src/hotkey/windows.rs
use super::{HotkeyError, HotkeyService};
pub struct WindowsHotkey;
impl WindowsHotkey { pub fn new() -> Self { Self } }
impl HotkeyService for WindowsHotkey {
    fn register(&mut self, _combo: &str) -> Result<(), HotkeyError> { Ok(()) }
    fn unregister(&mut self) {}
}
```

```rust
// src-tauri/src/hotkey/macos.rs
use super::{HotkeyError, HotkeyService};
pub struct MacosHotkey;
impl MacosHotkey { pub fn new() -> Self { Self } }
impl HotkeyService for MacosHotkey {
    fn register(&mut self, _combo: &str) -> Result<(), HotkeyError> {
        unimplemented!("macOS hotkey support deferred")
    }
    fn unregister(&mut self) {}
}
```

Repeat the same pattern for `capture` (`Capture` trait with `virtual_desktop() -> CaptureFrame`, `crop(frame, rect) -> Vec<u8>`), `clipboard` (`Clipboard` with `write_image(&[u8])`, `write_file_paths(&[PathBuf])`), `tray` (`TrayService` with `build()` that takes a Tauri `AppHandle`).

- [ ] **Step 5: Run test, expect pass**

```bash
cd src-tauri && cargo test --test module_presence
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(rust): scaffold platform abstraction modules with traits"
```

---

### Task 4: Implement `config::model` and defaults

**Files:**
- Create: `src-tauri/src/config/model.rs`
- Create: `src-tauri/src/config/defaults.rs`
- Create: `src-tauri/src/config/mod.rs`

- [ ] **Step 1: Write failing test**

`src-tauri/src/config/model.rs` bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serde_roundtrip() {
        let cfg = Config {
            schema_version: 1,
            hotkey: "Ctrl+Shift+S".into(),
            default_save_path: PathBuf::from("C:/temp"),
            image_format: ImageFormat::Png,
            jpeg_quality: 90,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn image_format_parses_lowercase() {
        let json = r#""jpeg""#;
        assert_eq!(serde_json::from_str::<ImageFormat>(json).unwrap(), ImageFormat::Jpeg);
    }
}
```

- [ ] **Step 2: Run, expect fail (undefined types)**

```bash
cd src-tauri && cargo test --lib config::
```

Expected: FAIL `unresolved import` / `cannot find type`.

- [ ] **Step 3: Implement model + defaults**

`src-tauri/src/config/model.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub hotkey: String,
    pub default_save_path: PathBuf,
    pub image_format: ImageFormat,
    pub jpeg_quality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self { Self::Png => "png", Self::Jpeg => "jpg" }
    }
}
```

`src-tauri/src/config/defaults.rs`:

```rust
use super::model::{Config, ImageFormat, CURRENT_SCHEMA_VERSION};
use std::path::PathBuf;

pub fn default_config() -> Config {
    Config {
        schema_version: CURRENT_SCHEMA_VERSION,
        hotkey: "Ctrl+Shift+S".to_string(),
        default_save_path: dirs::picture_dir()
            .unwrap_or_else(|| PathBuf::from(".")),
        image_format: ImageFormat::Png,
        jpeg_quality: 90,
    }
}
```

`src-tauri/src/config/mod.rs`:

```rust
pub mod defaults;
pub mod model;
pub mod store;

pub use model::{Config, ImageFormat, CURRENT_SCHEMA_VERSION};
pub use defaults::default_config;
```

- [ ] **Step 4: Run, expect pass**

```bash
cd src-tauri && cargo test --lib config::
```

Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/
git commit -m "feat(config): add Config model + defaults"
```

---

### Task 5: Implement `config::store` with load/save/migration

**Files:**
- Create: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/config/store.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_defaults_and_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(path.exists());
    }

    #[test]
    fn load_corrupt_backs_up_and_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{not valid json").unwrap();
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(dir.path().join("config.broken.json").exists());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = default_config();
        cfg.hotkey = "Ctrl+Alt+P".into();
        save(&path, &cfg).unwrap();
        let back = load_or_init(&path).unwrap();
        assert_eq!(back.hotkey, "Ctrl+Alt+P");
    }

    #[test]
    fn future_schema_version_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path,
            r#"{"schema_version":99,"hotkey":"X","default_save_path":".","image_format":"png","jpeg_quality":90}"#
        ).unwrap();
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
```

- [ ] **Step 2: Run, expect fail**

```bash
cd src-tauri && cargo test --lib config::store
```

Expected: FAIL — functions don't exist.

- [ ] **Step 3: Implement store**

```rust
use super::model::{Config, CURRENT_SCHEMA_VERSION};
use super::defaults::default_config;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn load_or_init(path: &Path) -> Result<Config, ConfigError> {
    if !path.exists() {
        let cfg = default_config();
        save(path, &cfg)?;
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(path)?;
    match serde_json::from_str::<Config>(&raw) {
        Ok(cfg) if cfg.schema_version <= CURRENT_SCHEMA_VERSION => Ok(cfg),
        Ok(_) | Err(_) => {
            // Corrupt or future-versioned → backup + defaults
            let backup_path = path.with_file_name("config.broken.json");
            let _ = std::fs::copy(path, &backup_path);
            let cfg = default_config();
            save(path, &cfg)?;
            Ok(cfg)
        }
    }
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn config_path(app_config_dir: PathBuf) -> PathBuf {
    app_config_dir.join("config.json")
}
```

- [ ] **Step 4: Run, expect pass**

```bash
cd src-tauri && cargo test --lib config::
```

Expected: PASS (6 tests, includes earlier model tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/store.rs
git commit -m "feat(config): add store with backup-on-corrupt + migration guard"
```

---

### Task 6: Implement `fs::filename` (timestamp generator)

**Files:**
- Create: `src-tauri/src/fs/mod.rs`
- Create: `src-tauri/src/fs/filename.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/fs/filename.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn screenshot_filename_matches_expected_format() {
        let ts = chrono::Local.with_ymd_and_hms(2026, 5, 12, 14, 30, 45).unwrap();
        assert_eq!(screenshot_filename(ts, "png"), "screenshot-20260512-143045.png");
        assert_eq!(screenshot_filename(ts, "jpg"), "screenshot-20260512-143045.jpg");
    }
}
```

- [ ] **Step 2: Run, expect fail**

```bash
cd src-tauri && cargo test --lib fs::filename
```

Expected: FAIL — function not defined.

- [ ] **Step 3: Implement**

`src-tauri/src/fs/mod.rs`:

```rust
pub mod filename;
pub mod save;
```

`src-tauri/src/fs/filename.rs`:

```rust
use chrono::{DateTime, Local};

pub fn screenshot_filename(ts: DateTime<Local>, ext: &str) -> String {
    format!("screenshot-{}.{}", ts.format("%Y%m%d-%H%M%S"), ext)
}

pub fn now_filename(ext: &str) -> String {
    screenshot_filename(Local::now(), ext)
}
```

- [ ] **Step 4: Run, expect pass**

```bash
cd src-tauri && cargo test --lib fs::filename
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/fs/
git commit -m "feat(fs): add screenshot filename generator"
```

---

### Task 7: Implement `fs::save` (path validation + write)

**Files:**
- Create: `src-tauri/src/fs/save.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validate_writable_dir_passes_on_existing_dir() {
        let dir = tempdir().unwrap();
        assert!(validate_writable_dir(dir.path()).is_ok());
    }

    #[test]
    fn validate_writable_dir_fails_on_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert!(matches!(
            validate_writable_dir(&missing),
            Err(SaveError::DirMissing(_))
        ));
    }

    #[test]
    fn write_image_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.png");
        write_image_file(&path, b"fake-bytes").unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"fake-bytes");
    }
}
```

- [ ] **Step 2: Run, expect fail**

- [ ] **Step 3: Implement**

```rust
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum SaveError {
    #[error("directory does not exist: {0}")]
    DirMissing(PathBuf),
    #[error("directory not writable: {0}")]
    NotWritable(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn validate_writable_dir(path: &Path) -> Result<(), SaveError> {
    if !path.exists() {
        return Err(SaveError::DirMissing(path.to_path_buf()));
    }
    let probe = path.join(".minipaste-write-probe");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(_) => Err(SaveError::NotWritable(path.to_path_buf())),
    }
}

pub fn write_image_file(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
```

- [ ] **Step 4: Run, expect pass**

```bash
cd src-tauri && cargo test --lib fs::
```

Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/fs/save.rs
git commit -m "feat(fs): add writable dir validation + image write"
```

---

### Task 8: Implement `clipboard/windows` — image + file paths

**Files:**
- Modify: `src-tauri/src/clipboard/mod.rs`
- Modify: `src-tauri/src/clipboard/windows.rs`

- [ ] **Step 1: Define trait + error in `mod.rs`**

```rust
#[cfg(target_os = "windows")] mod windows;
#[cfg(target_os = "macos")] mod macos;

#[cfg(target_os = "windows")]
pub use windows::WindowsClipboard as PlatformClipboard;
#[cfg(target_os = "macos")]
pub use macos::MacosClipboard as PlatformClipboard;

use std::path::PathBuf;

pub trait Clipboard {
    fn write_image(&self, png_bytes: &[u8]) -> Result<(), ClipboardError>;
    fn write_file_paths(&self, paths: &[PathBuf]) -> Result<(), ClipboardError>;
}

#[derive(thiserror::Error, Debug)]
pub enum ClipboardError {
    #[error("clipboard busy after retries")]
    Busy,
    #[error("backend error: {0}")]
    Backend(String),
}
```

- [ ] **Step 2: Implement Windows backend**

```rust
// src-tauri/src/clipboard/windows.rs
use super::{Clipboard, ClipboardError};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

pub struct WindowsClipboard;
impl WindowsClipboard { pub fn new() -> Self { Self } }

impl Clipboard for WindowsClipboard {
    fn write_image(&self, png_bytes: &[u8]) -> Result<(), ClipboardError> {
        // arboard expects raw RGBA, so decode PNG first.
        let img = image::load_from_memory(png_bytes)
            .map_err(|e| ClipboardError::Backend(e.to_string()))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        let data = arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(img.into_raw()),
        };
        retry_3(|| {
            let mut cb = arboard::Clipboard::new()
                .map_err(|e| ClipboardError::Backend(e.to_string()))?;
            cb.set_image(data.clone())
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        })
    }

    fn write_file_paths(&self, paths: &[PathBuf]) -> Result<(), ClipboardError> {
        let strings: Vec<String> = paths.iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        retry_3(|| {
            clipboard_win::set_clipboard(clipboard_win::formats::FileList, &strings)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        })
    }
}

fn retry_3<F>(mut f: F) -> Result<(), ClipboardError>
where F: FnMut() -> Result<(), ClipboardError>
{
    for attempt in 0..3 {
        match f() {
            Ok(()) => return Ok(()),
            Err(_) if attempt < 2 => sleep(Duration::from_millis(50)),
            Err(e) => return Err(e),
        }
    }
    Err(ClipboardError::Busy)
}
```

- [ ] **Step 3: Implement macOS stub**

```rust
// src-tauri/src/clipboard/macos.rs
use super::{Clipboard, ClipboardError};
use std::path::PathBuf;

pub struct MacosClipboard;
impl MacosClipboard { pub fn new() -> Self { Self } }

impl Clipboard for MacosClipboard {
    fn write_image(&self, _b: &[u8]) -> Result<(), ClipboardError> {
        unimplemented!("macOS clipboard deferred")
    }
    fn write_file_paths(&self, _p: &[PathBuf]) -> Result<(), ClipboardError> {
        unimplemented!("macOS clipboard deferred")
    }
}
```

- [ ] **Step 4: Compile check (manual smoke — no unit test, hits real clipboard)**

```bash
cd src-tauri && cargo build
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/clipboard/
git commit -m "feat(clipboard): Windows image + CF_HDROP file paths with retry"
```

---

### Task 9: Implement `capture/windows` — virtual desktop screenshot

**Files:**
- Modify: `src-tauri/src/capture/mod.rs`
- Modify: `src-tauri/src/capture/windows.rs`

- [ ] **Step 1: Define trait in `mod.rs`**

```rust
#[cfg(target_os = "windows")] mod windows;
#[cfg(target_os = "macos")] mod macos;

#[cfg(target_os = "windows")] pub use windows::WindowsCapture as PlatformCapture;
#[cfg(target_os = "macos")] pub use macos::MacosCapture as PlatformCapture;

#[derive(Debug, Clone)]
pub struct ScreenInfo {
    pub x: i32, pub y: i32, pub w: u32, pub h: u32, pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub png_bytes: Vec<u8>,        // entire virtual desktop, PNG-encoded
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,             // top-left of virtual desktop in OS coords
    pub origin_y: i32,
    pub screens: Vec<ScreenInfo>,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: i32, pub y: i32, pub w: u32, pub h: u32 }

pub trait Capture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError>;
    fn crop(&self, frame: &CaptureFrame, rect: Rect) -> Result<Vec<u8>, CaptureError>;
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("capture backend failed: {0}")]
    Backend(String),
    #[error("rect outside frame")]
    OutOfBounds,
}
```

- [ ] **Step 2: Write failing test for `crop`**

`src-tauri/src/capture/windows.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{Capture, CaptureFrame, Rect, ScreenInfo};

    #[test]
    fn crop_clips_a_rect_and_returns_png() {
        // 4x4 red image as PNG
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let frame = CaptureFrame {
            png_bytes: buf.into_inner(),
            width: 4, height: 4, origin_x: 0, origin_y: 0,
            screens: vec![ScreenInfo{x:0,y:0,w:4,h:4,scale:1.0}],
        };
        let cap = WindowsCapture::new();
        let cropped = cap.crop(&frame, Rect{x:1,y:1,w:2,h:2}).unwrap();
        let decoded = image::load_from_memory(&cropped).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
    }

    #[test]
    fn crop_out_of_bounds_errors() {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0,0,0,255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let frame = CaptureFrame {
            png_bytes: buf.into_inner(),
            width: 2, height: 2, origin_x: 0, origin_y: 0,
            screens: vec![],
        };
        let cap = WindowsCapture::new();
        assert!(matches!(
            cap.crop(&frame, Rect{x:1, y:1, w:5, h:5}),
            Err(CaptureError::OutOfBounds)
        ));
    }
}
```

- [ ] **Step 3: Run, expect fail**

- [ ] **Step 4: Implement Windows backend**

```rust
// src-tauri/src/capture/windows.rs
use super::{Capture, CaptureError, CaptureFrame, Rect, ScreenInfo};
use screenshots::Screen;

pub struct WindowsCapture;
impl WindowsCapture { pub fn new() -> Self { Self } }

impl Capture for WindowsCapture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError> {
        let screens = Screen::all()
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        if screens.is_empty() {
            return Err(CaptureError::Backend("no screens".into()));
        }
        // Compute virtual desktop bounding box
        let min_x = screens.iter().map(|s| s.display_info.x).min().unwrap();
        let min_y = screens.iter().map(|s| s.display_info.y).min().unwrap();
        let max_x = screens.iter()
            .map(|s| s.display_info.x + s.display_info.width as i32).max().unwrap();
        let max_y = screens.iter()
            .map(|s| s.display_info.y + s.display_info.height as i32).max().unwrap();
        let total_w = (max_x - min_x) as u32;
        let total_h = (max_y - min_y) as u32;

        // Allocate canvas, paste each screen capture at its offset.
        let mut canvas = image::RgbaImage::from_pixel(
            total_w, total_h, image::Rgba([0,0,0,255]));
        for s in &screens {
            let img = s.capture()
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            let rgba = image::RgbaImage::from_raw(
                s.display_info.width, s.display_info.height, img.into_raw()
            ).ok_or_else(|| CaptureError::Backend("bad screen buffer".into()))?;
            let dx = (s.display_info.x - min_x) as u32;
            let dy = (s.display_info.y - min_y) as u32;
            image::imageops::overlay(&mut canvas, &rgba, dx as i64, dy as i64);
        }

        let mut png_buf = std::io::Cursor::new(Vec::new());
        canvas.write_to(&mut png_buf, image::ImageFormat::Png)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;

        let screen_infos = screens.iter().map(|s| ScreenInfo {
            x: s.display_info.x, y: s.display_info.y,
            w: s.display_info.width, h: s.display_info.height,
            scale: s.display_info.scale_factor,
        }).collect();

        Ok(CaptureFrame {
            png_bytes: png_buf.into_inner(),
            width: total_w, height: total_h,
            origin_x: min_x, origin_y: min_y,
            screens: screen_infos,
        })
    }

    fn crop(&self, frame: &CaptureFrame, rect: Rect) -> Result<Vec<u8>, CaptureError> {
        let img = image::load_from_memory(&frame.png_bytes)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        // rect coords are in virtual-desktop space; convert to image-local
        let lx = (rect.x - frame.origin_x) as u32;
        let ly = (rect.y - frame.origin_y) as u32;
        if lx + rect.w > frame.width || ly + rect.h > frame.height {
            return Err(CaptureError::OutOfBounds);
        }
        let cropped = img.crop_imm(lx, ly, rect.w, rect.h);
        let mut buf = std::io::Cursor::new(Vec::new());
        cropped.write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        Ok(buf.into_inner())
    }
}
```

- [ ] **Step 5: Implement macOS stub** (same pattern as Task 8 stub)

- [ ] **Step 6: Run tests, expect pass**

```bash
cd src-tauri && cargo test --lib capture::
```

Expected: PASS (2 tests). `virtual_desktop` is not unit-tested (hits OS).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/capture/
git commit -m "feat(capture): Windows virtual desktop screenshot + crop"
```

---

### Task 10: Implement `hotkey/windows` — global-hotkey registration

**Files:**
- Modify: `src-tauri/src/hotkey/windows.rs`

- [ ] **Step 1: Implement**

```rust
use super::{HotkeyError, HotkeyService};
use global_hotkey::{
    hotkey::HotKey, GlobalHotKeyManager, GlobalHotKeyEvent,
};
use std::str::FromStr;

pub struct WindowsHotkey {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl WindowsHotkey {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()
                .map_err(|e| HotkeyError::Backend(e.to_string()))?,
            current: None,
        })
    }

    /// Subscribe to events. Caller polls `GlobalHotKeyEvent::receiver()` in a thread
    /// and dispatches into the tray-host state machine.
    pub fn event_receiver() -> std::sync::mpsc::Receiver<GlobalHotKeyEvent> {
        // global-hotkey 0.6 exposes a global crossbeam receiver:
        global_hotkey::GlobalHotKeyEvent::receiver().clone()
    }
}

impl HotkeyService for WindowsHotkey {
    fn register(&mut self, combo: &str) -> Result<(), HotkeyError> {
        let hk = HotKey::from_str(combo)
            .map_err(|_| HotkeyError::Invalid(combo.into()))?;
        if let Some(prev) = self.current.take() {
            let _ = self.manager.unregister(prev);
        }
        self.manager.register(hk)
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("registered") { HotkeyError::Conflict }
                else { HotkeyError::Backend(msg) }
            })?;
        self.current = Some(hk);
        Ok(())
    }

    fn unregister(&mut self) {
        if let Some(prev) = self.current.take() {
            let _ = self.manager.unregister(prev);
        }
    }
}
```

- [ ] **Step 2: Update macos.rs**

Already `unimplemented!()` from Task 3. Leave as-is.

- [ ] **Step 3: Compile check**

```bash
cd src-tauri && cargo build
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/hotkey/
git commit -m "feat(hotkey): Windows global-hotkey registration + conflict detection"
```

---

### Task 11: Implement state machine

**Files:**
- Create: `src-tauri/src/state.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_transitions_to_capturing() {
        let mut s = AppPhase::Idle;
        assert!(s.transition(PhaseEvent::HotkeyPressed).is_ok());
        assert_eq!(s, AppPhase::Capturing);
    }

    #[test]
    fn capturing_to_editing_on_selection() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::SelectionConfirmed).is_ok());
        assert_eq!(s, AppPhase::Editing);
    }

    #[test]
    fn editing_to_idle_on_finish() {
        let mut s = AppPhase::Editing;
        assert!(s.transition(PhaseEvent::ActionFinished).is_ok());
        assert_eq!(s, AppPhase::Idle);
    }

    #[test]
    fn hotkey_in_capturing_is_ignored() {
        let mut s = AppPhase::Capturing;
        assert!(matches!(s.transition(PhaseEvent::HotkeyPressed), Err(_)));
        assert_eq!(s, AppPhase::Capturing);
    }

    #[test]
    fn cancel_returns_to_idle_from_any_active_phase() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::Cancelled).is_ok());
        assert_eq!(s, AppPhase::Idle);
        let mut s = AppPhase::Editing;
        assert!(s.transition(PhaseEvent::Cancelled).is_ok());
        assert_eq!(s, AppPhase::Idle);
    }
}
```

- [ ] **Step 2: Implement**

```rust
use crate::capture::CaptureFrame;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPhase { Idle, Capturing, Editing }

#[derive(Debug, Clone, Copy)]
pub enum PhaseEvent {
    HotkeyPressed,
    SelectionConfirmed,
    ActionFinished,
    Cancelled,
}

#[derive(thiserror::Error, Debug)]
#[error("invalid transition: {from:?} → {event:?}")]
pub struct TransitionError { pub from: AppPhase, pub event: PhaseEvent }

impl AppPhase {
    pub fn transition(&mut self, ev: PhaseEvent) -> Result<(), TransitionError> {
        use AppPhase::*;
        use PhaseEvent::*;
        let next = match (*self, ev) {
            (Idle, HotkeyPressed) => Capturing,
            (Capturing, SelectionConfirmed) => Editing,
            (Editing, ActionFinished) => Idle,
            (Capturing | Editing, Cancelled) => Idle,
            (from, ev) => return Err(TransitionError { from, event: ev }),
        };
        *self = next;
        Ok(())
    }
}

/// Singleton runtime state, lives inside Tauri's `State`.
pub struct AppState {
    pub phase: Mutex<AppPhase>,
    pub capture: Mutex<Option<CaptureFrame>>,
    pub cropped: Mutex<Option<Vec<u8>>>,  // PNG bytes of selected region
    pub last_save_dir: Mutex<Option<std::path::PathBuf>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            phase: Mutex::new(AppPhase::Idle),
            capture: Mutex::new(None),
            cropped: Mutex::new(None),
            last_save_dir: Mutex::new(None),
        }
    }
}
```

- [ ] **Step 3: Run, expect pass**

```bash
cd src-tauri && cargo test --lib state::
```

Expected: PASS (5 tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "feat(state): add phase machine + AppState container"
```

---

### Task 12: Implement IPC commands skeleton

**Files:**
- Create: `src-tauri/src/ipc/mod.rs`
- Create: `src-tauri/src/ipc/commands.rs`
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Define unified IPC error**

`src-tauri/src/error.rs`:

```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    #[error("{0}")] Config(String),
    #[error("{0}")] Hotkey(String),
    #[error("{0}")] Capture(String),
    #[error("{0}")] Clipboard(String),
    #[error("{0}")] Save(String),
    #[error("{0}")] State(String),
    #[error("{0}")] Other(String),
}

// Explicit From impls for each underlying error type (no blanket impl — keeps
// the IPC error code accurate to its source).
use crate::config::store::ConfigError;
use crate::capture::CaptureError;
use crate::clipboard::ClipboardError;
use crate::fs::save::SaveError;
use crate::hotkey::HotkeyError;

impl From<ConfigError>    for AppError { fn from(e: ConfigError)    -> Self { Self::Config(e.to_string()) } }
impl From<CaptureError>   for AppError { fn from(e: CaptureError)   -> Self { Self::Capture(e.to_string()) } }
impl From<ClipboardError> for AppError { fn from(e: ClipboardError) -> Self { Self::Clipboard(e.to_string()) } }
impl From<SaveError>      for AppError { fn from(e: SaveError)      -> Self { Self::Save(e.to_string()) } }
impl From<HotkeyError>    for AppError { fn from(e: HotkeyError)    -> Self { Self::Hotkey(e.to_string()) } }
```

- [ ] **Step 2: Implement command stubs**

`src-tauri/src/ipc/commands.rs`:

```rust
use crate::capture::Rect;
use crate::config::Config;
use crate::error::AppError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FinishAction {
    CopyImage,
    Save { path: PathBuf },
    SaveAndCopyPath,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinishOutcome { pub saved_path: Option<PathBuf> }

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, AppError> {
    // Stub: read from cache in AppState (filled in Task 18).
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn update_config(_new: Config, _state: State<AppState>) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn selection_confirmed(_rect: Rect, _state: State<AppState>, _app: AppHandle)
    -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn selection_cancelled(_state: State<AppState>, _app: AppHandle) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn finish_action(
    _action: FinishAction,
    _image_bytes: Vec<u8>,
    _state: State<AppState>,
    _app: AppHandle,
) -> Result<FinishOutcome, AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn cancel_edit(_state: State<AppState>, _app: AppHandle) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}
```

`src-tauri/src/ipc/mod.rs`:

```rust
pub mod commands;
```

- [ ] **Step 3: Register in `main.rs`**

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use minipaste::ipc::commands::*;
use minipaste::state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            selection_confirmed,
            selection_cancelled,
            finish_action,
            cancel_edit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`Rect` needs `Deserialize` — add to its definition in `src-tauri/src/capture/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Rect { pub x: i32, pub y: i32, pub w: u32, pub h: u32 }
```

- [ ] **Step 4: Build**

```bash
cd src-tauri && cargo build
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ipc/ src-tauri/src/error.rs src-tauri/src/main.rs src-tauri/src/capture/mod.rs
git commit -m "feat(ipc): scaffold AppError + Tauri command stubs"
```

---

## Phase B — Settings UI (Vue)

### Task 13: Vue shared layer (types + IPC wrapper)

**Files:**
- Create: `src/shared/types.ts`
- Create: `src/shared/ipc.ts`
- Create: `src/shared/colors.ts`

- [ ] **Step 1: Define types**

`src/shared/types.ts`:

```ts
export type ImageFormat = "png" | "jpeg";

export interface Config {
  schema_version: number;
  hotkey: string;
  default_save_path: string;
  image_format: ImageFormat;
  jpeg_quality: number;
}

export interface Rect { x: number; y: number; w: number; h: number; }
export interface ScreenInfo { x: number; y: number; w: number; h: number; scale: number; }

export type ToolType = "line" | "rect" | "arrow" | "mosaic" | "text";
export type ColorKey = "red" | "orange" | "yellow" | "green" | "blue";
export type Thickness = "thin" | "medium" | "thick";

export interface Shape {
  id: string;
  tool: ToolType;
  color: ColorKey;
  thickness: Thickness;
  geometry: ShapeGeometry;
  text?: { content: string; fontSize: number };
}

export type ShapeGeometry =
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "rect"; x: number; y: number; w: number; h: number }
  | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "mosaic"; x: number; y: number; w: number; h: number; blockSize: number }
  | { kind: "text"; x: number; y: number; w: number; h: number };

export type FinishAction =
  | { kind: "CopyImage" }
  | { kind: "Save"; path: string }
  | { kind: "SaveAndCopyPath" };

export interface FinishOutcome { saved_path: string | null; }

export interface AppError { code: string; message: string; }
```

- [ ] **Step 2: IPC wrapper**

`src/shared/ipc.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, Event } from "@tauri-apps/api/event";

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

export function on<T>(event: string, handler: (payload: T) => void) {
  return listen<T>(event, (e: Event<T>) => handler(e.payload));
}
```

- [ ] **Step 3: Colors + thickness mapping**

`src/shared/colors.ts`:

```ts
import type { ColorKey, Thickness } from "./types";

export const COLOR_HEX: Record<ColorKey, string> = {
  red: "#ef4444",
  orange: "#f97316",
  yellow: "#eab308",
  green: "#22c55e",
  blue: "#3b82f6",
};

export const COLOR_ORDER: ColorKey[] = ["red", "orange", "yellow", "green", "blue"];

export const STROKE_WIDTH: Record<Thickness, number> = {
  thin: 2,
  medium: 4,
  thick: 8,
};

export const MOSAIC_BLOCK: Record<Thickness, number> = {
  thin: 8,
  medium: 16,
  thick: 24,
};

export const FONT_SIZE: Record<Thickness, number> = {
  thin: 16,
  medium: 24,
  thick: 36,
};
```

- [ ] **Step 4: Commit**

```bash
git add src/shared/
git commit -m "feat(frontend): add shared types + IPC wrapper + color mapping"
```

---

### Task 14: Settings App scaffold + main entry router

**Files:**
- Create: `src/main.ts`
- Create: `src/windows/settings/App.vue`
- Create: `src/windows/settings/settings.css`
- Create: `settings.html`

- [ ] **Step 1: Window router in `main.ts`**

```ts
import { createApp } from "vue";

const entry = document.documentElement.dataset.window;
async function bootstrap() {
  switch (entry) {
    case "settings": {
      const App = (await import("./windows/settings/App.vue")).default;
      createApp(App).mount("#app");
      break;
    }
    case "overlay": {
      const App = (await import("./windows/overlay/App.vue")).default;
      createApp(App).mount("#app");
      break;
    }
    case "editor": {
      const App = (await import("./windows/editor/App.vue")).default;
      createApp(App).mount("#app");
      break;
    }
    default:
      throw new Error(`unknown window: ${entry}`);
  }
}
bootstrap();
```

- [ ] **Step 2: `settings.html`**

```html
<!doctype html>
<html data-window="settings">
  <head>
    <meta charset="UTF-8" />
    <title>Settings</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

(`overlay.html` / `editor.html` identical structure with the corresponding `data-window`.)

- [ ] **Step 3: Settings App skeleton**

`src/windows/settings/App.vue`:

```vue
<script setup lang="ts">
import { onMounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import type { Config, ImageFormat } from "../../shared/types";
import HotkeyRecorder from "./HotkeyRecorder.vue";

const state = reactive({
  loaded: false,
  config: null as Config | null,
  error: "" as string,
});

onMounted(async () => {
  try {
    state.config = await call<Config>("get_config");
    state.loaded = true;
  } catch (e: any) {
    state.error = e.message ?? String(e);
  }
  on<{ attempted: string; reason: string }>("hotkey-conflict", (p) => {
    state.error = `Hotkey "${p.attempted}" 衝突：${p.reason}`;
  });
});

async function pickFolder() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, defaultPath: state.config?.default_save_path });
  if (picked && typeof picked === "string" && state.config) {
    state.config.default_save_path = picked;
  }
}

async function apply() {
  if (!state.config) return;
  try {
    await call<void>("update_config", { new: state.config });
    state.error = "";
  } catch (e: any) {
    state.error = e.message ?? String(e);
  }
}
</script>

<template>
  <div class="settings" v-if="state.loaded && state.config">
    <h2>Settings</h2>

    <label>Hotkey
      <HotkeyRecorder v-model="state.config.hotkey" />
    </label>

    <label>Default folder
      <input :value="state.config.default_save_path" readonly />
      <button @click="pickFolder">📁</button>
    </label>

    <label>Format
      <label><input type="radio" value="png" v-model="state.config.image_format" /> PNG</label>
      <label><input type="radio" value="jpeg" v-model="state.config.image_format" /> JPEG</label>
    </label>

    <label v-if="state.config.image_format === 'jpeg'">JPEG quality
      <input type="range" min="1" max="100" v-model.number="state.config.jpeg_quality" />
      <span>{{ state.config.jpeg_quality }}</span>
    </label>

    <p class="error" v-if="state.error">{{ state.error }}</p>

    <div class="actions">
      <button @click="apply">Save &amp; Apply</button>
    </div>
  </div>
  <div v-else-if="!state.loaded">Loading...</div>
</template>

<style scoped src="./settings.css"></style>
```

- [ ] **Step 4: Minimal CSS**

`src/windows/settings/settings.css`:

```css
.settings { padding: 16px; font-family: system-ui, sans-serif; display: flex; flex-direction: column; gap: 12px; }
.settings label { display: flex; flex-direction: column; gap: 4px; }
.error { color: #b91c1c; }
.actions { display: flex; justify-content: flex-end; }
button { padding: 6px 12px; }
```

- [ ] **Step 5: Commit**

```bash
git add src/main.ts src/windows/settings/ settings.html
git commit -m "feat(settings): scaffold Vue settings panel with config form"
```

---

### Task 15: Hotkey recorder component

**Files:**
- Create: `src/windows/settings/HotkeyRecorder.vue`
- Create: `src/__tests__/HotkeyRecorder.test.ts`

- [ ] **Step 1: Write failing test**

```ts
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import HotkeyRecorder from "../windows/settings/HotkeyRecorder.vue";

describe("HotkeyRecorder", () => {
  it("captures Ctrl+Shift+S and emits update", async () => {
    const w = mount(HotkeyRecorder, { props: { modelValue: "" } });
    await w.find(".hotkey-input").trigger("focus");
    await w.find(".hotkey-input").trigger("keydown", {
      key: "S", code: "KeyS", ctrlKey: true, shiftKey: true,
    });
    const emits = w.emitted("update:modelValue");
    expect(emits?.[0]?.[0]).toBe("Ctrl+Shift+S");
  });
});
```

- [ ] **Step 2: Set up Vitest config**

`vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: { environment: "jsdom", globals: true },
});
```

```bash
npm install -D vitest @vue/test-utils jsdom @vitejs/plugin-vue
```

- [ ] **Step 3: Run, expect fail**

```bash
npx vitest run
```

Expected: FAIL — component does not exist.

- [ ] **Step 4: Implement**

`src/windows/settings/HotkeyRecorder.vue`:

```vue
<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const recording = ref(false);

function format(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Meta");
  const k = e.key;
  if (!["Control", "Shift", "Alt", "Meta"].includes(k)) {
    parts.push(k.length === 1 ? k.toUpperCase() : k);
  }
  return parts.join("+");
}

function onKeydown(e: KeyboardEvent) {
  e.preventDefault();
  const formatted = format(e);
  // require at least one modifier + one key
  if (formatted.includes("+") && !["Control","Shift","Alt","Meta"].includes(e.key)) {
    emit("update:modelValue", formatted);
    recording.value = false;
  }
}
</script>

<template>
  <input
    class="hotkey-input"
    :value="recording ? '(press combo...)' : modelValue"
    readonly
    @focus="recording = true"
    @blur="recording = false"
    @keydown="onKeydown"
  />
</template>
```

- [ ] **Step 5: Run, expect pass**

```bash
npx vitest run
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/windows/settings/HotkeyRecorder.vue src/__tests__/ vitest.config.ts package.json
git commit -m "feat(settings): add HotkeyRecorder with key-combo capture"
```

---

### Task 16: Wire config IPC commands to real impl

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add config cache to AppState**

Modify `src-tauri/src/state.rs`:

```rust
use crate::config::Config;
// ... existing fields ...
pub struct AppState {
    pub phase: Mutex<AppPhase>,
    pub capture: Mutex<Option<CaptureFrame>>,
    pub cropped: Mutex<Option<Vec<u8>>>,
    pub last_save_dir: Mutex<Option<std::path::PathBuf>>,
    pub config: Mutex<Config>,
    pub config_path: std::path::PathBuf,
}

impl AppState {
    pub fn new(config: Config, config_path: std::path::PathBuf) -> Self {
        Self {
            phase: Mutex::new(AppPhase::Idle),
            capture: Mutex::new(None),
            cropped: Mutex::new(None),
            last_save_dir: Mutex::new(None),
            config: Mutex::new(config),
            config_path,
        }
    }
}
```

- [ ] **Step 2: Implement `get_config` + `update_config`**

```rust
#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, AppError> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn update_config(
    new: Config,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    // Persist first; if write fails, do not update in-memory state.
    crate::config::store::save(&state.config_path, &new)?;
    // Re-register hotkey if changed; if it conflicts, emit event & rollback.
    // (Hotkey re-registration wired in Task 19; for now just cache.)
    *state.config.lock().unwrap() = new;
    Ok(())
}
```

- [ ] **Step 3: Update `main.rs` to load config at startup**

```rust
use minipaste::config::{store, defaults};

fn main() {
    let app_data = dirs::config_dir()
        .expect("config dir")
        .join("minipaste");
    let config_path = store::config_path(app_data);
    let config = store::load_or_init(&config_path)
        .unwrap_or_else(|_| defaults::default_config());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(minipaste::state::AppState::new(config, config_path))
        .invoke_handler(tauri::generate_handler![/* ... */])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
```

- [ ] **Step 4: Manual verify**

```bash
npm run tauri dev
```

Open the Settings window programmatically via a temporary debug menu or by running the app and navigating to settings.html. (The tray wiring in Task 18 will make this automatic; for now you can open it manually from devtools or skip until Task 18.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(ipc): wire get_config + update_config to real store"
```

---

## Phase C — Tray, hotkey, capture, overlay

### Task 17: Tray icon + menu (Windows)

**Files:**
- Modify: `src-tauri/src/tray/windows.rs`
- Modify: `src-tauri/src/tray/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Place `src-tauri/icons/tray.png` (32×32)

- [ ] **Step 1: Define trait + impl**

`src-tauri/src/tray/mod.rs`:

```rust
#[cfg(target_os = "windows")] mod windows;
#[cfg(target_os = "macos")] mod macos;

#[cfg(target_os = "windows")] pub use windows::build_tray;
#[cfg(target_os = "macos")] pub use macos::build_tray;

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    OpenSettings,
    TriggerCapture,
    Quit,
}
```

`src-tauri/src/tray/windows.rs`:

```rust
use super::TrayEvent;
use tauri::{
    AppHandle, Manager,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
};

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItemBuilder::with_id("capture", "Capture").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app)
        .item(&capture).item(&settings).item(&separator).item(&quit).build()?;

    let _tray = TrayIconBuilder::with_id("minipaste-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            match event.id.as_ref() {
                "capture" => dispatch(app_handle, TrayEvent::TriggerCapture),
                "settings" => dispatch(app_handle, TrayEvent::OpenSettings),
                "quit" => dispatch(app_handle, TrayEvent::Quit),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left, button_state: MouseButtonState::Up, ..
            } = event {
                dispatch(tray.app_handle(), TrayEvent::OpenSettings);
            }
        })
        .build(app)?;
    Ok(())
}

fn dispatch(app: &AppHandle, ev: TrayEvent) {
    match ev {
        TrayEvent::OpenSettings => {
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        TrayEvent::TriggerCapture => {
            // Wired in Task 19
            let _ = app.emit("tray://trigger-capture", ());
        }
        TrayEvent::Quit => app.exit(0),
    }
}
```

`tray/macos.rs`: stub `pub fn build_tray(_app: &tauri::AppHandle) -> tauri::Result<()> { unimplemented!() }`.

- [ ] **Step 2: Wire in `main.rs`**

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(...)
    .setup(|app| {
        minipaste::tray::build_tray(app.handle())?;
        Ok(())
    })
    .invoke_handler(...)
    .run(...)
```

- [ ] **Step 3: Manual verify**

```bash
npm run tauri dev
```

Expected: tray icon appears bottom-right; left-click opens Settings; right-click shows menu with Capture / Settings / Quit; Quit exits app.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tray/ src-tauri/src/main.rs src-tauri/icons/
git commit -m "feat(tray): Windows tray icon with menu + left-click opens Settings"
```

---

### Task 18: Hotkey listener thread + dispatch to capture

**Files:**
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/src/hotkey/listener.rs`
- Modify: `src-tauri/src/hotkey/mod.rs`

- [ ] **Step 1: Add listener that polls global-hotkey events**

`src-tauri/src/hotkey/listener.rs`:

```rust
use crate::state::{AppState, AppPhase, PhaseEvent};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Spawn a background thread that listens for hotkey events and dispatches.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let rx = global_hotkey::GlobalHotKeyEvent::receiver();
        while let Ok(_event) = rx.recv() {
            // Only one hotkey is registered at a time so any event = trigger.
            handle_hotkey(&app);
        }
    });
}

fn handle_hotkey(app: &AppHandle) {
    let state: tauri::State<AppState> = app.state();
    let mut phase = state.phase.lock().unwrap();
    if phase.transition(PhaseEvent::HotkeyPressed).is_err() {
        return; // not idle, ignore
    }
    drop(phase);
    let _ = app.emit("tray://trigger-capture", ());
}
```

`src-tauri/src/hotkey/mod.rs`: add `pub mod listener;`

- [ ] **Step 2: Initialise hotkey in setup, register from config**

In `main.rs` setup:

```rust
.setup(|app| {
    minipaste::tray::build_tray(app.handle())?;

    // Register the configured hotkey
    let state: tauri::State<minipaste::state::AppState> = app.state();
    let combo = state.config.lock().unwrap().hotkey.clone();
    let mut hk = minipaste::hotkey::PlatformHotkey::new()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    if let Err(e) = hk.register(&combo) {
        // Emit conflict to settings
        let _ = app.emit("hotkey-conflict",
            serde_json::json!({ "attempted": combo, "reason": e.to_string() }));
    }
    // Store the manager so it isn't dropped (move into AppState if needed,
    // or leak with Box::leak for simplicity in MVP):
    Box::leak(Box::new(hk));

    minipaste::hotkey::listener::spawn(app.handle().clone());
    Ok(())
})
```

- [ ] **Step 3: Manual verify**

```bash
npm run tauri dev
```

Expected: pressing `Ctrl+Shift+S` emits `tray://trigger-capture` (observable via devtools console after listening). No window opens yet — wired in Task 19.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/hotkey/ src-tauri/src/main.rs
git commit -m "feat(hotkey): listener thread + register from config at startup"
```

---

### Task 19: Capture on trigger → store frame → open overlay

**Files:**
- Modify: `src-tauri/src/main.rs` (listen for `tray://trigger-capture`)
- Create: `src-tauri/src/capture/trigger.rs`
- Modify: `src-tauri/src/capture/mod.rs`

- [ ] **Step 1: Add trigger handler**

`src-tauri/src/capture/trigger.rs`:

```rust
use crate::capture::{Capture, PlatformCapture};
use crate::state::AppState;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Clone)]
pub struct CaptureReadyPayload {
    pub thumbnail_b64: String,   // PNG base64
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub screens: Vec<crate::capture::ScreenInfo>,
}

pub fn trigger_capture(app: &AppHandle) -> Result<(), String> {
    let cap = PlatformCapture::new();
    let frame = cap.virtual_desktop().map_err(|e| e.to_string())?;

    // Store full-res frame in state for later crop.
    let state: tauri::State<AppState> = app.state();
    *state.capture.lock().unwrap() = Some(frame.clone());

    // Show overlay with the captured background image.
    let b64 = base64::engine::general_purpose::STANDARD.encode(&frame.png_bytes);
    let payload = CaptureReadyPayload {
        thumbnail_b64: b64,
        width: frame.width,
        height: frame.height,
        origin_x: frame.origin_x,
        origin_y: frame.origin_y,
        screens: frame.screens.clone(),
    };

    if let Some(win) = app.get_webview_window("overlay") {
        // Resize / position to cover virtual desktop:
        let _ = win.set_position(tauri::PhysicalPosition { x: frame.origin_x, y: frame.origin_y });
        let _ = win.set_size(tauri::PhysicalSize { width: frame.width, height: frame.height });
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("capture-ready", payload);
    }
    Ok(())
}
```

Add `screenshots::ScreenInfo` derive `Serialize`: not possible since external; add `Serialize` to our `ScreenInfo` in `capture/mod.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenInfo { /* ... */ }
```

Add `base64 = "0.22"` to `Cargo.toml`.

`PlatformCapture::new()` — add to each impl (`WindowsCapture::new()` already exists). Adjust `capture/mod.rs` exports.

- [ ] **Step 2: Wire listener for `tray://trigger-capture` in `main.rs`**

```rust
.setup(|app| {
    minipaste::tray::build_tray(app.handle())?;
    // ... hotkey setup ...

    let app_handle = app.handle().clone();
    app.listen("tray://trigger-capture", move |_| {
        if let Err(e) = minipaste::capture::trigger::trigger_capture(&app_handle) {
            let _ = app_handle.emit("capture-error", e);
        }
    });
    Ok(())
})
```

- [ ] **Step 3: Manual verify**

```bash
npm run tauri dev
```

Expected: pressing `Ctrl+Shift+S` makes the overlay window appear full-screen, transparent (no content yet — Task 20). State machine in `Capturing` phase.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat(capture): trigger handler captures + opens overlay with frame"
```

---

### Task 20: Overlay Vue app — full-screen background + selection rectangle

**Files:**
- Create: `src/windows/overlay/App.vue`
- Create: `src/windows/overlay/selection.ts`
- Create: `src/windows/overlay/overlay.css`
- Create: `overlay.html`

- [ ] **Step 1: Write failing test for selection math**

`src/__tests__/selection.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { rectFromDrag, clampToBounds } from "../windows/overlay/selection";

describe("rectFromDrag", () => {
  it("returns positive w/h regardless of drag direction", () => {
    expect(rectFromDrag({ x: 100, y: 100 }, { x: 50, y: 30 }))
      .toEqual({ x: 50, y: 30, w: 50, h: 70 });
  });
});

describe("clampToBounds", () => {
  it("clamps a rect to [0..w]x[0..h]", () => {
    expect(clampToBounds({ x: -10, y: 5, w: 100, h: 100 }, 80, 80))
      .toEqual({ x: 0, y: 5, w: 80, h: 75 });
  });
});
```

- [ ] **Step 2: Run, expect fail**

- [ ] **Step 3: Implement**

`src/windows/overlay/selection.ts`:

```ts
export interface Point { x: number; y: number; }
export interface Rect { x: number; y: number; w: number; h: number; }

export function rectFromDrag(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const w = Math.abs(a.x - b.x);
  const h = Math.abs(a.y - b.y);
  return { x, y, w, h };
}

export function clampToBounds(r: Rect, maxW: number, maxH: number): Rect {
  const x = Math.max(0, r.x);
  const y = Math.max(0, r.y);
  const w = Math.min(maxW - x, r.w - (x - r.x));
  const h = Math.min(maxH - y, r.h - (y - r.y));
  return { x, y, w: Math.max(0, w), h: Math.max(0, h) };
}
```

- [ ] **Step 4: Build the App**

`src/windows/overlay/App.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import { rectFromDrag, clampToBounds, type Point } from "./selection";

const state = reactive({
  bgUrl: "",
  width: 0,
  height: 0,
  origin: { x: 0, y: 0 },
  dragStart: null as Point | null,
  dragEnd: null as Point | null,
});

const selectionStyle = computed(() => {
  if (!state.dragStart || !state.dragEnd) return {};
  const r = rectFromDrag(state.dragStart, state.dragEnd);
  return {
    left: r.x + "px", top: r.y + "px",
    width: r.w + "px", height: r.h + "px",
  };
});

onMounted(() => {
  on<{
    thumbnail_b64: string; width: number; height: number;
    origin_x: number; origin_y: number;
  }>("capture-ready", (p) => {
    state.bgUrl = `data:image/png;base64,${p.thumbnail_b64}`;
    state.width = p.width; state.height = p.height;
    state.origin = { x: p.origin_x, y: p.origin_y };
  });
  window.addEventListener("keydown", onKey);
});

onUnmounted(() => window.removeEventListener("keydown", onKey));

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") cancel();
}

function onMouseDown(e: MouseEvent) {
  state.dragStart = { x: e.clientX, y: e.clientY };
  state.dragEnd = { x: e.clientX, y: e.clientY };
}

function onMouseMove(e: MouseEvent) {
  if (state.dragStart) state.dragEnd = { x: e.clientX, y: e.clientY };
}

async function onMouseUp() {
  if (!state.dragStart || !state.dragEnd) return;
  const local = rectFromDrag(state.dragStart, state.dragEnd);
  const clamped = clampToBounds(local, state.width, state.height);
  state.dragStart = null;
  state.dragEnd = null;
  if (clamped.w < 5 || clamped.h < 5) return;  // ignore micro drags
  const rectInOsCoords = {
    x: clamped.x + state.origin.x,
    y: clamped.y + state.origin.y,
    w: clamped.w, h: clamped.h,
  };
  await call("selection_confirmed", { rect: rectInOsCoords });
}

async function cancel() {
  await call("selection_cancelled");
}
</script>

<template>
  <div class="overlay"
       :style="{ backgroundImage: `url(${state.bgUrl})` }"
       @mousedown="onMouseDown" @mousemove="onMouseMove" @mouseup="onMouseUp">
    <div class="dim"></div>
    <div v-if="state.dragStart && state.dragEnd" class="selection"
         :style="selectionStyle"></div>
  </div>
</template>

<style scoped src="./overlay.css"></style>
```

`src/windows/overlay/overlay.css`:

```css
.overlay { position: fixed; inset: 0; background-size: cover; background-position: top left; cursor: crosshair; }
.dim { position: absolute; inset: 0; background: rgba(0,0,0,0.35); pointer-events: none; }
.selection { position: absolute; border: 2px solid #3b82f6; background: rgba(59,130,246,0.1); pointer-events: none; }
```

`overlay.html`: same as settings.html but `data-window="overlay"`.

- [ ] **Step 5: Run unit tests**

```bash
npx vitest run
```

Expected: PASS.

- [ ] **Step 6: Manual smoke**

```bash
npm run tauri dev
```

Expected: hotkey → overlay appears with screenshot background, dim filter, drag a rectangle, release → IPC call (no editor yet).

- [ ] **Step 7: Commit**

```bash
git add src/windows/overlay/ overlay.html src/__tests__/selection.test.ts
git commit -m "feat(overlay): full-screen selection UI + drag rect logic"
```

---

### Task 21: Selection confirmed → crop → open editor

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Create: `src-tauri/src/capture/finalise.rs`

- [ ] **Step 1: Implement `selection_confirmed` + `selection_cancelled`**

```rust
#[tauri::command]
pub fn selection_confirmed(
    rect: crate::capture::Rect,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    phase.transition(crate::state::PhaseEvent::SelectionConfirmed)
        .map_err(|e| AppError::State(e.to_string()))?;
    drop(phase);

    let frame_opt = state.capture.lock().unwrap().clone();
    let frame = frame_opt.ok_or_else(|| AppError::Capture("no frame in state".into()))?;

    let cap = crate::capture::PlatformCapture::new();
    let cropped = cap.crop(&frame, rect)?;
    *state.cropped.lock().unwrap() = Some(cropped.clone());

    // Hide overlay, open editor with cropped image.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    if let Some(editor) = app.get_webview_window("editor") {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&cropped);
        let _ = editor.show();
        let _ = editor.set_focus();
        let _ = editor.emit("editor-ready", serde_json::json!({
            "image_b64": b64,
            "width": rect.w,
            "height": rect.h,
        }));
    }
    Ok(())
}

#[tauri::command]
pub fn selection_cancelled(state: State<AppState>, app: AppHandle)
    -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    let _ = phase.transition(crate::state::PhaseEvent::Cancelled);
    drop(phase);
    *state.capture.lock().unwrap() = None;
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    Ok(())
}
```

- [ ] **Step 2: Manual verify**

Drag a rectangle on the overlay → expect editor window to open with the cropped image visible (Task 22 will paint it; for now just verify window appears + event payload via devtools).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ipc/commands.rs
git commit -m "feat(ipc): wire selection_confirmed → crop → open editor"
```

---

## Phase D — Editor (Konva)

### Task 22: Editor App scaffold + Konva stage

**Files:**
- Create: `editor.html`
- Create: `src/windows/editor/App.vue`
- Create: `src/windows/editor/canvas/Stage.vue`
- Create: `src/windows/editor/editor.css`
- Install: `npm install konva`

- [ ] **Step 1: Install Konva**

```bash
npm install konva
```

- [ ] **Step 2: Editor App skeleton**

`src/windows/editor/App.vue`:

```vue
<script setup lang="ts">
import { onMounted, reactive } from "vue";
import { on } from "../../shared/ipc";
import Stage from "./canvas/Stage.vue";
import Toolbar from "./ui/Toolbar.vue";
import ActionBar from "./ui/ActionBar.vue";

const state = reactive({
  imgUrl: "",
  width: 0,
  height: 0,
});

onMounted(() => {
  on<{ image_b64: string; width: number; height: number }>("editor-ready", (p) => {
    state.imgUrl = `data:image/png;base64,${p.image_b64}`;
    state.width = p.width; state.height = p.height;
  });
});
</script>

<template>
  <div class="editor">
    <Toolbar />
    <Stage :image-url="state.imgUrl" :width="state.width" :height="state.height" />
    <ActionBar />
  </div>
</template>

<style scoped src="./editor.css"></style>
```

`editor.css`:

```css
.editor { display: grid; grid-template-rows: auto 1fr auto; height: 100vh; gap: 8px; padding: 8px; }
```

`src/windows/editor/canvas/Stage.vue`:

```vue
<script setup lang="ts">
import Konva from "konva";
import { onMounted, ref, watch } from "vue";

const props = defineProps<{ imageUrl: string; width: number; height: number }>();
const containerRef = ref<HTMLDivElement | null>(null);
let stage: Konva.Stage | null = null;
let bgLayer: Konva.Layer | null = null;
let annLayer: Konva.Layer | null = null;
let previewLayer: Konva.Layer | null = null;
let uiLayer: Konva.Layer | null = null;

onMounted(() => {
  if (!containerRef.value) return;
  stage = new Konva.Stage({
    container: containerRef.value,
    width: props.width || 800,
    height: props.height || 600,
  });
  bgLayer = new Konva.Layer({ listening: false });
  annLayer = new Konva.Layer();
  previewLayer = new Konva.Layer({ listening: false });
  uiLayer = new Konva.Layer();
  stage.add(bgLayer); stage.add(annLayer); stage.add(previewLayer); stage.add(uiLayer);
});

watch(() => props.imageUrl, (url) => {
  if (!url || !stage || !bgLayer) return;
  Konva.Image.fromURL(url, (img) => {
    img.setAttrs({ x: 0, y: 0, width: props.width, height: props.height });
    bgLayer!.destroyChildren();
    bgLayer!.add(img);
    bgLayer!.draw();
    stage!.size({ width: props.width, height: props.height });
  });
});

defineExpose({ getStage: () => stage });
</script>

<template>
  <div ref="containerRef" class="stage-host"></div>
</template>
```

(Toolbar / ActionBar empty stubs for now.)

- [ ] **Step 3: Manual verify**

```bash
npm run tauri dev
```

Hotkey → select → editor opens showing the cropped image inside Konva canvas.

- [ ] **Step 4: Commit**

```bash
git add src/windows/editor/ editor.html package.json
git commit -m "feat(editor): scaffold Vue app + Konva stage with 4 layers"
```

---

### Task 23: Shape store + history (undo/redo)

**Files:**
- Create: `src/windows/editor/state/shapes.ts`
- Create: `src/windows/editor/state/history.ts`
- Create: `src/__tests__/history.test.ts`

- [ ] **Step 1: Write failing test for history**

```ts
import { describe, expect, it } from "vitest";
import { createHistory } from "../windows/editor/state/history";
import type { Shape } from "../shared/types";

const s = (id: string): Shape => ({
  id, tool: "rect", color: "red", thickness: "medium",
  geometry: { kind: "rect", x: 0, y: 0, w: 1, h: 1 },
});

describe("history", () => {
  it("push then undo restores previous snapshot", () => {
    const h = createHistory();
    h.push([]);
    h.push([s("a")]);
    h.push([s("a"), s("b")]);
    expect(h.current()).toEqual([s("a"), s("b")]);
    h.undo();
    expect(h.current()).toEqual([s("a")]);
    h.undo();
    expect(h.current()).toEqual([]);
  });

  it("redo replays forward", () => {
    const h = createHistory();
    h.push([]); h.push([s("a")]);
    h.undo();
    h.redo();
    expect(h.current()).toEqual([s("a")]);
  });

  it("new push after undo drops the redo tail", () => {
    const h = createHistory();
    h.push([]); h.push([s("a")]);
    h.undo();
    h.push([s("b")]);
    expect(() => h.redo()).not.toThrow();
    expect(h.current()).toEqual([s("b")]);
  });

  it("limits to 50 snapshots", () => {
    const h = createHistory(50);
    for (let i = 0; i < 60; i++) h.push([s(`${i}`)]);
    expect(h.size()).toBe(50);
  });
});
```

- [ ] **Step 2: Run, expect fail**

- [ ] **Step 3: Implement**

`src/windows/editor/state/history.ts`:

```ts
import type { Shape } from "../../../shared/types";

export interface History {
  push(snapshot: Shape[]): void;
  undo(): void;
  redo(): void;
  current(): Shape[];
  size(): number;
}

export function createHistory(limit = 50): History {
  let stack: Shape[][] = [];
  let pointer = -1;
  return {
    push(snap) {
      // drop redo tail
      stack = stack.slice(0, pointer + 1);
      stack.push(snap.map(s => ({ ...s })));  // shallow clone
      if (stack.length > limit) stack.shift();
      pointer = stack.length - 1;
    },
    undo() { if (pointer > 0) pointer--; },
    redo() { if (pointer < stack.length - 1) pointer++; },
    current() { return pointer >= 0 ? stack[pointer] : []; },
    size() { return stack.length; },
  };
}
```

`src/windows/editor/state/shapes.ts`:

```ts
import { reactive } from "vue";
import { customAlphabet } from "nanoid";
import type { Shape, ToolType, ColorKey, Thickness } from "../../../shared/types";
import { createHistory } from "./history";

const nid = customAlphabet("abcdefghijklmnopqrstuvwxyz0123456789", 10);

export const editorState = reactive({
  tool: "rect" as ToolType,
  color: "red" as ColorKey,
  thickness: "medium" as Thickness,
  shapes: [] as Shape[],
  selectedId: null as string | null,
});

const history = createHistory();
history.push([]);

export function commitChange() {
  history.push(editorState.shapes);
}

export function undo() {
  history.undo();
  editorState.shapes = history.current().map(s => ({ ...s }));
}

export function redo() {
  history.redo();
  editorState.shapes = history.current().map(s => ({ ...s }));
}

export function newShape(partial: Omit<Shape, "id">): Shape {
  return { id: nid(), ...partial };
}
```

Install nanoid: `npm install nanoid`.

- [ ] **Step 4: Run tests, expect pass**

- [ ] **Step 5: Commit**

```bash
git add src/windows/editor/state/ src/__tests__/history.test.ts package.json
git commit -m "feat(editor): reactive shape store + undo/redo history with limit"
```

---

### Task 24: Toolbar UI (tool / color / thickness / undo / redo)

**Files:**
- Create: `src/windows/editor/ui/Toolbar.vue`

- [ ] **Step 1: Implement**

```vue
<script setup lang="ts">
import { editorState, undo, redo } from "../state/shapes";
import { COLOR_HEX, COLOR_ORDER } from "../../../shared/colors";
import type { ToolType, Thickness } from "../../../shared/types";

const tools: { key: ToolType; label: string }[] = [
  { key: "line", label: "／" }, { key: "rect", label: "▭" },
  { key: "arrow", label: "↗" }, { key: "mosaic", label: "▦" },
  { key: "text", label: "T" },
];
const thicknesses: Thickness[] = ["thin", "medium", "thick"];
</script>

<template>
  <div class="toolbar">
    <button v-for="t in tools" :key="t.key"
            :class="{ active: editorState.tool === t.key }"
            @click="editorState.tool = t.key">{{ t.label }}</button>

    <span class="sep"></span>

    <button v-for="c in COLOR_ORDER" :key="c"
            class="swatch"
            :class="{ active: editorState.color === c }"
            :style="{ background: COLOR_HEX[c] }"
            @click="editorState.color = c"></button>

    <span class="sep"></span>

    <button v-for="t in thicknesses" :key="t"
            :class="{ active: editorState.thickness === t }"
            @click="editorState.thickness = t">{{ t[0].toUpperCase() }}</button>

    <span class="sep"></span>

    <button @click="undo">↶</button>
    <button @click="redo">↷</button>
  </div>
</template>

<style scoped>
.toolbar { display: flex; gap: 6px; align-items: center; padding: 4px 8px; border-bottom: 1px solid #ddd; }
.toolbar button { padding: 4px 10px; background: #f3f4f6; border: 1px solid #d1d5db; border-radius: 4px; cursor: pointer; }
.toolbar button.active { background: #3b82f6; color: white; border-color: #2563eb; }
.swatch { width: 22px; height: 22px; padding: 0 !important; border-radius: 50% !important; }
.swatch.active { outline: 2px solid #1f2937; outline-offset: 2px; }
.sep { width: 1px; height: 18px; background: #d1d5db; }
</style>
```

- [ ] **Step 2: Manual verify**

Tool buttons highlight on click; color & thickness selectors work; Undo/Redo buttons trigger functions (no shapes yet to undo — empty state).

- [ ] **Step 3: Commit**

```bash
git add src/windows/editor/ui/Toolbar.vue
git commit -m "feat(editor): Toolbar with tools, color, thickness, undo/redo"
```

---

### Task 25: Line + Rect + Arrow tools (shared drag-to-draw)

**Files:**
- Modify: `src/windows/editor/canvas/Stage.vue`
- Create: `src/windows/editor/canvas/drawTools.ts`

- [ ] **Step 1: Implement shape factory**

`src/windows/editor/canvas/drawTools.ts`:

```ts
import Konva from "konva";
import { COLOR_HEX, STROKE_WIDTH } from "../../../shared/colors";
import type { Shape } from "../../../shared/types";

export function renderShape(shape: Shape): Konva.Node {
  const stroke = COLOR_HEX[shape.color];
  const width = STROKE_WIDTH[shape.thickness];
  switch (shape.geometry.kind) {
    case "line": {
      const g = shape.geometry;
      return new Konva.Line({
        points: [g.x1, g.y1, g.x2, g.y2],
        stroke, strokeWidth: width, lineCap: "round",
        id: shape.id,
      });
    }
    case "rect": {
      const g = shape.geometry;
      return new Konva.Rect({
        x: g.x, y: g.y, width: g.w, height: g.h,
        stroke, strokeWidth: width, id: shape.id,
      });
    }
    case "arrow": {
      const g = shape.geometry;
      return new Konva.Arrow({
        points: [g.x1, g.y1, g.x2, g.y2],
        stroke, fill: stroke, strokeWidth: width,
        pointerLength: width * 3, pointerWidth: width * 3,
        id: shape.id,
      });
    }
    // mosaic + text: later tasks
    default:
      throw new Error(`renderShape: ${(shape.geometry as any).kind} not yet supported`);
  }
}
```

- [ ] **Step 2: Wire pointer events in `Stage.vue`**

Augment `onMounted` to attach Konva stage event handlers:

```ts
// inside onMounted, after stage creation:
let drafting: { startX: number; startY: number; node: Konva.Node | null } | null = null;

stage.on("mousedown", (e) => {
  // ignore clicks on existing annotation (selection handled later)
  const tool = editorState.tool;
  if (tool !== "line" && tool !== "rect" && tool !== "arrow") return;
  const pos = stage!.getPointerPosition()!;
  drafting = { startX: pos.x, startY: pos.y, node: null };
});

stage.on("mousemove", () => {
  if (!drafting) return;
  const pos = stage!.getPointerPosition()!;
  const draft = buildDraftShape(drafting.startX, drafting.startY, pos.x, pos.y);
  if (drafting.node) drafting.node.destroy();
  const node = renderShape(draft);
  drafting.node = node;
  previewLayer!.destroyChildren();
  previewLayer!.add(node);
  previewLayer!.batchDraw();
});

stage.on("mouseup", () => {
  if (!drafting) return;
  const pos = stage!.getPointerPosition()!;
  const final = buildDraftShape(drafting.startX, drafting.startY, pos.x, pos.y);
  // ignore tiny drags
  if (shapeIsTooSmall(final)) {
    previewLayer!.destroyChildren();
    previewLayer!.batchDraw();
    drafting = null;
    return;
  }
  editorState.shapes.push(final);
  commitChange();
  // re-render annotations layer (subscribed below)
  previewLayer!.destroyChildren();
  previewLayer!.batchDraw();
  drafting = null;
});

function buildDraftShape(x1: number, y1: number, x2: number, y2: number): Shape {
  const tool = editorState.tool;
  const base = {
    id: "draft",
    color: editorState.color,
    thickness: editorState.thickness,
    tool,
  };
  if (tool === "rect") {
    return {
      ...base,
      geometry: { kind: "rect",
        x: Math.min(x1, x2), y: Math.min(y1, y2),
        w: Math.abs(x1 - x2), h: Math.abs(y1 - y2) },
    } as Shape;
  }
  return {
    ...base,
    geometry: { kind: tool === "line" ? "line" : "arrow", x1, y1, x2, y2 },
  } as Shape;
}

function shapeIsTooSmall(s: Shape): boolean {
  const g = s.geometry;
  if (g.kind === "rect") return g.w < 3 || g.h < 3;
  if (g.kind === "line" || g.kind === "arrow")
    return Math.hypot(g.x2 - g.x1, g.y2 - g.y1) < 3;
  return false;
}
```

Add reactive sync for `annotations` layer:

```ts
import { watch } from "vue";
import { editorState } from "../state/shapes";

watch(() => editorState.shapes.length, () => {
  annLayer!.destroyChildren();
  editorState.shapes.forEach(s => {
    try { annLayer!.add(renderShape(s) as any); }
    catch { /* unsupported shape kinds skipped */ }
  });
  annLayer!.batchDraw();
});
```

- [ ] **Step 3: Manual verify**

```bash
npm run tauri dev
```

Open editor → pick `▭` → drag → rect appears on release. Switch tool to `／` or `↗`, same drag flow. Undo button removes last shape.

- [ ] **Step 4: Commit**

```bash
git add src/windows/editor/canvas/
git commit -m "feat(editor): line/rect/arrow draw-to-create with preview layer"
```

---

### Task 26: Mosaic tool (reblit technique)

**Files:**
- Modify: `src/windows/editor/canvas/drawTools.ts`
- Modify: `src/windows/editor/canvas/Stage.vue`

- [ ] **Step 1: Add mosaic renderer**

In `drawTools.ts`, add a helper that takes the background image element and produces a `Konva.Image` filled with the pixelated reblit:

```ts
import { MOSAIC_BLOCK } from "../../../shared/colors";

export function renderMosaic(
  shape: Shape & { geometry: { kind: "mosaic" } },
  bgImage: HTMLImageElement
): Konva.Image {
  const { x, y, w, h, blockSize } = shape.geometry;
  // Sample bg at (x,y,w,h), downscale to (w/blockSize, h/blockSize), upscale back.
  const off = document.createElement("canvas");
  off.width = w; off.height = h;
  const ctx = off.getContext("2d")!;
  // Step 1: paint the source region
  ctx.drawImage(bgImage, x, y, w, h, 0, 0, w, h);
  // Step 2: downscale
  const small = document.createElement("canvas");
  small.width = Math.max(1, Math.floor(w / blockSize));
  small.height = Math.max(1, Math.floor(h / blockSize));
  const sctx = small.getContext("2d")!;
  sctx.imageSmoothingEnabled = false;
  sctx.drawImage(off, 0, 0, small.width, small.height);
  // Step 3: upscale back into off with nearest-neighbour
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, w, h);
  ctx.drawImage(small, 0, 0, small.width, small.height, 0, 0, w, h);
  return new Konva.Image({ x, y, image: off, width: w, height: h, id: shape.id });
}
```

Update `renderShape` to handle mosaic:

```ts
// ... inside renderShape switch
    case "mosaic":
      throw new Error("mosaic must use renderMosaic with bg image");
```

In `Stage.vue`, keep a reference to the loaded background `HTMLImageElement`:

```ts
let bgImage: HTMLImageElement | null = null;

watch(() => props.imageUrl, async (url) => {
  if (!url) return;
  const img = new Image();
  img.src = url;
  await img.decode();
  bgImage = img;
  // ... existing fromURL flow
});
```

Adjust the watcher that re-renders the annotations layer to use `renderMosaic` for mosaic shapes:

```ts
watch(() => editorState.shapes.length, () => {
  annLayer!.destroyChildren();
  editorState.shapes.forEach(s => {
    if (s.geometry.kind === "mosaic" && bgImage) {
      annLayer!.add(renderMosaic(s as any, bgImage));
    } else {
      try { annLayer!.add(renderShape(s) as any); } catch {}
    }
  });
  annLayer!.batchDraw();
});
```

Update `buildDraftShape` to construct mosaic geometry:

```ts
if (tool === "mosaic") {
  return {
    ...base,
    geometry: {
      kind: "mosaic",
      x: Math.min(x1, x2), y: Math.min(y1, y2),
      w: Math.abs(x1 - x2), h: Math.abs(y1 - y2),
      blockSize: MOSAIC_BLOCK[editorState.thickness],
    },
  } as Shape;
}
```

And add `"mosaic"` to the tool whitelist in `mousedown`/`mousemove`/`mouseup` handlers.

Preview during drag for mosaic: render a translucent rect placeholder (computing full mosaic on every mousemove is heavy):

```ts
// in mousemove for mosaic: replace renderShape() call with a translucent rect outline:
if (tool === "mosaic") {
  drafting.node = new Konva.Rect({
    x: Math.min(drafting.startX, pos.x), y: Math.min(drafting.startY, pos.y),
    width: Math.abs(drafting.startX - pos.x), height: Math.abs(drafting.startY - pos.y),
    fill: "rgba(0,0,0,0.4)", stroke: "white", strokeWidth: 1, dash: [4, 4],
  });
  previewLayer!.destroyChildren(); previewLayer!.add(drafting.node); previewLayer!.batchDraw();
}
```

- [ ] **Step 2: Manual verify**

Switch tool to `▦` → drag over text in the background → release → pixelated patch appears. Try different thickness levels for different block sizes.

- [ ] **Step 3: Commit**

```bash
git add src/windows/editor/canvas/
git commit -m "feat(editor): mosaic tool with reblit + dashed preview"
```

---

### Task 27: Text tool (HTML textarea overlay)

**Files:**
- Modify: `src/windows/editor/canvas/Stage.vue`
- Create: `src/windows/editor/canvas/textTool.ts`

- [ ] **Step 1: Implement textarea overlay helper**

`src/windows/editor/canvas/textTool.ts`:

```ts
import { COLOR_HEX, FONT_SIZE } from "../../../shared/colors";
import type { Shape, ColorKey, Thickness } from "../../../shared/types";

export interface TextEditOptions {
  containerEl: HTMLElement;
  stagePoint: { x: number; y: number };
  color: ColorKey;
  thickness: Thickness;
  initial?: string;
  onCommit: (text: string, bounds: { w: number; h: number }) => void;
  onCancel: () => void;
}

export function openTextEditor(opts: TextEditOptions) {
  const ta = document.createElement("textarea");
  ta.className = "konva-text-editor";
  Object.assign(ta.style, {
    position: "absolute",
    left: `${opts.stagePoint.x}px`,
    top: `${opts.stagePoint.y}px`,
    color: COLOR_HEX[opts.color],
    fontSize: `${FONT_SIZE[opts.thickness]}px`,
    fontFamily: "system-ui, sans-serif",
    background: "rgba(255,255,255,0.85)",
    border: "1px dashed #1f2937",
    padding: "2px 4px",
    minWidth: "60px",
    minHeight: `${FONT_SIZE[opts.thickness] + 8}px`,
    resize: "both",
    zIndex: "1000",
  });
  ta.value = opts.initial ?? "";
  opts.containerEl.appendChild(ta);
  ta.focus();

  function commit() {
    const text = ta.value;
    const w = ta.offsetWidth;
    const h = ta.offsetHeight;
    cleanup();
    if (text.trim()) opts.onCommit(text, { w, h });
    else opts.onCancel();
  }
  function cancel() { cleanup(); opts.onCancel(); }
  function cleanup() {
    ta.removeEventListener("keydown", onKey);
    ta.removeEventListener("blur", commit);
    ta.remove();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); cancel(); }
    else if (e.key === "Enter" && e.ctrlKey) { e.preventDefault(); commit(); }
  }
  ta.addEventListener("keydown", onKey);
  ta.addEventListener("blur", commit);
}
```

- [ ] **Step 2: Wire in `Stage.vue`**

In `mousedown` handler, when tool is `text`:

```ts
if (editorState.tool === "text") {
  const pos = stage!.getPointerPosition()!;
  openTextEditor({
    containerEl: containerRef.value!,
    stagePoint: pos,
    color: editorState.color,
    thickness: editorState.thickness,
    onCommit: (text, bounds) => {
      const shape: Shape = newShape({
        tool: "text", color: editorState.color, thickness: editorState.thickness,
        geometry: { kind: "text", x: pos.x, y: pos.y, w: bounds.w, h: bounds.h },
        text: { content: text, fontSize: FONT_SIZE[editorState.thickness] },
      });
      editorState.shapes.push(shape);
      commitChange();
    },
    onCancel: () => {},
  });
  return;
}
```

Render text shape in `drawTools.ts`:

```ts
case "text": {
  const g = shape.geometry;
  return new Konva.Text({
    x: g.x, y: g.y, width: g.w, height: g.h,
    text: shape.text?.content ?? "",
    fill: stroke,
    fontSize: shape.text?.fontSize ?? FONT_SIZE[shape.thickness],
    fontFamily: "system-ui, sans-serif",
    id: shape.id,
  });
}
```

- [ ] **Step 3: Manual verify**

Pick T tool → click on canvas → textarea appears → type → Ctrl+Enter → text commits as a Konva.Text on the canvas.

- [ ] **Step 4: Commit**

```bash
git add src/windows/editor/canvas/
git commit -m "feat(editor): text tool with HTML textarea overlay"
```

---

### Task 28: Selection / Transformer for editing existing shapes

**Files:**
- Modify: `src/windows/editor/canvas/Stage.vue`

- [ ] **Step 1: Add Transformer**

```ts
import Konva from "konva";

let transformer: Konva.Transformer | null = null;

// in onMounted, after layers created:
transformer = new Konva.Transformer({ rotateEnabled: false, ignoreStroke: true });
uiLayer!.add(transformer);

annLayer.on("click", (e) => {
  // Only when no tool is in draw mode? For MVP: any click on existing shape selects it.
  if (editorState.tool === "text") return;
  const target = e.target;
  if (target === stage) {
    transformer!.nodes([]);
    editorState.selectedId = null;
    return;
  }
  transformer!.nodes([target]);
  editorState.selectedId = target.id();
});

stage.on("click", (e) => {
  if (e.target === stage) {
    transformer!.nodes([]);
    editorState.selectedId = null;
  }
});
```

On drag/resize end, persist back to `editorState.shapes`:

```ts
annLayer.on("dragend transformend", (e) => {
  const node = e.target;
  const id = node.id();
  const shape = editorState.shapes.find(s => s.id === id);
  if (!shape) return;
  // Patch geometry based on shape kind:
  const g = shape.geometry;
  if (g.kind === "rect" || g.kind === "mosaic" || g.kind === "text") {
    shape.geometry = { ...g, x: node.x(), y: node.y(),
      w: node.width() * node.scaleX(), h: node.height() * node.scaleY() } as any;
    node.scale({ x: 1, y: 1 });
  } else if (g.kind === "line" || g.kind === "arrow") {
    const pts = (node as Konva.Line).points();
    shape.geometry = { ...g, x1: pts[0] + node.x(), y1: pts[1] + node.y(),
                              x2: pts[2] + node.x(), y2: pts[3] + node.y() } as any;
    node.position({ x: 0, y: 0 });
  }
  commitChange();
});
```

Make shapes draggable by adding `draggable: true` in `drawTools.ts` factories. Mosaic shape stays not draggable (re-sampling on move is too expensive for MVP) — alternatively allow dragging but re-render on drop. **For MVP: mosaic is not draggable.**

- [ ] **Step 2: Manual verify**

Draw a rect → click on it → transformer handles appear → drag → resize → release → undo restores original geometry.

- [ ] **Step 3: Commit**

```bash
git add src/windows/editor/canvas/
git commit -m "feat(editor): select + drag + resize existing shapes via Transformer"
```

---

### Task 29: Keyboard shortcuts (Ctrl+Z, Ctrl+Y, Esc, Delete)

**Files:**
- Modify: `src/windows/editor/App.vue`

- [ ] **Step 1: Implement**

In `App.vue` `onMounted`:

```ts
import { call } from "../../shared/ipc";
import { editorState, undo, redo, commitChange } from "./state/shapes";

window.addEventListener("keydown", onShortcut);
onUnmounted(() => window.removeEventListener("keydown", onShortcut));

function onShortcut(e: KeyboardEvent) {
  if (e.target instanceof HTMLTextAreaElement) return;  // textarea owns its keys
  if (e.ctrlKey && e.key.toLowerCase() === "z" && !e.shiftKey) {
    e.preventDefault(); undo();
  } else if ((e.ctrlKey && e.key.toLowerCase() === "y") ||
             (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "z")) {
    e.preventDefault(); redo();
  } else if (e.key === "Escape") {
    e.preventDefault();
    call("cancel_edit");
  } else if ((e.key === "Delete" || e.key === "Backspace") && editorState.selectedId) {
    e.preventDefault();
    editorState.shapes = editorState.shapes.filter(s => s.id !== editorState.selectedId);
    editorState.selectedId = null;
    commitChange();
  }
}
```

- [ ] **Step 2: Implement `cancel_edit` command**

`src-tauri/src/ipc/commands.rs`:

```rust
#[tauri::command]
pub fn cancel_edit(state: State<AppState>, app: AppHandle) -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    let _ = phase.transition(crate::state::PhaseEvent::Cancelled);
    drop(phase);
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    if let Some(editor) = app.get_webview_window("editor") {
        let _ = editor.hide();
    }
    Ok(())
}
```

- [ ] **Step 3: Manual verify**

Draw → Ctrl+Z removes; Ctrl+Y redoes; click a shape → Delete removes; Esc closes editor.

- [ ] **Step 4: Commit**

```bash
git add src/windows/editor/App.vue src-tauri/src/ipc/commands.rs
git commit -m "feat(editor): keyboard shortcuts (Ctrl+Z/Y, Esc, Delete)"
```

---

## Phase E — Output actions

### Task 30: ActionBar UI

**Files:**
- Create: `src/windows/editor/ui/ActionBar.vue`

- [ ] **Step 1: Implement**

```vue
<script setup lang="ts">
import { call } from "../../../shared/ipc";
import type { FinishAction, FinishOutcome } from "../../../shared/types";

const stageRef = inject("stage") as { value: any };  // injected from App via provide()
// Simpler: emit an event and let App orchestrate. For MVP, use a window-global ref.

async function exportPng(): Promise<Uint8Array> {
  const stage = (window as any).__editorStage;
  const dataUrl: string = stage.toDataURL({ pixelRatio: 1 });
  const res = await fetch(dataUrl);
  const buf = await res.arrayBuffer();
  return new Uint8Array(buf);
}

async function doAction(action: FinishAction) {
  const bytes = Array.from(await exportPng());
  try {
    const outcome = await call<FinishOutcome>("finish_action",
      { action, imageBytes: bytes });
    console.log("done", outcome);
  } catch (e: any) {
    alert(e.message ?? String(e));  // replaced with toast in Task 34
  }
}

async function copyImage() { doAction({ kind: "CopyImage" }); }
async function saveAs() {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({
    defaultPath: "screenshot.png",
    filters: [{ name: "Image", extensions: ["png", "jpg"] }],
  });
  if (path) doAction({ kind: "Save", path });
}
async function saveAndCopy() { doAction({ kind: "SaveAndCopyPath" }); }
</script>

<template>
  <div class="action-bar">
    <button @click="copyImage">Copy</button>
    <button @click="saveAs">Save...</button>
    <button @click="saveAndCopy">Save+Copy</button>
  </div>
</template>

<style scoped>
.action-bar { display: flex; justify-content: center; gap: 12px; padding: 8px; border-top: 1px solid #ddd; }
.action-bar button { padding: 8px 20px; background: #3b82f6; color: white; border: none; border-radius: 4px; cursor: pointer; }
.action-bar button:hover { background: #2563eb; }
</style>
```

In `App.vue` after stage mounts, expose:

```ts
import { ref } from "vue";
const stageRef = ref<any>(null);
// In template: <Stage ref="stageRef" ... />
// After mount:
onMounted(() => {
  // Wait for child to expose:
  setTimeout(() => {
    (window as any).__editorStage = stageRef.value?.getStage();
  }, 0);
});
```

(Cleaner: pass `stage` via a `provide/inject` keyed symbol. MVP uses the window global for brevity.)

- [ ] **Step 2: Commit**

```bash
git add src/windows/editor/ui/ActionBar.vue src/windows/editor/App.vue
git commit -m "feat(editor): ActionBar with Copy / Save / Save+Copy buttons"
```

---

### Task 31: Implement `finish_action` Copy variant

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`

- [ ] **Step 1: Implement Copy branch**

```rust
#[tauri::command]
pub fn finish_action(
    action: FinishAction,
    image_bytes: Vec<u8>,
    state: State<AppState>,
    app: AppHandle,
) -> Result<FinishOutcome, AppError> {
    let clipboard = crate::clipboard::PlatformClipboard::new();
    match action {
        FinishAction::CopyImage => {
            clipboard.write_image(&image_bytes)?;
            finalize(&app, &state, FinishOutcome { saved_path: None })
        }
        FinishAction::Save { path } => {
            crate::fs::save::write_image_file(&path, &image_bytes)?;
            *state.last_save_dir.lock().unwrap() = path.parent().map(|p| p.to_path_buf());
            finalize(&app, &state, FinishOutcome { saved_path: Some(path) })
        }
        FinishAction::SaveAndCopyPath => {
            let cfg = state.config.lock().unwrap().clone();
            crate::fs::save::validate_writable_dir(&cfg.default_save_path)?;
            let filename = crate::fs::filename::now_filename(cfg.image_format.extension());
            let path = cfg.default_save_path.join(filename);
            crate::fs::save::write_image_file(&path, &image_bytes)?;
            clipboard.write_file_paths(&[path.clone()])?;
            finalize(&app, &state, FinishOutcome { saved_path: Some(path) })
        }
    }
}

fn finalize(app: &AppHandle, state: &State<AppState>, outcome: FinishOutcome)
    -> Result<FinishOutcome, AppError> {
    let mut phase = state.phase.lock().unwrap();
    let _ = phase.transition(crate::state::PhaseEvent::ActionFinished);
    drop(phase);
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    if let Some(editor) = app.get_webview_window("editor") {
        let _ = editor.hide();
    }
    let _ = app.emit("action-complete", serde_json::json!({ "saved_path": outcome.saved_path }));
    Ok(outcome)
}
```

- [ ] **Step 2: Manual verify**

Hotkey → select → editor → Copy → editor closes → paste into Paint / Notepad → image appears. Try Save → file appears at chosen path. Try Save+Copy → file appears at default path; paste into File Explorer → file appears.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ipc/commands.rs
git commit -m "feat(actions): Copy / Save / Save+Copy with proper finalize"
```

---

### Task 32: Hotkey re-registration on settings update

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/state.rs` (hold hotkey handle in state)

- [ ] **Step 1: Store hotkey service in AppState**

```rust
use crate::hotkey::PlatformHotkey;
use std::sync::Mutex;

pub struct AppState {
    // ... existing ...
    pub hotkey: Mutex<PlatformHotkey>,
}
```

Wire constructor + initial registration in `main.rs` setup.

- [ ] **Step 2: Update `update_config` to re-register**

```rust
#[tauri::command]
pub fn update_config(new: Config, state: State<AppState>, app: AppHandle)
    -> Result<(), AppError> {
    let old = state.config.lock().unwrap().clone();
    let hotkey_changed = new.hotkey != old.hotkey;

    if hotkey_changed {
        let mut hk = state.hotkey.lock().unwrap();
        match hk.register(&new.hotkey) {
            Ok(()) => {}
            Err(e) => {
                let _ = app.emit("hotkey-conflict",
                    serde_json::json!({ "attempted": new.hotkey,
                                        "reason": e.to_string() }));
                // rollback: re-register old hotkey
                let _ = hk.register(&old.hotkey);
                return Err(e.into());
            }
        }
    }

    crate::config::store::save(&state.config_path, &new)?;
    *state.config.lock().unwrap() = new;
    Ok(())
}
```

- [ ] **Step 3: Manual verify**

Open Settings → change hotkey to `Ctrl+Alt+Q` → Save & Apply → new hotkey works, old doesn't. Try to set a conflicting combo (e.g. `Win+E`) → red error appears, old hotkey still functions.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat(settings): re-register hotkey on save + rollback on conflict"
```

---

## Phase F — Polish

### Task 33: Toast component + error wiring

**Files:**
- Create: `src/shared/Toast.vue`
- Create: `src/shared/toast.ts`
- Modify: each window App to mount toast host

- [ ] **Step 1: Toast store**

`src/shared/toast.ts`:

```ts
import { reactive } from "vue";

export interface Toast { id: number; level: "info" | "error" | "success"; msg: string; }

let nextId = 0;
export const toastState = reactive({ list: [] as Toast[] });

export function pushToast(level: Toast["level"], msg: string, ttlMs = 3000) {
  const t: Toast = { id: ++nextId, level, msg };
  toastState.list.push(t);
  setTimeout(() => {
    toastState.list = toastState.list.filter(x => x.id !== t.id);
  }, ttlMs);
}
```

- [ ] **Step 2: Toast component**

`src/shared/Toast.vue`:

```vue
<script setup lang="ts">
import { toastState } from "./toast";
</script>

<template>
  <div class="toast-host">
    <div v-for="t in toastState.list" :key="t.id" :class="['toast', t.level]">
      {{ t.msg }}
    </div>
  </div>
</template>

<style scoped>
.toast-host { position: fixed; bottom: 20px; right: 20px; display: flex; flex-direction: column; gap: 8px; z-index: 9999; }
.toast { padding: 8px 14px; border-radius: 4px; color: white; box-shadow: 0 4px 12px rgba(0,0,0,0.2); }
.toast.error { background: #b91c1c; }
.toast.success { background: #15803d; }
.toast.info { background: #1f2937; }
</style>
```

- [ ] **Step 3: Replace `alert()` and `console.error` with `pushToast`**

In `ActionBar.vue`:

```ts
import { pushToast } from "../../../shared/toast";
// catch block:
catch (e: any) {
  pushToast("error", e.message ?? String(e));
}
```

In Settings `apply()`, in `App.vue` error handlers, etc.

Mount `<Toast />` in editor App.vue, settings App.vue, overlay App.vue (overlay rarely shows toast but harmless).

- [ ] **Step 4: Commit**

```bash
git add src/shared/Toast.vue src/shared/toast.ts src/windows/
git commit -m "feat(ui): toast notifications replacing alert/console errors"
```

---

### Task 34: Tracing logger + panic handler

**Files:**
- Create: `src-tauri/src/logging.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement logger setup**

```rust
use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(log_dir: PathBuf) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    std::fs::create_dir_all(&log_dir).ok()?;
    let appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "minipaste.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);
    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(nb).with_ansi(false))
        .with(env)
        .init();
    Some(guard)
}

pub fn install_panic_handler(log_path_hint: PathBuf) {
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "panic", "{}", info);
        // Optional: write a marker file the next run can detect.
        let _ = std::fs::write(
            log_path_hint.parent().unwrap_or(std::path::Path::new(".")).join("LAST_CRASH"),
            info.to_string(),
        );
    }));
}
```

- [ ] **Step 2: Wire in `main.rs`**

```rust
fn main() {
    let app_data = dirs::config_dir().expect("config dir").join("minipaste");
    let log_dir = app_data.join("logs");
    let _guard = minipaste::logging::init(log_dir.clone());
    minipaste::logging::install_panic_handler(log_dir);
    // ... rest of main
}
```

- [ ] **Step 3: Manual verify**

Inspect `%APPDATA%/minipaste/logs/minipaste.log` after running — INFO entries appear.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/logging.rs src-tauri/src/main.rs
git commit -m "feat(logging): tracing file logger + panic handler"
```

---

### Task 35: E2E happy path test (Playwright + Tauri WebDriver)

**Files:**
- Create: `tests/e2e/happy-path.spec.ts`
- Create: `playwright.config.ts`

- [ ] **Step 1: Install Playwright**

```bash
npm install -D @playwright/test
npx playwright install --with-deps chromium
```

- [ ] **Step 2: Configure Playwright to target Tauri WebDriver**

`playwright.config.ts`:

```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 30000,
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  webServer: {
    command: "npm run tauri dev",
    port: 1420,
    timeout: 60000,
    reuseExistingServer: !process.env.CI,
  },
});
```

- [ ] **Step 3: Write happy-path test**

`tests/e2e/happy-path.spec.ts`:

```ts
import { test, expect } from "@playwright/test";

test("settings panel loads with default config", async ({ page }) => {
  await page.goto("/settings.html");
  await expect(page.locator(".settings h2")).toHaveText("Settings");
  await expect(page.locator(".hotkey-input")).toBeVisible();
});

test("hotkey recorder captures combo", async ({ page }) => {
  await page.goto("/settings.html");
  await page.locator(".hotkey-input").focus();
  await page.keyboard.press("Control+Shift+S");
  await expect(page.locator(".hotkey-input")).toHaveValue("Ctrl+Shift+S");
});

// Full hotkey → capture → editor → action flow needs Tauri WebDriver beyond
// scope of this initial E2E — covered by manual test checklist.
```

> Note: testing the Rust-side flow (global hotkey, tray, screenshot) requires Tauri WebDriver and platform-level fixtures beyond MVP scope. The two browser-level tests above protect the UI shell.

- [ ] **Step 4: Run**

```bash
npx playwright test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add tests/ playwright.config.ts package.json
git commit -m "test: Playwright UI shell tests for Settings + HotkeyRecorder"
```

---

### Task 36: Manual testing checklist runthrough

**Files:**
- Create: `docs/manual-test-checklist.md` (single-page checklist for tester)

- [ ] **Step 1: Author checklist**

```markdown
# minipaste Manual Test Checklist

Run before each release. Tick each row.

## Capture & Selection
- [ ] Hotkey (default Ctrl+Shift+S) opens overlay
- [ ] Overlay covers all monitors (test on dual-monitor setup if available)
- [ ] Drag selects a region; release opens editor
- [ ] Esc on overlay cancels and returns to idle
- [ ] Pressing hotkey twice rapidly does not crash (state machine ignores 2nd)

## Editor Tools
For each tool (line, rect, arrow, mosaic, text):
- [ ] Pick tool → drag/click → shape appears
- [ ] Color 1-5 produces correct color
- [ ] Thickness thin/medium/thick produces visible difference
- [ ] Ctrl+Z removes last shape; Ctrl+Y restores
- [ ] Click existing shape → transformer handles → drag → resize → release → undo restores

## Mosaic
- [ ] Apply over text → text becomes pixelated
- [ ] Different thickness → different block size
- [ ] Mosaic shape is not draggable (intentional MVP scope)

## Text
- [ ] Click text tool → click on canvas → textarea opens
- [ ] Ctrl+Enter commits
- [ ] Esc cancels
- [ ] Blur commits

## Actions
- [ ] Copy → paste into Paint → image appears
- [ ] Save → dialog opens → choose path → file appears at path
- [ ] Save+Copy → file at default path; paste in File Explorer → file pastes

## Settings
- [ ] Open settings via tray left-click
- [ ] Change hotkey → Save & Apply → new hotkey works
- [ ] Set conflict hotkey (e.g. Win+E) → red error, old hotkey still works
- [ ] Change default folder → next Save+Copy lands in new folder
- [ ] Toggle PNG/JPEG → next Save+Copy file has correct extension
- [ ] Delete default folder externally → Save+Copy shows error toast, does not crash

## Tray
- [ ] Tray icon visible in Windows notification area
- [ ] Left-click opens Settings
- [ ] Right-click menu shows: Capture / Settings... / Quit
- [ ] Quit closes app, tray icon disappears

## Edge cases
- [ ] Disconnect a monitor mid-session → next capture still works
- [ ] Lock screen + unlock → hotkey still registered
- [ ] Restart app → settings persisted

## Logs
- [ ] `%APPDATA%/minipaste/logs/minipaste.log` exists and has entries
- [ ] After deliberate panic (debug build), `LAST_CRASH` file is created
```

- [ ] **Step 2: Commit**

```bash
git add docs/manual-test-checklist.md
git commit -m "docs: manual testing checklist for release readiness"
```

---

### Task 37: Release build (portable + MSI)

**Files:**
- Modify: `src-tauri/tauri.conf.json` (bundle config)

- [ ] **Step 1: Configure bundle**

In `tauri.conf.json`:

```json
"bundle": {
  "active": true,
  "targets": ["msi", "nsis"],
  "category": "Productivity",
  "publisher": "minipaste",
  "windows": {
    "wix": { "language": ["en-US"] }
  }
}
```

- [ ] **Step 2: Build release**

```bash
npm run tauri build
```

Expected: artifacts under `src-tauri/target/release/bundle/`:
- `msi/minipaste_0.1.0_x64_en-US.msi`
- `nsis/minipaste_0.1.0_x64-setup.exe`

- [ ] **Step 3: Smoke test on a clean Windows VM (or just clean user dir)**

- [ ] **Step 4: Tag release**

```bash
git tag v0.1.0
```

- [ ] **Step 5: Commit configuration changes**

```bash
git add src-tauri/tauri.conf.json
git commit -m "build: configure MSI + NSIS bundle targets"
```

---

## Spec Coverage Check

| Spec section | Task(s) | Notes |
|---|---|---|
| § 1 目標與範圍 | All | MVP scope honoured; history & autostart excluded |
| § 2 技術棧 | 1, 2, 13 | Tauri/Vue/Konva, all deps in Task 2 |
| § 3 高層架構 | 1, 3, 14 | 4 windows configured; Rust trait modules |
| § 4 主流程 | 18, 19, 21, 31 | Hotkey → capture → overlay → editor → action |
| § 5 編輯器 | 22-29 | Konva stage, 5 tools, undo/redo, transformer |
| § 6 設定 + IPC | 14, 15, 16, 32 | Config form, hotkey recorder, IPC commands, re-register |
| § 7 錯誤處理 | 12, 33, 34 | AppError, toast, logging, panic handler |
| § 8 測試策略 | All tasks have tests; 35 = E2E; 36 = manual | Trait-mockable Rust modules |
| § 9 Platform scope | 3, 8, 9, 10, 17 | All platform-specific modules with Windows impl + Mac stub |
| § 10 開放議題 | (deferred) | Visual assets, bundle format chosen (Task 37 = MSI + NSIS) |

**Closed open issues (§ 10):**
- Bundle: MSI + NSIS (Task 37). MSIX deferred.
- Tauri sidecar: not used (in-process Rust).

**Still deferred:**
- Visual asset polish (icons placeholder; designer-supplied later)
- Tailwind vs CSS modules → scoped CSS chosen for MVP simplicity (Tasks 14, 22, 24, 30, 33)

---

## Self-review notes

- All shape types in `src/shared/types.ts` match what `drawTools.ts` / `Stage.vue` consume.
- `FinishAction` enum matches Rust definition (Task 12 + 31).
- `AppState` field changes accumulate across Tasks 11, 16, 32 — each task explicitly modifies state.rs.
- Mosaic dragging intentionally disabled in MVP (Task 28 note).
- HotkeyRecorder format must match `global-hotkey` parser (e.g. "Ctrl+Shift+S"). If `HotKey::from_str` rejects a combo, Task 10's `Invalid` error surfaces; Settings shows the message via Task 33's toast.
