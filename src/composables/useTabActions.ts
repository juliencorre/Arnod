/**
 * Actions sur les onglets de contenu partagées par les zones : fermer (en
 * enregistrant), diviser, déplacer, détacher dans une fenêtre (F1 à F8).
 */
import { contentKey, useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ContentTab, Side, WindowBounds } from "@/types/ipc";

/** Taille d'une fenêtre ouverte à l'endroit d'un dépôt (F8). */
const DROPPED_WINDOW = { width: 980, height: 720 };

export function useTabActions() {
  const windows = useWindowsStore();
  const workbench = useWorkbenchStore();

  function tab(id: string): ContentTab | null {
    for (const zone of windows.zones) {
      const found = zone.tabs.find((t) => t.id === id);
      if (found) return found;
    }
    return null;
  }

  /** Enregistre la requête de l'onglet si elle a changé : rien n'est perdu. */
  async function saveTab(id: string): Promise<void> {
    const found = tab(id);
    if (found?.content.kind === "request") await workbench.save(contentKey(found.content));
  }

  /** Ferme un onglet après avoir enregistré sa requête (F1). */
  async function close(id: string): Promise<void> {
    const found = tab(id);
    await saveTab(id);
    if (await windows.close(id)) {
      if (found?.content.kind === "request") workbench.release(contentKey(found.content));
    }
  }

  async function split(zone: string, id: string, side: Side): Promise<void> {
    await windows.split(zone, id, side);
  }

  /** L'onglet seul devient une fenêtre, à l'endroit d'un dépôt s'il est connu (F8). */
  async function detach(id: string, at: { x: number; y: number } | null = null): Promise<void> {
    await saveTab(id);
    const bounds: WindowBounds | null = at
      ? { x: Math.round(at.x), y: Math.round(at.y), ...DROPPED_WINDOW }
      : null;
    await windows.detachTab(id, bounds);
  }

  /** La zone devient une fenêtre (F8). */
  async function detachZone(zone: string): Promise<void> {
    const found = windows.zones.find((z) => z.id === zone);
    for (const t of found?.tabs ?? []) await saveTab(t.id);
    await windows.detachZone(zone, null);
  }

  return { tab, close, split, detach, detachZone, saveTab };
}
