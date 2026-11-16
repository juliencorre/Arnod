/**
 * Lot H - Réseau et authentification : faux cœur des préférences réseau, de l'authentification du
 * projet et des cookies (docs/plan-mvp-h-reseau-auth.md). Le vrai comportement est testé en Rust.
 */
import { emptyOAuth2 } from "@/lib/auth";
import type {
  Auth,
  CertificateInfo,
  CommandError,
  CookieJarView,
  NetworkFailure,
  NetworkPrefsInput,
  NetworkView,
  ProjectAuthView,
} from "@/types/ipc";

export interface FakeNetwork {
  view: NetworkView;
  /** Certificat renvoyé par le prochain `network_pick_authority` (`null` : annulé). */
  picked: CertificateInfo | null;
  /** Authentification du projet, par projet. */
  auth: Map<string, Auth>;
  /** Jeton de l'environnement : obtenu à, expire à. */
  token: { obtainedAt: number; expiresAt: number | null } | null;
  hasClientSecret: boolean;
  usage: ProjectAuthView["usage"];
  /** Cookies, par environnement. */
  cookies: Map<string, CookieJarView>;
}

export function certificateLotH(subject: string, issuer = subject): CertificateInfo {
  return {
    subject,
    issuer,
    notBefore: "02/01/2026",
    notAfter: "02/01/2027",
    fingerprint:
      "4F:1A:9C:07:2B:44:D1:E8:90:3A:5C:61:7E:12:AA:0B:C3:9D:58:F4:21:6E:07:B9:4C:13:88:D2:6A:F0:B3:E2",
    names: subject.includes(".") ? [subject] : [],
    authority: subject === issuer,
    selfSigned: subject === issuer,
    pem: `-----BEGIN CERTIFICATE-----\n${subject}\n-----END CERTIFICATE-----\n`,
  };
}

/** Échec « certificat non reconnu » de l'écran 5.1-i. */
export function certificateFailureLotH(): CommandError {
  const failure: NetworkFailure = {
    cause: "certificate",
    title: "Le certificat de recette.boutique.local n'est pas reconnu",
    explanation:
      "Il est signé par « Boutique Interne CA », une autorité que ni le système ni Arnod ne connaissent. La requête n'est pas partie : rien n'a été écrit sur le serveur.",
    notSent: true,
    host: "recette.boutique.local",
    port: 443,
    proxy: null,
    connectTimeoutMs: 10000,
    elapsedMs: 312,
    certificate: {
      problem: "unknownIssuer",
      server: certificateLotH("recette.boutique.local", "Boutique Interne CA"),
      authority: certificateLotH("Boutique Interne CA"),
    },
  };
  return { kind: "network", message: `${failure.title}. ${failure.explanation}`, failure };
}

export function createFakeNetwork(): FakeNetwork {
  return {
    view: {
      proxy: { mode: "system", manual: { url: "", noProxy: "", username: "" } },
      hasProxyPassword: false,
      system: {
        http: "proxy.entreprise.local:8080",
        https: "proxy.entreprise.local:8080",
        noProxy: "<local>",
        source: "windows",
        autoConfig: null,
      },
      connectTimeoutMs: 10000,
      systemAuthorities: 142,
      authorities: [],
    },
    picked: null,
    auth: new Map(),
    token: null,
    hasClientSecret: false,
    usage: { inheriting: 0, overrides: [] },
    cookies: new Map(),
  };
}

function emptyAuth(): Auth {
  return {
    type: "none",
    username: "",
    password: "",
    token: "",
    name: "",
    value: "",
    location: "header",
    other: "",
    oauth2: emptyOAuth2(),
  };
}

function authView(net: FakeNetwork, project: string, environment: string): ProjectAuthView {
  return {
    auth: structuredClone(net.auth.get(project) ?? emptyAuth()),
    environment,
    token: net.token,
    hasClientSecret: net.hasClientSecret,
    usage: structuredClone(net.usage),
  };
}

function jar(net: FakeNetwork, environment: string): CookieJarView {
  return net.cookies.get(environment) ?? { environment, host: null, domains: [] };
}

/** Commandes du lot H ; `undefined` si la commande n'en est pas une. */
export function handleNetwork(
  net: FakeNetwork,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  const project = String(args.project);
  const environment = String(args.environment);
  switch (cmd) {
    case "network_load":
      return structuredClone(net.view);
    case "network_save": {
      const prefs = args.prefs as NetworkPrefsInput;
      net.view.proxy = JSON.parse(JSON.stringify(prefs.proxy)) as NetworkView["proxy"];
      net.view.connectTimeoutMs = prefs.connectTimeoutMs;
      if (prefs.proxyPassword !== null) net.view.hasProxyPassword = prefs.proxyPassword !== "";
      return structuredClone(net.view);
    }
    case "network_pick_authority":
      return net.picked;
    case "network_add_authority": {
      const pem = String(args.pem);
      const subject = pem.split("\n")[1] ?? "Autorité";
      if (!net.view.authorities.some((a) => a.pem === pem)) {
        net.view.authorities.push({ ...certificateLotH(subject), pem, addedAt: 1 });
      }
      return structuredClone(net.view);
    }
    case "network_remove_authority":
      net.view.authorities = net.view.authorities.filter((a) => a.fingerprint !== args.fingerprint);
      return structuredClone(net.view);
    case "project_auth_load":
      return authView(net, project, environment);
    case "project_auth_save": {
      const auth = JSON.parse(JSON.stringify(args.auth)) as Auth;
      if (args.clientSecret) {
        net.hasClientSecret = true;
        auth.oauth2.clientSecret = "{{oauth2ClientSecret}}";
      }
      net.auth.set(project, auth);
      net.token = null;
      return authView(net, project, environment);
    }
    case "oauth2_token_renew":
      net.token = {
        obtainedAt: Date.UTC(2026, 10, 6, 10, 42),
        expiresAt: Date.UTC(2026, 10, 6, 11, 42),
      };
      return authView(net, project, environment);
    case "oauth2_token_forget":
      net.token = null;
      return authView(net, project, environment);
    case "cookies_list":
      return structuredClone(jar(net, environment));
    case "cookies_clear": {
      const current = jar(net, environment);
      current.domains = current.domains.filter((d) => args.domain && d.domain !== args.domain);
      net.cookies.set(environment, current);
      return structuredClone(current);
    }
    case "cookies_set_sending": {
      const current = jar(net, environment);
      for (const d of current.domains)
        if (d.domain === args.domain) d.sending = Boolean(args.enabled);
      net.cookies.set(environment, current);
      return structuredClone(current);
    }
    default:
      return undefined;
  }
}
