/** Lot H : valeurs d'affichage de l'authentification (docs/plan-mvp-h-reseau-auth.md). */
import type { Auth, AuthKind, OAuth2, TokenStatus } from "@/types/ipc";

/** OAuth 2 vide, comme `OAuth2::default()` en Rust. */
export function emptyOAuth2(): OAuth2 {
  return {
    accessTokenUrl: "",
    clientId: "",
    clientSecret: "",
    scope: "",
    credentials: "basicAuthHeader",
    tokenPlacement: "header",
    tokenName: "Authorization",
  };
}

/** Libellé court d'un type d'authentification. */
export function authLabel(kind: AuthKind, other = ""): string {
  switch (kind) {
    case "basic":
      return "Basic";
    case "bearer":
      return "Bearer";
    case "apiKey":
      return "clé d'API";
    case "oauth2":
      return "OAuth 2";
    case "inherit":
      return "héritée";
    case "other":
      return other;
    default:
      return "aucune";
  }
}

const clockFormatter = new Intl.DateTimeFormat("fr-FR", { hour: "2-digit", minute: "2-digit" });

/** Heure courte d'un jeton : « 11:42 ». */
export function formatClock(timestampMs: number): string {
  return clockFormatter.format(new Date(timestampMs));
}

/** Valeur masquée, comme dans « Envoyé » : une authentification n'y montre jamais de secret. */
const MASK = "••••••••";

/** Nature d'une authentification héritée, sur sa carte (3.4-b, AM35). */
export function authKindLabel(auth: Auth): string {
  switch (auth.type) {
    case "basic":
      return "Basic";
    case "bearer":
      return "Bearer";
    case "apiKey":
      return "Clé d'API";
    case "oauth2":
      return "OAuth 2 · identifiants client";
    case "other":
      return `${auth.other}, que Arnod n'envoie pas`;
    default:
      return "Aucune";
  }
}

/** Où part l'authentification, valeur masquée : « Authorization: Bearer •••••••• » (AM35). */
export function authSentIn(auth: Auth): string | null {
  switch (auth.type) {
    case "basic":
      return `Authorization: Basic ${MASK}`;
    case "bearer":
      return `Authorization: Bearer ${MASK}`;
    case "apiKey":
      return auth.location === "query" ? `?${auth.name}=${MASK}` : `${auth.name}: ${MASK}`;
    case "oauth2":
      return auth.oauth2.tokenPlacement === "query"
        ? `?${auth.oauth2.tokenName}=${MASK}`
        : `${auth.oauth2.tokenName}: Bearer ${MASK}`;
    default:
      return null;
  }
}

/** Variables `{{nom}}` lues par une authentification, dans l'ordre, sans doublon (AM35). */
export function authVariables(auth: Auth): string[] {
  const fields = [
    auth.username,
    auth.password,
    auth.token,
    auth.value,
    auth.oauth2.accessTokenUrl,
    auth.oauth2.clientId,
    auth.oauth2.clientSecret,
    auth.oauth2.scope,
  ];
  const names: string[] = [];
  for (const field of fields) {
    // Sans quantificateur paresseux encadré d'espaces : pas de retour arrière coûteux (S8786).
    for (const match of field.matchAll(/\{\{([^{}]*)\}\}/g)) {
      const name = match[1]?.trim();
      if (name !== undefined && name !== "" && !names.includes(name)) names.push(name);
    }
  }
  return names;
}

/** État d'un jeton : « obtenu à 11:42 · expire dans 52 min », ou à demander (AM35). */
export function tokenLine(status: TokenStatus | null, nowMs: number = Date.now()): string {
  if (!status) return "demandé au premier envoi";
  const obtained = `obtenu à ${formatClock(status.obtainedAt)}`;
  if (status.expiresAt === null) return obtained;
  const minutes = Math.round((status.expiresAt - nowMs) / 60_000);
  return minutes > 0 ? `${obtained} · expire dans ${String(minutes)} min` : `${obtained} · expiré`;
}
