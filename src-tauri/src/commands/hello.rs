//! Domaine « hello » : commande de démonstration de la chaîne IPC
//! interface → cœur natif.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Texte renvoyé par la commande `hello`.
pub const HELLO_MESSAGE: &str = "Hello World";

/// Réponse de la commande `hello`.
///
/// Miroir TypeScript : `HelloResponse` dans `src/types/ipc.ts`.
/// Toute modification ici doit être reportée de l'autre côté.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloResponse {
    /// Texte de salutation produit par le cœur natif.
    pub message: String,
    /// Heure du cœur natif au moment de l'appel, en millisecondes
    /// depuis l'epoch Unix (UTC).
    pub timestamp_ms: u64,
}

/// Commande Tauri `hello` : renvoie « Hello World » et l'heure du cœur natif.
#[tauri::command]
pub fn hello() -> HelloResponse {
    hello_at(SystemTime::now())
}

/// Construit la réponse pour un instant donné (séparé pour les tests).
fn hello_at(now: SystemTime) -> HelloResponse {
    let timestamp_ms = now
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        // Horloge système antérieure à 1970 : on renvoie l'epoch plutôt que d'échouer.
        .unwrap_or(0);

    HelloResponse {
        message: HELLO_MESSAGE.to_owned(),
        timestamp_ms,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn renvoie_hello_world() {
        assert_eq!(hello().message, "Hello World");
    }

    #[test]
    fn horodatage_en_millisecondes_unix() {
        let now = UNIX_EPOCH + Duration::from_millis(1_762_400_000_123);
        assert_eq!(hello_at(now).timestamp_ms, 1_762_400_000_123);
    }

    #[test]
    fn horodatage_proche_de_l_heure_courante() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let response = hello();
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let timestamp = u128::from(response.timestamp_ms);
        assert!(before <= timestamp && timestamp <= after);
    }

    #[test]
    fn horloge_avant_1970_renvoie_zero() {
        let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
        assert_eq!(hello_at(before_epoch).timestamp_ms, 0);
    }

    #[test]
    fn serialisation_json_en_camel_case() {
        let response = hello_at(UNIX_EPOCH + Duration::from_millis(42));
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "message": "Hello World", "timestampMs": 42 })
        );
    }
}
