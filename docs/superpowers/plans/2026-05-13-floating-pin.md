# Floating Pin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Snipaste-style paste-pin feature — a dedicated hotkey reads the clipboard, materializes the contents as an image, and spawns an always-on-top floating Tauri window at the cursor position. Pins are draggable, OS-resizable, multi-instance, and close via right-click / double-click.

**Architecture:** One dynamic Tauri `WebviewWindow` per pin (label `pin-{n}`). Rust owns clipboard read, content classification, text rasterization, and window lifecycle. Frontend pin window is a minimal `<img>` plus three handlers. Payload delivered via `initialization_script` to avoid the listener-race the project previously hit.

**Tech Stack:** Rust + Tauri 2, `ab_glyph` for text rasterization, `image` crate already in deps, `clipboard-win` extended on the read side. Vue 3 single-file component for the pin window, no new JS dependencies.

**Reference Spec:** `docs/superpowers/specs/2026-05-13-floating-pin-design.md`

---

## File Map

**Rust — new:**
- `src-tauri/src/pin/mod.rs` — module surface
- `src-tauri/src/pin/registry.rs` — id counter + active label set + 30-pin cap
- `src-tauri/src/pin/text_to_image.rs` — text → PNG via `ab_glyph`
- `src-tauri/src/pin/service.rs` — `paste_from_clipboard`, `spawn_pin`
- `src-tauri/capabilities/pin.json` — ACL for `pin-*` windows

**Rust — modify:**
- `src-tauri/Cargo.toml` — add `ab_glyph`
- `src-tauri/src/clipboard/mod.rs` — add `PasteContent` enum + trait method
- `src-tauri/src/clipboard/windows.rs` — impl `read_paste_content`
- `src-tauri/src/clipboard/macos.rs` — stub for non-Windows builds
- `src-tauri/src/config/model.rs` — add `paste_pin_hotkey` field
- `src-tauri/src/config/defaults.rs` — default `Ctrl+Shift+V`
- `src-tauri/src/hotkey/mod.rs` — add `HotkeyKind` enum
- `src-tauri/src/hotkey/windows.rs` — multi-slot registration (HashMap)
- `src-tauri/src/hotkey/listener.rs` — dispatch by `event.id` against registered slots
- `src-tauri/src/ipc/commands.rs` — add `pin_close`
- `src-tauri/src/state.rs` — change `hotkey` slot type (single → multi)
- `src-tauri/src/lib.rs` — register paste-pin hotkey; CloseRequested allows `pin-*` close
- `src-tauri/capabilities/default.json` — keep as-is (default capability is for the three pre-declared windows)

**Frontend — new:**
- `pin.html` — entry with `data-window="pin"`
- `src/windows/pin/App.vue` — image render + drag + close
- `src/windows/pin/pin.css` — transparent body, no scrollbar
- `src/__tests__/pin.test.ts` — vitest mount test

**Frontend — modify:**
- `src/main.ts` — add `case "pin"`
- `src/shared/types.ts` — add `paste_pin_hotkey` to `Config`
- `src/windows/settings/App.vue` — second `HotkeyRecorder` row
- `vite.config.ts` — add `pin.html` to rollup inputs (if needed)

**Docs — modify:**
- `docs/manual-test-checklist.md` — append paste-pin section (file may not exist; create minimally if missing)

---

## Task Ordering

Sequential dependencies, but tests inside a task can be batched.

1. Deps + ACL skeleton
2. Config field
3. Pin registry
4. Clipboard `PasteContent` (shared types)
5. Windows clipboard read impl
6. Text-to-image
7. Pin service (`spawn_pin`)
8. Hotkey multi-slot refactor
9. IPC `pin_close` + lib.rs wiring
10. Frontend entry + App.vue
11. Settings UI second row
12. Manual checklist + smoke test
13. Final commit + clean-up

---

## Task 1: Add `ab_glyph` dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependency**

In `[dependencies]`:

```toml
ab_glyph = "0.2"
```

- [ ] **Step 2: Verify build still compiles**

Run from `src-tauri/`:
```
cargo check
```
Expected: clean compile, only new crate downloaded.

- [ ] **Step 3: Commit**

```
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build: add ab_glyph for pin text rasterization"
```

---

## Task 2: Extend `Config` with `paste_pin_hotkey`

**Files:**
- Modify: `src-tauri/src/config/model.rs`
- Modify: `src-tauri/src/config/defaults.rs`
- Modify: `src/shared/types.ts`

- [ ] **Step 1: Add failing test for default + serde**

Replace the test module at the bottom of `src-tauri/src/config/model.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serde_roundtrip() {
        let cfg = Config {
            schema_version: 1,
            hotkey: "Ctrl+Shift+S".into(),
            paste_pin_hotkey: "Ctrl+Shift+V".into(),
            default_save_path: PathBuf::from("C:/temp"),
            image_format: ImageFormat::Png,
            jpeg_quality: 90,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn legacy_config_missing_paste_pin_hotkey_uses_default() {
        // Older configs on disk will not have the new field.
        let json = r#"{
            "schema_version": 1,
            "hotkey": "Ctrl+Shift+S",
            "default_save_path": "C:/temp",
            "image_format": "png",
            "jpeg_quality": 90
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.paste_pin_hotkey, "Ctrl+Shift+V");
    }

    #[test]
    fn image_format_parses_lowercase() {
        let json = r#""jpeg""#;
        assert_eq!(
            serde_json::from_str::<ImageFormat>(json).unwrap(),
            ImageFormat::Jpeg
        );
    }
}
```

- [ ] **Step 2: Run test, expect failure**

```
cd src-tauri && cargo test -p minipaste --lib config::model::tests
```
Expected: FAIL (field missing).

- [ ] **Step 3: Add field to `Config`**

Replace the struct in `src-tauri/src/config/model.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub hotkey: String,
    #[serde(default = "default_paste_pin_hotkey")]
    pub paste_pin_hotkey: String,
    pub default_save_path: PathBuf,
    pub image_format: ImageFormat,
    pub jpeg_quality: u8,
}

fn default_paste_pin_hotkey() -> String {
    "Ctrl+Shift+V".to_string()
}
```

- [ ] **Step 4: Update `default_config()`**

Replace the body of `default_config` in `src-tauri/src/config/defaults.rs`:

```rust
pub fn default_config() -> Config {
    Config {
        schema_version: CURRENT_SCHEMA_VERSION,
        hotkey: "Ctrl+Shift+S".to_string(),
        paste_pin_hotkey: "Ctrl+Shift+V".to_string(),
        default_save_path: dirs::picture_dir().unwrap_or_else(|| PathBuf::from(".")),
        image_format: ImageFormat::Png,
        jpeg_quality: 90,
    }
}
```

- [ ] **Step 5: Run tests, expect pass**

```
cargo test -p minipaste --lib config::model::tests
```
Expected: 3 pass.

- [ ] **Step 6: Mirror in TS types**

Edit `src/shared/types.ts`, in the `Config` interface insert after `hotkey`:

```ts
  paste_pin_hotkey: string;
```

- [ ] **Step 7: Verify frontend tsc**

```
npm run build
```
Expected: build succeeds (or fails only on places we haven't touched yet — keep an eye on Settings/Editor usage of `Config`).

- [ ] **Step 8: Commit**

```
git add src-tauri/src/config/ src/shared/types.ts
git commit -m "feat(config): add paste_pin_hotkey field with Ctrl+Shift+V default"
```

---

## Task 3: Pin registry

**Files:**
- Create: `src-tauri/src/pin/mod.rs`
- Create: `src-tauri/src/pin/registry.rs`
- Modify: `src-tauri/src/lib.rs` (declare module)

- [ ] **Step 1: Declare module in lib.rs**

Add to the top of `src-tauri/src/lib.rs`, after `pub mod ipc;`:

```rust
pub mod pin;
```

- [ ] **Step 2: Create module surface**

Write `src-tauri/src/pin/mod.rs`:

```rust
pub mod registry;
```

- [ ] **Step 3: Write failing tests**

Create `src-tauri/src/pin/registry.rs`:

```rust
use std::collections::HashSet;
use std::sync::Mutex;

pub const MAX_PINS: usize = 30;

#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("pin limit reached ({0})")]
    Full(usize),
}

pub struct PinRegistry {
    next_id: Mutex<u32>,
    active: Mutex<HashSet<String>>,
}

impl PinRegistry {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(0),
            active: Mutex::new(HashSet::new()),
        }
    }

    pub fn reserve(&self) -> Result<String, RegistryError> {
        let mut active = self.active.lock().unwrap();
        if active.len() >= MAX_PINS {
            return Err(RegistryError::Full(MAX_PINS));
        }
        let mut id = self.next_id.lock().unwrap();
        let label = format!("pin-{}", *id);
        *id = id.wrapping_add(1);
        active.insert(label.clone());
        Ok(label)
    }

    pub fn release(&self, label: &str) {
        self.active.lock().unwrap().remove(label);
    }

    pub fn len(&self) -> usize {
        self.active.lock().unwrap().len()
    }
}

impl Default for PinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_returns_unique_monotonic_labels() {
        let r = PinRegistry::new();
        let a = r.reserve().unwrap();
        let b = r.reserve().unwrap();
        assert_eq!(a, "pin-0");
        assert_eq!(b, "pin-1");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn release_drops_from_active() {
        let r = PinRegistry::new();
        let a = r.reserve().unwrap();
        r.release(&a);
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn reserve_caps_at_max() {
        let r = PinRegistry::new();
        for _ in 0..MAX_PINS {
            r.reserve().unwrap();
        }
        assert!(matches!(r.reserve(), Err(RegistryError::Full(_))));
    }
}
```

- [ ] **Step 4: Run tests, expect pass**

```
cargo test -p minipaste --lib pin::registry
```
Expected: 3 pass.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/pin/ src-tauri/src/lib.rs
git commit -m "feat(pin): add registry with 30-pin cap"
```

---

## Task 4: `PasteContent` enum + Clipboard trait extension

**Files:**
- Modify: `src-tauri/src/clipboard/mod.rs`
- Modify: `src-tauri/src/clipboard/macos.rs`

- [ ] **Step 1: Add enum and trait method to mod.rs**

Replace `src-tauri/src/clipboard/mod.rs`:

```rust
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::WindowsClipboard as PlatformClipboard;
#[cfg(not(target_os = "windows"))]
pub use macos::MacosClipboard as PlatformClipboard;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteContent {
    Image(Vec<u8>),
    Text(String),
    FilePath(PathBuf),
    Empty,
}

pub trait Clipboard {
    fn write_image(&self, png_bytes: &[u8]) -> Result<(), ClipboardError>;
    fn write_file_paths(&self, paths: &[PathBuf]) -> Result<(), ClipboardError>;
    fn read_paste_content(&self) -> Result<PasteContent, ClipboardError>;
}

#[derive(thiserror::Error, Debug)]
pub enum ClipboardError {
    #[error("clipboard busy after retries")]
    Busy,
    #[error("backend error: {0}")]
    Backend(String),
}
```

- [ ] **Step 2: Stub macos backend so non-Windows builds compile**

In `src-tauri/src/clipboard/macos.rs`, add inside the `impl Clipboard for MacosClipboard` block (or extend existing impl — read file first to see exact shape; only add the method if not present):

```rust
    fn read_paste_content(&self) -> Result<super::PasteContent, super::ClipboardError> {
        unimplemented!("paste-pin not supported on non-Windows builds")
    }
```

If `MacosClipboard` does not implement `Clipboard` yet, leave the file as-is — it is gated behind `#[cfg(not(target_os = "windows"))]` and only matters for cross-compile checks.

- [ ] **Step 3: Add a Windows stub so it compiles**

In `src-tauri/src/clipboard/windows.rs`, inside `impl Clipboard for WindowsClipboard`, append a stub:

```rust
    fn read_paste_content(&self) -> Result<super::PasteContent, ClipboardError> {
        Ok(super::PasteContent::Empty) // real impl in Task 5
    }
```

- [ ] **Step 4: Verify compile**

```
cd src-tauri && cargo check
```
Expected: clean.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/clipboard/
git commit -m "feat(clipboard): add PasteContent enum + read_paste_content trait method"
```

---

## Task 5: Windows clipboard `read_paste_content` impl

**Files:**
- Modify: `src-tauri/src/clipboard/windows.rs`

- [ ] **Step 1: Replace the stub with the real implementation**

In `src-tauri/src/clipboard/windows.rs`, replace the stub from Task 4 with this. Also adjust imports at the top: add `use clipboard_win::{formats, Getter};` next to the existing `use clipboard_win::Setter;`.

```rust
    fn read_paste_content(&self) -> Result<super::PasteContent, ClipboardError> {
        use super::PasteContent;
        use std::path::PathBuf;

        // Open clipboard once for the whole probe sequence to avoid races.
        let _clip = clipboard_win::Clipboard::new_attempts(10)
            .map_err(|e| ClipboardError::Backend(e.to_string()))?;

        // 1. Try DIB image.
        let mut buf: Vec<u8> = Vec::new();
        if formats::Bitmap.read_clipboard(&mut buf).is_ok() && !buf.is_empty() {
            return Ok(PasteContent::Image(buf));
        }

        // 2. Try plain text.
        let mut text = String::new();
        if formats::Unicode.read_clipboard(&mut text).is_ok() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Ok(PasteContent::Text(text));
            }
        }

        // 3. Try FileList — first entry only.
        let mut files: Vec<String> = Vec::new();
        if formats::FileList.read_clipboard(&mut files).is_ok() {
            if let Some(first) = files.into_iter().next() {
                return Ok(PasteContent::FilePath(PathBuf::from(first)));
            }
        }

        Ok(PasteContent::Empty)
    }
```

Notes:
- `formats::Bitmap` returns a BMP-format byte stream from CF_DIB. Callers must accept that and decode via `image::load_from_memory` (which handles BMP).
- `clipboard_win` 5.x APIs may differ slightly; if `read_clipboard` is not the actual method, check the docs (`cargo doc -p clipboard-win --open`) and adapt — the structure of "try image → text → files" stays the same. Common alternatives are `formats::Bitmap.read_clipboard(&mut buf)` vs `clipboard_win::get_clipboard(formats::Bitmap)`.

- [ ] **Step 2: Verify compile**

```
cd src-tauri && cargo check
```
Expected: clean. If `read_clipboard` signature is wrong, fix per the actual `clipboard-win` 5.x API surface.

- [ ] **Step 3: Manual smoke (deferred to Task 12)**

Real clipboard reads cannot be unit-tested without a Windows host. Marker: covered by manual checklist.

- [ ] **Step 4: Commit**

```
git add src-tauri/src/clipboard/windows.rs
git commit -m "feat(clipboard): implement read_paste_content (image/text/file path)"
```

---

## Task 6: Text-to-image renderer

**Files:**
- Create: `src-tauri/src/pin/text_to_image.rs`
- Modify: `src-tauri/src/pin/mod.rs`

- [ ] **Step 1: Add module declaration**

In `src-tauri/src/pin/mod.rs`:

```rust
pub mod registry;
pub mod text_to_image;
```

- [ ] **Step 2: Write failing tests + skeleton**

Create `src-tauri/src/pin/text_to_image.rs`:

```rust
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

const FONT_SIZE: f32 = 16.0;
const PADDING: u32 = 12;
const BG: [u8; 4] = [0x1f, 0x1f, 0x1f, 0xff];
const FG: [u8; 4] = [0xe5, 0xe5, 0xe5, 0xff];
const MAX_LINES: usize = 200;
const MAX_LINE_CHARS: usize = 200;
const TRUNCATE_SUFFIX: &str = "⋯（截斷）";

#[derive(thiserror::Error, Debug)]
pub enum TextRenderError {
    #[error("font load failed: {0}")]
    Font(String),
    #[error("image encode failed: {0}")]
    Encode(String),
    #[error("empty text after trim")]
    Empty,
}

/// Renders the given text to a PNG byte stream.
///
/// Font strategy:
/// - On Windows, try `C:\Windows\Fonts\msyh.ttc` (Microsoft YaHei) for CJK + Latin.
/// - Fall back to embedded `DejaVuSans.ttf` if available, else return Font error.
pub fn render(text: &str) -> Result<Vec<u8>, TextRenderError> {
    let trimmed = text.trim_end();
    if trimmed.trim().is_empty() {
        return Err(TextRenderError::Empty);
    }

    let font = load_font()?;
    let scale = PxScale::from(FONT_SIZE);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_h = (ascent - descent + scaled.line_gap()).ceil() as u32;

    let (lines, capped) = wrap_lines(trimmed);
    let visible: Vec<String> = if capped {
        let mut v: Vec<String> = lines.into_iter().take(MAX_LINES).collect();
        if let Some(last) = v.last_mut() {
            last.push_str(TRUNCATE_SUFFIX);
        }
        v
    } else {
        lines
    };

    // Width = widest line.
    let mut max_w_px = 0_f32;
    for line in &visible {
        let mut w = 0.0;
        let mut last: Option<char> = None;
        for ch in line.chars() {
            let gid = font.glyph_id(ch);
            if let Some(prev) = last {
                w += scaled.kern(font.glyph_id(prev), gid);
            }
            w += scaled.h_advance(gid);
            last = Some(ch);
        }
        if w > max_w_px {
            max_w_px = w;
        }
    }
    let img_w = (max_w_px.ceil() as u32) + PADDING * 2;
    let img_h = (visible.len() as u32) * line_h + PADDING * 2;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(img_w.max(1), img_h.max(1), Rgba(BG));

    for (row, line) in visible.iter().enumerate() {
        let baseline_y = PADDING as f32 + (row as f32) * (line_h as f32) + ascent;
        let mut pen_x = PADDING as f32;
        let mut last: Option<ab_glyph::GlyphId> = None;

        for ch in line.chars() {
            let gid = font.glyph_id(ch);
            if let Some(prev) = last {
                pen_x += scaled.kern(prev, gid);
            }
            let glyph = gid.with_scale_and_position(scale, ab_glyph::point(pen_x, baseline_y));
            if let Some(outline) = font.outline_glyph(glyph) {
                let bb = outline.px_bounds();
                outline.draw(|gx, gy, cov| {
                    let px = bb.min.x as i32 + gx as i32;
                    let py = bb.min.y as i32 + gy as i32;
                    if px < 0 || py < 0 || px as u32 >= img_w || py as u32 >= img_h {
                        return;
                    }
                    let base = img.get_pixel(px as u32, py as u32).0;
                    let blended = blend(base, FG, cov);
                    img.put_pixel(px as u32, py as u32, Rgba(blended));
                });
            }
            pen_x += scaled.h_advance(gid);
            last = Some(gid);
        }
    }

    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| TextRenderError::Encode(e.to_string()))?;
    Ok(out.into_inner())
}

fn wrap_lines(text: &str) -> (Vec<String>, bool) {
    let mut out = Vec::new();
    let mut capped = false;
    for line in text.lines() {
        if out.len() >= MAX_LINES {
            capped = true;
            break;
        }
        if line.chars().count() <= MAX_LINE_CHARS {
            out.push(line.to_string());
        } else {
            let chars: Vec<char> = line.chars().collect();
            for chunk in chars.chunks(MAX_LINE_CHARS) {
                if out.len() >= MAX_LINES {
                    capped = true;
                    break;
                }
                out.push(chunk.iter().collect());
            }
        }
    }
    if !capped && text.lines().count() > MAX_LINES {
        capped = true;
    }
    (out, capped)
}

fn blend(base: [u8; 4], fg: [u8; 4], cov: f32) -> [u8; 4] {
    let a = cov.clamp(0.0, 1.0);
    let mix = |b: u8, f: u8| -> u8 {
        ((b as f32) * (1.0 - a) + (f as f32) * a).round() as u8
    };
    [mix(base[0], fg[0]), mix(base[1], fg[1]), mix(base[2], fg[2]), 0xff]
}

#[cfg(target_os = "windows")]
fn load_font() -> Result<FontArc, TextRenderError> {
    use std::fs;
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
    ];
    for p in candidates {
        if let Ok(bytes) = fs::read(p) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Ok(font);
            }
        }
    }
    Err(TextRenderError::Font(
        "no usable Windows font found (tried msyh, segoeui)".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn load_font() -> Result<FontArc, TextRenderError> {
    Err(TextRenderError::Font(
        "paste-pin font loader is Windows-only in MVP".into(),
    ))
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn render_ascii_returns_decodable_png() {
        let png = render("hello world").expect("render");
        assert!(!png.is_empty());
        let img = image::load_from_memory(&png).expect("decode");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn render_cjk_returns_decodable_png() {
        let png = render("你好，世界").expect("render");
        let img = image::load_from_memory(&png).expect("decode");
        assert!(img.width() > 0);
    }

    #[test]
    fn render_empty_errors() {
        assert!(matches!(render("   \n   "), Err(TextRenderError::Empty)));
    }

    #[test]
    fn render_truncates_when_over_max_lines() {
        let many = (0..(MAX_LINES + 10))
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let png = render(&many).expect("render");
        let img = image::load_from_memory(&png).expect("decode");
        // Height should reflect MAX_LINES, not MAX_LINES+10.
        let line_count_visible = (img.height() - 2 * PADDING) / 19; // rough sanity
        assert!(line_count_visible <= MAX_LINES as u32);
    }
}
```

- [ ] **Step 3: Run tests**

```
cd src-tauri && cargo test -p minipaste --lib pin::text_to_image
```
Expected on Windows: 4 pass. On non-Windows, the test module is `cfg`-gated off; `cargo check` should still succeed.

- [ ] **Step 4: Commit**

```
git add src-tauri/src/pin/
git commit -m "feat(pin): render plain text to PNG via ab_glyph (Windows fonts)"
```

---

## Task 7: Pin spawn service

**Files:**
- Create: `src-tauri/src/pin/service.rs`
- Modify: `src-tauri/src/pin/mod.rs`
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: Expose `PinRegistry` on `AppState`**

In `src-tauri/src/state.rs`, add inside `AppState` struct:

```rust
    pub pins: crate::pin::registry::PinRegistry,
```

In `AppState::new`, set:

```rust
            pins: crate::pin::registry::PinRegistry::new(),
```

(`PinRegistry` already uses interior `Mutex`s — no outer `Arc` needed because access is always via `tauri::State<AppState>` references.)

- [ ] **Step 2: Declare submodule**

Update `src-tauri/src/pin/mod.rs`:

```rust
pub mod registry;
pub mod service;
pub mod text_to_image;
```

- [ ] **Step 3: Write the service**

Create `src-tauri/src/pin/service.rs`:

```rust
use crate::clipboard::{Clipboard, PasteContent, PlatformClipboard};
use crate::pin::text_to_image;
use crate::state::AppState;
use base64::Engine;
use std::path::Path;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};

const MAX_PIXELS: u64 = 50_000_000; // 50 MP
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

pub fn paste_from_clipboard(app: &AppHandle) {
    let cb = PlatformClipboard::new();
    let content = match cb.read_paste_content() {
        Ok(c) => c,
        Err(e) => {
            emit_error(app, format!("剪貼簿讀取失敗：{}", e));
            return;
        }
    };

    let png = match content_to_png(content) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            emit_error(app, "剪貼簿是空的".into());
            return;
        }
        Err(msg) => {
            emit_error(app, msg);
            return;
        }
    };

    if let Err(msg) = spawn_pin(app, png) {
        emit_error(app, msg);
    }
}

fn content_to_png(content: PasteContent) -> Result<Option<Vec<u8>>, String> {
    match content {
        PasteContent::Empty => Ok(None),
        PasteContent::Image(bytes) => {
            image::load_from_memory(&bytes)
                .map_err(|e| format!("圖片格式無法解析：{}", e))?;
            // Re-encode to PNG if it wasn't already (CF_DIB → BMP).
            let mut out = std::io::Cursor::new(Vec::new());
            image::load_from_memory(&bytes)
                .map_err(|e| format!("圖片格式無法解析：{}", e))?
                .write_to(&mut out, image::ImageFormat::Png)
                .map_err(|e| format!("PNG 編碼失敗：{}", e))?;
            Ok(Some(out.into_inner()))
        }
        PasteContent::Text(s) => match text_to_image::render(&s) {
            Ok(png) => Ok(Some(png)),
            Err(text_to_image::TextRenderError::Empty) => Ok(None),
            Err(e) => Err(format!("文字渲染失敗：{}", e)),
        },
        PasteContent::FilePath(p) => path_to_png(&p).map(Some),
    }
}

fn path_to_png(path: &Path) -> Result<Vec<u8>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !IMAGE_EXTS.iter().any(|e| *e == ext) {
        return Err(format!(
            "不是圖片：{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| format!("找不到檔案：{}（{}）", path.display(), e))?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("圖片格式無法解析：{}", e))?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("PNG 編碼失敗：{}", e))?;
    Ok(out.into_inner())
}

fn spawn_pin(app: &AppHandle, png_bytes: Vec<u8>) -> Result<(), String> {
    let img =
        image::load_from_memory(&png_bytes).map_err(|e| format!("圖片解碼失敗：{}", e))?;
    let (w, h) = (img.width(), img.height());
    if (w as u64) * (h as u64) > MAX_PIXELS {
        return Err("內容過大".into());
    }

    let (target_w, target_h) = clamp_to_screen(app, w, h);

    let state: tauri::State<AppState> = app.state();
    let label = state
        .pins
        .reserve()
        .map_err(|_| "Pin 上限 30 個".to_string())?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    // Cursor position; fallback to (200,200).
    let (px, py) = match app.cursor_position() {
        Ok(p) => (p.x as f64, p.y as f64),
        Err(_) => (200.0, 200.0),
    };

    let init_script = format!(
        "window.__pinData = {{ label: \"{label}\", image_b64: \"{b64}\", width: {w}, height: {h} }};"
    );

    let build_result = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("pin.html".into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .visible(false)
    .inner_size(target_w, target_h)
    .position(px, py)
    .initialization_script(&init_script)
    .build();

    match build_result {
        Ok(win) => {
            let _ = win.show();
            let _ = win.set_focus();
            tracing::info!("pin spawned: {} ({}x{})", label, target_w, target_h);
            Ok(())
        }
        Err(e) => {
            state.pins.release(&label);
            tracing::error!("pin build failed: {}", e);
            Err("無法建立視窗".into())
        }
    }
}

fn clamp_to_screen(app: &AppHandle, w: u32, h: u32) -> (f64, f64) {
    let monitor = app.primary_monitor().ok().flatten();
    let (mw, mh) = match monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            (
                (size.width as f64) / scale * 0.8,
                (size.height as f64) / scale * 0.8,
            )
        }
        None => (1280.0, 720.0),
    };
    let aspect = (w as f64) / (h as f64);
    let mut tw = w as f64;
    let mut th = h as f64;
    if tw > mw {
        tw = mw;
        th = tw / aspect;
    }
    if th > mh {
        th = mh;
        tw = th * aspect;
    }
    (tw.max(40.0), th.max(40.0))
}

fn emit_error(app: &AppHandle, reason: String) {
    tracing::warn!("pin-error: {}", reason);
    let _ = app.emit("pin-error", serde_json::json!({ "reason": reason }));
}
```

- [ ] **Step 4: Verify compile**

```
cd src-tauri && cargo check
```
Expected: clean.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/pin/ src-tauri/src/state.rs
git commit -m "feat(pin): add spawn service for clipboard → floating window"
```

---

## Task 8: Hotkey multi-slot refactor

**Files:**
- Modify: `src-tauri/src/hotkey/mod.rs`
- Modify: `src-tauri/src/hotkey/windows.rs`
- Modify: `src-tauri/src/hotkey/macos.rs`
- Modify: `src-tauri/src/hotkey/listener.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/ipc/commands.rs` (update_config call site)

### Why this exists

The current `WindowsHotkey` holds a single `current: Option<HotKey>` and the listener treats every event as the capture hotkey. We need two simultaneous registrations (capture + paste-pin) and we must dispatch by `event.id`.

- [ ] **Step 1: Add `HotkeyKind`**

In `src-tauri/src/hotkey/mod.rs`, append:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyKind {
    Capture,
    PastePin,
}
```

Update the trait:

```rust
pub trait HotkeyService: Send + Sync {
    fn register(&mut self, kind: HotkeyKind, combo: &str) -> Result<(), HotkeyError>;
    fn unregister(&mut self, kind: HotkeyKind);
    fn id_of(&self, kind: HotkeyKind) -> Option<u32>;
}
```

- [ ] **Step 2: Multi-slot Windows impl**

Replace `src-tauri/src/hotkey/windows.rs`:

```rust
use super::{HotkeyError, HotkeyKind, HotkeyService};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::collections::HashMap;
use std::str::FromStr;

pub struct WindowsHotkey {
    manager: GlobalHotKeyManager,
    slots: HashMap<HotkeyKind, HotKey>,
}

unsafe impl Send for WindowsHotkey {}
unsafe impl Sync for WindowsHotkey {}

impl WindowsHotkey {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()
                .map_err(|e| HotkeyError::Backend(e.to_string()))?,
            slots: HashMap::new(),
        })
    }

    pub fn event_receiver() -> crossbeam_channel::Receiver<GlobalHotKeyEvent> {
        GlobalHotKeyEvent::receiver().clone()
    }
}

impl HotkeyService for WindowsHotkey {
    fn register(&mut self, kind: HotkeyKind, combo: &str) -> Result<(), HotkeyError> {
        let hk = HotKey::from_str(combo).map_err(|_| HotkeyError::Invalid(combo.into()))?;
        if let Some(prev) = self.slots.remove(&kind) {
            let _ = self.manager.unregister(prev);
        }
        self.manager.register(hk).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("registered") {
                HotkeyError::Conflict
            } else {
                HotkeyError::Backend(msg)
            }
        })?;
        self.slots.insert(kind, hk);
        Ok(())
    }

    fn unregister(&mut self, kind: HotkeyKind) {
        if let Some(prev) = self.slots.remove(&kind) {
            let _ = self.manager.unregister(prev);
        }
    }

    fn id_of(&self, kind: HotkeyKind) -> Option<u32> {
        self.slots.get(&kind).map(|hk| hk.id())
    }
}
```

- [ ] **Step 3: Update macos stub**

Open `src-tauri/src/hotkey/macos.rs`. Update its `register`/`unregister`/add `id_of` signatures to match the new trait. The body can remain `unimplemented!()`. Use `_kind: HotkeyKind` to silence unused-param warnings.

- [ ] **Step 4: Listener dispatches by id**

Replace `src-tauri/src/hotkey/listener.rs`:

```rust
use crate::hotkey::HotkeyKind;
use crate::state::{AppState, PhaseEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let rx = global_hotkey::GlobalHotKeyEvent::receiver();
        while let Ok(event) = rx.recv() {
            // global-hotkey 0.6: only act on Pressed, ignore the Released event.
            if event.state == global_hotkey::HotKeyState::Released {
                continue;
            }
            dispatch(&app, event.id);
        }
    });
}

fn dispatch(app: &AppHandle, event_id: u32) {
    let state: tauri::State<AppState> = app.state();
    let kind_opt = {
        let hk_slot = state.hotkey.lock().unwrap();
        hk_slot.as_ref().and_then(|hk| {
            if hk.id_of(HotkeyKind::Capture) == Some(event_id) {
                Some(HotkeyKind::Capture)
            } else if hk.id_of(HotkeyKind::PastePin) == Some(event_id) {
                Some(HotkeyKind::PastePin)
            } else {
                None
            }
        })
    };
    use crate::hotkey::HotkeyService;
    let Some(kind) = kind_opt else { return };

    match kind {
        HotkeyKind::Capture => {
            let mut phase = state.phase.lock().unwrap();
            if phase.transition(PhaseEvent::HotkeyPressed).is_err() {
                return;
            }
            drop(phase);
            let _ = app.emit("tray://trigger-capture", ());
        }
        HotkeyKind::PastePin => {
            // Paste-pin does not interact with capture phase.
            crate::pin::service::paste_from_clipboard(app);
        }
    }
}
```

- [ ] **Step 5: AppState slot type stays the same**

`state.hotkey: Mutex<Option<PlatformHotkey>>` already holds the multi-slot struct. No change needed.

- [ ] **Step 6: Update `lib.rs` setup**

Replace the hotkey-registration block in `src-tauri/src/lib.rs` `setup` with:

```rust
            use crate::hotkey::{HotkeyKind, HotkeyService};
            let state: tauri::State<AppState> = app.state();
            let (capture_combo, paste_combo) = {
                let cfg = state.config.lock().unwrap();
                (cfg.hotkey.clone(), cfg.paste_pin_hotkey.clone())
            };
            match crate::hotkey::PlatformHotkey::new() {
                Ok(mut hk) => {
                    if let Err(e) = hk.register(HotkeyKind::Capture, &capture_combo) {
                        tracing::warn!("capture hotkey '{}' conflict: {}", capture_combo, e);
                        let _ = app.emit(
                            "hotkey-conflict",
                            serde_json::json!({
                                "kind": "capture",
                                "attempted": capture_combo,
                                "reason": e.to_string(),
                            }),
                        );
                    }
                    if let Err(e) = hk.register(HotkeyKind::PastePin, &paste_combo) {
                        tracing::warn!("paste-pin hotkey '{}' conflict: {}", paste_combo, e);
                        let _ = app.emit(
                            "hotkey-conflict",
                            serde_json::json!({
                                "kind": "paste_pin",
                                "attempted": paste_combo,
                                "reason": e.to_string(),
                            }),
                        );
                    }
                    *state.hotkey.lock().unwrap() = Some(hk);
                }
                Err(e) => {
                    tracing::error!("hotkey init failed: {}", e);
                }
            }
```

- [ ] **Step 7: Update `update_config` to handle both hotkeys**

Replace the hotkey-change block in `src-tauri/src/ipc/commands.rs::update_config`:

```rust
    use crate::hotkey::{HotkeyKind, HotkeyService};

    let old = state.config.lock().unwrap().clone();
    let capture_changed = new.hotkey != old.hotkey;
    let paste_changed = new.paste_pin_hotkey != old.paste_pin_hotkey;

    if capture_changed || paste_changed {
        let mut hk_slot = state.hotkey.lock().unwrap();
        if let Some(hk) = hk_slot.as_mut() {
            if capture_changed {
                if let Err(e) = hk.register(HotkeyKind::Capture, &new.hotkey) {
                    let _ = app.emit(
                        "hotkey-conflict",
                        serde_json::json!({
                            "kind": "capture",
                            "attempted": new.hotkey,
                            "reason": e.to_string(),
                        }),
                    );
                    let _ = hk.register(HotkeyKind::Capture, &old.hotkey);
                    return Err(e.into());
                }
            }
            if paste_changed {
                if let Err(e) = hk.register(HotkeyKind::PastePin, &new.paste_pin_hotkey) {
                    let _ = app.emit(
                        "hotkey-conflict",
                        serde_json::json!({
                            "kind": "paste_pin",
                            "attempted": new.paste_pin_hotkey,
                            "reason": e.to_string(),
                        }),
                    );
                    let _ = hk.register(HotkeyKind::PastePin, &old.paste_pin_hotkey);
                    return Err(e.into());
                }
            }
        }
    }
```

- [ ] **Step 8: Verify**

```
cd src-tauri && cargo check && cargo test -p minipaste --lib state
```
Expected: clean compile, phase tests still pass.

- [ ] **Step 9: Commit**

```
git add src-tauri/src/hotkey/ src-tauri/src/ipc/commands.rs src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "refactor(hotkey): support multi-slot registration, dispatch by event id"
```

---

## Task 9: `pin_close` command + CloseRequested for pin windows

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `pin_close` command**

Append to `src-tauri/src/ipc/commands.rs`:

```rust
#[tauri::command]
pub fn pin_close(label: String, state: State<AppState>, app: AppHandle) -> Result<(), AppError> {
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.close();
    }
    state.pins.release(&label);
    Ok(())
}
```

- [ ] **Step 2: Register the command**

In `src-tauri/src/lib.rs`, extend the `invoke_handler` list:

```rust
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            selection_confirmed,
            selection_cancelled,
            finish_action,
            cancel_edit,
            pin_close,
        ])
```

And in the corresponding `use` line at the top:

```rust
use crate::ipc::commands::{
    cancel_edit, finish_action, get_config, pin_close, selection_cancelled, selection_confirmed,
    update_config,
};
```

- [ ] **Step 3: CloseRequested for pin windows (registry release)**

Currently `lib.rs` intercepts close only for `editor`/`overlay`/`settings`. Pin windows must be allowed to actually close (not hide), but we need to release the registry slot when they do.

Inside the `setup` closure, after the existing `for label in ["editor", "overlay", "settings"]` block, add a listener for newly created pin windows. The cleanest path: hook the registry release inside `pin_close` (already done in Step 1). For user-triggered Alt+F4 or other OS close paths, add a global `on_window_event` once per pin via the builder — but Tauri 2 builders don't expose that hook directly on `WebviewWindowBuilder`, so register it post-build.

In `src-tauri/src/pin/service.rs`, inside `spawn_pin` Ok branch, before `win.show()`:

```rust
            let app_for_event = app.clone();
            let label_for_event = label.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let state: tauri::State<AppState> = app_for_event.state();
                    state.pins.release(&label_for_event);
                    tracing::info!("pin '{}' close requested", label_for_event);
                    // Do NOT prevent_close — pins should truly destroy.
                }
            });
```

(Replace the existing Ok branch in `spawn_pin` to keep the show/focus lines below this hook.)

- [ ] **Step 4: Verify compile**

```
cd src-tauri && cargo check
```
Expected: clean.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/ipc/commands.rs src-tauri/src/lib.rs src-tauri/src/pin/service.rs
git commit -m "feat(pin): add pin_close command + registry release on window destroy"
```

---

## Task 10: Capabilities for `pin-*` windows

**Files:**
- Create: `src-tauri/capabilities/pin.json`

- [ ] **Step 1: Write capability file**

Create `src-tauri/capabilities/pin.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "pin",
  "description": "APIs available to dynamically created pin windows",
  "windows": ["pin-*"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:window:default",
    "core:webview:default"
  ]
}
```

- [ ] **Step 2: Verify the schema matches what Tauri 2 supports**

Run a build to make sure ACL parsing accepts the glob:

```
npx tauri build --no-bundle
```

If Tauri rejects `"pin-*"` (older versions sometimes only support exact labels), fall back: add `"windows": ["pin-*"]` to the existing `default.json` instead, or remove the `windows` field entirely so the default capability applies globally. Document the chosen approach in the commit message.

- [ ] **Step 3: Commit**

```
git add src-tauri/capabilities/
git commit -m "feat(pin): grant core ACL to pin-* windows"
```

---

## Task 11: Frontend entry — `pin.html`, `main.ts`, App.vue, CSS

**Files:**
- Create: `pin.html`
- Create: `src/windows/pin/App.vue`
- Create: `src/windows/pin/pin.css`
- Modify: `src/main.ts`
- Modify: `vite.config.ts`

- [ ] **Step 1: Add `pin.html` to vite inputs**

Read `vite.config.ts` first. If it lists rollup inputs explicitly (likely the case for multi-page Tauri apps), add `pin.html` to the input map. If it does not (vite auto-discovers via filesystem in some setups), `pin.html` at project root is sufficient.

Example diff inside `vite.config.ts` (adapt to actual structure):

```ts
build: {
  rollupOptions: {
    input: {
      // existing entries...
      settings: "settings.html",
      overlay: "overlay.html",
      editor: "editor.html",
      pin: "pin.html",
    },
  },
},
```

- [ ] **Step 2: Create `pin.html`**

At project root, create `pin.html`:

```html
<!doctype html>
<html lang="en" data-window="pin">
  <head>
    <meta charset="UTF-8" />
    <title></title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 3: Extend `main.ts`**

In `src/main.ts`, add a case in the switch:

```ts
    case "pin":
      App = (await import("./windows/pin/App.vue")).default;
      break;
```

- [ ] **Step 4: Create `App.vue`**

Create `src/windows/pin/App.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { call } from "../../shared/ipc";

interface PinData {
  label: string;
  image_b64: string;
  width: number;
  height: number;
}

declare global {
  interface Window {
    __pinData?: PinData;
  }
}

const data = window.__pinData;
const ready = ref(!!data);

async function onMouseDown(e: MouseEvent) {
  // Only left-button starts native window drag.
  if (e.button !== 0) return;
  e.preventDefault();
  try {
    await getCurrentWebviewWindow().startDragging();
  } catch (err) {
    console.error("startDragging failed", err);
  }
}

async function closePin() {
  if (!data) return;
  try {
    await call<void>("pin_close", { label: data.label });
  } catch (err) {
    console.error("pin_close failed", err);
  }
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  closePin();
}

function onDblClick(e: MouseEvent) {
  e.preventDefault();
  closePin();
}

onMounted(() => {
  if (!data) {
    console.error("pin window mounted without __pinData");
  }
});
</script>

<template>
  <div
    v-if="ready && data"
    class="pin-root"
    @mousedown="onMouseDown"
    @contextmenu="onContextMenu"
    @dblclick="onDblClick"
  >
    <img
      :src="`data:image/png;base64,${data.image_b64}`"
      :alt="''"
      draggable="false"
      class="pin-image"
    />
  </div>
</template>

<style scoped src="./pin.css"></style>
```

- [ ] **Step 5: Create CSS**

Create `src/windows/pin/pin.css`:

```css
html,
body,
#app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
}

.pin-root {
  width: 100%;
  height: 100%;
  cursor: grab;
  user-select: none;
  -webkit-user-select: none;
}

.pin-root:active {
  cursor: grabbing;
}

.pin-image {
  width: 100%;
  height: 100%;
  object-fit: fill;
  pointer-events: none;
  display: block;
}
```

- [ ] **Step 6: vitest mount test**

Create `src/__tests__/pin.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ startDragging: vi.fn() }),
}));

vi.mock("../shared/ipc", () => ({
  call: vi.fn().mockResolvedValue(undefined),
}));

import App from "../windows/pin/App.vue";
import { call } from "../shared/ipc";

describe("pin window App.vue", () => {
  beforeEach(() => {
    (window as unknown as { __pinData?: unknown }).__pinData = {
      label: "pin-7",
      image_b64: "AAA=",
      width: 100,
      height: 80,
    };
    vi.clearAllMocks();
  });

  it("renders <img> with data URL when __pinData is present", () => {
    const wrapper = mount(App);
    const img = wrapper.find("img");
    expect(img.exists()).toBe(true);
    expect(img.attributes("src")).toContain("data:image/png;base64,AAA=");
  });

  it("invokes pin_close on dblclick", async () => {
    const wrapper = mount(App);
    await wrapper.find(".pin-root").trigger("dblclick");
    expect(call).toHaveBeenCalledWith("pin_close", { label: "pin-7" });
  });

  it("invokes pin_close on contextmenu", async () => {
    const wrapper = mount(App);
    await wrapper.find(".pin-root").trigger("contextmenu");
    expect(call).toHaveBeenCalledWith("pin_close", { label: "pin-7" });
  });
});
```

- [ ] **Step 7: Run vitest**

```
npm test
```
Expected: 3 new passing tests, no regressions in existing ones.

- [ ] **Step 8: Commit**

```
git add pin.html src/main.ts src/windows/pin/ src/__tests__/pin.test.ts vite.config.ts
git commit -m "feat(pin): add Vue frontend for floating pin window"
```

---

## Task 12: Settings — second `HotkeyRecorder` row

**Files:**
- Modify: `src/windows/settings/App.vue`

- [ ] **Step 1: Add the row**

In `src/windows/settings/App.vue` `<template>`, after the existing Hotkey `<label>` block, add:

```vue
    <label>
      Paste pin hotkey
      <HotkeyRecorder v-model="state.config.paste_pin_hotkey" />
    </label>
```

- [ ] **Step 2: Update the conflict toast to label which hotkey conflicted**

Replace the existing `on<{ attempted; reason }>("hotkey-conflict", …)` handler in the same file:

```ts
  on<{ kind?: string; attempted: string; reason: string }>(
    "hotkey-conflict",
    (p) => {
      const which =
        p.kind === "paste_pin" ? "Paste pin" : "Capture";
      state.error = `${which} hotkey "${p.attempted}" 衝突：${p.reason}`;
    },
  );
```

- [ ] **Step 3: Visual smoke**

Run `npm run dev` and confirm Settings shows two HotkeyRecorder rows. (Manual; no automated test required for the label string itself.)

- [ ] **Step 4: Commit**

```
git add src/windows/settings/App.vue
git commit -m "feat(settings): add paste-pin hotkey recorder row"
```

---

## Task 13: Manual checklist update

**Files:**
- Modify (or create if missing): `docs/manual-test-checklist.md`

- [ ] **Step 1: Append paste-pin section**

If the file does not exist, create it with a minimal header. Otherwise append at the end:

```markdown
## Paste Pin

- [ ] Default hotkey `Ctrl+Shift+V` pins clipboard image (copy a screenshot first)
- [ ] Hotkey pins clipboard text → renders as image (try ASCII + 中文 mixed)
- [ ] Hotkey pins clipboard file path when file is PNG/JPG/GIF/BMP (copy a file in Explorer)
- [ ] Empty clipboard → toast "剪貼簿是空的", no pin
- [ ] Non-image file path → toast "不是圖片：…", no pin
- [ ] Pin spawns at cursor position
- [ ] Pin is always-on-top (verify against fullscreen window)
- [ ] Drag pin by body (cursor: grab)
- [ ] Resize pin by OS corner drag
- [ ] Right-click closes pin
- [ ] Double-click closes pin
- [ ] Multiple pins coexist; closing one leaves the others alone
- [ ] Spawn 5+ pins → all responsive, RAM < 300 MB total
- [ ] Settings: change paste-pin hotkey to `Ctrl+Alt+V` → new hotkey works, old does not
- [ ] Restart app → paste-pin hotkey config persists
```

- [ ] **Step 2: Commit**

```
git add docs/manual-test-checklist.md
git commit -m "docs: add paste-pin manual test checklist"
```

---

## Task 14: End-to-end smoke build and manual run

**Files:** none (manual verification)

- [ ] **Step 1: Build release**

Kill any running `minipaste.exe`:

```
taskkill /F /IM minipaste.exe 2>$null
```

Build (do NOT use `cargo build --release` alone — see `~/.claude/projects/D--SideProject-MiniPaste/memory/tauri-build-trap.md`):

```
npx tauri build --no-bundle
```

- [ ] **Step 2: Run release exe**

```
& "src-tauri/target/release/minipaste.exe"
```

- [ ] **Step 3: Walk the manual checklist**

Tick each box in `docs/manual-test-checklist.md` "Paste Pin" section. Report unticked boxes back as follow-up tasks.

- [ ] **Step 4: Cleanup-only commit if anything was patched**

Only commit if Step 3 surfaced fixes. Otherwise no commit.

---

## Self-Review Notes

- Every spec requirement maps to a task: clipboard read (T4–T5), text-to-image (T6), spawn (T7), hotkey dispatch (T8), close (T9), ACL (T10), frontend (T11), settings (T12), manual coverage (T13–T14).
- No placeholders. Code blocks complete in every step.
- `pin-error` toast destination: the spec said "editor window's Toast." That assumes the editor window is hidden but reachable; since the editor window has `visible: false` until capture, the toast will only show if the user happens to be in an active capture session. **Accepted gap for MVP:** toasts log via `tracing::warn!` (already done in `emit_error`) so the user still has a way to diagnose via log file. Revisit if it becomes annoying.
- Tauri `clipboard-win` 5.x API surface is the most fragile part. Task 5 calls this out and gives a fallback path. If it diverges further, treat that as a sub-task within Task 5 rather than blocking the rest of the plan.

---

## Execution Choice

Plan complete and saved to `docs/superpowers/plans/2026-05-13-floating-pin.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task with two-stage review between tasks. Fast iteration, isolated context.
2. **Inline Execution** — run tasks in this session via executing-plans, batch with checkpoints.

Which approach?
