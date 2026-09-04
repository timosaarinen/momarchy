use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("deploy") => match args.next() {
            Some(target) => match deploy(&target) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("deploy failed: {error}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("usage: cargo deploy <ssh-target>\nexample: cargo deploy t@momarchy");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("xtask commands:\n  deploy <ssh-target>");
            ExitCode::FAILURE
        }
    }
}

fn deploy(target: &str) -> Result<(), String> {
    if env::consts::OS != "linux" {
        return Err("deployment currently expects a Linux build host (WSL2 is fine)".into());
    }

    let root = workspace_root()?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    println!("==> building Momarchy release");
    run(Command::new(cargo).current_dir(&root).args([
        "build",
        "--release",
        "--package",
        "momarchy",
    ]))?;

    let binary = root.join("target/release/momarchy");
    if !binary.is_file() {
        return Err(format!("release binary not found: {}", binary.display()));
    }

    println!("==> preparing {target}");
    run(Command::new("ssh")
        .arg(target)
        .arg("mkdir -p \"$HOME/.local/bin\""))?;

    println!("==> uploading {}", binary.display());
    let remote_staging = format!("{target}:~/.local/bin/momarchy.new");
    run(Command::new("scp").arg(&binary).arg(remote_staging))?;

    println!("==> activating release");
    run(Command::new("ssh").arg(target).arg(
        "set -eu; chmod 755 \"$HOME/.local/bin/momarchy.new\"; mv -f \"$HOME/.local/bin/momarchy.new\" \"$HOME/.local/bin/momarchy\"; \"$HOME/.local/bin/momarchy\" status",
    ))?;

    println!("==> deployed to {target}");
    Ok(())
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not find workspace root".into())
}

fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("could not start command: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with {status}"))
    }
}
