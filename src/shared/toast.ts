import { reactive } from "vue";

export interface Toast {
  id: number;
  level: "info" | "error" | "success";
  msg: string;
}

let nextId = 0;
export const toastState = reactive({ list: [] as Toast[] });

export function pushToast(level: Toast["level"], msg: string, ttlMs = 3000) {
  const t: Toast = { id: ++nextId, level, msg };
  toastState.list.push(t);
  setTimeout(() => {
    toastState.list = toastState.list.filter((x) => x.id !== t.id);
  }, ttlMs);
}
