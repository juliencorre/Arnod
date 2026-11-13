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
      await g.saisir(".tree .filter input", "stat");
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
      await g.saisir(".tree .filter input", "stat");
    },
  },
};
