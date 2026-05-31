const DRAG_THRESHOLD_PX = 4;

export function canStartSurfaceDrag(target: EventTarget | null) {
  return target instanceof Element
    && !target.closest("input,textarea,select,[contenteditable='true']");
}

export function movedPastDragThreshold(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
) {
  return Math.hypot(currentX - startX, currentY - startY) >= DRAG_THRESHOLD_PX;
}
