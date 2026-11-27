/**
 * Affichage de l'onglet « Envoyé » (5.1-h, AM29) : découpage du corps envoyé autour des valeurs
 * tirées à cet envoi, pour les surligner. Le cœur a tiré les valeurs (`Sent.drawn`) ; ici, on ne
 * fait que les retrouver dans le texte.
 */
import type { DrawnValue, UsedSource } from "@/types/ipc";

/** Morceau du corps : texte simple, ou valeur tirée (`drawn` : nom de la valeur dynamique). */
export interface SentPart {
  text: string;
  drawn: string | null;
}

/**
 * Découpe `text` autour des valeurs de `drawn` (la plus longue d'abord quand deux commencent au
 * même endroit). Une valeur vide est ignorée.
 */
export function drawnParts(text: string, drawn: DrawnValue[]): SentPart[] {
  const values = drawn
    .filter((d) => d.value !== "")
    .sort((a, b) => b.value.length - a.value.length);
  const parts: SentPart[] = [];
  let plain = "";
  let index = 0;
  while (index < text.length) {
    const found = values.find((d) => text.startsWith(d.value, index));
    if (found) {
      if (plain !== "") parts.push({ text: plain, drawn: null });
      plain = "";
      parts.push({ text: found.value, drawn: found.name });
      index += found.value.length;
    } else {
      plain += text.charAt(index);
      index += 1;
    }
  }
  if (plain !== "") parts.push({ text: plain, drawn: null });
  return parts;
}

/** Libellé de la source d'une variable utilisée : le nom de l'environnement, « locale », « extraite ». */
export function sourceLabel(source: UsedSource, environment: string): string {
  if (source === "local") return "locale";
  if (source === "extracted") return "extraite";
  return environment;
}

/**
 * Chemin canonique (`$.items[0].sku`) d'une extraction lue dans le corps (`body.items[0].sku`),
 * pour marquer « → nom » sur la valeur qu'elle lit (4.2-b bis, AM32) ; `null` hors du corps.
 */
export function bodyPathOf(source: string): string | null {
  if (source === "body") return "$";
  if (source.startsWith("body.") || source.startsWith("body[")) return `$${source.slice(4)}`;
  return null;
}
