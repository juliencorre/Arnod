/**
 * Affichage de la synchronisation (`docs/plan-v2.md`, lots E et F) : libellé de la barre
 * d'état, nature d'un élément partagé, initiale d'un auteur. Le cœur décrit tout le reste, en
 * clair (R14) ; ici, seulement des mots et des comptes d'affichage.
 */
import { countLabel } from "@/lib/format";
import type { SharedItemKind, SyncView } from "@/types/ipc";

/** Texte de la barre d'état (SY4). */
export function statusLabel(view: SyncView | null): string {
  if (!view?.remote) return "Pas encore partagé";
  if (view.conflicts > 0) return `${countLabel(view.conflicts, "conflit", "conflits")} à résoudre`;
  const parts: string[] = [];
  if (view.incoming.length > 0) parts.push(`${String(view.incoming.length)} à récupérer`);
  if (view.outgoing.length > 0) parts.push(`${String(view.outgoing.length)} à envoyer`);
  if (parts.length > 0) return parts.join(" · ");
  if (view.error !== null) return "Vérification impossible";
  return "À jour avec l'équipe";
}

const KINDS: Record<SharedItemKind, string> = {
  step: "Étape",
  request: "Requête",
  endpoint: "Endpoint",
  folder: "Dossier",
  contract: "Contrat",
  environment: "Environnement",
  project: "Projet",
  other: "Fichier",
};

/** Nature d'un élément partagé, en clair (pastille du panneau). */
export function kindLabel(kind: SharedItemKind): string {
  return KINDS[kind];
}

/** Initiale d'un auteur, pour sa pastille (CF4). */
export function initial(name: string): string {
  const letter = Array.from(name).find((c) => /[\p{L}\p{N}]/u.test(c));
  return letter ? letter.toUpperCase() : "?";
}

/** Forge d'une adresse, pour l'icône du panneau. */
export function forgeBadge(forge: string): string {
  if (forge === "github") return "GH";
  if (forge === "gitlab") return "GL";
  return "git";
}
