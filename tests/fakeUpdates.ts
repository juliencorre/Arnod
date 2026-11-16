/**
 * Faux cœur des mises à jour (écran 1.6-g), en mémoire. Le choix de la version, l'échéance, le
 * manifeste, la signature et l'installation sont testés côté Rust (`updates.rs`,
 * `commands/updates.rs`) ; ici, `available` est la version que la prochaine vérification
 * trouvera et `downloadOutcome` dit comment finira le prochain téléchargement.
 */
import { emit } from "@tauri-apps/api/event";

import { EVENTS } from "@/lib/ipc";
import type { CommandError, Release, UpdatesView } from "@/types/ipc";

export interface FakeUpdates {
  view: UpdatesView;
  /** Version trouvée à la prochaine vérification ; `null` : à jour. */
  available: Release | null;
  /** Une vérification automatique est due. */
  due: boolean;
  opened: string[];
  /** Lot L : la version trouvée s'installe depuis Arnod (manifeste, clé, AppImage ou NSIS). */
  installable: boolean;
  /**
   * Lot L : fin du prochain téléchargement. `ready` : paquet vérifié (progression émise avant) ;
   * `pending` : reste en cours ; `{ failed }` : refusé avec ce message.
   */
  downloadOutcome: "ready" | "pending" | { failed: string };
  /** Lot L : `now` de chaque `updates_install`. */
  installs: boolean[];
}

export function createFakeUpdates(): FakeUpdates {
  return {
    view: {
      current: "0.9.0",
      prefs: { automatic: true, channel: "stable", lastCheck: null },
      latest: null,
      checked: false,
      installable: false,
      install: { state: "idle" },
    },
    available: null,
    due: false,
    opened: [],
    installable: false,
    downloadOutcome: "ready",
    installs: [],
  };
}

function fail(kind: CommandError["kind"], message: string): never {
  throw { kind, message } satisfies CommandError;
}

async function download(fake: FakeUpdates): Promise<UpdatesView> {
  const version = fake.view.latest?.version;
  if (!version || !fake.view.installable) {
    fail("invalidRequest", "Cette version ne s'installe pas depuis Arnod.");
  }
  fake.view.install = { state: "downloading", received: 0, total: 10_485_760 };
  await emit(EVENTS.updates, structuredClone(fake.view));
  const outcome = fake.downloadOutcome;
  if (outcome === "pending") {
    fake.view.install = { state: "downloading", received: 3_250_000, total: 10_485_760 };
    await emit(EVENTS.updates, structuredClone(fake.view));
    return structuredClone(fake.view);
  }
  if (typeof outcome === "object") {
    fake.view.install = { state: "failed", message: outcome.failed };
    await emit(EVENTS.updates, structuredClone(fake.view));
    fail("invalidRequest", outcome.failed);
  }
  fake.view.install = { state: "ready", version, onExit: false };
  await emit(EVENTS.updates, structuredClone(fake.view));
  return structuredClone(fake.view);
}

/** Commandes des mises à jour ; `undefined` si `cmd` n'en est pas une. */
export function handleUpdates(
  fake: FakeUpdates,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "updates_load":
      return structuredClone(fake.view);
    case "updates_save":
      fake.view.prefs.automatic = Boolean(args.automatic);
      fake.view.prefs.channel = args.channel === "prerelease" ? "prerelease" : "stable";
      return structuredClone(fake.view);
    case "updates_check":
      if (args.automatic === true && !fake.due) return structuredClone(fake.view);
      fake.view.latest = fake.available;
      fake.view.installable = fake.available !== null && fake.installable;
      fake.view.checked = true;
      fake.view.prefs.lastCheck = Date.UTC(2026, 10, 14, 8, 12);
      return structuredClone(fake.view);
    case "updates_open":
      fake.opened.push(String(args.url));
      return null;
    case "updates_download":
      return download(fake);
    case "updates_cancel":
      fake.view.install = { state: "idle" };
      return structuredClone(fake.view);
    case "updates_install": {
      const ready = fake.view.install;
      if (ready.state !== "ready") fail("invalidRequest", "Aucune version téléchargée.");
      fake.installs.push(args.now === true);
      // `now` : l'application réelle se ferme et ne répond pas.
      if (args.now === true) return new Promise(() => undefined);
      fake.view.install = { ...ready, onExit: true };
      return structuredClone(fake.view);
    }
    default:
      return undefined;
  }
}
