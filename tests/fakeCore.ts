/**
 * Faux cœur Rust pour les tests de composants : répond aux commandes IPC en
 * mémoire, en appliquant une version simplifiée des règles R1 à R11 (le vrai
 * comportement est testé côté Rust). Les appels sont notés dans `calls`.
 */
import { mockIPC } from "@tauri-apps/api/mocks";

import type { CatalogSnapshot, CommandError, WorkspaceColor } from "@/types/ipc";

export const HOME = "/home/julien";
export const DEFAULT_LOCATION = `${HOME}/arnod`;

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
  };

  const same = (a: string, b: string): boolean => a.toLowerCase() === b.toLowerCase();
  const workspace = (id: unknown) =>
    state.workspaces.find((w) => w.id === id) ?? fail("notFound", "Ce workspace n'existe plus.");
  const project = (id: unknown) =>
    state.projects.find((p) => p.id === id) ??
    fail("notFound", "Ce projet n'existe plus dans le référentiel.");

  /** Recalcule les champs dérivés et l'ordre R11, comme l'instantané Rust. */
  function normalize(): CatalogSnapshot {
    state.workspaces.sort((a, b) =>
      a.id === state.defaultWorkspaceId
        ? -1
        : b.id === state.defaultWorkspaceId
          ? 1
          : a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
    );
    for (const w of state.workspaces) w.isDefault = w.id === state.defaultWorkspaceId;
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

  mockIPC((cmd, rawArgs) => {
    const args = (rawArgs ?? {}) as Record<string, unknown>;
    core.calls.push({ cmd, args });
    const failure = failures.get(cmd);
    if (failure) {
      failures.delete(cmd);
      throw failure;
    }
    switch (cmd) {
      case "catalog_load":
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
          fail("nameTaken", `Un projet « ${taken.name} » existe déjà dans le référentiel.`);
        const target = workspace(args.workspaceId);
        const location = (args.location as string | null) ?? DEFAULT_LOCATION;
        const id = `p-${name}`;
        state.projects.push({
          id,
          name,
          path: `${location}/${name}`,
          status: "ok",
          workspaceIds: [],
        });
        for (const w of [workspace(state.defaultWorkspaceId), target]) {
          if (!w.projectIds.includes(id)) w.projectIds.push(id);
        }
        return { snapshot: normalize(), projectId: id };
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
        workspace(args.workspaceId);
        state.lastWorkspaceId = String(args.workspaceId);
        return normalize();
      case "workspace_create": {
        const name = checkWorkspaceName(String(args.name));
        const used = new Set(state.workspaces.map((w) => w.color));
        const color = COLORS.find((c) => !used.has(c)) ?? "orange";
        const id = `ws-${name.toLowerCase().replaceAll(" ", "-")}`;
        state.workspaces.push({ id, name, color, isDefault: false, projectIds: [] });
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
      case "workspace_set_default":
        state.defaultWorkspaceId = workspace(args.workspaceId).id;
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
        if (args.workspaceId === state.defaultWorkspaceId) {
          fail("defaultWorkspace", "Le workspace par défaut ne peut pas être supprimé.");
        }
        const before = structuredClone(state);
        workspace(args.workspaceId);
        state.workspaces = state.workspaces.filter((w) => w.id !== args.workspaceId);
        if (state.lastWorkspaceId === args.workspaceId)
          state.lastWorkspaceId = state.defaultWorkspaceId;
        return undoable(before);
      }
      case "undo": {
        const before = undo.get(String(args.undoToken));
        if (!before) fail("undoExpired", "Cette action ne peut plus être annulée.");
        undo.delete(String(args.undoToken));
        state = before;
        return normalize();
      }
      default:
        throw new Error(`commande inattendue : ${cmd}`);
    }
  });
  return core;
}
