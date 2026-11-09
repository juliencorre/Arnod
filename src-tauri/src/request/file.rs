//! Lecture et écriture du fichier d'une requête, au format OpenCollection
//! 1.0.0 (docs/plan-opencollection.md, S2, R1 à R3).

use std::path::Path;

use serde_json::Value;

use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::request::{from_value, into_value, name_of, set_name};
use crate::opencollection::schema::{self, Part};
use crate::opencollection::yaml;
use crate::request::model::Request;

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn invalid(path: &Path, reason: &str) -> CommandError {
    CommandError::new(
        CommandErrorKind::InvalidRequest,
        format!("Le fichier {} est illisible : {reason}", path.display()),
    )
}

/// Lit une requête. Un fichier absent donne `notFound`, un fichier qui n'est
/// pas une requête HTTP OpenCollection valide `invalidRequest` (R2).
pub fn read(path: &Path) -> Result<Request, CommandError> {
    let value = yaml::read(path).map_err(|error| {
        if error.kind == CommandErrorKind::NotFound {
            CommandError::new(
                CommandErrorKind::NotFound,
                format!("La requête {} n'existe plus.", path.display()),
            )
        } else {
            error
        }
    })?;
    from_value(&value, &stem(path)).map_err(|reason| invalid(path, &reason))
}

/// Nom d'une requête lisible, sans la traduire entièrement.
pub fn read_name(path: &Path) -> Option<String> {
    let value = yaml::read(path).ok()?;
    from_value(&value, &stem(path)).ok()?;
    Some(name_of(&value).unwrap_or_else(|| stem(path)))
}

/// Interprète le texte d'un fichier de requête.
pub fn parse(text: &str, path: &Path) -> Result<Request, CommandError> {
    let value = yaml::parse(text).map_err(|reason| invalid(path, &reason))?;
    from_value(&value, &stem(path)).map_err(|reason| invalid(path, &reason))
}

/// Fichier actuel, s'il est une requête lisible : base de la fusion (R3).
fn existing(path: &Path) -> Option<Value> {
    let value = yaml::read(path).ok()?;
    schema::check(Part::HttpRequest, &value).ok()?;
    Some(value)
}

fn write_value(path: &Path, value: &Value, name: &str) -> Result<(), CommandError> {
    schema::check(Part::HttpRequest, value).map_err(|reason| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("La requête « {name} » n'a pas pu être écrite : {reason}"),
        )
    })?;
    yaml::write(path, value)
}

/// Texte YAML d'une requête neuve (sans fichier existant).
pub fn render(request: &Request) -> Result<String, CommandError> {
    yaml::render(&into_value(request, None))
}

/// Écrit une requête : seuls les champs gérés par Arnod remplacent ceux du
/// fichier existant (R3) ; rien n'est réécrit si le contenu n'a pas changé.
pub fn write(path: &Path, request: &Request) -> Result<(), CommandError> {
    let value = into_value(request, existing(path).as_ref());
    write_value(path, &value, &request.name)
}

/// Copie d'une requête sous un autre nom, tout le contenu conservé (A9).
pub fn copy_as(source: &Path, target: &Path, name: &str) -> Result<(), CommandError> {
    let mut value = yaml::read(source)?;
    read(source)?;
    set_name(&mut value, name);
    write_value(target, &value, name)
}

/// Renomme une requête dans son fichier, tout le contenu conservé.
pub fn rename(path: &Path, name: &str) -> Result<(), CommandError> {
    copy_as(path, path, name)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::request::model::*;

    fn sample() -> Request {
        let mut request = Request::new("Créer une commande");
        request.method = Method::Post;
        request.url = "{{baseUrl}}/orders".to_owned();
        request
            .headers
            .push(Pair::new("Content-Type", "application/json"));
        request.auth = Auth {
            kind: AuthKind::Bearer,
            token: "{{token}}".to_owned(),
            ..Auth::default()
        };
        request.body = Body::raw(BodyKind::Json, "{\n  \"qty\": 2\n}");
        request.assertions.push(Assertion {
            id: "a1".to_owned(),
            target: "status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Int(201)),
            origin: Origin::Proposed,
            enabled: true,
        });
        request.assertions.push(Assertion {
            id: "a2".to_owned(),
            target: "body.status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Text("pending".to_owned())),
            origin: Origin::Manual,
            enabled: true,
        });
        request
    }

    /// Identifiants et origines sont de session : recalculés à la lecture (Q2).
    fn session_free(mut request: Request) -> Request {
        for (index, assertion) in request.assertions.iter_mut().enumerate() {
            assertion.id = format!("a{index}");
            assertion.origin = Origin::Manual;
        }
        request
    }

    #[test]
    fn aller_retour_stable() {
        let request = sample();
        let text = render(&request).unwrap();
        let back = parse(&text, Path::new("x.yml")).unwrap();
        assert_eq!(back, session_free(request));
        assert_eq!(render(&back).unwrap(), text);
    }

    #[test]
    fn les_valeurs_gardent_leur_type() {
        let text = render(&sample()).unwrap();
        let back = parse(&text, Path::new("x.yml")).unwrap();
        assert_eq!(back.assertions[1].value, Some(Scalar::Int(201)));
        assert_eq!(
            back.assertions[2].value,
            Some(Scalar::Text("pending".to_owned()))
        );
    }

    #[test]
    fn corps_multiligne_en_bloc_litteral() {
        let text = render(&sample()).unwrap();
        assert!(text.contains("data: |-\n"), "{text}");
    }

    #[test]
    fn fichier_illisible() {
        let error = parse("info: [", Path::new("x.yml")).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        let error = parse("info:\n  name: x\n  couleur: rouge\n", Path::new("x.yml")).unwrap_err();
        assert!(
            error.message.contains("non conforme à OpenCollection"),
            "{}",
            error.message
        );
    }

    #[test]
    fn pas_de_reecriture_sans_changement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.yml");
        let request = sample();
        write(&path, &request).unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        write(&path, &request).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    }
}
