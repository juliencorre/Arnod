/**
 * Statuts de la dernière exécution, pour la barre latérale (`docs/plan-fin-premier-usage.md`,
 * ST1, ST2, R6). Le cœur garde le statut de chaque requête ; ce module ne fait que des
 * comptages d'affichage.
 */
import { formatDayTime } from "@/lib/format";
import type { RequestStatus, RequestStatuses, TreeNode } from "@/types/ipc";

/** Requêtes d'un dossier ou d'une ressource : réussies, exécutées, total. */
export interface StatusCount {
  passed: number;
  executed: number;
  total: number;
}

/** Compte les statuts des requêtes `paths` (R6). */
export function countStatuses(paths: string[], statuses: RequestStatuses | undefined): StatusCount {
  let passed = 0;
  let executed = 0;
  for (const path of paths) {
    const status = statuses?.[path];
    if (!status) continue;
    executed += 1;
    if (status.passed) passed += 1;
  }
  return { passed, executed, total: paths.length };
}

/** Chemins des requêtes d'un nœud de l'arborescence, sous-dossiers compris. */
export function requestPaths(node: TreeNode): string[] {
  if (node.kind === "request") return [node.path];
  return node.children.flatMap(requestPaths);
}

/** « Réussie le 9 nov., 14:05 » ou « En échec le … ». */
export function statusTitle(status: RequestStatus): string {
  return `${status.passed ? "Réussie" : "En échec"} le ${formatDayTime(status.at)}`;
}

/** « 15 réussies sur 16 exécutées (18 requêtes) ». */
export function countTitle(count: StatusCount): string {
  const plural = (n: number, word: string): string => `${String(n)} ${word}${n > 1 ? "s" : ""}`;
  return `${plural(count.passed, "réussie")} sur ${plural(count.executed, "exécutée")} (${plural(count.total, "requête")})`;
}
