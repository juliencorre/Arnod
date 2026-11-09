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
