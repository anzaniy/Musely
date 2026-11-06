import type { LyricsLine, TrackInfo } from "./types";

const OFFSET_KEY = "slw.offsetMs";
export const OFFSET_STEP = 100;
export const OFFSET_LIMIT = 3_000;
/** Jeda toleransi sebelum widget dianggap idle (anti-kedip). */
export const IDLE_GRACE_MS = 2_500;

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function loadOffset(): number {
  const raw = Number(localStorage.getItem(OFFSET_KEY));
  return Number.isFinite(raw) ? clamp(raw, -OFFSET_LIMIT, OFFSET_LIMIT) : 0;
}

export function saveOffset(value: number) {
  localStorage.setItem(OFFSET_KEY, String(value));
}

/**
 * Satu objek state supaya modul lain bisa membaca dan menulis tanpa
 * `export let` (binding impor ESM tidak bisa di-assign dari modul lain).
 */
export const state = {
  track: null as TrackInfo | null,
  trackKey: null as string | null,
  lines: [] as LyricsLine[],

  anchorPosition: 0,
  anchorTime: performance.now(),
  playing: false,
  lastActiveAt: 0,
  lastSnapshotPosition: -1,

  offsetMs: loadOffset(),
  lastIndex: -2,
  pendingBackIndex: -2,
  pendingBackAt: 0,

  demoMode: false,
  demoLines: [] as LyricsLine[],
  demoStart: performance.now(),
  infoStage: "demo",
};

export function currentPositionMs(): number {
  const elapsed = state.playing ? performance.now() - state.anchorTime : 0;
  const duration = state.track?.durationMs ?? 0;
  const raw = state.anchorPosition + elapsed + state.offsetMs;
  return clamp(raw, 0, duration > 0 ? duration : Number.MAX_SAFE_INTEGER);
}