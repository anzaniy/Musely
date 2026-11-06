export const el = {
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

let noticeTimer: number | undefined;

export function setHeader(title: string, subtitle: string) {
  if (el.title) el.title.textContent = title;
  if (el.subtitle) el.subtitle.textContent = subtitle;
}

/** Tampilkan pesan singkat di badge, lalu kembalikan ke `restore`. */
export function setNotice(text: string, restore: string) {
  if (!el.badge) return;
  el.badge.textContent = text;
  window.clearTimeout(noticeTimer);
  noticeTimer = window.setTimeout(() => {
    if (el.badge) el.badge.textContent = restore;
  }, 1_800);
}

export function setBadge(text: string) {
  if (!el.badge) return;
  window.clearTimeout(noticeTimer);
  el.badge.textContent = text;
}

export function showMessage(message: string) {
  if (el.prev) el.prev.textContent = "\u00a0";
  if (el.active) {
    el.active.textContent = message;
    el.active.classList.add("msg");
  }
  if (el.next) el.next.textContent = "\u00a0";
}

export function setPlayIcon(isPlaying: boolean) {
  el.btnPlay?.classList.toggle("is-playing", isPlaying);
}