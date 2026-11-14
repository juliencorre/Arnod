/**
 * Faux cœur Rust pour les tests de composants : répond aux commandes IPC en
 * mémoire, en appliquant une version simplifiée des règles R1 à R12 (le vrai
 * comportement est testé côté Rust). Les appels sont notés dans `calls`.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

import type { CatalogSnapshot, CommandError, WorkspaceColor } from "@/types/ipc";

import { createFakeContracts, handleContracts, type FakeContracts } from "./fakeContracts";
import {
  closePath,
  createFakeWindows,
  handleWindows,
  snapshotOf,
  type FakeWindows,
} from "./fakeWindows";
import { handleInput } from "./fakeInput";
import { handleTrials } from "./fakeTrials";
import { handleTransfer } from "./fakeTransfer";
import { createFakeUpdates, type FakeUpdates, handleUpdates } from "./fakeUpdates";
import { STEP_CATALOG, detect } from "./fakeSteps";
// Lot C - Import Bruno.
import { createFakeImports, handleImports, type FakeImports } from "./fakeImports";
import {
  createFakeWorkbench,
  handleWorkbench,
  newRequest,
  okOutcome,
  type FakeWorkbench,
} from "./fakeWorkbench";
import { createFakeResponse, handleResponse, type FakeResponse } from "./fakeResponse";
// Lot H - Réseau et authentification.
import { createFakeNetwork, handleNetwork, type FakeNetwork } from "./fakeNetwork";
// Lot I - Fichiers modifiés hors d'Arnod.
import { handleFilesLotI } from "./fakeFiles";
import { createFakeStartup, handleStartupLotG, type FakeStartup } from "./fakeStartup";

export const HOME = "/home/julien";
export const DEFAULT_LOCATION = `${HOME}/arnod`;
/** Identifiant du projet « Non classés » créé par le premier essai. */
export const INBOX = "p-non-classes";

export interface FakeCore {
  calls: { cmd: string; args: Record<string, unknown> }[];
  snapshot: () => CatalogSnapshot;
  /** Dossier renvoyé par le prochain `project_pick_folder` (`null` : annulé). */
  pickedFolder: string | null;
  /** Projets dont le dossier a disparu (P10). */
  missing: Set<string>;
  commands: () => string[];
  /** Fait échouer le prochain appel de `cmd` avec `error`. */
  failNext: (cmd: string, error: CommandError) => void;
  /** État du faux cœur du premier usage (arborescence, requêtes, environnements, runs). */
  workbench: FakeWorkbench;
  /** Fenêtres et zones ; `windows.label` est la fenêtre servie par l'interface. */
  windows: FakeWindows;
  /** Contrats de services, communs à tous les projets. */
  contracts: FakeContracts;
  /** Diffuse un évènement du cœur (`arnod://…`) aux abonnés de l'interface. */
  emit: (event: string, payload: unknown) => Promise<void>;
  /** Diffuse l'état des fenêtres, comme après une opération faite ailleurs. */
  emitWindows: () => Promise<void>;
  /** Corps de la réponse (lot A) : fichier du prochain enregistrement. */
  response: FakeResponse;
  /** Lot C - Import Bruno : sources reconnues, rapports. */
  imports: FakeImports;
  /** Lot H - Réseau et authentification : préférences, authentification du projet, cookies. */
  network: FakeNetwork;
  /** Lot G - Démarrage et erreurs : erreur bloquante, démarrage lent. */
  startup: FakeStartup;
  /** Lot L allégé - Mises à jour : préférences, version trouvée. */
  updates: FakeUpdates;
}

const COLORS: WorkspaceColor[] = [
  "orange",
  "blue",
  "green",
  "purple",
  "red",
  "yellow",
  "teal",
  "gray",
];

function fail(kind: CommandError["kind"], message: string): never {
  throw { kind, message } satisfies CommandError;
}

export function installFakeCore(initial: CatalogSnapshot): FakeCore {
  let state: CatalogSnapshot = structuredClone(initial);
  const undo = new Map<string, CatalogSnapshot>();
  let counter = 0;
  const failures = new Map<string, CommandError>();
  const core: FakeCore = {
    calls: [],
    snapshot: () => structuredClone(state),
    pickedFolder: null,
    missing: new Set(),
    commands: () => core.calls.map((call) => call.cmd),
    failNext: (cmd, error) => {
      failures.set(cmd, error);
    },
    workbench: createFakeWorkbench(),
    windows: createFakeWindows(),
    contracts: createFakeContracts(),
    emit: (event, payload) => emit(event, payload),
    emitWindows: () => emit("arnod://fenetres", snapshotOf(core.windows)),
    response: createFakeResponse(),
    imports: createFakeImports(),
    network: createFakeNetwork(),
    startup: createFakeStartup(),
    updates: createFakeUpdates(),
  };

  const same = (a: string, b: string): boolean => a.toLowerCase() === b.toLowerCase();
  const workspace = (id: unknown) =>
    state.workspaces.find((w) => w.id === id) ?? fail("notFound", "Ce workspace n'existe plus.");
  const project = (id: unknown) =>
    state.projects.find((p) => p.id === id) ??
    fail("notFound", "Ce projet n'existe plus dans le référentiel.");

  /** Recalcule les champs dérivés, comme l'instantané Rust (onglets dans l'ordre de création, R11). */
  function normalize(): CatalogSnapshot {
    for (const p of state.projects) {
      p.workspaceIds = state.workspaces.filter((w) => w.projectIds.includes(p.id)).map((w) => w.id);
      p.status = core.missing.has(p.id) ? "missing" : "ok";
    }
    return structuredClone(state);
  }

  function undoable(before: CatalogSnapshot) {
    const undoToken = `jeton-${++counter}`;
    undo.set(undoToken, before);
    return { snapshot: normalize(), undoToken };
  }

  function checkWorkspaceName(name: string, except?: string): string {
    const trimmed = name.trim();
    if (trimmed === "") fail("invalidName", "Le nom ne peut pas être vide.");
    const taken = state.workspaces.find((w) => w.id !== except && same(w.name, trimmed));
    if (taken) fail("nameTaken", `Un workspace « ${taken.name} » existe déjà.`);
    return trimmed;
  }

  /** Commandes après lesquelles le cœur diffuse les statuts des requêtes (R4). */
  const STATUS_CHANGES = new Set([
    "request_send",
    "run_start",
    "run_retry_failures",
    "run_ignore_check",
    "result_ignore_check",
  ]);

  mockIPC(
    (cmd, rawArgs) => {
      const args = (rawArgs ?? {}) as Record<string, unknown>;
      core.calls.push({ cmd, args });
      const failure = failures.get(cmd);
      if (failure) {
        failures.delete(cmd);
        throw failure;
      }
      switch (cmd) {
        case "catalog_load":
          // Lot G : démarrage lent, le squelette reste affiché (0.3-a).
          if (core.startup.slow) return new Promise(() => undefined);
          return { snapshot: normalize(), warning: null };
        case "window_set_title":
          return null;
        case "project_suggest_folder": {
          const location = (args.location as string | null) ?? DEFAULT_LOCATION;
          const name = String(args.name).trim();
          return name === "" ? location : `${location}/${name}`;
        }
        case "project_pick_folder":
          return core.pickedFolder;
        case "project_create": {
          const name = String(args.name).trim();
          if (name === "") fail("invalidName", "Le nom ne peut pas être vide.");
          if (name.includes("/"))
            fail("invalidName", "Le caractère « / » n'est pas autorisé dans un nom de projet.");
          const taken = state.projects.find((p) => same(p.name, name));
          if (taken)
            fail("nameTaken", `Un projet « ${taken.name} » existe déjà dans « Tous les projets ».`);
          const target = args.workspaceId == null ? null : workspace(args.workspaceId);
          const location = (args.location as string | null) ?? DEFAULT_LOCATION;
          const id = `p-${name}`;
          state.projects.push({
            id,
            name,
            path: `${location}/${name}`,
            status: "ok",
            workspaceIds: [],
          });
          target?.projectIds.push(id);
          return { snapshot: normalize(), projectId: id };
        }
        case "project_add_existing": {
          const path = String(args.path);
          if (path.endsWith("/vide")) {
            fail(
              "notAProject",
              `Le dossier ${path} ne contient pas de projet Arnod (arnod.yml absent).`,
            );
          }
          const name = path.split("/").at(-1) ?? path;
          const taken = state.projects.find((p) => same(p.name, name));
          if (taken)
            fail("nameTaken", `Un projet « ${taken.name} » existe déjà dans « Tous les projets ».`);
          const target = args.workspaceId == null ? null : workspace(args.workspaceId);
          const id = `p-${name}`;
          state.projects.push({ id, name, path, status: "ok", workspaceIds: [] });
          target?.projectIds.push(id);
          return { snapshot: normalize(), projectId: id };
        }
        case "project_forget": {
          const before = structuredClone(state);
          project(args.projectId);
          state.projects = state.projects.filter((p) => p.id !== args.projectId);
          for (const w of state.workspaces) {
            w.projectIds = w.projectIds.filter((id) => id !== args.projectId);
          }
          return undoable(before);
        }
        case "project_open": {
          const found = project(args.projectId);
          if (core.missing.has(found.id)) {
            fail(
              "notFound",
              `Le dossier du projet « ${found.name} » est introuvable : ${found.path}.`,
            );
          }
          return { ...found };
        }
        case "workspace_open":
          if (args.workspaceId != null) workspace(args.workspaceId);
          state.lastWorkspaceId = (args.workspaceId as string | null) ?? null;
          return normalize();
        case "workspace_create": {
          const name = checkWorkspaceName(String(args.name));
          const used = new Set(state.workspaces.map((w) => w.color));
          const color = COLORS.find((c) => !used.has(c)) ?? "orange";
          const id = `ws-${name.toLowerCase().replaceAll(" ", "-")}`;
          state.workspaces.push({ id, name, color, projectIds: [] });
          return { snapshot: normalize(), workspaceId: id };
        }
        case "workspace_rename": {
          const target = workspace(args.workspaceId);
          target.name = checkWorkspaceName(String(args.name), target.id);
          return normalize();
        }
        case "workspace_set_color":
          workspace(args.workspaceId).color = args.color as WorkspaceColor;
          return normalize();
        case "workspace_add_projects": {
          const target = workspace(args.workspaceId);
          const ids = args.projectIds as string[];
          ids.forEach(project);
          for (const id of ids) if (!target.projectIds.includes(id)) target.projectIds.push(id);
          return normalize();
        }
        case "workspace_remove_project": {
          const before = structuredClone(state);
          const target = workspace(args.workspaceId);
          if (!target.projectIds.includes(String(args.projectId))) {
            fail("notFound", "Ce projet n'est pas dans ce workspace.");
          }
          target.projectIds = target.projectIds.filter((id) => id !== args.projectId);
          return undoable(before);
        }
        case "workspace_delete": {
          const before = structuredClone(state);
          workspace(args.workspaceId);
          state.workspaces = state.workspaces.filter((w) => w.id !== args.workspaceId);
          if (state.lastWorkspaceId === args.workspaceId) state.lastWorkspaceId = null;
          return undoable(before);
        }
        case "steps_catalog":
          return structuredClone(STEP_CATALOG);
        case "steps_detect":
          return detect(String(args.input));
        case "trial_send": {
          // Essai REST : « Non classés » créé au besoin, « Essai N » sous le contrat déduit.
          const subject = String(args.subject);
          const host = /^https?:\/\/([^/:?#]+)/i.exec(subject)?.[1];
          if (host === undefined)
            fail("invalidRequest", "L'adresse doit commencer par http(s)://.");
          if (!state.projects.some((p) => p.id === INBOX)) {
            state.projects.unshift({
              id: INBOX,
              name: "Non classés",
              path: `${DEFAULT_LOCATION}/Non classés`,
              status: "ok",
              workspaceIds: [],
            });
          }
          state.inboxProjectId = INBOX;
          const count = [...core.workbench.requests.values()].filter((r) =>
            r.name.startsWith("Essai "),
          ).length;
          const name = `Essai ${String(count + 1)}`;
          const path = `${host}/essai-${String(count + 1)}.yml`;
          const values = (args.values ?? {}) as Record<string, string>;
          const method = (values.method ?? "GET") as "GET";
          const route = /^https?:\/\/[^/?#]+(\/[^?#]*)?/i.exec(subject)?.[1] ?? "/";
          const key = `${method} ${route}`;
          const file = `contracts/${host.replaceAll(".", "-")}.yaml`;
          const contract = core.contracts.files.get(file) ?? {
            name: host,
            version: "0.0.0",
            deduced: true,
            source: `openapi: 3.1.0\ninfo:\n  title: ${host}\n  version: 0.0.0\n`,
            operations: [],
          };
          if (!contract.operations.some((o) => o.key === key)) {
            contract.operations.push({ key, method, path: route, summary: null, tag: null });
          }
          core.contracts.files.set(file, contract);
          core.workbench.folders.add(host);
          core.workbench.requests.set(
            path,
            newRequest(name, { url: subject, method, contract: { file, operation: key } }),
          );
          const outcome = okOutcome();
          core.workbench.results.set(path, {
            projectId: INBOX,
            path,
            sending: false,
            outcome,
            error: null,
            receivedAt: 1_700_000_000_000,
            request: null,
          });
          return {
            snapshot: normalize(),
            projectId: INBOX,
            path,
            contract: file,
            outcome,
          };
        }
        case "undo": {
          const before = undo.get(String(args.undoToken));
          if (!before) fail("undoExpired", "Cette action ne peut plus être annulée.");
          undo.delete(String(args.undoToken));
          state = before;
          return normalize();
        }
        default: {
          const windows = handleWindows(core.windows, cmd, args);
          if (windows !== undefined) return windows;
          const response = handleResponse(core.response, core.workbench, cmd, args);
          if (response !== undefined) return response;
          // Lot B : saisie et cURL.
          const input = handleInput(core.workbench, cmd, args);
          if (input !== undefined) return input;
          // Lot C - Import Bruno.
          const imported = handleImports(
            core.imports,
            cmd,
            args,
            (name, path, workspaceId) => {
              const taken = state.projects.find((p) => same(p.name, name));
              if (taken) {
                fail(
                  "nameTaken",
                  `Un projet « ${taken.name} » existe déjà dans « Tous les projets ».`,
                );
              }
              const id = `p-${name}`;
              state.projects.push({ id, name, path, status: "ok", workspaceIds: [] });
              if (workspaceId !== null) workspace(workspaceId).projectIds.push(id);
              return { snapshot: normalize(), projectId: id };
            },
            (id) => project(id).name,
            DEFAULT_LOCATION,
          );
          if (imported !== undefined) return imported;
          // Essais rangés (2.2-b, 2.2-c).
          const trials = handleTrials(core.workbench, core.contracts, cmd, args);
          if (trials !== undefined) return trials;
          // Lot H - Réseau et authentification.
          const network = handleNetwork(core.network, cmd, args);
          if (network !== undefined) return network;
          // Lot I - Fichiers modifiés hors d'Arnod.
          const files = handleFilesLotI(core.workbench, cmd, args);
          if (files !== undefined) return files;
          const started = handleStartupLotG(core.startup, cmd, args);
          if (started !== undefined) return started;
          // Lot J - Copier et déplacer.
          const transferred = handleTransfer(core.workbench, core.contracts, cmd, args);
          if (transferred !== undefined) return transferred;
          // Lot L allégé - Mises à jour.
          const updated = handleUpdates(core.updates, cmd, args);
          if (updated !== undefined) return updated;
          const contracts = handleContracts(core.contracts, core.workbench, cmd, args);
          const handled =
            contracts !== undefined ? contracts : handleWorkbench(core.workbench, cmd, args);
          if (handled === undefined) throw new Error(`commande inattendue : ${cmd}`);
          if (STATUS_CHANGES.has(cmd)) {
            // Le cœur diffuse les statuts après chaque exécution (R4).
            void core.emit("arnod://statuts", {
              projectId: String(args.project),
              statuses: structuredClone(core.workbench.statuses),
            });
          }
          if (cmd === "tree_delete" || cmd === "contract_delete") {
            // Le cœur ferme les onglets de ce qui est supprimé et le diffuse.
            closePath(core.windows, String(args.project), String(args.path));
            void core.emitWindows();
          }
          return handled;
        }
      }
    },
    { shouldMockEvents: true },
  );
  return core;
}
