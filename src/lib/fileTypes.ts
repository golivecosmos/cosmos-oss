// Keep in sync with src-tauri/src/commands/indexing.rs::PURE_AUDIO_EXTENSIONS.
// Audio files are transcribable but not indexable directly — they go through
// the Transcribe flow, not the Add-to-Index flow.
export const PURE_AUDIO_EXTENSIONS: ReadonlySet<string> = new Set([
  "wav",
  "mp3",
  "m4a",
  "flac",
  "ogg",
  "aac",
  "wma",
]);

const TRANSCRIBABLE_VIDEO_EXTENSIONS: readonly string[] = [
  "mp4",
  "mov",
  "avi",
  "mkv",
  "webm",
];

export const TRANSCRIBABLE_EXTENSIONS: ReadonlySet<string> = new Set([
  ...PURE_AUDIO_EXTENSIONS,
  ...TRANSCRIBABLE_VIDEO_EXTENSIONS,
]);

export function extensionOf(filename: string): string {
  return filename.split(".").pop()?.toLowerCase() ?? "";
}

export function isPureAudio(filename: string): boolean {
  return PURE_AUDIO_EXTENSIONS.has(extensionOf(filename));
}

export function isTranscribable(filename: string): boolean {
  return TRANSCRIBABLE_EXTENSIONS.has(extensionOf(filename));
}
