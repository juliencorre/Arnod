/**
 * Faux cœur des mises à jour (lot L allégé, écran 1.6-g), en mémoire. Le choix de la version et
 * l'échéance sont testés côté Rust (`updates.rs`) ; ici, `available` est la version que la
 * prochaine vérification trouvera.
 */
import type { Release, UpdatesView } from "@/types/ipc";

export interface FakeUpdates {
  view: UpdatesView;
  /** Version trouvée à la prochaine vérification ; `null` : à jour. */
  available: Release | null;
  /** Une vérification automatique est due. */
  due: boolean;
  opened: string[];
}

export function createFakeUpdates(): FakeUpdates {
  return {
    view: {
      current: "0.9.0",
      prefs: { automatic: true, channel: "stable", lastCheck: null },
      latest: null,
      checked: false,
    },
    available: null,
    due: false,
    opened: [],
  };
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
      fake.view.checked = true;
      fake.view.prefs.lastCheck = Date.UTC(2026, 10, 14, 8, 12);
      return structuredClone(fake.view);
    case "updates_open":
      fake.opened.push(String(args.url));
      return null;
    default:
      return undefined;
  }
}
