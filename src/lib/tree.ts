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

/**
 * Chemins des lignes visibles, dans l'ordre affiché : un dossier replié (`open` faux) cache ses
 * enfants. Sert à garder l'arrêt de tabulation de l'arbre sur une ligne affichée (AC1).
 */
export function visiblePaths(nodes: TreeNode[], open: (folder: FolderNode) => boolean): string[] {
  const paths: string[] = [];
  const walk = (list: TreeNode[]): void => {
    for (const node of list) {
      paths.push(node.path);
      if (node.kind === "folder" && open(node)) walk(node.children);
    }
  };
  walk(nodes);
  return paths;
}
