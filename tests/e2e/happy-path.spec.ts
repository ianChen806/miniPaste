import { test, expect } from "@playwright/test";

const DEFAULT_CONFIG = {
  schema_version: 1,
  hotkey: "Ctrl+Shift+S",
  default_save_path: "C:/Users/test/Pictures/minipaste",
  image_format: "png",
  jpeg_quality: 90,
};

async function mockTauri(page: import("@playwright/test").Page) {
  await page.addInitScript((cfg) => {
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      invoke: (cmd: string) => {
        if (cmd === "get_config") return Promise.resolve(cfg);
        return Promise.resolve(null);
      },
      transformCallback: (cb: unknown) => cb,
    };
  }, DEFAULT_CONFIG);
}

test("settings panel loads with default config", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/settings.html");
  await expect(page.locator(".settings h2")).toHaveText("Settings");
  await expect(page.locator(".hotkey-input")).toBeVisible();
});

test("hotkey recorder captures combo", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/settings.html");
  await page.locator(".hotkey-input").focus();
  await page.keyboard.press("Control+Shift+S");
  await expect(page.locator(".hotkey-input")).toHaveValue("Ctrl+Shift+S");
});

// Full hotkey → capture → editor → action flow needs Tauri WebDriver beyond
// scope of this initial E2E — covered by manual test checklist.
