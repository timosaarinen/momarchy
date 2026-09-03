mod home;
mod status;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        None | Some("--help" | "-h" | "help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            println!("momarchy {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("status") => match status::print() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&format!("status failed: {error}")),
        },
        Some("home") => match home::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&format!("home failed: {error}")),
        },
        Some(command) => fail(&format!("unknown command: {command}\n\nRun 'momarchy --help'.")),
    }
}

fn print_help() {
    println!(
        "Momarchy {}\n\n\
Usage:\n  momarchy <command>\n\n\
Commands:\n  home       Open the Momarchy Home TUI prototype\n  status     Print a small machine status summary\n  help       Show this help\n\n\
Options:\n  -h, --help       Show help\n  -V, --version    Show version",
        env!("CARGO_PKG_VERSION")
    );
}

fn fail(message: &str) -> ExitCode {
    eprintln!("momarchy: {message}");
    ExitCode::FAILURE
}
