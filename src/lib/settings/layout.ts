// Order and visibility of the Home sections and sidebar items (spec 6.10),
// from what the preferences saved.
import type { LayoutItem } from "#lib/api.ts";

/**
 * The `defaults` in the saved order, each with its saved visibility. Items
 * the save does not know (new in this version) go to their default place,
 * shown; saved IDs this version does not know are dropped.
 */
export function arrange<T extends { id: string }>(defaults: readonly T[], saved: LayoutItem[]): (T & { visible: boolean })[] {
  const byId = new Map(defaults.map((item) => [item.id, item]));
  const result: (T & { visible: boolean })[] = [];
  for (const { id, visible } of saved) {
    const item = byId.get(id);
    if (item && !result.some((r) => r.id === id)) result.push({ ...item, visible });
  }
  defaults.forEach((item, index) => {
    if (!result.some((r) => r.id === item.id)) result.splice(Math.min(index, result.length), 0, { ...item, visible: true });
  });
  return result;
}

/** At least `min` items visible: the first hidden ones are shown again. */
export function keepVisible<T extends { visible: boolean }>(items: T[], min: number): T[] {
  let missing = min - items.filter((item) => item.visible).length;
  return items.map((item) => {
    if (missing > 0 && !item.visible) {
      missing--;
      return { ...item, visible: true };
    }
    return item;
  });
}
