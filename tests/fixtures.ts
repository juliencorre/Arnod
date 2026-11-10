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
  options: { color?: WorkspaceColor; projectIds?: string[] } = {},
): WorkspaceView {
  return {
    id,
    name,
    color: options.color ?? "orange",
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

/** Premier lancement : aucun projet, aucun workspace, « Tous les projets » ouvert (1a). */
export function firstRunSnapshot(): CatalogSnapshot {
  return { projects: [], workspaces: [], lastWorkspaceId: null, inboxProjectId: null };
}

/**
 * Ouverture habituelle (2a) : onglets « Mes projets » (gris) puis « Boutique »,
 * qui contiennent tous deux les deux projets ; « Boutique » est ouvert.
 */
export function boutiqueSnapshot(): CatalogSnapshot {
  return {
    projects: [
      project(BOUTIQUE_TESTS, "boutique-tests", { workspaceIds: [MES_PROJETS, BOUTIQUE] }),
      project(AUTH_SSO, "auth-sso", { workspaceIds: [MES_PROJETS, BOUTIQUE] }),
    ],
    workspaces: [
      workspace(MES_PROJETS, "Mes projets", {
        color: "gray",
        projectIds: [BOUTIQUE_TESTS, AUTH_SSO],
      }),
      workspace(BOUTIQUE, "Boutique", { projectIds: [BOUTIQUE_TESTS, AUTH_SSO] }),
    ],
    lastWorkspaceId: BOUTIQUE,
    inboxProjectId: null,
  };
}
