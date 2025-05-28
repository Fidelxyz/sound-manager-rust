import { extname } from "@tauri-apps/api/path";

export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export async function isAudioFile(path: string): Promise<boolean> {
  const audioExtensions = ["wav", "mp3", "flac", "ogg"];
  const fileExt = await extname(path);
  return audioExtensions.includes(fileExt.toLowerCase());
}
