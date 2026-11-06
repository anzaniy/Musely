import { clamp01, currentPositionMs, state } from "./state";
import { el } from "./ui";

export function findLineIndex(posMs: number): number {
  let lo = 0;
  let hi = state.lines.length - 1;
  let ans = -1;

  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (state.lines[mid].startMs <= posMs) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return ans;
}

export function renderLines(index: number) {
  if (!el.active) return;

  el.active.classList.remove("msg");

  if (index < 0) {
    if (el.prev) el.prev.textContent = "\u00a0";
    el.active.textContent = "\u266a";
    if (el.next) el.next.textContent = state.lines[0]?.text ?? "\u00a0";
    return;
  }

  const current = state.lines[index];
  if (!current) return;

  if (el.prev) el.prev.textContent = index > 0 ? state.lines[index - 1].text : "\u00a0";
  el.active.textContent = current.text;
  if (el.next) el.next.textContent = state.lines[index + 1]?.text ?? "\u00a0";

  el.active.animate(
    [
      { opacity: 0, transform: "translateY(7px)" },
      { opacity: 1, transform: "translateY(0)" },
    ],
    { duration: 220, easing: "ease-out" },
  );
}

export function tick() {
  if (state.demoMode) {
    const pos = performance.now() - state.demoStart;
    if (pos > 24_000) {
      state.demoStart = performance.now();
      state.lastIndex = -2;
    }
    state.lines = state.demoLines;
  }

  const pos = state.demoMode ? performance.now() - state.demoStart : currentPositionMs();
  const duration = state.demoMode ? 24_000 : (state.track?.durationMs ?? 0);

  if (el.progress) {
    const ratio = duration > 0 ? clamp01(pos / duration) : 0;
    el.progress.style.width = `${ratio * 100}%`;
  }

  if (state.lines.length > 0) {
    const index = findLineIndex(pos);
    if (index !== state.lastIndex) {
      // Mundur satu baris karena jitter posisi GSMTC: tunggu stabil dulu
      // sebelum benar-benar mengganti baris (anti-kedip di batas waktu).
      if (index < state.lastIndex) {
        if (index !== state.pendingBackIndex) {
          state.pendingBackIndex = index;
          state.pendingBackAt = performance.now();
        } else if (performance.now() - state.pendingBackAt >= 140) {
          state.pendingBackIndex = -2;
          state.lastIndex = index;
          renderLines(index);
        }
      } else {
        state.pendingBackIndex = -2;
        state.lastIndex = index;
        renderLines(index);
      }
    }
  }
}

function loop() {
  tick();
  requestAnimationFrame(loop);
}

export function startRenderLoop() {
  requestAnimationFrame(loop);
  window.setInterval(tick, 200);
}