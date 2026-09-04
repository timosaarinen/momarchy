# Development

Momarchy is intentionally small. The deployment target is an appliance, not a development workstation.

## Current shape

```text
newer Linux / WSL2 / macOS development machine
    cargo build / test / deploy
                |
                | SSH (LAN or Tailscale)
                v
Momarchy target
    ~/.local/bin/momarchy
    ~/.config/momarchy/init.lua
```

The source repository, Git history, Cargo registry and Rust compiler stay on the development machine. Lua 5.4 is vendored into the Momarchy executable; the target does not need a separate Lua installation.

The first deployment target is a 2009 x86-64 MacBook Pro. Release builds therefore use a generic x86-64 CPU baseline. Do not use `target-cpu=native` for deploy builds: a modern development CPU can generate AVX/AVX2 instructions that the Core 2 Duo cannot execute.

## Development requirements

- Linux/WSL2 or macOS development environment.
- Current stable Rust. Momarchy currently has a Rust 1.88 floor.
- `ssh` and `scp` for deployment.
- On macOS only, Linux cross-build support through Zig + cargo-zigbuild.

Cargo owns Rust dependencies. There is no separate development dependency installer for the Momarchy target.

On macOS, install the cross-build tools once:

```bash
brew install zig
cargo install --locked cargo-zigbuild
rustup target add x86_64-unknown-linux-gnu
```

Linux/WSL2 builds use ordinary Cargo. macOS deploy builds use `cargo zigbuild` to produce the same x86-64 Linux executable without requiring Docker, a VM, a remote build host, or a compiler on the Momarchy laptop. The Zig build targets glibc 2.17 as a conservative minimum and still uses Momarchy's generic `x86-64` CPU baseline.

Useful first commands:

```bash
cargo check
cargo test
cargo run -- status
cargo run -- home
cargo build --release
```

`Cargo.lock` is committed because Momarchy is an application.

## Lua configuration

Momarchy's Rust executable is the stable engine. Home screens, labels, hints and actions are data in Lua so ordinary UI changes do not require rebuilding or redeploying the binary.

The canonical default is `lua/init.lua` in this repository. It is compiled into the executable with `include_str!`. On the first `momarchy home` run, if no user configuration exists, Momarchy creates:

```text
~/.config/momarchy/init.lua
```

`$XDG_CONFIG_HOME/momarchy/init.lua` is used instead when `XDG_CONFIG_HOME` is set.

The executable never overwrites an existing user `init.lua`. Once materialized, that file belongs to the machine administrator.

A minimal configuration has this shape:

```lua
return {
  version = 1,
  home = "home",

  screens = {
    home = {
      title = "MOMARCHY",
      subtitle = "Mitä haluat tehdä?",
      buttons = {
        {
          id = "internet",
          label = "INTERNET",
          hint = "Avaa selain",
          action = {
            open = "https://www.google.fi/",
            live_message = "Avataan internet.",
          },
        },
      },
    },
  },
}
```

Supported action forms are deliberately small:

- `action = { screen = "games" }` navigates to another configured screen.
- `action = { message = "..." }` changes the Home status message.
- `action = { open = "https://...", live_message = "..." }` launches the target with `xdg-open` when live actions are enabled.
- `action = { command = { "program", "arg1", "arg2" }, kind = "...", live_message = "..." }` launches a normal host process when live actions are enabled.

Each action must contain exactly one of `screen`, `message`, `open`, or `command`. Screen targets, button IDs and other structural invariants are validated before a configuration is accepted.

Lua's normal `require()` works for future modularization. Momarchy prepends these paths to `package.path`:

```text
~/.config/momarchy/?.lua
~/.config/momarchy/?/init.lua
```

Start with one `init.lua`; split it only when that becomes clearer.

### Reload behavior

Momarchy does not poll configuration files. On Linux, a blocking inotify watch observes the Momarchy config directory and wakes only when a `.lua` file is written, replaced or deleted. Editors that save through a temporary file plus rename are covered by the directory watch.

Each reload creates a fresh Lua VM, evaluates the complete configuration, converts it to owned Rust data and validates it. There is no persistent Lua module cache to invalidate.

A successful reload atomically replaces the active Rust `Config` and preserves the current screen/selection when possible. If the new Lua is malformed or invalid, the currently running configuration remains active and the error is shown in Home's status area.

If an existing user config is already broken at cold start, Momarchy loads the embedded known-good `init.lua` in memory and leaves the broken file untouched for administrator repair.

The live TUI has no periodic timer. Terminal input and inotify each block in a small helper thread and send events to the main UI loop. Idle Momarchy therefore does no configuration polling and has no timer-driven CPU or disk wakeups.

The automation interface remains deterministic and supports an explicit `reload` command instead of starting the live watcher.

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

1. builds an x86-64 Linux `momarchy` release binary (native Cargo on Linux/WSL2, Zig cross-build on macOS);
2. creates `~/.local/bin` on the target if needed;
3. uploads to `~/.local/bin/momarchy.new` with `scp`;
4. renames it to `~/.local/bin/momarchy` on the target;
5. runs `momarchy status` as a cheap health check.

The editable Lua configuration is not part of normal binary deployment. An existing `~/.config/momarchy/init.lua` survives executable updates. A new machine receives the embedded default automatically the first time Home starts.

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

A browser and Tailscale are useful but not hard runtime dependencies of the first binary. Lua itself is not a target dependency because Lua 5.4 is vendored into `momarchy`.

Remote SSH access itself is an administrator choice. On Omarchy we currently use OpenSSH plus its UFW firewall; the bootstrap script does not silently open firewall ports.

## KISS rules

- KISS means minimum accidental complexity, not choosing a crude or inferior primitive.
- Prefer the best-known underlying primitive when it materially improves correctness, idle cost or robustness; for example, kernel inotify rather than periodic file polling.
- One `momarchy` binary until there is a concrete reason for more.
- Ratatui + Crossterm for the home UI.
- Lua is the editable configuration layer; do not invent a second config language or plugin framework without a concrete need.
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

`momarchy home` is the full-screen TUI. Rust owns terminal mechanics, rendering, input, validated state and launching host actions. Lua owns the fast-changing Home content and navigation model.

Expected next steps are to fill in the real actions, return to Home cleanly after launched applications, expose a small `doctor` view from existing Linux tools, and only then decide whether a resident daemon is actually useful.
