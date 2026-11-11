import type { WorkspaceColor } from "@/types/ipc";

const timeFormatter = new Intl.DateTimeFormat("fr-FR", {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
  fractionalSecondDigits: 3,
});

/** Formate un horodatage Unix (ms) en heure locale, ex. « 14:05:09,123 ». */
export function formatTime(timestampMs: number): string {
  return timeFormatter.format(new Date(timestampMs));
}

/** Nombre suivi du nom accordé, au singulier jusqu'à 1 : « 0 projet », « 6 projets ». */
export function countLabel(count: number, singular: string, plural: string): string {
  return `${count} ${count > 1 ? plural : singular}`;
}

/** Durée lisible : « 85 ms », « 1,2 s ». */
export function formatDuration(ms: number): string {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1).replace(".", ",")} s`;
}

/** Taille lisible : « 512 o », « 1,5 Ko », « 2,0 Mo ». */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1).replace(".", ",")} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1).replace(".", ",")} Mo`;
}

/** Libellés français des couleurs de workspace (W8, O5). */
export const COLOR_LABELS: Record<WorkspaceColor, string> = {
  gray: "Gris",
  orange: "Orange",
  blue: "Bleu",
  green: "Vert",
  purple: "Violet",
  red: "Rouge",
  yellow: "Jaune",
  teal: "Turquoise",
};

const dayTimeFormatter = new Intl.DateTimeFormat("fr-FR", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});

/** Date courte d'un run : « 9 nov., 14:05 ». */
export function formatDayTime(timestampMs: number): string {
  return dayTimeFormatter.format(new Date(timestampMs));
}

/**
 * Chemin découpé après chaque séparateur (« / » ou « \ »), pour ne le couper qu'entre deux
 * dossiers : « /home/a/b » donne « / », « home/ », « a/ », « b ».
 */
export function pathSegments(path: string): string[] {
  const segments: string[] = [];
  let start = 0;
  for (let index = 0; index < path.length; index++) {
    const char = path[index];
    if (char === "/" || char === "\\") {
      segments.push(path.slice(start, index + 1));
      start = index + 1;
    }
  }
  // Le dernier dossier, ou le chemin entier s'il n'a pas de séparateur (même vide).
  if (start < path.length || segments.length === 0) segments.push(path.slice(start));
  return segments;
}
