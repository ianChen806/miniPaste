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

## Auto-start
- [ ] Settings shows "Launch at startup" row between Paste pin hotkey and Default folder
- [ ] Initial toggle state matches actual registry state (`HKCU\...\Run\minipaste`)
- [ ] Toggle ON writes the registry entry and shows success toast
- [ ] Toggle OFF removes the registry entry and shows success toast
- [ ] After toggling ON and restarting Windows, the tray icon appears within seconds of login
- [ ] After toggling OFF and restarting Windows, the app does NOT auto-launch
- [ ] If `Run\minipaste` is removed externally (regedit) while the app is closed, reopening Settings shows the toggle as OFF

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

## Paste Pin

- [ ] Default hotkey `Ctrl+Shift+V` pins clipboard image (copy a screenshot first)
- [ ] Hotkey pins clipboard text → renders as image (try ASCII + 中文 mixed)
- [ ] Hotkey pins clipboard file path when file is PNG/JPG/GIF/BMP (copy a file in Explorer)
- [ ] Empty clipboard → toast "剪貼簿是空的", no pin
- [ ] Non-image file path → toast "不是圖片：…", no pin
- [ ] Pin spawns at cursor position
- [ ] Pin is always-on-top (verify against fullscreen window)
- [ ] Drag pin by body (cursor: grab)
- [ ] Scroll wheel zooms pin (up = larger, down = smaller, aspect ratio preserved)
- [ ] Right-click closes pin
- [ ] Esc closes focused pin
- [ ] Multiple pins coexist; closing one leaves the others alone
- [ ] Spawn 5+ pins → all responsive, RAM < 300 MB total
- [ ] Settings: change paste-pin hotkey to `Ctrl+Alt+V` → new hotkey works, old does not
- [ ] Restart app → paste-pin hotkey config persists

## Inline Capture (Snipaste-style)

### Framing
- [ ] Capture hotkey → overlay shows, drag-to-frame works
- [ ] Magnifier follows cursor, offsets away from it, shows correct coords
- [ ] Drag < 5px → no transition, can re-drag
- [ ] Esc → cancels back to idle

### Editing
- [ ] mouseup shows 8 handles + toolbar below selection
- [ ] Toolbar flips above when below is tight
- [ ] Toolbar falls back to inside when overlay is shorter than expected
- [ ] Resizing via handle: annotations stay at their pixel positions, clip outside new selection
- [ ] Magnifier appears during handle drag
- [ ] Left-click outside selection → shapes cleared, returns to framing
- [ ] Double-click inside selection → default action (Copy) + exit
- [ ] Enter → default action (Copy) + exit
- [ ] Esc / right-click → cancels everything

### Finish actions
- [ ] Copy → clipboard has image, overlay exits
- [ ] Save → dialog, path picked, file written, toast
- [ ] Save+Copy → default path used, clipboard FileList set, toast
- [ ] Any action failure → toast shown, stays in editing

### Multi-monitor
- [ ] Primary + secondary monitor both captureable
- [ ] Cross-monitor selection, magnifier, toolbar placement all correct
