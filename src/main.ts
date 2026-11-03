import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/* ------------------------------------------------------------------
 * Tipe data — harus sama dengan struct Rust di src-tauri/src/
 * ------------------------------------------------------------------ */

type PlaybackStatus = "playing" | "paused" | "stopped";

interface TrackInfo {
  id: string | null;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  coverUrl: string | null;
  source: string;
}

interface PlaybackSnapshot {
  track: TrackInfo | null;
  status: PlaybackStatus;
  positionMs: number;
  capturedAt: number;
}

interface LyricsLine {
  text: string;
  startMs: number;
  endMs: number;
}

interface LyricsDoc {
  trackKey: string;
  instrumental: boolean;
  hasWordSync: boolean;
  unsynced: boolean;
  lines: LyricsLine[];
  provider: string;
  providerId: number | null;
  fetchedAt: number;
}

type LyricsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; doc: LyricsDoc }
  | { kind: "instrumental" }
  | { kind: "ad" }
  | { kind: "not_found" }
  | { kind: "error"; message: string };

/* ------------------------------------------------------------------ */

const el = {
  title: document.getElementById("title"),
  subtitle: document.getElementById("subtitle"),
  badge: document.getElementById("badge"),
  prev: document.getElementById("line-prev"),
  active: document.getElementById("line-active"),
  next: document.getElementById("line-next"),
  progress: document.getElementById("progress"),
  btnPrev: document.getElementById("btn-prev") as HTMLButtonElement | null,
  btnPlay: document.getElementById("btn-play") as HTMLButtonElement | null,
  btnNext: document.getElementById("btn-next") as HTMLButtonElement | null,
  btnClose: document.getElementById("btn-close") as HTMLButtonElement | null,
  btnOffsetMinus: document.getElementById("btn-offset-minus") as HTMLButtonElement | null,
  btnOffsetPlus: document.getElementById("btn-offset-plus") as HTMLButtonElement | null,
  offsetValue: document.getElementById("offset-value") as HTMLButtonElement | null,
};

/* ---------------- state ---------------- */

const OFFSET_KEY = "slw.offsetMs";
const OFFSET_STEP = 100;
const OFFSET_LIMIT = 3_000;
/** Jeda toleransi sebelum widget dianggap idle (anti-kedip). */
const IDLE_GRACE_MS = 2_500;

let track: TrackInfo | null = null;
let trackKey: string | null = null;
let lines: LyricsLine[] = [];


let anchorPosition = 0;
let anchorTime = performance.now();
let playing = false;
let lastActiveAt = 0;
/** Posisi dari snapshot sebelumnya, untuk mendeteksi posisi yang beku. */
let lastSnapshotPosition = -1;

let offsetMs = loadOffset();
let lastIndex = -2;
let pendingBackIndex = -2;
let pendingBackAt = 0;

let demoMode = false;
let demoLines: LyricsLine[] = [];
let demoStart = performance.now();
let infoStage = "demo";
let noticeTimer: number | undefined;

/* ---------------- helper ---------------- */

function loadOffset(): number {
  const raw = Number(localStorage.getItem(OFFSET_KEY));
  return Number.isFinite(raw) ? clamp(raw, -OFFSET_LIMIT, OFFSET_LIMIT) : 0;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function fmtOffset(value: number): string {
  if (value === 0) return "0 ms";
  const sign = value > 0 ? "+" : "\u2212";
  return `${sign}${Math.abs(value)} ms`;
}

function pushOffset(delta: number) {
  setOffset(offsetMs + delta);
}

function setOffset(value: number) {
  offsetMs = clamp(Math.round(value), -OFFSET_LIMIT, OFFSET_LIMIT);
  localStorage.setItem(OFFSET_KEY, String(offsetMs));

  if (el.offsetValue) {
    el.offsetValue.textContent = fmtOffset(offsetMs);
    // restart animasi
    el.offsetValue.classList.remove("bump");
    void el.offsetValue.offsetWidth;
    el.offsetValue.classList.add("bump");
  }
  lastIndex = -2;
}

function currentPositionMs(): number {
  const elapsed = playing ? performance.now() - anchorTime : 0;
  const duration = track?.durationMs ?? 0;
  const raw = anchorPosition + elapsed + offsetMs;
  return clamp(raw, 0, duration > 0 ? duration : Number.MAX_SAFE_INTEGER);
}

/* ---------------- render helpers ---------------- */

function setHeader(title: string, subtitle: string) {
  if (el.title) el.title.textContent = title;
  if (el.subtitle) el.subtitle.textContent = subtitle;
}

function setNotice(text: string) {
  if (!el.badge) return;
  el.badge.textContent = text;
  window.clearTimeout(noticeTimer);
  noticeTimer = window.setTimeout(() => {
    if (el.badge) el.badge.textContent = infoStage;
  }, 1_800);
}

function setBadge(text: string) {
  if (!el.badge) return;
  window.clearTimeout(noticeTimer);
  el.badge.textContent = text;
}

function showMessage(message: string) {
  if (el.prev) el.prev.textContent = "\u00a0";
  if (el.active) {
    el.active.textContent = message;
    el.active.classList.add("msg");
  }
  if (el.next) el.next.textContent = "\u00a0";
}

function setPlayIcon(isPlaying: boolean) {
  el.btnPlay?.classList.toggle("is-playing", isPlaying);
}

function findLineIndex(posMs: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let ans = -1;

  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].startMs <= posMs) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return ans;
}

function renderLines(index: number) {
  if (!el.active) return;

  el.active.classList.remove("msg");

  if (index < 0) {
    if (el.prev) el.prev.textContent = "\u00a0";
    el.active.textContent = "\u266a";
    if (el.next) el.next.textContent = lines[0]?.text ?? "\u00a0";
    return;
  }

  const current = lines[index];
  if (!current) return;

  if (el.prev) el.prev.textContent = index > 0 ? lines[index - 1].text : "\u00a0";
  el.active.textContent = current.text;
  if (el.next) el.next.textContent = lines[index + 1]?.text ?? "\u00a0";

  el.active.animate(
    [
      { opacity: 0, transform: "translateY(7px)" },
      { opacity: 1, transform: "translateY(0)" },
    ],
    { duration: 220, easing: "ease-out" },
  );
}

/* ---------------- playback ---------------- */

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
  lines = [];
  lastIndex = -2;
  expandWidget();
  setBadge("AD");
  setHeader("Iklan Spotify", "Lirik dijeda sementara");
  showMessage("Iklan \u2014 lirik dijeda");
}

function showIdle() {
  track = null;
  trackKey = null;
  lines = [];
  lastIndex = -2;
  playing = false;
  expandWidget();
  setBadge(infoStage);
  setPlayIcon(false);
  setHeader("Tidak ada lagu", "Putar lagu di Spotify");
  showMessage("Menunggu playback\u2026");
  if (el.progress) el.progress.style.width = "0%";
}

function onPlayback(snapshot: PlaybackSnapshot) {
  const now = performance.now();
  const nextPlaying = snapshot.status === "playing";
  const next = snapshot.track;
  const nextKey = next ? trackKeyOf(next) : null;
  const trackChanged = nextKey !== null && nextKey !== trackKey;
  const position = snapshot.positionMs;
  const previousPosition = lastSnapshotPosition;
  lastSnapshotPosition = position;

  if (trackChanged || nextPlaying !== playing) {
    // Lagu baru atau status berubah: posisi snapshot adalah acuan terbaik.
    anchorPosition = position;
    anchorTime = now;
    playing = nextPlaying;
  } else if (!nextPlaying) {
    anchorPosition = position;
    anchorTime = now;
  } else if (position === previousPosition) {
    // Sebagian sumber (mis. YouTube di browser) hanya memperbarui posisi saat
    // play/pause/seek. Kalau posisi beku tetap dipakai sebagai acuan, baris
    // lirik akan ditarik mundur terus dan tidak pernah berganti. Biarkan jam
    // lokal berjalan sampai snapshot benar-benar bergerak.
  } else {
    const estimate = anchorPosition + (now - anchorTime);
    const drift = position - estimate;

    if (position < previousPosition - 1_200) {
      // Seek mundur yang nyata.
      anchorPosition = position;
      anchorTime = now;
    } else if (drift > 900) {
      // Sumber lebih maju (seek maju atau jam lokal tertinggal): ikuti.
      anchorPosition = position;
      anchorTime = now;
    } else if (drift < -900) {
      // Sumber tertinggal, bukan seek: koreksi pelan supaya baris tidak
      // melompat mundur.
      anchorPosition = estimate + drift * 0.15;
      anchorTime = now;
    } else {
      anchorPosition = estimate + drift * 0.25;
      anchorTime = now;
    }
  }

  setPlayIcon(playing);

  if (!snapshot.track) {
    // Abaikan idle sesaat (mis. Spotify sedang berganti lagu).
    if (track && performance.now() - lastActiveAt < IDLE_GRACE_MS) return;
    showIdle();
    return;
  }

  lastActiveAt = performance.now();

  if (!next) {
    showIdle();
    return;
  }

  if (nextKey === trackKey) {
    track = next;
    return;
  }

  track = next;
  trackKey = nextKey;
  lines = [];
  lastIndex = -2;
  expandWidget();

  if (isAdTrack(next)) {
    showAd();
    return;
  }

  setHeader(next.title, [next.artist, next.album].filter(Boolean).join(" \u2014 "));
  showMessage("Memuat lirik\u2026");
}

function onLyrics(state: LyricsState) {
  setBadge(infoStage);

  switch (state.kind) {
    case "ready": {
      const nextLines = state.doc.lines ?? [];
      lines = nextLines;
      lastIndex = -2;
      // Lirik tanpa timestamp: beri tanda agar pengguna tahu waktunya perkiraan.
      if (state.doc.unsynced) setBadge("PLAIN");
      if (nextLines.length === 0) {
        expandWidget();
        showMessage("Lirik kosong");
      } else if (!isHovered()) {
        scheduleCompact(COMPACT_AFTER_CONTENT_MS);
      }
      break;
    }

    case "instrumental":
      lines = [];
      lastIndex = -2;
      expandWidget();
      showMessage("Instrumental \u2014 tanpa lirik");
      break;

    case "ad":
      showAd();
      break;

    case "not_found":
      lines = [];
      lastIndex = -2;
      expandWidget();
      showMessage("Lirik tidak tersedia");
      break;

    case "error":
      lines = [];
      lastIndex = -2;
      expandWidget();
      showMessage(state.message || "Gagal memuat lirik");
      break;

    case "loading":
      if (lines.length === 0) showMessage("Memuat lirik\u2026");
      break;

    case "idle":
      // Jangan langsung menghapus lirik: beri waktu sebelum idle sungguhan.
      if (track && performance.now() - lastActiveAt < IDLE_GRACE_MS) return;
      showIdle();
      break;
  }
}

/* ---------------- mode ringkas ---------------- */

/**
 * Saat kursor tidak berada di atas widget, kartu disembunyikan dan hanya teks
 * lirik ber-outline yang tersisa. Begitu kursor mendekat, tampilan penuh
 * kembali. Tidak ada resize window supaya transisi tidak berkedip.
 */
const COMPACT_DELAY_MS = 700;
const COMPACT_AFTER_CONTENT_MS = 1_100;
const START_COMPACT_MS = 2_400;

let compactTimer: number | undefined;
let pointerDownAt = 0;
/** Batas waktu penjaga drag; kalau pointerup tidak sampai ke webview (mis.
 *  jendela digeser), mode ringkas tetap bisa aktif lagi setelah ini. */
const POINTER_HOLD_MAX_MS = 4_000;

function widgetEl(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".widget");
}

function isHovered(): boolean {
  return widgetEl()?.matches(":hover") ?? false;
}

function isPointerHeld(): boolean {
  return pointerDownAt > 0 && performance.now() - pointerDownAt < POINTER_HOLD_MAX_MS;
}

function expandWidget() {
  window.clearTimeout(compactTimer);
  compactTimer = undefined;
  widgetEl()?.classList.remove("compact");
}

function collapseWidget() {
  compactTimer = undefined;
  const node = widgetEl();
  if (!node) return;

  // Jangan mengecil saat kursor masih di atas, sedang drag, atau belum ada
  // lirik yang bisa ditampilkan.
  if (isPointerHeld() || isHovered() || lines.length === 0) return;

  node.classList.add("compact");
}

function scheduleCompact(delay = COMPACT_DELAY_MS) {
  window.clearTimeout(compactTimer);
  compactTimer = window.setTimeout(collapseWidget, delay);
}

function wireCompactMode() {
  const node = widgetEl();
  if (!node) return;

  node.addEventListener("mouseenter", expandWidget);
  node.addEventListener("mouseleave", () => scheduleCompact());
  node.addEventListener("pointerdown", () => {
    pointerDownAt = performance.now();
    expandWidget();
  });

  const release = () => {
    if (pointerDownAt === 0) return;
    pointerDownAt = 0;
    if (!isHovered()) scheduleCompact();
  };
  window.addEventListener("pointerup", release);
  window.addEventListener("pointercancel", release);

  // Tampil penuh dulu, lalu mengecil sendiri kalau kursor tidak di atasnya.
  scheduleCompact(START_COMPACT_MS);
}

/* ---------------- render loop ---------------- */

function tick() {
  if (demoMode) {
    const pos = performance.now() - demoStart;
    if (pos > 24_000) {
      demoStart = performance.now();
      lastIndex = -2;
    }
    lines = demoLines;
  }

  const pos = demoMode ? performance.now() - demoStart : currentPositionMs();
  const duration = demoMode ? 24_000 : (track?.durationMs ?? 0);

  if (el.progress) {
    const ratio = duration > 0 ? clamp01(pos / duration) : 0;
    el.progress.style.width = `${ratio * 100}%`;
  }

  if (lines.length > 0) {
    const index = findLineIndex(pos);
    if (index !== lastIndex) {
      // Mundur satu baris karena jitter posisi GSMTC: tunggu stabil dulu
      // sebelum benar-benar mengganti baris (anti-kedip di batas waktu).
      if (index < lastIndex) {
        if (index !== pendingBackIndex) {
          pendingBackIndex = index;
          pendingBackAt = performance.now();
        } else if (performance.now() - pendingBackAt >= 140) {
          pendingBackIndex = -2;
          lastIndex = index;
          renderLines(index);
        }
      } else {
        pendingBackIndex = -2;
        lastIndex = index;
        renderLines(index);
      }
    }
  }
}

function loop() {
  tick();
  requestAnimationFrame(loop);
}

function startRenderLoop() {
  requestAnimationFrame(loop);
  // WebView2 bisa berhenti memanggil requestAnimationFrame saat jendela tidak
  // fokus/tertutup. Timer ini menjaga baris lirik tetap berganti dan sekaligus
  // memicu repaint ketika teks berubah.
  window.setInterval(tick, 200);
}

/* ---------------- kontrol ---------------- */

async function sendControl(action: "toggle" | "next" | "previous") {
  if (demoMode) return;

  const wasPlaying = playing;
  const beforePos = currentPositionMs();

  // Umpan balik instan untuk tombol play/pause.
  if (action === "toggle") {
    playing = !wasPlaying;
    anchorPosition = beforePos;
    anchorTime = performance.now();
    setPlayIcon(playing);
  }

  try {
    await invoke("playback_control", { action });
  } catch (error) {
    // Perintah gagal (mis. tidak ada sesi media): kembalikan ikon.
    if (action === "toggle") {
      playing = wasPlaying;
      anchorPosition = beforePos;
      anchorTime = performance.now();
      setPlayIcon(playing);
    }
    setNotice("GAGAL");
    console.error("playback_control", error);
  }
}

async function quitApp() {
  try {
    await invoke("quit_app");
  } catch (error) {
    console.error("quit_app", error);
    setNotice("GAGAL");
  }
}

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

/* ---------------- bootstrap ---------------- */

function startDemo() {
  demoMode = true;
  setHeader("Creep", "Radiohead \u2014 Pablo Honey");
  demoLines = [
    { text: "When you were here before", startMs: 0, endMs: 4000 },
    { text: "Couldn't look you in the eye", startMs: 4000, endMs: 8000 },
    { text: "You're just like an angel", startMs: 8000, endMs: 12000 },
    { text: "Your skin makes me cry", startMs: 12000, endMs: 16000 },
    { text: "You float like a feather", startMs: 16000, endMs: 20000 },
    { text: "In a beautiful world", startMs: 20000, endMs: 24000 },
  ];
  demoStart = performance.now();
  lines = demoLines;
  lastIndex = -2;
  playing = true;
  setPlayIcon(true);
}

window.addEventListener("DOMContentLoaded", async () => {
  // Tampilkan offset tersimpan sejak awal.
  if (el.offsetValue) el.offsetValue.textContent = fmtOffset(offsetMs);

  wireControls();
  wireCompactMode();

  try {
    const info = await invoke<{ stage: string }>("app_info");
    infoStage = info.stage;
    if (el.badge) el.badge.textContent = info.stage;
  } catch {
    if (el.badge) el.badge.textContent = "web";
  }

  try {
    await Promise.all([
      listen<PlaybackSnapshot>("playback:update", (event) => onPlayback(event.payload)),
      listen<LyricsState>("lyrics:update", (event) => onLyrics(event.payload)),
    ]);
  } catch {
    // Dibuka di browser biasa, bukan Tauri → pakai demo
    startDemo();
  }

  startRenderLoop();
});
