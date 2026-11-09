/**
 * Glisser-déposer d'un projet sur un onglet de workspace (O7). Le type MIME
 * propre à Arnod évite d'accepter un texte ou un fichier glissé d'ailleurs.
 */
export const PROJECT_DRAG_TYPE = "application/x-arnod-project";

/** Prépare le glisser d'un projet. */
export function startProjectDrag(event: DragEvent, projectId: string): void {
  if (!event.dataTransfer) return;
  event.dataTransfer.setData(PROJECT_DRAG_TYPE, projectId);
  event.dataTransfer.effectAllowed = "copy";
}

/** Le glisser en cours porte-t-il un projet ? */
export function isProjectDrag(event: DragEvent): boolean {
  return Array.from(event.dataTransfer?.types ?? []).includes(PROJECT_DRAG_TYPE);
}

/** Projet déposé, ou `null`. */
export function droppedProject(event: DragEvent): string | null {
  const id = event.dataTransfer?.getData(PROJECT_DRAG_TYPE) ?? "";
  return id === "" ? null : id;
}

/** Glisser d'un onglet de contenu (F2, F8). */
export const TAB_DRAG_TYPE = "application/x-arnod-tab";

/** Onglet glissé : sa fenêtre, sa zone et son identifiant. */
export interface DraggedTab {
  window: string;
  zone: string;
  tab: string;
}

export function startTabDrag(event: DragEvent, dragged: DraggedTab): void {
  if (!event.dataTransfer) return;
  event.dataTransfer.setData(TAB_DRAG_TYPE, JSON.stringify(dragged));
  event.dataTransfer.effectAllowed = "move";
}

export function isTabDrag(event: DragEvent): boolean {
  return Array.from(event.dataTransfer?.types ?? []).includes(TAB_DRAG_TYPE);
}

export function droppedTab(event: DragEvent): DraggedTab | null {
  try {
    const value: unknown = JSON.parse(event.dataTransfer?.getData(TAB_DRAG_TYPE) ?? "");
    if (
      typeof value === "object" &&
      value !== null &&
      "window" in value &&
      "zone" in value &&
      "tab" in value &&
      typeof value.window === "string" &&
      typeof value.zone === "string" &&
      typeof value.tab === "string"
    ) {
      return { window: value.window, zone: value.zone, tab: value.tab };
    }
  } catch {
    // Contenu étranger : ignoré.
  }
  return null;
}

/** Côté de dépôt d'un onglet dans une zone : bord droit, bord bas, ou centre (F2). */
export type DropRegion = "right" | "bottom" | "center";

/** Région de dépôt d'après la position du pointeur dans la zone. */
export function dropRegion(rect: DOMRect, clientX: number, clientY: number): DropRegion {
  const x = (clientX - rect.left) / (rect.width || 1);
  const y = (clientY - rect.top) / (rect.height || 1);
  if (x > 0.7 && x - 0.7 >= y - 0.7) return "right";
  if (y > 0.7) return "bottom";
  return "center";
}

/** Le glisser s'est terminé hors de la fenêtre (F8). */
export function endedOutside(event: DragEvent): boolean {
  return (
    event.dataTransfer?.dropEffect === "none" &&
    (event.clientX <= 0 ||
      event.clientY <= 0 ||
      event.clientX >= window.innerWidth ||
      event.clientY >= window.innerHeight)
  );
}
