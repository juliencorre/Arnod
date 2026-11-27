//! Arguments de `arnod` (LC1 à LC5), lus à la main : messages et aide en français.

use std::path::PathBuf;

use crate::suggest::closest;

/// Rapport demandé par `--rapport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Junit,
    Allure,
    Html,
}

impl ReportKind {
    const ALL: [(&'static str, Self); 3] = [
        ("junit", Self::Junit),
        ("allure", Self::Allure),
        ("html", Self::Html),
    ];
}

/// Options de `arnod run`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunArgs {
    /// Dossier, test ou requête, relatif au projet ; absent : tout le projet.
    pub target: Option<String>,
    pub env: Option<String>,
    /// Étiquettes cumulées (ou).
    pub tags: Vec<String>,
    pub parallel: Option<usize>,
    pub reports: Vec<ReportKind>,
    /// Dossier des rapports (`rapports` par défaut).
    pub reports_dir: Option<PathBuf>,
    /// Dossier du projet (par défaut : le dossier courant ou un parent).
    pub project: Option<PathBuf>,
    /// Environnement protégé : envoyer quand même les méthodes qui modifient.
    pub confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    RunHelp,
    Version,
    Run(RunArgs),
}

pub const HELP: &str = "\
arnod - joue les tests d'un projet Arnod hors de l'application.

Utilisation :
  arnod run [CIBLE] [options]   Lance un dossier, un test ou une requête du projet
  arnod --help                  Affiche cette aide
  arnod --version               Affiche la version

Lancez la commande depuis le dossier du projet (celui qui contient opencollection.yml),
ou donnez-le par --projet. Voir « arnod run --help ».
";

pub const RUN_HELP: &str = "\
arnod run - lance un dossier, un test, une requête ou une étiquette du projet.

Utilisation :
  arnod run [CIBLE] [options]

CIBLE
  Chemin d'un dossier, d'un test ou d'une requête, relatif au projet
  (« commandes/v2 », « commandes/v2/lister.yml »). Sans cible : tout le projet.

Options :
  -e, --env NOM            Environnement (par défaut : le premier du projet)
  -t, --etiquette NOM      Ne joue que les tests qui portent l'étiquette ; plusieurs
                           --etiquette se cumulent (l'une ou l'autre)
  -p, --parallele N        Tests joués en même temps
  -r, --rapport LISTE      Rapports à écrire : junit, allure, html (séparés par des virgules)
      --rapports DOSSIER   Dossier des rapports (par défaut : rapports)
      --projet DOSSIER     Dossier du projet (par défaut : le dossier courant)
      --confirmer          Envoie aussi les méthodes qui modifient sur un environnement protégé
  -h, --help               Affiche cette aide

Variables locales et secrets : ARNOD_VAR_<nom> fournit la variable <nom>
(par exemple ARNOD_VAR_token). Sinon, la valeur du poste (coffre, valeurs locales) est prise ;
une variable sans valeur arrête la commande avant tout envoi.

Rien n'est écrit hors du dossier des rapports.

Codes de sortie : 0 tout a réussi, 1 au moins un test en échec ou en erreur,
2 commande invalide (projet, cible, environnement introuvable, variable manquante).
";

const RUN_OPTIONS: [&str; 10] = [
    "--env",
    "--etiquette",
    "--parallele",
    "--rapport",
    "--rapports",
    "--projet",
    "--confirmer",
    "--help",
    "-h",
    "-e",
];

fn unknown_option(option: &str) -> String {
    let mut message = format!("Option inconnue : {option}.");
    if let Some(near) = closest(option, RUN_OPTIONS.iter().copied().filter(|o| o.len() > 2)) {
        message.push_str(&format!(" Vouliez-vous dire {near} ?"));
    }
    message.push_str(" Voir « arnod run --help ».");
    message
}

/// Lit la liste de `--rapport` : `junit,allure,html`.
fn reports(list: &str, into: &mut Vec<ReportKind>) -> Result<(), String> {
    for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let kind = ReportKind::ALL
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(_, kind)| *kind)
            .ok_or_else(|| {
                format!("Rapport inconnu : « {name} ». Rapports possibles : junit, allure, html.")
            })?;
        if !into.contains(&kind) {
            into.push(kind);
        }
    }
    Ok(())
}

/// Lit les arguments (sans le nom du programme).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(first) = args.first() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "-h" | "--help" | "aide" => return Ok(Command::Help),
        "-V" | "--version" | "version" => return Ok(Command::Version),
        "run" => {}
        other => {
            let mut message = format!("Commande inconnue : « {other} ».");
            if closest(other, ["run"].into_iter()).is_some() {
                message.push_str(" Vouliez-vous dire « arnod run » ?");
            }
            message.push_str(" Voir « arnod --help ».");
            return Err(message);
        }
    }
    let mut run = RunArgs::default();
    let mut rest = args.iter().skip(1).peekable();
    let mut options_done = false;
    while let Some(arg) = rest.next() {
        if options_done || !arg.starts_with('-') || arg == "-" {
            if run.target.is_some() {
                return Err(format!(
                    "Une seule cible à la fois : « {} » puis « {arg} ». Mettez entre guillemets un \
                     chemin qui contient des espaces.",
                    run.target.as_deref().unwrap_or_default()
                ));
            }
            run.target = Some(arg.clone());
            continue;
        }
        if arg == "--" {
            options_done = true;
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let mut value = |label: &str| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| rest.next().cloned())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("{name} attend {label}."))
        };
        match name {
            "-h" | "--help" => return Ok(Command::RunHelp),
            "-e" | "--env" => run.env = Some(value("le nom d'un environnement")?),
            "-t" | "--etiquette" => {
                let tags = value("une étiquette")?;
                for tag in tags.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                    let tag = tag.trim_start_matches('#').to_owned();
                    if !run.tags.contains(&tag) {
                        run.tags.push(tag);
                    }
                }
            }
            "-p" | "--parallele" => {
                let text = value("un nombre")?;
                let count = text
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (1..=64).contains(n))
                    .ok_or_else(|| {
                        format!("--parallele attend un nombre de 1 à 64, pas « {text} ».")
                    })?;
                run.parallel = Some(count);
            }
            "-r" | "--rapport" => reports(&value("une liste de rapports")?, &mut run.reports)?,
            "--rapports" => run.reports_dir = Some(PathBuf::from(value("un dossier")?)),
            "--projet" => run.project = Some(PathBuf::from(value("un dossier")?)),
            "--confirmer" => run.confirm = true,
            _ => return Err(unknown_option(name)),
        }
    }
    Ok(Command::Run(run))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn lc1_lc2_lc3_options_de_run() {
        let parsed = parse(&args(&[
            "run",
            "commandes/v2",
            "--env",
            "Recette fonctionnelle",
            "--rapport",
            "junit,allure",
            "-r",
            "HTML,junit",
            "--etiquette=smoke",
            "-t",
            "#paiement,smoke",
            "-p",
            "4",
            "--rapports",
            "sortie",
            "--confirmer",
        ]))
        .unwrap();
        assert_eq!(
            parsed,
            Command::Run(RunArgs {
                target: Some("commandes/v2".into()),
                env: Some("Recette fonctionnelle".into()),
                tags: vec!["smoke".into(), "paiement".into()],
                parallel: Some(4),
                reports: vec![ReportKind::Junit, ReportKind::Allure, ReportKind::Html],
                reports_dir: Some("sortie".into()),
                project: None,
                confirm: true,
            })
        );
        assert_eq!(parse(&args(&["run"])), Ok(Command::Run(RunArgs::default())));
        assert_eq!(
            parse(&args(&["run", "--", "-dossier"])),
            Ok(Command::Run(RunArgs {
                target: Some("-dossier".into()),
                ..RunArgs::default()
            }))
        );
    }

    #[test]
    fn lc5_aide_version_et_erreurs_en_francais() {
        assert_eq!(parse(&[]), Ok(Command::Help));
        assert_eq!(parse(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(&args(&["-V"])), Ok(Command::Version));
        assert_eq!(parse(&args(&["run", "x", "--help"])), Ok(Command::RunHelp));
        assert_eq!(
            parse(&args(&["rnu"])),
            Err("Commande inconnue : « rnu ». Vouliez-vous dire « arnod run » ? Voir « arnod --help ».".into())
        );
        assert_eq!(
            parse(&args(&["run", "--etiquete", "smoke"])),
            Err("Option inconnue : --etiquete. Vouliez-vous dire --etiquette ? Voir « arnod run --help ».".into())
        );
        assert_eq!(
            parse(&args(&["run", "--env"])),
            Err("--env attend le nom d'un environnement.".into())
        );
        assert_eq!(
            parse(&args(&["run", "-r", "pdf"])),
            Err("Rapport inconnu : « pdf ». Rapports possibles : junit, allure, html.".into())
        );
        assert!(parse(&args(&["run", "-p", "0"])).is_err());
        assert!(
            parse(&args(&["run", "a", "b"]))
                .unwrap_err()
                .starts_with("Une seule cible")
        );
        assert!(HELP.contains("arnod run") && RUN_HELP.contains("ARNOD_VAR_"));
    }
}
