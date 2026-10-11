// A positive scrollTop change, or a positive wheel deltaY, means the gesture
// moved the content downward. That is an inverted wheel: HID "up" became "down".
export function invertedFromProbe(scrollDelta: number, wheelDeltaY: number): boolean | null {
  const delta = Math.abs(scrollDelta) >= 1 ? scrollDelta : wheelDeltaY;
  if (Math.abs(delta) < 1) {
    return null;
  }
  return delta > 0;
}
