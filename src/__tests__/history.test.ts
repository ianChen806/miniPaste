import { describe, expect, it } from "vitest";
import { createHistory } from "../shared/editor/state/history";
import type { Shape } from "../shared/types";

const s = (id: string): Shape => ({
  id,
  tool: "rect",
  color: "red",
  thickness: "medium",
  geometry: { kind: "rect", x: 0, y: 0, w: 1, h: 1 },
});

describe("history", () => {
  it("push then undo restores previous snapshot", () => {
    const h = createHistory();
    h.push([]);
    h.push([s("a")]);
    h.push([s("a"), s("b")]);
    expect(h.current()).toEqual([s("a"), s("b")]);
    h.undo();
    expect(h.current()).toEqual([s("a")]);
    h.undo();
    expect(h.current()).toEqual([]);
  });

  it("redo replays forward", () => {
    const h = createHistory();
    h.push([]);
    h.push([s("a")]);
    h.undo();
    h.redo();
    expect(h.current()).toEqual([s("a")]);
  });

  it("new push after undo drops the redo tail", () => {
    const h = createHistory();
    h.push([]);
    h.push([s("a")]);
    h.undo();
    h.push([s("b")]);
    expect(() => h.redo()).not.toThrow();
    expect(h.current()).toEqual([s("b")]);
  });

  it("limits to 50 snapshots", () => {
    const h = createHistory(50);
    for (let i = 0; i < 60; i++) h.push([s(`${i}`)]);
    expect(h.size()).toBe(50);
  });
});
