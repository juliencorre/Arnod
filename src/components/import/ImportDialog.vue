<script setup lang="ts">
/**
 * Fenêtre « Importer » (2.3-a, 2.3-b, docs/plan-mvp-c-import-bruno.md) : un dossier ou un
 * fichier choisi, son format reconnu par le cœur (collection Bruno, projet OpenCollection,
 * contrat OpenAPI), la destination (nouveau projet, ou un dossier du projet ouvert) et ce qui
 * sera repris. Rien n'est écrit avant « Importer » ; le rapport s'ouvre ensuite.
 *
 * AM41 (2.3-b) : chaque environnement de la collection va dans un environnement du projet (ou
 * un nouveau), avec le nombre de variables qu'il y ajoute ; les valeurs qui ressemblent à des
 * secrets vont en variables locales, au coffre (case cochée par défaut). Le cœur applique ;
 * l'interface ne fait que compter.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { IpcError } from "@/lib/ipc";
import { envSwatch } from "@/lib/environments";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useImportStore } from "@/stores/imports";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ImportSource, ImportTarget, SourceEnvironment } from "@/types/ipc";

const imports = useImportStore();
const catalog = useCatalogStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const { announceCreated } = useProjectActions();
const ids = { destination: useId(), folder: useId(), environments: useId() };

const source = ref<ImportSource | null>(null);
const destination = ref<"newProject" | "project">("newProject");
const location = ref<string | null>(null);
const folder = ref("");
const error = ref<string | null>(null);
const busy = ref(false);
/** Environnement du projet qui reçoit chaque environnement de la collection (AM41). */
const targets = ref<Record<string, string>>({});
/** Valeurs qui ressemblent à des secrets en variables locales (Q5 : oui par défaut). */
const secretsLocal = ref(true);

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
    secretsLocal.value = true;
    // Les environnements du projet, pour la correspondance (AM41).
    if (current?.projectId && !workbench.environments[current.projectId]) {
      workbench.refreshEnvironments(current.projectId).catch(() => undefined);
    }
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

/** Environnements du projet de destination ; aucun pour un nouveau projet. */
const projectEnvironments = computed(() => {
  const id = state.value?.projectId;
  return id && !toNewProject.value ? (workbench.environments[id]?.environments ?? []) : [];
});

function existing(name: string) {
  const key = name.toLocaleLowerCase();
  return projectEnvironments.value.find((env) => env.name.toLocaleLowerCase() === key);
}

/** Par défaut, l'environnement de même nom du projet, sinon un nouvel environnement. */
watch(
  [() => source.value?.environments, projectEnvironments],
  ([environments]) => {
    targets.value = Object.fromEntries(
      (environments ?? []).map((env) => [env.name, existing(env.name)?.name ?? env.name]),
    );
  },
  { immediate: true },
);

function targetOf(env: SourceEnvironment): string {
  return targets.value[env.name] ?? env.name;
}

/** Variables que l'environnement de la collection ajoute à sa cible (affichage). */
function addedCount(env: SourceEnvironment): number {
  const target = existing(targetOf(env));
  if (!target) return env.variables.length;
  const known = new Set([...target.variables.map((v) => v.name), ...target.locals]);
  return env.variables.filter((name) => !known.has(name)).length;
}

/** « token, password, apiKey : elles iront en variables locales… » (2.3-b). */
const secretsHint = computed(() => {
  const names = source.value?.secrets ?? [];
  const one = names.length === 1;
  const fate = secretsLocal.value
    ? one
      ? "elle ira en variable locale, au coffre du système, jamais partagée."
      : "elles iront en variables locales, au coffre du système, jamais partagées."
    : one
      ? "elle restera partagée, dans les fichiers du projet."
      : "elles resteront partagées, dans les fichiers du projet.";
  return `${names.join(", ")} : ${fate}`;
});

function swatchOf(env: SourceEnvironment): string {
  return envSwatch(existing(targetOf(env))?.color);
}

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
  // Lu avant la fermeture, qui vide l'état de la fenêtre.
  const ws = workspace.value;
  if (!s || !current || busy.value) return;
  const target: ImportTarget =
    toNewProject.value || current.projectId === null
      ? { kind: "newProject", location: location.value, workspaceId: current.workspaceId }
      : { kind: "project", projectId: current.projectId };
  busy.value = true;
  error.value = null;
  try {
    const done = await imports.run(s.path, target, {
      environments: s.environments.map((env) => ({
        from: env.name,
        to: toNewProject.value ? env.name : targetOf(env),
      })),
      secretsLocal: secretsLocal.value,
    });
    imports.close();
    if (s.format === "openCollection") {
      // Projet ajouté tel quel (P12) : ouvert et annoncé, sans rapport.
      navigation.showProject(done.projectId, current.workspaceId);
      const where = ws ? `« ${ALL_PROJECTS} » et à « ${ws.name} »` : `« ${ALL_PROJECTS} »`;
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
          <dt :id="ids.environments">Environnements</dt>
          <dd>
            <span v-if="source.environments.length === 0">
              Aucun : le projet aura l'environnement « Local ».
            </span>
            <ul v-else class="import__envs" :aria-labelledby="ids.environments">
              <li v-for="env in source.environments" :key="env.name" class="import__env">
                <code class="import__env-from">{{ env.name }}</code>
                <span class="import__env-arrow" aria-hidden="true">→</span>
                <span v-if="toNewProject" class="import__env-new">nouvel environnement</span>
                <span v-else class="import__env-target">
                  <span
                    class="import__swatch"
                    :style="{ backgroundColor: swatchOf(env) }"
                    aria-hidden="true"
                  />
                  <select
                    v-model="targets[env.name]"
                    class="import__env-select"
                    :aria-label="`Environnement du projet qui reçoit ${env.name}`"
                  >
                    <option
                      v-for="target in projectEnvironments"
                      :key="target.name"
                      :value="target.name"
                    >
                      {{ target.name }}
                    </option>
                    <option v-if="!existing(env.name)" :value="env.name">
                      Nouvel environnement « {{ env.name }} »
                    </option>
                  </select>
                </span>
                <span class="import__hint">{{
                  countLabel(addedCount(env), "variable ajoutée", "variables ajoutées")
                }}</span>
              </li>
            </ul>
          </dd>
          <template v-if="source.secrets.length > 0 || source.counts.secrets > 0">
            <dt>Secrets</dt>
            <dd>
              <label v-if="source.secrets.length > 0" class="import__check">
                <input v-model="secretsLocal" type="checkbox" />
                <span>
                  <span>{{
                    source.secrets.length === 1
                      ? "1 valeur ressemble à un secret"
                      : `${String(source.secrets.length)} valeurs ressemblent à des secrets`
                  }}</span>
                  <span class="import__hint">{{ secretsHint }}</span>
                </span>
              </label>
              <span v-if="source.counts.secrets > 0">
                {{ countLabel(source.counts.secrets, "variable secrète", "variables secrètes") }}
                de Bruno
                <span class="import__hint">
                  Variables locales, au coffre du système, jamais dans le projet.
                </span>
              </span>
            </dd>
          </template>
          <dt>Scripts</dt>
          <dd>
            {{
              countLabel(
                source.counts.assertions,
                "vérification deviendra une assertion",
                "vérifications deviendront des assertions",
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

.import__envs {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.import__env {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.import__env-from {
  min-width: 5rem;
  font: var(--text-body) var(--font-mono);
}

.import__env-arrow {
  color: var(--color-muted);
}

.import__env-target {
  display: inline-flex;
  gap: 0.375rem;
  align-items: center;
}

.import__swatch {
  flex: none;
  width: 0.625rem;
  height: 0.625rem;
  border-radius: var(--radius-sm);
}

.import__check {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  cursor: pointer;
}

.import__check > span {
  display: flex;
  flex-direction: column;
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
