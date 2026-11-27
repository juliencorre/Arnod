//! Secret saisi en clair dans l'authentification (lot K, écran 4.2-c) : le jeton, le mot de passe
//! ou la clé d'API d'une requête partent dans son fichier, donc dans le dépôt de l'équipe. Arnod
//! le signale et propose une variable locale secrète (valeur au coffre du système) dont le nom est
//! libre parmi les variables connues.

use serde::Serialize;

use crate::request::model::{AuthKind, Request};

/// Champ de l'authentification qui porte un secret en clair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ClearSecret {
    /// Champ de `Auth` : `token`, `password`, `value` ou `oauth2.clientSecret`.
    pub field: String,
    /// Ce qu'il porte, pour le message (« jeton », « mot de passe », « clé d'API »).
    pub label: String,
    /// Nom proposé pour la variable locale, libre parmi `known`.
    pub suggested: String,
}

/// Une valeur est en clair si elle n'est ni vide ni faite d'une référence `{{…}}`.
fn in_clear(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.contains("{{")
}

/// Nom libre : `base`, sinon `base2`, `base3`…
fn free(base: &str, known: &[String]) -> String {
    let taken = |name: &str| known.iter().any(|k| k.eq_ignore_ascii_case(name));
    if !taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base}{n}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}

/// Secrets en clair de l'authentification de `request`, avec le nom proposé.
pub fn clear_secrets(request: &Request, known: &[String]) -> Vec<ClearSecret> {
    let auth = &request.auth;
    let found: Option<(&str, &str, &str, &str)> = match auth.kind {
        AuthKind::Bearer => Some(("token", "jeton", auth.token.as_str(), "token")),
        AuthKind::Basic => Some((
            "password",
            "mot de passe",
            auth.password.as_str(),
            "password",
        )),
        AuthKind::ApiKey => Some(("value", "clé d'API", auth.value.as_str(), "apiKey")),
        AuthKind::OAuth2 => Some((
            "oauth2.clientSecret",
            "secret client",
            auth.oauth2.client_secret.as_str(),
            "clientSecret",
        )),
        _ => None,
    };
    found
        .filter(|(_, _, value, _)| in_clear(value))
        .map(|(field, label, _, base)| ClearSecret {
            field: field.to_owned(),
            label: label.to_owned(),
            suggested: free(base, known),
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_auth(kind: AuthKind, f: impl FnOnce(&mut crate::request::model::Auth)) -> Request {
        let mut request = Request::new("t");
        request.auth.kind = kind;
        f(&mut request.auth);
        request
    }

    #[test]
    fn k1_jeton_en_clair_signale_avec_un_nom_libre() {
        let request = with_auth(AuthKind::Bearer, |a| a.token = "eyJhbGciOi.xyz".into());
        assert_eq!(
            clear_secrets(&request, &["token".to_owned(), "Token2".to_owned()]),
            [ClearSecret {
                field: "token".into(),
                label: "jeton".into(),
                suggested: "token3".into(),
            }]
        );
    }

    #[test]
    fn k2_une_reference_ou_un_champ_vide_n_est_pas_un_secret_en_clair() {
        let reference = with_auth(AuthKind::Bearer, |a| a.token = "{{token}}".into());
        let empty = with_auth(AuthKind::Basic, |a| a.password = "  ".into());
        let mixed = with_auth(AuthKind::ApiKey, |a| a.value = "pre-{{cle}}".into());
        let inherited = with_auth(AuthKind::Inherit, |a| a.token = "abc".into());
        for request in [reference, empty, mixed, inherited] {
            assert_eq!(clear_secrets(&request, &[]), []);
        }
    }

    #[test]
    fn k3_mot_de_passe_cle_d_api_et_secret_client() {
        let basic = with_auth(AuthKind::Basic, |a| a.password = "s3cret".into());
        let key = with_auth(AuthKind::ApiKey, |a| a.value = "abc123".into());
        let oauth = with_auth(AuthKind::OAuth2, |a| a.oauth2.client_secret = "xyz".into());
        let fields: Vec<(String, String)> = [basic, key, oauth]
            .iter()
            .flat_map(|r| clear_secrets(r, &[]))
            .map(|s| (s.field, s.suggested))
            .collect();
        assert_eq!(
            fields,
            [
                ("password".into(), "password".into()),
                ("value".into(), "apiKey".into()),
                ("oauth2.clientSecret".into(), "clientSecret".into()),
            ]
        );
    }
}
