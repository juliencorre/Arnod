<script setup lang="ts">
/**
 * Relier le projet (`docs/plan-v2.md`, lot E, 7.1-a, SY1, SY2) : GitHub, GitLab (y compris
 * auto-hébergé) ou une autre adresse Git ; connexion par le navigateur (Q2) ou jeton personnel,
 * rangé au coffre (R9) ; choix ou création du dépôt, branche ; « Relier et envoyer ».
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useGuard } from "@/composables/useGuard";
import { ipc } from "@/lib/ipc";
import { initial } from "@/lib/sync";
import { useCatalogStore } from "@/stores/catalog";
import { useSyncStore } from "@/stores/sync";
import type { SyncAccount, SyncDeviceCode, SyncForge, SyncRepository } from "@/types/ipc";

const sync = useSyncStore();
const catalog = useCatalogStore();
const guard = useGuard();

const FORGES: { id: SyncForge; badge: string; name: string; hint: string; host: string }[] = [
  {
    id: "github",
    badge: "GH",
    name: "GitHub",
    hint: "github.com ou Enterprise",
    host: "github.com",
  },
  {
    id: "gitlab",
    badge: "GL",
    name: "GitLab",
    hint: "gitlab.com ou auto-hébergé",
    host: "gitlab.com",
  },
  { id: "other", badge: "git", name: "Autre URL Git", hint: "Gitea, Azure…", host: "" },
];

const projectName = computed(
  () => catalog.projects.find((p) => p.id === sync.linking)?.name ?? "projet",
);
const forge = ref<SyncForge>("github");
const host = ref("github.com");
const account = ref<SyncAccount | null>(null);
const token = ref("");
const login = ref("");
const device = ref<SyncDeviceCode | null>(null);
const repositories = ref<SyncRepository[]>([]);
const query = ref("");
const url = ref("");
const branch = ref("main");
const error = ref<string | null>(null);

const browser = computed(
  () =>
    (forge.value === "github" && sync.loginOptions.githubBrowser) ||
    (forge.value === "gitlab" && sync.loginOptions.gitlabBrowser),
);
const shown = computed(() => {
  const wanted = query.value.trim().toLowerCase();
  return repositories.value.filter((r) => wanted === "" || r.name.toLowerCase().includes(wanted));
});
const canLink = computed(() => url.value.trim() !== "" && !sync.busy);

// À chaque ouverture : le compte déjà relié au projet, s'il y en a un.
watch(
  () => sync.linking,
  (id) => {
    if (!id) return;
    const known = sync.views[id]?.account ?? null;
    forge.value = known?.forge ?? "github";
    host.value = known?.host ?? "github.com";
    account.value = known;
    token.value = "";
    login.value = "";
    device.value = null;
    repositories.value = [];
    query.value = "";
    url.value = "";
    branch.value = "main";
    error.value = null;
    if (known) void loadRepositories();
  },
);

function choose(next: SyncForge): void {
  forge.value = next;
  host.value = FORGES.find((f) => f.id === next)?.host ?? "";
  account.value = null;
  repositories.value = [];
  url.value = "";
  device.value = null;
}

async function loadRepositories(): Promise<void> {
  if (!account.value || account.value.forge === "other") return;
  repositories.value = await ipc.syncRepositories(account.value, "");
}

async function connected(next: SyncAccount): Promise<void> {
  account.value = next;
  token.value = "";
  device.value = null;
  await loadRepositories();
}

async function withBrowser(): Promise<void> {
  device.value = await ipc.syncConnectStart(forge.value, host.value.trim());
  try {
    await connected(await ipc.syncConnectWait(forge.value, host.value.trim()));
  } finally {
    device.value = null;
  }
}

function cancelBrowser(): void {
  void ipc.syncConnectCancel(forge.value, host.value.trim());
  device.value = null;
}

async function withToken(): Promise<void> {
  const target = forge.value === "other" ? hostOf(url.value) : host.value.trim();
  await connected(
    await ipc.syncToken(forge.value, target, token.value, login.value.trim() || null),
  );
}

function hostOf(address: string): string {
  const match = /^[a-z]+:\/\/([^/@]+@)?([^/]+)/i.exec(address.trim());
  return match?.[2] ?? address.trim();
}

async function create(): Promise<void> {
  if (!account.value) return;
  const created = await ipc.syncCreateRepository(account.value, projectName.value);
  repositories.value = [created, ...repositories.value];
  url.value = created.url;
}

function close(): void {
  if (device.value) cancelBrowser();
  sync.linking = null;
}

async function link(): Promise<void> {
  const id = sync.linking;
  if (!id) return;
  error.value = null;
  try {
    await sync.link(id, url.value.trim(), branch.value.trim() || "main", account.value);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
}
</script>

<template>
  <BaseDialog
    :open="sync.linking !== null"
    :title="`Synchroniser ${projectName}`"
    description="On relie le projet à un dépôt GitHub ou GitLab pour travailler à plusieurs."
    wide
    closable
    @close="close"
  >
    <div class="link__forges" role="radiogroup" aria-label="Où partager le projet">
      <button
        v-for="item in FORGES"
        :key="item.id"
        type="button"
        role="radio"
        class="link__forge"
        :aria-checked="forge === item.id"
        @click="choose(item.id)"
      >
        <span class="link__badge" aria-hidden="true">{{ item.badge }}</span>
        <span class="link__forge-name">{{ item.name }}</span>
        <span class="link__hint">{{ item.hint }}</span>
      </button>
    </div>

    <div class="link__grid">
      <template v-if="forge !== 'other'">
        <label class="link__label" for="link-host">Serveur</label>
        <input id="link-host" v-model="host" class="link__input" type="text" />
      </template>

      <span class="link__label">Compte</span>
      <div class="link__account">
        <p v-if="account" class="link__connected">
          <span class="link__avatar" aria-hidden="true">{{ initial(account.login) }}</span>
          <strong>{{ account.login }}</strong>
          <span class="link__ok">✓ connecté</span>
          <BaseButton variant="link" @click="account = null">Changer</BaseButton>
        </p>
        <template v-else>
          <div v-if="device" class="link__device" role="status">
            Saisissez le code <strong class="link__code">{{ device.userCode }}</strong> sur la page
            ouverte dans le navigateur…
            <BaseButton variant="link" @click="cancelBrowser">Annuler</BaseButton>
          </div>
          <BaseButton v-else-if="browser" @click="guard(withBrowser)"
            >Se connecter par le navigateur</BaseButton
          >
          <div class="link__token">
            <input
              v-if="forge === 'other'"
              v-model="login"
              class="link__input"
              type="text"
              aria-label="Nom d'utilisateur"
              placeholder="Nom d'utilisateur (facultatif)"
            />
            <input
              v-model="token"
              class="link__input"
              type="password"
              autocomplete="off"
              :aria-label="browser ? 'Ou un jeton personnel' : 'Jeton personnel'"
              :placeholder="browser ? 'Ou un jeton personnel' : 'Jeton personnel'"
            />
            <BaseButton :disabled="token.trim() === ''" @click="guard(withToken)"
              >Vérifier le jeton</BaseButton
            >
          </div>
        </template>
        <p class="link__note">
          Connexion par le navigateur : Arnod ne voit jamais votre mot de passe. Le jeton est rangé
          dans le coffre du système, jamais dans le projet.
        </p>
      </div>

      <span class="link__label">Dépôt</span>
      <div class="link__repo">
        <template v-if="account && forge !== 'other'">
          <input
            v-model="query"
            class="link__input"
            type="search"
            aria-label="Rechercher un dépôt"
            placeholder="Rechercher…"
          />
          <ul class="link__repos" role="radiogroup" aria-label="Dépôts du compte">
            <li v-for="repo in shown" :key="repo.url">
              <button
                type="button"
                role="radio"
                class="link__repo-item"
                :aria-checked="url === repo.url"
                @click="url = repo.url"
              >
                <span>{{ repo.name }}</span>
                <span class="link__hint">{{ repo.empty ? "vide" : "" }}</span>
                <span v-if="url === repo.url" aria-hidden="true">✓</span>
              </button>
            </li>
          </ul>
          <BaseButton variant="link" @click="guard(create)"
            >+ Créer un dépôt « {{ projectName }} »</BaseButton
          >
        </template>
        <input
          v-else
          v-model="url"
          class="link__input"
          type="text"
          aria-label="Adresse du dépôt"
          placeholder="https://git.exemple.fr/equipe/boutique-tests.git"
        />
      </div>

      <label class="link__label" for="link-branch">Branche</label>
      <input id="link-branch" v-model="branch" class="link__input link__branch" type="text" />
    </div>
    <p v-if="error" class="link__error" role="alert">{{ error }}</p>

    <template #footer>
      <BaseButton @click="close">Annuler</BaseButton>
      <BaseButton variant="primary" :disabled="!canLink" @click="guard(link)"
        >Relier et envoyer</BaseButton
      >
    </template>
  </BaseDialog>
</template>

<style scoped>
.link__forges {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.link__forge {
  display: grid;
  grid-template-columns: auto 1fr;
  column-gap: 0.5rem;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.link__forge[aria-checked="true"] {
  border-color: var(--color-focus);
  background: var(--color-accent-soft);
}

.link__badge {
  grid-row: span 2;
  padding: 0.125rem 0.375rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  font-weight: 600;
  background: var(--color-chip);
}

.link__forge-name {
  font-weight: 600;
}

.link__hint,
.link__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.link__grid {
  display: grid;
  grid-template-columns: 6rem 1fr;
  gap: 0.75rem;
  align-items: start;
}

.link__label {
  padding-top: 0.375rem;
  font-weight: 600;
}

.link__input {
  box-sizing: border-box;
  width: 100%;
  height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.link__branch {
  width: 12rem;
}

.link__account,
.link__repo,
.link__token {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: flex-start;
}

.link__connected {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  margin: 0;
}

.link__avatar {
  display: inline-grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  background: var(--color-accent-soft);
}

.link__ok {
  color: var(--color-success);
}

.link__code {
  font-family: var(--editor-font);
}

.link__note {
  margin: 0;
}

.link__repos {
  width: 100%;
  max-height: 10rem;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.link__repo-item {
  display: flex;
  gap: 0.5rem;
  justify-content: space-between;
  width: 100%;
  min-height: var(--control-sm);
  padding: 0 0.5rem;
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
}

.link__repo-item[aria-checked="true"] {
  background: var(--color-accent-soft);
}

.link__error {
  margin: 0.75rem 0 0;
  color: var(--color-danger);
}
</style>
