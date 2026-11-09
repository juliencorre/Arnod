//! Lecture et écriture du fichier YAML d'une requête.

use std::fs;
use std::io;
use std::path::Path;

use crate::error::{CommandError, CommandErrorKind};
use crate::request::model::{REQUEST_VERSION, Request};

/// Commentaire placé en tête de chaque fichier de requête.
const HEADER: &str = "# Requête Arnod\n";

/// Lit une requête. Une version plus récente est refusée (`newerVersion`), un
/// fichier illisible donne `invalidRequest`.
pub fn read(path: &Path) -> Result<Request, CommandError> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CommandError::new(
                CommandErrorKind::NotFound,
                format!("La requête {} n'existe plus.", path.display()),
            )
        } else {
            CommandError::io("Lecture de la requête impossible", path, &error)
        }
    })?;
    parse(&text, path)
}

/// Interprète le texte d'un fichier de requête.
pub fn parse(text: &str, path: &Path) -> Result<Request, CommandError> {
    let request: Request = serde_saphyr::from_str(text).map_err(|error| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("Le fichier {} est illisible : {error}", path.display()),
        )
    })?;
    if request.version > REQUEST_VERSION {
        return Err(CommandError::new(
            CommandErrorKind::NewerVersion,
            format!(
                "Le fichier {} a été écrit par une version plus récente d'Arnod.",
                path.display()
            ),
        ));
    }
    Ok(request)
}

/// Texte YAML d'une requête.
pub fn render(request: &Request) -> Result<String, CommandError> {
    let body = serde_saphyr::to_string(request).map_err(|error| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "La requête « {} » n'a pas pu être écrite : {error}",
                request.name
            ),
        )
    })?;
    Ok(format!("{HEADER}{body}"))
}

/// Écrit une requête ; rien n'est réécrit si le contenu n'a pas changé (Git).
pub fn write(path: &Path, request: &Request) -> Result<(), CommandError> {
    let text = render(request)?;
    if fs::read_to_string(path).is_ok_and(|current| current == text) {
        return Ok(());
    }
    let tmp = path.with_extension("yml.tmp");
    fs::write(&tmp, &text)
        .map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        CommandError::io("Écriture impossible", path, &error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::model::*;

    fn sample() -> Request {
        let mut request = Request::new("Créer une commande");
        request.method = Method::Post;
        request.url = "{{baseUrl}}/orders".to_owned();
        request.headers.push(Pair {
            name: "Content-Type".to_owned(),
            value: "application/json".to_owned(),
            enabled: true,
        });
        request.auth = Auth {
            kind: AuthKind::Bearer,
            token: "{{token}}".to_owned(),
            ..Auth::default()
        };
        request.body = Body {
            kind: BodyKind::Json,
            content: "{ \"qty\": 2 }\n".to_owned(),
            form: Vec::new(),
        };
        request.assertions.push(Assertion {
            id: "a1".to_owned(),
            target: "status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Int(201)),
            origin: Origin::Proposed,
        });
        request.assertions.push(Assertion {
            id: "a2".to_owned(),
            target: "body.status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Text("pending".to_owned())),
            origin: Origin::Manual,
        });
        request
    }

    #[test]
    fn aller_retour_stable() {
        let request = sample();
        let text = render(&request).unwrap();
        let back = parse(&text, Path::new("x.yml")).unwrap();
        assert_eq!(back, request);
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
    fn version_plus_recente_refusee() {
        let text = render(&sample())
            .unwrap()
            .replace("version: 1", "version: 9");
        let error = parse(&text, Path::new("x.yml")).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NewerVersion);
    }

    #[test]
    fn fichier_illisible() {
        let error = parse("name: [", Path::new("x.yml")).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
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
