/**
 * Faux cœur de la synchronisation (`docs/plan-v2.md`, lots E et F) : l'état de chaque projet
 * (relié ou non, ce qui arrive, mes modifications, conflits), réglé par les tests, et les
 * commandes `sync_*` qui le font évoluer comme le vrai cœur (récupérer vide « à récupérer »,
 * envoyer vide les éléments choisis, un conflit attend un choix).
 */
import type {
  FileConflict,
  JournalEntry,
  MissingSecrets,
  SharedItem,
  SyncAccount,
  SyncChoice,
  SyncView,
} from "@/types/ipc";

export interface FakeSync {
  views: Map<string, SyncView>;
  /** Conflits renvoyés par la prochaine récupération, par projet. */
  conflicts: Map<string, FileConflict[]>;
  journal: JournalEntry[];
  missing: MissingSecrets[];
  /** Choix reçus par `sync_resolve`. */
  resolved: Record<string, Record<string, SyncChoice>> | null;
  later: { environment: string; names: string[] }[];
  /** Messages des envois. */
  sent: { items: string[]; message: string }[];
}

export function createFakeSync(): FakeSync {
  return {
    views: new Map(),
    conflicts: new Map(),
    journal: [],
    missing: [],
    resolved: null,
    later: [],
    sent: [],
  };
}

export function unlinked(projectId: string): SyncView {
  return {
    projectId,
    repository: false,
    remote: null,
    account: null,
    incoming: [],
    outgoing: [],
    conflicts: 0,
    checkedAt: null,
    error: null,
    members: [],
    received: {},
  };
}

export const ACCOUNT: SyncAccount = {
  forge: "github",
  host: "github.com",
  login: "julien-corre",
  name: "Julien",
  email: "j@exemple.fr",
};

/** Un projet relié à `acme/boutique-tests`, avec ce qui arrive et ce qui part. */
export function linked(
  projectId: string,
  incoming: SharedItem[] = [],
  outgoing: SharedItem[] = [],
): SyncView {
  return {
    ...unlinked(projectId),
    repository: true,
    remote: {
      url: "https://github.com/acme/boutique-tests.git",
      forge: "github",
      host: "github.com",
      name: "acme/boutique-tests",
      branch: "main",
    },
    account: ACCOUNT,
    incoming,
    outgoing,
    checkedAt: Date.now() - 60_000,
    members: ["Julien", "Marc", "Sophie"],
  };
}

export function item(key: string, label: string, detail: string, author?: string): SharedItem {
  return {
    key,
    kind: "request",
    label,
    place: "Clients",
    detail,
    author: author ?? null,
    at: author ? Date.now() - 2 * 3_600_000 : null,
  };
}

function view(sync: FakeSync, project: string): SyncView {
  return structuredClone(sync.views.get(project) ?? unlinked(project));
}

/** Réponse d'une commande `sync_*`, ou `undefined` si `cmd` n'en est pas une. */
export function handleSync(sync: FakeSync, cmd: string, args: Record<string, unknown>): unknown {
  const project = String(args.project);
  switch (cmd) {
    case "sync_status":
    case "sync_check":
      return view(sync, project);
    case "sync_login_options":
      return { githubBrowser: false, gitlabBrowser: false };
    case "sync_token":
      return { ...ACCOUNT, host: String(args.host) };
    case "sync_repositories":
      return [
        {
          name: "acme/boutique-tests",
          url: "https://github.com/acme/boutique-tests.git",
          private: true,
          empty: true,
        },
      ];
    case "sync_create_repository":
      return {
        name: `julien-corre/${String(args.name)}`,
        url: `https://github.com/julien-corre/${String(args.name)}.git`,
        private: true,
        empty: true,
      };
    case "sync_link": {
      const next = linked(project);
      if (next.remote)
        next.remote = { ...next.remote, url: String(args.url), branch: String(args.branch) };
      // Reçu comme le vrai cœur : en JSON (l'interface peut passer un objet réactif).
      next.account = args.account
        ? (JSON.parse(JSON.stringify(args.account)) as SyncAccount)
        : null;
      sync.views.set(project, next);
      return structuredClone(next);
    }
    case "sync_pull_push": {
      const current = sync.views.get(project) ?? linked(project);
      const conflicts = sync.conflicts.get(project);
      if (args.pull && conflicts && conflicts.length > 0) {
        current.conflicts = conflicts.length;
        sync.views.set(project, current);
        return structuredClone(current);
      }
      if (args.pull) current.incoming = [];
      if (args.push) {
        const items = args.items as string[];
        sync.sent.push({ items, message: String(args.message) });
        current.outgoing = current.outgoing.filter((i) => !items.includes(i.key));
      }
      sync.views.set(project, current);
      return structuredClone(current);
    }
    case "sync_conflicts":
      return { projectId: project, conflicts: structuredClone(sync.conflicts.get(project) ?? []) };
    case "sync_resolve": {
      sync.resolved = args.choices as Record<string, Record<string, SyncChoice>>;
      sync.conflicts.delete(project);
      const current = sync.views.get(project) ?? linked(project);
      current.conflicts = 0;
      current.incoming = [];
      sync.views.set(project, current);
      return structuredClone(current);
    }
    case "sync_abandon": {
      sync.conflicts.delete(project);
      const current = sync.views.get(project) ?? linked(project);
      current.conflicts = 0;
      sync.views.set(project, current);
      return structuredClone(current);
    }
    case "sync_journal":
      return structuredClone(sync.journal);
    case "sync_missing_secrets":
      return structuredClone(sync.missing);
    case "sync_secrets_later":
      sync.later.push({ environment: String(args.environment), names: args.names as string[] });
      sync.missing = [];
      return null;
    case "sync_connect_start":
    case "sync_connect_wait":
    case "sync_connect_cancel":
      return null;
    default:
      return undefined;
  }
}
