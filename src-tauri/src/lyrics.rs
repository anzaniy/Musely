//! Klien LRCLIB + parser lirik.
//!
//! Dokumentasi API: https://lrclib.net/docs.txt
//! - GET /api/get     → pencarian presisi (durasi ±2 detik)
//! - GET /api/search  → fallback kata kunci

use crate::playback::TrackInfo;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://lrclib.net";
const USER_AGENT: &str = concat!(
    "Musely/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/example/musely)"
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    id: i64,
    #[allow(dead_code)]
    track_name: String,
    #[allow(dead_code)]
    artist_name: String,
    #[allow(dead_code)]
    album_name: String,
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    has_word_sync: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsLine {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsDoc {
    pub track_key: String,
    pub instrumental: bool,
    pub has_word_sync: bool,
    pub unsynced: bool,
    pub lines: Vec<LyricsLine>,
    pub provider: String,
    pub provider_id: Option<i64>,
    pub fetched_at: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
pub enum LyricsState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "loading")]
    Loading,
    #[serde(rename = "ready")]
    Ready { doc: LyricsDoc },
    #[serde(rename = "instrumental")]
    Instrumental,
    #[serde(rename = "ad")]
    Ad,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "error")]
    Error { message: String },
}

/// Spotify menyisipkan iklan lewat sesi media yang sama. GSMTC melaporkannya
/// dengan artis/album "Spotify", jadi jangan sampai widget mencari liriknya.
pub fn is_ad(track: &TrackInfo) -> bool {
    let artist = track.artist.trim().to_lowercase();
    let album = track.album.trim().to_lowercase();
    let title = track.title.trim().to_lowercase();

    artist == "spotify"
        || album == "spotify"
        || artist == "advertisement"
        || title == "advertisement"
        || title.contains("dengarkan musik tanpa iklan")
        || title.contains("listen to music without ads")
}

/// Kunci cache: hash metadata yang sudah dinormalisasi.
pub fn track_key(track: &TrackInfo) -> String {
    let normalize = |s: &str| -> String {
        let lowered = s.to_lowercase();
        let mut out = String::with_capacity(lowered.len());
        let mut depth = 0i32;
        for ch in lowered.chars() {
            match ch {
                '(' | '[' => depth += 1,
                ')' | ']' => depth = (depth - 1).max(0),
                _ if depth > 0 => {}
                c if c.is_alphanumeric() => out.push(c),
                _ => out.push(' '),
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    let seconds = (track.duration_ms as f64 / 1000.0).round() as u64;
    format!(
        "{}|{}|{}",
        normalize(&track.artist),
        normalize(&track.title),
        seconds
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())
}

enum Lookup {
    Found(reqwest::Response),
    /// LRCLIB menjawab bahwa lagunya memang tidak ada (404/400).
    Empty,
    /// Gangguan sementara (429/5xx). Jangan dianggap "tidak ada".
    Transient,
}

fn is_transient(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status.as_u16() == 429
}

/// Kirim request dengan satu percobaan ulang untuk error jaringan atau 5xx.
/// LRCLIB cukup sering membalas 503/429 sesaat.
async fn send_with_retry(
    client: &reqwest::Client,
    url: &str,
    params: &[(&str, String)],
) -> Result<Lookup, reqwest::Error> {
    let mut last_error: Option<reqwest::Error> = None;

    for attempt in 0..2u32 {
        match client.get(url).query(params).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(Lookup::Found(resp)),
            Ok(resp) => {
                if !is_transient(resp.status()) {
                    return Ok(Lookup::Empty);
                }
                if attempt == 1 {
                    return Ok(Lookup::Transient);
                }
            }
            Err(error) => {
                last_error = Some(error);
                if attempt == 1 {
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1_000)).await;
    }

    Err(last_error.expect("percobaan kedua selalu menyimpan error"))
}

pub async fn fetch(track: &TrackInfo) -> LyricsState {
    if is_ad(track) {
        return LyricsState::Ad;
    }

    let client = match build_client() {
        Ok(c) => c,
        Err(message) => return LyricsState::Error { message },
    };

    let seconds = if track.duration_ms > 0 {
        ((track.duration_ms as f64) / 1000.0).round() as u64
    } else {
        0
    };

    eprintln!(
        "[lyrics] cari \"{}\" - \"{}\" ({} ms)",
        track.title, track.artist, track.duration_ms
    );

    let mut params: Vec<(&str, String)> = vec![
        ("track_name", track.title.clone()),
        ("artist_name", track.artist.clone()),
    ];
    if !track.album.is_empty() {
        params.push(("album_name", track.album.clone()));
    }
    if seconds > 0 {
        params.push(("duration", seconds.to_string()));
    }

    // Dipakai kalau pencarian bebas tidak menemukan hasil yang lebih baik.
    let mut exact: Option<Record> = None;

    match send_with_retry(&client, &format!("{BASE_URL}/api/get"), &params).await {
        Ok(Lookup::Found(resp)) => {
            if let Ok(record) = resp.json::<Record>().await {
                if record.synced_lyrics.is_some() {
                    eprintln!("[lyrics] exact hit id={} (synced)", record.id);
                    return to_state(track, record);
                }
                eprintln!(
                    "[lyrics] exact hit id={} tanpa synced, coba search",
                    record.id
                );
                exact = Some(record);
            }
        }
        Ok(Lookup::Empty) => {
            eprintln!("[lyrics] /api/get tidak ada hasil");
        }
        Ok(Lookup::Transient) => {
            eprintln!("[lyrics] /api/get gangguan sementara, coba search");
        }
        Err(e) => {
            eprintln!("[lyrics] /api/get error: {e}");
        }
    }

    // Fallback: pencarian bebas
    let query = format!("{} {}", track.title, track.artist);
    let search_params: Vec<(&str, String)> = vec![("q", query.clone())];

    match send_with_retry(&client, &format!("{BASE_URL}/api/search"), &search_params).await {
        Ok(Lookup::Found(resp)) => {
            if let Ok(records) = resp.json::<Vec<Record>>().await {
                eprintln!("[lyrics] search \"{}\" -> {} hasil", query, records.len());
                if let Some(best) = pick_best(records, track.duration_ms) {
                    eprintln!(
                        "[lyrics] pilih id={} synced={} dur={}",
                        best.id,
                        best.synced_lyrics.is_some(),
                        best.duration
                    );
                    return to_state(track, best);
                }
                eprintln!("[lyrics] tidak ada hasil yang cocok");
            }
            match exact {
                Some(record) => to_state(track, record),
                None => LyricsState::NotFound,
            }
        }
        Ok(Lookup::Empty) => {
            eprintln!("[lyrics] /api/search tidak ada hasil");
            match exact {
                Some(record) => to_state(track, record),
                None => LyricsState::NotFound,
            }
        }
        Ok(Lookup::Transient) => {
            eprintln!("[lyrics] /api/search gangguan sementara");
            match exact {
                Some(record) => to_state(track, record),
                None => LyricsState::Error {
                    message: "LRCLIB sedang sibuk, coba lagi".into(),
                },
            }
        }
        Err(e) => {
            eprintln!("[lyrics] /api/search error: {e}");
            match exact {
                Some(record) => to_state(track, record),
                None => LyricsState::Error {
                    message: e.to_string(),
                },
            }
        }
    }
}

/// Pilih hasil terbaik: utamakan yang punya lirik tersinkron, lalu durasi
/// terdekat. Tanpa ini, versi "plain only" bisa menang hanya karena muncul
/// lebih dulu di hasil pencarian.
fn pick_best(records: Vec<Record>, target_ms: u64) -> Option<Record> {
    let target_s = target_ms as f64 / 1000.0;

    records
        .into_iter()
        .filter(|r| !r.instrumental)
        .filter(|r| target_s <= 0.0 || (r.duration - target_s).abs() <= 8.0)
        .min_by(|a, b| {
            let synced_a = a.synced_lyrics.is_some();
            let synced_b = b.synced_lyrics.is_some();

            synced_b.cmp(&synced_a).then_with(|| {
                let da = (a.duration - target_s).abs();
                let db = (b.duration - target_s).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
        })
}

fn to_state(track: &TrackInfo, record: Record) -> LyricsState {
    let key = track_key(track);

    if record.instrumental {
        return LyricsState::Instrumental;
    }

    if let Some(lrc) = record.synced_lyrics.as_deref() {
        let lines = parse_lrc(lrc);
        if lines.len() >= 2 {
            return LyricsState::Ready {
                doc: LyricsDoc {
                    track_key: key,
                    instrumental: false,
                    has_word_sync: record.has_word_sync,
                    unsynced: false,
                    lines,
                    provider: "lrclib".into(),
                    provider_id: Some(record.id),
                    fetched_at: now_ms(),
                },
            };
        }
    }

    if let Some(plain) = record.plain_lyrics.as_deref() {
        let texts: Vec<&str> = plain
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        if !texts.is_empty() {
            // Lirik tanpa timestamp: sebar merata sepanjang durasi lagu supaya
            // tidak melenceng jauh seperti kalau memakai 4 detik per baris.
            let total = if track.duration_ms > 0 {
                track.duration_ms
            } else {
                texts.len() as u64 * 4_000
            };
            let per_line = (total / texts.len() as u64).clamp(1_500, 8_000);

            let lines: Vec<LyricsLine> = texts
                .iter()
                .enumerate()
                .map(|(i, text)| LyricsLine {
                    text: (*text).to_string(),
                    start_ms: i as u64 * per_line,
                    end_ms: (i as u64 + 1) * per_line,
                })
                .collect();

            return LyricsState::Ready {
                doc: LyricsDoc {
                    track_key: key,
                    instrumental: false,
                    has_word_sync: false,
                    unsynced: true,
                    lines,
                    provider: "lrclib".into(),
                    provider_id: Some(record.id),
                    fetched_at: now_ms(),
                },
            };
        }
    }

    LyricsState::NotFound
}

/// Parse LRC: `[mm:ss.xx] teks`, mendukung beberapa timestamp per baris.
fn parse_lrc(input: &str) -> Vec<LyricsLine> {
    let mut offset_ms: i64 = 0;
    let mut raw: Vec<(i64, String)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // metadata tag: [offset:123]
        if line.starts_with("[offset:") {
            if let Some(value) = line
                .trim_start_matches("[offset:")
                .split(']')
                .next()
                .and_then(|v| v.trim().parse::<i64>().ok())
            {
                offset_ms = value;
            }
            continue;
        }

        let Some(close) = line.find(']') else { continue };
        if !line.starts_with('[') {
            continue;
        }

        let stamp = &line[1..close];
        let Some(ms) = parse_timestamp(stamp) else {
            continue;
        };

        let text = line[close + 1..].trim();

        // Satu baris boleh punya beberapa timestamp: [00:10.00][01:20.00] reff
        let mut stamps = vec![ms];
        let mut rest = text;
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let Some(extra) = parse_timestamp(&stripped[..end]) else {
                break;
            };
            stamps.push(extra);
            rest = stripped[end + 1..].trim_start();
        }

        let text = rest.to_string();
        for stamp_ms in stamps {
            raw.push((stamp_ms, text.clone()));
        }
    }

    raw.sort_by_key(|(ms, _)| *ms);

    let len = raw.len();
    let mut lines = Vec::with_capacity(len);

    for (i, (ms, text)) in raw.iter().enumerate() {
        let start = (*ms + offset_ms).max(0) as u64;
        let end = raw
            .get(i + 1)
            .map(|(next, _)| ((*next + offset_ms).max(0) as u64).max(start + 300))
            .unwrap_or(start + 4_000);

        if text.is_empty() {
            continue;
        }

        lines.push(LyricsLine {
            text: text.clone(),
            start_ms: start,
            end_ms: end,
        });
    }

    lines
}

fn parse_timestamp(stamp: &str) -> Option<i64> {
    let (mm, rest) = stamp.split_once(':')?;
    let minutes: i64 = mm.trim().parse().ok()?;

    let (ss, frac) = match rest.split_once('.') {
        Some((s, f)) => (s, Some(f)),
        None => (rest, None),
    };

    let seconds: i64 = ss.trim().parse().ok()?;

    let millis: i64 = match frac {
        Some(f) => {
            let digits: String = f.chars().take(3).collect();
            match digits.len() {
                1 => digits.parse::<i64>().ok()? * 100,
                2 => digits.parse::<i64>().ok()? * 10,
                _ => digits.parse::<i64>().ok()?,
            }
        }
        None => 0,
    };

    Some((minutes * 60 + seconds) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_lrc() {
        let lrc = "[00:19.16]When you were here before\n[00:24.09]Couldn't look you in the eye\n";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start_ms, 19_160);
        assert_eq!(lines[1].start_ms, 24_090);
        assert_eq!(lines[0].end_ms, 24_090);
    }

    #[test]
    fn parses_three_digit_millis() {
        let lrc = "[01:05.830]I'm a weirdo";
        let lines = parse_lrc(lrc);
        assert_eq!(lines[0].start_ms, 65_830);
    }

    #[test]
    fn applies_offset_tag() {
        let lrc = "[offset:-500]\n[00:10.00]Hello\n[00:20.00]World";
        let lines = parse_lrc(lrc);
        assert_eq!(lines[0].start_ms, 9_500);
    }

    #[test]
    fn prefers_synced_over_plain() {
        let make = |id: i64, synced: Option<&str>| Record {
            id,
            track_name: "Terpesona".into(),
            artist_name: "Bunga Citra Lestari".into(),
            album_name: String::new(),
            duration: 267.0,
            instrumental: false,
            has_word_sync: false,
            plain_lyrics: Some("lirik biasa".into()),
            synced_lyrics: synced.map(str::to_string),
        };

        let records = vec![
            make(1, None),
            make(2, Some("[00:01.00]halo\n[00:05.00]dunia")),
        ];

        let best = pick_best(records, 267_000).expect("harus ada hasil");
        assert_eq!(best.id, 2);
    }

    #[test]
    fn skips_results_far_from_target_duration() {
        let make = |id: i64, duration: f64| Record {
            id,
            track_name: "X".into(),
            artist_name: "Y".into(),
            album_name: String::new(),
            duration,
            instrumental: false,
            has_word_sync: false,
            plain_lyrics: Some("lirik".into()),
            synced_lyrics: None,
        };

        let best = pick_best(vec![make(1, 100.0), make(2, 300.0)], 267_000);
        assert!(best.is_none());
    }

    #[test]
    fn detects_spotify_ads() {
        let ad = TrackInfo {
            id: None,
            title: "Dengarkan musik tanpa iklan.".into(),
            artist: "Spotify".into(),
            album: "Spotify".into(),
            duration_ms: 30_000,
            cover_url: None,
            source: "gsmtc".into(),
        };
        assert!(is_ad(&ad));

        let song = TrackInfo {
            id: None,
            title: "Now I Know".into(),
            artist: "Kaleb J".into(),
            album: "Now I Know".into(),
            duration_ms: 258_050,
            cover_url: None,
            source: "gsmtc".into(),
        };
        assert!(!is_ad(&song));
    }

    #[test]
    fn expands_multi_timestamp_lines() {
        let lrc = "[00:10.00][01:20.00]chorus\n[00:30.00]verse";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start_ms, 10_000);
        assert_eq!(lines[1].start_ms, 30_000);
        assert_eq!(lines[2].start_ms, 80_000);
        assert_eq!(lines[2].text, "chorus");
    }

    #[test]
    fn treats_429_and_5xx_as_transient() {
        assert!(is_transient(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_transient(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_transient(reqwest::StatusCode::NOT_FOUND));
        assert!(!is_transient(reqwest::StatusCode::BAD_REQUEST));
    }

    #[test]
    fn normalizes_track_key() {
        let track = TrackInfo {
            id: None,
            title: "Creep (Remastered 2011)".into(),
            artist: "Radiohead".into(),
            album: "Pablo Honey".into(),
            duration_ms: 238_640,
            cover_url: None,
            source: "gsmtc".into(),
        };
        assert_eq!(track_key(&track), "radiohead|creep|239");
    }
}
