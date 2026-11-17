//! Une seule instance d'Arnod par poste (plan qualité, ES8 ; décision Q1 : fichier verrou écrit par
//! Arnod, sans plugin).
//!
//! Deux Arnod lancés ensemble s'écraseraient `arnod.json`, `windows.json` et `statuses.json`. Au
//! démarrage, Arnod prend un verrou exclusif du système sur `arnod.lock`, dans son dossier de
//! données, et le garde jusqu'à sa sortie ; le système le rend même après un arrêt brutal. Une
//! seconde instance attend un peu (le temps qu'Arnod se relance après une mise à jour), puis
//! s'arrête en le disant.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// Fichier verrou, dans le dossier de données.
pub const LOCK_FILE: &str = "arnod.lock";

/// Attente d'une instance qui se ferme (redémarrage après une mise à jour).
pub const WAIT: Duration = Duration::from_secs(5);

/// Intervalle entre deux essais pendant l'attente.
const STEP: Duration = Duration::from_millis(100);

/// Message de la seconde instance, avant de s'arrêter.
pub const ALREADY_RUNNING: &str = "Arnod est déjà ouvert sur ce poste : revenez à sa fenêtre. \
     Ouverts ensemble, deux Arnod écraseraient l'un l'autre la liste des projets et des onglets.";

/// Verrou tenu par l'instance en cours : le garder vivant jusqu'à la sortie.
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
}

/// Issue de la prise du verrou.
#[derive(Debug)]
pub enum Acquired {
    /// Cette instance est la seule.
    Alone(InstanceLock),
    /// Une autre instance tient le verrou.
    AlreadyRunning,
    /// Verrou impossible à poser (dossier inaccessible, système sans verrou) : Arnod démarre
    /// quand même, l'écran de démarrage (lot G) dira ce qui ne va pas avec le dossier.
    Unavailable,
}

/// Prend le verrou de `data_dir`, en réessayant pendant `wait` s'il est tenu.
pub fn acquire(data_dir: &Path, wait: Duration) -> Acquired {
    if fs::create_dir_all(data_dir).is_err() {
        return Acquired::Unavailable;
    }
    let Ok(file) = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data_dir.join(LOCK_FILE))
    else {
        return Acquired::Unavailable;
    };
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => return Acquired::Alone(InstanceLock { _file: file }),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => thread::sleep(STEP),
            Err(TryLockError::WouldBlock) => return Acquired::AlreadyRunning,
            Err(TryLockError::Error(_)) => return Acquired::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn es8_une_seconde_instance_trouve_le_verrou_tenu() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire(dir.path(), Duration::ZERO);
        assert!(matches!(first, Acquired::Alone(_)), "{first:?}");
        // Le verrou du système vaut par ouverture du fichier : une seconde ouverture le trouve
        // tenu, comme le ferait un second Arnod.
        let second = acquire(dir.path(), Duration::from_millis(250));
        assert!(matches!(second, Acquired::AlreadyRunning), "{second:?}");
        drop(first);
        let third = acquire(dir.path(), Duration::ZERO);
        assert!(matches!(third, Acquired::Alone(_)), "{third:?}");
    }

    #[test]
    fn attend_qu_une_instance_qui_se_ferme_rende_le_verrou() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire(dir.path(), Duration::ZERO);
        let release = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            drop(first);
        });
        let next = acquire(dir.path(), Duration::from_secs(5));
        release.join().unwrap();
        assert!(matches!(next, Acquired::Alone(_)), "{next:?}");
    }

    #[test]
    fn un_dossier_impossible_a_creer_n_empeche_pas_de_demarrer() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fichier");
        fs::write(&file, "x").unwrap();
        assert!(matches!(
            acquire(&file.join("donnees"), Duration::ZERO),
            Acquired::Unavailable
        ));
    }
}
