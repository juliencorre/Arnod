/**
 * Lecture des règles CSS d'un composant (`<style>`) ou d'une feuille de style, pour vérifier une
 * couleur ou une disposition que Vitest ne calcule pas (happy-dom n'applique pas les styles des
 * composants). Les blocs `@media` sont lus comme leurs règles.
 */

/** Sources brutes des composants et de la feuille de style globale, par chemin du dépôt. */
const SOURCES = import.meta.glob<string>(["/src/**/*.vue", "/src/styles.css"], {
  query: "?raw",
  import: "default",
  eager: true,
});

export interface CssRule {
  selectors: string[];
  declarations: Map<string, string>;
}

/** Composants Vue de `src/`, chemins relatifs au dépôt. */
export function componentPaths(): string[] {
  return Object.keys(SOURCES)
    .filter((path) => path.endsWith(".vue"))
    .map((path) => path.slice(1));
}

/** Règles du fichier `path` (relatif au dépôt) : `<style>` d'un `.vue`, ou le fichier `.css`. */
export function cssRules(path: string): CssRule[] {
  const text = SOURCES[`/${path}`];
  if (text === undefined) throw new Error(`fichier inconnu : ${path}`);
  const css = path.endsWith(".vue")
    ? [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1] ?? "").join("\n")
    : text;
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules: CssRule[] = [];
  for (const match of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const selectors = (match[1] ?? "")
      .split(",")
      .map((s) => s.trim().replace(/\s+/g, " "))
      .filter((s) => s !== "" && !s.startsWith("@"));
    const declarations = new Map<string, string>();
    for (const line of (match[2] ?? "").split(";")) {
      const colon = line.indexOf(":");
      if (colon < 0) continue;
      declarations.set(line.slice(0, colon).trim(), line.slice(colon + 1).trim());
    }
    rules.push({ selectors, declarations });
  }
  return rules;
}

/** Déclaration `property` des règles de `path` qui nomment exactement `selector` (la dernière). */
export function cssValue(path: string, selector: string, property: string): string | undefined {
  let found: string | undefined;
  for (const rule of cssRules(path)) {
    if (rule.selectors.includes(selector) && rule.declarations.has(property)) {
      found = rule.declarations.get(property);
    }
  }
  return found;
}

/** Source brute du fichier `path` (relatif au dépôt), pour vérifier une forme du gabarit. */
export function rawSource(path: string): string {
  const text = SOURCES[`/${path}`];
  if (text === undefined) throw new Error(`fichier inconnu : ${path}`);
  return text;
}
