/**
 * Scénarios de l'aperçu : l'application réelle (composants, stores, styles) sur le faux cœur,
 * dans un navigateur, pour la comparer aux écrans de la maquette (`scripts/apercu.mjs`).
 *
 * Un scénario prépare le faux cœur (`setup`), puis joue des gestes dans la page (`play`) avec les
 * aides de `Gestes`. Ajouter un scénario par écran livré, nommé d'après son repère si possible.
 */
import { installFakeCore, type FakeCore } from "../fakeCore";
import { newRequest, okOutcome } from "../fakeWorkbench";
import { boutiqueSnapshot, firstRunSnapshot } from "../fixtures";

/** Gestes dans la page, par sélecteur CSS et texte visible. */
export interface Gestes {
  /** Clique le premier élément qui correspond (et contient `texte`, s'il est donné). */
  cliquer: (selecteur: string, texte?: string) => Promise<void>;
  /** Saisit une valeur dans un champ (événements `input` et `change`). */
  saisir: (selecteur: string, valeur: string) => Promise<void>;
  /** Envoie une touche au document ou à l'élément ciblé. */
  touche: (cle: string, options?: KeyboardEventInit & { selecteur?: string }) => Promise<void>;
  /** Attend que les promesses et le rendu se terminent. */
  attendre: (ms?: number) => Promise<void>;
}

export interface Scenario {
  description: string;
  setup: () => FakeCore;
  play?: (gestes: Gestes, core: FakeCore) => Promise<void>;
}

/** Projet « boutique-tests » ouvert, avec quelques requêtes (comme `tests/workbench/helpers.ts`). */
function boutique(): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  const wb = core.workbench;
  wb.folders.add("Clients");
  wb.requests.set(
    "Clients/lister.yml",
    newRequest("Lister les clients", { url: "{{baseUrl}}/clients", tags: ["smoke"] }),
  );
  wb.requests.set(
    "Clients/creer.yml",
    newRequest("Créer un client", { method: "POST", url: "{{baseUrl}}/clients" }),
  );
  wb.requests.set("sante.yml", newRequest("Santé", { url: "{{baseUrl}}/health" }));
  return core;
}

async function ouvrirProjet(g: Gestes): Promise<void> {
  await g.cliquer("main article button", "Ouvrir");
}

// --- Lot B : saisie, texte et cURL (docs/plan-mvp-b-saisie-curl.md) -----------------

/** « Créer une commande » de la maquette : une variable inconnue de Recette fonctionnelle. */
function commande(): FakeCore {
  const core = boutique();
  const wb = core.workbench;
  wb.envs = [
    {
      name: "Recette fonctionnelle",
      color: "orange",
      variables: [{ name: "baseUrl", value: "https://recette.boutique.dev" }],
      locals: ["token"],
    },
    {
      name: "Local",
      color: "gray",
      variables: [
        { name: "baseUrl", value: "http://localhost:8080" },
        { name: "apiVersion", value: "v2" },
      ],
      locals: [],
    },
  ];
  wb.active = "Recette fonctionnelle";
  wb.folders.add("commandes");
  const ligne = (name: string, value: string) => ({
    name,
    value,
    enabled: true,
    kind: "query" as const,
    description: "",
  });
  const request = newRequest("Créer une commande", {
    method: "POST",
    url: "{{baseUrl}}/{{apiVersion}}/orders",
    headers: [ligne("Content-Type", "application/json"), ligne("X-Client", "arnod-tests")],
    body: {
      kind: "json",
      variants: [
        {
          title: "",
          content: '{"customerId":"cus_31","items":[{"sku":"TSH-BLU-M","qty":2}]}',
          form: [],
        },
      ],
      selected: 0,
      other: "",
    },
  });
  request.inherited.headers = [
    { name: "Authorization", value: "Bearer {{token}}", source: "projet" },
  ];
  wb.requests.set("commandes/creer.yml", request);
  return core;
}

async function ouvrirCommande(g: Gestes): Promise<void> {
  await ouvrirProjet(g);
  await g.cliquer('[role="treeitem"]', "Créer une commande");
  await g.attendre(400);
}

const CURL_COLLEE =
  'curl -X POST https://api.boutique.dev/orders -H \'Content-Type: application/json\' -H \'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOi\' -d \'{"customerId":"cus_31","items":[{"sku":"TSH-BLU-M","qty":2}]}\' --compressed';

export const SCENARIOS: Record<string, Scenario> = {
  accueil: {
    description: "Premier lancement (2.1-a)",
    setup: () => installFakeCore(firstRunSnapshot()),
  },
  "tous-les-projets": {
    description: "Workspace « Boutique » et ses projets (1.1-a)",
    setup: boutique,
  },
  projet: {
    description: "Projet ouvert dans la barre latérale (1.2-a)",
    setup: boutique,
    play: ouvrirProjet,
  },
  requete: {
    description: "Requête ouverte et envoyée (5.1-b)",
    setup: () => {
      const core = boutique();
      core.workbench.nextSend = okOutcome({
        response: {
          status: 200,
          statusText: "OK",
          headers: [["content-type", "application/json"]],
          body: '{"clients":[{"id":1,"name":"Alice","email":"alice@boutique.dev"},{"id":2,"name":"Bruno","email":"bruno@boutique.dev"}],"total":2}',
          truncated: false,
          sizeBytes: 118,
          waitMs: 112,
          receiveMs: 6,
          totalMs: 118,
        },
      });
      return core;
    },
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[role="treeitem"]', "Lister les clients");
      await g.cliquer(".editor__line button[type='submit']");
      await g.attendre(100);
    },
  },
  // Lot B : saisie, texte et cURL.
  "5.1-b-ter": {
    description: "Variable inconnue soulignée, compteurs d'onglets, en-têtes ajoutés (5.1-b ter)",
    setup: commande,
    play: async (g) => {
      await ouvrirCommande(g);
      await g.cliquer('[role="tab"]', "En-têtes");
      await g.attendre(400);
    },
  },
  "5.1-b-quater": {
    description: "En-têtes édités en texte et copie en cURL (5.1-b quater)",
    setup: commande,
    play: async (g) => {
      await ouvrirCommande(g);
      await g.cliquer('[role="tab"]', "En-têtes");
      await g.cliquer(".pairs__mode", "Éditer en texte");
      await g.saisir(
        ".pairs__textarea",
        "Content-Type: application/json\nX-Client: arnod-tests\n# X-Debug: true\nAccept-Language: fr-FR\nCache-Control no-cache",
      );
      await g.cliquer(".editor__curl-button");
      await g.attendre(200);
    },
  },
  "2.1-a-sexies": {
    description: "Une commande cURL collée dans « Essayer une étape » (2.1-a sexies)",
    setup: () => installFakeCore(firstRunSnapshot()),
    play: async (g) => {
      await g.saisir(".try__input", CURL_COLLEE);
      await g.attendre(200);
    },
  },
};
