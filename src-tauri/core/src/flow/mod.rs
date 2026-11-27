//! Tests en schéma (`docs/plan-v4.md`, lots A et B) : un test enchaîne des étapes de natures
//! variées (requête, condition, boucle, attente, variable, script, test existant, template),
//! chacune avec sa suite.
//!
//! Le schéma d'un test vit dans `extensions.arnod.tests` de `opencollection.yml`, sous le chemin
//! du test (R1, Q1) ; ses requêtes restent des fichiers OpenCollection du dossier. Un test sans
//! schéma écrit est la liste de ses requêtes. [`reconcile`] accorde le schéma relu au dossier
//! (R2), [`store`] le lit, l'écrit et le fait suivre (R1, R4), [`check`] évalue une condition
//! (R6), [`engine`] joue un test (R5, R7, R8).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::request::model::{Operator, Scalar};

pub mod check;
pub mod edit;
pub mod engine;
pub mod reconcile;
pub mod store;

#[cfg(test)]
mod tests;

/// Suite « Fin ✓ » : le test se termine sans échec ajouté (R5).
pub const END: &str = "end";
/// Suite « Fin en échec » : le test se termine en échec (R5).
pub const FAIL: &str = "fail";

/// Tours d'une boucle au plus (R7).
pub const MAX_LOOP: usize = 1_000;
/// Essais d'une attente au plus (R7).
pub const MAX_ATTEMPTS: u32 = 100;
/// Intervalle d'une attente au plus, en millisecondes (R7).
pub const MAX_INTERVAL_MS: u64 = 60_000;
/// Profondeur des tests appelés (« test existant », template) au plus (R7).
pub const MAX_DEPTH: usize = 8;
/// Étapes jouées par un parcours au plus (R7).
pub const MAX_STEPS: usize = 10_000;

/// Ce que fait un test quand une étape échoue (4.2-a « Si une étape échoue », 4.3-c, R5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OnFailure {
    /// « Arrêter le test ».
    #[default]
    Stop,
    /// « Continuer » : les étapes suivantes se jouent, le test reste en échec.
    Continue,
}

/// Condition d'une étape (R6) : une cible, un opérateur et une valeur, comme une assertion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "FlowCheck"))]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// `status`, `body.…`, `headers.…`, `responseTime` (dernière réponse) ou `vars.nom`.
    pub target: String,
    pub operator: Operator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub value: Option<Scalar>,
}

impl Check {
    /// Condition neuve : `status` égal à 200.
    pub fn new() -> Self {
        Self {
            target: "status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Int(200)),
        }
    }
}

impl Default for Check {
    fn default() -> Self {
        Self::new()
    }
}

/// Nature d'une étape et ce qu'elle porte (4.3-e).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "FlowAction"))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    /// Une requête : un fichier du dossier du test.
    Request { file: String },
    /// « Si … aller à une étape, sinon à une autre » : la suite de l'étape (`next`) vaut
    /// « si oui », `otherwise` « sinon ».
    Condition {
        check: Check,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        otherwise: Option<String>,
    },
    /// Répète les étapes qui la suivent, jusqu'à l'étape `until` comprise (Q5) : `count` fois, ou
    /// pour chaque élément de la liste `list` (`{{nom}}`, `vars.nom` ou `body.…` de la dernière
    /// réponse), rangé dans la variable `item`.
    Loop {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        count: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        list: Option<String>,
        #[serde(default)]
        item: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        until: Option<String>,
    },
    /// « Jusqu'à ce qu'une condition soit vraie » : renvoie la requête `file` (un fichier du
    /// dossier, propre à l'étape, R14), ou relit les variables, toutes les `interval_ms`, au plus
    /// `attempts` fois.
    Wait {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        file: Option<String>,
        check: Check,
        interval_ms: u64,
        attempts: u32,
    },
    /// « Définir ou calculer une valeur pour les étapes suivantes » : `{{…}}` résolues.
    Variable { variable: String, value: String },
    /// Quelques lignes de JavaScript isolé (4.3-g, R9).
    Script { code: String },
    /// Un test du référentiel, joué tel quel avec les variables du test (chemin du dossier).
    Test { test: String },
    /// Un template (chemin sous `templates/`) et les valeurs de ses paramètres.
    Template {
        template: String,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
}

/// Nature d'une étape, sans ce qu'elle porte (lignes d'un run, palette).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum StepKind {
    #[default]
    Request,
    Condition,
    Loop,
    Wait,
    Variable,
    Script,
    Test,
    Template,
}

impl StepKind {
    /// Libellé de la nature, en français.
    pub fn label(self) -> &'static str {
        match self {
            Self::Request => "Requête",
            Self::Condition => "Condition",
            Self::Loop => "Boucle",
            Self::Wait => "Attente",
            Self::Variable => "Variable",
            Self::Script => "Script",
            Self::Test => "Test existant",
            Self::Template => "Template",
        }
    }
}

impl Action {
    /// Nature de l'étape.
    pub fn step_kind(&self) -> StepKind {
        match self {
            Self::Request { .. } => StepKind::Request,
            Self::Condition { .. } => StepKind::Condition,
            Self::Loop { .. } => StepKind::Loop,
            Self::Wait { .. } => StepKind::Wait,
            Self::Variable { .. } => StepKind::Variable,
            Self::Script { .. } => StepKind::Script,
            Self::Test { .. } => StepKind::Test,
            Self::Template { .. } => StepKind::Template,
        }
    }

    /// Nom de la nature, tel qu'écrit dans le fichier (`request`, `condition`…).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Request { .. } => "request",
            Self::Condition { .. } => "condition",
            Self::Loop { .. } => "loop",
            Self::Wait { .. } => "wait",
            Self::Variable { .. } => "variable",
            Self::Script { .. } => "script",
            Self::Test { .. } => "test",
            Self::Template { .. } => "template",
        }
    }
}

/// Étape d'un schéma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "FlowStep"))]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Identifiant stable : les renvois suivent l'étape, pas son numéro (R3).
    pub id: String,
    /// Nom affiché ; vide pour une requête (son nom est celui de la requête).
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub action: Action,
    /// Suite : une étape, [`END`] ou [`FAIL`] ; absente, l'étape suivante de la liste (R5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    /// « Si cette étape échoue » ; absent, le réglage du test (4.3-c).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub on_failure: Option<OnFailure>,
}

impl Step {
    /// Étape requête du fichier `file`, d'identifiant tiré de son nom.
    pub fn request(file: &str) -> Self {
        Self {
            id: request_id(file),
            name: String::new(),
            action: Action::Request {
                file: file.to_owned(),
            },
            next: None,
            on_failure: None,
        }
    }

    /// Fichier de requête que l'étape joue, s'il y en a un (requête ou attente).
    pub fn file(&self) -> Option<&str> {
        match &self.action {
            Action::Request { file } => Some(file),
            Action::Wait { file, .. } => file.as_deref(),
            _ => None,
        }
    }

    /// Renvois de l'étape vers d'autres étapes (suite, « sinon », fin de boucle).
    pub fn targets(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.next.as_deref().into_iter().collect();
        match &self.action {
            Action::Condition {
                otherwise: Some(target),
                ..
            }
            | Action::Loop {
                until: Some(target),
                ..
            } => out.push(target),
            _ => {}
        }
        out
    }
}

/// Identifiant de l'étape requête d'un fichier, tant que le schéma n'est pas écrit.
pub fn request_id(file: &str) -> String {
    format!("r:{file}")
}

/// Schéma d'un test (R1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    #[serde(default)]
    pub on_failure: OnFailure,
    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Flow {
    /// Le test se joue comme la liste de ses requêtes (R1) : pas de renvoi, pas d'autre nature,
    /// arrêt sur échec, dans l'ordre `files` du dossier.
    pub fn is_linear(&self, files: &[String]) -> bool {
        self.on_failure == OnFailure::Stop
            && self.steps.len() == files.len()
            && self.steps.iter().zip(files).all(|(step, file)| {
                step.next.is_none()
                    && step.on_failure.is_none()
                    && matches!(&step.action, Action::Request { file: f } if f == file)
            })
    }

    /// Position de l'étape `id`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.steps.iter().position(|step| step.id == id)
    }

    /// Suite effective de l'étape `index` : son renvoi, sinon l'étape suivante, sinon la fin.
    pub fn following(&self, index: usize) -> String {
        match self.steps.get(index).and_then(|s| s.next.clone()) {
            Some(target) => target,
            None => self
                .steps
                .get(index + 1)
                .map_or_else(|| END.to_owned(), |s| s.id.clone()),
        }
    }

    /// Identifiant libre pour une nouvelle étape (`s1`, `s2`…).
    pub fn free_id(&self) -> String {
        (1..)
            .map(|n| format!("s{n}"))
            .find(|id| self.position(id).is_none())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }
}

/// Phase d'un Avant / Après de dossier (4.7-c, R13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HookPhase {
    /// « Avant le dossier » : une fois, au début.
    BeforeAll,
    /// « Avant chaque test ».
    BeforeEach,
    /// « Après chaque test », même en échec.
    AfterEach,
    /// « Après le dossier », une fois, à la fin, même en échec.
    AfterAll,
}

impl HookPhase {
    pub const ALL: [Self; 4] = [
        Self::BeforeAll,
        Self::BeforeEach,
        Self::AfterEach,
        Self::AfterAll,
    ];

    /// Libellé de l'interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::BeforeAll => "Avant le dossier",
            Self::BeforeEach => "Avant chaque test",
            Self::AfterEach => "Après chaque test",
            Self::AfterAll => "Après le dossier",
        }
    }
}

/// Avant / Après d'un dossier (`extensions.arnod.folders`, R13).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FolderHooks {
    #[serde(default)]
    pub before_all: Vec<Step>,
    #[serde(default)]
    pub before_each: Vec<Step>,
    #[serde(default)]
    pub after_each: Vec<Step>,
    #[serde(default)]
    pub after_all: Vec<Step>,
}

impl FolderHooks {
    pub fn is_empty(&self) -> bool {
        self.before_all.is_empty()
            && self.before_each.is_empty()
            && self.after_each.is_empty()
            && self.after_all.is_empty()
    }

    pub fn phase(&self, phase: HookPhase) -> &[Step] {
        match phase {
            HookPhase::BeforeAll => &self.before_all,
            HookPhase::BeforeEach => &self.before_each,
            HookPhase::AfterEach => &self.after_each,
            HookPhase::AfterAll => &self.after_all,
        }
    }

    pub fn phase_mut(&mut self, phase: HookPhase) -> &mut Vec<Step> {
        match phase {
            HookPhase::BeforeAll => &mut self.before_all,
            HookPhase::BeforeEach => &mut self.before_each,
            HookPhase::AfterEach => &mut self.after_each,
            HookPhase::AfterAll => &mut self.after_all,
        }
    }

    /// Toutes les étapes, phase par phase.
    pub fn steps(&self) -> impl Iterator<Item = (HookPhase, &Step)> {
        HookPhase::ALL
            .into_iter()
            .flat_map(move |phase| self.phase(phase).iter().map(move |step| (phase, step)))
    }
}
