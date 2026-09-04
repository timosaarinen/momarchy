use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";
const ZIG_LINUX_TARGET: &str = "x86_64-unknown-linux-gnu.2.17";

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
    let root = workspace_root()?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let binary = build_release(&root, &cargo)?;

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

fn build_release(root: &Path, cargo: &str) -> Result<PathBuf, String> {
    match env::consts::OS {
        "linux" => {
            println!("==> building Momarchy release for Linux");
            run(Command::new(cargo).current_dir(root).args([
                "build",
                "--release",
                "--package",
                "momarchy",
            ]))?;

            Ok(root.join("target/release/momarchy"))
        }
        "macos" => {
            require_command(
                Command::new("zig").arg("version"),
                "macOS deployment needs Zig for the Linux cross-linker; install it once with `brew install zig`",
            )?;
            require_command(
                Command::new(cargo).args(["zigbuild", "--version"]),
                "macOS deployment needs cargo-zigbuild; install it once with `cargo install --locked cargo-zigbuild`",
            )?;

            println!("==> cross-building Momarchy release for Linux with Zig");
            run(Command::new(cargo).current_dir(root).args([
                "zigbuild",
                "--release",
                "--package",
                "momarchy",
                "--target",
                ZIG_LINUX_TARGET,
            ]))
            .map_err(|error| {
                format!(
                    "{error}\nif the Rust Linux target is missing, install it once with `rustup target add {LINUX_TARGET}`"
                )
            })?;

            Ok(root.join(format!("target/{LINUX_TARGET}/release/momarchy")))
        }
        host => Err(format!(
            "unsupported deployment build host `{host}`; use Linux or macOS"
        )),
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not find workspace root".into())
}

fn require_command(command: &mut Command, message: &str) -> Result<(), String> {
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        _ => Err(message.into()),
    }
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
