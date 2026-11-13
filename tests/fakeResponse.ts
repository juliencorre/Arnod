/**
 * Faux cœur du corps de la réponse (lot A, `docs/plan-mvp-a-reponse.md`) : mise en forme,
 * aperçu, filtre JSONPath, enregistrement et effacement, en version simplifiée. Le filtre ne
 * connaît que `$`, `.nom`, `['nom']`, `[n]`, `[*]` et `.*` ; les règles exactes (RFC 9535,
 * XML et HTML) sont testées côté Rust (`src-tauri/src/response/tests.rs`).
 */
import type {
  Assertion,
  BodyToken,
  FilterMatch,
  FilterResult,
  HtmlNode,
  RenderedBody,
  RequestResult,
  ResponseFormat,
} from "@/types/ipc";

import type { FakeWorkbench } from "./fakeWorkbench";

export interface FakeResponse {
  /** Fichier renvoyé par le prochain `response_save` (`null` : annulé). */
  savePath: string | null;
}

export function createFakeResponse(): FakeResponse {
  return { savePath: "/home/julien/Créer une commande.json" };
}

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

function detect(headers: [string, string][], body: string): ResponseFormat {
  const type = headers.find(([name]) => name.toLowerCase() === "content-type")?.[1] ?? "";
  if (/[/+]json/.test(type)) return "json";
  if (type.includes("html")) return "html";
  if (/[/+]xml/.test(type)) return "xml";
  const start = body.trimStart();
  if (start.startsWith("{") || start.startsWith("[")) {
    try {
      JSON.parse(body);
      return "json";
    } catch {
      return "text";
    }
  }
  if (start.toLowerCase().startsWith("<!doctype html") || start.startsWith("<html")) return "html";
  return start.startsWith("<") ? "xml" : "text";
}

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;

function child(path: string, key: string | number): string {
  if (typeof key === "number") return `${path}[${String(key)}]`;
  return IDENTIFIER.test(key) ? `${path}.${key}` : `${path}['${key.replace(/['\\]/g, "\\$&")}']`;
}

function scalar(value: Json): boolean {
  return value === null || typeof value !== "object";
}

function jsonLines(root: Json): BodyToken[][] {
  const lines: BodyToken[][] = [];
  let line: BodyToken[] = [];
  const push = (text: string, kind: BodyToken["kind"], path?: string) =>
    line.push(path === undefined ? { text, kind } : { text, kind, path });
  const newline = (depth: number) => {
    lines.push(line);
    line = depth > 0 ? [{ text: "  ".repeat(depth), kind: "text" }] : [];
  };
  const write = (value: Json, path: string, depth: number): void => {
    if (value === null || typeof value !== "object") {
      const kind =
        typeof value === "string" ? "string" : typeof value === "number" ? "number" : "literal";
      push(JSON.stringify(value), kind, path);
      return;
    }
    const entries: [string | number, Json][] = Array.isArray(value)
      ? value.map((v, i) => [i, v])
      : Object.entries(value);
    const [open, close] = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
    push(open, "punct", path);
    const inline =
      entries.every(([, v]) => scalar(v)) && JSON.stringify(value).length + 2 * depth <= 60;
    entries.forEach(([key, v], index) => {
      if (inline) push(index === 0 ? (Array.isArray(value) ? "" : " ") : ", ", "punct");
      else {
        if (index > 0) push(",", "punct");
        newline(depth + 1);
      }
      if (typeof key === "string") {
        push(JSON.stringify(key), "key");
        push(": ", "punct");
      }
      write(v, child(path, key), depth + 1);
    });
    if (entries.length > 0) {
      if (inline) push(Array.isArray(value) ? "" : " ", "punct");
      else newline(depth);
    }
    push(close, "punct", path);
  };
  write(root, "$", 0);
  newline(0);
  return lines.map((l) => l.filter((t) => t.text !== ""));
}

/** Découpe linéaire en commentaires, balises et textes (sans expression régulière). */
function splitMarkup(body: string): string[] {
  const items: string[] = [];
  let i = 0;
  while (i < body.length) {
    if (body[i] !== "<") {
      const next = body.indexOf("<", i);
      const end = next === -1 ? body.length : next;
      items.push(body.slice(i, end));
      i = end;
      continue;
    }
    const comment = body.startsWith("<!--", i);
    const close = comment ? body.indexOf("-->", i + 4) : body.indexOf(">", i + 1);
    const end = close === -1 ? body.length : close + (comment ? 3 : 1);
    items.push(body.slice(i, end));
    i = end;
  }
  return items;
}

/** Balise lue sans expression régulière : fermante, nom, attributs, auto-fermante. */
function parseTag(raw: string): { close: string; name: string; attrs: string; self: string } {
  let inner = raw.slice(1, -1);
  const close = inner.startsWith("/") ? "/" : "";
  if (close) inner = inner.slice(1);
  const self = inner.endsWith("/") ? "/" : "";
  if (self) inner = inner.slice(0, -1);
  const space = inner.search(/\s/);
  const name = space === -1 ? inner : inner.slice(0, space);
  return { close, name, attrs: space === -1 ? "" : inner.slice(space), self };
}

/** Balises une par ligne, indentées ; texte court sur la ligne de sa balise (R3, simplifié). */
function markupLines(body: string): BodyToken[][] {
  const items = splitMarkup(body);
  const lines: BodyToken[][] = [];
  let depth = 0;
  const indent = (): BodyToken[] => (depth > 0 ? [{ text: "  ".repeat(depth), kind: "text" }] : []);
  const tag = (raw: string): BodyToken[] => {
    if (raw.startsWith("<!--")) return [{ text: raw, kind: "comment" }];
    if (raw.startsWith("<?") || raw.startsWith("<!")) return [{ text: raw, kind: "punct" }];
    if (!raw.endsWith(">")) return [{ text: raw, kind: "text" }];
    const { close, name, attrs, self } = parseTag(raw);
    const tokens: BodyToken[] = [
      { text: `<${close}`, kind: "punct" },
      { text: name, kind: "tag" },
    ];
    for (const a of attrs.matchAll(/([^\s=]+)(?:=("[^"]*"|'[^']*'|\S+))?/g)) {
      tokens.push({ text: " ", kind: "text" }, { text: a[1] ?? "", kind: "attr" });
      if (a[2] !== undefined) {
        tokens.push({ text: "=", kind: "punct" }, { text: a[2], kind: "attrValue" });
      }
    }
    tokens.push({ text: `${self}>`, kind: "punct" });
    return tokens;
  };
  for (let i = 0; i < items.length; i++) {
    const item = items[i] ?? "";
    if (!item.startsWith("<")) {
      if (item.trim() !== "") lines.push([...indent(), { text: item.trim(), kind: "text" }]);
    } else if (item.startsWith("</")) {
      depth = Math.max(0, depth - 1);
      lines.push([...indent(), ...tag(item)]);
    } else if (item.startsWith("<!") || item.startsWith("<?") || item.endsWith("/>")) {
      lines.push([...indent(), ...tag(item)]);
    } else {
      const next = items[i + 1] ?? "";
      const after = items[i + 2] ?? "";
      if (!next.startsWith("<") && after.startsWith("</")) {
        lines.push([...indent(), ...tag(item), { text: next.trim(), kind: "text" }, ...tag(after)]);
        i += 2;
      } else {
        lines.push([...indent(), ...tag(item)]);
        depth++;
      }
    }
  }
  return lines;
}

function render(body: string, headers: [string, string][], chosen: ResponseFormat | null) {
  const detected = detect(headers, body);
  const format = chosen ?? detected;
  const plain = () => body.split("\n").map((text): BodyToken[] => [{ text, kind: "text" }]);
  let notice: string | null = null;
  let lines: BodyToken[][] = [];
  if (body !== "") {
    if (format === "json") {
      try {
        lines = jsonLines(JSON.parse(body) as Json);
      } catch {
        notice = "Ce corps n'est pas du JSON valide (ligne 1, colonne 1) : affiché tel quel.";
        lines = plain();
      }
    } else if (format === "text") lines = plain();
    else lines = markupLines(body);
  }
  return { format, detected, lines, notice } satisfies RenderedBody;
}

/** Titres, paragraphes et lignes de liste, sans tête ni script (R4, simplifié). */
function preview(body: string): HtmlNode[] {
  const KEPT = new Set(["h1", "h2", "h3", "h4", "h5", "h6", "p", "li", "pre"]);
  const HIDDEN = new Set(["head", "script", "style"]);
  const nodes: HtmlNode[] = [];
  let hidden: string | null = null;
  let open: { tag: string; text: string } | null = null;
  for (const item of splitMarkup(body)) {
    if (!item.startsWith("<")) {
      if (open && hidden === null) open.text += item;
      continue;
    }
    if (item.startsWith("<!")) continue;
    const { close, name } = parseTag(item);
    const tag = name.toLowerCase();
    if (hidden !== null) {
      if (close && tag === hidden) hidden = null;
      continue;
    }
    if (!close && HIDDEN.has(tag)) hidden = tag;
    else if (!close && KEPT.has(tag)) open = { tag, text: "" };
    else if (close && open?.tag === tag) {
      nodes.push({ kind: "element", tag, children: [{ kind: "text", text: open.text }] });
      open = null;
    }
  }
  return nodes;
}

function assertionFor(path: string, value: Json): Assertion | null {
  const target = path.replace(/^\$/, "body");
  if (target.includes("['")) return null;
  const base = { id: `filtre-${target.replace(/[^A-Za-z0-9]/g, "-")}`, target };
  const common = { origin: "proposed" as const, enabled: true };
  if (value === null) return { ...base, operator: "isNull", ...common };
  if (Array.isArray(value)) return { ...base, operator: "length", value: value.length, ...common };
  if (typeof value === "object") return { ...base, operator: "exists", ...common };
  return { ...base, operator: "equals", value, ...common };
}

function filter(body: string, expression: string): FilterResult {
  const expr = expression.trim();
  const error = (message: string): FilterResult => ({ matches: [], total: 0, error: message });
  if (expr === "") return { matches: [], total: 0, error: null };
  if (!expr.startsWith("$")) {
    return error("Une expression JSONPath commence par $ (par exemple $.items[*].sku).");
  }
  if ((expr.match(/\[/g) ?? []).length > (expr.match(/\]/g) ?? []).length) {
    return error("Crochet « [ » non fermé dans l'expression.");
  }
  let root: Json;
  try {
    root = JSON.parse(body) as Json;
  } catch {
    return error("Le filtre JSONPath s'applique à un corps JSON : celui-ci n'en est pas.");
  }
  let nodes: [string, Json][] = [["$", root]];
  const segment = /^(?:\.([A-Za-z_][A-Za-z0-9_]*)|\.\*|\[\*\]|\[(\d+)\]|\['([^']*)'\])/;
  let rest = expr.slice(1);
  while (rest !== "") {
    const m = segment.exec(rest);
    if (!m) {
      return error(
        `Expression JSONPath invalide au caractère ${String(expr.length - rest.length + 1)} : « ${rest.slice(0, 12)} ».`,
      );
    }
    const [all, name, index, quoted] = m;
    const key = name ?? quoted;
    nodes = nodes.flatMap(([path, value]): [string, Json][] => {
      if (value === null || typeof value !== "object") return [];
      if (key !== undefined) {
        return !Array.isArray(value) && key in value
          ? [[child(path, key), value[key] ?? null]]
          : [];
      }
      if (index !== undefined) {
        const i = Number(index);
        return Array.isArray(value) && i < value.length ? [[child(path, i), value[i] ?? null]] : [];
      }
      return Array.isArray(value)
        ? value.map((v, i): [string, Json] => [child(path, i), v])
        : Object.entries(value).map(([k, v]): [string, Json] => [child(path, k), v]);
    });
    rest = rest.slice(all.length);
  }
  const matches = nodes.map(([path, value]): FilterMatch => ({
    path,
    value: JSON.stringify(value),
    assertion: assertionFor(path, value),
  }));
  return { matches, total: matches.length, error: null };
}

/** Commandes `response_*` ; `undefined` pour une autre commande. */
export function handleResponse(
  fake: FakeResponse,
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "response_render":
      return render(
        String(args.body),
        args.headers as [string, string][],
        (args.format ?? null) as ResponseFormat | null,
      );
    case "response_preview":
      return preview(String(args.body));
    case "response_filter":
      return filter(String(args.body), String(args.expression));
    case "response_save":
      return fake.savePath;
    case "response_clear": {
      const path = String(args.path);
      const sending = wb.results.get(path)?.sending ?? false;
      wb.results.delete(path);
      return {
        projectId: String(args.project),
        path,
        sending,
        outcome: null,
        error: null,
        receivedAt: null,
        request: null,
      } satisfies RequestResult;
    }
    default:
      return undefined;
  }
}
