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

const clockFormatter = new Intl.DateTimeFormat("fr-FR", { hour: "2-digit", minute: "2-digit" });

/** Fin d'un run (5.2-f) : « aujourd'hui 10:42 » le jour même, sinon « 9 nov., 14:05 ». */
export function formatFinished(timestampMs: number, nowMs: number = Date.now()): string {
  const day = new Date(timestampMs);
  const today = new Date(nowMs);
  return day.toDateString() === today.toDateString()
    ? `aujourd'hui ${clockFormatter.format(day)}`
    : formatDayTime(timestampMs);
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

/** Temps écoulé lisible : « à l'instant », « il y a 5 min », « il y a 2 h », « il y a 3 j ». */
export function formatAgo(timestampMs: number, nowMs: number = Date.now()): string {
  const minutes = Math.floor(Math.max(0, nowMs - timestampMs) / 60_000);
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${String(minutes)} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `il y a ${String(hours)} h`;
  return `il y a ${String(Math.floor(hours / 24))} j`;
}

const shortDayFormatter = new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short" });

/** Envoi de l'historique (5.1-c bis) : « auj. 09:14 », « hier 17:20 », « 4 nov. 11:40 ». */
export function formatSentAt(timestampMs: number, nowMs: number = Date.now()): string {
  const day = new Date(timestampMs);
  const today = new Date(nowMs);
  const yesterday = new Date(nowMs);
  yesterday.setDate(today.getDate() - 1);
  const clock = clockFormatter.format(day);
  if (day.toDateString() === today.toDateString()) return `auj. ${clock}`;
  if (day.toDateString() === yesterday.toDateString()) return `hier ${clock}`;
  return `${shortDayFormatter.format(day)} ${clock}`;
}
