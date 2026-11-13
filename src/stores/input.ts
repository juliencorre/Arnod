import { defineStore } from "pinia";
import { reactive, ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  CurlCopy,
  DynamicValue,
  Pair,
  PairsFormat,
  PairsText,
  RequestInput,
} from "@/types/ipc";

/**
 * Saisie d'une requête (docs/plan-mvp-b-saisie-curl.md) : variables connues et inconnues,
 * en-têtes ajoutés à l'envoi, listes éditées en texte, copie en cURL. Le cœur calcule tout ; ce
 * store relaie et garde la dernière réponse par requête ouverte.
 */
export const useInputStore = defineStore("input", () => {
  const workbench = useWorkbenchStore();
  /** Dernière lecture de `request_input`, par clé de requête ouverte. */
  const inputs = reactive(new Map<string, RequestInput>());
  /** Dernière demande par clé : une réponse plus ancienne est ignorée. */
  const asked = new Map<string, number>();
  let counter = 0;

  /** Relit les variables et les en-têtes ajoutés de la requête ouverte `key`. */
  async function inspect(key: string): Promise<void> {
    const editor = workbench.editors[key];
    const environment = editor ? workbench.activeEnvironment(editor.projectId) : null;
    if (!editor?.draft || environment === null) return;
    const ticket = ++counter;
    asked.set(key, ticket);
    try {
      const input = await ipc.requestInput(
        editor.projectId,
        environment,
        editor.path,
        editor.draft,
      );
      if (asked.get(key) === ticket) inputs.set(key, input);
    } catch {
      // Environnement disparu, projet fermé : rien n'est souligné.
      if (asked.get(key) === ticket) inputs.delete(key);
    }
  }

  function forget(key: string): void {
    asked.delete(key);
    inputs.delete(key);
  }

  /** La requête ouverte `key` en commande cURL, dans son environnement actif (R8). */
  async function curl(key: string): Promise<CurlCopy> {
    const editor = workbench.editors[key];
    const environment = editor ? workbench.activeEnvironment(editor.projectId) : null;
    if (!editor?.draft || environment === null) {
      throw new Error("La requête n'est pas encore chargée.");
    }
    return ipc.requestCurl(editor.projectId, environment, editor.path, editor.draft);
  }

  /**
   * Ajoute `name` aux variables partagées de l'environnement actif du projet, avec `value`
   * (celle d'un autre environnement, ou vide).
   */
  async function addVariable(projectId: string, name: string, value: string): Promise<string> {
    const view = workbench.environments[projectId];
    const env = view?.environments.find((e) => e.name === view.active);
    if (!env) throw new Error("Aucun environnement actif.");
    await workbench.saveEnvironment(
      projectId,
      {
        name: env.name,
        color: env.color,
        protected: env.protected,
        variables: [...env.variables, { name, value }],
        locals: env.locals,
      },
      env.name,
    );
    return env.name;
  }

  function toText(pairs: Pair[], format: PairsFormat): Promise<string> {
    return ipc.pairsToText(pairs, format);
  }

  function fromText(text: string, format: PairsFormat, previous: Pair[]): Promise<PairsText> {
    return ipc.pairsFromText(text, format, previous);
  }

  /** Valeurs dynamiques proposées à la frappe de `{{$` (5.1-h), lues une fois. */
  const dynamicValues = ref<DynamicValue[]>([]);
  let dynamicLoad: Promise<void> | null = null;

  /** Lit la liste des valeurs dynamiques au premier besoin ; un échec sera retenté. */
  function loadDynamicValues(): Promise<void> {
    dynamicLoad ??= ipc
      .dynamicValues()
      .then((list) => {
        dynamicValues.value = list;
      })
      .catch(() => {
        dynamicLoad = null;
      });
    return dynamicLoad;
  }

  return {
    inputs,
    inspect,
    forget,
    curl,
    addVariable,
    toText,
    fromText,
    dynamicValues,
    loadDynamicValues,
  };
});
