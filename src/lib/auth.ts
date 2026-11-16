/** Lot H : valeurs d'affichage de l'authentification (docs/plan-mvp-h-reseau-auth.md). */
import type { AuthKind, OAuth2 } from "@/types/ipc";

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
