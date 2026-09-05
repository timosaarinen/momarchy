use std::{
    env, fs,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";
const ZIG_LINUX_TARGET: &str = "x86_64-unknown-linux-gnu.2.17";
const REMOTE_SCREENSHOT: &str = "~/.local/state/momarchy/screenshot.png";
const REMOTE_PROVISIONER_NEW: &str = "~/.local/state/momarchy/install.sh.new";
const REMOTE_PROVISIONER_CHECK: &str = "~/.local/state/momarchy/install.sh.check";
const SSH_OPTIONS: &[&str] = &[
    "-o",
    "BatchMode=yes",
    "-o",
    "ConnectTimeout=5",
    "-o",
    "ConnectionAttempts=1",
    "-o",
    "ServerAliveInterval=10",
    "-o",
    "ServerAliveCountMax=3",
];

fn main() -> ExitCode {
    let mut args = env::args().skip(1);

    let result = match args.next().as_deref() {
        Some("provision") => match args.next() {
            Some(target) => provision(&target),
            None => Err(
                "usage: cargo provision <ssh-target>\nexample: cargo provision t@momarchy".into(),
            ),
        },
        Some("deploy") => match args.next() {
            Some(target) => deploy(&target),
            None => Err("usage: cargo deploy <ssh-target>\nexample: cargo deploy t@momarchy".into()),
        },
        Some("home") => dev_home(args.collect()),
        Some("screenshot") => match args.next() {
            Some(target) => screenshot(&target),
            None => Err(
                "usage: cargo screenshot <ssh-target>\nexample: cargo screenshot t@momarchy".into(),
            ),
        },
        _ => Err(
            "xtask commands:\n  home [momarchy-home-options]\n  provision <ssh-target>\n  deploy <ssh-target>\n  screenshot <ssh-target>"
                .into(),
        ),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask failed:\n{error}");
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

fn screenshot(target: &str) -> Result<(), String> {
    ensure_ssh_reachable(target, "remote screenshot")?;

    let root = workspace_root()?;
    let output_dir = root.join("target/screenshots");
    let output = output_dir.join("momarchy.png");

    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("could not create {}: {error}", output_dir.display()))?;

    println!("==> capturing full Wayland output on {target}");
    let mut capture = ssh_command(target, false);
    capture.arg(
        r#"set -eu; state="$HOME/.local/state/momarchy"; mkdir -p "$state"; omarchy_cmd=$(command -v omarchy) || { printf '%s\n' 'Omarchy screenshot command is unavailable on the target' >&2; exit 127; }; hyprctl_cmd=$(command -v hyprctl) || { printf '%s\n' 'hyprctl is unavailable on the target' >&2; exit 127; }; command -v jq >/dev/null 2>&1 || { printf '%s\n' 'jq is required to inspect target display state' >&2; exit 127; }; rm -f "$state"/screenshot-*.png "$state/screenshot.png"; if ! dpms_json=$(systemd-run --user --quiet --pipe --wait --collect --property=RuntimeMaxSec=5s --setenv=PATH="$PATH" "$hyprctl_cmd" monitors -j); then printf '%s\n' 'Could not query Hyprland display state through the graphical user session' >&2; exit 1; fi; was_asleep=$(printf '%s\n' "$dpms_json" | jq -r '[.[] | select(.disabled == false)] | length > 0 and all(.dpmsStatus == false)'); if [ "$was_asleep" = true ]; then printf '%s\n' 'Target display is asleep; waking it temporarily through Omarchy.' >&2; systemd-run --user --pipe --wait --collect --property=RuntimeMaxSec=5s --setenv=PATH="$PATH" "$omarchy_cmd" brightness display on; sleep 0.5; fi; set +e; systemd-run --user --pipe --wait --collect --property=RuntimeMaxSec=15s --setenv=PATH="$PATH" --setenv=OMARCHY_SCREENSHOT_DIR="$state" "$omarchy_cmd" capture screenshot fullscreen save; capture_status=$?; set -e; if [ "$was_asleep" = true ]; then printf '%s\n' 'Restoring target display sleep state through Omarchy.' >&2; systemd-run --user --pipe --wait --collect --property=RuntimeMaxSec=5s --setenv=PATH="$PATH" "$omarchy_cmd" brightness display off || printf '%s\n' 'Warning: could not restore target display sleep state' >&2; fi; [ "$capture_status" -eq 0 ] || exit "$capture_status"; shot=$(find "$state" -maxdepth 1 -type f -name 'screenshot-*.png' -print -quit); [ -n "$shot" ] || { printf '%s\n' 'Omarchy screenshot command completed without producing a PNG' >&2; exit 1; }; mv -f "$shot" "$state/screenshot.png""#,
    );
    run(&mut capture).map_err(|error| {
        format!(
            "{error}\n\nSuggested checks:\n  ssh {target} 'command -v omarchy grim hyprctl jq'\n  ssh {target} 'systemctl --user show-environment | grep -E \"^(WAYLAND_DISPLAY|HYPRLAND_INSTANCE_SIGNATURE)=\"'\n\nIf the target display is asleep, the screenshot task should now wake it temporarily with `omarchy brightness display on` and restore sleep afterward. The screenshot path intentionally delegates to Omarchy first. Any Omarchy/systemd/grim/hyprctl stderr should appear above this summary."
        )
    })?;

    println!("==> copying screenshot to {}", output.display());
    let mut copy = scp_command();
    copy.arg(format!("{target}:{REMOTE_SCREENSHOT}")).arg(&output);
    run(&mut copy)?;

    let mut cleanup = ssh_command(target, false);
    let _ = cleanup
        .arg("rm -f \"$HOME/.local/state/momarchy/screenshot.png\"")
        .status();

    println!("==> screenshot saved to {}", output.display());

    if env::consts::OS == "macos" {
        run(Command::new("open").arg(&output))?;
    }

    Ok(())
}

fn provision(target: &str) -> Result<(), String> {
    ensure_ssh_reachable(target, "target provisioning")?;

    let root = workspace_root()?;
    let installer = root.join("install.sh");
    if !installer.is_file() {
        return Err(format!("provision input not found: {}", installer.display()));
    }

    println!("==> preparing provisioning state on {target}");
    let mut prepare = ssh_command(target, false);
    prepare.arg("mkdir -p \"$HOME/.local/state/momarchy\"");
    run(&mut prepare)?;

    println!("==> uploading target provisioner {}", installer.display());
    let mut upload = scp_command();
    upload
        .arg(&installer)
        .arg(format!("{target}:{REMOTE_PROVISIONER_NEW}"));
    run(&mut upload)?;

    println!("==> provisioning Momarchy appliance settings on {target}");
    let mut provision = ssh_command(target, true);
    provision.arg(
        "set -eu; new=\"$HOME/.local/state/momarchy/install.sh.new\"; applied=\"$HOME/.local/state/momarchy/install.sh.applied\"; chmod 755 \"$new\"; set +e; bash \"$new\"; status=$?; set -e; if [ \"$status\" -eq 0 ]; then mv -f \"$new\" \"$applied\"; else rm -f \"$new\"; exit \"$status\"; fi",
    );
    run(&mut provision).map_err(|error| {
        format!(
            "{error}\n\nTarget provisioning failed. Passwordless SSH must already work before `cargo provision`. Provisioning is the explicit operation that may ask for the target user's sudo password when packages or SDDM configuration need privileged changes.\n\nSuggested checks:\n  ssh {target} true\n  ssh -t {target} 'sudo -v'\n  ssh {target} 'command -v omarchy && omarchy --version || true'"
        )
    })?;

    println!("==> provisioning is current; deploying Momarchy");
    deploy_release(target)?;
    println!("    reboot the target once after first-time provisioning to prove boot -> Momarchy Home");
    Ok(())
}

fn deploy(target: &str) -> Result<(), String> {
    ensure_ssh_reachable(target, "deployment")?;

    let root = workspace_root()?;
    let installer = root.join("install.sh");
    if !installer.is_file() {
        return Err(format!("deploy input not found: {}", installer.display()));
    }

    ensure_provision_current(target, &installer)?;
    deploy_release(target)
}

fn ensure_provision_current(target: &str, installer: &Path) -> Result<(), String> {
    println!("==> checking target provisioning state on {target}");
    let mut prepare = ssh_command(target, false);
    prepare.arg("mkdir -p \"$HOME/.local/state/momarchy\"");
    run(&mut prepare)?;

    let mut upload = scp_command();
    upload
        .arg(installer)
        .arg(format!("{target}:{REMOTE_PROVISIONER_CHECK}"));
    run(&mut upload)?;

    let mut check = ssh_command(target, false);
    check.arg(
        "set -eu; check=\"$HOME/.local/state/momarchy/install.sh.check\"; applied=\"$HOME/.local/state/momarchy/install.sh.applied\"; trap 'rm -f \"$check\"' EXIT; [ -f \"$applied\" ] || { printf '%s\\n' 'Momarchy target has not been provisioned with the current workflow.' >&2; exit 1; }; cmp -s \"$check\" \"$applied\" || { printf '%s\\n' 'Momarchy target provisioning is stale relative to this checkout.' >&2; exit 1; }",
    );

    run(&mut check).map_err(|error| {
        format!(
            "{error}\n\nNormal deploys never re-run privileged/system provisioning automatically. Apply the current provisioning explicitly, then deploy will become fast again:\n  cargo provision {target}"
        )
    })
}

fn deploy_release(target: &str) -> Result<(), String> {
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
    let mut prepare = ssh_command(target, false);
    prepare.arg("mkdir -p \"$HOME/.local/bin\" \"$HOME/.config/momarchy/momarchy\"");
    run(&mut prepare)?;

    println!("==> uploading {}", binary.display());
    let mut upload_binary = scp_command();
    upload_binary
        .arg(&binary)
        .arg(format!("{target}:~/.local/bin/momarchy.new"));
    run(&mut upload_binary)?;

    println!("==> uploading {}", config.display());
    let mut upload_config = scp_command();
    upload_config
        .arg(&config)
        .arg(format!("{target}:~/.config/momarchy/init.lua.new"));
    run(&mut upload_config)?;

    println!("==> uploading {}", ui_module.display());
    let mut upload_ui = scp_command();
    upload_ui
        .arg(&ui_module)
        .arg(format!("{target}:~/.config/momarchy/momarchy/ui.lua.new"));
    run(&mut upload_ui)?;

    println!("==> activating release and repo Lua");
    let mut activate = ssh_command(target, false);
    activate.arg(
        "set -eu; chmod 755 \"$HOME/.local/bin/momarchy.new\"; mv -f \"$HOME/.config/momarchy/momarchy/ui.lua.new\" \"$HOME/.config/momarchy/momarchy/ui.lua\"; mv -f \"$HOME/.local/bin/momarchy.new\" \"$HOME/.local/bin/momarchy\"; mv -f \"$HOME/.config/momarchy/init.lua.new\" \"$HOME/.config/momarchy/init.lua\"; \"$HOME/.local/bin/momarchy\" status; printf 'quit\\n' | \"$HOME/.local/bin/momarchy\" home --automation >/dev/null",
    );
    run(&mut activate)?;

    println!("==> deployed binary and repo Lua to {target}");
    Ok(())
}

fn ensure_ssh_reachable(target: &str, operation: &str) -> Result<(), String> {
    println!("==> checking passwordless SSH reachability to {target}");
    let mut command = ssh_command(target, false);
    command.arg("true");

    run(&mut command).map_err(|error| {
        format!(
            "{error}\n\nCould not reach {target} over passwordless SSH for {operation}. The target may be powered off, suspended/hibernating, have its lid closed, be off-network, or SSH may not be ready.\n\nSuggested checks:\n  wake/open the target and retry\n  ssh -o BatchMode=yes -o ConnectTimeout=5 {target} true\n\nMomarchy uses one SSH connection attempt with a 5-second connect timeout. Established remote sessions use SSH keepalives so a target that sleeps or drops off the network mid-operation also fails instead of hanging indefinitely."
        )
    })
}

fn ssh_command(target: &str, tty: bool) -> Command {
    let mut command = Command::new("ssh");
    command.args(SSH_OPTIONS);
    if tty {
        command.arg("-t");
    }
    command.arg(target);
    command
}

fn scp_command() -> Command {
    let mut command = Command::new("scp");
    command.args(SSH_OPTIONS);
    command
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
                    "{error}\n\nSuggested fix if the Rust Linux target is missing:\n  rustup target add {LINUX_TARGET}"
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
    let command_line = command_line(command);
    let cwd = command_cwd(command);
    let output = command.output().map_err(|error| {
        format!(
            "{message}\n  command: {command_line}{cwd}\n  start error: {error}"
        )
    })?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{message}\n  command: {command_line}{cwd}\n  status: {}{}{}",
        output.status,
        captured_output("stdout", &output.stdout),
        captured_output("stderr", &output.stderr),
    ))
}

fn run(command: &mut Command) -> Result<(), String> {
    let command_line = command_line(command);
    let cwd = command_cwd(command);
    let status = command.status().map_err(|error| {
        format!(
            "could not start command\n  command: {command_line}{cwd}\n  start error: {error}"
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command failed\n  command: {command_line}{cwd}\n  status: {status}\n  stdout/stderr: inherited live; see output above"
        ))
    }
}

fn command_line(command: &Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn command_cwd(command: &Command) -> String {
    command
        .get_current_dir()
        .map(|path| format!("\n  working directory: {}", path.display()))
        .unwrap_or_default()
}

fn shell_word(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    let simple = !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_alphanumeric() || "-_./:@=+,%".contains(character)
        });

    if simple {
        text.into_owned()
    } else {
        format!("{text:?}")
    }
}

fn captured_output(label: &str, bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("\n  {label}:\n{}", indent(trimmed, "    "))
    }
}

fn indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
