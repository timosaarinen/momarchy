#!/usr/bin/env bash
set -Eeuo pipefail

trap 'status=$?; line=$LINENO; command=$BASH_COMMAND; printf "Momarchy provisioning failed at line %s while running: %s (exit %s)\n" "$line" "$command" "$status" >&2; exit "$status"' ERR

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
HYPR_BACKUP_DIR=""

mkdir -p "$HOME/.local/bin" "$CONFIG_DIR" "$STATE_DIR" "$HYPR_DIR"

cleanup() {
  if [[ -n "$HYPR_BACKUP_DIR" && -d "$HYPR_BACKUP_DIR" ]]; then
    rm -rf "$HYPR_BACKUP_DIR"
  fi
}
trap cleanup EXIT

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
    printf '==> migrated exact legacy Momarchy line in %s\n' "$file"
  else
    rm -f "$temp"
  fi
}

legacy_inline_home_present() {
  local file="$1"
  [[ -f "$file" ]] || return 1

  grep -Eq -- '-- Momarchy Home|org\.momarchy\.home|\.local/bin/momarchy.*[[:space:]]home' "$file"
}

refuse_legacy_inline_home() {
  local file="$1"
  if legacy_inline_home_present "$file"; then
    printf 'Legacy inline Momarchy Home config detected in %s.\n' "$file" >&2
    printf '%s\n' 'Momarchy will not rewrite arbitrary Lua automatically.' >&2
    printf 'Open %s, remove only the old inline Momarchy Home block, keep normal Omarchy config, then rerun `cargo provision`.\n' "$file" >&2
    return 1
  fi
}

hyprland_session_running() {
  systemctl --user show-environment 2>/dev/null | grep -q '^HYPRLAND_INSTANCE_SIGNATURE='
}

run_graphical() {
  systemd-run --user --quiet --pipe --wait --collect \
    --property=RuntimeMaxSec=10s \
    --setenv=PATH="$PATH" \
    "$@"
}

reload_hyprland() {
  local reload_bin
  if reload_bin="$(command -v omarchy-restart-hyprctl 2>/dev/null)"; then
    run_graphical "$reload_bin"
  else
    local hyprctl_bin
    hyprctl_bin="$(command -v hyprctl)" || {
      printf '%s\n' 'hyprctl is unavailable; cannot validate Hyprland config.' >&2
      return 127
    }
    run_graphical "$hyprctl_bin" reload
  fi
}

hyprland_config_errors() {
  local hyprctl_bin
  hyprctl_bin="$(command -v hyprctl)" || {
    printf '%s\n' 'hyprctl is unavailable; cannot inspect Hyprland config errors.' >&2
    return 127
  }
  run_graphical "$hyprctl_bin" configerrors
}

validate_hyprland_config() {
  local phase="$1"

  if ! hyprland_session_running; then
    printf '==> no running Hyprland session; %s config validation deferred until login\n' "$phase"
    return 0
  fi

  printf '==> validating Hyprland Lua (%s)\n' "$phase"
  reload_hyprland >/dev/null

  local errors
  errors="$(hyprland_config_errors)"
  if grep -q '[^[:space:]]' <<<"$errors"; then
    printf 'Hyprland config errors (%s):\n%s\n' "$phase" "$errors" >&2
    return 1
  fi
}

backup_file() {
  local file="$1"
  local key="$2"

  if [[ -e "$file" ]]; then
    cp -p -- "$file" "$HYPR_BACKUP_DIR/$key"
  else
    : >"$HYPR_BACKUP_DIR/$key.absent"
  fi
}

restore_file() {
  local file="$1"
  local key="$2"

  if [[ -f "$HYPR_BACKUP_DIR/$key.absent" ]]; then
    rm -f -- "$file"
  else
    mkdir -p "$(dirname "$file")"
    cp -p -- "$HYPR_BACKUP_DIR/$key" "$file"
  fi
}

start_hypr_backup() {
  HYPR_BACKUP_DIR="$(mktemp -d "$STATE_DIR/hypr-config-backup.XXXXXX")"
  backup_file "$HYPR_DIR/autostart.lua" hypr-autostart.lua
  backup_file "$HYPR_DIR/bindings.lua" hypr-bindings.lua
  backup_file "$HYPR_DIR/input.lua" hypr-input.lua
  backup_file "$CONFIG_DIR/hypr-autostart.lua" momarchy-hypr-autostart.lua
  backup_file "$CONFIG_DIR/hypr-bindings.lua" momarchy-hypr-bindings.lua
  backup_file "$CONFIG_DIR/hypr-input.lua" momarchy-hypr-input.lua
}

restore_hypr_backup() {
  printf '%s\n' 'Restoring Hyprland files from the pre-provision backup.' >&2
  restore_file "$HYPR_DIR/autostart.lua" hypr-autostart.lua
  restore_file "$HYPR_DIR/bindings.lua" hypr-bindings.lua
  restore_file "$HYPR_DIR/input.lua" hypr-input.lua
  restore_file "$CONFIG_DIR/hypr-autostart.lua" momarchy-hypr-autostart.lua
  restore_file "$CONFIG_DIR/hypr-bindings.lua" momarchy-hypr-bindings.lua
  restore_file "$CONFIG_DIR/hypr-input.lua" momarchy-hypr-input.lua

  if hyprland_session_running; then
    reload_hyprland >/dev/null || printf '%s\n' 'Warning: could not reload restored Hyprland config.' >&2
  fi
}

# Never mutate arbitrary Lua that is already known-bad. Omarchy itself recommends
# validating Hyprland edits with reload + configerrors.
validate_hyprland_config existing

# Earlier Momarchy prototypes wrote Home directly into these user files. Do not
# try to parse/remove arbitrary Lua with grep/awk: ask the developer to remove the
# old block once, then future provisioning owns only one dofile() hook per file.
refuse_legacy_inline_home "$HYPR_DIR/autostart.lua"
refuse_legacy_inline_home "$HYPR_DIR/bindings.lua"

start_hypr_backup

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

if ! validate_hyprland_config Momarchy; then
  restore_hypr_backup
  exit 1
fi

rm -rf "$HYPR_BACKUP_DIR"
HYPR_BACKUP_DIR=""

# Omarchy's explicit stay-awake operation is idempotent and is preferable to
# recreating its idle-state implementation ourselves.
if ! omarchy toggle idle status 2>/dev/null | grep -q '"enabled":true'; then
  printf '%s\n' '==> disabling idle lock/screensaver through Omarchy'
  omarchy toggle idle stay-awake
fi

# Omarchy separately locks the graphical session before logind suspends or
# hibernates. Momarchy's appliance account intentionally has no password gates,
# so disable only that user-level pre-sleep lock monitor. Do not change logind's
# lid-switch or suspend policy: closing the lid must still suspend the machine.
SLEEP_LOCK_SERVICE="omarchy-sleep-lock.service"
if ! systemctl --user list-unit-files "$SLEEP_LOCK_SERVICE" --no-legend 2>/dev/null \
  | grep -q "^${SLEEP_LOCK_SERVICE}[[:space:]]"; then
  printf 'Expected Omarchy pre-sleep lock service %s was not found.\n' "$SLEEP_LOCK_SERVICE" >&2
  printf '%s\n' 'Refusing to guess at a replacement service; inspect the installed Omarchy sleep/lock units, then update Momarchy provisioning explicitly.' >&2
  exit 1
fi
printf '%s\n' '==> disabling pre-sleep session lock; lid suspend remains enabled'
systemctl --user mask --now "$SLEEP_LOCK_SERVICE"

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
# generic installer generic by keying this to the actual PCI ID and ask pacman,
# rather than a guessed firmware path, whether the AUR package is installed.
if lspci -nn 2>/dev/null | grep -qi '\[14e4:432b\]' \
  && ! pacman -Qq b43-firmware >/dev/null 2>&1; then
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
printf '%s\n' '    pre-sleep session lock: disabled; lid suspend unchanged'
printf '%s\n' '    SDDM: appliance autologin configured'
printf '%s\n' 'A reboot is recommended after first-time provisioning to prove boot -> Home.'
