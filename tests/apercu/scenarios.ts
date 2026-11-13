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
