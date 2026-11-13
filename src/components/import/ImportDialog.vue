<script setup lang="ts">
/**
 * Fenêtre « Importer » (2.3-a, 2.3-b, docs/plan-mvp-c-import-bruno.md) : un dossier ou un
 * fichier choisi, son format reconnu par le cœur (collection Bruno, projet OpenCollection,
 * contrat OpenAPI), la destination (nouveau projet, ou un dossier du projet ouvert) et ce qui
 * sera repris. Rien n'est écrit avant « Importer » ; le rapport s'ouvre ensuite.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { IpcError } from "@/lib/ipc";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useImportStore } from "@/stores/imports";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ImportSource, ImportTarget } from "@/types/ipc";

const imports = useImportStore();
const catalog = useCatalogStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const { announceCreated } = useProjectActions();
const ids = { destination: useId(), folder: useId() };

const source = ref<ImportSource | null>(null);
const destination = ref<"newProject" | "project">("newProject");
const location = ref<string | null>(null);
const folder = ref("");
const error = ref<string | null>(null);
const busy = ref(false);

const state = computed(() => imports.dialog);
const project = computed(() =>
  state.value?.projectId ? catalog.projectById(state.value.projectId) : null,
);
const workspace = computed(() =>
  state.value?.workspaceId ? catalog.workspaceById(state.value.workspaceId) : null,
);

watch(
  state,
  (current) => {
    source.value = current?.source ?? null;
    destination.value = current?.projectId ? "project" : "newProject";
    location.value = null;
    error.value = null;
  },
  { immediate: true },
);

/** Un projet OpenCollection est ajouté tel quel : pas de choix de destination. */
const canChoose = computed(
  () => project.value !== null && source.value !== null && source.value.format !== "openCollection",
);
const toNewProject = computed(
  () => destination.value === "newProject" || source.value?.format === "openCollection",
);

watch(
  [() => source.value?.name, location, toNewProject],
  async ([name]) => {
    if (!name || !toNewProject.value || source.value?.format === "openCollection") {
      folder.value = source.value?.path ?? "";
      return;
    }
    try {
      folder.value = await catalog.suggestFolder(name, location.value);
    } catch {
      folder.value = "";
    }
  },
  { immediate: true },
);

const badge = computed(() => {
  switch (source.value?.format) {
    case "bruno":
      return "BRU";
    case "openApi":
      return "API";
    default:
      return "OC";
  }
});

const summary = computed(() => {
  const s = source.value;
  if (!s) return "";
  const c = s.counts;
  switch (s.format) {
    case "bruno":
      return [
        "✓ Collection Bruno",
        `${countLabel(c.requests, "requête", "requêtes")} dans ${countLabel(c.folders, "dossier", "dossiers")}`,
        countLabel(c.environments, "environnement", "environnements"),
      ].join(" · ");
    case "openApi":
      return `✓ Contrat OpenAPI · ${countLabel(c.requests, "opération", "opérations")}`;
    default:
      return "✓ Projet Arnod ou OpenCollection : ajouté tel quel";
  }
});

const sourceName = computed(() => {
  const s = source.value;
  if (!s) return "";
  return s.path.split(/[\\/]/).filter(Boolean).at(-1) ?? s.path;
});

async function pick(file: boolean): Promise<void> {
  error.value = null;
  try {
    const picked = await imports.pick(file);
    if (picked) source.value = picked;
  } catch (cause) {
    error.value = cause instanceof IpcError ? cause.message : String(cause);
  }
}

async function changeLocation(): Promise<void> {
  try {
    const picked = await catalog.pickFolder(location.value);
    if (picked !== null) location.value = picked;
  } catch (cause) {
    toasts.showError(cause);
  }
}

function close(): void {
  if (!busy.value) imports.close();
}

async function submit(): Promise<void> {
  const s = source.value;
  const current = state.value;
  if (!s || !current || busy.value) return;
  const target: ImportTarget =
    toNewProject.value || current.projectId === null
      ? { kind: "newProject", location: location.value, workspaceId: current.workspaceId }
      : { kind: "project", projectId: current.projectId };
  busy.value = true;
  error.value = null;
  try {
    const done = await imports.run(s.path, target);
    imports.close();
    if (s.format === "openCollection") {
      // Projet ajouté tel quel (P12) : ouvert et annoncé, sans rapport.
      navigation.showProject(done.projectId, current.workspaceId);
      const where = workspace.value
        ? `« ${ALL_PROJECTS} » et à « ${workspace.value.name} »`
        : `« ${ALL_PROJECTS} »`;
      toasts.show(`« ${catalog.projectById(done.projectId)?.name ?? s.name} » ajouté à ${where}.`);
    } else if (target.kind === "newProject") {
      announceCreated(done.projectId, current.workspaceId);
    } else {
      navigation.showProject(done.projectId, navigation.currentWorkspaceId);
      await Promise.all([
        workbench.refreshTree(done.projectId),
        workbench.refreshEnvironments(done.projectId),
        contracts.sync(done.projectId),
      ]);
    }
    if (done.report) await imports.openReport(done.projectId);
    else if (done.contract) await contracts.show(done.projectId, done.contract);
  } catch (cause) {
    error.value = cause instanceof IpcError ? cause.message : String(cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog :open="state !== null" title="Importer" @close="close">
    <div class="import">
      <div v-if="source" class="import__source">
        <span class="import__badge" aria-hidden="true">{{ badge }}</span>
        <span class="import__source-text">
          <strong class="import__source-name">{{ sourceName }}</strong>
          <span class="import__summary">{{ summary }}</span>
        </span>
        <span class="import__change">
          <BaseButton variant="link" @click="pick(false)">Changer…</BaseButton>
        </span>
      </div>
      <div v-else class="import__pick">
        <p class="import__text">
          Une collection Bruno, un fichier OpenAPI ou un projet Arnod : Arnod reconnaît le format.
        </p>
        <div class="import__pick-actions">
          <BaseButton @click="pick(false)">Choisir un dossier…</BaseButton>
          <BaseButton @click="pick(true)">Choisir un fichier…</BaseButton>
        </div>
      </div>

      <dl v-if="source" class="import__rows">
        <dt :id="ids.destination">Destination</dt>
        <dd>
          <div
            v-if="canChoose"
            role="radiogroup"
            :aria-labelledby="ids.destination"
            class="import__choices"
          >
            <label class="import__choice">
              <input v-model="destination" type="radio" value="project" />
              <span>
                <span class="import__choice-title">Dans {{ project?.name }}</span>
                <span class="import__hint">
                  {{
                    source.format === "openApi"
                      ? "Un contrat de service de plus."
                      : `Un nouveau dossier « ${source.name} » dans Référentiel de tests.`
                  }}
                </span>
              </span>
            </label>
            <label class="import__choice">
              <input v-model="destination" type="radio" value="newProject" />
              <span>
                <span class="import__choice-title">Nouveau projet</span>
                <span class="import__hint"
                  >« {{ source.name }} », dans « {{ ALL_PROJECTS }} ».</span
                >
              </span>
            </label>
          </div>
          <span v-else-if="source.format === 'openCollection'">
            Ajouté à « {{ ALL_PROJECTS }} »{{ workspace ? ` et à « ${workspace.name} »` : "" }},
            sans conversion.
          </span>
          <span v-else>
            Nouveau projet « {{ source.name }} », dans « {{ ALL_PROJECTS }} »{{
              workspace ? ` et « ${workspace.name} »` : ""
            }}.
          </span>
        </dd>

        <template v-if="toNewProject && source.format !== 'openCollection'">
          <dt :id="ids.folder">Dossier</dt>
          <dd class="import__folder">
            <code :aria-labelledby="ids.folder">{{ folder }}</code>
            <BaseButton variant="link" @click="changeLocation">Changer</BaseButton>
          </dd>
        </template>

        <template v-if="source.format === 'bruno'">
          <dt>Environnements</dt>
          <dd>
            {{
              source.counts.environments === 0
                ? "Aucun : le projet aura l'environnement « Local »."
                : `${countLabel(source.counts.environments, "environnement repris", "environnements repris")}${toNewProject ? "" : " ; un environnement de même nom est complété, sans rien remplacer"}.`
            }}
          </dd>
          <template v-if="source.counts.secrets > 0">
            <dt>Secrets</dt>
            <dd>
              {{ countLabel(source.counts.secrets, "variable secrète", "variables secrètes") }}
              <span class="import__hint">
                Elles iront en variables locales, au coffre du système, jamais dans le projet.
              </span>
            </dd>
          </template>
          <dt>Scripts</dt>
          <dd>
            {{
              countLabel(
                source.counts.assertions,
                "vérification deviendra",
                "vérifications deviendront",
              )
            }}
            sans code, {{ countLabel(source.counts.extractions, "extraction", "extractions") }} ;
            {{
              source.counts.scripts === 0
                ? "aucun script."
                : `${countLabel(source.counts.scripts, "script gardé", "scripts gardés")} dans les fichiers, pas exécutés.`
            }}
          </dd>
        </template>
      </dl>

      <p v-if="error" class="import__error" role="alert">{{ error }}</p>
    </div>

    <template #footer>
      <p class="import__note">Rien n'est modifié avant « Importer ». Un rapport s'ouvre ensuite.</p>
      <BaseButton :disabled="busy" @click="close">Annuler</BaseButton>
      <BaseButton variant="primary" :disabled="source === null || busy" @click="submit">
        Importer
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.import {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.import__source {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.75rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-bg);
}

.import__badge {
  display: grid;
  flex: none;
  place-items: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: var(--radius-md);
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
  background: var(--color-chip);
}

.import__source-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.import__source-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.import__summary {
  font-size: var(--text-body);
  color: var(--color-success);
}

.import__change {
  flex: none;
}

.import__pick {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
}

.import__pick-actions {
  display: flex;
  gap: 0.5rem;
}

.import__text {
  margin: 0;
  color: var(--color-muted);
}

.import__rows {
  display: grid;
  grid-template-columns: 8rem minmax(0, 1fr);
  gap: 0.875rem 1rem;
  margin: 0;
}

.import__rows dt {
  color: var(--color-muted);
}

.import__rows dd {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  margin: 0;
}

.import__choices {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
}

.import__choice {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  cursor: pointer;
}

.import__choice > span {
  display: flex;
  flex-direction: column;
}

.import__choice-title {
  font-weight: 600;
}

.import__hint {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.import__rows .import__folder {
  flex-direction: row;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.75rem;
}

.import__folder code {
  font: var(--text-body) var(--font-mono);
  overflow-wrap: anywhere;
}

.import__error {
  margin: 0;
  color: var(--color-danger);
}

.import__note {
  flex: 1;
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

@media (width < 40rem) {
  .import__rows {
    grid-template-columns: minmax(0, 1fr);
    gap: 0.25rem;
  }
}
</style>
