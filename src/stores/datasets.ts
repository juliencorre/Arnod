/**
 * Jeux de données (docs/plan-v4.md, lot D, 4.7-a à 4.7-b) : tableau de chaque requête ouverte,
 * cas choisi, requête du cas, résultat de chaque cas. Le cœur lit et écrit le CSV, résout et
 * envoie (R11, R12) ; ce store garde l'état d'affichage.
 */
import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import type { CasePreview, CasesOutcome, DataSet, DataTable, RequestDoc } from "@/types/ipc";

export const useDatasetsStore = defineStore("datasets", () => {
  /** Jeu de données de chaque requête ouverte, par clé d'éditeur (`null` : aucun). */
  const sets = ref<Record<string, DataSet | null>>({});
  /** Dernier envoi de tous les cas, par clé d'éditeur. */
  const outcomes = ref<Record<string, CasesOutcome | null>>({});
  /** Cas choisi (à partir de 1), par clé d'éditeur. */
  const selected = ref<Record<string, number>>({});
  /** Cas ignorés pour le résultat affiché (4.7-b « Ignorer ce cas pour ce résultat »). */
  const ignored = ref<Record<string, number[]>>({});
  /** Requête du cas choisi, résolue (4.7-a). */
  const previews = ref<Record<string, CasePreview | null>>({});

  async function load(key: string, projectId: string, path: string): Promise<DataSet | null> {
    const set = await ipc.dataRead(projectId, path);
    sets.value[key] = set;
    return set;
  }

  async function save(
    key: string,
    projectId: string,
    path: string,
    table: DataTable,
  ): Promise<DataSet> {
    const set = await ipc.dataSave(projectId, path, table);
    sets.value[key] = set;
    return set;
  }

  async function importFile(key: string, projectId: string, path: string): Promise<boolean> {
    const set = await ipc.dataImport(projectId, path);
    if (set === null) return false;
    sets.value[key] = set;
    return true;
  }

  async function remove(key: string, projectId: string, path: string): Promise<void> {
    await ipc.dataRemove(projectId, path);
    sets.value[key] = null;
    outcomes.value[key] = null;
  }

  function parse(text: string): Promise<string[][]> {
    return ipc.dataParse(text);
  }

  function open(projectId: string, path: string): Promise<void> {
    return ipc.dataOpen(projectId, path);
  }

  async function preview(
    key: string,
    projectId: string,
    path: string,
    request: RequestDoc,
    environment: string,
  ): Promise<void> {
    const number = selected.value[key] ?? 1;
    previews.value[key] = await ipc.dataCase(projectId, path, request, environment, number);
  }

  function select(key: string, number: number): void {
    selected.value[key] = number;
  }

  function setOutcome(key: string, outcome: CasesOutcome | null): void {
    outcomes.value[key] = outcome;
    ignored.value[key] = [];
  }

  function ignore(key: string, number: number): void {
    ignored.value[key] = [...(ignored.value[key] ?? []), number];
  }

  /** Cas en échec qui comptent : ni réussis, ni ignorés (comptage d'affichage). */
  function failures(key: string): number[] {
    const outcome = outcomes.value[key];
    const skip = ignored.value[key] ?? [];
    return (outcome?.cases ?? [])
      .filter((c) => c.outcome?.passed !== true && !skip.includes(c.number))
      .map((c) => c.number);
  }

  return {
    sets,
    outcomes,
    selected,
    ignored,
    previews,
    load,
    save,
    importFile,
    remove,
    parse,
    open,
    preview,
    select,
    setOutcome,
    ignore,
    failures,
  };
});
