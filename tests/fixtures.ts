/**
 * Données de test : instantanés du catalogue tels que les renvoie le cœur Rust.
 */
import type { CatalogSnapshot, ProjectView, WorkspaceColor, WorkspaceView } from "@/types/ipc";

export const MES_PROJETS = "ws-mes-projets";
export const BOUTIQUE = "ws-boutique";
export const BOUTIQUE_TESTS = "p-boutique-tests";
export const AUTH_SSO = "p-auth-sso";

export function workspace(
  id: string,
  name: string,
  options: { color?: WorkspaceColor; isDefault?: boolean; projectIds?: string[] } = {},
): WorkspaceView {
  return {
    id,
    name,
    color: options.color ?? "orange",
    isDefault: options.isDefault ?? false,
    projectIds: options.projectIds ?? [],
  };
}

export function project(
  id: string,
  name: string,
  options: { workspaceIds?: string[]; missing?: boolean } = {},
): ProjectView {
  return {
    id,
    name,
    path: `/home/julien/arnod/${name}`,
    status: options.missing === true ? "missing" : "ok",
    workspaceIds: options.workspaceIds ?? [],
  };
}

/** Premier lancement : « Mes projets » seul, aucun projet (1a). */
export function firstRunSnapshot(): CatalogSnapshot {
  return {
    projects: [],
    workspaces: [workspace(MES_PROJETS, "Mes projets", { color: "gray", isDefault: true })],
    defaultWorkspaceId: MES_PROJETS,
    lastWorkspaceId: MES_PROJETS,
  };
}

/** Ouverture habituelle : « Boutique » ouvert avec deux projets (2a). */
export function boutiqueSnapshot(): CatalogSnapshot {
  return {
    projects: [
      project(BOUTIQUE_TESTS, "boutique-tests", { workspaceIds: [MES_PROJETS, BOUTIQUE] }),
      project(AUTH_SSO, "auth-sso", { workspaceIds: [MES_PROJETS, BOUTIQUE] }),
    ],
    workspaces: [
      workspace(MES_PROJETS, "Mes projets", {
        color: "gray",
        isDefault: true,
        projectIds: [BOUTIQUE_TESTS, AUTH_SSO],
      }),
      workspace(BOUTIQUE, "Boutique", { projectIds: [BOUTIQUE_TESTS, AUTH_SSO] }),
    ],
    defaultWorkspaceId: MES_PROJETS,
    lastWorkspaceId: BOUTIQUE,
  };
}
