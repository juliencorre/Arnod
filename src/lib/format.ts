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
