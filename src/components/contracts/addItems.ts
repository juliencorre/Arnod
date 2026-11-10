import type { MenuItem } from "@/components/ui/menu";

/** Façons d'ajouter un contrat (C1, C2) : menu de la section et du « + » du projet. */
export const CONTRACT_ADD_ITEMS: MenuItem[] = [
  { id: "file", label: "Importer un fichier…", hint: "OpenAPI 3 ou Swagger 2, JSON ou YAML" },
  { id: "text", label: "Coller un contrat…" },
  { id: "url", label: "Depuis une adresse…" },
  { id: "template", label: "Nouveau contrat…", separatorBefore: true },
];
