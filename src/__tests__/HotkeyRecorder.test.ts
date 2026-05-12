import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import HotkeyRecorder from "../windows/settings/HotkeyRecorder.vue";

describe("HotkeyRecorder", () => {
  it("captures Ctrl+Shift+S and emits update", async () => {
    const w = mount(HotkeyRecorder, { props: { modelValue: "" } });
    const input = w.find(".hotkey-input");
    await input.trigger("focus");
    await input.trigger("keydown", {
      key: "S",
      code: "KeyS",
      ctrlKey: true,
      shiftKey: true,
    });
    const emits = w.emitted("update:modelValue");
    expect(emits?.[0]?.[0]).toBe("Ctrl+Shift+S");
  });
});
