/**
 * Scénarios de l'aperçu : l'application réelle (composants, stores, styles) sur le faux cœur,
 * dans un navigateur, pour la comparer aux écrans de la maquette (`scripts/apercu.mjs`).
 *
 * Un scénario prépare le faux cœur (`setup`), puis joue des gestes dans la page (`play`) avec les
 * aides de `Gestes`. Ajouter un scénario par écran livré, nommé d'après son repère si possible.
 */
import { installFakeCore, type FakeCore } from "../fakeCore";
import { newRequest, okOutcome } from "../fakeWorkbench";
import { PETSTORE_SOURCE } from "../fakeContracts";
import { AUTH_SSO, BOUTIQUE_TESTS, boutiqueSnapshot, firstRunSnapshot } from "../fixtures";
import type { Example } from "@/types/ipc";
import { brunoSource } from "../fakeImports";

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

/** Lot D : largeur de la barre latérale (320 px comme la maquette, 200 px au minimum). */
function largeur(core: FakeCore, px: number): FakeCore {
  const main = core.windows.windows[0];
  if (main) main.sidebarWidth = px;
  return core;
}

/** Lot D : réponse attendue d'un endpoint. */
function reponse(status: number, statusText: string, name: string, description: string): Example {
  return {
    name,
    description,
    status,
    statusText,
    headers: [{ name: "Content-Type", value: "application/json" }],
    bodyKind: "json",
    body: "{}",
  };
}

/** Lot D : référentiel « commandes/v2 », comme l'écran 1.2-e. */
function commandes(): FakeCore {
  const core = boutique();
  const wb = core.workbench;
  wb.folders.add("commandes");
  wb.folders.add("commandes/v2");
  wb.requests.set(
    "commandes/v2/lister.yml",
    newRequest("Lister les commandes", { url: "{{baseUrl}}/orders?statut=ouvert" }),
  );
  wb.requests.set(
    "commandes/v2/lire.yml",
    newRequest("Lire une commande", { url: "{{baseUrl}}/orders/{{orderId}}" }),
  );
  wb.requests.set(
    "commandes/v2/changer.yml",
    newRequest("Changer le statut", {
      method: "PATCH",
      url: "{{baseUrl}}/orders/{{orderId}}/status",
      tags: ["critique"],
    }),
  );
  wb.requests.set(
    "commandes/v2/annuler.yml",
    newRequest("Annuler une commande", {
      method: "DELETE",
      url: "{{baseUrl}}/orders/{{orderId}}",
    }),
  );
  return core;
}

/** Lot D : contrat « API Commandes » et son endpoint `POST /orders` (3.2-a3). */
function ajouterContratCommandes(core: FakeCore): void {
  const file = "contracts/commandes.yaml";
  const op = (method: string, path: string, summary: string) => ({
    key: `${method} ${path}`,
    method,
    path,
    summary,
    tag: "commandes",
  });
  core.contracts.files.set(file, {
    name: "API Commandes",
    version: "2.4.0",
    source: PETSTORE_SOURCE,
    operations: [
      op("GET", "/products", "Lister les produits"),
      op("GET", "/products/{id}", "Lire un produit"),
      op("GET", "/orders", "Lister les commandes"),
      op("POST", "/orders", "Créer une commande"),
      op("GET", "/orders/{id}", "Lire une commande"),
      op("PATCH", "/orders/{id}/status", "Changer le statut"),
    ],
  });
  const wb = core.workbench;
  wb.folders.add("API Commandes");
  const link = (operation: string) => ({ file, operation });
  wb.requests.set(
    "API Commandes/lister.yml",
    newRequest("Lister les commandes", {
      url: "{{baseUrl}}/orders",
      contract: link("GET /orders"),
      examples: [
        reponse(200, "OK", "Liste", "Les commandes du client."),
        reponse(401, "Unauthorized", "Non authentifié", "Jeton absent ou expiré."),
      ],
    }),
  );
  wb.requests.set(
    "API Commandes/creer.yml",
    newRequest("Créer une commande", {
      method: "POST",
      url: "{{baseUrl}}/orders",
      contract: link("POST /orders"),
      examples: [
        reponse(201, "Created", "Commande créée", "La commande est enregistrée au statut pending."),
        reponse(400, "Bad Request", "Erreur de validation", "Un champ requis manque."),
        reponse(422, "Unprocessable Entity", "Article indisponible", "Plus en stock."),
      ],
    }),
  );
  wb.requests.set(
    "API Commandes/lire.yml",
    newRequest("Lire une commande", {
      url: "{{baseUrl}}/orders/{{id}}",
      contract: link("GET /orders/{id}"),
      examples: [reponse(200, "OK", "Commande", ""), reponse(404, "Not Found", "Introuvable", "")],
    }),
  );
}

async function ouvrirProjet(g: Gestes): Promise<void> {
  await g.cliquer("main article button", "Ouvrir");
}

// --- Lot B : saisie, texte et cURL (docs/plan-mvp-b-saisie-curl.md) -----------------

/** « Créer une commande » de la maquette : une variable inconnue de Recette fonctionnelle. */
function commandeSaisie(): FakeCore {
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

// --- Lot E : tests et étapes (docs/plan-mvp-e-tests-etapes.md) -------------------------------

/** Projet avec le test « Créer puis lire un client » (maquette 4.2-a). */
function boutiqueTests(): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  const wb = core.workbench;
  // Les statuts écrits par le run datent d'il y a 5 minutes, comme sur la maquette.
  wb.now = Date.now() - 5 * 60_000;
  const test = "Clients/Créer puis lire un client";
  for (const folder of ["Clients", test, "Clients/Client inconnu", "Catalogue", "Commandes"]) {
    wb.folders.add(folder);
  }
  const check = (id: string, target: string, value: number) => ({
    id,
    target,
    operator: "equals" as const,
    value,
    origin: "manual" as const,
    enabled: true,
  });
  wb.requests.set(
    `${test}/creer.yml`,
    newRequest("Créer le client", {
      method: "POST",
      url: "{{baseUrl}}/customers",
      seq: 1,
      extractions: [{ name: "client.id", source: "body.id", enabled: true }],
      assertions: [check("s", "status", 201), check("d", "responseTime", 500)],
    }),
  );
  wb.requests.set(
    `${test}/lire.yml`,
    newRequest("Lire le client", {
      url: "{{baseUrl}}/customers/{{client.id}}",
      seq: 2,
      assertions: [
        check("s", "status", 200),
        check("i", "body.id", 1),
        check("d", "responseTime", 500),
      ],
    }),
  );
  wb.requests.set(
    `${test}/supprimer.yml`,
    newRequest("Supprimer le client", {
      method: "DELETE",
      url: "{{baseUrl}}/customers/{{client.id}}",
      seq: 3,
      assertions: [check("s", "status", 204)],
    }),
  );
  wb.requests.set(
    "Clients/Client inconnu/lire.yml",
    newRequest("Lire un client inconnu", { url: "{{baseUrl}}/customers/x" }),
  );
  wb.requests.set("Catalogue/produits.yml", newRequest("Lister les produits"));
  wb.requests.set("Commandes/commande.yml", newRequest("Créer une commande", { method: "POST" }));
  wb.requests.set("sante.yml", newRequest("santé", { url: "{{baseUrl}}/health" }));
  return core;
}

/** Requête « Créer une commande » et ses voisines (maquettes 4.2-b bis et 5.1-h). */
function commandesLotE(): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  const wb = core.workbench;
  wb.folders.add("commandes");
  wb.requests.set(
    "commandes/creer.yml",
    newRequest("Créer une commande", {
      method: "POST",
      url: "{{baseUrl}}/orders",
      body: {
        kind: "json",
        variants: [
          {
            title: "",
            content:
              '{\n  "externalRef": "{{$randomUUID}}",\n  "customerId": "cus_31",\n  "customerEmail": "{{$randomEmail}}",\n  "deliveryDate": "{{$date:+3d}}",\n  "requestedAt": "{{$isoTimestamp}}"\n}',
            form: [],
          },
        ],
        selected: 0,
        other: "",
      },
      extractions: [{ name: "order.id", source: "body.id", enabled: true }],
      seq: 1,
    }),
  );
  wb.requests.set(
    "commandes/lire.yml",
    newRequest("Lire une commande", { url: "{{baseUrl}}/orders/{{order.id}}", seq: 2 }),
  );
  wb.requests.set(
    "commandes/statut.yml",
    newRequest("Changer le statut", {
      method: "PATCH",
      url: "{{baseUrl}}/orders/{{order.id}}/status",
      seq: 3,
    }),
  );
  const body =
    '{"id":"ord_8F2kQ","status":"pending","customerId":"cus_31","items":[{"sku":"TSH-BLU-M","qty":2}],"total":59.8,"createdAt":"2026-11-06T09:14Z"}';
  wb.nextSend = okOutcome({
    response: {
      status: 201,
      statusText: "Created",
      headers: [["content-type", "application/json"]],
      body,
      truncated: false,
      sizeBytes: 612,
      waitMs: 40,
      receiveMs: 2,
      totalMs: 42,
    },
    sent: {
      method: "POST",
      url: "https://recette.boutique.dev/orders",
      headers: [["Content-Type", "application/json"]],
      body: '{"externalRef":"7c1e2b04-5d3a-4f6e-9b21-0c8d4e7a91f3","customerId":"cus_31","customerEmail":"lea.martin42@example.com","deliveryDate":"2026-11-15","requestedAt":"2026-11-12T15:02:11Z"}',
      drawn: [
        {
          name: "$randomUUID",
          value: "7c1e2b04-5d3a-4f6e-9b21-0c8d4e7a91f3",
          source: "request.body.externalRef",
        },
        {
          name: "$randomEmail",
          value: "lea.martin42@example.com",
          source: "request.body.customerEmail",
        },
        { name: "$date:+3d", value: "2026-11-15", source: "request.body.deliveryDate" },
        {
          name: "$isoTimestamp",
          value: "2026-11-12T15:02:11Z",
          source: "request.body.requestedAt",
        },
      ],
      used: [{ name: "baseUrl", value: "https://recette.boutique.dev", runtime: false }],
      extracted: [{ name: "order.id", source: "body.id", value: "ord_8F2kQ", message: null }],
    },
  });
  return core;
}

const LOT_E: Record<string, Scenario> = {
  "4.2-a": {
    description: "Test ouvert : étapes numérotées et dernier résultat (4.2-a)",
    setup: boutiqueTests,
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[role="treeitem"]', "Créer puis lire un client");
      await g.cliquer(".test button", "Exécuter le test");
      await g.cliquer(".zone__tab-button", "Créer puis lire un client");
      await g.attendre(100);
    },
  },
  "4.2-b-bis": {
    description: "Onglet « Variables » avec une extraction (4.2-b bis), modifié depuis l'envoi",
    setup: commandesLotE,
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[role="treeitem"]', "Créer une commande");
      await g.cliquer(".editor__line button[type='submit']");
      await g.attendre(100);
      await g.cliquer(".editor__tab", "Variables");
      await g.saisir(".meta__input", "Créer une commande v3");
      await g.attendre(200);
    },
  },
  "5.1-h": {
    description: "« Envoyé » avec les valeurs dynamiques tirées et « Garder » (5.1-h)",
    setup: commandesLotE,
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[role="treeitem"]', "Créer une commande");
      await g.cliquer(".editor__tab", "Corps");
      await g.cliquer(".editor__line button[type='submit']");
      await g.attendre(100);
      await g.cliquer(".response__tab", "Envoyé");
      await g.cliquer(".sent__keep");
      await g.attendre(100);
    },
  },
  "4.1-b": {
    description: "Nouveau test : dossier et étape 1 ouverte, curseur dans l'adresse (4.1-b)",
    setup: boutiqueTests,
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[aria-label="Ajouter au référentiel de tests de boutique-tests"]');
      await g.cliquer('[role="menuitem"]', "Nouveau test");
      await g.attendre(100);
    },
  },
};

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

  // --- Lot D - Barre latérale (docs/plan-mvp-d-barre-laterale.md) -------------
  "1.2-e": {
    description: "Filtre du référentiel : « stat », dossiers dépliés, compte (1.2-e)",
    setup: () => largeur(commandes(), 320),
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[role="treeitem"]', "Changer le statut");
      await g.saisir(".tree .tree-filter input", "stat");
    },
  },
  "1.2-d": {
    description: "Projet créé : sections repliées, quatre points de départ (1.2-d)",
    setup: () => {
      const core = installFakeCore(boutiqueSnapshot());
      core.contracts.counts = { [BOUTIQUE_TESTS]: 0, [AUTH_SSO]: 0 };
      return largeur(core, 320);
    },
    play: ouvrirProjet,
  },
  "1.2-a-ter": {
    description: "Projet vide et menu « + » ouvert, avec ses raccourcis (1.2-a ter)",
    setup: () => largeur(installFakeCore(boutiqueSnapshot()), 320),
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer(".contracts .section-head__toggle");
      await g.cliquer(".tree .section-head__toggle");
      await g.cliquer('[aria-label="Ajouter au projet boutique-tests"]');
    },
  },
  "3.2-a3": {
    description: "Endpoint déplié sur ses réponses attendues, la 201 ouverte (3.2-a3)",
    setup: () => {
      const core = commandes();
      ajouterContratCommandes(core);
      return largeur(core, 320);
    },
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[aria-label="Déplier le contrat API Commandes"]');
      await g.cliquer(".crow__resource", "/orders");
      await g.cliquer('[aria-label="Déplier POST /orders"]');
      await g.cliquer(".crow__example", "Commande créée");
      await g.attendre(100);
    },
  },
  "1.1-a": {
    description: "Workspace « Boutique » : cartes avec leur nombre de contrats (1.1-a)",
    setup: () => {
      const core = boutique();
      core.contracts.counts = { [BOUTIQUE_TESTS]: 3, [AUTH_SSO]: 0 };
      return core;
    },
  },
  "1.2-e-etroite": {
    description: "Filtre et endpoint déplié, barre latérale à sa largeur minimale (200 px)",
    setup: () => {
      const core = commandes();
      ajouterContratCommandes(core);
      return largeur(core, 200);
    },
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer('[aria-label="Déplier le contrat API Commandes"]');
      await g.cliquer(".crow__resource", "/orders");
      await g.cliquer('[aria-label="Déplier POST /orders"]');
      await g.saisir(".tree .tree-filter input", "stat");
    },
  },
  // Lot B : saisie, texte et cURL.
  "5.1-b-ter": {
    description: "Variable inconnue soulignée, compteurs d'onglets, en-têtes ajoutés (5.1-b ter)",
    setup: commandeSaisie,
    play: async (g) => {
      await ouvrirCommande(g);
      await g.cliquer('[role="tab"]', "En-têtes");
      await g.attendre(400);
    },
  },
  "5.1-b-quater": {
    description: "En-têtes édités en texte et copie en cURL (5.1-b quater)",
    setup: commandeSaisie,
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

  // Lot C - Import Bruno (docs/plan-mvp-c-import-bruno.md).
  "2.3-a": {
    description: "Premier lancement : « Importer… » reconnaît le format (2.3-a)",
    setup: () => installFakeCore(firstRunSnapshot()),
  },
  "2.3-b": {
    description: "Projet vide : « Reprendre l'existant », puis la fenêtre « Importer » (2.3-b)",
    setup: () => {
      const core = installFakeCore(boutiqueSnapshot());
      core.imports.sources[BRUNO_DIR] = brunoSource(BRUNO_DIR);
      core.imports.picked = BRUNO_DIR;
      return core;
    },
    play: async (g) => {
      await ouvrirProjet(g);
      await g.cliquer(".start button", "Importer…");
      await g.cliquer("dialog[open] button", "Choisir un dossier…");
      await g.attendre(100);
    },
  },
  "2.3-b-points": {
    description: "Projet vide : le point de départ « Reprendre l'existant » (2.3-b)",
    setup: () => installFakeCore(boutiqueSnapshot()),
    play: ouvrirProjet,
  },
  "2.3-f": {
    description: "Rapport d'import d'une collection Bruno (2.3-f)",
    setup: () => {
      const core = installFakeCore(firstRunSnapshot());
      core.imports.sources[BRUNO_DIR] = brunoSource(BRUNO_DIR);
      core.imports.picked = BRUNO_DIR;
      const wb = core.workbench;
      wb.folders.add("Authentification");
      wb.folders.add("Commandes");
      wb.requests.set(
        "Commandes/creer-une-commande.yml",
        newRequest("Créer une commande", { method: "POST", url: "{{baseUrl}}/orders" }),
      );
      wb.requests.set(
        "Commandes/lire-une-commande.yml",
        newRequest("Lire une commande", { url: "{{baseUrl}}/orders/:id" }),
      );
      wb.requests.set(
        "Authentification/obtenir-un-jeton.yml",
        newRequest("Obtenir un jeton", { method: "POST", url: "{{baseUrl}}/auth/token" }),
      );
      wb.requests.set("sante.yml", newRequest("Santé", { url: "{{baseUrl}}/health" }));
      return core;
    },
    play: async (g) => {
      await g.cliquer(".first-run__import button", "Choisir un dossier…");
      await g.cliquer("dialog[open] button", "Importer");
      await g.attendre(200);
    },
  },
  ...LOT_E,
};

// --- Lot A - Réponse (5.1-b bis, docs/plan-mvp-a-reponse.md) ---------------------------------

const COMMANDE =
  '{"id":"ord_8F2kQ","status":"pending","customerId":"cus_31","items":[{"sku":"TSH-BLU-M","qty":2,"price":19.9},{"sku":"MUG-NOIR","qty":1,"price":12.5}],"coupon":"BIENVENUE","total":47.82}';

/** Projet ouvert, « Créer une commande » envoyée avec la réponse `body` de type `type`. */
function commande(body: string, type: string, status = 201, statusText = "Created"): FakeCore {
  const core = boutique();
  core.workbench.folders.add("Commandes");
  core.workbench.requests.set(
    "Commandes/creer.yml",
    newRequest("Créer une commande", { method: "POST", url: "{{baseUrl}}/orders" }),
  );
  core.workbench.nextSend = okOutcome({
    response: {
      status,
      statusText,
      headers: [["content-type", type]],
      body,
      truncated: false,
      sizeBytes: new TextEncoder().encode(body).length,
      waitMs: 38,
      receiveMs: 4,
      totalMs: 42,
    },
  });
  return core;
}

async function envoyerCommande(g: Gestes): Promise<void> {
  await ouvrirProjet(g);
  await g.cliquer('[role="treeitem"]', "Créer une commande");
  await g.cliquer(".editor__line button[type='submit']");
  await g.attendre(100);
}

async function filtrer(g: Gestes, expression: string): Promise<void> {
  await g.saisir(".filter__input", expression);
  await g.attendre(300);
}

Object.assign(SCENARIOS, {
  "5.1-b-bis": {
    description: "Réponse JSON, filtre JSONPath actif et valeurs surlignées (5.1-b bis)",
    setup: () => commande(COMMANDE, "application/json"),
    play: async (g: Gestes) => {
      await envoyerCommande(g);
      await filtrer(g, "$.items[*].sku");
    },
  },
  "5.1-b-bis-filtre-invalide": {
    description: "Filtre JSONPath invalide, expliqué en français (5.1-b bis)",
    setup: () => commande(COMMANDE, "application/json"),
    play: async (g: Gestes) => {
      await envoyerCommande(g);
      await filtrer(g, "$.items[0");
    },
  },
  "5.1-b-bis-xml": {
    description: "Réponse XML mise en forme (5.1-b bis)",
    setup: () =>
      commande(
        '<?xml version="1.0" encoding="UTF-8"?><commande id="ord_8F2kQ" statut="pending"><client>cus_31</client><articles><article sku="TSH-BLU-M" qte="2">19.90</article><article sku="MUG-NOIR" qte="1">12.50</article></articles><!-- total TTC --><total devise="EUR">47.82</total></commande>',
        "application/xml",
      ),
    play: envoyerCommande,
  },
  "5.1-b-bis-html": {
    description: "Réponse HTML en aperçu, sans script (5.1-b bis)",
    setup: () =>
      commande(
        "<!DOCTYPE html><html><head><title>Erreur</title><script>alert(1)</script></head><body><h1>502 Bad Gateway</h1><p>Le serveur amont ne répond pas.</p></body></html>",
        "text/html; charset=utf-8",
        502,
        "Bad Gateway",
      ),
    play: async (g: Gestes) => {
      await envoyerCommande(g);
      await g.cliquer(".body__bar [role='switch']");
      await g.attendre(100);
    },
  },
} satisfies Record<string, Scenario>);

/** Collection Bruno des scénarios d'import. */
const BRUNO_DIR = "/home/julien/bruno/boutique-api";

/** « Non classés » et l'essai 3 `GET /orders/42/tracking` de son contrat déduit (2.2-b, 2.2-c). */
function essaisNonClasses(): FakeCore {
  const snapshot = boutiqueSnapshot();
  // Le projet ouvert tient lieu de « Non classés » : ses essais montrent « Requête au point ? ».
  snapshot.inboxProjectId = BOUTIQUE_TESTS;
  const core = installFakeCore(snapshot);
  const file = "contracts/api-boutique-dev.yaml";
  const op = (path: string) => ({
    key: `GET ${path}`,
    method: "GET",
    path,
    summary: null,
    tag: null,
  });
  core.contracts.files.set(file, {
    name: "api.boutique.dev",
    version: "0.0.0",
    deduced: true,
    source: "openapi: 3.1.0\n",
    operations: [op("/products"), op("/orders"), op("/orders/42/tracking")],
  });
  const wb = core.workbench;
  wb.folders.add("api.boutique.dev");
  wb.folders.add("commandes");
  wb.requests.set("commandes/lister.yml", newRequest("Lister les commandes", { seq: 1 }));
  const essai = (n: number, path: string) =>
    wb.requests.set(
      `api.boutique.dev/essai-${String(n)}.yml`,
      newRequest(`Essai ${String(n)}`, {
        url: `https://api.boutique.dev${path}`,
        contract: { file, operation: `GET ${path}` },
      }),
    );
  essai(1, "/products");
  essai(2, "/orders");
  essai(3, "/orders/42/tracking");
  return core;
}

async function ouvrirEssai3(g: Gestes): Promise<void> {
  await ouvrirProjet(g);
  await g.cliquer('[aria-label="Déplier le contrat api.boutique.dev"]');
  await g.cliquer(".crow__main", "/orders");
  await g.cliquer(".crow__main", "/orders/42/tracking");
  await g.attendre(150);
}

Object.assign(SCENARIOS, {
  "2.2-c": {
    description: "Essai dont le chemin ressemble à /orders/{id}/tracking : proposition (2.2-c)",
    setup: essaisNonClasses,
    play: ouvrirEssai3,
  },
  "2.2-b": {
    description: "Fenêtre « Transférer au référentiel de tests » d'un essai (2.2-b)",
    setup: essaisNonClasses,
    play: async (g: Gestes) => {
      await ouvrirEssai3(g);
      await g.cliquer(".trial-transfer button", "Transférer au référentiel de tests");
      await g.saisir(".transfer input", "Suivre une commande");
      await g.attendre(100);
    },
  },
} satisfies Record<string, Scenario>);
