//! Coffre des secrets (R9) : variables locales marquées secrètes.
//!
//! Sous Windows, gestionnaire d'identification ; sous Linux, Secret Service.
//! Le trait permet de tester sans toucher au coffre du poste.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{CommandError, CommandErrorKind};

/// Service sous lequel Arnod range ses secrets.
const SERVICE: &str = "io.github.juliencorre.arnod";

/// Lieu de stockage des secrets, par clé (`arnod/<projet>/<environnement>/<nom>`).
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, CommandError>;
    fn set(&self, key: &str, value: &str) -> Result<(), CommandError>;
    fn delete(&self, key: &str) -> Result<(), CommandError>;
}

fn unavailable(error: impl std::fmt::Display) -> CommandError {
    CommandError::new(
        CommandErrorKind::SecretStore,
        format!(
            "Le coffre du système est indisponible ({error}). La valeur n'a pas été enregistrée."
        ),
    )
}

/// Coffre du système (`keyring`).
#[derive(Debug, Default)]
pub struct SystemSecretStore;

impl SecretStore for SystemSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, CommandError> {
        let entry = keyring::Entry::new(SERVICE, key).map_err(unavailable)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(unavailable(error)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), CommandError> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|entry| entry.set_password(value))
            .map_err(unavailable)
    }

    fn delete(&self, key: &str) -> Result<(), CommandError> {
        let entry = keyring::Entry::new(SERVICE, key).map_err(unavailable)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(unavailable(error)),
        }
    }
}

/// Coffre en mémoire, pour les tests.
#[derive(Debug, Default)]
pub struct MemorySecretStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemorySecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, CommandError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| unavailable("verrou"))?
            .get(key)
            .cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), CommandError> {
        self.0
            .lock()
            .map_err(|_| unavailable("verrou"))?
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), CommandError> {
        self.0
            .lock()
            .map_err(|_| unavailable("verrou"))?
            .remove(key);
        Ok(())
    }
}

/// Clé d'un secret.
pub fn key(project: impl std::fmt::Display, environment: &str, name: &str) -> String {
    format!("arnod/{project}/{environment}/{name}")
}
