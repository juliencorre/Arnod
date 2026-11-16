/**
 * Ajout d'un contrat (C1 à C4) : importer un fichier, coller, lire à une adresse ou créer un
 * contrat vide. Partagé par la section « Contrats de service » de la barre latérale et par les
 * points de départ d'un projet vide (docs/plan-mvp-d-barre-laterale.md, D2) ; le composant
 * affiche `ContractDialog` avec `mode`, `error` et `busy`.
 */
import { ref, type Ref } from "vue";

import { type ContractDialogMode, useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import type { ContractSource } from "@/types/ipc";

export interface ContractAdd {
  mode: Ref<ContractDialogMode | null>;
  error: Ref<string | null>;
  busy: Ref<boolean>;
  /** Choix du menu d'ajout (`CONTRACT_ADD_ITEMS`). */
  choose: (id: string) => void;
  submit: (source: ContractSource) => Promise<void>;
  close: () => void;
}

export function useContractAdd(projectId: () => string): ContractAdd {
  const contracts = useContractsStore();
  const toasts = useToastStore();
  const mode = ref<ContractDialogMode | null>(null);
  const error = ref<string | null>(null);
  const busy = ref(false);

  function choose(id: string): void {
    if (id === "file") {
      void contracts.importFile(projectId());
      return;
    }
    error.value = null;
    mode.value = id as ContractDialogMode;
  }

  async function submit(source: ContractSource): Promise<void> {
    busy.value = true;
    error.value = null;
    try {
      const view = await contracts.add(projectId(), source);
      mode.value = null;
      toasts.show(`Contrat « ${view.name} » ajouté.`);
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    } finally {
      busy.value = false;
    }
  }

  function close(): void {
    mode.value = null;
  }

  return { mode, error, busy, choose, submit, close };
}
