mod browser;
mod config;
mod games;
mod home;
mod ipc;
mod status;
mod tv;
mod watch;

use std::{
    io::{self, Read},
    process::ExitCode,
};

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
        Some("ping") => match ipc::send_ping() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&format!("ping failed: {error}")),
        },
        Some("msg") => {
            let message_args = args.collect::<Vec<_>>();
            let message = if message_args.as_slice() == ["--stdin"] {
                let mut message = String::new();
                if let Err(error) = io::stdin().read_to_string(&mut message) {
                    return fail(&format!("could not read message from stdin: {error}"));
                }
                message
                    .trim_end_matches(|character| character == '\r' || character == '\n')
                    .to_owned()
            } else {
                message_args.join(" ")
            };

            if message.is_empty() {
                return fail("usage: momarchy msg <message>\n       momarchy msg --stdin");
            }

            match ipc::send_message(&message) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => fail(&format!("message failed: {error}")),
            }
        }
        Some("home") => {
            let mut options = home::Options::default();

            for arg in args {
                match arg.as_str() {
                    "--automation" => options.automation = true,
                    "--live-actions" => options.live_actions = true,
                    "--dry-run" => options.live_actions = false,
                    "--help" | "-h" => {
                        print_home_help();
                        return ExitCode::SUCCESS;
                    }
                    _ => {
                        return fail(&format!(
                            "unknown home option: {arg}\n\nRun 'momarchy home --help'."
                        ));
                    }
                }
            }

            match home::run(options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => fail(&format!("home failed: {error}")),
            }
        }
        Some(command) => fail(&format!(
            "unknown command: {command}\n\nRun 'momarchy --help'."
        )),
    }
}

fn print_help() {
    println!(
        "Momarchy {}\n\n\
Usage:\n  momarchy <command>\n\n\
Commands:\n  home       Open Momarchy Home\n  status     Print a small machine status summary\n  ping       Show an attention ping in a running Momarchy Home\n  msg        Show a message in a running Momarchy Home\n  help       Show this help\n\n\
Options:\n  -h, --help       Show help\n  -V, --version    Show version",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_home_help() {
    println!(
        "Momarchy Home\n\n\
Usage:\n  momarchy home [options]\n\n\
Options:\n  --dry-run        Do not launch host programs (default)\n  --live-actions   Allow host programs to launch\n  --automation     Use the stdin/stdout automation interface instead of a real terminal\n  -h, --help       Show this help"
    );
}

fn fail(message: &str) -> ExitCode {
    eprintln!("momarchy: {message}");
    ExitCode::FAILURE
}
