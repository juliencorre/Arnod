/**
 * Faux cœur de l'import (lot C, docs/plan-mvp-c-import-bruno.md) : sources reconnues par chemin,
 * import simulé et rapports gardés par projet. La conversion réelle est testée côté Rust.
 */
import type {
  CatalogSnapshot,
  CommandError,
  ImportReport,
  ImportSource,
  ImportTarget,
} from "@/types/ipc";

export interface FakeImports {
  /** Chemin renvoyé par le prochain `import_pick` (`null` : annulé). */
  picked: string | null;
  /** Sources reconnues, par chemin ; un chemin absent n'est pas reconnu. */
  sources: Record<string, ImportSource>;
  /** Dernier rapport par projet. */
  reports: Record<string, ImportReport>;
}

export function createFakeImports(): FakeImports {
  return { picked: null, sources: {}, reports: {} };
}

/** Collection Bruno « Boutique API » (comme la collection d'exemple des tests Rust). */
export function brunoSource(path = "/home/julien/bruno/boutique-api"): ImportSource {
  return {
    path,
    format: "bruno",
    name: "Boutique API",
    counts: {
      folders: 2,
      requests: 6,
      environments: 2,
      assertions: 10,
      extractions: 2,
      secrets: 3,
      dynamicValues: 2,
      scripts: 3,
    },
    files: 12,
  };
}

/** Rapport d'import de « Boutique API ». */
export function brunoReport(
  projectId: string,
  projectName: string,
  folder: string | null = null,
): ImportReport {
  const at = (path: string) => (folder ? `${folder}/${path}` : path);
  return {
    collection: "Boutique API",
    format: "bruno",
    source: "/home/julien/bruno/boutique-api",
    sourceFiles: 12,
    projectId,
    projectName,
    folder,
    importedAt: Date.UTC(2026, 10, 13, 14, 2),
    counts: brunoSource().counts,
    items: [
      {
        kind: "tests",
        status: "notRun",
        name: "Créer une commande",
        place: "Commandes",
        file: "Commandes/Créer une commande.bru",
        block: "tests",
        request: at("Commandes/creer-une-commande.yml"),
        detail:
          "Aucun JavaScript n'est exécuté par Arnod : ce script n'a pas d'équivalent sans code.",
        code: 'test("total cohérent", function () {\n  const o = res.getBody();\n  const sum = o.items.reduce(\n    (s, i) => s + i.price * i.qty, 0);\n  expect(o.total).to.equal(sum);\n});',
      },
      {
        kind: "preRequest",
        status: "notRun",
        name: "Commandes",
        place: "réglages du dossier",
        file: "Commandes/folder.bru",
        block: "script:pre-request",
        request: null,
        detail:
          "Aucun JavaScript n'est exécuté par Arnod : ce script n'a pas d'équivalent sans code.",
        code: 'bru.setVar("start", Date.now());',
      },
      {
        kind: "postResponse",
        status: "notRun",
        name: "Obtenir un jeton",
        place: "Authentification",
        file: "Authentification/Obtenir un jeton.bru",
        block: "script:post-response",
        request: at("Authentification/obtenir-un-jeton.yml"),
        detail:
          "Aucun JavaScript n'est exécuté par Arnod : ce script n'a pas d'équivalent sans code.",
        code: 'bru.setEnvVar("token", res.body.access_token);',
      },
      {
        kind: "env",
        status: "toFill",
        name: "token, password",
        place: "Recette",
        file: "environments/Recette.bru",
        block: "vars:secret",
        request: null,
        detail:
          "Valeurs gardées par Bruno hors de la collection : variables locales à renseigner, jamais dans le projet.",
        code: null,
      },
      {
        kind: "auth",
        status: "skipped",
        name: "Obtenir un jeton",
        place: "Authentification",
        file: "Authentification/Obtenir un jeton.bru",
        block: "auth:oauth2",
        request: at("Authentification/obtenir-un-jeton.yml"),
        detail: "Authentification OAuth 2 non reprise : à régler dans l'onglet Auth.",
        code: "grant_type: client_credentials\nclient_id: arnod",
      },
      {
        kind: "request",
        status: "skipped",
        name: "Profil",
        place: "Authentification",
        file: "Authentification/Profil.bru",
        block: null,
        request: null,
        detail: "Requête GraphQL : non prise en charge par Arnod, non importée.",
        code: null,
      },
    ],
    examples: [
      { before: "assert { res.status: eq 201 }", after: "status égal à 201" },
      {
        before: "vars:post-response { orderId: res.body.id }",
        after: "extrait orderId ← body.id",
      },
      { before: "{{$randomUUID}}", after: "valeur dynamique, même nom" },
      {
        before: "vars:secret [ token, password ]",
        after: "variables locales, à renseigner (coffre du système)",
      },
    ],
  };
}

function fail(kind: CommandError["kind"], message: string): never {
  throw { kind, message } satisfies CommandError;
}

/** Inscrit un projet dans le faux catalogue. */
export type Register = (
  name: string,
  path: string,
  workspaceId: string | null,
) => { snapshot: CatalogSnapshot; projectId: string };

/** Répond aux commandes `import_*` ; `undefined` pour une autre commande. */
export function handleImports(
  fi: FakeImports,
  cmd: string,
  args: Record<string, unknown>,
  register: Register,
  projectName: (id: string) => string,
  defaultLocation: string,
): unknown {
  switch (cmd) {
    case "import_pick":
      return fi.picked;
    case "import_detect": {
      const source = fi.sources[String(args.path)];
      if (!source) {
        fail(
          "notAProject",
          `Arnod ne reconnaît pas ${String(args.path)} : choisissez une collection Bruno (dossier avec bruno.json), un projet Arnod ou OpenCollection, ou un contrat OpenAPI.`,
        );
      }
      return structuredClone(source);
    }
    case "import_run": {
      const source = fi.sources[String(args.path)] ?? fail("notAProject", "Format inconnu.");
      const target = args.target as ImportTarget;
      if (source.format === "openCollection" || target.kind === "newProject") {
        const workspaceId = target.kind === "newProject" ? target.workspaceId : null;
        const location = target.kind === "newProject" ? target.location : null;
        const path =
          source.format === "openCollection"
            ? source.path
            : `${location ?? defaultLocation}/${source.name}`;
        const created = register(source.name, path, workspaceId);
        const report =
          source.format === "bruno" ? brunoReport(created.projectId, source.name) : null;
        if (report) fi.reports[created.projectId] = report;
        return {
          projectId: created.projectId,
          snapshot: created.snapshot,
          report,
          contract: source.format === "openApi" ? "contracts/petstore.yaml" : null,
        };
      }
      const report =
        source.format === "bruno"
          ? brunoReport(target.projectId, projectName(target.projectId), source.name)
          : null;
      if (report) fi.reports[target.projectId] = report;
      return {
        projectId: target.projectId,
        snapshot: null,
        report,
        contract: source.format === "openApi" ? "contracts/petstore.yaml" : null,
      };
    }
    case "import_report":
      return structuredClone(fi.reports[String(args.projectId)] ?? null);
    default:
      return undefined;
  }
}
