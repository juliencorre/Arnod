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
function commandes(): FakeCore {
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
    setup: commandes,
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
    setup: commandes,
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
  ...LOT_E,
};
