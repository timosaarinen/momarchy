<p align="center">
  <img src="docs/images/momarchy-logo.png" alt="Momarchy" width="100%">
</p>

Momarchy is a small open-source side project: make an old laptop simple enough that my mom can just use it.

**Very early development phase (5 days in).** The current Momarchy Home is an experiment, not a polished distro or installer.

## I don't want your crappy in-progress auto-load Momarchy Home, just tell me how to get standard Omarchy on my 2009 MacBook

Plain [Omarchy](https://omarchy.org/) 4.0.2 works surprisingly well on the current reference machine, a **13-inch MacBook Pro from 2009** (Core 2 Duo P7350, 2 GB RAM, GeForce 9400M, Broadcom BCM4322 Wi-Fi). Props to DHH & Omarchy contributors.

These are the short version of the steps that worked here:

1. **Back up anything you care about.** I kept the old OS X El Capitan install as archaeology/fallback instead of wiping the disk.
2. **Make some free space from macOS.** I created about 50 GB with Disk Utility, then deleted that temporary HFS+ partition in the Omarchy installer's partition tool so the space was truly unallocated. If you do not care about keeping macOS, the whole-disk install is obviously simpler.
3. **Flash the Omarchy ISO directly to a USB stick.** Ventoy 1.1.17 froze on old Apple EFI after selecting `EFI Boot`; a directly flashed Omarchy 4.0.2 ISO booted fine.
4. **Boot the USB and install Omarchy into the unallocated space.** On this machine the install took 12m 56s, preserved El Capitan, and after installation a normal reboot went straight into Omarchy without needing Option-key boot selection.
5. **Fix the built-in Broadcom Wi-Fi.** The kernel already detects the BCM4322 with `b43`, but the firmware is missing. Use Ethernet for the first boot and install:

   ```bash
   yay -S b43-firmware
   reboot
   ```

   After reboot, `wlan0` appears and NetworkManager can use the built-in Wi-Fi normally. There has been no reason to switch to proprietary `broadcom-wl`.
6. **Fix the old Apple Num Lock quirk if needed.** If `j` becomes `1`, `k` becomes `2`, etc., plain `F6` toggles Num Lock off. To make it permanent for this machine, put this in `~/.config/hypr/input.lua`:

   ```lua
   hl.config({ input = { numlock_by_default = false } })
   ```

That's basically it. Omarchy itself boots and runs on 2 GB RAM; that is tight, but Chrome and normal desktop use have been surprisingly usable so far.

### Optional: SSH for remote admin

If this is going to be a family computer, remote maintenance is handy. Enable SSH and let it through UFW:

```bash
sudo systemctl enable --now sshd
sudo ufw allow ssh
```

Then, from your admin machine, copy your SSH key and connect:

```bash
ssh-copy-id <user>@<macbook-hostname-or-ip>
ssh <user>@<macbook-hostname-or-ip>
```

For Momarchy development, this passwordless SSH step is the one manual prerequisite before the repo's `cargo provision <user>@<target>` command can turn a fresh Omarchy machine into the appliance.

For remote administration outside the local network, Tailscale also works cleanly on this hardware:

```bash
sudo pacman -S tailscale
sudo systemctl enable --now tailscaled
sudo tailscale up --accept-routes
```

After joining the machine to your tailnet, normal SSH works over its Tailscale address without public port forwarding.

More hardware details and archaeology live in [docs/HARDWARE.md](docs/HARDWARE.md).

## Status

The current idea is to use [Omarchy](https://omarchy.org/) as the Linux base, then hide almost all of the computer-y parts behind **Momarchy Home**: a tiny full-screen UI with a few obvious things to do. User-facing text is Finnish; code, CLI, docs and configuration stay English.

The first real deployment target is a **13-inch MacBook Pro from 2009** with a Core 2 Duo, 2 GB RAM and GeForce 9400M. Omarchy 4.0.2 installed and booted on it just fine. That makes low memory, old CPUs and boring reliability real design constraints instead of hypothetical ones.

Current implementation direction: **Rust + Ratatui TUI** as the small stable appliance engine, with Home structure/content and one global theme authored in Lua. Builds happen on a newer computer and deploy as one small binary over SSH/Tailscale; the source repo and Rust toolchain do not need to exist on the Momarchy laptop.

## Try the current scaffold

Requires a current stable Rust toolchain. Ratatui 0.30 currently requires Rust 1.88+.

```bash
git clone https://github.com/timosaarinen/momarchy.git
cd momarchy
cargo run -- status
cargo home
```

`cargo home` is the normal development UI loop: it stages the current repo `lua/init.lua` under `target/` and launches Momarchy Home against that isolated config, so development always uses the checked-out Lua without touching `~/.config/momarchy`. Host actions stay dry-run unless explicitly enabled.

For a fresh Omarchy deployment target, manually get key-based SSH working first. Then explicitly provision the appliance once:

```bash
cargo provision <user>@<target>
```

For the current reference machine:

```bash
cargo provision t@momarchy
```

Provisioning is the operation allowed to make system/session changes and may ask for the target user's sudo password when privileged package or SDDM work is actually needed. The repo `install.sh` codifies the settings we proved manually: required tools, Omarchy Home autostart with live actions, `Super+M`, persistent stay-awake/no idle password lock, SDDM autologin, and the reference MacBook's hardware-specific NumLock/Broadcom fixes when that hardware is detected.

Provisioning is intentionally **fail-closed and non-destructive**. It only makes exact/owned changes, validates known state before mutation, backs up touched Hyprland files, validates the resulting config, rolls back introduced errors, and stops with an informative manual repair instruction when state is ambiguous. See [docs/PROVISIONING.md](docs/PROVISIONING.md) for the normative policy. After provisioning succeeds, the same command deploys the current Momarchy binary + repo Lua and runs the normal health checks.

Normal development updates are then deliberately narrower:

```bash
cargo deploy <user>@<target>
```

`cargo deploy` never re-runs privileged/system provisioning. It first compares the current repo `install.sh` with the exact provisioner snapshot last applied successfully on the target. If provisioning has never been applied or the script has changed, deploy refuses with a concrete `cargo provision ...` instruction instead of guessing or double-applying system changes. Once provisioning is current, deploy only builds/uploads the binary + repo Lua and runs `momarchy status` plus a headless Home startup.

We deliberately do **not** call either operation `cargo install`: Cargo already uses that command to install Rust crates/binaries on the local machine. Momarchy's operations are remote appliance provisioning and deployment.

For a visual check of the real Wayland target without touching its keyboard:

```bash
cargo screenshot t@momarchy
```

That delegates the graphical operation to Omarchy itself (`omarchy capture screenshot fullscreen save`), copies the full-screen PNG back over ordinary SSH/SCP as `target/screenshots/momarchy.png`, removes the temporary target copy, and opens the image automatically on macOS. If the target panel is asleep, the command detects that through Hyprland, temporarily wakes it with `omarchy brightness display on`, captures, then restores the display to sleep with `omarchy brightness display off`. The whole asleep-display path is proven on the real MBP13. No screenshot server or custom graphical protocol is involved.

A plain SSH shell does not inherit the running Omarchy/Hyprland environment. For ad-hoc admin/debug commands that need the real graphical session, use the generic session helper instead of reconstructing Wayland environment variables by hand:

```bash
cargo session t@momarchy hyprctl configerrors
cargo session t@momarchy hyprctl monitors
```

See [docs/SESSION.md](docs/SESSION.md) for the intentionally small contract: one generic graphical-session primitive, dedicated commands only when they add real workflow or safety semantics.

### Configure Home in Lua

The preferred config is intentionally closer to tiny semantic HTML than a UI framework. Screens contain a few semantic elements; styling lives once in the global theme instead of on every button. The current default uses one vertical menu column; the renderer caps the menu at 64 terminal cells, gives every button the natural text-mode height of border + label + hint (four rows total, no fake blank-line padding), centers the resulting menu when there is spare room, and only compresses vertically when the terminal genuinely cannot fit the natural layout.

```lua
local ui = require("momarchy.ui")

return ui.app {
  home = "home",

  theme = {
    layout = { columns = 1, gap = 1, margin = 1 },
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
  },
}
```

The vocabulary is deliberately tiny: `ui.title`, `ui.subtitle`, `ui.text`, `ui.button`, plus `ui.go`, `ui.message`, `ui.open` and `ui.run` actions. There are no per-element style tables, selectors, classes or CSS-like cascading. Stable explicit button IDs stay because the SSH automation interface uses them (`select games`, `activate`, etc.). Existing verbose version-1 Lua configs remain supported; the DSL expands to the same validated Rust config underneath.

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the current KISS development/deployment rules, [docs/PROVISIONING.md](docs/PROVISIONING.md) for the fail-closed provisioning policy, [docs/SESSION.md](docs/SESSION.md) for remote graphical-session commands, and [docs/HARDWARE.md](docs/HARDWARE.md) for hardware archaeology.

## Dev diary

### 2026-09-01 — Day 1

- Had been thinking about installing Omarchy for my own use on an old Lenovo IdeaPad Y500 gaming laptop; dug it out
- Y500 still booted from HDD, although slowly - made suspicious noises, smelled a bit burny on the first start, but booted old Windows after some traditional percussive maintenance
- Got the sidequest idea: what if I give a pre-installed super-simple Omarchy laptop as a gift to my mom? She probably complains about not wanting any new machines at her age, so need to keep this very simple for her
- First candidate was an ancient HP; no charger handy, no-go
- Next candidate: Y500 itself
- Started rescuing some old Windows files before touching the disk
- Made a Ventoy USB for Mint / Omarchy / rescue stuff instead of reflashing one ISO at a time
- Created this public `momarchy` repo. Public/reproducible is useful anyway; secrets stay on the actual machine.

### 2026-09-02 — Day 2

- Found out that I also have a much nicer Momarchy candidate lying around: old 13-inch MacBook Pro from 2009
- Cleaned enough old macOS cruft to get ~64 GB free and carved out ~50 GB for Linux while keeping El Capitan as archaeology/fallback
- Ventoy 1.1.17 froze in old Apple EFI, so went simpler and flashed the Omarchy 4.0.2 ISO directly
- Omarchy USB booted. Okay, this became interesting.

### 2026-09-03 — Day 3

- Omarchy 4.0.2 installed on the 2009 MacBook in 12m 56s and preserved the old macOS install. Not bad for ancient hardware
- It booted straight into Omarchy without needing Option/EFI boot babysitting. Props to DHH + Omarchy team.
- Hardware: Core 2 Duo P7350 2.0 GHz, 2 GB RAM, NVIDIA C79/GeForce 9400M on `nouveau`, Broadcom BCM4322 Wi-Fi
- Good ol' Apple hardware: sensors, fan control, battery, keyboard/trackpad and graphics all basically just work; battery still reports ~95% of design capacity (!)
- Hit the Apple/Linux Num Lock quirk again after reboot: the right-hand letter keys came up as an emulated numeric keypad (`j` → `1`, `k` → `2`, etc.); on this MacBook, plain `F6` toggles Num Lock off
- Fixed Num Lock permanently for this target: Omarchy intentionally sets Hyprland `numlock_by_default = true`, which triggers Linux `hid_apple`'s embedded-keypad emulation on this old MacBook. Added a user override in `~/.config/hypr/input.lua` with `hl.config({ input = { numlock_by_default = false } })`; after restart the keyboard comes up normally with no F6 workaround
- Fixed the BCM4322 Wi-Fi cleanly: `b43` already detected the card but was missing `ucode16_mimo.fw`; installing `b43-firmware` made `wlan0` appear and scanning/connect work. Cold reboot with Ethernet unplugged auto-connected over Wi-Fi, and SSH plus IPv4/IPv6 Internet access worked normally
- Omarchy itself leaves roughly 1GB out of 2GB memory available after boot. Chrome homepage ~800 MB available; scrolling a real `is.fi` page stayed around ~500 MB available. Tight, but surprisingly usable
- Chrome Memory Saver on; normal operation should avoid swap even though swap stays available as the emergency cushion
- Voxtype looked promising for mom-friendly speech input, then immediately died on an AVX2 illegal instruction. Old CPU build later.
- Enabled SSH through Omarchy/UFW; `ssh t@momarchy` works from the desktop, so the MacBook is now a deployment target instead of a development machine
- Confirmed lid-close can be temporarily inhibited while SSH stays alive; Apple SMC sensors work nicely from Linux
- Considered HTML, Tauri and terminal UI for Momarchy Home. Naturally ended at: rewrite it in Rust and make it a TUI
- Decided the source repo/compiler stay on newer machines; Momarchy targets get tiny deployed artifacts over SSH/Tailscale
- Bootstrapped the real Rust project: Ratatui/Crossterm Home skeleton, `momarchy status`, `cargo deploy` over SSH and a tiny target bootstrap script
- Found the Chromecast too, a 2021/01-era unit. Momarchy bundle is now shaping up as MacBook + Chromecast, both preconfigured before handoff
- Added a low-priority `Pelit` idea: tiny TUI games, starting with an original falling-block game and maybe Snake
- First proper Home pass: eight big Finnish actions, mouse/keyboard navigation, `Pelit` with Palikat/Mato placeholders, `Apua`, dry-run host actions and a tiny stdin/stdout automation mode.

### 2026-09-04 — Day 4

- Started turning the tested TUI into the actual appliance shell: first target was making Momarchy Home appear automatically with the Omarchy/Hyprland session instead of inventing a Momarchy daemon
- Confirmed Omarchy 4.0.2 has the intended user-owned `~/.config/hypr/autostart.lua` hook and `o.launch_on_start(...)` helper, so Momarchy can stay inside the normal Omarchy startup model
- First manual launch from SSH failed with `failed to connect to wayland; no compositor running?`; useful reminder that an SSH shell does not inherit the graphical session's `WAYLAND_DISPLAY` / `HYPRLAND_INSTANCE_SIGNATURE`
- Confirmed Omarchy had imported the graphical environment into the user systemd manager (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`, Hyprland instance signature present)
- Proved remote GUI launching cleanly with `systemd-run --user`: a Foot window running `~/.local/bin/momarchy home` appeared on the real MacBook from the SSH session
- Added Momarchy Home to `~/.config/hypr/autostart.lua` using a dedicated Foot app id, `org.momarchy.home`
- Rebooted for the real test. Success: Momarchy Home opens fullscreen immediately, even before the top bar appears. `ESC` still exits to ordinary Omarchy as the intentional maintenance escape hatch
- Added a permanent `Super+M` (`Command+M` on the MacBook keyboard) binding in `~/.config/hypr/bindings.lua` for “take me Home”. It uses Omarchy's launch-or-focus behavior, so it focuses an existing Momarchy Home window or launches one if Home has been exited
- Confirmed `Super+M` works on the real 2009 MacBook. Basic appliance loop is now real: boot -> Home; `ESC` -> Omarchy; `Super+M` -> Home
- Remote-maintenance footnote: plain `hyprctl` from SSH has the same missing-session-environment issue; running it through the graphical user's systemd environment (for example `systemd-run --user --collect --wait hyprctl reload`) works
- Created a separate maintenance account, `tims`, with its own home directory and `wheel` membership. On this install `wheel` alone only inherited Omarchy's narrow command-specific sudo rules, not general administrator access, so added an explicit password-required `tims ALL=(ALL:ALL) ALL` drop-in under `/etc/sudoers.d/` and verified it with `visudo`
- Added the development-machine SSH key to `tims` with `ssh-copy-id`; fresh `ssh tims@momarchy` is now passwordless while `sudo` still requires the maintenance password. The original `t` account is intentionally left unchanged for now; there is no need to rush privilege cleanup while the appliance flow is still evolving
- Added minimal device-level Tailscale instead of Omarchy's optional desktop extras: installed the `tailscale` package, enabled `tailscaled`, joined the existing tailnet with `tailscale up --accept-routes`, and skipped operator grants, Taildrop receiver, bar widget and admin-console webapp because Momarchy does not need them
- Confirmed ordinary users can read `tailscale status` without extra privileges, while configuration changes can simply keep using `sudo` when needed
- Final remote-support proof: connected to `tims` over the MacBook's Tailscale IP from another tailnet machine using the existing SSH key. No public port forwarding needed. Remote maintenance outside mom's LAN is now working.
- Moved the fast-changing Momarchy Home content/configuration out of compile-time Rust and into Lua while keeping Rust + Ratatui as the small stable appliance engine
- Vendored Lua 5.4 into the executable, so deployment is still one small binary with no separate Lua runtime to install; current release build is about 993 KiB
- Added embedded `lua/init.lua` as the known-good default. First Home run atomically materializes it as `~/.config/momarchy/init.lua`; after that the file belongs to the machine administrator and Momarchy never silently overwrites it
- Added event-driven Linux inotify hot reload for `.lua` config files — no timer polling. Every reload gets a fresh Lua VM, so normal `require()` modules can be edited and reloaded without stale module state
- Bad live Lua edits keep the last valid UI running; a bad config on cold start falls back to the embedded known-good config in memory without destroying the broken admin file
- Added binary/inotify/config tests and checked the whole thing against the Rust 1.88 floor: 10 tests, strict Clippy and release build green; `ldd` confirms the executable has no system Lua dependency

### 2026-09-05 — Day 5

- New WLAN, new archaeology: Tailscale correctly found a direct LAN path between the two MacBooks, but raw LAN ping to the 2009 machine still showed huge latency spikes and initially ~42% packet loss. Kernel logs showed repeated BCM4322/`b43` handshake and `MAC suspend failed` errors. Parked deeper investigation in TODO instead of letting the Wi-Fi sidequest eat the night
- Removed the artificial Linux-only restriction from `cargo deploy`; MBP16/macOS is now a supported Momarchy development and deployment host instead of requiring a WSL/Linux detour
- Kept Linux-specific behavior genuinely Linux-specific: inotify hot reload is compiled on Linux targets, while host-side macOS builds simply omit that watcher. `cargo check` now passes cleanly on MBP16 without inventing a fake macOS file-watching abstraction
- Added macOS -> Linux release cross-builds with Zig + `cargo-zigbuild`, targeting `x86_64-unknown-linux-gnu.2.17` while retaining the generic x86-64 CPU baseline needed by the Core 2 Duo
- Shook out the small portability bugs during the real run: removed a non-Linux dead-code warning and fixed a bogus `cargo zigbuild --version` capability probe that the tool does not support
- End-to-end path is now proven from MBP16: `cargo check` is clean and `cargo deploy t@momarchy` successfully cross-builds the Linux release, uploads it over SSH, atomically replaces `~/.local/bin/momarchy` and passes the remote `momarchy status` health check
- Proved the semantic automation interface remotely over ordinary SSH: `momarchy home --automation` exposes stable screen/button IDs, selection, actions and status, and can navigate the real deployed Home without touching the MBP13 keyboard
- Replaced verbose hand-authored screen tables with a bundled semantic Lua vocabulary: `ui.title`, `ui.subtitle`, `ui.text`, `ui.button` and tiny action helpers. The helper expands to the existing normalized v1 config instead of introducing a DOM or second runtime model
- Added a single global Lua theme for layout, colors and borders. Ratatui now follows theme columns/gap/margin and configured colors/border style; keyboard navigation follows the chosen column count. No per-button styling, selectors, classes or CSS cascade
- Kept backwards compatibility: existing normalized version-1 Lua configs still load, while new configs can `require("momarchy.ui")` from the binary with no extra runtime file to install
- Made the repo Lua files the explicit development/deployment source of truth: `cargo home` stages the checked-out `lua/init.lua` into an isolated dev config, while `cargo deploy` replaces the target binary and repo-managed Lua files and validates Home through the automation interface
- Made browser actions use Omarchy's own `omarchy-launch-browser` instead of pretending `xdg-open` owns the lifecycle. Home stays resident underneath while Omarchy launches/detaches the configured browser; Hyprland owns focus, so closing the browser naturally returns to Home. A missing Omarchy helper falls back to `xdg-open`, and macOS live development uses `open`
- Proved phase-1 remote visual testing end to end with `cargo screenshot <ssh-target>`. Capture delegates to Omarchy, the PNG comes back over plain SCP, and an asleep MBP13 panel is temporarily woken through Omarchy and returned to sleep after capture. The first real remote screenshot also usefully exposed the next appliance problem: the running Omarchy session was sitting at its password lock screen
- Turned the reference MacBook into an actual appliance session using Omarchy/SDDM rather than custom login plumbing: added an SDDM autologin drop-in for user `t` and the normal `omarchy.desktop` session, then enabled Omarchy's persistent stay-awake toggle so idle no longer lands on a password lock screen
- Rebooted and proved the full path remotely: boot -> SDDM autologin -> Omarchy/Hyprland -> Momarchy Home, with no password prompt. Updated Home autostart to run `momarchy home --live-actions`, so the appliance has real actions while local `cargo home` stays safely dry-run
- Proved real browser launch on the MBP13. A cold Chromium start can take roughly 20 seconds on the Core 2 Duo/2 GB machine, but once it appears Hyprland tiles it beside the still-running Home exactly like normal Omarchy. That native side-by-side behavior is now explicitly a feature, not something Momarchy should replace
- Switched the default Home layout from a two-column dashboard to a one-column game-menu style list. The renderer centers the menu and caps it at 64 terminal cells, so it stays comfortably narrow fullscreen while automatically using the available width when Hyprland tiles Home beside another app. Development-only banners/status now use English instead of Finnish
- Caught the original `install.sh` lagging behind the real appliance. Turned it into an idempotent provisioner that codifies the proven Omarchy/Hyprland/SDDM settings instead of relying on the dev diary and memory alone
- Reconsidered running that provisioner on every normal deploy: too much privilege and too much opportunity for a detection bug. Split the lifecycle into explicit `cargo provision` and fast `cargo deploy`; deploy only compares the current provisioner with the last successfully applied snapshot and refuses with a fix instruction if provisioning is missing/stale
- The first one-column local Home screenshot exposed a text-mode layout bug: stretched rows plus a leading blank line made buttons huge and eventually hid their contents. Buttons now use the natural four-row height (border + label + hint) with no fake vertical padding, centered as a group when space allows and compressed only when necessary
- The same pass caught a useful stale test: `embedded_config_is_valid` still expected two columns after the default moved to one. Updated the assertion rather than weakening the test
- Closing the MBP13 lid while starting a remote provision made the old SSH path look frozen. Remote provision/deploy/screenshot now use bounded passwordless SSH connection attempts plus keepalives; the short reachability probe has a hard wall-clock deadline, while the actual provision is deliberately not given a bogus global timeout that could kill legitimate `pacman`, AUR or sudo work on old hardware
- The first real `cargo provision` also caught a more serious mistake in our own migration logic: line-based heuristic deletion of an old multi-line `Super+M` Lua binding left `~/.config/hypr/bindings.lua` syntactically broken. Repaired it manually in NeoVIM and promoted the lesson into a normative rule instead of hiding the incident
- Added [docs/PROVISIONING.md](docs/PROVISIONING.md): provisioning is fail-closed and non-destructive, never heuristically rewrites arbitrary user/Omarchy Lua, backs up touched Hyprland files, validates existing and resulting config, rolls back introduced failures, and stops with precise manual instructions when state is ambiguous. `b43-firmware` detection now asks the package database instead of guessing from one firmware filename
- Re-ran the repaired path on the real target: all 12 Rust unit tests plus 4 Lua-config integration tests passed, `cargo provision t@momarchy` reported both `validating Hyprland Lua (existing)` and `(Momarchy)`, completed cleanly, deployed the current binary/Lua, and the machine rebooted back into the appliance session
- Added one generic `cargo session <target> <command> [args...]` escape hatch for commands that need the real Omarchy/Hyprland environment instead of adding a pile of tiny wrappers. Proved it remotely: `cargo session t@momarchy hyprctl configerrors` returned cleanly with no errors and `hyprctl monitors` reported the real Apple LVDS-1 panel at 1280×800. Dedicated commands remain only where they add workflow or safety semantics

## TODO

- [ ] Prove `cargo provision` from a genuinely fresh Omarchy 4.0.2 target (clean VM/machine/user) before treating the installer as handoff-ready; exercise both the normal first-run path and at least one fail-closed/rollback case rather than relying only on the already-hand-tuned MBP13.
- [ ] Verify live inotify config reload and bad-edit recovery on the actual MBP13.
- [ ] Launch external GUI/terminal apps as plain child processes; suspend/restore the Momarchy terminal around terminal apps and use a shell only when shell semantics are actually needed.
- [ ] Make TUI terminal cleanup bulletproof on normal exit, errors, signals and panics; never leave raw mode / mouse tracking / alternate screen behind.
- [ ] Make the Rust/Ratatui Momarchy Home actually mom-ready; tune layout, focus, wording and real actions, then do final sizing/geometry checks on the 13-inch target.
- [ ] **Experiment:** prototype Quickshell as an optional graphical Momarchy Home frontend: keep Rust/Lua as the semantic model and action engine plus Ratatui as the boring fallback, first try a fullscreen Quickshell `FloatingWindow` talking to `momarchy home --automation` over a tiny JSON-lines IPC, and only if that proves simple/reliable on the real MBP13 evaluate integrating it into Omarchy's existing Quickshell shell as a Home/overlay plugin instead of coupling Rust directly to Qt.
- [ ] Automation should support all useful stable semantic commands plus human-equivalent input, including optional `click x y` for hitbox testing.
- [ ] Add an optional automation `render` command that dumps the whole virtual Ratatui frame from an in-memory backend when semantic state/actions are not enough.
- [ ] `Kysy mitä vain`: tiny UI routed to a service on ASUS/Tailscale; no technical backend jargon in mom UI.
- [ ] Rebuild/evaluate Voxtype for the Core 2 Duo; speech input could be genuinely useful here.
- [ ] `Katso televisiosta`: make the found Chromecast useful from Momarchy; test YouTube, Yle Areena and other useful Finnish streams.
- [ ] `Pelit`: evaluate existing open-source terminal games first; integrate/fork only mom-worthy ones. Palikat + Mato are the first targets.
- [ ] Grow `momarchy status` / `momarchy doctor` from real Linux tools and `/sys`, not a parallel monitoring stack.
- [ ] Add a calm colored ASCII-art background through the global theme, likely a Finnish lake/forest scene framing the usable center; keep artwork separate from screen structure and avoid per-screen styling.
- [ ] Add a hidden developer/admin control for temporary appliance policy overrides such as inhibiting lid-close sleep during remote work; keep it behind a developer hotkey/screen and out of the normal mom-facing menu.
- [ ] Test audio, suspend/resume, browser video and long-running stability on the MacBook.
- [ ] Investigate MBP13 Wi-Fi reliability across different WLANs: BCM4322 + `b43` showed severe latency/packet loss on one crowded 2.4 GHz network, repeated `4WAY_HANDSHAKE_TIMEOUT` and `b43-phy0 ERROR: MAC suspend failed`; compare another AP/hotspot and 5 GHz before changing drivers.
- [ ] Add safe live update/restart behavior once there is actually a resident Momarchy service to restart.
- [ ] Keep Q4OS Trinity / other lean GUI Linux as fallback if Omarchy eventually becomes too much for 2 GB.

## Guidelines

Keep normal 2 GB operation out of swap; measure first, optimize only what matters.

For host/desktop operations, prefer an Omarchy-provided primitive first, then an ordinary Arch/Linux primitive, and only write custom Momarchy machinery when neither already fits — especially around Hyprland/Wayland.

Developer/admin command failures should say what was attempted, preserve useful underlying output, report the failing status, and suggest a concrete recovery step when one is known.

`cargo provision` is fail-closed and non-destructive by policy: exact/owned edits only, validate before/after when possible, back up non-Momarchy files before touching them, roll back introduced errors, and stop for manual inspection when state is ambiguous. See [docs/PROVISIONING.md](docs/PROVISIONING.md).

KISS.