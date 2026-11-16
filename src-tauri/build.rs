fn main() {
    tauri_build::build();

    // Lot L : les tests d'intégration (`tests/`) construisent une application Tauri simulée, qui
    // importe des fonctions des Common Controls v6. Sous Windows (MSVC), le système ne les charge
    // qu'avec le manifeste d'application, que `tauri-build` n'ajoute qu'aux exécutables de
    // l'application : sans lui, le binaire de test ne démarre pas (STATUS_ENTRYPOINT_NOT_FOUND).
    // Même réglage que les tests de Tauri (`tauri/build.rs`), limité aux tests d'intégration pour
    // ne pas doubler le manifeste de l'application.
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if windows && msvc {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
