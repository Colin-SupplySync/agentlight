// @vitest-environment jsdom

import { describe, expect, it } from "vitest";
import { canStartSurfaceDrag, movedPastDragThreshold } from "./drag";

describe("drag helpers", () => {
  it("allows visible overlay modules such as buttons to start a drag", () => {
    const button = document.createElement("button");

    expect(canStartSurfaceDrag(button)).toBe(true);
  });

  it("keeps text-editing controls interactive instead of dragging", () => {
    const input = document.createElement("input");

    expect(canStartSurfaceDrag(input)).toBe(false);
  });

  it("starts dragging only after pointer movement passes the threshold", () => {
    expect(movedPastDragThreshold(10, 10, 12, 12)).toBe(false);
    expect(movedPastDragThreshold(10, 10, 16, 10)).toBe(true);
  });
});
