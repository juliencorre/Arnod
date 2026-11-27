//! `arnod` : joue les tests d'un projet Arnod hors de l'application (`docs/plan-v2.md`, lot D).
//! Exécutable autonome (R16) : même cœur que l'application, sans fenêtre ni installation à
//! part.

mod args;
mod output;
mod project;
mod run;
mod select;
mod suggest;

use std::io::Write;
use std::process::ExitCode;

use args::Command;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = match args::parse(&argv) {
        Ok(Command::Help) => {
            print!("{}", args::HELP);
            run::OK
        }
        Ok(Command::RunHelp) => {
            print!("{}", args::RUN_HELP);
            run::OK
        }
        Ok(Command::Version) => {
            println!("arnod {}", env!("CARGO_PKG_VERSION"));
            run::OK
        }
        Ok(Command::Run(options)) => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let vars: Vec<(String, String)> = std::env::vars().collect();
            run::run(
                &options,
                &cwd,
                &vars,
                Box::new(std::io::stdout()),
                &mut std::io::stderr(),
            )
        }
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "{message}");
            run::INVALID
        }
    };
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}
