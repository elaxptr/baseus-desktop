import { createSignal } from 'solid-js';

const [message, setMessage] = createSignal<string | null>(null);
let timer: number | undefined;

export const toastMessage = message;

/** Show a short error at the bottom of the window. */
export function showError(e: unknown) {
  setMessage(String(e));
  clearTimeout(timer);
  timer = window.setTimeout(() => setMessage(null), 4000);
}
