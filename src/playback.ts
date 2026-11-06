import { invoke } from "@tauri-apps/api/core";
import { COMPACT_AFTER_CONTENT_MS, expandWidget, isHovered, scheduleCompact } from "./compact";
import { currentPositionMs, IDLE_GRACE_MS, state } from "./state";
import type { LyricsState, PlaybackSnapshot, TrackInfo } from "./types";
import { el, setBadge, setHeader, setNotice, setPlayIcon, showMessage } from "./ui";

function trackKeyOf(t: TrackInfo): string {
  return `${t.title}\u0000${t.artist}\u0000${t.durationMs}`;
}

/** Sama dengan `lyrics::is_ad` di Rust. */
function isAdTrack(t: TrackInfo): boolean {
  const artist = t.artist.trim().toLowerCase();
  const album = t.album.trim().toLowerCase();
  const title = t.title.trim().toLowerCase();

  return (
    artist === "spotify" ||
    album === "spotify" ||
    artist === "advertisement" ||
    title === "advertisement" ||
    title.includes("dengarkan musik tanpa iklan") ||
    title.includes("listen to music without ads")
  );
}

function showAd() {
  state.lines = [];
  state.lastIndex = -2;
  expandWidget();
  setBadge("AD");
  setHeader("Iklan Spotify", "Lirik dijeda sementara");
  showMessage("Iklan \u2014 lirik dijeda");
}

function showIdle() {
  state.track = null;
  state.trackKey = null;
  state.lines = [];
  state.lastIndex = -2;
  state.playing = false;
  expandWidget();
  setBadge(state.infoStage);
  setPlayIcon(false);
  setHeader("Tidak ada lagu", "Putar lagu di Spotify");
  showMessage("Menunggu playback\u2026");
  if (el.progress) el.progress.style.width = "0%";
}

export function onPlayback(snapshot: PlaybackSnapshot) {
  const now = performance.now();
  const nextPlaying = snapshot.status === "playing";
  const next = snapshot.track;
  const nextKey = next ? trackKeyOf(next) : null;
  const trackChanged = nextKey !== null && nextKey !== state.trackKey;
  const position = snapshot.positionMs;
  const previousPosition = state.lastSnapshotPosition;
  state.lastSnapshotPosition = position;

  if (trackChanged || nextPlaying !== state.playing) {
    state.anchorPosition = position;
    state.anchorTime = now;
    state.playing = nextPlaying;
  } else if (!nextPlaying) {
    state.anchorPosition = position;
    state.anchorTime = now;
  } else if (position !== previousPosition) {
    const estimate = state.anchorPosition + (now - state.anchorTime);
    const drift = position - estimate;

    if (position < previousPosition - 1_200) {
      state.anchorPosition = position;
      state.anchorTime = now;
    } else if (drift > 900) {
      state.anchorPosition = position;
      state.anchorTime = now;
    } else if (drift < -900) {
      state.anchorPosition = estimate + drift * 0.15;
      state.anchorTime = now;
    } else {
      state.anchorPosition = estimate + drift * 0.25;
      state.anchorTime = now;
    }
  }

  setPlayIcon(state.playing);

  if (!snapshot.track) {
    // Abaikan idle sesaat (mis. Spotify sedang berganti lagu).
    if (state.track && performance.now() - state.lastActiveAt < IDLE_GRACE_MS) return;
    showIdle();
    return;
  }

  state.lastActiveAt = performance.now();

  if (!next) {
    showIdle();
    return;
  }

  if (nextKey === state.trackKey) {
    state.track = next;
    return;
  }

  state.track = next;
  state.trackKey = nextKey;
  state.lines = [];
  state.lastIndex = -2;
  expandWidget();

  if (isAdTrack(next)) {
    showAd();
    return;
  }

  setHeader(next.title, [next.artist, next.album].filter(Boolean).join(" \u2014 "));
  showMessage("Memuat lirik\u2026");
}

export function onLyrics(lyrics: LyricsState) {
  setBadge(state.infoStage);

  switch (lyrics.kind) {
    case "ready": {
      const nextLines = lyrics.doc.lines ?? [];
      state.lines = nextLines;
      state.lastIndex = -2;
      // Lirik tanpa timestamp: beri tanda agar pengguna tahu waktunya perkiraan.
      if (lyrics.doc.unsynced) setBadge("PLAIN");
      if (nextLines.length === 0) {
        expandWidget();
        showMessage("Lirik kosong");
      } else if (!isHovered()) {
        scheduleCompact(COMPACT_AFTER_CONTENT_MS);
      }
      break;
    }

    case "instrumental":
      state.lines = [];
      state.lastIndex = -2;
      expandWidget();
      showMessage("Instrumental \u2014 tanpa lirik");
      break;

    case "ad":
      showAd();
      break;

    case "not_found":
      state.lines = [];
      state.lastIndex = -2;
      expandWidget();
      showMessage("Lirik tidak tersedia");
      break;

    case "error":
      state.lines = [];
      state.lastIndex = -2;
      expandWidget();
      showMessage(lyrics.message || "Gagal memuat lirik");
      break;

    case "loading":
      if (state.lines.length === 0) showMessage("Memuat lirik\u2026");
      break;

    case "idle":
      // Jangan langsung menghapus lirik: beri waktu sebelum idle sungguhan.
      if (state.track && performance.now() - state.lastActiveAt < IDLE_GRACE_MS) return;
      showIdle();
      break;
  }
}

export async function sendControl(action: "toggle" | "next" | "previous") {
  if (state.demoMode) return;

  const wasPlaying = state.playing;
  const beforePos = currentPositionMs();

  // Umpan balik instan untuk tombol play/pause.
  if (action === "toggle") {
    state.playing = !wasPlaying;
    state.anchorPosition = beforePos;
    state.anchorTime = performance.now();
    setPlayIcon(state.playing);
  }

  try {
    await invoke("playback_control", { action });
  } catch (error) {
    // Perintah gagal (mis. tidak ada sesi media): kembalikan ikon.
    if (action === "toggle") {
      state.playing = wasPlaying;
      state.anchorPosition = beforePos;
      state.anchorTime = performance.now();
      setPlayIcon(state.playing);
    }
    setNotice("GAGAL", state.infoStage);
    console.error("playback_control", error);
  }
}

export async function quitApp() {
  try {
    await invoke("quit_app");
  } catch (error) {
    console.error("quit_app", error);
    setNotice("GAGAL", state.infoStage);
  }
}