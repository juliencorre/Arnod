<script setup lang="ts">
/**
 * « Copier depuis un autre projet… » (3.3-a, AM43) : un projet de « Tous les projets », puis ses
 * environnements à cocher. Le cœur les ajoute en fin de chaîne avec leurs variables partagées et
 * les noms de leurs variables locales, jamais les valeurs locales ni les secrets.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ open: boolean; projectId: string }>();
const emit = defineEmits<{ close: []; copied: [name: string] }>();

const catalog = useCatalogStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const guard = useGuard();
const projectFieldId = useId();
const listId = useId();

/** Les autres projets de « Tous les projets ». */
const projects = computed(() => catalog.projects.filter((p) => p.id !== props.projectId));
const from = ref<string>(projects.value[0]?.id ?? "");
const chosen = ref<string[]>([]);
const busy = ref(false);

const environments = computed(() => workbench.environments[from.value]?.environments ?? []);

watch(
  from,
  (id) => {
    chosen.value = [];
    if (id !== "" && !workbench.environments[id]) {
      workbench.refreshEnvironments(id).catch(toasts.showError);
    }
  },
  { immediate: true },
);

const submitLabel = computed(() =>
  chosen.value.length === 0
    ? "Copier"
    : `Copier ${countLabel(chosen.value.length, "environnement", "environnements")}`,
);

function submit(): void {
  const names = [...chosen.value];
  const source = catalog.projectById(from.value)?.name ?? "l'autre projet";
  if (names.length === 0 || busy.value) return;
  busy.value = true;
  guard(async () => {
    const before = new Set(
      (workbench.environments[props.projectId]?.environments ?? []).map((e) => e.name),
    );
    try {
      await workbench.copyEnvironments(props.projectId, from.value, names);
    } finally {
      busy.value = false;
    }
    const added = (workbench.environments[props.projectId]?.environments ?? []).filter(
      (e) => !before.has(e.name),
    );
    toasts.show(
      `${countLabel(added.length, "environnement copié", "environnements copiés")} de « ${source} » : les valeurs locales restent à saisir sur ce poste.`,
    );
    emit("copied", added[0]?.name ?? "");
  });
}
</script>

<template>
  <BaseDialog
    :open="open"
    title="Copier depuis un autre projet"
    description="Variables partagées et noms des variables locales ; jamais leurs valeurs ni les secrets."
    @close="emit('close')"
  >
    <form id="copy-environments" class="copy" novalidate @submit.prevent="submit">
      <p v-if="projects.length === 0" class="copy__empty">
        Aucun autre projet dans « Tous les projets ».
      </p>
      <template v-else>
        <label :for="projectFieldId" class="copy__label">Projet</label>
        <select :id="projectFieldId" v-model="from">
          <option v-for="project in projects" :key="project.id" :value="project.id">
            {{ project.name }}
          </option>
        </select>
        <fieldset class="copy__list" :aria-describedby="listId">
          <legend class="copy__label">Environnements</legend>
          <p v-if="environments.length === 0" :id="listId" class="copy__empty">
            Ce projet n'a pas d'environnement.
          </p>
          <label v-for="env in environments" :key="env.name" class="copy__row">
            <input v-model="chosen" type="checkbox" :value="env.name" />
            <span>{{ env.name }}</span>
            <span class="copy__detail">{{
              countLabel(env.variables.length + env.locals.length, "variable", "variables")
            }}</span>
          </label>
        </fieldset>
      </template>
    </form>
    <template #footer>
      <BaseButton @click="emit('close')">Annuler</BaseButton>
      <BaseButton
        variant="primary"
        type="submit"
        form="copy-environments"
        :disabled="chosen.length === 0 || busy"
      >
        {{ submitLabel }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.copy {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.copy__label {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.copy__list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0.5rem 0 0;
  padding: 0;
  border: 0;
}

.copy__row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  cursor: pointer;
}

.copy__detail {
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.copy__empty {
  margin: 0;
  color: var(--color-muted);
}
</style>
