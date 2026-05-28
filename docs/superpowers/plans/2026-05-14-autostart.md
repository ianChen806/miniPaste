# Auto-start on system boot — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a single toggle to the Settings window that lets the user enable or disable launching MiniPaste automatically on Windows login.

**Architecture:** Use `tauri-plugin-autostart` as the single source of truth (writes the Windows `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` registry value). The frontend toggle queries and mutates the plugin directly — no field is added to our `Config` JSON.

**Tech Stack:** `tauri-plugin-autostart` 2.x (Rust crate + `@tauri-apps/plugin-autostart` npm package), Vue 3 reactive, existing settings.css dark theme.

---

## File Structure

| File | Responsibility | Action |
|---|---|---|
| `src-tauri/Cargo.toml` | Rust deps | Modify — add `tauri-plugin-autostart` |
| `src-tauri/src/lib.rs` | Tauri builder setup | Modify — register plugin |
| `src-tauri/capabilities/default.json` | IPC permissions | Modify — allow `autostart:default` |
| `package.json` | Frontend deps | Modify (via `npm install`) |
| `src/windows/settings/settings.css` | Settings styling | Modify — add `.switch` styles |
| `src/windows/settings/App.vue` | Settings UI + logic | Modify — toggle state, lifecycle, template row |
| `docs/manual-test-checklist.md` | Test plan | Modify — append auto-start section |

---

## Task 1: Install the autostart plugin dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json` (via npm install — also updates `package-lock.json`)

- [ ] **Step 1: Add the Rust crate**

Open `src-tauri/Cargo.toml` and add this line to the `[dependencies]` section, right after the existing `tauri-plugin-dialog` line:

```toml
tauri-plugin-autostart = "2"
```

The `[dependencies]` section header is at line 17; insert at line 20 so the file reads:

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png", "devtools"] }
tauri-plugin-dialog = "2"
tauri-plugin-autostart = "2"
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 2: Install the npm package**

From the repo root, run:

```sh
npm install @tauri-apps/plugin-autostart
```

Expected: `package.json` and `package-lock.json` updated; no errors. The new entry under `dependencies` in `package.json` should be `"@tauri-apps/plugin-autostart": "^2..."`.

- [ ] **Step 3: Verify everything still builds**

From the repo root, run:

```sh
npx tauri build
```

Expected: build completes, produces `src-tauri/target/release/bundle/{msi,nsis}/...`. No new warnings about autostart (it isn't wired in yet, just downloaded). If `npx tauri build` is too slow during plan execution, an interim `cargo check` from inside `src-tauri/` is also acceptable.

- [ ] **Step 4: Commit**

```sh
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json
git commit -m "chore: add tauri-plugin-autostart dependency"
```

---

## Task 2: Register the plugin in the Tauri builder

**Files:**
- Modify: `src-tauri/src/lib.rs:38-40`
- Modify: `src-tauri/capabilities/default.json:6-12`

- [ ] **Step 1: Register the plugin in `lib.rs`**

Find the `tauri::Builder::default()` chain (currently around line 38). It looks like this:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(AppState::new(config, config_path))
```

Change it to:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ))
    .manage(AppState::new(config, config_path))
```

The first plugin arg (`MacosLauncher::LaunchAgent`) is required by the plugin's API even though it is macOS-only — Windows ignores it. The second arg (`None`) means no extra CLI flags are baked into the auto-start command; the registry entry will point at `minipaste.exe` with no arguments.

- [ ] **Step 2: Grant the plugin's IPC permissions**

Open `src-tauri/capabilities/default.json`. The current `permissions` array (line 6-12) is:

```json
"permissions": [
  "core:default",
  "core:event:default",
  "core:window:default",
  "core:webview:default",
  "dialog:default"
]
```

Change it to:

```json
"permissions": [
  "core:default",
  "core:event:default",
  "core:window:default",
  "core:webview:default",
  "dialog:default",
  "autostart:default"
]
```

- [ ] **Step 3: Verify the Rust build still passes**

```sh
npx tauri build
```

Expected: build completes with no errors. The release `minipaste.exe` now has the plugin compiled in, but no UI exposes it yet.

- [ ] **Step 4: Commit**

```sh
git add src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat(autostart): register tauri-plugin-autostart with default permissions"
```

---

## Task 3: Add the switch-style toggle CSS

**Files:**
- Modify: `src/windows/settings/settings.css` (append at end)

- [ ] **Step 1: Append switch styles to `settings.css`**

Append the following block to the end of `src/windows/settings/settings.css`:

```css
.field-inline {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.switch-slider {
  position: absolute;
  inset: 0;
  cursor: pointer;
  background: var(--border-strong);
  border-radius: 999px;
  transition: background 160ms ease;
}

.switch-slider::before {
  content: "";
  position: absolute;
  width: 14px;
  height: 14px;
  left: 3px;
  top: 3px;
  background: #fff;
  border-radius: 50%;
  transition: transform 160ms ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
}

.switch input:checked + .switch-slider {
  background: var(--primary);
}

.switch input:checked + .switch-slider::before {
  transform: translateX(16px);
}

.switch input:focus-visible + .switch-slider {
  box-shadow: 0 0 0 3px var(--ring);
}

.switch input:disabled + .switch-slider {
  opacity: 0.5;
  cursor: not-allowed;
}
```

The `.field-inline` modifier is used in Task 4 to lay out the label and switch in a single row rather than the default stacked column.

- [ ] **Step 2: Commit**

```sh
git add src/windows/settings/settings.css
git commit -m "feat(settings): add switch-style toggle CSS"
```

---

## Task 4: Wire the toggle into the Settings UI

**Files:**
- Modify: `src/windows/settings/App.vue`

- [ ] **Step 1: Add the plugin imports**

Open `src/windows/settings/App.vue`. Find the existing imports block at the top of the `<script setup>` (currently lines 1-7):

```ts
import { onMounted, reactive } from "vue";
import { call, on } from "../../shared/ipc";
import type { Config } from "../../shared/types";
import HotkeyRecorder from "./HotkeyRecorder.vue";
import Toast from "../../shared/Toast.vue";
import { pushToast } from "../../shared/toast";
```

Add an import for the autostart plugin functions right after the last import:

```ts
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
```

- [ ] **Step 2: Add reactive state for the toggle**

Find the existing `reactive({...})` block (currently lines 9-13). Add two fields:

```ts
const state = reactive({
  loaded: false,
  config: null as Config | null,
  error: "" as string,
  launchAtStartup: false,
  autostartBusy: false,
});
```

- [ ] **Step 3: Query autostart state on mount**

Find the `onMounted` block. Inside the `try` block, after `state.loaded = true;`, also fetch the autostart state. The updated `onMounted` should look like:

```ts
onMounted(async () => {
  try {
    state.config = await call<Config>("get_config");
    state.loaded = true;
    state.launchAtStartup = await isEnabled();
  } catch (e: unknown) {
    state.error = errorMessage(e);
  }
  on<{ kind?: string; attempted: string; reason: string }>(
    "hotkey-conflict",
    (p) => {
      const which = p.kind === "paste_pin" ? "Paste pin" : "Capture";
      state.error = `${which} hotkey "${p.attempted}" 衝突：${p.reason}`;
    },
  );
});
```

If `isEnabled()` throws, the existing `catch` will surface the error in the error banner and `state.launchAtStartup` will remain `false` — acceptable degraded behavior.

- [ ] **Step 4: Add the toggle handler**

Just below `apply()` (around line 53), add the `toggleAutostart` function:

```ts
async function toggleAutostart(next: boolean) {
  if (state.autostartBusy) return;
  state.autostartBusy = true;
  const previous = state.launchAtStartup;
  state.launchAtStartup = next;
  try {
    if (next) {
      await enable();
    } else {
      await disable();
    }
    pushToast("success", next ? "Auto-start enabled" : "Auto-start disabled");
  } catch (e: unknown) {
    state.launchAtStartup = previous;
    pushToast("error", errorMessage(e));
  } finally {
    state.autostartBusy = false;
  }
}
```

- [ ] **Step 5: Add the template row**

In the template, find the `Paste pin hotkey` field block:

```vue
<div class="field">
  <span class="field-label">Paste pin hotkey</span>
  <HotkeyRecorder v-model="state.config.paste_pin_hotkey" />
</div>
```

Immediately after that closing `</div>`, insert a new field for the autostart toggle:

```vue
<div class="field field-inline">
  <span class="field-label">Launch at startup</span>
  <label class="switch">
    <input
      type="checkbox"
      :checked="state.launchAtStartup"
      :disabled="state.autostartBusy"
      @change="toggleAutostart(($event.target as HTMLInputElement).checked)"
    />
    <span class="switch-slider"></span>
  </label>
</div>
```

- [ ] **Step 6: Build to verify TypeScript + bundling**

```sh
npm run build
```

Expected: `vue-tsc --noEmit && vite build` completes with no errors. If TypeScript complains about the `enable`/`disable`/`isEnabled` import not being found, double-check that Task 1 Step 2 successfully installed `@tauri-apps/plugin-autostart`.

- [ ] **Step 7: Commit**

```sh
git add src/windows/settings/App.vue
git commit -m "feat(settings): add Launch-at-startup toggle"
```

---

## Task 5: End-to-end smoke test and update manual checklist

**Files:**
- Modify: `docs/manual-test-checklist.md` (append a new section)

- [ ] **Step 1: Build the release binary**

```sh
npx tauri build
```

Expected: build succeeds, `src-tauri/target/release/minipaste.exe` exists.

- [ ] **Step 2: Kill any running instance and launch the fresh build**

```powershell
Get-Process minipaste -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process "D:\SideProject\MiniPaste\src-tauri\target\release\minipaste.exe"
```

- [ ] **Step 3: Open the Settings window via tray and verify the new row appears**

Right-click the tray icon → Settings. Verify:
- A row labeled **Launch at startup** appears between **Paste pin hotkey** and **Default folder**.
- The label sits on the left, the switch sits on the right (`.field-inline` row layout).
- The switch position reflects the current registry state — for a fresh user this is OFF.

- [ ] **Step 4: Toggle ON and verify the registry entry was written**

Click the switch to turn it ON. The slider should animate to the right, fill with the primary blue color, and a green "Auto-start enabled" toast should appear.

Open the Registry Editor (`regedit`) and navigate to:

```
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
```

Expected: a new value named `minipaste` (the plugin uses the binary name) with data pointing at the path of `minipaste.exe`. If the value is missing, the plugin failed silently — check the toast for an error message.

- [ ] **Step 5: Toggle OFF and verify the registry entry was removed**

Click the switch back to OFF. Toast: "Auto-start disabled". Re-check `regedit` — the `minipaste` value under `Run` should now be gone.

- [ ] **Step 6: Append manual-test items to the checklist**

Open `docs/manual-test-checklist.md` and append a new section at the end:

```markdown

## Auto-start
- [ ] Settings shows "Launch at startup" row between Paste pin hotkey and Default folder
- [ ] Initial toggle state matches actual registry state (`HKCU\...\Run\minipaste`)
- [ ] Toggle ON writes the registry entry and shows success toast
- [ ] Toggle OFF removes the registry entry and shows success toast
- [ ] After toggling ON and restarting Windows, the tray icon appears within seconds of login
- [ ] After toggling OFF and restarting Windows, the app does NOT auto-launch
- [ ] If `Run\minipaste` is removed externally (regedit) while the app is closed, reopening Settings shows the toggle as OFF
```

- [ ] **Step 7: Commit**

```sh
git add docs/manual-test-checklist.md
git commit -m "docs: manual test items for auto-start"
```

- [ ] **Step 8 (optional): Real reboot test**

A full Windows restart is the only way to verify that the auto-start actually works end-to-end. If the user is willing, restart Windows after toggling ON and confirm the tray icon appears at login. This can be deferred until release smoke-testing.
