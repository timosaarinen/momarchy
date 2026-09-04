<p align="center">
  <img src="docs/images/momarchy-logo.png" alt="Momarchy" width="100%">
</p>

Momarchy is a small open-source side project: make an old laptop simple enough that my mom can just use it.

**Very early development phase (4 days in).** The current Momarchy Home is an experiment, not a polished distro or installer.

## I don't want your crappy in-progress auto-load Momarchy Home, just tell me how to get standard Omarchy on my 2009 MacBook

Plain [Omarchy](https://omarchy.org/) 4.0.2 works surprisingly well on the current reference machine, a **13-inch MacBook Pro from 2009** (Core 2 Duo P7350, 2 GB RAM, GeForce 9400M, Broadcom BCM4322 Wi-Fi). Props to DHH & Omarchy contributors.

These are the short version of the steps that worked here:

1. **Back up anything you care about.** I kept the old OS X El Capitan install as archaeology/fallback instead of wiping the disk.
2. **Make some free space from macOS.** I created about 50 GB with Disk Utility, then deleted that temporary HFS+ partition in the Omarchy installer's partition tool so the space was truly unallocated. If you do not care about keeping macOS, the whole-disk install is obviously simpler.
3. **Flash the Omarchy ISO directly to a USB stick.** Ventoy 1.1.17 froze on this old Apple EFI after selecting `EFI Boot`; a directly flashed Omarchy 4.0.2 ISO booted fine.
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

Current implementation direction: **Rust + Ratatui TUI**, built on a newer computer and deployed as a small binary over SSH/Tailscale. The source repo and Rust toolchain do not need to exist on the Momarchy laptop.

## Try the current scaffold

Requires a current stable Rust toolchain. Ratatui 0.30 currently requires Rust 1.88+.

```bash
git clone https://github.com/timosaarinen/momarchy.git
cd momarchy
cargo run -- status
cargo run -- home
```

`home` now has the first real Finnish Momarchy Home screen with mouse/keyboard navigation, `Pelit` and `Apua` subviews, and safe dry-run host actions. Development runs do not launch external programs unless explicitly started with `--live-actions`.

Remote deployment is project-local Cargo automation:

```bash
cargo deploy t@momarchy
```

That builds a release binary, copies it over SSH to `~/.local/bin/momarchy`, atomically replaces the previous binary and runs `momarchy status` remotely. No Git checkout or Rust compiler is needed on the target.

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the current KISS development/deployment rules and [docs/HARDWARE.md](docs/HARDWARE.md) for hardware archaeology.

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

## TODO

- [ ] Deploy the Lua-configured Home build to the real 2009 MacBook and verify first-run materialization, live inotify reload, bad-edit recovery and normal boot -> Home behavior on the actual target.
- [ ] Define an explicit config update/reset/migration path before Momarchy's default Lua or config schema starts changing; existing admin config must remain authoritative and must never be silently replaced by a binary update.
- [ ] Investigate MBP13 Wi-Fi reliability across different WLANs: BCM4322 + `b43` showed severe latency/packet loss on one crowded 2.4 GHz network, repeated `4WAY_HANDSHAKE_TIMEOUT` and `b43-phy0 ERROR: MAC suspend failed`; compare another AP/hotspot and 5 GHz before changing drivers.
- [ ] Make the Rust/Ratatui Momarchy Home actually mom-ready; tune layout, text size, focus, wording and real actions on the 13-inch target.
- [ ] Make TUI terminal cleanup bulletproof on normal exit, errors, signals and panics; never leave raw mode / mouse tracking / alternate screen behind.
- [ ] Launch Chrome/URLs from Home and return cleanly to Home when the task is done.
- [ ] Launch external GUI/terminal apps as plain child processes; suspend/restore the Momarchy terminal around terminal apps and use a shell only when shell semantics are actually needed.
- [ ] Automation should support all useful stable semantic commands plus human-equivalent input, including optional `click x y` for hitbox testing.
- [ ] Add an optional automation `render` command that dumps the whole virtual Ratatui frame from an in-memory backend when semantic state/actions are not enough.
- [ ] Grow `momarchy status` / `momarchy doctor` from real Linux tools and `/sys`, not a parallel monitoring stack.
- [ ] Add safe live update/restart behavior once there is actually a resident Momarchy service to restart.
- [ ] `Kysy mitä vain`: tiny UI routed to a service on ASUS/Tailscale; no technical backend jargon in mom UI.
- [ ] Rebuild/evaluate Voxtype for the Core 2 Duo; speech input could be genuinely useful here.
- [ ] `Katso televisiosta`: make the found Chromecast useful from Momarchy; test YouTube, Yle Areena and other useful Finnish streams.
- [ ] `Pelit`: evaluate existing open-source terminal games first; integrate/fork only mom-worthy ones. Palikat + Mato are the first targets.
- [ ] Test audio, suspend/resume, browser video and long-running stability on the MacBook.
- [ ] Keep Q4OS Trinity / other lean GUI Linux as fallback if Omarchy eventually becomes too much for 2 GB.

## Guidelines

Keep normal 2 GB operation out of swap; measure first, optimize only what matters.

If Linux already has a good tool for something, use it. Momarchy code is for the Momarchy-specific parts.

KISS.
