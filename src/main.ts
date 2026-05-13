import { createApp, type Component } from "vue";

const entry = document.documentElement.dataset.window;

async function bootstrap() {
  let App: Component;
  switch (entry) {
    case "settings":
      App = (await import("./windows/settings/App.vue")).default;
      break;
    case "overlay":
      App = (await import("./windows/overlay/App.vue")).default;
      break;
    case "pin":
      App = (await import("./windows/pin/App.vue")).default;
      break;
    default:
      throw new Error(`unknown window entry: ${entry}`);
  }
  createApp(App).mount("#app");
}

bootstrap();
