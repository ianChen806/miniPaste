import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    startDragging: vi.fn().mockResolvedValue(undefined),
    innerSize: vi.fn().mockResolvedValue({ width: 100, height: 80 }),
    scaleFactor: vi.fn().mockResolvedValue(1),
    setSize: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock("@tauri-apps/api/dpi", () => ({
  LogicalSize: class {
    constructor(public width: number, public height: number) {}
  },
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

  it("invokes pin_close on contextmenu", async () => {
    const wrapper = mount(App);
    await wrapper.find(".pin-root").trigger("contextmenu");
    expect(call).toHaveBeenCalledWith("pin_close", { label: "pin-7" });
  });
});
