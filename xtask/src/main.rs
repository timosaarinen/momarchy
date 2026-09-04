use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";
const ZIG_LINUX_TARGET: &str = "x86_64-unknown-linux-gnu.2.17";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);

    let result = match args.next().as_deref() {
        Some("deploy") => match args.next() {
            Some(target) => deploy(&target),
            None => Err("usage: cargo deploy <ssh-target>\nexample: cargo deploy t@momarchy".into()),
        },
        Some("home") => dev_home(args.collect()),
        _ => Err("xtask commands:\n  home [momarchy-home-options]\n  deploy <ssh-target>".into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dev_home(home_args: Vec<String>) -> Result<(), String> {
    let root = workspace_root()?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let source_config = root.join("lua/init.lua");
    let xdg_config_home = root.join("target/momarchy-dev-config");
    let config_dir = xdg_config_home.join("momarchy");
    let staged_config = config_dir.join("init.lua");

    fs::create_dir_all(&config_dir)
        .map_err(|error| format!("could not create {}: {error}", config_dir.display()))?;
    fs::copy(&source_config, &staged_config).map_err(|error| {
        format!(
            "could not stage {} as {}: {error}",
            source_config.display(),
            staged_config.display()
        )
    })?;

    let mut command = Command::new(cargo);
    command
        .current_dir(&root)
        .env("XDG_CONFIG_HOME", &xdg_config_home)
        .args(["run", "--package", "momarchy", "--", "home"])
        .args(home_args);

    run(&mut command)
}

fn deploy(target: &str) -> Result<(), String> {
    let root = workspace_root()?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let binary = build_release(&root, &cargo)?;
    let config = root.join("lua/init.lua");
    let ui_module = root.join("lua/momarchy/ui.lua");

    for path in [&binary, &config, &ui_module] {
        if !path.is_file() {
            return Err(format!("deploy input not found: {}", path.display()));
        }
    }

    println!("==> preparing {target}");
    run(Command::new("ssh").arg(target).arg(
        "mkdir -p \"$HOME/.local/bin\" \"$HOME/.config/momarchy/momarchy\"",
    ))?;

    println!("==> uploading {}", binary.display());
    run(Command::new("scp")
        .arg(&binary)
        .arg(format!("{target}:~/.local/bin/momarchy.new")))?;

    println!("==> uploading {}", config.display());
    run(Command::new("scp")
        .arg(&config)
        .arg(format!("{target}:~/.config/momarchy/init.lua.new")))?;

    println!("==> uploading {}", ui_module.display());
    run(Command::new("scp")
        .arg(&ui_module)
        .arg(format!("{target}:~/.config/momarchy/momarchy/ui.lua.new")))?;

    println!("==> activating release and repo Lua");
    run(Command::new("ssh").arg(target).arg(
        "set -eu; chmod 755 \"$HOME/.local/bin/momarchy.new\"; mv -f \"$HOME/.config/momarchy/momarchy/ui.lua.new\" \"$HOME/.config/momarchy/momarchy/ui.lua\"; mv -f \"$HOME/.local/bin/momarchy.new\" \"$HOME/.local/bin/momarchy\"; mv -f \"$HOME/.config/momarchy/init.lua.new\" \"$HOME/.config/momarchy/init.lua\"; \"$HOME/.local/bin/momarchy\" status; printf 'quit\\n' | \"$HOME/.local/bin/momarchy\" home --automation >/dev/null",
    ))?;

    println!("==> deployed binary and repo Lua to {target}");
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
                Command::new(cargo).args(["zigbuild", "--help"]),
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
    match command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
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
