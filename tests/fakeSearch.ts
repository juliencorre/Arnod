/**
 * Faux cœur de la recherche Ctrl+K (1.5-a, 1.5-b), sur les requêtes et dossiers du faux établi.
 * Les règles (sans casse ni accents, adresses, groupes, ordre) sont testées côté Rust
 * (`search/tests.rs`) ; le faux cœur en reprend ce dont les parcours d'interface ont besoin :
 * recherche par nom dans les requêtes et les dossiers, par adresse dans les requêtes.
 */
import type { SearchGroup, SearchHit, SearchResults } from "@/types/ipc";

import { BOUTIQUE_TESTS } from "./fixtures";
import type { FakeWorkbench } from "./fakeWorkbench";

function plain(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

function place(path: string): string {
  const at = path.lastIndexOf("/");
  return at < 0 ? "boutique-tests" : `boutique-tests · ${path.slice(0, at).replaceAll("/", " / ")}`;
}

function group(title: string, hits: SearchHit[]): SearchGroup[] {
  return hits.length === 0 ? [] : [{ title, hits: hits.slice(0, 8), total: hits.length }];
}

export function fakeSearch(wb: FakeWorkbench, text: string): SearchResults {
  const query = text.trim();
  if (query === "") return { text, address: false, groups: [] };
  const requests = [...wb.requests.entries()].sort(([a], [b]) => a.localeCompare(b));
  const address = /^(?:[A-Z]+\s+)?\//.exec(query);
  if (address) {
    const path = query.replace(/^[A-Z]+\s+/, "");
    const calls = requests
      .filter(([, r]) => r.url.replace(/^\{\{[^}]+\}\}/, "").split("?")[0] === path)
      .map(([p, r]): SearchHit => hitOf(p, r.name, r.method, []));
    return { text, address: true, groups: group("Tests qui l'appellent", calls) };
  }
  const words = query.split(/\s+/).map(plain);
  const match = (label: string): [number, number][] | null => {
    const folded = plain(label);
    const out: [number, number][] = [];
    for (const word of words) {
      const at = folded.indexOf(word);
      if (at < 0) return null;
      out.push([at, at + word.length]);
    }
    return out;
  };
  const tests: SearchHit[] = [];
  for (const [path, request] of requests) {
    const highlight = match(request.name);
    if (highlight) tests.push(hitOf(path, request.name, request.method, highlight));
  }
  const folders: SearchHit[] = [];
  for (const path of [...wb.folders].sort()) {
    const name = path.split("/").at(-1) ?? path;
    const highlight = match(name);
    if (highlight)
      folders.push({ ...hitOf(path, name, null, highlight), kind: "folder", method: null });
  }
  return {
    text,
    address: false,
    groups: [...group("Tests", tests), ...group("Dossiers", folders)],
  };
}

function hitOf(
  path: string,
  label: string,
  method: string | null,
  highlight: [number, number][],
): SearchHit {
  return {
    kind: "request",
    projectId: BOUTIQUE_TESTS,
    label,
    detail: place(path),
    method,
    path,
    contract: null,
    highlight,
  };
}
