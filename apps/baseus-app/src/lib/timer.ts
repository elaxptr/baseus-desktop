import { createSignal } from 'solid-js';

const [elapsed, setElapsed] = createSignal(0); // seconds since session start
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startTimer() {
  setElapsed(0);
  if (intervalId !== null) clearInterval(intervalId);
  intervalId = setInterval(() => setElapsed((s) => s + 1), 1000);
}

export function stopTimer() {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
  setElapsed(0);
}

// Return the signal accessor directly so callers can use it reactively in JSX:
//   elapsed={useElapsed()()}  — reactive; re-reads when timer ticks
export const useElapsed = () => elapsed;

export function formatElapsed(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}
