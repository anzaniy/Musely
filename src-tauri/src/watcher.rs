use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::{lyrics, playback};

/// Berapa kali pembacaan GSMTC boleh gagal sebelum widget dianggap idle.
/// 3 × 500 ms ≈ 1,5 detik — cukup untuk menutupi gangguan sesaat tanpa
/// membuat lirik berkedip.
const IDLE_MISS_LIMIT: u32 = 3;

/// Hasil fetch hanya boleh dipakai kalau lagunya masih yang diputar.
fn is_still_current(current: &Mutex<Option<String>>, key: &Option<String>) -> bool {
    current
        .lock()
        .map(|c| c.as_ref() == key.as_ref())
        .unwrap_or(false)
}

/// Polling GSMTC + pengambilan lirik saat lagu berganti.
pub(crate) fn start_playback_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        playback::init_com();

        let mut last_key: Option<String> = None;
        let mut misses: u32 = 0;
        let mut idle_announced = false;
        // Cache lirik di memori supaya memutar ulang lagu yang sama tidak
        // menampilkan "Memuat lirik…" lagi.
        let cache: Arc<Mutex<HashMap<String, lyrics::LyricsState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Lagu yang sedang diputar. Hasil fetch lagu lama tidak boleh menimpa
        // lirik lagu baru setelah pengguna skip.
        let current_key: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        loop {
            // Abaikan snapshot tanpa track: bisa jadi gangguan sesaat saat
            // Spotify berganti lagu atau sedang membuka sesi baru.
            let snapshot = playback::read().filter(|s| s.track.is_some());

            match snapshot {
                Some(snapshot) => {
                    misses = 0;
                    idle_announced = false;
                    let _ = app.emit("playback:update", &snapshot);

                    let key = snapshot
                        .track
                        .as_ref()
                        .map(|t| format!("{}|{}|{}", t.title, t.artist, t.duration_ms));

                    if key != last_key {
                        last_key = key.clone();
                        if let Ok(mut current) = current_key.lock() {
                            *current = key.clone();
                        }

                        if let Some(track) = snapshot.track.clone() {
                            if lyrics::is_ad(&track) {
                                // Iklan Spotify: jangan panggil LRCLIB.
                                let _ = app.emit("lyrics:update", lyrics::LyricsState::Ad);
                            } else {
                                let cached =
                                    key.as_ref().and_then(|k| {
                                        cache.lock().ok().and_then(|c| c.get(k).cloned())
                                    });

                                if let Some(state) = cached {
                                    let _ = app.emit("lyrics:update", state);
                                } else {
                                    let _ =
                                        app.emit("lyrics:update", lyrics::LyricsState::Loading);

                                    let handle = app.clone();
                                    let cache_key = key.clone();
                                    let cache_ref = Arc::clone(&cache);
                                    let current_ref = Arc::clone(&current_key);
                                    tauri::async_runtime::spawn(async move {
                                        let mut state = lyrics::fetch(&track).await;

                                        // Error bisa berarti jaringan/LRCLIB sedang
                                        // gangguan: coba lagi sebelum menyerah.
                                        let mut retries = 0;
                                        while matches!(&state, lyrics::LyricsState::Error { .. })
                                            && retries < 2
                                        {
                                            retries += 1;
                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            if !is_still_current(&current_ref, &cache_key) {
                                                break;
                                            }
                                            state = lyrics::fetch(&track).await;
                                        }

                                        if is_still_current(&current_ref, &cache_key) {
                                            let _ = handle.emit("lyrics:update", state.clone());
                                        }

                                        // Simpan hanya hasil final, bukan error/timeout.
                                        let cacheable = matches!(
                                            state,
                                            lyrics::LyricsState::Ready { .. }
                                                | lyrics::LyricsState::Instrumental
                                                | lyrics::LyricsState::NotFound
                                        );
                                        if cacheable {
                                            if let (Some(k), Ok(mut map)) =
                                                (cache_key, cache_ref.lock())
                                            {
                                                map.insert(k, state);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
                None => {
                    misses += 1;
                    // Emit idle pertama setelah ~1,5 dtk, lalu ulangi tiap
                    // ~6 dtk supaya frontend yang ter-reload bisa sinkron lagi.
                    let should_emit =
                        misses >= IDLE_MISS_LIMIT && (!idle_announced || misses % 12 == 0);

                    if should_emit {
                        idle_announced = true;
                        last_key = None;
                        if let Ok(mut current) = current_key.lock() {
                            *current = None;
                        }
                        let _ = app.emit("playback:update", playback::PlaybackSnapshot::idle());
                        let _ = app.emit("lyrics:update", lyrics::LyricsState::Idle);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_result_for_old_track() {
        let current = Mutex::new(Some("B".to_string()));
        assert!(!is_still_current(&current, &Some("A".to_string())));
        assert!(is_still_current(&current, &Some("B".to_string())));

        *current.lock().unwrap() = None;
        assert!(!is_still_current(&current, &Some("B".to_string())));
    }
}