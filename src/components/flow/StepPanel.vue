<script setup lang="ts">
/**
 * Étape choisie (4.3-c, 4.3-g, 4.3-b quater) : un formulaire par nature. Une requête montre ses
 * entrées (« ← étape · source »), ses extractions et ses assertions, et s'ouvre dans l'éditeur ;
 * une condition, une boucle, une attente, une variable, un script, un test existant ou un template
 * se règlent ici. « Si cette étape échoue » et la suite (« puis → ») valent pour toute étape. Les
 * modifications vont au brouillon du schéma, qui s'enregistre seul.
 */
import { computed, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { OPERATORS } from "@/lib/assertions";
import { KIND_LABELS, following, stepName, stepRequest } from "@/lib/flow";
import { useFlowsStore } from "@/stores/flows";
import { useTemplatesStore } from "@/stores/templates";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { Flow, FlowCheck, FlowStep, OnFailure, Scalar, TestView, TreeNode } from "@/types/ipc";

const props = defineProps<{
  projectId: string;
  path: string;
  view: TestView;
  flow: Flow;
  step: FlowStep;
}>();

const flows = useFlowsStore();
const workbench = useWorkbenchStore();
const templates = useTemplatesStore();
const toasts = useToastStore();

const index = computed(() => props.flow.steps.findIndex((s) => s.id === props.step.id));
const request = computed(() => stepRequest(props.step, props.view.steps));
const info = computed(() => props.view.info.find((i) => i.id === props.step.id) ?? null);
const justInserted = computed(
  () => flows.inserted[flows.key(props.projectId, props.path)] === props.step.id,
);

/** Remplace l'étape dans le brouillon. */
function update(change: (step: FlowStep) => void): void {
  const copy = JSON.parse(JSON.stringify(props.step)) as FlowStep;
  change(copy);
  flows.setStep(props.projectId, props.path, copy);
}

/** Cibles possibles d'un renvoi : les autres étapes, puis les fins. */
const targets = computed(() => [
  ...props.flow.steps
    .map((s, i) => ({ id: s.id, label: `${String(i + 1)} · ${stepName(s, props.view.steps)}` }))
    .filter((t) => t.id !== props.step.id),
  { id: "end", label: "Fin ✓" },
  { id: "fail", label: "Fin en échec" },
]);

const next = computed({
  get: () => props.step.next ?? following(props.flow, index.value),
  set: (value: string) => {
    update((s) => {
      s.next = value;
    });
  },
});

const otherwise = computed({
  get: () =>
    props.step.kind === "condition"
      ? (props.step.otherwise ?? following(props.flow, index.value))
      : "",
  set: (value: string) => {
    update((s) => {
      if (s.kind === "condition") s.otherwise = value;
    });
  },
});

const onFailure = computed({
  get: () => props.step.onFailure ?? "",
  set: (value: OnFailure | "") => {
    update((s) => {
      if (value === "") delete s.onFailure;
      else s.onFailure = value;
    });
  },
});

/** Valeur saisie d'une condition : un nombre, un booléen, sinon un texte. */
function scalarOf(text: string): Scalar {
  const trimmed = text.trim();
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed);
  if (trimmed === "true" || trimmed === "false") return trimmed === "true";
  return text;
}

function valueText(check: FlowCheck): string {
  return check.value === undefined || check.value === null ? "" : String(check.value);
}

function setCheck(change: (check: FlowCheck) => void): void {
  update((s) => {
    if (s.kind === "condition" || s.kind === "wait") change(s.check);
  });
}

/** Tests du référentiel, pour « Test existant ». */
const tests = computed(() => {
  const out: { path: string; name: string }[] = [];
  const walk = (nodes: TreeNode[]): void => {
    for (const node of nodes) {
      if (node.kind !== "folder") continue;
      if (node.test && node.path !== props.path) out.push({ path: node.path, name: node.path });
      walk(node.children);
    }
  };
  walk(workbench.trees[props.projectId]?.tests ?? []);
  return out;
});

const templateList = computed(() => templates.list(props.projectId));
const templateView = computed(() =>
  props.step.kind === "template" && props.step.template !== ""
    ? templates.view(props.projectId, props.step.template)
    : null,
);
watch(
  () => (props.step.kind === "template" ? props.step.template : ""),
  (template) => {
    if (template !== "" && !templates.view(props.projectId, template))
      void templates.read(props.projectId, template).catch(() => undefined);
  },
  { immediate: true },
);

/** Étapes après la boucle, pour « jusqu'à l'étape ». */
const after = computed(() =>
  props.flow.steps
    .map((s, i) => ({ id: s.id, label: `${String(i + 1)} · ${stepName(s, props.view.steps)}`, i }))
    .filter((s) => s.i > index.value),
);

function sourceName(id: string | null): string {
  if (id === null) return "environnement";
  const step = props.flow.steps.find((s) => s.id === id);
  return step ? stepName(step, props.view.steps) : id;
}

function operatorLabel(id: string): string {
  return OPERATORS.find((o) => o.id === id)?.label ?? id;
}

async function remove(): Promise<void> {
  try {
    await flows.remove(props.projectId, props.path, props.step.id);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

function openRequest(): void {
  if (request.value) void workbench.openRequest(props.projectId, request.value.path);
}
</script>

<template>
  <section class="panel" :aria-label="`Étape ${index + 1}`">
    <header class="panel__head">
      <span class="panel__kind">Étape · {{ KIND_LABELS[step.kind] }}</span>
      <span v-if="justInserted" class="panel__new">à configurer</span>
    </header>
    <label v-if="step.kind !== 'request'" class="panel__field">
      <span class="panel__label">Nom</span>
      <input
        class="panel__input"
        :value="step.name"
        @input="update((s) => (s.name = ($event.target as HTMLInputElement).value))"
      />
    </label>
    <h3 v-else class="panel__title">{{ stepName(step, view.steps) }}</h3>

    <!-- Requête (4.3-c) -->
    <template v-if="step.kind === 'request' && request">
      <p class="panel__url">
        <b>{{ request.method }}</b> <code>{{ request.url }}</code>
      </p>
      <div v-if="info?.inputs.length" class="panel__block">
        <p class="panel__label">Entrées</p>
        <p v-for="input in info.inputs" :key="input.name" class="panel__line">
          <code>{{ input.name }}</code> ← {{ sourceName(input.from) }} · {{ input.source }}
        </p>
      </div>
      <div class="panel__block">
        <p class="panel__label">Extraire pour les étapes suivantes</p>
        <p v-for="e in request.extracts" :key="e.name" class="panel__line">
          <code>{{ e.name }}</code> ← {{ e.source }}
        </p>
        <p v-if="request.extracts.length === 0" class="panel__muted">Aucune valeur extraite.</p>
      </div>
      <div class="panel__block">
        <p class="panel__label">Assertions · {{ request.assertions.length }}</p>
        <p v-for="a in request.assertions" :key="a.id" class="panel__line">
          <code>{{ a.target }}</code> {{ operatorLabel(a.operator) }}
          {{ a.value === undefined || a.value === null ? "" : String(a.value) }}
        </p>
      </div>
      <BaseButton variant="link" @click="openRequest">Ouvrir dans l'éditeur</BaseButton>
    </template>

    <!-- Condition (4.3-a « Si … aller à ») et attente -->
    <template v-if="step.kind === 'condition' || step.kind === 'wait'">
      <div class="panel__block">
        <p class="panel__label">{{ step.kind === "wait" ? "Jusqu'à ce que" : "Condition" }}</p>
        <div class="panel__row">
          <input
            class="panel__input"
            aria-label="Cible : status, body.chemin, headers.Nom ou vars.nom"
            :value="step.check.target"
            @input="setCheck((c) => (c.target = ($event.target as HTMLInputElement).value))"
          />
          <select
            class="panel__select"
            aria-label="Opérateur"
            :value="step.check.operator"
            @change="
              setCheck(
                (c) =>
                  (c.operator = ($event.target as HTMLSelectElement)
                    .value as FlowCheck['operator']),
              )
            "
          >
            <option v-for="o in OPERATORS" :key="o.id" :value="o.id">{{ o.label }}</option>
          </select>
          <input
            class="panel__input"
            aria-label="Valeur"
            :value="valueText(step.check)"
            @input="
              setCheck((c) => (c.value = scalarOf(($event.target as HTMLInputElement).value)))
            "
          />
        </div>
        <p class="panel__muted">
          Sur la réponse de la dernière requête (status, body.…, headers.…) ou une variable
          (vars.nom). Une condition ne fait jamais échouer le test.
        </p>
      </div>
    </template>
    <template v-if="step.kind === 'wait'">
      <div class="panel__row">
        <label class="panel__field">
          <span class="panel__label">Toutes les (s)</span>
          <input
            class="panel__input"
            type="number"
            min="0"
            max="60"
            step="0.5"
            :value="step.intervalMs / 1000"
            @input="
              update((s) => {
                if (s.kind === 'wait')
                  s.intervalMs = Math.round(
                    Number(($event.target as HTMLInputElement).value) * 1000,
                  );
              })
            "
          />
        </label>
        <label class="panel__field">
          <span class="panel__label">Essais au plus</span>
          <input
            class="panel__input"
            type="number"
            min="1"
            max="100"
            :value="step.attempts"
            @input="
              update((s) => {
                if (s.kind === 'wait')
                  s.attempts = Number(($event.target as HTMLInputElement).value);
              })
            "
          />
        </label>
      </div>
      <p v-if="request" class="panel__line">
        Renvoie <b>{{ request.method }}</b> <code>{{ request.url }}</code> à chaque essai.
        <BaseButton variant="link" @click="openRequest">Ouvrir la requête</BaseButton>
      </p>
    </template>

    <!-- Boucle -->
    <template v-if="step.kind === 'loop'">
      <fieldset class="panel__block panel__set">
        <legend class="panel__label">Répéter</legend>
        <label class="panel__radio">
          <input
            type="radio"
            :checked="step.list === undefined"
            @change="
              update((s) => {
                if (s.kind === 'loop') {
                  delete s.list;
                  s.count ??= 3;
                }
              })
            "
          />
          N fois
        </label>
        <label class="panel__radio">
          <input
            type="radio"
            :checked="step.list !== undefined"
            @change="
              update((s) => {
                if (s.kind === 'loop') {
                  s.list = s.list ?? '';
                  delete s.count;
                }
              })
            "
          />
          Pour chaque élément d'une liste
        </label>
        <input
          v-if="step.list === undefined"
          class="panel__input"
          type="number"
          min="1"
          max="1000"
          aria-label="Nombre de tours"
          :value="step.count ?? 1"
          @input="
            update((s) => {
              if (s.kind === 'loop') s.count = Number(($event.target as HTMLInputElement).value);
            })
          "
        />
        <input
          v-else
          class="panel__input"
          aria-label="Liste : {{nom}}, vars.nom ou body.chemin"
          placeholder="body.items ou {{ids}}"
          :value="step.list"
          @input="
            update((s) => {
              if (s.kind === 'loop') s.list = ($event.target as HTMLInputElement).value;
            })
          "
        />
      </fieldset>
      <label class="panel__field">
        <span class="panel__label">Élément rangé dans la variable</span>
        <input
          class="panel__input"
          :value="step.item"
          @input="
            update((s) => {
              if (s.kind === 'loop') s.item = ($event.target as HTMLInputElement).value;
            })
          "
        />
      </label>
      <label class="panel__field">
        <span class="panel__label">Répète les étapes jusqu'à</span>
        <select
          class="panel__select"
          :value="step.until ?? flow.steps[index + 1]?.id ?? ''"
          @change="
            update((s) => {
              if (s.kind === 'loop') s.until = ($event.target as HTMLSelectElement).value;
            })
          "
        >
          <option v-for="t in after" :key="t.id" :value="t.id">{{ t.label }}</option>
        </select>
      </label>
    </template>

    <!-- Variable -->
    <template v-if="step.kind === 'variable'">
      <div class="panel__row">
        <label class="panel__field">
          <span class="panel__label">Variable</span>
          <input
            class="panel__input"
            :value="step.variable"
            @input="
              update((s) => {
                if (s.kind === 'variable') s.variable = ($event.target as HTMLInputElement).value;
              })
            "
          />
        </label>
        <label class="panel__field panel__field--wide">
          <span class="panel__label">Valeur</span>
          <input
            class="panel__input"
            placeholder="{{prix}} ou {{$randomUUID}}"
            :value="step.value"
            @input="
              update((s) => {
                if (s.kind === 'variable') s.value = ($event.target as HTMLInputElement).value;
              })
            "
          />
        </label>
      </div>
    </template>

    <!-- Script (4.3-g) -->
    <template v-if="step.kind === 'script'">
      <label class="panel__field">
        <span class="panel__label">JavaScript</span>
        <textarea
          class="panel__code"
          spellcheck="false"
          rows="8"
          :value="step.code"
          @input="
            update((s) => {
              if (s.kind === 'script') s.code = ($event.target as HTMLTextAreaElement).value;
            })
          "
        />
      </label>
      <div class="panel__block">
        <p class="panel__label">À disposition</p>
        <p class="panel__line">
          <code>vars</code> — variables du test (<code>vars.set(nom, valeur)</code>)
        </p>
        <p class="panel__line"><code>response</code> — réponse de l'étape précédente</p>
        <p class="panel__line"><code>assert()</code> — échec explicite</p>
      </div>
      <p class="panel__muted">
        Exécution isolée : pas d'accès au disque ni au réseau, 2 s maximum. Pour un appel, utilisez
        une étape REST.
      </p>
    </template>

    <!-- Test existant -->
    <template v-if="step.kind === 'test'">
      <label class="panel__field">
        <span class="panel__label">Test du référentiel</span>
        <select
          class="panel__select"
          :value="step.test"
          @change="
            update((s) => {
              if (s.kind === 'test') s.test = ($event.target as HTMLSelectElement).value;
            })
          "
        >
          <option value="" disabled>Choisir un test…</option>
          <option v-for="t in tests" :key="t.path" :value="t.path">{{ t.name }}</option>
        </select>
      </label>
      <p class="panel__muted">
        Joué tel quel, avec les variables du test ; ce qu'il extrait passe à la suite.
      </p>
    </template>

    <!-- Template -->
    <template v-if="step.kind === 'template'">
      <label class="panel__field">
        <span class="panel__label">Template</span>
        <select
          class="panel__select"
          :value="step.template"
          @change="
            update((s) => {
              if (s.kind === 'template') {
                s.template = ($event.target as HTMLSelectElement).value;
                s.values = {};
              }
            })
          "
        >
          <option value="" disabled>Choisir un template…</option>
          <option v-for="t in templateList" :key="t.path" :value="t.path">{{ t.name }}</option>
        </select>
      </label>
      <p v-if="templateList.length === 0" class="panel__muted">
        Aucun template dans ce projet : créez-en un depuis la vue d'un dossier.
      </p>
      <div v-if="templateView" class="panel__block">
        <p class="panel__label">Valeurs des paramètres</p>
        <label v-for="param in templateView.parameters" :key="param.name" class="panel__field">
          <span class="panel__line"
            ><code>{{ param.name }}</code>
            {{ param.default === "" ? "· obligatoire" : `· défaut ${param.default}` }}</span
          >
          <input
            class="panel__input"
            :placeholder="param.default"
            :value="step.values[param.name] ?? ''"
            @input="
              update((s) => {
                if (s.kind === 'template') {
                  const value = ($event.target as HTMLInputElement).value;
                  if (value === '') delete s.values[param.name];
                  else s.values[param.name] = value;
                }
              })
            "
          />
        </label>
      </div>
    </template>

    <!-- Enchaînement -->
    <div class="panel__block">
      <p class="panel__label">Enchaînement</p>
      <template v-if="step.kind === 'condition'">
        <label class="panel__row">
          <span class="panel__hint">si oui →</span>
          <select v-model="next" class="panel__select">
            <option v-for="t in targets" :key="t.id" :value="t.id">{{ t.label }}</option>
          </select>
        </label>
        <label class="panel__row">
          <span class="panel__hint">sinon →</span>
          <select v-model="otherwise" class="panel__select">
            <option v-for="t in targets" :key="t.id" :value="t.id">{{ t.label }}</option>
          </select>
        </label>
      </template>
      <label v-else-if="step.kind !== 'loop'" class="panel__row">
        <span class="panel__hint">puis →</span>
        <select v-model="next" class="panel__select">
          <option v-for="t in targets" :key="t.id" :value="t.id">{{ t.label }}</option>
        </select>
      </label>
    </div>

    <label v-if="step.kind !== 'condition'" class="panel__field">
      <span class="panel__label">Si cette étape échoue</span>
      <select v-model="onFailure" class="panel__select">
        <option value="">Réglage du test</option>
        <option value="stop">Arrêter le test</option>
        <option value="continue">Continuer</option>
      </select>
    </label>

    <footer class="panel__foot">
      <BaseButton v-if="justInserted" variant="link" @click="remove"
        >Annuler l'insertion · Ctrl+Z</BaseButton
      >
      <BaseButton variant="danger" @click="remove">Supprimer l'étape</BaseButton>
    </footer>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.panel__head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.panel__kind,
.panel__label {
  margin: 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.panel__new {
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-caption);
  color: var(--color-accent-text);
  background: var(--color-accent);
}

.panel__title {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.panel__url {
  margin: 0;
  overflow-wrap: anywhere;
}

.panel__url code,
.panel__line code {
  font: var(--text-small) var(--font-mono);
}

.panel__block {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.panel__set {
  margin: 0;
  padding: 0;
  border: 0;
}

.panel__line {
  margin: 0;
  font-size: var(--text-small);
}

.panel__muted {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.panel__field {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.panel__field--wide {
  flex: 2;
}

.panel__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.panel__hint {
  min-width: 4.5rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.panel__radio {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: var(--text-small);
}

.panel__input,
.panel__select {
  flex: 1;
  min-width: 0;
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-bg);
}

.panel__code {
  min-height: 9rem;
  padding: 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: var(--editor-font-size) var(--editor-font);
  color: var(--color-text);
  background-color: var(--color-bg);
  resize: vertical;
}

.panel__input:focus-visible,
.panel__select:focus-visible,
.panel__code:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.panel__foot {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
