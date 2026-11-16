/**
 * Libellés d'affichage des natures d'étape (catalogue des étapes). Aucune règle :
 * les familles, moteurs et origines viennent du cœur, ce module ne fait que les
 * nommer en français.
 */
import type { StepEngine, StepFamily, StepField, StepNature, StepRule } from "@/types/ipc";

/** Nom d'une famille, comme dans le catalogue. */
export const FAMILY_LABELS: Record<StepFamily, string> = {
  calls: "Appels",
  data: "Données",
  messages: "Messages",
  jobs: "Traitements",
  interface: "Interface",
  code: "Code",
  files: "Fichiers",
};

/** Moteur d'une nature : « intégré » ou « WASM ». */
export function engineLabel(engine: StepEngine): string {
  return engine.kind === "builtin" ? "intégré" : "WASM";
}

/** Valeur affichée d'un champ : saisie, sinon valeur par défaut, sinon première option. */
export function fieldValue(field: StepField, values: Record<string, string>): string {
  return (
    values[field.key] ?? field.default ?? (field.kind === "choice" ? (field.options[0] ?? "") : "")
  );
}

/** Le champ est-il affiché ? Sa condition porte sur un choix placé avant lui. */
export function isVisible(
  nature: StepNature,
  field: StepField,
  values: Record<string, string>,
): boolean {
  const condition = field.visibleWhen;
  if (condition === null) return true;
  const target = nature.form.find((f) => f.key === condition.key);
  return target !== undefined && condition.in.includes(fieldValue(target, values));
}

/** Règle de reconnaissance, en clair. */
export function ruleLabel(rule: StepRule): string {
  switch (rule.kind) {
    case "url": {
      const schemes = rule.schemes.map((s) => `${s}://`).join(" ou ");
      return rule.methodField === null
        ? `Une adresse ${schemes}.`
        : `Une adresse ${schemes}, éventuellement précédée de la méthode (POST https://…).`;
    }
    case "keywords":
      return `Un texte qui commence par ${rule.words.join(", ")}.`;
    case "prefix":
      return `Un texte qui commence par « ${rule.text} ».`;
    case "suffix":
      return `Un texte qui finit par « ${rule.text} ».`;
    case "curl":
      return "Une commande cURL (curl …) : méthode, adresse, en-têtes et corps repris.";
  }
}

/** Nature d'un champ, en clair. */
export function fieldKindLabel(field: StepField): string {
  switch (field.kind) {
    case "choice":
      return field.options.join(", ");
    case "text":
      return "une ligne de texte";
    case "multiline":
      return "plusieurs lignes";
  }
}
