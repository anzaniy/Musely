import { state } from "./state";

/**
 * Saat kursor tidak berada di atas widget, kartu disembunyikan dan hanya teks
 * lirik ber-outline yang tersisa. Begitu kursor mendekat, tampilan penuh
 * kembali. Tidak ada resize window supaya transisi tidak berkedip.
 */
const COMPACT_DELAY_MS = 700;
export const COMPACT_AFTER_CONTENT_MS = 1_100;
const START_COMPACT_MS = 2_400;

let compactTimer: number | undefined;
let pointerDownAt = 0;
/** Batas waktu penjaga drag; kalau pointerup tidak sampai ke webview (mis.
 *  jendela digeser), mode ringkas tetap bisa aktif lagi setelah ini. */
const POINTER_HOLD_MAX_MS = 4_000;

function widgetEl(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".widget");
}

export function isHovered(): boolean {
  return widgetEl()?.matches(":hover") ?? false;
}

function isPointerHeld(): boolean {
  return pointerDownAt > 0 && performance.now() - pointerDownAt < POINTER_HOLD_MAX_MS;
}

export function expandWidget() {
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
  if (isPointerHeld() || isHovered() || state.lines.length === 0) return;

  node.classList.add("compact");
}

export function scheduleCompact(delay = COMPACT_DELAY_MS) {
  window.clearTimeout(compactTimer);
  compactTimer = window.setTimeout(collapseWidget, delay);
}

export function wireCompactMode() {
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