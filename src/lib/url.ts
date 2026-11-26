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

/**
 * Chemin et hôte d'une adresse, pour la confirmation d'un environnement protégé (3.3-c, AM31) :
 * les `{{nom}}` connus de `values` sont remplacés, puis l'adresse est lue. Une adresse illisible
 * garde son texte en chemin, sans hôte.
 */
export function targetOf(
  url: string,
  values: Record<string, string>,
): { path: string; host: string | null } {
  // Sans quantificateur paresseux encadré d'espaces : pas de retour arrière coûteux (S8786).
  const resolved = url.replace(
    /\{\{([^{}]*)\}\}/g,
    (all, name: string) => values[name.trim()] ?? all,
  );
  try {
    const parsed = new URL(resolved);
    return { path: decodeURI(parsed.pathname) + parsed.search, host: parsed.host };
  } catch {
    return { path: url, host: null };
  }
}
