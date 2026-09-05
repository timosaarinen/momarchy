use std::{
    env,
    ffi::OsStr,
    process::{Command, ExitCode},
    time::{Duration, Instant},
};

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
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("session failed:\n{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let target = args
        .next()
        .ok_or_else(|| usage().to_string())?
        .to_string_lossy()
        .into_owned();
    let command: Vec<_> = args.collect();

    if command.is_empty() {
        return Err(usage().into());
    }

    ensure_ssh_reachable(&target)?;

    let rendered = command
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    println!("==> running in {target} graphical session: {rendered}");

    let remote_command = command
        .iter()
        .map(|argument| shell_quote(argument))
        .collect::<Vec<_>>()
        .join(" ");

    let script = format!(
        "set -eu; env=$(systemctl --user show-environment) || {{ printf '%s\\n' 'Could not read the target user systemd environment.' >&2; exit 1; }}; printf '%s\\n' \"$env\" | grep -q '^WAYLAND_DISPLAY=' || {{ printf '%s\\n' 'No WAYLAND_DISPLAY is registered in the target user systemd environment; the graphical Omarchy session may not be running.' >&2; exit 1; }}; printf '%s\\n' \"$env\" | grep -q '^HYPRLAND_INSTANCE_SIGNATURE=' || {{ printf '%s\\n' 'No HYPRLAND_INSTANCE_SIGNATURE is registered in the target user systemd environment; Hyprland may not be running.' >&2; exit 1; }}; command -v systemd-run >/dev/null 2>&1 || {{ printf '%s\\n' 'systemd-run is unavailable on the target.' >&2; exit 127; }}; exec systemd-run --user --quiet --pipe --wait --collect --setenv=PATH=\"$PATH\" -- {remote_command}"
    );

    let mut ssh = ssh_command(&target);
    ssh.arg(script);
    run(&mut ssh).map_err(|error| {
        format!(
            "{error}\n\nThe command was run through the target user's systemd manager so it inherits the real Omarchy/Hyprland session environment.\n\nUseful checks:\n  cargo session {target} hyprctl configerrors\n  cargo session {target} hyprctl reload\n\nFor shell syntax such as pipes or redirects, invoke a shell explicitly, for example:\n  cargo session {target} sh -lc 'hyprctl monitors -j | jq .[0]'"
        )
    })
}

fn usage() -> &'static str {
    "usage: cargo session <ssh-target> <command> [args...]\nexample: cargo session t@momarchy hyprctl configerrors"
}

fn ensure_ssh_reachable(target: &str) -> Result<(), String> {
    println!("==> checking passwordless SSH reachability to {target}");
    let mut command = ssh_command(target);
    command.arg("true");

    run_with_timeout(&mut command, Duration::from_secs(8)).map_err(|error| {
        format!(
            "{error}\n\nCould not reach {target} over passwordless SSH. The target may be powered off, suspended/hibernating, have its lid closed, be off-network, or SSH may not be ready.\n\nSuggested checks:\n  wake/open the target and retry\n  ssh -o BatchMode=yes -o ConnectTimeout=5 {target} true"
        )
    })
}

fn ssh_command(target: &str) -> Command {
    let mut command = Command::new("ssh");
    command.args(SSH_OPTIONS).arg(target);
    command
}

fn run(command: &mut Command) -> Result<(), String> {
    let command_line = command_line(command);
    let status = command.status().map_err(|error| {
        format!(
            "could not start command\n  command: {command_line}\n  start error: {error}"
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command failed\n  command: {command_line}\n  status: {status}\n  stdout/stderr: inherited live; see output above"
        ))
    }
}

fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<(), String> {
    let command_line = command_line(command);
    let mut child = command.spawn().map_err(|error| {
        format!(
            "could not start command\n  command: {command_line}\n  start error: {error}"
        )
    })?;
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(format!(
                    "command failed\n  command: {command_line}\n  status: {status}\n  stdout/stderr: inherited live; see output above"
                ));
            }
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "command timed out\n  command: {command_line}\n  timeout: {}s\n  stdout/stderr: inherited live; see output above",
                    timeout.as_secs()
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "could not wait for command\n  command: {command_line}\n  wait error: {error}"
                ));
            }
        }
    }
}

fn command_line(command: &Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(display_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn display_word(value: &OsStr) -> String {
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

fn shell_quote(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}
