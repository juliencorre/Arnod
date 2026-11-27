//! LC6 : commande copiée, citée pour PowerShell et sh.

use super::*;

#[test]
fn lc6_commande_d_un_dossier_avec_l_environnement_actif() {
    for shell in [Shell::PowerShell, Shell::Sh] {
        assert_eq!(
            run_command("commandes/v2", Some("Recette fonctionnelle"), shell),
            "arnod run \"commandes/v2\" --env \"Recette fonctionnelle\""
        );
    }
    assert_eq!(run_command("", None, Shell::Sh), "arnod run");
    assert_eq!(
        run_command("/Clients/", Some("  "), Shell::Sh),
        "arnod run \"Clients\""
    );
}

#[test]
fn lc6_rien_ne_s_interprete_dans_une_valeur_citee() {
    assert_eq!(quote("Prix $HT", Shell::Sh), "'Prix $HT'");
    assert_eq!(quote("l'essai $1", Shell::Sh), "'l'\\''essai $1'");
    assert_eq!(quote("Prix $HT", Shell::PowerShell), "'Prix $HT'");
    assert_eq!(
        quote("l'essai `n $x", Shell::PowerShell),
        "'l''essai `n $x'"
    );
    assert_eq!(quote("dit \"oui\"", Shell::Sh), "'dit \"oui\"'");
    // Une apostrophe seule ne s'interprète pas entre guillemets doubles.
    assert_eq!(quote("l'essai", Shell::Sh), "\"l'essai\"");
}

#[test]
fn le_terminal_se_lit_en_camel_case() {
    let shell: Shell = serde_json::from_str("\"powerShell\"").unwrap();
    assert_eq!(shell, Shell::PowerShell);
    let shell: Shell = serde_json::from_str("\"sh\"").unwrap();
    assert_eq!(shell, Shell::Sh);
}
