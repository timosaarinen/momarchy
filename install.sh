#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  printf '%s\n' 'Momarchy target provisioning currently supports Linux only.' >&2
  exit 1
fi

if ! command -v omarchy >/dev/null 2>&1; then
  printf '%s\n' 'Momarchy target provisioning expects an existing Omarchy installation.' >&2
  printf '%s\n' 'Install Omarchy first, enable passwordless SSH, then run cargo provision <ssh-target> from the development machine.' >&2
  exit 1
fi

TARGET_USER="$(id -un)"
STATE_DIR="$HOME/.local/state/momarchy"
CONFIG_DIR="$HOME/.config/momarchy"
HYPR_DIR="$HOME/.config/hypr"

mkdir -p "$HOME/.local/bin" "$CONFIG_DIR" "$STATE_DIR" "$HYPR_DIR"

missing_packages=()

need_command() {
  local command_name="$1"
  local package_name="$2"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    missing_packages+=("$package_name")
  fi
}

# Keep target dependencies boring. Omarchy already provides most of these, but a
# fresh target should be repairable rather than failing later with a mystery
# command-not-found.
need_command foot foot
need_command nmcli networkmanager
need_command sensors lm_sensors
need_command xdg-open xdg-utils
need_command jq jq
need_command grim grim
need_command lspci pciutils

if ((${#missing_packages[@]} > 0)); then
  if ! command -v pacman >/dev/null 2>&1; then
    printf 'Missing target packages: %s\n' "${missing_packages[*]}" >&2
    printf '%s\n' 'Automatic package installation is implemented only for Arch/Omarchy targets.' >&2
    exit 1
  fi

  printf '==> installing missing target packages: %s\n' "${missing_packages[*]}"
  sudo pacman -S --needed --noconfirm "${missing_packages[@]}"
fi

write_if_changed() {
  local destination="$1"
  local temp
  temp="$(mktemp)"
  cat >"$temp"
  mkdir -p "$(dirname "$destination")"

  if [[ ! -f "$destination" ]] || ! cmp -s "$temp" "$destination"; then
    mv -f "$temp" "$destination"
    printf '==> updated %s\n' "$destination"
  else
    rm -f "$temp"
  fi
}

ensure_line() {
  local file="$1"
  local line="$2"
  mkdir -p "$(dirname "$file")"
  touch "$file"

  if ! grep -Fqx -- "$line" "$file"; then
    printf '\n%s\n' "$line" >>"$file"
    printf '==> connected Momarchy config from %s\n' "$file"
  fi
}

remove_exact_line() {
  local file="$1"
  local line="$2"
  [[ -f "$file" ]] || return 0

  local temp
  temp="$(mktemp)"
  awk -v exact="$line" '$0 != exact { print }' "$file" >"$temp"

  if ! cmp -s "$temp" "$file"; then
    mv -f "$temp" "$file"
    printf '==> migrated legacy inline Momarchy config in %s\n' "$file"
  else
    rm -f "$temp"
  fi
}

remove_legacy_home_lines() {
  local file="$1"
  [[ -f "$file" ]] || return 0

  local temp
  temp="$(mktemp)"
  awk '
    /-- Momarchy Home/ { next }
    /org\.momarchy\.home/ { next }
    /\.local\/bin\/momarchy.* home/ { next }
    { print }
  ' "$file" >"$temp"

  if ! cmp -s "$temp" "$file"; then
    mv -f "$temp" "$file"
    printf '==> migrated legacy inline Momarchy config in %s\n' "$file"
  else
    rm -f "$temp"
  fi
}

# Own Momarchy's small Hyprland snippets without taking ownership of the user's
# normal Omarchy config files. The user files only get one dofile() hook each.
write_if_changed "$CONFIG_DIR/hypr-autostart.lua" <<'EOF'
-- Managed by Momarchy install.sh.
o.launch_on_start("foot --app-id=org.momarchy.home -e " .. o.shell_quote(os.getenv("HOME") .. "/.local/bin/momarchy") .. " home --live-actions")
EOF

write_if_changed "$CONFIG_DIR/hypr-bindings.lua" <<'EOF'
-- Managed by Momarchy install.sh.
o.bind("SUPER + M", "Momarchy Home", {
  launch = "foot --app-id=org.momarchy.home -e " .. o.shell_quote(os.getenv("HOME") .. "/.local/bin/momarchy") .. " home --live-actions",
  focus = "org.momarchy.home",
})
EOF

remove_legacy_home_lines "$HYPR_DIR/autostart.lua"
remove_legacy_home_lines "$HYPR_DIR/bindings.lua"
ensure_line "$HYPR_DIR/autostart.lua" 'dofile(os.getenv("HOME") .. "/.config/momarchy/hypr-autostart.lua")'
ensure_line "$HYPR_DIR/bindings.lua" 'dofile(os.getenv("HOME") .. "/.config/momarchy/hypr-bindings.lua")'

# The 13-inch 2009 MacBook Pro exposes an embedded numeric keypad through
# hid_apple when Omarchy enables Num Lock by default. Apply the proven override
# only on that hardware, not on arbitrary Momarchy targets.
PRODUCT_NAME="$(cat /sys/class/dmi/id/product_name 2>/dev/null || true)"
if [[ "$PRODUCT_NAME" == "MacBookPro5,5" ]]; then
  write_if_changed "$CONFIG_DIR/hypr-input.lua" <<'EOF'
-- Managed by Momarchy install.sh for the 2009 13-inch MacBook Pro.
hl.config({ input = { numlock_by_default = false } })
EOF
  remove_exact_line "$HYPR_DIR/input.lua" 'hl.config({ input = { numlock_by_default = false } })'
  ensure_line "$HYPR_DIR/input.lua" 'dofile(os.getenv("HOME") .. "/.config/momarchy/hypr-input.lua")'
fi

# Omarchy's explicit stay-awake operation is idempotent and is preferable to
# recreating its idle-state implementation ourselves.
if ! omarchy toggle idle status 2>/dev/null | grep -q '"enabled":true'; then
  printf '%s\n' '==> disabling idle lock/screensaver through Omarchy'
  omarchy toggle idle stay-awake
fi

sddm_has_setting() {
  local pattern="$1"

  if [[ -d /etc/sddm.conf.d ]] && grep -RqsE "$pattern" /etc/sddm.conf.d 2>/dev/null; then
    return 0
  fi

  [[ -f /etc/sddm.conf ]] && grep -qsE "$pattern" /etc/sddm.conf
}

# SDDM has no Momarchy CLI for persistent appliance autologin. Use its normal
# drop-in mechanism, but avoid touching sudo when an equivalent config already
# exists. Explicit provisioning may therefore ask for the target user's sudo
# password; normal cargo deploy never runs this logic.
autologin_user_pattern="^[[:space:]]*User[[:space:]]*=[[:space:]]*${TARGET_USER}[[:space:]]*$"
autologin_session_pattern='^[[:space:]]*Session[[:space:]]*=[[:space:]]*omarchy\.desktop[[:space:]]*$'
if ! sddm_has_setting "$autologin_user_pattern" \
  || ! sddm_has_setting "$autologin_session_pattern"; then
  printf '==> configuring SDDM autologin for %s (sudo may prompt)\n' "$TARGET_USER"
  temp_autologin="$(mktemp)"
  cat >"$temp_autologin" <<EOF
[Autologin]
User=$TARGET_USER
Session=omarchy.desktop
EOF
  sudo install -Dm644 -o root -g root "$temp_autologin" /etc/sddm.conf.d/99-momarchy-autologin.conf
  rm -f "$temp_autologin"
fi

# Hardware-specific Wi-Fi firmware for the reference 2009 MacBook Pro. Keep the
# generic installer generic by keying this to the actual PCI ID.
if lspci -nn 2>/dev/null | grep -qi '\[14e4:432b\]' \
  && [[ ! -e /usr/lib/firmware/b43/ucode16_mimo.fw ]]; then
  if ! command -v yay >/dev/null 2>&1; then
    printf '%s\n' 'Broadcom BCM4322 detected but b43 firmware is missing.' >&2
    printf '%s\n' 'Install b43-firmware from the AUR (Omarchy normally provides yay), then reboot.' >&2
    exit 1
  fi

  printf '%s\n' '==> installing BCM4322 b43 firmware for the reference MacBook'
  yay -S --needed --noconfirm b43-firmware
fi

printf '%s\n' '==> Momarchy target provisioning complete'
printf '    user: %s\n' "$TARGET_USER"
printf '%s\n' '    Home: Omarchy autostart, live actions enabled'
printf '%s\n' '    Home key: Super+M'
printf '%s\n' '    idle lock/screensaver: disabled (stay awake)'
printf '%s\n' '    SDDM: appliance autologin configured'
printf '%s\n' 'A reboot is recommended after first-time provisioning to prove boot -> Home.'
