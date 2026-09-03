#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  printf '%s\n' 'Momarchy target bootstrap currently supports Linux only.' >&2
  exit 1
fi

missing_packages=()

need_command() {
  local command_name="$1"
  local package_name="$2"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    missing_packages+=("$package_name")
  fi
}

need_command sshd openssh
need_command foot foot
need_command nmcli networkmanager
need_command sensors lm_sensors
need_command xdg-open xdg-utils

if ((${#missing_packages[@]} > 0)); then
  if ! command -v pacman >/dev/null 2>&1; then
    printf 'Missing runtime packages: %s\n' "${missing_packages[*]}" >&2
    printf '%s\n' 'Automatic installation is currently implemented only for Arch/Omarchy targets.' >&2
    exit 1
  fi

  printf 'Installing missing runtime packages: %s\n' "${missing_packages[*]}"
  sudo pacman -S --needed "${missing_packages[@]}"
fi

mkdir -p \
  "$HOME/.local/bin" \
  "$HOME/.config/momarchy" \
  "$HOME/.local/state/momarchy"

printf '%s\n' 'Momarchy target bootstrap complete.'
printf '%s\n' 'Runtime directory: ~/.local/bin'
printf '%s\n' 'SSH/firewall and Tailscale access are intentionally left to the administrator.'
