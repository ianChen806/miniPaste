# Auto-start on system boot — Design

**Date:** 2026-05-14
**Status:** Approved
**Scope:** Add a single toggle to the Settings window that enables/disables launching MiniPaste automatically when Windows starts.

---

## Goal

Let the user opt MiniPaste into Windows' user-level auto-start so it appears in the tray immediately after login, without manual launch.

## Non-goals

- Machine-wide auto-start (admin-only `HKLM` registry hive) — not needed for a personal productivity tool.
- Delayed-start / login-task scheduling beyond what Windows' default "Run" key offers.
- Exposing the toggle in the tray menu (Settings-only per user choice).
- Persisting the state in our own `Config` JSON. The Windows registry entry that the plugin writes is the single source of truth.

## Architecture

```
Settings UI ──toggle──▶ @tauri-apps/plugin-autostart (frontend)
                                  │
                                  ▼
                       tauri-plugin-autostart (Rust)
                                  │
                                  ▼
        HKCU\Software\Microsoft\Windows\CurrentVersion\Run
              └─ Value name: "minipaste"  (set by plugin)
              └─ Value data: path to minipaste.exe
```

The plugin owns persistence. The app never reads or writes the registry directly; we ask the plugin "is autostart enabled?" each time the Settings window opens.

## Components

### Rust backend (`src-tauri/`)

- **`Cargo.toml`**: add `tauri-plugin-autostart = "2"` to `[dependencies]`.
- **`src/lib.rs`**: in the `tauri::Builder::default()` chain, add:
  ```rust
  .plugin(tauri_plugin_autostart::init(
      tauri_plugin_autostart::MacosLauncher::LaunchAgent,
      None,
  ))
  ```
  The first arg is required by the plugin's API for macOS — irrelevant here but must be present. The second arg (`None`) means no extra CLI args are baked into the auto-start command; the registry will simply point at `minipaste.exe`.

No new IPC commands, no `AppState` changes, no `Config` schema bump. The plugin registers its own commands (`plugin:autostart|enable`, `plugin:autostart|disable`, `plugin:autostart|is_enabled`) that the frontend uses directly.

### Frontend dependency

- **`package.json`**: add `@tauri-apps/plugin-autostart` to `dependencies`.

### Settings UI (`src/windows/settings/App.vue`)

Add a new field "Launch at startup" with a switch-style toggle.

**Position:** between the **Paste pin hotkey** row and the **Default folder** row, because it is a top-level on/off behavior, not a value-selection field.

**State additions to `reactive({...})`:**
- `launchAtStartup: false` — current toggle position
- `autostartBusy: false` — true while an `enable`/`disable` call is in flight (to prevent races)

**Lifecycle:**
- `onMounted`: after `get_config` resolves, also `await isEnabled()` and set `state.launchAtStartup` to the result.
- On toggle change: optimistically reflect the new value, set `autostartBusy = true`, call `enable()` or `disable()`. On success: leave it. On error: revert the toggle and push an error toast.

**Styling:**
A switch-style toggle (CSS-only) sized to match the existing dark theme. Width ≈ 36px, height ≈ 20px, primary-color fill when ON.

## Data flow

```
First app launch ever
  ├─ Config loads (no autostart field)
  └─ Plugin sees no registry entry → isEnabled() returns false

User opens Settings
  └─ UI calls isEnabled() → toggle reflects actual state

User flips toggle ON
  ├─ UI: optimistic update, disable input
  ├─ Plugin writes HKCU\...\Run\minipaste = "<path>\minipaste.exe"
  └─ UI: re-enable input

User flips toggle OFF
  ├─ UI: optimistic update, disable input
  ├─ Plugin removes HKCU\...\Run\minipaste
  └─ UI: re-enable input

Windows reboots
  └─ Windows reads HKCU\...\Run → spawns minipaste.exe → app starts identically to manual launch (silent, tray only)
```

## Error handling

| Failure | Behavior |
|---|---|
| Plugin throws (registry locked, antivirus blocked, permission denied) | Revert toggle UI, push `error` toast with the plugin's message |
| `isEnabled()` throws on mount | Default toggle to `false`, push `error` toast — the user can still try to toggle and see a clearer error |
| Race: user toggles twice rapidly | The `autostartBusy` flag disables the input until the in-flight call resolves |

## Testing

- **Unit:** none added. The plugin is third-party and ships its own tests. Our wiring is too thin to meaningfully unit-test (a toggle that calls one of two methods).
- **Manual checklist** added to `docs/manual-test-checklist.md`:
  1. Open Settings, toggle "Launch at startup" ON, restart Windows, verify the tray icon appears within a few seconds of login.
  2. Open Settings, confirm the toggle is still ON.
  3. Toggle OFF, restart Windows, verify the app does NOT auto-launch.
  4. Toggle ON, then manually delete the registry value via `regedit` (simulating external removal). Reopen Settings — toggle should now read OFF (plugin re-queries registry).
  5. Disconnect from the network and toggle — should still work (registry is local).

## Open questions

None.

## Rejected alternatives

- **Manual `winreg` crate**: more code, no benefit. The plugin abstracts the same operation in three lines.
- **`Startup` folder shortcut (.lnk)**: requires COM (`IShellLink`) for shortcut creation; significantly more complex with no UX benefit. The registry approach is what most Windows apps use.
- **Persist in our `Config`**: introduces a state-sync problem (registry vs JSON file). Avoided by treating the registry as the single source of truth.
