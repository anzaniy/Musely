import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { wireCompactMode } from "./compact";
import { startDemo } from "./demo";
import { startRenderLoop, tick } from "./lyrics";
import { fmtOffset, pushOffset, setOffset } from "./offset";
import { onLyrics, onPlayback, quitApp, sendControl } from "./playback";
import { OFFSET_STEP, state } from "./state";
import type { LyricsState, PlaybackSnapshot } from "./types";
import { el, setBadge } from "./ui";

function wireControls() {
  el.btnPrev?.addEventListener("click", () => sendControl("previous"));
  el.btnPlay?.addEventListener("click", () => sendControl("toggle"));
  el.btnNext?.addEventListener("click", () => sendControl("next"));
  el.btnClose?.addEventListener("click", quitApp);

  el.btnOffsetMinus?.addEventListener("click", () => pushOffset(-OFFSET_STEP));
  el.btnOffsetPlus?.addEventListener("click", () => pushOffset(OFFSET_STEP));
  el.offsetValue?.addEventListener("click", () => setOffset(0));

  // Geser lirik dari menu tray.
  void listen<number>("offset:adjust", (event) => {
    pushOffset(Number(event.payload) || 0);
  });
  void listen<number>("offset:set", (event) => {
    setOffset(Number(event.payload) || 0);
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  // Tampilkan offset tersimpan sejak awal.
  if (el.offsetValue) el.offsetValue.textContent = fmtOffset(state.offsetMs);

  wireControls();
  wireCompactMode();

  try {
    const info = await invoke<{ stage: string }>("app_info");
    state.infoStage = info.stage;
    setBadge(info.stage);
  } catch {
    setBadge("web");
  }

  try {
    await Promise.all([
      listen<PlaybackSnapshot>("playback:update", (event) => {
        onPlayback(event.payload);
        tick();
      }),
      listen<LyricsState>("lyrics:update", (event) => {
        onLyrics(event.payload);
        tick();
      }),
    ]);
  } catch {
    // Dibuka di browser biasa, bukan Tauri → pakai demo
    startDemo();
  }

  startRenderLoop();
});