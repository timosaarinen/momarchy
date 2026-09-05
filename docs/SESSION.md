# Remote graphical session commands

A plain SSH shell does not inherit Omarchy's graphical Hyprland/Wayland environment. Commands such as `hyprctl` therefore fail over ordinary SSH even while the graphical session is running.

Use the project helper when a developer/admin wants to run one ordinary noninteractive command inside the target user's real graphical session:

```bash
cargo session t@momarchy hyprctl configerrors
cargo session t@momarchy hyprctl reload
cargo session t@momarchy omarchy brightness display on
```

`cargo session`:

- first checks bounded passwordless SSH reachability;
- verifies that the target user's systemd manager has `WAYLAND_DISPLAY` and `HYPRLAND_INSTANCE_SIGNATURE`;
- delegates execution through `systemd-run --user --pipe --wait --collect`, which inherits the actual Omarchy/Hyprland session environment;
- preserves the child command's stdout/stderr and exit status;
- uses SSH keepalives so a target that sleeps or disappears mid-command does not hang forever.

Arguments are passed as arguments, not interpreted as shell syntax. When pipes, redirects, expansion, or other shell semantics are genuinely needed, invoke a shell explicitly:

```bash
cargo session t@momarchy sh -lc 'hyprctl monitors -j | jq .[0]'
```

This is an admin/debug primitive, not a replacement for higher-level project commands. Keep dedicated commands when they encode a useful workflow or safety contract: for example, `cargo screenshot` handles display sleep, capture, restore, transfer, and opening the PNG; `cargo provision` owns cautious appliance configuration and validation.

The rule is: one generic session primitive for ad-hoc graphical commands, dedicated commands only when they add real semantics.
