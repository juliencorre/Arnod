/**
 * Affichage d'un environnement (pastille, menu, liste de la gestion) : couleur en jeton du thème
 * et adresse de base sans le schéma, de quoi reconnaître l'environnement.
 */

/** Couleurs proposées pour un environnement, dans l'ordre du menu « Couleur ». */
export const ENVIRONMENT_COLORS = [
  ["gray", "Gris"],
  ["blue", "Bleu"],
  ["green", "Vert"],
  ["orange", "Orange"],
  ["red", "Rouge"],
  ["purple", "Violet"],
] as const;

/** Couleur d'un environnement, en jeton du thème. */
export function envSwatch(color: string | undefined): string {
  return `var(--color-ws-${color ?? "gray"})`;
}

/** Adresse de base, sans le schéma ni la barre finale ; `undefined` sans `baseUrl`. */
export function envHost(env: { variables: { name: string; value: string }[] }): string | undefined {
  const base = env.variables.find((v) => v.name === "baseUrl")?.value.trim() ?? "";
  return base === "" ? undefined : base.replace(/^[a-z]+:\/\//i, "").replace(/\/$/, "");
}
