import { defineStore } from "pinia";
import { reactive, ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  Auth,
  ClearSecret,
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
      if (asked.get(key) === ticket) {
        inputs.set(key, input);
        // Lot K (4.2-c) : un secret en clair suspend l'enregistrement jusqu'au choix.
        workbench.holdSecret(key, pendingSecrets(key).length > 0);
      }
    } catch {
      // Environnement disparu, projet fermé : rien n'est souligné.
      if (asked.get(key) === ticket) inputs.delete(key);
    }
  }

  /**
   * Oublie l'analyse de la requête `key` (éditeur démonté). Le choix « Laisser dans le test »
   * reste pour la session : il vaut pour cette valeur, quel que soit l'onglet qui la montre (IN4).
   */
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

  // --- Lot K : secret saisi en clair dans l'authentification (4.2-c) ---------------

  /**
   * Valeurs que l'utilisateur a choisi de laisser dans la requête, par clé et champ ; gardées
   * pour la session, même quand l'onglet se ferme ou change (IN4).
   */
  const kept = reactive(new Map<string, Set<string>>());

  function authOf(key: string): Auth | null {
    return workbench.editors[key]?.draft?.auth ?? null;
  }

  /** Valeur du champ `field` de l'authentification. */
  function fieldValue(auth: Auth, field: string): string {
    if (field === "oauth2.clientSecret") return auth.oauth2.clientSecret;
    if (field === "token" || field === "password" || field === "value") return auth[field];
    return "";
  }

  /** Secrets en clair de la requête `key` qui attendent un choix. */
  function pendingSecrets(key: string): ClearSecret[] {
    const auth = authOf(key);
    const secrets = inputs.get(key)?.secrets ?? [];
    if (!auth) return [];
    return secrets.filter((s) => !kept.get(key)?.has(`${s.field}\n${fieldValue(auth, s.field)}`));
  }

  /** « Laisser dans le test » : la valeur est écrite telle quelle. */
  function keepSecrets(key: string): void {
    const auth = authOf(key);
    if (!auth) return;
    const set = kept.get(key) ?? new Set<string>();
    for (const s of inputs.get(key)?.secrets ?? [])
      set.add(`${s.field}\n${fieldValue(auth, s.field)}`);
    kept.set(key, set);
    workbench.holdSecret(key, false);
  }

  /**
   * « Mettre dans une variable locale » : la valeur va au coffre du système, dans une variable
   * locale secrète de l'environnement actif, et le champ devient `{{nom}}`.
   */
  async function secretToLocal(key: string, secret: ClearSecret): Promise<string> {
    const editor = workbench.editors[key];
    const auth = authOf(key);
    const view = editor ? workbench.environments[editor.projectId] : undefined;
    const env = view?.environments.find((e) => e.name === view.active);
    if (!editor || !auth || !env) throw new Error("Aucun environnement actif.");
    const value = fieldValue(auth, secret.field);
    const name = secret.suggested;
    if (!env.locals.includes(name)) {
      await workbench.saveEnvironment(
        editor.projectId,
        {
          name: env.name,
          color: env.color,
          protected: env.protected,
          variables: env.variables,
          locals: [...env.locals, name],
        },
        env.name,
      );
    }
    await workbench.setLocal(editor.projectId, env.name, name, value, true);
    const reference = `{{${name}}}`;
    if (secret.field === "oauth2.clientSecret") auth.oauth2.clientSecret = reference;
    else if (secret.field === "token" || secret.field === "password" || secret.field === "value")
      auth[secret.field] = reference;
    workbench.holdSecret(key, false);
    return env.name;
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
    // Lot K : secret en clair.
    pendingSecrets,
    keepSecrets,
    secretToLocal,
  };
});
