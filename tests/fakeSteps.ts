/**
 * Faux catalogue des étapes (docs/plan-accueil-non-classes.md) : la définition
 * REST intégrée, telle que `steps_catalog` la renvoie, et une reconnaissance
 * simplifiée (le vrai comportement est testé côté Rust).
 */
import type { Detection, StepCatalog, StepNature } from "@/types/ipc";

export const REST: StepNature = {
  format: 1,
  id: "rest",
  version: "1.0.0",
  name: "REST",
  title: "Une requête HTTP",
  description:
    "Appelle une API par HTTP : méthode, adresse, corps. Les vérifications portent sur le statut, les en-têtes, le corps et la durée.",
  family: "calls",
  publisher: "Arnod",
  engine: { kind: "builtin", id: "http", abi: 1 },
  action: "Envoyer",
  subject: { key: "url", label: "Adresse", placeholder: "https://…" },
  detect: [{ kind: "url", schemes: ["http", "https"], methodField: "method" }],
  form: [
    {
      key: "method",
      label: "Méthode",
      kind: "choice",
      default: "GET",
      placeholder: null,
      hint: null,
      options: ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"],
      visibleWhen: null,
    },
    {
      key: "bodyType",
      label: "Type du corps",
      kind: "choice",
      default: "JSON",
      placeholder: null,
      hint: null,
      options: ["JSON", "XML", "Texte"],
      visibleWhen: { key: "method", in: ["POST", "PUT", "PATCH"] },
    },
    {
      key: "body",
      label: "Corps",
      kind: "multiline",
      default: null,
      placeholder: "{ … }",
      hint: null,
      options: [],
      visibleWhen: { key: "method", in: ["POST", "PUT", "PATCH"] },
    },
  ],
  origin: "builtin",
};

export const STEP_CATALOG: StepCatalog = {
  source: { name: "arnod-steps", updated: "2026-11-10", synced: false },
  natures: [REST],
};

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/** `[MÉTHODE ]http(s)://hôte…` : REST ; autre texte : aucune nature. */
export function detect(input: string): Detection | null {
  let subject = input.trim();
  const values: Record<string, string> = {};
  const [first, ...rest] = subject.split(/\s+/);
  if (first !== undefined && METHODS.includes(first.toUpperCase()) && rest.length > 0) {
    values.method = first.toUpperCase();
    subject = rest.join(" ");
  }
  return /^https?:\/\/[^/\s]+/i.test(subject) ? { nature: "rest", subject, values } : null;
}
