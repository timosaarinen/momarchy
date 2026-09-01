# Momarchy

**Omarchy for people who don't want to use a computer.**

Momarchy is an experiment in turning an Omarchy Linux installation into a simple, resilient, appliance-like laptop for a non-technical user.

The target experience is deliberately boring:

- boot the laptop
- see a small set of obvious things to do
- click large, understandable controls
- avoid Linux terminology and configuration
- require essentially no routine maintenance from the user
- remain remotely recoverable by a trusted administrator

The first target machine is an old Lenovo IdeaPad Y500 (Core i7-3630QM, 8 GB RAM, dual-GPU-era hardware and a questionable 1 TB HDD), so graceful operation on imperfect old hardware is part of the project rather than an edge case.

## Principles

1. **Appliance, not desktop.** The user should not need to understand Omarchy, Hyprland, Arch, package managers, workspaces, terminals, or dotfiles.
2. **Public and reproducible.** This repository contains no personal credentials or secrets. A fresh Omarchy system should be transformable into Momarchy from public code and configuration.
3. **Local secrets stay local.** Wi-Fi credentials, browser sessions, Tailscale authentication and personal data are never committed.
4. **Big obvious actions beat flexibility.** A few useful choices are better than a general-purpose launcher.
5. **Failure should be understandable.** If networking, storage or another essential service fails, Momarchy should say what happened in plain language and offer one obvious recovery action where possible.
6. **Remote recovery is first-class.** The administrator should be able to diagnose and repair the machine without requiring the user to operate a terminal.
7. **Updates must not casually break the appliance.** Prefer stable, reversible changes and retain a recovery path.

## Initial direction

The intended stack is:

- Omarchy as the underlying Linux system
- Quickshell / Omarchy shell customization for the Momarchy home experience
- browser/web apps for most user-facing services
- Tailscale for remote administration
- a small set of scripts for installation, health checks and recovery

The exact UI is intentionally not designed yet. First milestone is proving the target hardware can boot and run a current Linux/Omarchy environment reliably.

## Repository layout

```text
momarchy/
├── README.md
├── install.sh
├── config/          # Momarchy-managed configuration
├── home/            # Home/start screen implementation
├── scripts/         # Setup, diagnostics and recovery helpers
└── docs/            # Design notes and hardware/recovery documentation
```

## Status

Very early sidequest. The current work is hardware archaeology and compatibility testing before we commit to an installation strategy.

## Security

This is a public repository. Do not commit passwords, tokens, private keys, cookies, personal data or machine-specific credentials.
