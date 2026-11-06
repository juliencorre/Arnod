// Évite l'ouverture d'une console supplémentaire sous Windows en release. NE PAS SUPPRIMER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    arnod_lib::run();
}
