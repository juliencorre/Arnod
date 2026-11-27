/**
 * Templates (docs/plan-v4.md, lot F) : templates ouverts, utilisations, création depuis un
 * dossier ou un endpoint, section « Templates » et « Plus » de la barre latérale. Le cœur décide
 * (paramètres détectés, valeurs effectives, R16 à R19) ; ce store garde ce qui est affiché.
 */
import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  InstanceView,
  Literal,
  TemplateCandidate,
  TemplateParam,
  TemplateSummary,
  TemplateView,
} from "@/types/ipc";

/** Sections que « Plus » propose dans cette version (R21) et leur raison d'être (1.2-d bis). */
export const MORE_SECTIONS: { id: string; title: string; hint: string }[] = [
  { id: "templates", title: "Templates", hint: "Des modèles de requêtes et de tests" },
];

/** Dossier réservé aux templates, à la racine du projet (R20). */
export const TEMPLATES_DIR = "templates";

/** Le chemin est sous `templates/`. */
export function isTemplatePath(path: string): boolean {
  return path === TEMPLATES_DIR || path.startsWith(`${TEMPLATES_DIR}/`);
}

export const useTemplatesStore = defineStore("templates", () => {
  const workbench = useWorkbenchStore();
  /** Templates ouverts, par projet puis chemin. */
  const views = ref<Record<string, TemplateView>>({});
  /** Utilisations ouvertes, par projet puis chemin. */
  const instances = ref<Record<string, InstanceView>>({});

  const key = (projectId: string, path: string): string => `${projectId}:${path}`;

  /** Templates du projet, tels que l'arborescence les donne. */
  function list(projectId: string): TemplateSummary[] {
    return workbench.trees[projectId]?.templates ?? [];
  }

  /** La section « Templates » est affichée : un template existe, ou « Afficher » (R21). */
  function sectionShown(projectId: string, section: string): boolean {
    const tree = workbench.trees[projectId];
    if (!tree) return false;
    if (section === "templates" && tree.templates.length > 0) return true;
    return tree.sections.includes(section);
  }

  /** « Afficher » ou masquer une section depuis « Plus ». */
  async function showSection(projectId: string, section: string, shown: boolean): Promise<void> {
    const sections = await ipc.sectionsShow(projectId, section, shown);
    const tree = workbench.trees[projectId];
    if (tree) tree.sections = sections;
  }

  async function read(projectId: string, path: string): Promise<TemplateView> {
    const view = await ipc.templateRead(projectId, path);
    views.value[key(projectId, path)] = view;
    return view;
  }

  function view(projectId: string, path: string): TemplateView | null {
    return views.value[key(projectId, path)] ?? null;
  }

  async function saveParams(
    projectId: string,
    path: string,
    params: TemplateParam[],
  ): Promise<TemplateView> {
    const saved = await ipc.templateSaveParams(projectId, path, params);
    views.value[key(projectId, path)] = saved;
    await workbench.refreshTree(projectId);
    return saved;
  }

  function candidates(projectId: string, folder: string): Promise<TemplateCandidate[]> {
    const environment = workbench.activeEnvironment(projectId) ?? "";
    return ipc.templateCandidates(projectId, folder, environment);
  }

  function categories(projectId: string): Promise<string[]> {
    return ipc.templateCategories(projectId);
  }

  /** Crée un template depuis un dossier (4.8-b) et l'ouvre. */
  async function create(
    projectId: string,
    folder: string,
    name: string,
    category: string,
    params: TemplateParam[],
    replace: boolean,
  ): Promise<TemplateView> {
    const created = await ipc.templateCreate(projectId, folder, name, category, params, replace);
    views.value[key(projectId, created.path)] = created;
    await workbench.refreshTree(projectId);
    await workbench.openFolder(projectId, created.path);
    return created;
  }

  /** « Utiliser dans un dossier… » (4.8-d) : l'utilisation créée s'ouvre. */
  async function use(
    projectId: string,
    template: string,
    folder: string,
    name: string,
  ): Promise<string> {
    const path = await ipc.templateUse(projectId, template, folder, name);
    await workbench.refreshTree(projectId);
    await workbench.openFolder(projectId, path);
    return path;
  }

  async function instance(projectId: string, path: string): Promise<InstanceView> {
    const found = await ipc.templateInstance(projectId, path);
    instances.value[key(projectId, path)] = found;
    return found;
  }

  function instanceView(projectId: string, path: string): InstanceView | null {
    return instances.value[key(projectId, path)] ?? null;
  }

  async function setValues(
    projectId: string,
    path: string,
    values: Record<string, string>,
  ): Promise<InstanceView> {
    const saved = await ipc.templateSetValues(projectId, path, values);
    instances.value[key(projectId, path)] = saved;
    return saved;
  }

  function literals(projectId: string, path: string): Promise<Literal[]> {
    return ipc.templateLiterals(projectId, path);
  }

  /** « Transférer vers les templates » (3.2-d) : le template s'ouvre. */
  async function fromRequest(
    projectId: string,
    path: string,
    name: string,
    category: string,
    params: string[],
  ): Promise<TemplateView> {
    const created = await ipc.templateFromRequest(projectId, path, name, category, params);
    views.value[key(projectId, created.path)] = created;
    await workbench.refreshTree(projectId);
    await workbench.openFolder(projectId, created.path);
    return created;
  }

  return {
    views,
    instances,
    list,
    sectionShown,
    showSection,
    read,
    view,
    saveParams,
    candidates,
    categories,
    create,
    use,
    instance,
    instanceView,
    setValues,
    literals,
    fromRequest,
  };
});
