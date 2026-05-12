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
