/**
 * Faux cœur de la saisie (docs/plan-mvp-b-saisie-curl.md) : variables connues et inconnues,
 * en-têtes ajoutés, listes en texte et copie en cURL, en version simplifiée (le vrai
 * comportement est testé côté Rust).
 */
import type {
  AddedHeader,
  CurlCopy,
  Pair,
  PairsFormat,
  PairsText,
  RequestDoc,
  RequestInput,
  TextError,
  VariableSource,
} from "@/types/ipc";

import type { FakeWorkbench } from "./fakeWorkbench";

/** Noms des `{{…}}` d'un texte. */
function names(text: string): string[] {
  return [...text.matchAll(/\{\{\s*([^}]*?)\s*\}\}/g)].map((m) => m[1] ?? "");
}

function used(request: RequestDoc): string[] {
  const texts = [
    request.url,
    ...request.params.filter((p) => p.enabled).flatMap((p) => [p.name, p.value]),
    ...request.headers.filter((p) => p.enabled).flatMap((p) => [p.name, p.value]),
    request.auth.token,
    request.auth.username,
    request.auth.password,
    ...request.body.variants.map((v) => v.content),
  ];
  return [...new Set(texts.flatMap(names))].filter((n) => n !== "" && !n.startsWith("$"));
}

function added(request: RequestDoc): AddedHeader[] {
  const written = new Set(
    request.headers.filter((h) => h.enabled).map((h) => h.name.toLowerCase()),
  );
  const list: AddedHeader[] = [];
  if (request.auth.type === "bearer") {
    list.push({ name: "Authorization", value: "Bearer ••••", origin: "auth", source: null });
  }
  const push = (name: string, value: string, origin: AddedHeader["origin"]): void => {
    if (!written.has(name.toLowerCase())) list.push({ name, value, origin, source: null });
  };
  if (request.body.kind === "json") push("Content-Type", "application/json", "body");
  push("User-Agent", "Arnod/0.0.2", "client");
  push("Accept", "*/*", "client");
  if (request.body.kind !== "none") push("Content-Length", "calculé à l'envoi", "client");
  push("Host", "recette.boutique.dev", "client");
  return list;
}

function toText(pairs: Pair[], format: PairsFormat): string {
  return pairs
    .map((p) => {
      const line = format === "header" ? `${p.name}: ${p.value}` : `${p.name}=${p.value}`;
      return p.enabled ? line : `# ${line}`;
    })
    .join("\n");
}

function fromText(text: string, format: PairsFormat, previous: Pair[]): PairsText {
  const pairs: Pair[] = [];
  const errors: TextError[] = [];
  text.split("\n").forEach((raw, index) => {
    let line = raw.trim();
    if (line === "") return;
    const enabled = !line.startsWith("#");
    if (!enabled) line = line.replace(/^#+\s*/, "");
    const at = format === "header" ? line.indexOf(":") : line.search(/[=:]/);
    if (at < 0 && format === "header") {
      errors.push({
        line: index + 1,
        message: `Ligne ${String(index + 1)} : « : » manquant entre le nom et la valeur.`,
      });
      return;
    }
    const name = (at < 0 ? line : line.slice(0, at)).trim();
    const value = at < 0 ? "" : line.slice(at + 1).trim();
    const description = previous.find((p) => p.name === name)?.description ?? "";
    pairs.push({ name, value, enabled, kind: "query", description });
  });
  return { pairs, errors };
}

/** Commande cURL simplifiée : variables de l'environnement résolues, jeton en `$TOKEN`. */
function curl(wb: FakeWorkbench, request: RequestDoc): CurlCopy {
  const env = wb.envs.find((e) => e.name === wb.active);
  const resolve = (text: string): string =>
    text.replace(/\{\{\s*([^}]*?)\s*\}\}/g, (all, name: string) => {
      if (env?.locals.includes(name)) return `$${name.toUpperCase()}`;
      return env?.variables.find((v) => v.name === name)?.value ?? all;
    });
  const quote = (text: string): string => (text.includes("$") ? `"${text}"` : `'${text}'`);
  const lines = [`curl -X ${request.method} '${resolve(request.url)}'`];
  for (const h of [...request.headers.filter((p) => p.enabled), ...request.inherited.headers]) {
    lines.push(`-H ${quote(`${h.name}: ${resolve(h.value)}`)}`);
  }
  if (request.auth.type === "bearer") lines.push(`-H "Authorization: Bearer $TOKEN"`);
  const content = request.body.variants[request.body.selected]?.content ?? "";
  if (request.body.kind !== "none" && content !== "") lines.push(`--data '${content}'`);
  return { command: lines.join(" \\\n  "), environment: wb.active };
}

/** Commandes du lot B ; `undefined` si `cmd` n'en est pas une. */
export function handleInput(
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "request_input": {
      const request = args.request as RequestDoc;
      const environment = String(args.environment);
      const env = wb.envs.find((e) => e.name === environment);
      const known = [...(env?.variables.map((v) => v.name) ?? []), ...(env?.locals ?? [])];
      const unknown = used(request)
        .filter((name) => !known.includes(name))
        .map((name) => ({
          name,
          definedIn: wb.envs
            .filter((e) => e.name !== environment)
            .flatMap((e): VariableSource[] => {
              const v = e.variables.find((x) => x.name === name);
              if (v) return [{ environment: e.name, value: v.value }];
              return e.locals.includes(name) ? [{ environment: e.name, value: null }] : [];
            }),
        }));
      return { environment, known, unknown, added: added(request) } satisfies RequestInput;
    }
    case "request_curl":
      return curl(wb, args.request as RequestDoc);
    case "pairs_to_text":
      return toText(args.pairs as Pair[], args.format as PairsFormat);
    case "pairs_from_text":
      return fromText(String(args.text), args.format as PairsFormat, args.previous as Pair[]);
    default:
      return undefined;
  }
}
