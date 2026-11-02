//! Sumber playback via GSMTC (Global System Media Transport Controls).
//!
//! Membaca sesi media yang sedang aktif di Windows tanpa OAuth.
//! API ini memberi judul, artis, album, durasi, posisi, dan status putar.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

/// TimeSpan Windows memakai satuan 100 nanodetik.
const TICKS_PER_MS: i64 = 10_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    /// GSMTC tidak memberi ID stabil; dipakai hash metadata di frontend.
    pub id: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub track: Option<TrackInfo>,
    /// "playing" | "paused" | "stopped"
    pub status: String,
    pub position_ms: u64,
    /// epoch milidetik saat snapshot diambil
    pub captured_at: u64,
}

impl PlaybackSnapshot {
    pub fn idle() -> Self {
        Self {
            track: None,
            status: "stopped".into(),
            position_ms: 0,
            captured_at: now_ms(),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Wajib dipanggil sekali di thread yang akan memakai WinRT.
pub fn init_com() {
    // HRESULT diabaikan: S_FALSE / RPC_E_CHANGED_MODE tetap boleh lanjut.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn ticks_to_ms(ticks: i64) -> u64 {
    if ticks <= 0 {
        0
    } else {
        (ticks / TICKS_PER_MS) as u64
    }
}

/// Mengambil snapshot playback saat ini. `None` jika tidak ada sesi aktif.
pub fn read() -> Option<PlaybackSnapshot> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .join()
        .ok()?;

    let session = manager.GetCurrentSession().ok()?;

    let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;

    let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
    if title.trim().is_empty() {
        return Some(PlaybackSnapshot::idle());
    }

    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default();

    let playback_info = session.GetPlaybackInfo().ok()?;
    let raw_status = playback_info
        .PlaybackStatus()
        .unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed);

    let status = if raw_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
        "playing"
    } else if raw_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused {
        "paused"
    } else {
        "stopped"
    };

    let (position_ms, duration_ms) = match session.GetTimelineProperties() {
        Ok(timeline) => {
            let pos = timeline.Position().map(|t| ticks_to_ms(t.Duration)).unwrap_or(0);
            let end = timeline.EndTime().map(|t| ticks_to_ms(t.Duration)).unwrap_or(0);
            let start = timeline.StartTime().map(|t| ticks_to_ms(t.Duration)).unwrap_or(0);
            (pos.saturating_sub(start), end.saturating_sub(start))
        }
        Err(_) => (0, 0),
    };

    Some(PlaybackSnapshot {
        track: Some(TrackInfo {
            id: None,
            title,
            artist,
            album,
            duration_ms,
            cover_url: None,
            source: "gsmtc".into(),
        }),
        status: status.into(),
        position_ms,
        captured_at: now_ms(),
    })
}

/* ------------------------------------------------------------------
 * Kontrol playback
 * ------------------------------------------------------------------ */

/// Aksi yang bisa dikirim ke sesi media aktif.
#[derive(Clone, Copy, Debug)]
pub enum ControlAction {
    TogglePlayPause,
    Next,
    Previous,
}

impl ControlAction {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "toggle" | "toggle_play_pause" => Some(Self::TogglePlayPause),
            "next" => Some(Self::Next),
            "previous" | "prev" => Some(Self::Previous),
            _ => None,
        }
    }
}

fn send(action: ControlAction) -> Result<bool, String> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?;

    let session: GlobalSystemMediaTransportControlsSession =
        manager.GetCurrentSession().map_err(|e| e.to_string())?;

    let operation = match action {
        ControlAction::TogglePlayPause => session.TryTogglePlayPauseAsync(),
        ControlAction::Next => session.TrySkipNextAsync(),
        ControlAction::Previous => session.TrySkipPreviousAsync(),
    }
    .map_err(|e| e.to_string())?;

    operation.join().map_err(|e| e.to_string())
}

/// Menjalankan aksi di thread sendiri karena `CoInitializeEx` bersifat per-thread
/// dan Tauri memanggil command dari thread yang berbeda-beda.
pub fn control(action: ControlAction) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<bool, String>>();

    std::thread::spawn(move || {
        init_com();
        let _ = tx.send(send(action));
    });

    match rx.recv_timeout(std::time::Duration::from_secs(4)) {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err("Perintah ditolak oleh aplikasi pemutar".into()),
        Ok(Err(message)) => Err(message),
        Err(_) => Err("Timeout saat menghubungi pemutar".into()),
    }
}