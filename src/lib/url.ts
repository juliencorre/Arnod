/**
 * Lecture d'une adresse pour l'affichage. La règle (R1 du plan de configuration d'un
 * endpoint) est appliquée par le cœur à l'enregistrement : ici, seulement les lignes à
 * montrer pendant la frappe.
 */

/** Noms des paramètres de chemin : segments `:nom` ou `{nom}`, dans l'ordre, sans doublon. */
export function pathParamNames(url: string): string[] {
  const path = url.split(/[?#]/)[0] ?? "";
  const names: string[] = [];
  for (const segment of path.split("/")) {
    let name: string | null = null;
    if (segment.startsWith(":")) name = segment.slice(1);
    else if (segment.startsWith("{") && segment.endsWith("}") && !segment.startsWith("{{")) {
      name = segment.slice(1, -1);
    }
    if (name && /^[\p{L}\p{N}_.-]+$/u.test(name) && !names.includes(name)) names.push(name);
  }
  return names;
}
