/**
 * Actions sur les onglets de contenu partagées par les zones : fermer (en
 * enregistrant), diviser, déplacer, détacher dans une fenêtre (F1 à F8) ; fermer les
 * autres ou tout (lot F, 1.3-b).
 */
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { contentKey, useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ContentTab, Side, WindowBounds } from "@/types/ipc";

/** Taille d'une fenêtre ouverte à l'endroit d'un dépôt (F8). */
const DROPPED_WINDOW = { width: 980, height: 720 };

export function useTabActions() {
  const windows = useWindowsStore();
  const workbench = useWorkbenchStore();
  const contracts = useContractsStore();
  const toasts = useToastStore();

  function tab(id: string): ContentTab | null {
    for (const zone of windows.zones) {
      const found = zone.tabs.find((t) => t.id === id);
      if (found) return found;
    }
    return null;
  }

  /**
   * Enregistre la requête ou la source du contrat de l'onglet si elle a changé :
   * rien n'est perdu. `false` si une source de contrat illisible n'a pas pu l'être (R10).
   */
  async function saveTab(id: string): Promise<boolean> {
    const found = tab(id);
    if (found?.content.kind === "request") await workbench.save(contentKey(found.content));
    if (found?.content.kind === "contract") {
      const key = contentKey(found.content);
      if (!(await contracts.save(key))) {
        toasts.show(
          `Source du contrat non enregistrée : ${contracts.open[key]?.saveError ?? "illisible"}`,
          { tone: "error" },
        );
        return false;
      }
    }
    return true;
  }

  /** Ferme un onglet après avoir enregistré sa requête ou son contrat (F1). */
  async function close(id: string): Promise<void> {
    const found = tab(id);
    if (!(await saveTab(id))) return;
    if (await windows.close(id)) {
      if (found?.content.kind === "request") workbench.release(contentKey(found.content));
      if (found?.content.kind === "contract") contracts.release(contentKey(found.content));
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

  /**
   * Ferme plusieurs onglets (« Fermer les autres », « Fermer tout », 1.3-b) après avoir
   * enregistré leurs requêtes et contrats ; un contrat illisible garde tout ouvert.
   */
  async function closeMany(ids: string[]): Promise<void> {
    const found = ids.map((id) => tab(id)).filter((t): t is ContentTab => t !== null);
    for (const t of found) if (!(await saveTab(t.id))) return;
    if (found.length === 0 || !(await windows.closeTabs(found.map((t) => t.id)))) return;
    for (const t of found) {
      if (t.content.kind === "request") workbench.release(contentKey(t.content));
      if (t.content.kind === "contract") contracts.release(contentKey(t.content));
    }
  }

  return { tab, close, closeMany, split, detach, detachZone, saveTab };
}
