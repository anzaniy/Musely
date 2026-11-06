import { clamp, OFFSET_LIMIT, saveOffset, state } from "./state";
import { el } from "./ui";

export function fmtOffset(value: number): string {
  if (value === 0) return "0 ms";
  const sign = value > 0 ? "+" : "\u2212";
  return `${sign}${Math.abs(value)} ms`;
}

export function pushOffset(delta: number) {
  setOffset(state.offsetMs + delta);
}

export function setOffset(value: number) {
  state.offsetMs = clamp(Math.round(value), -OFFSET_LIMIT, OFFSET_LIMIT);
  saveOffset(state.offsetMs);

  if (el.offsetValue) {
    el.offsetValue.textContent = fmtOffset(state.offsetMs);
    // restart animasi
    el.offsetValue.classList.remove("bump");
    void el.offsetValue.offsetWidth;
    el.offsetValue.classList.add("bump");
  }
  state.lastIndex = -2;
}