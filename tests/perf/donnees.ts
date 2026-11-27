/**
 * Données de mesure : gonfle le faux cœur (`tests/fakeCore.ts`) pour l'audit de performance de
 * l'interface. Un projet de `total` requêtes : dossiers, tests de 8 étapes, contrats dont un gros
 * avec ses requêtes liées, un dossier de 200 requêtes ; puis de quoi mesurer l'éditeur (100
 * en-têtes, corps de 200 ko), la réponse (corps JSON ou HTML d'une taille donnée), un run et 40
 * onglets dans 4 zones.
 *
 * Les commandes dont le faux cœur recalcule tout à chaque appel (`tree_load`, `contract_list`…)
 * sont figées en JSON et relues par `JSON.parse` : le coût du faux cœur sort des mesures, celui de
 * la désérialisation (que paie aussi la vraie application) y reste.
 */
import type {
  CatalogSnapshot,
  ContentTab,
  LayoutNode,
  Pair,
  RequestDoc,
  RequestStatuses,
  RunItem,
  RunView,
  SendOutcome,
} from "@/types/ipc";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { handleContracts } from "../fakeContracts";
import { handleResponse } from "../fakeResponse";
import { tab } from "../fakeWindows";
import { groupsOf, handleWorkbench, newRequest, okOutcome, runOf, testsOf } from "../fakeWorkbench";
import { project, workspace } from "../fixtures";

export const PROJET = "p-gros";
export const ESPACE = "ws-perf";
/** Dossier de 200 requêtes, replié et déplié par les mesures. */
export const DOSSIER_200 = "Catalogue/Produits";
/** Requête de l'éditeur : 100 en-têtes, corps JSON de 200 ko. */
export const GROSSE = "Catalogue/grosse-requete.yml";
/** Requête dont l'envoi renvoie le corps à afficher. */
export const REPONSE = "Catalogue/reponse.yml";

export function instantane(): CatalogSnapshot {
  return {
    projects: [project(PROJET, "gros-projet", { workspaceIds: [ESPACE] })],
    workspaces: [workspace(ESPACE, "Perf", { projectIds: [PROJET] })],
    lastWorkspaceId: ESPACE,
    inboxProjectId: null,
  };
}

const METHODES = ["GET", "POST", "PUT", "PATCH", "DELETE"] as const;
const VERBES = ["Lister", "Lire", "Créer", "Modifier", "Supprimer", "Chercher", "Valider"];
const NOMS = [
  "commandes",
  "clients",
  "paiements",
  "livraisons",
  "factures",
  "stocks",
  "retours",
  "avis",
  "paniers",
  "catalogues",
];
const ETIQUETTES = ["smoke", "critique", "v2", "lent", "régression"];

function requete(n: number, domaine: string, extra: Partial<RequestDoc> = {}): RequestDoc {
  const verbe = VERBES[n % VERBES.length] ?? "Lire";
  const methode = METHODES[n % METHODES.length] ?? "GET";
  return newRequest(`${verbe} ${domaine} ${String(n)}`, {
    method: methode,
    url: `{{baseUrl}}/api/v2/${domaine}/${String(n)}?page={{page}}`,
    tags: n % 3 === 0 ? [ETIQUETTES[n % ETIQUETTES.length] ?? "smoke"] : [],
    ...extra,
  });
}

export interface Projet {
  /** Nombre de requêtes du projet. */
  total: number;
  /** Lignes attendues dans le référentiel de tests, tout déplié (dossiers et requêtes). */
  lignes: number;
  dossiers: string[];
}

/** Remplit le faux cœur d'un projet de `total` requêtes. */
export function projet(core: FakeCore, total: number): Projet {
  const wb = core.workbench;
  const grand = total >= 2000;
  const operations = grand ? 300 : 75;
  const petites = grand ? 10 : 3;
  const tests = grand ? 30 : 8;
  const domaines = grand ? 10 : 5;
  const sous = grand ? 9 : 4;
  wb.envs = [
    {
      name: "Recette",
      color: "orange",
      protected: false,
      variables: [
        { name: "baseUrl", value: "https://recette.boutique.dev" },
        { name: "page", value: "1" },
        { name: "token", value: "abc" },
      ],
      locals: [],
    },
    {
      name: "Local",
      color: "gray",
      protected: false,
      variables: [{ name: "baseUrl", value: "" }],
      locals: [],
    },
  ];
  wb.active = "Recette";
  wb.now = Date.now();
  let n = 0;
  let reste = total;
  const pose = (path: string, doc: RequestDoc): void => {
    wb.requests.set(path, doc);
    reste -= 1;
  };

  // Gros contrat et ses requêtes liées, puis neuf petits.
  const contrat = (index: number, nom: string, nombre: number): void => {
    const file = `contracts/${nom.toLowerCase().replaceAll(" ", "-")}.yaml`;
    const ops = Array.from({ length: nombre }, (_, i) => {
      const ressource = `${NOMS[Math.floor(i / 10) % NOMS.length] ?? "x"}${String(Math.floor(i / 100) + 1)}`;
      const method = METHODES[i % METHODES.length] ?? "GET";
      const path =
        i % 2 === 0 ? `/${ressource}/op${String(i)}` : `/${ressource}/{id}/op${String(i)}`;
      return {
        key: `${method} ${path}`,
        method,
        path,
        summary: `Opération ${String(i)}`,
        tag: ressource,
      };
    });
    core.contracts.files.set(file, {
      name: nom,
      version: `${String(index + 1)}.0.0`,
      source: `openapi: 3.0.3\ninfo:\n  title: ${nom}\n  version: 1.0.0\n`,
      operations: ops,
    });
    wb.folders.add(nom);
    for (const op of ops) {
      wb.folders.add(`${nom}/${op.tag}`);
      n += 1;
      pose(
        `${nom}/${op.tag}/op-${String(n)}.yml`,
        requete(n, op.tag, {
          name: op.summary,
          method: op.method,
          url: `{{baseUrl}}${op.path}`,
          contract: { file, operation: op.key },
          examples:
            n % 2 === 0
              ? [
                  {
                    name: "Succès",
                    description: "",
                    status: 200,
                    statusText: "OK",
                    headers: [],
                    bodyKind: "json",
                    body: "{}",
                  },
                  {
                    name: "Introuvable",
                    description: "",
                    status: 404,
                    statusText: "Not Found",
                    headers: [],
                    bodyKind: "json",
                    body: "{}",
                  },
                ]
              : [],
        }),
      );
    }
  };
  contrat(0, "API Boutique", operations);
  for (let c = 1; c < 10; c++) contrat(c, `API Service ${String(c)}`, petites);

  // Tests de 8 étapes.
  wb.folders.add("Parcours");
  for (let t = 1; t <= tests; t++) {
    const dossier = `Parcours/Parcours ${String(t).padStart(2, "0")}`;
    wb.folders.add(dossier);
    for (let e = 1; e <= 8; e++) {
      n += 1;
      pose(`${dossier}/etape-${String(e)}.yml`, requete(n, "parcours", { seq: e }));
    }
  }

  // Dossier de 200 requêtes : un sous-dossier en fait un dossier, pas un test (un dossier sans
  // sous-dossier est un test, lot E).
  wb.folders.add("Catalogue");
  wb.folders.add(DOSSIER_200);
  wb.folders.add(`${DOSSIER_200}/Archives`);
  for (let i = 0; i < 3; i++) {
    n += 1;
    pose(`${DOSSIER_200}/Archives/ancien-${String(i)}.yml`, requete(n, "produits"));
  }
  for (let i = 0; i < 200; i++) {
    n += 1;
    pose(`${DOSSIER_200}/produit-${String(i).padStart(3, "0")}.yml`, requete(n, "produits"));
  }

  // Le reste, réparti dans les autres dossiers.
  const dossiers: string[] = [];
  for (let d = 0; d < domaines; d++) {
    const haut = `${String(d + 1).padStart(2, "0")} ${NOMS[d % NOMS.length] ?? "x"}`;
    wb.folders.add(haut);
    for (let s = 0; s < sous; s++) {
      const dossier = `${haut}/${NOMS[(d + s + 1) % NOMS.length] ?? "y"} ${String(s + 1)}`;
      wb.folders.add(dossier);
      dossiers.push(dossier);
    }
  }
  // Une requête sur trois directement dans son domaine (requête libre), les autres dans un
  // sous-dossier (qui, sans sous-dossier, est un test).
  for (let i = 0; reste > 0; i++) {
    n += 1;
    const feuille = dossiers[i % dossiers.length] ?? "Catalogue";
    const dossier = i % 3 === 0 ? (feuille.split("/")[0] ?? feuille) : feuille;
    pose(`${dossier}/requete-${String(n)}.yml`, requete(n, feuille.split("/")[0]?.slice(3) ?? "x"));
  }

  // Statuts : tout a été exécuté, une requête sur dix en échec.
  let k = 0;
  for (const path of wb.requests.keys()) {
    wb.statuses[path] = { passed: k % 10 !== 0, at: (wb.now ?? 1) - k * 1000 };
    k += 1;
  }
  const hors = operations + 9 * petites;
  const dossiersVus = 1 + tests + 3 + domaines + domaines * sous;
  return { total, lignes: total - hors + dossiersVus, dossiers: [...wb.folders] };
}

/** Statuts du projet dont `nombre` ont changé (résultat inversé, heure nouvelle). */
export function statutsChanges(core: FakeCore, nombre: number, tour: number): RequestStatuses {
  const statuses: RequestStatuses = JSON.parse(JSON.stringify(core.workbench.statuses));
  let i = 0;
  for (const path of Object.keys(statuses)) {
    if (i >= nombre) break;
    // Seulement des requêtes du référentiel de tests, affichées dans la barre latérale.
    if (path.startsWith("API ")) continue;
    const status = statuses[path];
    if (status) statuses[path] = { passed: (i + tour) % 2 === 0, at: status.at + tour + 1 };
    i += 1;
  }
  return statuses;
}

// --- Éditeur ---------------------------------------------------------------------

function entetes(nombre: number, inconnues: boolean): Pair[] {
  return Array.from({ length: nombre }, (_, i) => ({
    name: `X-En-Tete-${String(i).padStart(3, "0")}`,
    value:
      i % 5 === 0
        ? inconnues
          ? `Bearer {{jeton${String(i)}}}`
          : "Bearer {{token}}"
        : `valeur-${String(i)}-${"x".repeat(i % 17)}`,
    enabled: i % 9 !== 8,
    kind: "query" as const,
    description: i % 4 === 0 ? `Description de l'en-tête ${String(i)}` : "",
  }));
}

/** Corps JSON d'environ `ko` kilo-octets, comme une liste de commandes. */
export function corpsJson(ko: number, inconnue = false, joli = false, nombre = 0): string {
  const articles: unknown[] = [];
  let taille = 0;
  for (let i = 0; nombre > 0 ? i < nombre : taille < ko * 1024; i++) {
    const article = {
      id: `ord_${String(100000 + i)}`,
      sku: `SKU-${String(i).padStart(6, "0")}`,
      name: `Article ${String(i)} en coton biologique`,
      price: Math.round((9.9 + (i % 97) * 1.37) * 100) / 100,
      quantity: (i % 5) + 1,
      inStock: i % 3 !== 0,
      tags: ["textile", i % 2 === 0 ? "été" : "hiver"],
      customer: { id: `cus_${String(i % 311)}`, email: `client${String(i % 311)}@boutique.dev` },
      note: i === 3 && inconnue ? "{{inconnue}}" : null,
    };
    taille += JSON.stringify(article, null, joli ? 2 : 0).length + (joli ? 60 : 1);
    articles.push(article);
  }
  return JSON.stringify({ total: articles.length, items: articles }, null, joli ? 2 : 0);
}

/** Corps HTML d'environ `ko` kilo-octets : titres, paragraphes et listes. */
export function corpsHtml(ko: number): string {
  const parts = ["<!DOCTYPE html><html><head><title>Rapport</title></head><body>"];
  let taille = 0;
  for (let i = 0; taille < ko * 1024; i++) {
    const bloc =
      `<h2>Section ${String(i)}</h2><p>Le serveur amont a répondu en ${String(i % 900)} ms pour la ` +
      `commande ${String(i)}, avec un statut inattendu et un message détaillé.</p>` +
      `<ul><li>Premier point ${String(i)}</li><li>Second point ${String(i)}</li></ul>`;
    taille += bloc.length;
    parts.push(bloc);
  }
  parts.push("</body></html>");
  return parts.join("");
}

function reponse(body: string, type: string): SendOutcome {
  return okOutcome({
    response: {
      status: 200,
      statusText: "OK",
      headers: [["content-type", type]],
      body,
      truncated: false,
      sizeBytes: body.length,
      waitMs: 112,
      receiveMs: 6,
      totalMs: 118,
      timeline: {
        dnsMs: null,
        connectMs: null,
        secure: false,
        serverMs: 112,
        receiveMs: 6,
        totalMs: 118,
      },
    },
  });
}

/** Ajoute la grosse requête de l'éditeur ; `inconnues` : des variables inconnues à souligner. */
export function grosseRequete(core: FakeCore, inconnues: boolean): void {
  core.workbench.folders.add("Catalogue");
  core.workbench.requests.set(
    GROSSE,
    newRequest("Grosse requête", {
      method: "POST",
      url: inconnues
        ? "{{baseUrl}}/api/{{version}}/orders/{{orderId}}"
        : "{{baseUrl}}/api/v2/orders/{{page}}",
      headers: entetes(100, inconnues),
      body: {
        kind: "json",
        variants: [{ title: "", content: corpsJson(200, inconnues, true), form: [] }],
        selected: 0,
        other: "",
      },
    }),
  );
}

/** Ajoute la requête dont l'envoi renvoie `body`. */
export function requeteReponse(core: FakeCore, body: string, type: string): void {
  core.workbench.folders.add("Catalogue");
  core.workbench.requests.set(
    REPONSE,
    newRequest("Lister les commandes", { url: "{{baseUrl}}/api/v2/orders" }),
  );
  core.workbench.nextSend = reponse(body, type);
}

// --- Run --------------------------------------------------------------------------

/** Run de tout le projet, toutes ses lignes en attente (`fini` : toutes terminées). */
export function run(core: FakeCore, fini: boolean): RunView {
  const view = runOf(core.workbench, "");
  view.id = "run-perf";
  view.items.forEach((item, index) => {
    if (fini) terminer(item, index);
    else {
      item.status = "pending";
      item.httpStatus = null;
      item.durationMs = null;
    }
  });
  view.finished = fini;
  view.finishedAt = fini ? view.startedAt + 41000 : null;
  view.groups = groupsOf("", view.items);
  view.tests = testsOf(view.items);
  return view;
}

/** Termine une ligne : une sur dix en échec avec une assertion, une sur sept lente. */
export function terminer(item: RunItem, index: number): void {
  const echec = index % 10 === 3;
  item.status = echec ? "failed" : "passed";
  item.httpStatus = echec ? 500 : 200;
  item.durationMs = 40 + ((index * 37) % 900);
  item.slow = index % 7 === 0;
  item.assertions = echec
    ? [
        {
          id: "statut",
          target: "status",
          passed: false,
          message: "attendu 200, reçu 500",
          actual: "500",
          ignored: false,
        },
      ]
    : [];
}

// --- Onglets ------------------------------------------------------------------------

/** 4 zones de `parZone` onglets de requêtes, dans la fenêtre principale. */
export function zones(core: FakeCore, parZone: number): void {
  const chemins = [...core.workbench.requests.keys()].filter((p) => !p.startsWith("API "));
  let prochain = 0;
  let compteur = 0;
  const zone = (): LayoutNode => {
    const tabs: ContentTab[] = Array.from({ length: parZone }, () => {
      const path = chemins[(prochain += 37) % chemins.length] ?? "";
      return tab({ kind: "request", projectId: PROJET, path });
    });
    return {
      kind: "zone",
      id: `zone-perf-${String(++compteur)}`,
      tabs,
      activeTab: tabs[0]?.id ?? null,
      placement: "auto",
      recent: tabs.map((t) => t.id),
    };
  };
  const split = (axis: "row" | "column", first: LayoutNode, second: LayoutNode): LayoutNode => ({
    kind: "split",
    id: `separation-perf-${String(++compteur)}`,
    axis,
    ratio: 0.5,
    first,
    second,
  });
  const [a, b, c, d] = [zone(), zone(), zone(), zone()];
  const main = core.windows.windows[0];
  if (!main) return;
  main.layout = {
    root: split("row", split("column", a, b), split("column", c, d)),
    activeZone: a.kind === "zone" ? a.id : "",
  };
}

// --- Commandes figées ---------------------------------------------------------------

export interface Compteur {
  /** Temps passé dans le faux cœur (retiré des mesures), en ms. */
  faux: number;
  /** Temps de `JSON.parse` des réponses figées (gardé dans les mesures), en ms. */
  parse: number;
  appels: Record<string, number>;
  /** Temps du faux cœur par commande. */
  parCommande: Record<string, number>;
  /** Appels au faux cœur : commande, début et fin de sa réponse. */
  journal: { cmd: string; debut: number; fin: number }[];
  /** Taille en JSON de la dernière réponse des commandes sérialisées. */
  octets: Record<string, number>;
  /** Oublie les réponses figées (les données ont changé). */
  vider: () => void;
}

const FIGEES = new Set(["tree_load", "contract_list", "request_statuses"]);
/** Réponses passées par JSON, comme sur le vrai canal : leur lecture reste dans les mesures. */
const SERIALISEES = new Set(["response_render", "response_filter", "response_preview"]);
/** Commandes dont le faux cœur diffuse un événement en répondant : son temps n'est pas retiré. */
const DIFFUSENT = new Set([
  "request_send",
  "run_start",
  "run_retry_failures",
  "run_ignore_check",
  "result_ignore_check",
  "tree_batch",
  "contract_delete",
]);

interface Internals {
  invoke: (cmd: string, args?: Record<string, unknown>, options?: unknown) => Promise<unknown>;
}

/** Fige les commandes coûteuses du faux cœur et compte son temps. */
export function figer(core: FakeCore): Compteur {
  const internals = (window as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;
  const base = internals.invoke;
  const cache = new Map<string, string>();
  const compteur: Compteur = {
    faux: 0,
    parse: 0,
    appels: {},
    parCommande: {},
    journal: [],
    octets: {},
    vider: () => {
      cache.clear();
    },
  };
  internals.invoke = (cmd, args, options) => {
    if (cmd.startsWith("plugin:")) return base(cmd, args, options);
    compteur.appels[cmd] = (compteur.appels[cmd] ?? 0) + 1;
    const t0 = performance.now();
    if (FIGEES.has(cmd)) {
      const a = args ?? {};
      const cle = `${cmd}:${JSON.stringify(a.project ?? "")}`;
      let json = cache.get(cle);
      if (json === undefined) {
        const valeur =
          handleContracts(core.contracts, core.workbench, cmd, a) ??
          handleWorkbench(core.workbench, cmd, a);
        json = JSON.stringify(valeur);
        cache.set(cle, json);
        const passe = performance.now() - t0;
        compteur.faux += passe;
        compteur.parCommande[cmd] = (compteur.parCommande[cmd] ?? 0) + passe;
      }
      const t1 = performance.now();
      const valeur = JSON.parse(json) as Record<string, unknown>;
      // L'état replié change sans que l'arborescence soit recalculée.
      if (cmd === "tree_load") valeur.collapsed = [...core.workbench.collapsed];
      compteur.parse += performance.now() - t1;
      core.calls.push({ cmd, args: a });
      compteur.journal.push({ cmd, debut: t0, fin: performance.now() });
      return Promise.resolve(valeur);
    }
    if (SERIALISEES.has(cmd)) {
      const brut = handleResponse(core.response, core.workbench, cmd, args ?? {});
      const json = JSON.stringify(brut);
      const t1 = performance.now();
      compteur.faux += t1 - t0;
      compteur.parCommande[cmd] = (compteur.parCommande[cmd] ?? 0) + t1 - t0;
      const lu: unknown = JSON.parse(json);
      compteur.parse += performance.now() - t1;
      compteur.octets[cmd] = json.length;
      compteur.journal.push({ cmd, debut: t0, fin: performance.now() });
      return Promise.resolve(lu);
    }
    const reponse = base(cmd, args, options);
    if (!DIFFUSENT.has(cmd)) {
      const passe = performance.now() - t0;
      compteur.faux += passe;
      compteur.parCommande[cmd] = (compteur.parCommande[cmd] ?? 0) + passe;
    }
    compteur.journal.push({ cmd, debut: t0, fin: performance.now() });
    return reponse;
  };
  return compteur;
}

export interface Preparation {
  core: FakeCore;
  compteur: Compteur;
  projet: Projet;
}

/**
 * Prépare le faux cœur du scénario `nom` à la taille `taille` :
 * `barre` (taille : requêtes), `editeur` (taille : requêtes ; `inconnues`), `reponse` (taille :
 * ko du corps JSON), `html` (taille : ko), `run` (taille : requêtes), `onglets` (taille : requêtes).
 */
export function preparer(nom: string, taille: number, options: URLSearchParams): Preparation {
  const core = installFakeCore(instantane());
  const largeur = Number(options.get("barre") ?? "320");
  const main = core.windows.windows[0];
  if (main) main.sidebarWidth = largeur;
  let infos: Projet;
  if (nom === "reponse" || nom === "html") {
    infos = projet(core, 500);
    requeteReponse(
      core,
      nom === "html"
        ? corpsHtml(taille)
        : corpsJson(taille, false, false, Number(options.get("articles") ?? "0")),
      nom === "html" ? "text/html; charset=utf-8" : "application/json",
    );
  } else {
    infos = projet(core, taille);
    if (nom === "editeur") grosseRequete(core, options.has("inconnues"));
    if (nom === "onglets") zones(core, Number(options.get("parZone") ?? "10"));
  }
  const compteur = figer(core);
  // Réponses figées calculées avant le montage : le faux cœur ne pèse pas sur le premier rendu.
  const internals = (window as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;
  for (const cmd of FIGEES) void internals.invoke(cmd, { project: PROJET });
  compteur.appels = {};
  compteur.parse = 0;
  compteur.journal.length = 0;
  core.calls.length = 0;
  return { core, compteur, projet: infos };
}
