# Provisioning policy

`cargo provision` is the explicit operation that is allowed to change a Momarchy target outside the Momarchy binary/Lua payload. Treat it as privileged appliance configuration, not as a convenient place for best-effort shell mutations.

This document is normative for Momarchy provisioning code.

## Core rule: fail closed

Provisioning must be cautious and non-destructive by default.

If the target state is missing, unexpected, ambiguous, already invalid, or cannot be checked with enough confidence, **stop**. Print what was found, what Momarchy expected, and the smallest concrete manual repair/check needed before retrying.

Do not guess. Do not silently "repair" arbitrary user/Omarchy configuration. Do not keep going after a validation failure just because later steps might still work.

## Ownership boundary

Momarchy may freely replace files that are explicitly Momarchy-owned, such as files under `~/.config/momarchy/` and its own state files.

For Omarchy/user-owned files such as `~/.config/hypr/*.lua`, Momarchy must make only minimal, exact changes whose ownership and effect are known. Prefer adding one explicit hook such as:

```lua
dofile(os.getenv("HOME") .. "/.config/momarchy/hypr-bindings.lua")
```

Do **not** parse or rewrite arbitrary Lua with broad `grep`, `sed`, `awk`, regex, or substring heuristics. In particular, never delete lines merely because they mention Momarchy: a multi-line Lua expression can be left syntactically broken even when every deleted line looked individually correct.

If legacy inline Momarchy code cannot be identified as one exact known block, report the file and ask the developer to remove that block manually.

## Required safety sequence

A provisioning change that touches target configuration should follow this order:

1. **Preflight** — verify the expected target, required commands, relevant current state, and any known configuration errors before mutation.
2. **Detect exact need** — only change something when the desired state can be determined reliably. Idempotent "already correct" must be a normal no-op.
3. **Back up touched non-Momarchy files** — before editing user/Omarchy-owned configuration, save the exact previous files needed for rollback.
4. **Apply the smallest change** — prefer Omarchy's own primitive first, then ordinary Arch/Linux configuration mechanisms, then Momarchy-specific code only when necessary.
5. **Validate the result** — use the subsystem's real validation mechanism. For Hyprland Lua, reload the config and inspect `hyprctl configerrors` when a graphical session is available.
6. **Rollback on introduced failure** — if Momarchy's change makes the validated state worse or invalid, restore the pre-provision files and report the failure.
7. **Record success only at the end** — `install.sh.applied` represents a provisioner that completed successfully; never mark a failed or partially applied run as current.

If live validation is unavailable, provisioning may only make changes that are exact, additive/owned, and safely reversible. It must not use that lack of validation as permission for heuristic migration.

## Privileged changes

`cargo provision` may prompt for `sudo` when an actual privileged change is required, for example an SDDM drop-in or missing package.

Before using privilege, first determine whether the target is already correct without sudo. Re-running provisioning on a correct target should be boring and should not ask for privilege unnecessarily.

Normal `cargo deploy` must never run privileged/system provisioning automatically. Deploy compares the checked-out provisioner with the last successfully applied snapshot and refuses with a concrete `cargo provision <target>` instruction when provisioning is missing or stale.

## Failure output

Provisioning failures are part of the admin interface. A useful failure should include, when applicable:

- the provisioning step that failed;
- the actual command or file involved;
- exit/start status;
- useful underlying stdout/stderr;
- the unexpected state that caused Momarchy to stop;
- a concrete check or repair command/file to inspect next.

A bare `exit status: 1` is not sufficient.

## Remote robustness

Remote provisioning must not appear frozen merely because the target is asleep or unreachable. SSH/SCP connection establishment is bounded and should fail with an explicit target-reachability diagnosis.

Do not put a small blanket wall-clock timeout around the entire provision operation: legitimate package/AUR installation and an intentional sudo prompt can take a long time on old hardware. Bound the operations that are expected to be short; keep long operations visible and diagnosable.

## Bias

When choosing between "probably safe automation" and "stop and ask the developer to inspect one file", provisioning chooses the latter.

Momarchy Home should be simple for the end user. The provisioning path is allowed to be conservative for the developer.