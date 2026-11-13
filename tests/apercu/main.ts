/**
 * Point d'entrée de l'aperçu (`vite.apercu.config.ts`) : monte l'application sur le faux cœur du
 * scénario demandé par `?scenario=…`, joue ses gestes, puis signale `data-apercu="pret"` sur
 * `<html>` (ou `data-apercu="erreur"`). Le thème suit `prefers-color-scheme`.
 */
import { createPinia } from "pinia";
import { createApp, nextTick } from "vue";

import App from "@/App.vue";
import "@/styles.css";

import { SCENARIOS, type Gestes } from "./scenarios";

const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function attendre(ms = 30): Promise<void> {
  await nextTick();
  await pause(ms);
  await nextTick();
}

function trouver(selecteur: string, texte?: string): HTMLElement {
  const elements = Array.from(document.querySelectorAll<HTMLElement>(selecteur));
  const cible =
    texte === undefined ? elements[0] : elements.find((e) => e.textContent.includes(texte));
  if (!cible) throw new Error(`élément introuvable : ${selecteur} ${texte ?? ""}`);
  return cible;
}

const gestes: Gestes = {
  cliquer: async (selecteur, texte) => {
    trouver(selecteur, texte).click();
    await attendre();
  },
  saisir: async (selecteur, valeur) => {
    const champ = trouver(selecteur) as HTMLInputElement | HTMLTextAreaElement;
    champ.focus();
    champ.value = valeur;
    champ.dispatchEvent(new Event("input", { bubbles: true }));
    champ.dispatchEvent(new Event("change", { bubbles: true }));
    await attendre();
  },
  touche: async (cle, options = {}) => {
    const { selecteur, ...init } = options;
    const cible = selecteur ? trouver(selecteur) : (document.activeElement ?? document.body);
    cible.dispatchEvent(new KeyboardEvent("keydown", { key: cle, bubbles: true, ...init }));
    await attendre();
  },
  attendre,
};

async function demarrer(): Promise<void> {
  const nom = new URLSearchParams(location.search).get("scenario") ?? "accueil";
  const scenario = SCENARIOS[nom];
  if (!scenario) throw new Error(`scénario inconnu : ${nom}`);
  const core = scenario.setup();
  createApp(App).use(createPinia()).mount("#app");
  await attendre(100);
  await scenario.play?.(gestes, core);
  await attendre(100);
}

demarrer().then(
  () => {
    document.documentElement.setAttribute("data-apercu", "pret");
  },
  (error: unknown) => {
    console.error(error);
    document.documentElement.setAttribute("data-apercu", "erreur");
    document.documentElement.setAttribute("data-apercu-message", String(error));
  },
);
