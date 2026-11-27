//! Règles des tests en schéma (`docs/plan-v4.md`, R1 à R8).

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::Mutex;

use super::engine::{self, BoxFuture, Call, CallDone, Report, RequestDone, Runner, StepState};
use super::reconcile::{normalize, reconcile, validate};
use super::store::{self, Extensions};
use super::*;
use crate::opencollection::ROOT_FILE;
use crate::request::assertions::ResponseData;
use crate::request::model::{Operator, Scalar};
use crate::script::ScriptOutcome;

fn files(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn step(id: &str, action: Action) -> Step {
    Step {
        id: id.to_owned(),
        name: String::new(),
        action,
        next: None,
        on_failure: None,
    }
}

fn request(id: &str, file: &str) -> Step {
    step(
        id,
        Action::Request {
            file: file.to_owned(),
        },
    )
}

fn condition(id: &str, target: &str, value: i64, yes: &str, no: &str) -> Step {
    let mut s = step(
        id,
        Action::Condition {
            check: Check {
                target: target.to_owned(),
                operator: Operator::Equals,
                value: Some(Scalar::Int(value)),
            },
            otherwise: Some(no.to_owned()),
        },
    );
    s.next = Some(yes.to_owned());
    s
}

fn ids(flow: &Flow) -> Vec<&str> {
    flow.steps.iter().map(|s| s.id.as_str()).collect()
}

// ---------------------------------------------------------------- réconciliation (R2, R3)

#[test]
fn r2_sans_schema_ecrit_la_liste_des_requetes() {
    let flow = reconcile(None, &files(&["a.yml", "b.yml"]), &HashSet::new());
    assert_eq!(ids(&flow), ["r:a.yml", "r:b.yml"]);
    assert!(flow.is_linear(&files(&["a.yml", "b.yml"])));
}

#[test]
fn r2_une_requete_disparue_est_retiree_et_ses_renvois_passent_a_sa_suite() {
    let mut first = request("s1", "a.yml");
    first.next = Some("s2".to_owned());
    let mut gone = request("s2", "disparue.yml");
    gone.next = Some("s4".to_owned());
    let stored = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![first, gone, request("s3", "c.yml"), request("s4", "d.yml")],
    };
    let flow = reconcile(
        Some(&stored),
        &files(&["a.yml", "c.yml", "d.yml"]),
        &HashSet::new(),
    );
    assert_eq!(ids(&flow), ["s1", "s3", "s4"]);
    // La suite de la requête disparue (s4) remplace le renvoi vers elle.
    assert_eq!(flow.steps[0].next.as_deref(), Some("s4"));
}

#[test]
fn r2_une_requete_nouvelle_s_ajoute_a_la_fin_et_un_renvoi_inconnu_vaut_la_suite() {
    let mut first = request("s1", "a.yml");
    first.next = Some("inconnue".to_owned());
    let stored = Flow {
        on_failure: OnFailure::Continue,
        steps: vec![first],
    };
    let flow = reconcile(
        Some(&stored),
        &files(&["a.yml", "nouvelle.yml"]),
        &HashSet::new(),
    );
    assert_eq!(ids(&flow), ["s1", "r:nouvelle.yml"]);
    assert_eq!(flow.steps[0].next, None);
    assert_eq!(flow.on_failure, OnFailure::Continue);
}

#[test]
fn r14_les_requetes_d_une_attente_ou_d_un_avant_ne_sont_pas_des_etapes() {
    let stored = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            request("s1", "a.yml"),
            step(
                "w",
                Action::Wait {
                    file: Some("poll.yml".to_owned()),
                    check: Check::new(),
                    interval_ms: 10,
                    attempts: 3,
                },
            ),
        ],
    };
    let owned: HashSet<String> = ["avant.yml".to_owned()].into();
    let flow = reconcile(
        Some(&stored),
        &files(&["a.yml", "avant.yml", "poll.yml"]),
        &owned,
    );
    assert_eq!(ids(&flow), ["s1", "w"]);
    let none = reconcile(None, &files(&["a.yml", "avant.yml"]), &owned);
    assert_eq!(ids(&none), ["r:a.yml"]);
}

#[test]
fn r1_un_renvoi_vers_l_etape_suivante_ne_s_ecrit_pas() {
    let mut a = request("s1", "a.yml");
    a.next = Some("s2".to_owned());
    let mut b = request("s2", "b.yml");
    b.next = Some(END.to_owned());
    let mut flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![a, b],
    };
    normalize(&mut flow);
    assert!(flow.is_linear(&files(&["a.yml", "b.yml"])));
}

#[test]
fn validation_refuse_un_renvoi_inconnu_et_une_requete_jouee_deux_fois() {
    let mut a = request("s1", "a.yml");
    a.next = Some("nulle-part".to_owned());
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![a],
    };
    assert!(validate(&flow, &files(&["a.yml"])).is_err());
    let twice = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![request("s1", "a.yml"), request("s2", "a.yml")],
    };
    assert!(validate(&twice, &files(&["a.yml"])).is_err());
    let ok = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            request("s1", "a.yml"),
            condition("c", "status", 201, END, FAIL),
        ],
    };
    assert_eq!(validate(&ok, &files(&["a.yml"])), Ok(()));
}

// ---------------------------------------------------------------- stockage (R1, R4)

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Projet\nextensions:\n  arnod:\n    version: 1\n    id: 6c1f4e0a-2b3c-4d5e-8f90-0a1b2c3d4e5f\n",
    )
    .unwrap();
    dir
}

fn branching() -> Flow {
    Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            request("s1", "a.yml"),
            condition("c", "status", 201, "s2", "s3"),
            {
                let mut s = request("s2", "b.yml");
                s.next = Some(END.to_owned());
                s
            },
            {
                let mut s = request("s3", "c.yml");
                s.next = Some(FAIL.to_owned());
                s
            },
        ],
    }
}

#[test]
fn r1_le_schema_s_ecrit_dans_extensions_arnod_et_garde_l_identifiant() {
    let dir = project();
    store::save_flow(dir.path(), "commandes/parcours", Some(&branching())).unwrap();
    let text = fs::read_to_string(dir.path().join(ROOT_FILE)).unwrap();
    assert!(text.contains("tests:"), "{text}");
    assert!(text.contains("6c1f4e0a-2b3c-4d5e-8f90-0a1b2c3d4e5f"));
    let ext = Extensions::read(dir.path());
    assert_eq!(ext.flow("commandes/parcours"), Some(branching()));
    // L'identifiant et les environnements gérés ailleurs ne l'effacent pas.
    crate::opencollection::collection::update_meta(dir.path(), |meta| {
        meta.protected = vec!["Recette".to_owned()];
    })
    .unwrap();
    assert_eq!(
        Extensions::read(dir.path()).flow("commandes/parcours"),
        Some(branching())
    );
    store::save_flow(dir.path(), "commandes/parcours", None).unwrap();
    assert!(
        Extensions::read(dir.path())
            .flow("commandes/parcours")
            .is_none()
    );
}

#[test]
fn r4_renommer_deplacer_copier_supprimer_font_suivre_le_schema() {
    let dir = project();
    let mut flow = branching();
    flow.steps.push(step(
        "t",
        Action::Test {
            test: "commandes/parcours".to_owned(),
        },
    ));
    store::save_flow(dir.path(), "commandes/parcours", Some(&flow)).unwrap();
    store::save_flow(dir.path(), "autre", Some(&flow)).unwrap();
    // Le dossier parent est renommé : le test et le renvoi vers lui suivent.
    store::follow(dir.path(), "commandes", "orders").unwrap();
    let ext = Extensions::read(dir.path());
    assert!(ext.flow("commandes/parcours").is_none());
    let moved = ext.flow("orders/parcours").unwrap();
    assert!(matches!(&moved.steps[4].action, Action::Test { test } if test == "orders/parcours"));
    // Une requête renommée dans son dossier : l'étape qui la joue suit.
    store::follow(
        dir.path(),
        "orders/parcours/b.yml",
        "orders/parcours/bb.yml",
    )
    .unwrap();
    let renamed = Extensions::read(dir.path())
        .flow("orders/parcours")
        .unwrap();
    assert!(matches!(&renamed.steps[2].action, Action::Request { file } if file == "bb.yml"));
    store::copy(dir.path(), "orders/parcours", "orders/parcours (copie)").unwrap();
    assert!(
        Extensions::read(dir.path())
            .flow("orders/parcours (copie)")
            .is_some()
    );
    store::forget(dir.path(), "orders").unwrap();
    let ext = Extensions::read(dir.path());
    assert!(ext.flow("orders/parcours").is_none());
    assert!(ext.flow("autre").is_some());
}

#[test]
fn r13_avant_apres_et_fichiers_propres() {
    let dir = project();
    let hooks = FolderHooks {
        before_all: vec![request("h1", "avant.yml")],
        after_all: vec![request("h2", "apres.yml")],
        ..FolderHooks::default()
    };
    store::save_hooks(dir.path(), "commandes/v2", &hooks).unwrap();
    let ext = Extensions::read(dir.path());
    assert_eq!(ext.hooks("commandes/v2"), hooks);
    assert_eq!(ext.hooked_folders(), ["commandes/v2"]);
    let owned = ext.owned_files();
    assert!(owned.contains("commandes/v2/avant.yml"));
    assert!(owned.contains("commandes/v2/apres.yml"));
    store::save_hooks(dir.path(), "commandes/v2", &FolderHooks::default()).unwrap();
    assert!(Extensions::read(dir.path()).hooked_folders().is_empty());
}

// ---------------------------------------------------------------- conditions (R6)

fn data(status: u16, body: &str) -> ResponseData {
    ResponseData {
        status,
        duration_ms: 10,
        headers: Vec::new(),
        body: body.to_owned(),
    }
}

#[test]
fn r6_condition_sur_la_reponse_ou_une_variable() {
    let vars: BTreeMap<String, String> = [("creditOk".to_owned(), "true".to_owned())].into();
    let status = Check {
        target: "status".to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Int(201)),
    };
    assert!(check::evaluate(&status, Some(&data(201, "{}")), &vars).holds);
    assert!(!check::evaluate(&status, Some(&data(400, "{}")), &vars).holds);
    let variable = Check {
        target: "vars.creditOk".to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Bool(true)),
    };
    assert!(check::evaluate(&variable, None, &vars).holds);
    let body = Check {
        target: "body.status".to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Text("paid".to_owned())),
    };
    assert!(check::evaluate(&body, Some(&data(200, r#"{"status":"paid"}"#)), &vars).holds);
    // Sans réponse, une condition sur la réponse est fausse, sans erreur.
    assert!(!check::evaluate(&status, None, &vars).holds);
}

#[test]
fn liste_d_une_boucle() {
    let vars: BTreeMap<String, String> = [("ids".to_owned(), r#"["a","b"]"#.to_owned())].into();
    assert_eq!(
        check::list_items("{{ids}}", None, &vars).unwrap(),
        ["a", "b"]
    );
    assert_eq!(
        check::list_items(
            "body.items",
            Some(&data(200, r#"{"items":[1,{"x":2}]}"#)),
            &vars
        )
        .unwrap(),
        ["1", r#"{"x":2}"#]
    );
    assert!(check::list_items("vars.absente", None, &vars).is_err());
}

// ---------------------------------------------------------------- parcours (R5, R7, R8)

/// Faux établi : chaque fichier rend un statut écrit d'avance ; tout est noté.
#[derive(Default)]
struct Fake {
    statuses: BTreeMap<String, Vec<u16>>,
    played: Mutex<Vec<String>>,
    reports: Mutex<Vec<(String, Report)>>,
}

impl Fake {
    fn with(statuses: &[(&str, &[u16])]) -> Self {
        Self {
            statuses: statuses
                .iter()
                .map(|(f, s)| ((*f).to_owned(), s.to_vec()))
                .collect(),
            ..Self::default()
        }
    }

    fn played(&self) -> Vec<String> {
        self.played.lock().unwrap().clone()
    }

    fn last_report(&self, id: &str) -> Option<Report> {
        self.reports
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(s, _)| s == id)
            .map(|(_, r)| r.clone())
    }
}

impl Runner for Fake {
    fn request<'a>(
        &'a self,
        step: &'a str,
        file: &'a str,
        vars: &'a BTreeMap<String, String>,
        attempt: u32,
    ) -> BoxFuture<'a, RequestDone> {
        Box::pin(async move {
            self.played.lock().unwrap().push(format!(
                "{step}:{file}:{}",
                vars.get("item").map_or("", |v| v)
            ));
            let list = self
                .statuses
                .get(file)
                .cloned()
                .unwrap_or_else(|| vec![200]);
            let index = usize::try_from(attempt).unwrap_or(1).saturating_sub(1);
            let status = *list.get(index).or(list.last()).unwrap_or(&200);
            let mut extracted = BTreeMap::new();
            extracted.insert(format!("{file}.status"), status.to_string());
            RequestDone {
                passed: status < 400,
                response: Some(data(status, &format!(r#"{{"status":{status}}}"#))),
                extracted,
            }
        })
    }

    fn call<'a>(
        &'a self,
        _step: &'a str,
        call: Call<'a>,
        vars: &'a mut BTreeMap<String, String>,
        _depth: usize,
    ) -> BoxFuture<'a, CallDone> {
        Box::pin(async move {
            let name = match call {
                Call::Test(test) => test.to_owned(),
                Call::Template { template, .. } => template.to_owned(),
            };
            vars.insert("appel".to_owned(), name.clone());
            CallDone {
                passed: !name.contains("ko"),
                passed_steps: 9,
                total_steps: 12,
                message: None,
            }
        })
    }

    fn script<'a>(
        &'a self,
        code: &'a str,
        vars: &'a BTreeMap<String, String>,
        response: Option<&'a ResponseData>,
    ) -> BoxFuture<'a, ScriptOutcome> {
        Box::pin(async move { crate::script::run_step(code, vars, response) })
    }

    fn substitute(&self, text: &str, vars: &BTreeMap<String, String>) -> Result<String, String> {
        let mut out = text.to_owned();
        for (name, value) in vars {
            out = out.replace(&format!("{{{{{name}}}}}"), value);
        }
        if out.contains("{{") {
            Err("variable inconnue".to_owned())
        } else {
            Ok(out)
        }
    }

    fn report(&self, step: &str, report: Report) {
        self.reports.lock().unwrap().push((step.to_owned(), report));
    }

    fn cancelled(&self) -> bool {
        false
    }

    fn sleep(&self, _ms: u64) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn r5_la_condition_choisit_sa_branche_et_fin_en_echec() {
    let fake = Fake::with(&[("a.yml", &[201])]);
    let mut vars = BTreeMap::new();
    let done = block(engine::run(&branching(), &fake, &mut vars, 0));
    assert_eq!(fake.played(), ["s1:a.yml:", "s2:b.yml:"]);
    assert!(done.passed);
    assert_eq!(fake.last_report("c").unwrap().branch, Some(true));
    // Sinon : la requête de refus, puis « Fin en échec ».
    let fake = Fake::with(&[("a.yml", &[400])]);
    let mut flow = branching();
    flow.on_failure = OnFailure::Continue;
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert_eq!(fake.played(), ["s1:a.yml:", "s3:c.yml:"]);
    assert!(!done.passed);
    assert_eq!(done.reason.as_deref(), Some("Fin en échec"));
}

#[test]
fn r5_une_etape_en_echec_arrete_le_test_sauf_continuer() {
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![request("s1", "ko.yml"), request("s2", "b.yml")],
    };
    let fake = Fake::with(&[("ko.yml", &[500])]);
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert_eq!(fake.played().len(), 1);
    assert_eq!(done.failed_step.as_deref(), Some("s1"));
    let mut flow = flow;
    flow.steps[0].on_failure = Some(OnFailure::Continue);
    let fake = Fake::with(&[("ko.yml", &[500])]);
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert_eq!(fake.played().len(), 2);
    assert!(!done.passed, "le test reste en échec");
}

#[test]
fn boucle_pour_chaque_element_et_n_fois() {
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            step(
                "l",
                Action::Loop {
                    count: None,
                    list: Some("{{ids}}".to_owned()),
                    item: "item".to_owned(),
                    until: Some("s2".to_owned()),
                },
            ),
            request("s1", "a.yml"),
            request("s2", "b.yml"),
            request("s3", "c.yml"),
        ],
    };
    let fake = Fake::default();
    let mut vars: BTreeMap<String, String> = [("ids".to_owned(), r#"["x","y"]"#.to_owned())].into();
    let done = block(engine::run(&flow, &fake, &mut vars, 0));
    assert!(done.passed);
    assert_eq!(
        fake.played(),
        [
            "s1:a.yml:x",
            "s2:b.yml:x",
            "s1:a.yml:y",
            "s2:b.yml:y",
            "s3:c.yml:y"
        ]
    );
    assert_eq!(
        fake.last_report("l").unwrap().detail.as_deref(),
        Some("2 tours")
    );
    // Trois fois, sans liste.
    let mut counted = flow.clone();
    counted.steps[0].action = Action::Loop {
        count: Some(3),
        list: None,
        item: "tour".to_owned(),
        until: None,
    };
    let fake = Fake::default();
    block(engine::run(&counted, &fake, &mut BTreeMap::new(), 0));
    let played = fake.played();
    assert_eq!(played.iter().filter(|p| p.starts_with("s1:")).count(), 3);
    assert_eq!(played.iter().filter(|p| p.starts_with("s2:")).count(), 1);
}

#[test]
fn attente_jusqu_a_ce_que_la_condition_soit_vraie() {
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![step(
            "w",
            Action::Wait {
                file: Some("poll.yml".to_owned()),
                check: Check {
                    target: "status".to_owned(),
                    operator: Operator::Equals,
                    value: Some(Scalar::Int(200)),
                },
                interval_ms: 1,
                attempts: 5,
            },
        )],
    };
    let fake = Fake::with(&[("poll.yml", &[202, 202, 200])]);
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert!(done.passed);
    assert_eq!(fake.played().len(), 3);
    assert_eq!(
        fake.last_report("w").unwrap().detail.as_deref(),
        Some("3 essais")
    );
    let fake = Fake::with(&[("poll.yml", &[202])]);
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert!(!done.passed);
    assert_eq!(fake.played().len(), 5);
}

#[test]
fn variable_script_test_et_template() {
    let flow = Flow {
        on_failure: OnFailure::Continue,
        steps: vec![
            step(
                "v",
                Action::Variable {
                    variable: "montant".to_owned(),
                    value: "{{prix}}0".to_owned(),
                },
            ),
            step(
                "js",
                Action::Script {
                    code: "assert(vars.montant === '59.80'); vars.set('ttc', 71.76);".to_owned(),
                },
            ),
            step(
                "t",
                Action::Test {
                    test: "clients/creer".to_owned(),
                },
            ),
            step(
                "tpl",
                Action::Template {
                    template: "templates/payer-ko".to_owned(),
                    values: BTreeMap::new(),
                },
            ),
        ],
    };
    let fake = Fake::default();
    let mut vars: BTreeMap<String, String> = [("prix".to_owned(), "59.8".to_owned())].into();
    let done = block(engine::run(&flow, &fake, &mut vars, 0));
    assert_eq!(vars.get("montant").map(String::as_str), Some("59.80"));
    assert_eq!(vars.get("ttc").map(String::as_str), Some("71.76"));
    let script = fake.last_report("js").unwrap();
    assert_eq!(script.state, StepState::Passed, "{:?}", script.error);
    assert_eq!(fake.last_report("t").unwrap().state, StepState::Passed);
    let template = fake.last_report("tpl").unwrap();
    assert_eq!(template.state, StepState::Failed);
    assert_eq!(template.detail.as_deref(), Some("9/12"));
    assert!(!done.passed);
    assert_eq!(done.failed_step.as_deref(), Some("tpl"));
}

#[test]
fn r7_renvois_sans_fin_et_profondeur() {
    let mut a = request("s1", "a.yml");
    a.next = Some("s1".to_owned());
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![a],
    };
    let fake = Fake::default();
    let done = block(engine::run(&flow, &fake, &mut BTreeMap::new(), 0));
    assert!(!done.passed);
    assert!(done.reason.unwrap().contains("étapes jouées"));
    let done = block(engine::run(
        &flow,
        &fake,
        &mut BTreeMap::new(),
        MAX_DEPTH + 1,
    ));
    assert!(!done.passed);
}

// ---------------------------------------------------------------- édition (R3)

#[test]
fn r3_inserer_sur_un_lien_reconnecte_le_lien() {
    use super::edit::{Placement, insert, new_step, remove};
    let mut flow = branching();
    // Sur « sinon » de la condition : la condition renvoie vers la nouvelle, qui renvoie au refus.
    let step = new_step(&flow, StepKind::Script, None);
    let id = step.id.clone();
    insert(
        &mut flow,
        step,
        &Placement::Link {
            after: "c".into(),
            otherwise: true,
        },
    );
    let at = flow.position("c").unwrap();
    assert_eq!(flow.steps[at + 1].id, id);
    assert_eq!(
        flow.steps[at].next.as_deref(),
        Some("s2"),
        "« si oui » garde sa suite"
    );
    assert!(
        matches!(&flow.steps[at].action, Action::Condition { otherwise: Some(o), .. } if *o == id)
    );
    assert_eq!(flow.steps[at + 1].next.as_deref(), Some("s3"));
    // Sur « puis » de la première requête : elle passe par la nouvelle étape.
    let step = new_step(&flow, StepKind::Variable, None);
    let second = step.id.clone();
    insert(
        &mut flow,
        step,
        &Placement::Link {
            after: "s1".into(),
            otherwise: false,
        },
    );
    assert_eq!(flow.steps[1].id, second);
    assert_eq!(flow.steps[0].next, None);
    assert_eq!(flow.steps[1].next, None, "la suite vaut l'étape suivante");
    // Retirer l'étape : les renvois vers elle passent à sa suite.
    remove(&mut flow, &id);
    let at = flow.position("c").unwrap();
    assert!(
        matches!(&flow.steps[at].action, Action::Condition { otherwise: Some(o), .. } if o == "s3")
    );
}

#[test]
fn r1_r3_ajouter_une_etape_cree_sa_requete_et_garde_l_ordre() {
    use super::edit::{Placement, add};
    let dir = project();
    let folder = crate::tree::create_folder(dir.path(), "", "parcours").unwrap();
    crate::tree::create_request(dir.path(), &folder, Some("Créer un client")).unwrap();
    crate::tree::create_request(dir.path(), &folder, Some("Créer une commande")).unwrap();
    // Linéaire : rien n'est écrit (R1).
    let added = add(
        dir.path(),
        &folder,
        StepKind::Request,
        &Placement::End,
        Some("Payer"),
    )
    .unwrap();
    assert!(added.request.is_some());
    assert!(Extensions::read(dir.path()).flow(&folder).is_none());
    // Une condition rend le schéma non linéaire : il est écrit, les requêtes rangées.
    let condition = add(
        dir.path(),
        &folder,
        StepKind::Condition,
        &Placement::Link {
            after: "r:creer-un-client.yml".into(),
            otherwise: false,
        },
        None,
    )
    .unwrap();
    let flow = Extensions::read(dir.path()).flow(&folder).unwrap();
    assert_eq!(flow.steps[1].id, condition.step);
    let view = crate::tree::test_view(dir.path(), &folder).unwrap();
    assert_eq!(view.flow.steps.len(), 4);
    assert_eq!(view.info.len(), 4);
    // Une attente reçoit sa requête, absente de l'arborescence (R14).
    let wait = add(dir.path(), &folder, StepKind::Wait, &Placement::End, None).unwrap();
    let request = wait.request.unwrap();
    let nodes = crate::tree::read(dir.path()).unwrap();
    assert!(crate::template::find(&nodes, &request).is_none());
    let view = crate::tree::test_view(dir.path(), &folder).unwrap();
    assert!(view.steps.iter().any(|s| s.path == request));
}
