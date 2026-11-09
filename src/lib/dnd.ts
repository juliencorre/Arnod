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
