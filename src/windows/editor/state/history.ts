import type { Shape } from "../../../shared/types";

export interface History {
  push(snapshot: Shape[]): void;
  undo(): void;
  redo(): void;
  current(): Shape[];
  size(): number;
}

export function createHistory(limit = 50): History {
  let stack: Shape[][] = [];
  let pointer = -1;
  return {
    push(snap) {
      stack = stack.slice(0, pointer + 1);
      stack.push(snap.map((s) => ({ ...s })));
      if (stack.length > limit) stack.shift();
      pointer = stack.length - 1;
    },
    undo() {
      if (pointer > 0) pointer--;
    },
    redo() {
      if (pointer < stack.length - 1) pointer++;
    },
    current() {
      return pointer >= 0 ? stack[pointer] : [];
    },
    size() {
      return stack.length;
    },
  };
}
