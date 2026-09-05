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
    ~/.config/momarchy/
    small hooks in ~/.config/hypr/
```

The source repository, Git history, Cargo registry and Rust compiler stay on the development machine. Lua 5.4 is vendored into the Momarchy executable; the target does not need a separate Lua installation.

The first deployment target is a 2009 x86-64 MacBook Pro. Release builds therefore use a generic x86-64 CPU baseline. Do not use `target-cpu=native` for deploy builds: a modern development CPU can generate AVX/AVX2 instructions that the Core 2 Duo cannot execute.

## Development requirements

- Linux/WSL2 or macOS development environment.
- Current stable Rust. Momarchy currently has a Rust 1.88 floor.
- `ssh` and `scp` for deployment.
- On macOS only, Linux cross-build support through Zig + cargo-zigbuild.

Cargo owns Rust dependencies. There is no Rust toolchain or source checkout on the Momarchy target.

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
cargo home
cargo deploy t@momarchy
cargo screenshot t@momarchy
cargo build --release
```

`cargo home` stages the checked-out `lua/init.lua` into an isolated config under `target/` and launches the interactive Home against that copy. Local UI development therefore uses repo Lua directly without touching `~/.config/momarchy`.

`Cargo.lock` is committed because Momarchy is an application.

## Lua configuration

Momarchy's Rust executable is the stable engine. Home structure, content and actions live in Lua so ordinary UI changes do not require rebuilding the binary.

The canonical default is `lua/init.lua` in this repository. It is compiled into the executable with `include_str!`. On the first `momarchy home` run, if no user configuration exists, Momarchy creates:

```text
~/.config/momarchy/init.lua
```

`$XDG_CONFIG_HOME/momarchy/init.lua` is used instead when `XDG_CONFIG_HOME` is set.

Normal runtime never overwrites an existing user `init.lua`. An explicit `cargo deploy`, however, is an administrator action and treats the checked-out repo Lua as authoritative for the target.

### Semantic UI authoring

The preferred authoring surface is the bundled `momarchy.ui` Lua module. It is preloaded by the executable, so there is no extra Lua runtime dependency:

```lua
local ui = require("momarchy.ui")

return ui.app {
  home = "home",

  theme = {
    layout = {
      columns = 1,
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
- `ui.open(target, live_message)` hands a URL to the host browser when live actions are enabled.
- `ui.run({ "program", "arg1", ... }, kind, live_message)` launches a normal host process when live actions are enabled.

On Omarchy/Linux, `ui.open` uses `omarchy-launch-browser`, so browser selection and detached launch behavior stay with Omarchy. Momarchy does not wait on the browser PID: browsers commonly reuse an existing process, so that PID is not a meaningful lifecycle signal. Home simply stays resident underneath; Hyprland owns window focus and tiling, and closing the browser naturally returns to the still-running Home. If the Omarchy helper is missing, Linux falls back to `xdg-open`. macOS live development uses the normal `open` command.

Stable explicit button IDs are intentional. They are not visible to the user, but they are the automation contract used by commands such as `select games`.

`momarchy.ui` is authoring sugar only. It expands the semantic elements into the same normalized version-1 tables that Rust already parses and validates. Existing verbose `version = 1` configurations therefore remain valid; no schema migration is required just to adopt the nicer authoring syntax.

### Theme

Presentation is global rather than attached to individual elements. This is deliberately closer to a tiny semantic theme than real CSS: no selectors, classes, cascading, specificity or per-button style tables.

Current theme tokens are:

```lua
theme = {
  layout = {
    columns = 1, -- 1..4
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

If `theme` or one of its fields is omitted, Momarchy uses the built-in defaults. Keyboard navigation follows the configured column count. The current renderer centers the menu and caps it at 64 terminal cells so the one-column game-menu layout stays readable both fullscreen and when Hyprland tiles Home beside another app. That cap is deliberately a renderer invariant for now rather than another theme setting.

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

## Fresh Omarchy target

The one manual prerequisite is ordinary SSH access. On a fresh Omarchy target, enable OpenSSH/UFW and copy the development machine's SSH key so this succeeds without a login password prompt:

```bash
ssh <user>@<target> true
```

For example, the current reference target uses `t@momarchy`.

After that, the canonical Momarchy install/update command is simply:

```bash
cargo deploy <user>@<target>
```

Do not use the name `cargo install` for this workflow. `cargo install` already has a standard Rust meaning: install a crate/binary on the local machine. Momarchy's operation is remote provisioning + deployment, so `cargo deploy` is the clearer project-local command.

The first deploy may ask for the target user's `sudo` password inside the SSH terminal if privileged target changes are actually needed. Later deploys are designed to detect already-correct state and normally remain noninteractive.

After first-time provisioning, reboot the target once and verify the real appliance path:

```text
boot -> SDDM autologin -> Omarchy session -> Momarchy Home
```

Tailscale remains optional external remote-access enrollment rather than a Momarchy runtime requirement. Once a target is enrolled, the same `cargo deploy` command works over Tailscale/MagicDNS; Momarchy does not grow a separate deployment protocol.

## Deployment

The repository uses the common Rust `xtask` pattern for project-local automation. `.cargo/config.toml` exposes:

```bash
cargo deploy <ssh-target>
```

For the current prototype machine:

```bash
cargo deploy t@momarchy
```

The task:

1. builds an x86-64 Linux `momarchy` release binary (native Cargo on Linux/WSL2, Zig cross-build on macOS);
2. creates the target Momarchy directories if needed;
3. uploads the checked-out `install.sh` to `~/.local/state/momarchy/install.sh`;
4. runs that provisioner through `ssh -t`, so first-time package/SDDM changes can ask for sudo interactively;
5. uploads the binary to `~/.local/bin/momarchy.new`;
6. uploads repo `lua/init.lua` and `lua/momarchy/ui.lua` as staged target config files;
7. atomically-ish renames the staged files into their normal target paths;
8. runs `momarchy status` and a headless `momarchy home --automation` startup as cheap health checks.

The provisioner is idempotent and currently codifies the appliance settings we proved manually:

- required boring Arch/Omarchy tools are installed only when missing;
- Home is connected to Omarchy's normal `~/.config/hypr/autostart.lua` through a tiny Momarchy-owned Lua snippet and launches with `--live-actions`;
- `Super+M` is connected through Omarchy's normal binding API and focuses/launches `org.momarchy.home`;
- Omarchy's own `omarchy toggle idle stay-awake` primitive disables the idle password-lock/screensaver path;
- SDDM autologin is configured with its normal drop-in mechanism for the SSH target user and `omarchy.desktop` session;
- on the reference `MacBookPro5,5`, the proven NumLock override is connected without replacing the rest of `input.lua`;
- if the Broadcom BCM4322 PCI ID is detected and b43 firmware is missing, the provisioner installs `b43-firmware` through Omarchy's normal `yay`/AUR path.

The user's normal Omarchy Hyprland files are not replaced wholesale. Momarchy writes its small managed snippets under `~/.config/momarchy/` and adds one `dofile(...)` hook to the corresponding user files. The installer also migrates the earlier inline Momarchy Home/autostart lines used during prototyping.

An explicit deploy treats the repo as source of truth and intentionally replaces the target's Momarchy-managed Lua files. Normal runtime still never rewrites an existing config by itself.

The binary rename is deliberately simple and same-filesystem. Replacing the executable does not kill an already running copy; service/TUI restart semantics will be added only when there is a real resident service that needs them.

If you want `cargo deploy momarchy` without the `user@` prefix, configure the SSH destination normally in `~/.ssh/config`. Momarchy should not grow its own SSH configuration system.

## Remote target screenshots

For a visual check of a running Wayland target without physically operating it:

```bash
cargo screenshot <ssh-target>
```

For the current MBP13:

```bash
cargo screenshot t@momarchy
```

Phase 1 deliberately captures the full Wayland output only. The task delegates capture to Omarchy's own `omarchy capture screenshot fullscreen save` command inside the graphical user's systemd environment, writes a temporary PNG under `~/.local/state/momarchy/`, copies it back over ordinary SCP to `target/screenshots/momarchy.png`, removes the remote temporary file, and opens the local image automatically on macOS.

If all active target displays are asleep through DPMS, the task temporarily wakes them with Omarchy's own `omarchy brightness display on`, performs the capture, then restores sleep with `omarchy brightness display off`. Capture and display-control units have short runtime limits so a broken Wayland/display path cannot hang the development command indefinitely.

`grim` remains an implementation detail of Omarchy's screenshot stack rather than something Momarchy calls directly. There is no screenshot server, resident agent or custom transport protocol.

## KISS rules

- KISS means minimum accidental complexity, not choosing a crude or inferior primitive.
- Prefer existing operations in this order: **Omarchy first, then ordinary Arch/Linux, then Momarchy-specific code only when the lower layers do not already solve the problem cleanly**. This matters especially for graphical/session operations where Omarchy already owns Hyprland/Wayland conventions.
- Prefer the best-known underlying primitive when it materially improves correctness, idle cost or robustness; for example, kernel inotify rather than periodic file polling.
- Developer/admin failures must be self-explaining: show what operation and command were attempted, preserve useful child stdout/stderr, report start/exit status, and include concrete recovery checks or fix suggestions when known. Do not replace useful diagnostics with a bare exit code, and do not dump unrelated environment/secrets just for verbosity.
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
- Prefer existing host launchers and normal processes for browser/apps; do not fake lifecycle from a browser PID when the browser/compositor already owns it.
- User-facing Momarchy UI is Finnish. CLI, source, logs, config and docs are English.
- Measure memory and latency on the 2 GB target before optimizing abstractions.

## Near-term code shape

`momarchy status` is the first non-UI command and should remain cheap enough to use as a deploy health check.

`momarchy home` is the full-screen TUI. Rust owns terminal mechanics, layout/rendering, input, validated state and launching host actions. Lua owns the fast-changing semantic Home document and global theme.

Expected next steps are to handle external GUI/terminal applications cleanly, verify live reload/recovery on the real target, expose a small `doctor` view from existing Linux tools, and only then decide whether a resident daemon is actually useful.
