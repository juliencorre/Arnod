import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import type {
  Auth,
  CertificateInfo,
  CookieJarView,
  NetworkPrefsInput,
  NetworkView,
  ProjectAuthView,
} from "@/types/ipc";

/** Sections des préférences ; les suivantes viendront (Apparence, Raccourcis, Mises à jour). */
export type PreferencesSection = "network" | "updates";

/**
 * Lot H - Réseau et authentification (docs/plan-mvp-h-reseau-auth.md) : préférences réseau du
 * poste (1.6-e), autorité à confirmer (5.1-i), réglages du projet (3.4-a), cookies (3.4-g).
 * Le cœur décide de tout ; le store garde ce qu'il a renvoyé.
 */
export const useNetworkStore = defineStore("network", () => {
  /** Préférences ouvertes (Ctrl+,), et leur section. */
  const preferencesOpen = ref(false);
  const section = ref<PreferencesSection>("network");
  const view = ref<NetworkView | null>(null);
  /** Autorité lue, en attente de « Faire confiance » (R4). */
  const pendingAuthority = ref<CertificateInfo | null>(null);
  /** Réglages du projet ouverts : projet et onglet. */
  const projectSettings = ref<{ projectId: string } | null>(null);
  const projectAuth = ref<ProjectAuthView | null>(null);
  /** Cookies gardés, par clé « projet · environnement · adresse ». */
  const cookies = ref<Record<string, CookieJarView>>({});

  async function load(): Promise<NetworkView> {
    view.value = await ipc.networkLoad();
    return view.value;
  }

  async function openPreferences(next: PreferencesSection = "network"): Promise<void> {
    section.value = next;
    preferencesOpen.value = true;
    await load();
  }

  function closePreferences(): void {
    preferencesOpen.value = false;
    pendingAuthority.value = null;
  }

  async function save(prefs: NetworkPrefsInput): Promise<void> {
    view.value = await ipc.networkSave(prefs);
  }

  /** « + Ajouter une autorité… » : fichier choisi et lu, montré avant d'être ajouté. */
  async function pickAuthority(): Promise<void> {
    const picked = await ipc.networkPickAuthority();
    if (picked) pendingAuthority.value = picked;
  }

  /** Montre une autorité à confirmer (depuis l'échec d'un envoi, 5.1-i). */
  function proposeAuthority(certificate: CertificateInfo): void {
    pendingAuthority.value = certificate;
  }

  async function trustPending(): Promise<void> {
    const pending = pendingAuthority.value;
    if (!pending) return;
    view.value = await ipc.networkAddAuthority(pending.pem);
    pendingAuthority.value = null;
  }

  function cancelPending(): void {
    pendingAuthority.value = null;
  }

  async function removeAuthority(fingerprint: string): Promise<void> {
    view.value = await ipc.networkRemoveAuthority(fingerprint);
  }

  // --- Réglages du projet (3.4-a) -------------------------------------------

  async function openProjectSettings(projectId: string, environment: string): Promise<void> {
    projectSettings.value = { projectId };
    projectAuth.value = null;
    projectAuth.value = await ipc.projectAuthLoad(projectId, environment);
  }

  function closeProjectSettings(): void {
    projectSettings.value = null;
    projectAuth.value = null;
  }

  async function loadProjectAuth(projectId: string, environment: string): Promise<void> {
    projectAuth.value = await ipc.projectAuthLoad(projectId, environment);
  }

  async function saveProjectAuth(
    projectId: string,
    environment: string,
    auth: Auth,
    clientSecret: string | null,
  ): Promise<void> {
    projectAuth.value = await ipc.projectAuthSave(projectId, environment, auth, clientSecret);
  }

  async function renewToken(projectId: string, environment: string): Promise<void> {
    projectAuth.value = await ipc.oauth2TokenRenew(projectId, environment);
  }

  async function forgetToken(projectId: string, environment: string): Promise<void> {
    projectAuth.value = await ipc.oauth2TokenForget(projectId, environment);
  }

  // --- Cookies (3.4-g) ------------------------------------------------------

  function cookieKey(projectId: string, environment: string, url: string | null): string {
    return `${projectId} · ${environment} · ${url ?? ""}`;
  }

  async function loadCookies(
    projectId: string,
    environment: string,
    url: string | null,
  ): Promise<void> {
    cookies.value[cookieKey(projectId, environment, url)] = await ipc.cookiesList(
      projectId,
      environment,
      url,
    );
  }

  async function clearCookies(
    projectId: string,
    environment: string,
    domain: string | null,
    url: string | null,
  ): Promise<void> {
    cookies.value[cookieKey(projectId, environment, url)] = await ipc.cookiesClear(
      projectId,
      environment,
      domain,
      url,
    );
  }

  async function setCookieSending(
    projectId: string,
    environment: string,
    domain: string,
    enabled: boolean,
    url: string | null,
  ): Promise<void> {
    cookies.value[cookieKey(projectId, environment, url)] = await ipc.cookiesSetSending(
      projectId,
      environment,
      domain,
      enabled,
      url,
    );
  }

  return {
    preferencesOpen,
    section,
    view,
    pendingAuthority,
    projectSettings,
    projectAuth,
    cookies,
    load,
    openPreferences,
    closePreferences,
    save,
    pickAuthority,
    proposeAuthority,
    trustPending,
    cancelPending,
    removeAuthority,
    openProjectSettings,
    closeProjectSettings,
    loadProjectAuth,
    saveProjectAuth,
    renewToken,
    forgetToken,
    cookieKey,
    loadCookies,
    clearCookies,
    setCookieSending,
  };
});
