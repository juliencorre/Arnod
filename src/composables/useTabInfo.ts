/**
 * Libellé, méthode (badge) et état « modifié » d'un onglet de contenu, pour la barre des onglets,
 * Ctrl+Tab et la bulle de taille d'une séparation (1.4-b).
 */
import { useCatalogStore } from "@/stores/catalog";
import { contractKey, useContractsStore } from "@/stores/contracts";
import { useImportStore } from "@/stores/imports";
import { findRequest, requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { ContentTab } from "@/types/ipc";

export interface TabInfo {
  name: string;
  badge: string;
  dirty: boolean;
}

export function useTabInfo(): (tab: ContentTab) => TabInfo {
  const workbench = useWorkbenchStore();
  const catalog = useCatalogStore();
  const contracts = useContractsStore();
  const imports = useImportStore();

  /** Libellé, méthode et état d'un onglet. */
  return function info(tab: ContentTab): { name: string; badge: string; dirty: boolean } {
    const content = tab.content;
    if (content.kind === "result") return { name: "Résultat", badge: "RES", dirty: false };
    if (content.kind === "run") {
      return {
        name: `Run · ${catalog.projectById(content.projectId)?.name ?? "projet"}`,
        badge: "RUN",
        dirty: false,
      };
    }
    if (content.kind === "environments") {
      return { name: "Environnements", badge: "ENV", dirty: false };
    }
    if (content.kind === "importReport") {
      const report = imports.reports[content.projectId];
      return {
        name: report ? `Rapport d'import · ${report.collection}` : "Rapport d'import",
        badge: "⇩",
        dirty: false,
      };
    }
    if (content.kind === "contract") {
      const key = contractKey(content.projectId, content.path);
      const summary = contracts.lists[content.projectId]?.find((c) => c.path === content.path);
      return {
        name:
          contracts.open[key]?.view?.name ??
          summary?.name ??
          content.path.split("/").at(-1) ??
          content.path,
        badge: "API",
        dirty: contracts.isDirty(key),
      };
    }
    if (content.kind === "test") {
      return {
        name: content.path.split("/").at(-1) ?? content.path,
        badge: "TEST",
        dirty: false,
      };
    }
    if (content.kind === "folder") {
      return { name: content.path.split("/").at(-1) ?? content.path, badge: "DOS", dirty: false };
    }
    const key = requestKey(content.projectId, content.path);
    const draft = workbench.editors[key]?.draft;
    const node = findRequest(workbench.trees[content.projectId]?.nodes ?? [], content.path);
    const fallback =
      content.path
        .split("/")
        .at(-1)
        ?.replace(/\.yml$/, "") ?? content.path;
    return {
      name: draft?.name ?? node?.name ?? fallback,
      badge: draft?.method ?? (node?.kind === "request" ? node.method : null) ?? "GET",
      dirty: workbench.isDirty(key),
    };
  };
}
