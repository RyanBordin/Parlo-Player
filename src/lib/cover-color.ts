// The color of a cover for the phone Now Playing background (spec 4.5): its
// predominant color, darkened and muted so white text stays readable on it.
import { coverUrl } from "#lib/covers.ts";

/** Side of the small copy that is read, in pixels. */
const SAMPLE = 24;

/** Colors found, by cover ID: a cover's color does not change. */
const found = new Map<string, string | null>();

/** The cover's background color as `rgb(...)`, or null (no image, unreadable). */
export async function coverColor(coverId: string): Promise<string | null> {
  if (found.has(coverId)) return found.get(coverId) ?? null;
  let color: string | null = null;
  try {
    color = predominant(await load(coverUrl(coverId, SAMPLE / (window.devicePixelRatio || 1))));
  } catch {
    // Offline or not an image: no color, the page's background shows.
  }
  found.set(coverId, color);
  return color;
}

function load(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    // Read back by the canvas: the cover protocol allows it (CORS).
    image.crossOrigin = "anonymous";
    image.onload = () => resolve(image);
    image.onerror = reject;
    image.src = src;
  });
}

/**
 * The most common color among the cover's colorful pixels (by 4-bit-per-
 * channel buckets, each bucket averaged); near black, near white and grays
 * count little. Then darkened and muted for the background.
 */
function predominant(image: HTMLImageElement): string | null {
  const canvas = document.createElement("canvas");
  canvas.width = SAMPLE;
  canvas.height = SAMPLE;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return null;
  context.drawImage(image, 0, 0, SAMPLE, SAMPLE);
  const data = context.getImageData(0, 0, SAMPLE, SAMPLE).data;

  const buckets = new Map<number, { weight: number; r: number; g: number; b: number }>();
  for (let i = 0; i < data.length; i += 4) {
    const [r, g, b] = [data[i], data[i + 1], data[i + 2]];
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    // Grays and the extremes say little about the cover's color.
    const saturation = max === 0 ? 0 : (max - min) / max;
    const weight = 0.1 + saturation * (max > 24 && min < 235 ? 1 : 0.2);
    const key = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
    const bucket = buckets.get(key) ?? { weight: 0, r: 0, g: 0, b: 0 };
    bucket.weight += weight;
    bucket.r += r * weight;
    bucket.g += g * weight;
    bucket.b += b * weight;
    buckets.set(key, bucket);
  }
  let best: { weight: number; r: number; g: number; b: number } | null = null;
  for (const bucket of buckets.values()) if (!best || bucket.weight > best.weight) best = bucket;
  if (!best) return null;
  return background(best.r / best.weight, best.g / best.weight, best.b / best.weight);
}

/** Darkened and muted (HSL lightness 22%, saturation at most 45%). */
function background(r: number, g: number, b: number): string {
  const [h, s] = hsl(r, g, b);
  return `hsl(${Math.round(h)} ${Math.round(Math.min(s, 0.45) * 100)}% 22%)`;
}

function hsl(r: number, g: number, b: number): [number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0];
  const s = d / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === r) h = ((g - b) / d) % 6;
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  return [(h * 60 + 360) % 360, s];
}
