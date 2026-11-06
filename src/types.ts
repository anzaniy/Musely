/* Tipe data — harus sama dengan struct Rust di src-tauri/src/ */

export type PlaybackStatus = "playing" | "paused" | "stopped";

export interface TrackInfo {
  id: string | null;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  coverUrl: string | null;
  source: string;
}

export interface PlaybackSnapshot {
  track: TrackInfo | null;
  status: PlaybackStatus;
  positionMs: number;
  capturedAt: number;
}

export interface LyricsLine {
  text: string;
  startMs: number;
  endMs: number;
}

export interface LyricsDoc {
  trackKey: string;
  instrumental: boolean;
  hasWordSync: boolean;
  unsynced: boolean;
  lines: LyricsLine[];
  provider: string;
  providerId: number | null;
  fetchedAt: number;
}

export type LyricsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; doc: LyricsDoc }
  | { kind: "instrumental" }
  | { kind: "ad" }
  | { kind: "not_found" }
  | { kind: "error"; message: string };