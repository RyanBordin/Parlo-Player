// Discreet confirmations ("Playlist created"), shown above the Player Bar
// for a few seconds.
const DURATION_MS = 3500;

export const toasts = $state<{ id: number; text: string }[]>([]);
let nextId = 0;

export function showToast(text: string): void {
  const id = nextId++;
  toasts.push({ id, text });
  setTimeout(() => {
    const index = toasts.findIndex((t) => t.id === id);
    if (index !== -1) toasts.splice(index, 1);
  }, DURATION_MS);
}
