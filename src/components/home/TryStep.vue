<script setup lang="ts">
/**
 * « Essayer une étape » (docs/plan-accueil-non-classes.md, A1 à A5) : une seule
 * entrée, qui reconnaît la nature de ce que l'on colle (le cœur décide), ou une
 * nature choisie sur sa pastille. La nature choisie ouvre son formulaire sous
 * l'entrée (`open`) ; « Envoyer » garde l'essai dans « Non classés » et l'ouvre.
 * Échap ou « Fermer » revient à l'état de départ.
 *
 * Une commande cURL collée (docs/plan-mvp-b-saisie-curl.md, 2.1-a sexies) est lue par le cœur :
 * méthode, adresse, en-têtes et corps passent dans le formulaire, le jeton est retiré du texte
 * (gardé au coffre à l'envoi), les options sans équivalent sont signalées.
 */
import { computed, nextTick, onMounted, ref, useId, useTemplateRef, watch } from "vue";

import NatureForm from "@/components/home/NatureForm.vue";
import NaturePill from "@/components/steps/NaturePill.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { IpcError } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useNavigationStore } from "@/stores/navigation";
import { useStepsStore } from "@/stores/steps";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { CurlImport, Detection } from "@/types/ipc";

withDefaults(defineProps<{ note?: string; headingLevel?: 2 | 3 }>(), {
  note: "Sans projet : l'essai est gardé dans « Non classés ».",
  headingLevel: 2,
});
const emit = defineEmits<{ open: [boolean] }>();

const steps = useStepsStore();
const catalog = useCatalogStore();
const contracts = useContractsStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const ids = { title: useId(), status: useId(), error: useId() };
const input = useTemplateRef<HTMLInputElement>("input");

const text = ref("");
const chosen = ref<string | null>(null);
const detected = ref<Detection | null>(null);
const values = ref<Record<string, string>>({});
const sending = ref(false);
const error = ref<string | null>(null);
/** Commande cURL collée : secrets retirés et options signalées, jusqu'à la fin de l'essai. */
const imported = ref<CurlImport | null>(null);

const nature = computed(() => steps.nature(chosen.value ?? detected.value?.nature ?? null));
const placeholder = computed(() => {
  if (nature.value) return nature.value.subject.placeholder;
  const examples = steps.natures.map((n) => n.subject.placeholder).join(", ");
  return examples === ""
    ? "Collez ce que vous voulez essayer"
    : `Collez ce que vous voulez essayer, par exemple ${examples}`;
});
const status = computed(() => {
  if (!nature.value) return "";
  if (imported.value) return "Reconnu : une commande cURL.";
  return chosen.value === null
    ? `Reconnu : ${nature.value.name} (${nature.value.title}).`
    : `Nature choisie : ${nature.value.name}.`;
});

watch(
  nature,
  (next) => {
    emit("open", next !== null);
  },
  { immediate: true },
);

onMounted(() => {
  steps.load().catch((cause: unknown) => {
    toasts.showError(cause);
  });
});

// Le cœur reconnaît la nature ; une méthode en tête passe dans le formulaire.
watch(text, async (value) => {
  error.value = null;
  const found = value.trim() === "" ? null : await steps.detect(value).catch(() => null);
  if (text.value !== value) return;
  detected.value = found;
  // Entrée vidée : la commande collée et son jeton sont oubliés.
  if (value.trim() === "") imported.value = null;
  if (found?.curl) {
    // Une commande cURL remplace tout le formulaire ; l'entrée garde l'adresse.
    imported.value = found.curl;
    values.value = { ...found.values };
    text.value = found.subject;
  } else if (found && Object.keys(found.values).length > 0 && found.subject !== value.trim()) {
    values.value = { ...values.value, ...found.values };
    text.value = found.subject;
  }
});

/** Ce que la commande cURL a donné, en une phrase (2.1-a sexies). */
const importNote = computed(() => {
  const curl = imported.value;
  if (!curl) return null;
  const headers = (values.value.headers ?? "").split("\n").filter((l) => l.trim() !== "").length;
  const parts = ["méthode", "adresse"];
  if (headers > 0) parts.push(headers === 1 ? "un en-tête" : `${String(headers)} en-têtes`);
  if ((values.value.body ?? "").trim() !== "") parts.push("le corps");
  const taken = `${parts.slice(0, -1).join(", ")} et ${parts[parts.length - 1] ?? ""}`;
  return {
    taken: `Repris de la commande : ${taken}.`,
    secrets: curl.secrets.map(
      (secret) =>
        `${secret.header} : la valeur ne reste pas dans le texte, elle part dans le coffre du système et l'essai l'utilise par la variable locale {{${secret.name}}}.`,
    ),
    ignored: curl.ignored.map((item) => `${item.option} est ignorée : ${item.reason}.`),
  };
});

function choose(id: string): void {
  chosen.value = chosen.value === id ? null : id;
  error.value = null;
  void nextTick(() => input.value?.focus());
}

function reset(): void {
  text.value = "";
  chosen.value = null;
  detected.value = null;
  values.value = {};
  error.value = null;
  imported.value = null;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && (text.value !== "" || nature.value !== null)) {
    event.preventDefault();
    reset();
  }
}

async function send(): Promise<void> {
  const current = nature.value;
  if (current === null) {
    error.value =
      "Arnod ne reconnaît pas cette saisie : collez une adresse http(s)://, ou choisissez une nature.";
    return;
  }
  if (text.value.trim() === "") {
    error.value = `Saisissez d'abord : ${current.subject.label.toLowerCase()}.`;
    void nextTick(() => input.value?.focus());
    return;
  }
  sending.value = true;
  error.value = null;
  try {
    const secrets = Object.fromEntries(
      (imported.value?.secrets ?? []).map((secret) => [secret.name, secret.value]),
    );
    const result = await steps.send(current.id, text.value.trim(), values.value, secrets);
    // L'instantané d'abord : « Non classés » existe avant d'être ouvert.
    catalog.apply(result.snapshot);
    // « Non classés » déjà affiché : l'essai paraît sous le contrat déduit (RC8).
    await contracts.sync(result.projectId);
    navigation.showProject(result.projectId, null);
    await workbench.openRequest(result.projectId, result.path);
    toasts.show(`✓ Essai gardé dans « Non classés », sous le contrat de son serveur.`);
    reset();
  } catch (cause) {
    if (!(cause instanceof IpcError && cause.kind === "cancelled")) {
      error.value = cause instanceof IpcError ? cause.message : String(cause);
    }
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <section class="try" :aria-labelledby="ids.title">
    <form class="try__form" novalidate @submit.prevent="send">
      <div class="try__head">
        <component :is="`h${headingLevel}`" :id="ids.title" class="try__title">
          Essayer une étape
        </component>
        <span class="try__hint">Arnod reconnaît la nature d'étape de ce que vous collez</span>
        <span class="try__note" :class="{ 'try__note--found': imported }">
          {{ imported ? status : note }}
        </span>
      </div>
      <div class="try__row">
        <div class="try__box" :class="{ 'try__box--idle': nature === null && text === '' }">
          <span class="try__nature" aria-hidden="true">
            <NaturePill v-if="nature" :name="nature.name" :family="nature.family" />
            <span v-else class="try__auto">Auto</span>
          </span>
          <input
            ref="input"
            v-model="text"
            class="try__input"
            type="text"
            spellcheck="false"
            autocomplete="off"
            :placeholder="placeholder"
            :aria-labelledby="ids.title"
            :aria-describedby="error ? ids.error : ids.status"
            :aria-invalid="error !== null"
            @keydown="onKeydown"
          />
          <button
            v-if="nature || text"
            type="button"
            class="try__clear"
            aria-label="Fermer le formulaire et vider l'entrée"
            title="Fermer (Échap)"
            @click="reset"
          >
            ×
          </button>
        </div>
        <BaseButton v-if="sending" @click="steps.cancel()">Annuler</BaseButton>
        <BaseButton v-else type="submit" variant="primary">
          {{ nature?.action ?? "Essayer" }}
        </BaseButton>
      </div>
      <p :id="ids.status" class="visually-hidden" aria-live="polite">{{ status }}</p>
      <div class="try__chips">
        <span class="try__chips-label">Ou choisissez :</span>
        <button
          v-for="item in steps.natures"
          :key="item.id"
          type="button"
          class="try__chip"
          :aria-pressed="chosen === item.id"
          :title="`${item.name} : ${item.title}`"
          @click="choose(item.id)"
        >
          <NaturePill :name="item.name" :family="item.family" size="md" />
        </button>
        <BaseButton variant="link" @click="navigation.showSteps()">
          Parcourir le catalogue…
        </BaseButton>
      </div>
      <NatureForm v-if="nature" v-model="values" :nature="nature" />
      <div v-if="importNote" class="try__import" role="note">
        <p>{{ importNote.taken }}</p>
        <p v-for="line in importNote.secrets" :key="line" class="try__secret">
          {{ line }}
        </p>
        <p v-for="line in importNote.ignored" :key="line" class="try__ignored">{{ line }}</p>
      </div>
      <p v-if="sending" class="try__sending" role="status">Envoi en cours…</p>
      <p v-if="error" :id="ids.error" class="try__error" role="alert">{{ error }}</p>
    </form>
  </section>
</template>

<style scoped>
.try {
  width: 100%;
}

.try__form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.try__head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.25rem 0.625rem;
}

.try__title {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.try__hint,
.try__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.try__note {
  margin-left: auto;
}

.try__row {
  display: flex;
  gap: 0.5rem;
}

.try__box {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
  border: 1.5px solid var(--color-text);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--color-surface);
}

.try__box--idle {
  border-color: var(--color-border-strong);
}

.try__box:focus-within {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.try__nature {
  display: flex;
  align-items: center;
  align-self: stretch;
  padding: 0 0.625rem;
  border-right: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.try__auto {
  display: inline-flex;
  align-items: center;
  height: 1.25rem;
  padding: 0 0.5rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: var(--radius-pill);
  font-family: var(--font-mono);
  font-size: var(--text-caption);
  font-weight: 600;
  color: var(--color-muted);
  background: var(--color-surface);
}

.try__input {
  flex: 1;
  min-width: 0;
  padding: 0.625rem 0.75rem;
  border: 0;
  font-family: var(--font-mono);
  font-size: var(--text-body);
  color: var(--color-text);
  background: none;
}

.try__input:focus {
  outline: none;
}

.try__clear {
  margin-right: 0.25rem;
  padding: 0.25rem 0.5rem;
  border: 0;
  border-radius: var(--radius-sm);
  font: inherit;
  font-size: 1rem;
  line-height: 1;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.try__clear:hover {
  color: var(--color-text);
  background: var(--color-chip);
}

.try__clear:focus-visible {
  outline: 2px solid var(--color-focus);
}

.try__chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.try__chips-label {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.try__chip {
  display: inline-flex;
  padding: 0;
  border: 0;
  border-radius: var(--radius-pill);
  background: none;
  cursor: pointer;
}

.try__chip[aria-pressed="true"] {
  box-shadow: 0 0 0 1.5px var(--color-text);
}

.try__chip:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.try__note--found {
  font-weight: 600;
  color: var(--color-text);
}

.try__import {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  color: var(--color-muted);
  background-color: var(--color-sidebar);
}

.try__import p {
  margin: 0;
}

.try__secret {
  color: var(--color-text);
}

.try__ignored {
  color: var(--color-link);
}

.try__sending {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.try__error {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-danger);
}
</style>
