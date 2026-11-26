/**
 * Lectures d'affichage dans l'arborescence fournie par le cœur (`TreeNode`) : aucune règle, le cœur
 * décide déjà ce qu'est un test (`TreeNode.test`).
 */
import type { TreeNode } from "@/types/ipc";

/** Dossier de l'arborescence (`TreeNode` de nature `folder`). */
export type FolderNode = Extract<TreeNode, { kind: "folder" }>;

/** Dossier `path` de l'arborescence, s'il existe. */
export function findFolder(nodes: TreeNode[], path: string): FolderNode | null {
  for (const node of nodes) {
    if (node.kind !== "folder") continue;
    if (node.path === path) return node;
    const found = findFolder(node.children, path);
    if (found) return found;
  }
  return null;
}

/** Test qui range la requête `path`, si son dossier en est un (une étape). */
export function testOf(nodes: TreeNode[], path: string): FolderNode | null {
  const folder = path.split("/").slice(0, -1).join("/");
  if (folder === "") return null;
  const node = findFolder(nodes, folder);
  return node?.test ? node : null;
}
