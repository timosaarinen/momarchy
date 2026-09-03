# Development

Momarchy is intentionally small. The deployment target is an appliance, not a development workstation.

## Current shape

```text
newer Linux / WSL2 development machine
    cargo build / test / deploy
                |
                | SSH (LAN or Tailscale)
                v
Momarchy target
    ~/.local/bin/momarchy
```

The source repository, Git history, Cargo registry and Rust compiler stay on the development machine.

The first deployment target is a 2009 x86-64 MacBook Pro. Release builds therefore use a generic x86-64 CPU baseline. Do not use `target-cpu=native` for deploy builds: a modern development CPU can generate AVX/AVX2 instructions that the Core 2 Duo cannot execute.

## Development requirements

- Linux development environment; WSL2 is fine.
- Current stable Rust. The current Ratatui version requires Rust 1.88+.
- `ssh` and `scp` for deployment.

Cargo owns Rust dependencies. There is no separate development dependency installer.

Useful first commands:

```bash
cargo check
cargo test
cargo run -- status
cargo run -- home
cargo build --release
```

The first build will generate `Cargo.lock`; this is an application, so commit the lockfile once generated and checked.

## Deployment

The repository uses the common Rust `xtask` pattern for project-local automation. `.cargo/config.toml` exposes the deployment task as:

```bash
cargo deploy <ssh-target>
```

For the current prototype machine:

```bash
cargo deploy t@momarchy
```

The task:

1. builds `momarchy` in release mode;
2. creates `~/.local/bin` on the target if needed;
3. uploads to `~/.local/bin/momarchy.new` with `scp`;
4. renames it to `~/.local/bin/momarchy` on the target;
5. runs `momarchy status` as a cheap health check.

The rename is deliberately simple and same-filesystem. Replacing the executable does not kill an already running copy; service/TUI restart semantics will be added only when there is a real resident service that needs them.

If you want `cargo deploy momarchy` without the `user@` prefix, configure the SSH destination normally in `~/.ssh/config`. Momarchy should not grow its own SSH configuration system.

Tailscale does not change the deployment model. Once the target is reachable through Tailscale/MagicDNS, the same SSH command is used.

## Preparing an Omarchy target

The target bootstrap script is `install.sh`. It is safe to run more than once and only installs missing runtime tools currently used by Momarchy.

Because the repo does not need to be cloned on the target, it can be run directly from the public repository:

```bash
curl -fsSL https://raw.githubusercontent.com/timosaarinen/momarchy/main/install.sh | bash
```

For development targets where you prefer not to pipe remote content into a shell, download/inspect the file first or copy it over SSH and run it locally.

The current runtime/tool assumptions are intentionally boring:

- systemd
- a terminal emulator (`foot` on Omarchy)
- NetworkManager / `nmcli`
- `lm_sensors` / `sensors`
- OpenSSH
- `xdg-open`

A browser and Tailscale are useful but not hard runtime dependencies of the first binary.

Remote SSH access itself is an administrator choice. On Omarchy we currently use OpenSSH plus its UFW firewall; the bootstrap script does not silently open firewall ports.

## KISS rules

- One `momarchy` binary until there is a concrete reason for more.
- Ratatui + Crossterm for the home UI.
- No Electron.
- No browser engine embedded just to render the home screen.
- No async runtime until we have an async problem.
- No database until we have data that needs a database.
- No custom remote-update protocol; SSH already exists.
- No custom monitoring stack; use `/proc`, `/sys`, `systemctl`, `nmcli`, `sensors`, `journalctl`, etc.
- Prefer normal Linux processes for launching browser/apps.
- User-facing Momarchy UI is Finnish. CLI, source, logs, config and docs are English.
- Measure memory and latency on the 2 GB target before optimizing abstractions.

## Near-term code shape

`momarchy status` is the first non-UI command and should remain cheap enough to use as a deploy health check.

`momarchy home` is the full-screen TUI. The initial scaffold only proves rendering, keyboard selection and mouse clicks. Actions are placeholders on purpose.

Expected next steps are to launch the browser/URLs, return to Home cleanly, expose a small `doctor` view from existing Linux tools, and only then decide whether a small resident daemon is actually useful.
