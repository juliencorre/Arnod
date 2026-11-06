/**
 * Types échangés entre l'interface et le cœur Rust via IPC.
 *
 * Maintenus à la main : chaque type doit refléter exactement la
 * sérialisation serde de son équivalent Rust (champs en camelCase).
 * Toute modification côté Rust doit être reportée ici, et inversement.
 */

/** Noms des commandes Rust exposées par `src-tauri/src/commands/`. */
export type CommandName = "hello";

/**
 * Nature d'une erreur de commande.
 *
 * Équivalent Rust : `CommandErrorKind` dans `src-tauri/src/error.rs`. Un test Rust
 * vérifie que les deux listes concordent (pas de guillemet droit dans les commentaires
 * de cette déclaration : le test en extrait les chaînes).
 */
export type CommandErrorKind =
  | "invalidName" // Nom vide, trop long ou inutilisable comme nom de dossier.
  | "nameTaken" // Nom déjà utilisé, sans tenir compte de la casse.
  | "folderNotEmpty" // Le dossier cible existe déjà et n'est pas vide.
  | "alreadyRegistered" // Le projet est déjà inscrit au référentiel.
  | "notAProject" // Le dossier ne contient pas de manifeste arnod.yml.
  | "notFound" // Projet ou workspace introuvable.
  | "defaultWorkspace" // Action interdite sur le workspace par défaut.
  | "undoExpired" // Le jeton d'annulation n'est plus valable.
  | "io" // Erreur d'entrée-sortie : disque, droits, chemin.
  | "corruptState" // Le fichier d'état de l'application est illisible.
  | "newerVersion"; // Fichier d'état écrit par une version plus récente d'Arnod.

/**
 * Erreur renvoyée par une commande Rust qui échoue : la promesse d'`invoke` est
 * rejetée avec cet objet.
 *
 * Équivalent Rust : `CommandError` dans `src-tauri/src/error.rs`.
 */
export interface CommandError {
  /** Nature de l'erreur, pour réagir selon le cas. */
  kind: CommandErrorKind;
  /** Message en français, prêt à afficher. */
  message: string;
}

/**
 * Réponse de la commande `hello`.
 *
 * Équivalent Rust : `HelloResponse` dans `src-tauri/src/commands/hello.rs`.
 */
export interface HelloResponse {
  /** Texte de salutation produit par le cœur natif (« Hello World »). */
  message: string;
  /** Heure du cœur natif au moment de l'appel, en millisecondes depuis l'epoch Unix (UTC). */
  timestampMs: number;
}
