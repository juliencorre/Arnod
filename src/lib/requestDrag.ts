/**
 * Requête glissée dans la barre latérale (lot J, 4.1-f) : type de donnée propre à Arnod, pour que
 * seul un dépôt de requête soit accepté. Glisser déplace, Ctrl+glisser copie.
 */

/** Type de la donnée glissée. */
export const REQUEST_DRAG = "application/x-arnod-request";

/** Requête glissée : son projet, son chemin et son nom. */
export interface DraggedRequest {
  projectId: string;
  path: string;
  name: string;
}

/** Commence à glisser la requête `request`. */
export function startRequestDrag(event: DragEvent, request: DraggedRequest): void {
  if (!event.dataTransfer) return;
  event.dataTransfer.setData(REQUEST_DRAG, JSON.stringify(request));
  event.dataTransfer.setData("text/plain", request.name);
  event.dataTransfer.effectAllowed = "copyMove";
}

/** Requête déposée, ou `null` si la donnée n'en est pas une. */
export function dragged(event: DragEvent): DraggedRequest | null {
  const raw = event.dataTransfer?.getData(REQUEST_DRAG);
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<DraggedRequest>;
    return typeof value.projectId === "string" &&
      typeof value.path === "string" &&
      typeof value.name === "string"
      ? { projectId: value.projectId, path: value.path, name: value.name }
      : null;
  } catch {
    return null;
  }
}
