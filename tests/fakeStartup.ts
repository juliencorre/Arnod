/**
 * Lot G - Démarrage et erreurs : faux cœur des commandes `startup_*` et `window_ready`.
 *
 * `problem` est l'erreur bloquante servie par `startup_status` ; `retry` et `startEmpty`
 * disent ce que renvoient les issues (`null` : réglé). `slow` laisse `catalog_load` en attente,
 * pour montrer le squelette du démarrage lent (0.3-a).
 */
import type { StartupProblem } from "@/types/ipc";

export interface FakeStartup {
  problem: StartupProblem | null;
  /** Erreur renvoyée par le prochain `startup_retry` (sinon `null` : réglé). */
  afterRetry: StartupProblem | null;
  /** `catalog_load` ne répond jamais : démarrage lent. */
  slow: boolean;
  /** Échec de `startup_open` (navigateur absent). */
  openFails: boolean;
  shown: number;
  quit: boolean;
}

export function createFakeStartup(): FakeStartup {
  return { problem: null, afterRetry: null, slow: false, openFails: false, shown: 0, quit: false };
}

/** Commandes du démarrage ; `undefined` si `cmd` n'en est pas une. */
export function handleStartupLotG(
  startup: FakeStartup,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "startup_status":
      return structuredClone(startup.problem);
    case "startup_retry":
      startup.problem = startup.afterRetry;
      return structuredClone(startup.problem);
    case "startup_start_empty":
      startup.problem = null;
      return null;
    case "startup_open":
      if (startup.openFails) {
        throw { kind: "io", message: `Ouverture impossible (${String(args.target)}).` };
      }
      return null;
    case "startup_quit":
      startup.quit = true;
      return null;
    case "window_ready":
      startup.shown += 1;
      return null;
    default:
      return undefined;
  }
}

const ENV_ROWS = [
  { label: "Arnod", value: "0.0.2 · Windows x86_64 · WebView 131" },
  { label: "Date", value: "10/11/2026 09:12:04 UTC" },
];

/** Erreur « version plus récente » d'`arnod.json`, comme la décrit le cœur (0.3-b). */
export function versionPlusRecenteLotG(): StartupProblem {
  const details = [
    { label: "Nature", value: "newerVersion" },
    { label: "Fichier", value: "C:\\Users\\julien\\AppData\\Local\\Arnod\\arnod.json" },
    { label: "Format du fichier", value: "3 (écrit par Arnod 1.0.2, le 08/11/2026)" },
    { label: "Formats lus ici", value: "1 et 2" },
    ...ENV_ROWS,
  ];
  return {
    nature: "newerVersion",
    file: "arnod.json",
    title: "Cet état vient d'une version plus récente d'Arnod",
    message:
      "Le fichier d'état a été écrit par Arnod 1.0.2 (format 3). Vous lancez Arnod 0.0.2, qui lit les formats 1 et 2 : il ne l'ouvre pas, pour ne rien abîmer.",
    intactLead: "Rien n'a été modifié.",
    intactText:
      "Ce fichier est intact, et vos projets aussi : leurs dossiers et leurs fichiers ne dépendent pas de cet état.",
    options: [
      {
        action: "openReleases",
        title: "Installer Arnod 1.0.2 ou plus récent",
        description:
          "La nouvelle version relit cet état tel quel : projets, workspaces et onglets reviennent.",
        button: "Ouvrir la page des versions",
        recommended: true,
        note: null,
      },
      {
        action: "startEmpty",
        title: "Repartir d'un état vide",
        description:
          "L'état actuel est mis de côté sous arnod.json.format-3, sans être modifié. Arnod démarre sans projet ; rouvrez les vôtres avec « Ouvrir un projet ».",
        button: "Repartir d'un état vide",
        recommended: false,
        note: null,
      },
    ],
    pathLabel: "Fichier d'état",
    path: "C:\\Users\\julien\\AppData\\Local\\Arnod\\arnod.json",
    retry: null,
    details,
    supportText: `Arnod ne démarre pas : Cet état vient d'une version plus récente d'Arnod\n\n${details
      .map((row) => `${row.label}  ${row.value}`)
      .join("\n")}\n`,
    issueUrl: "https://github.com/juliencorre/Arnod/issues/new?title=D%C3%A9marrage&body=x",
    footer: "Arnod 0.0.2 · Windows",
  };
}

/** Fichier d'état verrouillé (0.3-b bis) : essai automatique dans `delay` secondes. */
export function fichierVerrouilleLotG(delay = 4): StartupProblem {
  const base = versionPlusRecenteLotG();
  const details = [
    { label: "Nature", value: "locked" },
    { label: "Fichier", value: base.path },
    { label: "Erreur technique", value: "Accès refusé. (os error 5) (code 5)" },
    ...ENV_ROWS,
  ];
  return {
    ...base,
    nature: "locked",
    title: "Arnod n'arrive pas à lire son état",
    message:
      "Accès refusé au fichier d'état (erreur système 5). Il est sans doute ouvert par un autre programme : un antivirus qui l'analyse, ou une synchronisation (OneDrive, Dropbox) de ce dossier.",
    intactText: "Arnod n'a rien écrit ; vos projets ne dépendent pas de ce fichier.",
    options: [
      {
        action: "retry",
        title: "Réessayer",
        description: "Le plus souvent, le verrou se libère en quelques secondes.",
        button: "Réessayer",
        recommended: true,
        note: null,
      },
      {
        action: "openFolder",
        title: "Vérifier le dossier de données",
        description:
          "Excluez-le de la synchronisation, ou vérifiez que votre compte peut lire et écrire ce fichier.",
        button: "Ouvrir le dossier",
        recommended: false,
        note: null,
      },
    ],
    retry: { attempt: 1, maxAttempts: 3, delaySeconds: delay },
    details,
  };
}
