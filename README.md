# MiniPaste

A lightweight Snipaste-style screenshot tool for Windows. Capture, annotate, and copy or save — all in place, no separate editor window.

## Features

- **Inline capture**: select a region and the editing toolbar appears alongside the selection. No window swap, no context loss.
- **Annotation tools**: rectangle, ellipse, line, arrow, mosaic, text. Configurable color and thickness.
- **History**: undo / redo (Ctrl+Z / Ctrl+Y).
- **Finish actions**: copy image to clipboard, save as PNG, or save & copy the file path.
- **Configurable hotkeys**: capture and paste-pin keys editable from the Settings window.
- **Tray icon**: quick access to capture and settings, runs in the background.

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vite
- **Canvas**: Konva.js
- **Shell**: Tauri 2 (Rust)
- **Platform**: Windows 10/11

## Development

```sh
npm install
npm run tauri dev
```

The dev URL is `http://localhost:1420`.

## Build

```sh
npx tauri build
```

Always use `npx tauri build` rather than `cargo build --release` — the latter silently falls back to the dev URL.

Outputs:
- `src-tauri/target/release/minipaste.exe`
- `src-tauri/target/release/bundle/msi/`
- `src-tauri/target/release/bundle/nsis/`

## Tests

```sh
npm test
```

Unit tests cover handle hit-testing, toolbar placement, magnifier rendering, and undo/redo history.

## License

[MIT](LICENSE)
