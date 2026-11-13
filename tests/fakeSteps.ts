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
  detect: [{ kind: "url", schemes: ["http", "https"], methodField: "method" }, { kind: "curl" }],
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
      key: "headers",
      label: "En-têtes",
      kind: "multiline",
      default: null,
      placeholder: "Nom: valeur, une ligne par en-tête",
      hint: null,
      options: [],
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

/** Mots d'une commande, guillemets simples ou doubles retirés (version simplifiée du cœur). */
function words(text: string): string[] {
  return [...text.replace(/\\\n/g, " ").matchAll(/'([^']*)'|"([^"]*)"|(\S+)/g)].map(
    (m) => m[1] ?? m[2] ?? m[3] ?? "",
  );
}

/** Commande cURL : `-X`, `-H`, `-d`/`--data*`, adresse ; jeton retiré, `--compressed` signalée. */
function detectCurl(input: string): Detection | null {
  const all = words(input);
  if (all[0] !== "curl") return null;
  let method: string | null = null;
  let url = "";
  const headers: string[] = [];
  let body = "";
  const secrets: { name: string; value: string; header: string }[] = [];
  const ignored: { option: string; reason: string }[] = [];
  for (let i = 1; i < all.length; i++) {
    const word = all[i] ?? "";
    if (word === "-X") method = (all[++i] ?? "").toUpperCase();
    else if (word === "-H") {
      let header = all[++i] ?? "";
      const colon = header.indexOf(":");
      const value = colon < 0 ? "" : header.slice(colon + 1).trim();
      const isAuth = header.slice(0, Math.max(colon, 0)).trim().toLowerCase() === "authorization";
      if (isAuth && value.toLowerCase().startsWith("bearer ")) {
        secrets.push({ name: "token", value: value.slice(7).trim(), header: "Authorization" });
        header = "Authorization: Bearer {{token}}";
      }
      headers.push(header);
    } else if (word === "-d" || word.startsWith("--data")) body = all[++i] ?? "";
    else if (word === "--compressed")
      ignored.push({
        option: word,
        reason: "Arnod reçoit la réponse sans compression, il n'y a rien à régler",
      });
    else if (!word.startsWith("-")) url = word;
  }
  if (url === "") return null;
  const values: Record<string, string> = { method: method ?? (body === "" ? "GET" : "POST") };
  if (headers.length > 0) values.headers = headers.join("\n");
  if (body !== "") {
    values.bodyType = headers.some((h) => /json/i.test(h)) ? "JSON" : "Texte";
    values.body = body;
  }
  return { nature: "rest", subject: url, values, curl: { secrets, ignored } };
}

/** `[MÉTHODE ]http(s)://hôte…` : REST ; autre texte : aucune nature. */
export function detect(input: string): Detection | null {
  if (input.trim().startsWith("curl ")) return detectCurl(input.trim());
  let subject = input.trim();
  const values: Record<string, string> = {};
  const [first, ...rest] = subject.split(/\s+/);
  if (first !== undefined && METHODS.includes(first.toUpperCase()) && rest.length > 0) {
    values.method = first.toUpperCase();
    subject = rest.join(" ");
  }
  return /^https?:\/\/[^/\s]+/i.test(subject) ? { nature: "rest", subject, values } : null;
}
