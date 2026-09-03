# Momarchy

A small side project: make an old laptop simple enough that my mom can just use it.

The current idea is to use [Omarchy](https://omarchy.org/) as the Linux base, then hide almost all of the computer-y parts behind **Momarchy Home**: a tiny full-screen UI with a few obvious things to do. User-facing text is Finnish; code, CLI, docs and configuration stay English.

The slightly ridiculous first real deployment target is a **13-inch MacBook Pro from 2009** with a Core 2 Duo, 2 GB RAM and GeForce 9400M. Omarchy 4.0.2 installed and booted on it just fine. That makes low memory, old CPUs and boring reliability real design constraints instead of hypothetical ones.

Current implementation direction: **Rust + Ratatui TUI**, built on a newer computer and deployed as a small binary over SSH/Tailscale. The source repo and Rust toolchain do not need to exist on the Momarchy laptop.

## Try the current scaffold

Requires a current stable Rust toolchain. Ratatui 0.30 currently requires Rust 1.88+.

```bash
git clone https://github.com/timosaarinen/momarchy.git
cd momarchy
cargo run -- status
cargo run -- home
```

`home` is intentionally only the first interactive TUI skeleton. The useful part now is proving the shape of the project and deployment workflow before adding features.

Remote deployment is project-local Cargo automation:

```bash
cargo deploy t@momarchy
```

That builds a release binary, copies it over SSH to `~/.local/bin/momarchy`, atomically replaces the previous binary and runs `momarchy status` remotely. No Git checkout or Rust compiler is needed on the target.

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the current KISS development/deployment rules and [docs/HARDWARE.md](docs/HARDWARE.md) for hardware archaeology.

## Dev diary

### 2026-09-01 — Day 1

- Had been thinking about installing Omarchy for my own use on an old gaming laptop; dug it out and it still booted, slowly :D
- Got the sidequest idea: what if I donate a pre-installed super-simple Omarchy laptop to my mom?
- First candidate was an ancient HP; no charger handy, so archaeology paused immediately.
- Next candidate: Lenovo IdeaPad Y500, my old dual-GPU gaming laptop. Much more power, much more gamer laptop.
- Y500 HDD made suspicious noises, smelled a bit hot/burny on first start, then booted old Windows after some traditional percussive maintenance :D
- Started rescuing old Windows archaeology before touching the disk.
- Made a Ventoy USB for Mint / Omarchy / rescue stuff instead of reflashing one ISO at a time.
- Created this public `momarchy` repo. Public/reproducible is useful anyway; secrets stay on the actual machine.

### 2026-09-02 — Day 2

- Remembered I also have a much nicer Momarchy candidate lying around: old 13-inch MacBook Pro from 2009.
- Cleaned enough old macOS cruft to get ~64 GB free and carved out ~50 GB for Linux while keeping El Capitan as archaeology/fallback.
- Ventoy 1.1.17 froze in old Apple EFI, so went simpler and flashed the Omarchy 4.0.2 ISO directly.
- Omarchy USB booted. Okay, this became interesting.

### 2026-09-03 — Day 3

- Omarchy 4.0.2 installed on the 2009 MacBook in 12m 56s and preserved the old macOS install. Not bad for ancient hardware.
- It booted straight into Omarchy without needing Option/EFI boot babysitting. Props to DHH + Omarchy team.
- Hardware: Core 2 Duo P7350 2.0 GHz, 2 GB RAM, NVIDIA C79/GeForce 9400M on `nouveau`, Broadcom BCM4322 Wi-Fi.
- Good ol' Apple hardware: sensors, fan control, battery, keyboard/trackpad and graphics all basically just work; battery still reports ~95% of design capacity (!).
- Broadcom Wi-Fi is still an open investigation; Ethernet works.
- Omarchy itself leaves roughly ~900 MB available after boot. Chrome homepage ~800 MB available; scrolling a real `is.fi` page stayed around ~500 MB available. Tight, but surprisingly usable.
- Chrome Memory Saver on; normal operation should avoid swap even though swap stays available as the emergency cushion.
- Voxtype looked promising for mom-friendly speech input, then immediately died on an AVX2 illegal instruction. Old CPU build later :D
- Enabled SSH through Omarchy/UFW; `ssh t@momarchy` works from the desktop, so the MacBook is now a deployment target instead of a development machine.
- Confirmed lid-close can be temporarily inhibited while SSH stays alive; Apple SMC sensors work nicely from Linux.
- Considered HTML, Tauri and terminal UI for Momarchy Home. Naturally ended at: rewrite it in Rust and make it a TUI.
- Decided the source repo/compiler stay on newer machines; Momarchy targets get tiny deployed artifacts over SSH/Tailscale.
- Bootstrapped the real Rust project: Ratatui/Crossterm Home skeleton, `momarchy status`, `cargo deploy` over SSH and a tiny target bootstrap script.

## TODO

- [x] Bootstrap Rust/Ratatui project and simple SSH `cargo deploy` workflow.
- [x] Add simple target bootstrap/setup script and document runtime assumptions.
- [ ] Make the Rust/Ratatui Momarchy Home actually useful; huge, obvious, mouse-first controls with Finnish UI text.
- [ ] Launch Chrome/URLs from Home and return cleanly to Home when the task is done.
- [ ] Keep normal 2 GB operation out of swap; measure first, optimize only what matters.
- [ ] Finish Broadcom BCM4322 Wi-Fi on the 2009 MacBook.
- [ ] Create separate admin/maintenance user; keep mom account boring, non-admin and eventually auto-login.
- [ ] Add Tailscale for remote maintenance and deployment outside the LAN.
- [ ] Grow `momarchy status` / `momarchy doctor` from real Linux tools and `/sys`, not a parallel monitoring stack.
- [ ] Add safe live update/restart behavior once there is actually a resident Momarchy service to restart.
- [ ] `Kysy mitä vain`: tiny UI routed to a service on ASUS/Tailscale; no technical backend jargon in mom UI.
- [ ] Rebuild/evaluate Voxtype for the Core 2 Duo; speech input could be genuinely useful here.
- [ ] `Katso televisiosta`: investigate Chromecast flow for YouTube, Yle Areena and other useful Finnish streams.
- [ ] Test audio, suspend/resume, browser video and long-running stability on the MacBook.
- [ ] Keep Q4OS Trinity / other lean GUI Linux as fallback if Omarchy eventually becomes too much for 2 GB.

## One rule

If Linux already has a good tool for something, use it. Momarchy code is for the Momarchy-specific parts.
