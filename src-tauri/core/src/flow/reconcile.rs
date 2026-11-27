//! Réconciliation du schéma relu avec le dossier du test (`docs/plan-v4.md`, R2, R3) et
//! validation avant écriture. Module pur : il reçoit les fichiers du dossier.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{Action, END, FAIL, Flow, MAX_ATTEMPTS, MAX_INTERVAL_MS, MAX_LOOP, Step, request_id};

/// Schéma du test d'après le schéma écrit (`stored`) et les fichiers de requête du dossier
/// (`files`, dans l'ordre du dossier), sans ceux d'un Avant / Après (`owned`, R14) :
///
/// - sans schéma écrit, la liste des requêtes ;
/// - une étape requête dont le fichier a disparu est retirée, ses renvois passent à sa suite ;
/// - une requête du dossier absente du schéma s'ajoute à la fin ;
/// - un renvoi vers une étape inconnue vaut « étape suivante » (R2).
pub fn reconcile(stored: Option<&Flow>, files: &[String], owned: &HashSet<String>) -> Flow {
    let available: Vec<&String> = files.iter().filter(|f| !owned.contains(*f)).collect();
    let Some(stored) = stored else {
        return Flow {
            on_failure: Default::default(),
            steps: available.into_iter().map(|f| Step::request(f)).collect(),
        };
    };
    let present: HashSet<&str> = available.iter().map(|f| f.as_str()).collect();
    // Fichiers des attentes : propres à leur étape, jamais une étape requête (R14).
    let waited: HashSet<&str> = stored
        .steps
        .iter()
        .filter_map(|s| match &s.action {
            Action::Wait {
                file: Some(file), ..
            } if present.contains(file.as_str()) => Some(file.as_str()),
            _ => None,
        })
        .collect();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut seen_files: HashSet<String> = HashSet::new();
    let keep: Vec<bool> = stored
        .steps
        .iter()
        .map(|step| {
            let file_ok = match &step.action {
                Action::Request { file } => {
                    present.contains(file.as_str())
                        && !waited.contains(file.as_str())
                        && seen_files.insert(file.clone())
                }
                _ => true,
            };
            file_ok && !step.id.trim().is_empty() && seen_ids.insert(step.id.clone())
        })
        .collect();
    let kept_ids: HashSet<&str> = stored
        .steps
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(s, _)| s.id.as_str())
        .collect();
    // Suite d'une étape retirée : son renvoi (résolu à son tour), sinon la première étape gardée
    // qui la suit, sinon la fin.
    let index_of: HashMap<&str, usize> = stored
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let successor = |start: usize| -> String {
        let mut index = start;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > stored.steps.len() + 1 {
                return END.to_owned();
            }
            let step = &stored.steps[index];
            match step.next.as_deref() {
                Some(target @ (END | FAIL)) => return target.to_owned(),
                Some(target) if kept_ids.contains(target) => return target.to_owned(),
                Some(target) => match index_of.get(target) {
                    Some(&other) if other != index => {
                        index = other;
                        continue;
                    }
                    _ => {}
                },
                None => {}
            }
            return stored
                .steps
                .iter()
                .zip(&keep)
                .skip(index + 1)
                .find(|(_, k)| **k)
                .map_or_else(|| END.to_owned(), |(s, _)| s.id.clone());
        }
    };
    let map_target = |target: &str| -> Option<String> {
        if target == END || target == FAIL || kept_ids.contains(target) {
            return Some(target.to_owned());
        }
        index_of.get(target).map(|&index| successor(index))
    };
    let mut steps: Vec<Step> = Vec::new();
    for (step, _) in stored.steps.iter().zip(&keep).filter(|(_, k)| **k) {
        let mut step = step.clone();
        step.next = step.next.as_deref().and_then(map_target);
        match &mut step.action {
            Action::Condition { otherwise, .. } => {
                *otherwise = otherwise.as_deref().and_then(map_target);
            }
            Action::Loop { until, .. } => {
                *until = until
                    .as_deref()
                    .and_then(map_target)
                    .filter(|t| t != END && t != FAIL);
            }
            Action::Wait { file, .. } if file.as_deref().is_some_and(|f| !present.contains(f)) => {
                *file = None;
            }
            _ => {}
        }
        steps.push(step);
    }
    for file in available {
        if seen_files.contains(file.as_str()) || waited.contains(file.as_str()) {
            continue;
        }
        let mut step = Step::request(file);
        let mut n = 2;
        while steps.iter().any(|s| s.id == step.id) {
            step.id = format!("{}#{n}", request_id(file));
            n += 1;
        }
        steps.push(step);
    }
    let mut flow = Flow {
        on_failure: stored.on_failure,
        steps,
    };
    normalize(&mut flow);
    flow
}

/// Retire les renvois qui valent la suite par défaut : vers l'étape suivante, ou « Fin ✓ » de la
/// dernière (R1 : un test linéaire ne s'écrit pas).
pub fn normalize(flow: &mut Flow) {
    let ids: Vec<String> = flow.steps.iter().map(|s| s.id.clone()).collect();
    let last = ids.len().saturating_sub(1);
    for (index, step) in flow.steps.iter_mut().enumerate() {
        let default = ids.get(index + 1).map_or(END, String::as_str);
        if step.next.as_deref() == Some(default)
            || (index == last && step.next.as_deref() == Some(END))
        {
            step.next = None;
        }
    }
}

/// Fichiers de requête du schéma, dans son ordre : étapes requête puis attentes.
pub fn files_in_order(flow: &Flow) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for step in &flow.steps {
        if let Some(file) = step.file()
            && !out.iter().any(|f| f == file)
        {
            out.push(file.to_owned());
        }
    }
    out
}

/// Vérifie un schéma avant de l'écrire ; renvoie la raison d'un refus, en français.
pub fn validate(flow: &Flow, files: &[String]) -> Result<(), String> {
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for step in &flow.steps {
        let id = step.id.trim();
        if id.is_empty() || id == END || id == FAIL {
            return Err("Une étape n'a pas d'identifiant valable.".to_owned());
        }
        if !ids.insert(id) {
            return Err(format!("L'identifiant d'étape « {id} » est en double."));
        }
    }
    let known = |target: &str| target == END || target == FAIL || ids.contains(target);
    let mut used_files: HashSet<&str> = HashSet::new();
    for (index, step) in flow.steps.iter().enumerate() {
        let label = if step.name.trim().is_empty() {
            format!("L'étape {}", index + 1)
        } else {
            format!("L'étape « {} »", step.name.trim())
        };
        for target in step.targets() {
            if !known(target) {
                return Err(format!("{label} renvoie vers une étape qui n'existe plus."));
            }
        }
        if let Some(file) = step.file() {
            if !files.iter().any(|f| f == file) {
                return Err(format!(
                    "{label} joue la requête « {file} », absente du test."
                ));
            }
            if !used_files.insert(file) {
                return Err(format!("La requête « {file} » est jouée par deux étapes."));
            }
        }
        match &step.action {
            Action::Loop {
                count, list, until, ..
            } => {
                if count.is_none() && list.as_deref().is_none_or(|l| l.trim().is_empty()) {
                    return Err(format!(
                        "{label} : choisissez un nombre de tours ou une liste à parcourir."
                    ));
                }
                if count.is_some_and(|n| n as usize > MAX_LOOP) {
                    return Err(format!("{label} : {MAX_LOOP} tours au plus."));
                }
                if let Some(until) = until {
                    let position = flow.position(until).unwrap_or(0);
                    if position <= index {
                        return Err(format!(
                            "{label} : la dernière étape répétée doit suivre la boucle."
                        ));
                    }
                }
            }
            Action::Wait {
                interval_ms,
                attempts,
                ..
            } => {
                if *attempts == 0 || *attempts > MAX_ATTEMPTS {
                    return Err(format!("{label} : de 1 à {MAX_ATTEMPTS} essais."));
                }
                if *interval_ms > MAX_INTERVAL_MS {
                    return Err(format!(
                        "{label} : {} s au plus entre deux essais.",
                        MAX_INTERVAL_MS / 1000
                    ));
                }
            }
            // Une variable sans nom, un test ou un template à choisir s'enregistrent « à
            // configurer » (4.3-b quater) : l'étape échoue à l'exécution (`incomplete`).
            _ => {}
        }
    }
    Ok(())
}

/// Ce qui manque à une étape enregistrée « à configurer » pour être jouée, en clair ; `None`
/// pour une étape complète.
pub fn incomplete(step: &Step) -> Option<&'static str> {
    match &step.action {
        Action::Variable { variable, .. } if variable.trim().is_empty() => {
            Some("nommez la variable")
        }
        Action::Test { test } if test.trim().is_empty() => Some("choisissez le test à jouer"),
        Action::Template { template, .. } if template.trim().is_empty() => {
            Some("choisissez le template à jouer")
        }
        _ => None,
    }
}
