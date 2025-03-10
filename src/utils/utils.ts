export function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  seconds %= 60;
  seconds = Math.floor(seconds);
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}
