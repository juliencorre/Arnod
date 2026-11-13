/**
 * Corps de la réponse affichée (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis), par onglet de
 * requête : format choisi, mise en forme, aperçu HTML, filtre JSONPath et ses correspondances.
 * Le cœur met en forme, filtre et tire les vérifications ; ce store garde l'état d'affichage et
 * relaie les actions (copier, enregistrer, effacer, ajouter une vérification).
 */
import { defineStore } from "pinia";
import { reactive } from "vue";

import { ipc } from "@/lib/ipc";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  Assertion,
  FilterMatch,
  FilterResult,
  HtmlNode,
  RenderedBody,
  ResponseFormat,
} from "@/types/ipc";

/** Attente après la dernière frappe avant d'évaluer le filtre. */
export const FILTER_DEBOUNCE_MS = 150;

/** État d'affichage du corps d'une réponse, pour un onglet de requête. */
export interface BodyView {
  /** Format choisi ; `null` : celui détecté par le cœur (R1). */
  format: ResponseFormat | null;
  rendered: RenderedBody | null;
  /** Aperçu rendu (HTML seulement). */
  preview: boolean;
  nodes: HtmlNode[] | null;
  /** Expression JSONPath saisie. */
  expression: string;
  result: FilterResult | null;
  /** Correspondance courante, surlignée plus fort et cible de « + Vérification ». */
  current: number;
}

function emptyView(): BodyView {
  return {
    format: null,
    rendered: null,
    preview: false,
    nodes: null,
    expression: "",
    result: null,
    current: 0,
  };
}

export const useResponseStore = defineStore("response", () => {
  const views = reactive<Record<string, BodyView>>({});
  const workbench = useWorkbenchStore();
  const toasts = useToastStore();
  /** Dernier appel par onglet : une réponse plus ancienne est ignorée. */
  const renderTurn = new Map<string, number>();
  const filterTurn = new Map<string, number>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function view(key: string): BodyView {
    const existing = views[key];
    if (existing) return existing;
    views[key] = emptyView();
    return views[key];
  }

  function response(key: string) {
    return workbench.editors[key]?.outcome?.response ?? null;
  }

  /** Met en forme le corps reçu, dans le format choisi ou détecté (R1 à R3). */
  async function render(key: string): Promise<void> {
    const current = view(key);
    const received = response(key);
    const turn = (renderTurn.get(key) ?? 0) + 1;
    renderTurn.set(key, turn);
    if (!received) {
      current.rendered = null;
      current.nodes = null;
      return;
    }
    const rendered = await ipc.responseRender(received.body, received.headers, current.format);
    if (renderTurn.get(key) !== turn) return;
    current.rendered = rendered;
    current.nodes = null;
    if (current.preview) await loadPreview(key);
  }

  async function setFormat(key: string, format: ResponseFormat): Promise<void> {
    const current = view(key);
    current.format = format;
    if (format !== "html") current.preview = false;
    await render(key);
  }

  async function loadPreview(key: string): Promise<void> {
    const current = view(key);
    const received = response(key);
    if (!received || current.nodes) return;
    current.nodes = await ipc.responsePreview(received.body);
  }

  /** Aperçu rendu d'un corps HTML (R4). */
  async function setPreview(key: string, on: boolean): Promise<void> {
    const current = view(key);
    current.preview = on;
    if (on) await loadPreview(key);
  }

  /** Évalue le filtre maintenant (R5, R7). */
  async function runFilter(key: string): Promise<void> {
    const current = view(key);
    const received = response(key);
    const turn = (filterTurn.get(key) ?? 0) + 1;
    filterTurn.set(key, turn);
    if (!received || current.expression.trim() === "") {
      current.result = null;
      current.current = 0;
      return;
    }
    const result = await ipc.responseFilter(received.body, current.expression);
    if (filterTurn.get(key) !== turn) return;
    current.result = result;
    current.current = 0;
  }

  /** Change l'expression ; le filtre est évalué après une courte pause. */
  function setExpression(key: string, expression: string): void {
    view(key).expression = expression;
    const pending = timers.get(key);
    if (pending !== undefined) clearTimeout(pending);
    timers.set(
      key,
      setTimeout(() => {
        timers.delete(key);
        void runFilter(key);
      }, FILTER_DEBOUNCE_MS),
    );
  }

  /** Passe à la correspondance suivante (`delta` 1) ou précédente (-1), en boucle. */
  function move(key: string, delta: number): void {
    const current = view(key);
    const count = current.result?.matches.length ?? 0;
    if (count === 0) return;
    current.current = (current.current + delta + count) % count;
  }

  function currentMatch(key: string): FilterMatch | null {
    const current = views[key];
    return current?.result?.matches[current.current] ?? null;
  }

  /** Copie le corps reçu, tel quel (R10 : un corps tronqué est signalé). */
  async function copy(key: string): Promise<void> {
    const received = response(key);
    if (!received) return;
    try {
      await navigator.clipboard.writeText(received.body);
      toasts.show(received.truncated ? "Corps copié (tronqué à 5 Mo)." : "Corps copié.");
    } catch {
      toasts.show("Copie impossible : le presse-papiers est indisponible.", { tone: "error" });
    }
  }

  /** Enregistre le corps reçu dans un fichier choisi dans le sélecteur natif (R8, R10). */
  async function save(key: string): Promise<void> {
    const received = response(key);
    const editor = workbench.editors[key];
    if (!received || !editor) return;
    const format = view(key).rendered?.format ?? "text";
    try {
      const path = await ipc.responseSave(
        received.body,
        editor.draft?.name ?? "",
        format,
        received.truncated,
      );
      if (path === null) return;
      toasts.show(
        received.truncated
          ? `Corps enregistré dans ${path} (tronqué à 5 Mo).`
          : `Corps enregistré dans ${path}.`,
      );
    } catch (error) {
      toasts.showError(error);
    }
  }

  /** Efface la réponse affichée, dans toutes les fenêtres (R9). */
  async function clear(key: string): Promise<void> {
    const editor = workbench.editors[key];
    if (!editor) return;
    try {
      workbench.applyResult(await ipc.responseClear(editor.projectId, editor.path));
      const current = view(key);
      current.rendered = null;
      current.nodes = null;
      current.result = null;
      current.current = 0;
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Ajoute au brouillon la vérification tirée d'une correspondance (R6) ; renvoie faux si une
   * vérification de même cible et même opérateur existe déjà.
   */
  function addAssertion(key: string, found: FilterMatch): boolean {
    const draft = workbench.editors[key]?.draft;
    const assertion: Assertion | null = found.assertion;
    if (!draft || !assertion) return false;
    const exists = draft.assertions.some(
      (a) => a.target === assertion.target && a.operator === assertion.operator,
    );
    if (exists) {
      toasts.show(`La vérification sur ${assertion.target} existe déjà.`);
      return false;
    }
    const taken = new Set(draft.assertions.map((a) => a.id));
    draft.assertions = [
      ...draft.assertions,
      taken.has(assertion.id)
        ? { ...assertion, id: `${assertion.id}-${crypto.randomUUID().slice(0, 8)}` }
        : { ...assertion },
    ];
    toasts.show(`Vérification ajoutée sur ${assertion.target}.`);
    return true;
  }

  /**
   * « Extraire dans une variable » (lot E, 4.2-b bis) : la valeur trouvée devient une extraction
   * de la requête, que les étapes suivantes du test lisent.
   */
  function addExtraction(key: string, found: FilterMatch): string | null {
    const target = found.assertion?.target;
    if (target === undefined) return null;
    const name = workbench.addExtraction(key, target);
    if (name !== null) toasts.show(`Variable « ${name} » extraite de ${target}.`);
    return name;
  }

  return {
    views,
    view,
    render,
    setFormat,
    setPreview,
    runFilter,
    setExpression,
    move,
    currentMatch,
    copy,
    save,
    clear,
    addAssertion,
    addExtraction,
  };
});
