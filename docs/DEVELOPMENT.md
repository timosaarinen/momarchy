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

Momarchy's Rust executable is the stable engine. Home structure, content and actions live in Lua so ordinary UI changes do not require rebuilding the binary.

The canonical default is `lua/init.lua` in this repository. It is compiled into the executable with `include_str!`. On the first `momarchy home` run, if no user configuration exists, Momarchy creates:

```text
~/.config/momarchy/init.lua
```

`$XDG_CONFIG_HOME/momarchy/init.lua` is used instead when `XDG_CONFIG_HOME` is set.

The executable never overwrites an existing user `init.lua`. Once materialized, that file belongs to the machine administrator.

### Semantic UI authoring

The preferred authoring surface is the bundled `momarchy.ui` Lua module. It is preloaded by the executable, so there is no extra Lua file to install on the target:

```lua
local ui = require("momarchy.ui")

return ui.app {
  home = "home",

  theme = {
    layout = {
      columns = 2,
      gap = 1,
      margin = 1,
    },
    colors = {
      background = "black",
      text = "white",
      muted = "gray",
      selected_background = "white",
      selected_text = "black",
    },
    border = "rounded",
  },

  screens = {
    home = ui.screen {
      ui.title "MOMARCHY",
      ui.subtitle "Mitä haluat tehdä?",

      ui.button(
        "internet",
        "INTERNET",
        "Avaa selain",
        ui.open("https://www.google.fi/", "Avataan internet.")
      ),

      ui.button("games", "PELIT", "Palikat, Mato...", ui.go "games"),
    },

    games = ui.screen {
      ui.title "PELIT",
      ui.subtitle "Valitse peli",
      ui.button("back", "TAKAISIN", "Palaa alkuun", ui.go "home"),
    },
  },
}
```

The deliberately small semantic element vocabulary is:

- `ui.title "..."`
- `ui.subtitle "..."`
- `ui.text "..."`
- `ui.button(id, label, hint, action)`

Actions are similarly small:

- `ui.go "games"` navigates to another configured screen.
- `ui.message "..."` changes the Home status message.
- `ui.open(target, live_message)` launches the target with `xdg-open` when live actions are enabled.
- `ui.run({ "program", "arg1", ... }, kind, live_message)` launches a normal host process when live actions are enabled.

Stable explicit button IDs are intentional. They are not visible to the user, but they are the automation contract used by commands such as `select games`.

`momarchy.ui` is authoring sugar only. It expands the semantic elements into the same normalized version-1 tables that Rust already parses and validates. Existing verbose `version = 1` configurations therefore remain valid; no schema migration is required just to adopt the nicer authoring syntax.

### Theme

Presentation is global rather than attached to individual elements. This is deliberately closer to a tiny semantic theme than real CSS: no selectors, classes, cascading, specificity or per-button style tables.

Current theme tokens are:

```lua
theme = {
  layout = {
    columns = 2, -- 1..4
    gap = 1,     -- 0..16 terminal cells
    margin = 1,  -- 0..16 terminal cells
  },

  colors = {
    background = "black",
    text = "white",
    muted = "gray",
    selected_background = "white",
    selected_text = "black",
  },

  border = "rounded",
}
```

Supported color names are `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `darkgray` and `white` (`grey` spellings are also accepted). Borders are `plain`, `rounded`, `double` or `thick`.

If `theme` or one of its fields is omitted, Momarchy uses the built-in defaults. Keyboard navigation follows the configured column count.

The Rust boundary stays strict: the Lua result is converted into owned Rust data, all actions and navigation targets are validated, and invalid config never reaches the renderer.

Lua's normal `require()` also works for administrator-created modules. Momarchy prepends these paths to `package.path`:

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
- Lua is the editable configuration layer; `momarchy.ui` is a tiny semantic authoring vocabulary, not a DOM or plugin framework.
- Styling stays in one global theme. Do not add per-element styles, selectors, classes or CSS-like cascading without a concrete need.
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

`momarchy home` is the full-screen TUI. Rust owns terminal mechanics, layout/rendering, input, validated state and launching host actions. Lua owns the fast-changing semantic Home document and global theme.

Expected next steps are to fill in the real actions, return to Home cleanly after launched applications, expose a small `doctor` view from existing Linux tools, and only then decide whether a resident daemon is actually useful.
