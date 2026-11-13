import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import type { ImportDone, ImportReport, ImportSource, ImportTarget } from "@/types/ipc";

/** Fenêtre « Importer » : projet ouvert (destination proposée) et workspace courant. */
export interface ImportDialogState {
  /** Projet où proposer l'import (« Dans <projet> »), ou aucun : nouveau projet. */
  projectId: string | null;
  /** Workspace du nouveau projet. */
  workspaceId: string | null;
  /** Source déjà choisie et reconnue. */
  source: ImportSource | null;
}

/**
 * Import (docs/plan-mvp-c-import-bruno.md) : choix d'un dossier ou d'un fichier, format reconnu
 * par le cœur, import, puis rapport (2.3-f). Le cœur fait tout ; le store garde la fenêtre
 * ouverte et les rapports lus.
 */
export const useImportStore = defineStore("imports", () => {
  const catalog = useCatalogStore();
  const windows = useWindowsStore();
  const toasts = useToastStore();
  const dialog = ref<ImportDialogState | null>(null);
  /** Dernier rapport d'import par projet (`null` : aucun). */
  const reports = ref<Record<string, ImportReport | null>>({});

  /** Ouvre la fenêtre « Importer », vide ou avec une source reconnue. */
  function open(
    projectId: string | null,
    workspaceId: string | null,
    source: ImportSource | null = null,
  ): void {
    dialog.value = { projectId, workspaceId, source };
  }

  function close(): void {
    dialog.value = null;
  }

  /** Sélecteur natif (dossier, ou fichier avec `file`), puis reconnaissance ; `null` si annulé. */
  async function pick(file: boolean): Promise<ImportSource | null> {
    const path = await ipc.importPick(file);
    return path === null ? null : ipc.importDetect(path);
  }

  /**
   * Choisit un dossier (ou un fichier) puis ouvre « Importer » avec le format reconnu ; un
   * format inconnu est notifié.
   */
  async function choose(
    projectId: string | null,
    workspaceId: string | null,
    file = false,
  ): Promise<void> {
    try {
      const source = await pick(file);
      if (source) open(projectId, workspaceId, source);
    } catch (error) {
      toasts.showError(error);
    }
  }

  /** Importe ; le catalogue et le rapport sont mis à jour. */
  async function run(path: string, target: ImportTarget): Promise<ImportDone> {
    const done = await ipc.importRun(path, target);
    if (done.snapshot) catalog.apply(done.snapshot);
    if (done.report) reports.value[done.projectId] = done.report;
    return done;
  }

  /** Lit le dernier rapport d'un projet. */
  async function load(projectId: string): Promise<ImportReport | null> {
    const report = await ipc.importReport(projectId);
    reports.value[projectId] = report;
    return report;
  }

  /** Ouvre l'onglet du rapport d'import d'un projet. */
  async function openReport(projectId: string): Promise<void> {
    await windows.openSelection({ kind: "importReport", projectId });
  }

  return { dialog, reports, open, close, pick, choose, run, load, openReport };
});
