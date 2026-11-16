import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TransferDone, TransferMode, TransferTarget } from "@/types/ipc";

/** Requête à copier ou déplacer (dialogue 4.1-g, presse-papiers de la barre latérale). */
export interface TransferSource {
  projectId: string;
  path: string;
  name: string;
}

/** Requête copiée (Ctrl+C) ou coupée (Ctrl+X) dans la barre latérale. */
export interface Clipboard extends TransferSource {
  mode: TransferMode;
}

/** Dernier transfert, pour Ctrl+Z. */
interface LastTransfer {
  token: string;
  projects: string[];
}

function parentOf(path: string): string {
  const index = path.lastIndexOf("/");
  return index < 0 ? "" : path.slice(0, index);
}

/**
 * Copier ou déplacer une requête (lot J, écrans 4.1-f et 4.1-g) : le cœur décide de tout
 * (destinations, « Ce qui change », écriture et annulation) ; ce store garde le dialogue ouvert,
 * le presse-papiers de la barre latérale (Ctrl+C, Ctrl+X, Ctrl+V) et le dernier transfert à
 * annuler (Ctrl+Z, « Annuler » de la notification). Propre à la fenêtre.
 */
export const useTransferStore = defineStore("transfer", () => {
  const dialog = ref<TransferSource | null>(null);
  const clipboard = ref<Clipboard | null>(null);
  const last = ref<LastTransfer | null>(null);

  function openDialog(source: TransferSource): void {
    dialog.value = source;
  }

  function closeDialog(): void {
    dialog.value = null;
  }

  function copy(source: TransferSource, mode: TransferMode): void {
    clipboard.value = { ...source, mode };
    useToastStore().show(
      mode === "copy"
        ? `« ${source.name} » copiée : Ctrl+V sur un dossier ou un test pour la coller.`
        : `« ${source.name} » coupée : Ctrl+V sur un dossier ou un test pour la déplacer.`,
    );
  }

  /** Relit les projets touchés : arborescence, contrats et requêtes ouvertes du dossier. */
  async function refresh(projects: string[], folder?: { projectId: string; path: string }) {
    const workbench = useWorkbenchStore();
    const contracts = useContractsStore();
    await Promise.all(
      [...new Set(projects)].filter((id) => id in workbench.trees).map((id) => contracts.sync(id)),
    );
    if (folder) await workbench.reloadIn(folder.projectId, folder.path);
  }

  /** Copie ou déplace `source` vers `destination` du projet `target`, avec « Annuler ». */
  async function apply(
    source: TransferSource,
    target: string,
    destination: TransferTarget,
    mode: TransferMode,
  ): Promise<TransferDone> {
    const workbench = useWorkbenchStore();
    // Les requêtes ouvertes sont enregistrées : rien ne réécrit un rang d'étape renuméroté.
    await workbench.saveAll();
    const done = await ipc.transferApply(source.projectId, source.path, target, destination, mode);
    if (mode === "move")
      workbench.followTransfer(
        source.projectId,
        source.path,
        target === source.projectId ? done.path : null,
      );
    const projects = [source.projectId, target];
    last.value = { token: done.undoToken, projects };
    if (clipboard.value?.mode === "move" && clipboard.value.path === source.path)
      clipboard.value = null;
    await refresh(projects, { projectId: target, path: parentOf(done.path) });
    useToastStore().show(
      mode === "copy" ? `« ${done.name} » copiée.` : `« ${done.name} » déplacée.`,
      {
        action: {
          label: "Annuler",
          run: () => void undo(),
        },
      },
    );
    return done;
  }

  /**
   * Colle le presse-papiers dans le dossier ou le test `folder` du projet `projectId` : à la
   * place `position` d'un test (à la fin par défaut). `mode` remplace celui du presse-papiers
   * (Ctrl+glisser copie).
   */
  async function paste(
    projectId: string,
    folder: string,
    test: boolean,
    mode?: TransferMode,
    position?: number,
  ): Promise<void> {
    const source = clipboard.value;
    if (!source) return;
    const destination: TransferTarget = test
      ? { kind: "step", test: folder, position: position ?? Number.MAX_SAFE_INTEGER }
      : { kind: "folder", folder };
    await apply(source, projectId, destination, mode ?? source.mode);
  }

  /** Annule le dernier transfert (Ctrl+Z) ; `false` s'il n'y en a pas. */
  async function undo(): Promise<boolean> {
    const done = last.value;
    if (!done) return false;
    last.value = null;
    const workbench = useWorkbenchStore();
    await workbench.saveAll();
    await ipc.transferUndo(done.token);
    await refresh(done.projects);
    for (const projectId of done.projects) {
      for (const [key, editor] of Object.entries(workbench.editors)) {
        if (editor.projectId === projectId) await workbench.reload(key);
      }
    }
    useToastStore().show("Copie ou déplacement annulé.");
    return true;
  }

  return { dialog, clipboard, last, openDialog, closeDialog, copy, apply, paste, undo };
});
