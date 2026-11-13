/**
 * Onglets d'une zone, pour l'affichage (lot F, docs/plan-mvp-f-onglets-clavier.md) : onglets
 * qui tiennent dans la barre (1.3-b), ordre d'utilisation (1.3-c), exécution en cours (◐, 1.3-a).
 * Fonctions pures : l'ordre et l'état viennent du cœur, l'interface ne fait que les montrer.
 */
import type { ContentRef, RunView } from "@/types/ipc";

/**
 * Indices des onglets visibles quand la barre mesure `available` pixels : les premiers qui
 * tiennent, la place du bouton « +N » (`more`) réservée dès qu'un onglet déborde. L'onglet
 * actif reste toujours visible : il prend la place des derniers onglets visibles.
 */
export function fitTabs(
  widths: readonly number[],
  available: number,
  more: number,
  active: number,
): number[] {
  const all = widths.map((_, index) => index);
  const total = widths.reduce((sum, width) => sum + width, 0);
  if (total <= available) return all;
  const budget = Math.max(0, available - more);
  const visible: number[] = [];
  let used = 0;
  for (const index of all) {
    const width = widths[index] ?? 0;
    if (used + width > budget) break;
    visible.push(index);
    used += width;
  }
  if (active >= 0 && active < widths.length && !visible.includes(active)) {
    const activeWidth = widths[active] ?? 0;
    while (visible.length > 0 && used + activeWidth > budget) {
      used -= widths[visible.pop() ?? 0] ?? 0;
    }
    visible.push(active);
  }
  return visible;
}

/**
 * Onglets d'une zone dans l'ordre d'utilisation (`recent`, tenu par le cœur), puis ceux qu'il
 * ne cite pas encore, dans l'ordre de la barre.
 */
export function byUse<T extends { id: string }>(
  tabs: readonly T[],
  recent: readonly string[],
): T[] {
  const ordered = recent.flatMap((id) => tabs.filter((tab) => tab.id === id));
  return [...ordered, ...tabs.filter((tab) => !recent.includes(tab.id))];
}

/** Ce que l'interface sait des exécutions en cours, pour ◐. */
export interface Activity {
  /** Envoi en cours de cette requête (`projet::chemin`). */
  sending: (projectId: string, path: string) => boolean;
  /** Run du projet, ou `null`. */
  run: (projectId: string) => RunView | null | undefined;
}

/**
 * Le contenu s'exécute-t-il (◐, 1.3-a) ? Une requête en cours d'envoi ou jouée par un run en
 * cours, un test dont une étape est jouée, le run d'un projet tant qu'il n'est pas fini.
 */
export function isRunning(content: ContentRef, activity: Activity): boolean {
  if (content.kind === "result" || content.kind === "contract" || content.kind === "importReport")
    return false;
  const run = activity.run(content.projectId);
  const live = run && !run.finished ? run : null;
  switch (content.kind) {
    case "run":
      return live !== null;
    case "test":
      return (
        live?.items.some(
          (item) =>
            item.test === content.path && (item.status === "running" || item.status === "pending"),
        ) ?? false
      );
    case "request":
      return (
        activity.sending(content.projectId, content.path) ||
        (live?.items.some((item) => item.path === content.path && item.status === "running") ??
          false)
      );
  }
}
