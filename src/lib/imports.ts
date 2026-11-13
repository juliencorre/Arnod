/**
 * Libellés d'affichage de l'import (docs/plan-mvp-c-import-bruno.md) : formats, natures et
 * suites des éléments du rapport. Aucune règle : le cœur classe, l'interface nomme.
 */
import type { ImportFormat, ImportItemKind, ImportItemStatus, ImportReportItem } from "@/types/ipc";

/** « collection Bruno », « contrat OpenAPI »… */
export const FORMAT_LABELS: Record<ImportFormat, string> = {
  bruno: "collection Bruno",
  openCollection: "projet Arnod ou OpenCollection",
  openApi: "contrat OpenAPI",
};

/** Repère court d'une nature, en tête de ligne (« TESTS », « PRÉ-REQ. »…). */
export const KIND_TAGS: Record<ImportItemKind, string> = {
  tests: "TESTS",
  preRequest: "PRÉ-REQ.",
  postResponse: "SCRIPT",
  env: "ENV",
  auth: "AUTH",
  body: "CORPS",
  request: "REQUÊTE",
  assertion: "VÉRIF.",
  extraction: "EXTRACT.",
  option: "OPTION",
};

/** Suite donnée, à droite de la ligne. */
export const STATUS_LABELS: Record<ImportItemStatus, string> = {
  notRun: "non repris",
  toFill: "à renseigner",
  toCheck: "à vérifier",
  skipped: "non importé",
  stored: "au coffre",
};

/** Phrase de tête de « Après · dans Arnod ». */
export const STATUS_HEADLINES: Record<ImportItemStatus, string> = {
  notRun: "Gardé dans le fichier, pas exécuté.",
  toFill: "Variable locale, à renseigner.",
  toCheck: "Repris, à vérifier.",
  skipped: "Non importé.",
  stored: "Rangé dans le coffre du système.",
};

/** Contenu d'origine tel qu'il s'écrit dans le fichier `.bru` (« tests { … } »). */
export function originalBlock(item: ImportReportItem): string | null {
  if (item.code === null) return null;
  if (item.block === null) return item.code;
  const body = item.code
    .split("\n")
    .map((line) => (line === "" ? line : `  ${line}`))
    .join("\n");
  return `${item.block} {\n${body}\n}`;
}
