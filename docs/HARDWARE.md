# Hardware notes

## Primary target: 13-inch MacBook Pro (2009)

This became the first real Momarchy deployment target mostly by accident, and current Omarchy 4.0.2 works much better on it than expected.

Observed under Omarchy:

- CPU: Intel Core 2 Duo P7350 @ 2.00 GHz
- Architecture: x86-64
- Memory: 2 GB
- GPU/chipset: NVIDIA C79 / GeForce 9400M G
- Linux graphics driver: `nouveau`
- Display: 1280×800 native panel (16:10)
- Wi-Fi: Broadcom BCM4322 802.11a/b/g/n, working with Linux `b43` + `b43-firmware`
- Storage: ~112 GB Hitachi SATA disk
- Omarchy partition: ~50 GB carved out beside the existing macOS installation
- Swap: ~3.3 GiB free after initial boot
- Available memory after initial Omarchy boot: ~933 MiB
- Available memory with Chrome on its home page: ~800 MiB
- Available memory while scrolling a real `is.fi` page: roughly ~500 MiB
- Apple SMC sensors: working
- Fan reporting/control: working; observed ~2000 RPM at idle/minimum
- Battery reporting: working
- Reported full charge capacity: 4,795,000
- Reported design capacity: 5,020,000 (~95.5% remaining)

The GeForce 9400M-era NVIDIA chipset also provides a large part of the platform I/O (memory controller, SATA, USB, audio, PCI, etc.), so `lspci` contains a lot more NVIDIA than just the display controller.

### Wallpaper target

The reference MBP13 panel is exactly 1280×800 / 16:10, which makes **320×200** a convenient retro/pixel-art authoring resolution: it scales to the native panel at an exact 4× in both dimensions. **320×240 is 4:3 and does not match the panel**; a crop-to-fill background would have to crop the top and bottom.

For the planned low-key Momarchy Finnish-lake wallpaper, prefer authoring/editing at 320×200. Letting the desktop scale that small source is fine when a little softness is acceptable. If exact hard pixel edges are wanted later, nearest-neighbor upscale the finished 320×200 image to 1280×800 before deployment instead of relying on the background renderer's scaling.

### Installation notes

- Kept OS X El Capitan as an archaeology/fallback install.
- Created ~50 GB from macOS Disk Utility, then deleted the temporary HFS+ partition in the Omarchy installer's partition tool so the space was truly unallocated.
- Ventoy 1.1.17 froze after selecting EFI Boot in old Apple firmware.
- Flashing the Omarchy 4.0.2 ISO directly to USB worked.
- Omarchy installed in 12m 56s.
- Normal reboot selected Omarchy correctly; no Option-key boot selection was required after installation.
- Linux's Apple keyboard driver can emulate the old embedded numpad layout when Num Lock is active: `j` becomes `1`, `k` becomes `2`, etc. This has occasionally come up enabled after boot on this machine; plain `F6` toggles Num Lock off.

### Wi-Fi

The built-in Broadcom BCM4322 is PCI ID `14e4:432b` (Apple AirPort Extreme subsystem `106b:008e`). The kernel already bound it through `b43-pci-bridge` / `ssb`, and `b43` successfully detected the BCM4322 N-PHY and radio, but no wireless interface appeared because the required firmware was missing:

```text
b43-phy0 ERROR: Firmware file "b43/ucode16_mimo.fw" not found
```

Installing the AUR package `b43-firmware` supplied the missing firmware. After reboot, `b43` loaded firmware version 784.2, `wlan0` appeared, NetworkManager could scan both 2.4 GHz and 5 GHz networks, and the MacBook connected normally.

A cold reboot with Ethernet unplugged verified that NetworkManager auto-connects to the saved Wi-Fi network. IPv4, IPv6, DNS and SSH over Wi-Fi all worked. Keep `b43`; there is no reason to switch to proprietary `broadcom-wl` unless real instability appears later.

One later crowded 2.4 GHz WLAN exposed severe latency/packet loss plus repeated `4WAY_HANDSHAKE_TIMEOUT` and `b43-phy0 ERROR: MAC suspend failed` messages. That needs comparison against another AP/hotspot and 5 GHz before changing drivers. Resume testing also caught one `mac80211`/`cfg80211` `wiphy_resume` path failing an order-2 page allocation and printing `Hardware became unavailable upon resume`; that is a clue, not proof that Wi-Fi caused the intermittent hard resume hang described below.

### Suspend/resume archaeology and current no-sleep policy

The reference MBP13 can enter ACPI S3 (`PM: suspend entry (deep)`), but repeated testing showed that resume is not appliance-grade reliable on this hardware/software combination.

Early tests were confusing because Omarchy's separate `omarchy-sleep-lock.service` added a password gate before suspend. Before that service was masked, resume sometimes showed an almost black/dim password screen with only a tiny/pixel-mess-looking center UI; typing the password still unlocked into normal-looking Omarchy graphics. Masking the pre-sleep lock proved that a normal successful deep-S3 cycle could resume directly into the existing session without a password.

Repeated deep-S3 testing then exposed the actual blocker: one cycle did not wake normally at all. Opening the lid left the panel completely black, keyboard and short power-button presses did nothing, and SSH was unreachable; recovery required a long power-button forced shutdown. Previous successful resumes also logged real `nouveau` GeForce 9400M faults (`INVALID_OPCODE`, `TRAP_TEXTURE`, `PT_NOT_PRESENT`) involving Quickshell, and one resume hit the Wi-Fi allocation failure noted above. The final hard-hang attempt ended at `PM: suspend entry (deep)` with no recorded lid-open event, ACPI wake, or `PM: suspend exit`, so the evidence never isolated one culprit.

A temporary `s2idle` experiment improved the hard-wake behavior in a short test, but a roughly five-minute suspend still resumed through black/brief-Home/password-screen weirdness. That was enough evidence to stop spending product-critical time on sleep semantics for now.

**Current Momarchy policy: do not sleep.** Provisioning disables systemd suspend, hibernation, hybrid sleep and suspend-then-hibernate through a Momarchy-owned `sleep.conf.d` drop-in. A separate `logind.conf.d` drop-in explicitly ignores lid-close, suspend-key and hibernate-key events (on battery and external power). Omarchy's idle stay-awake mode remains enabled and the pre-sleep lock monitor remains masked as defense in depth. Closing the lid should therefore leave the machine running; shutting down is an explicit user action.

This is deliberately a product policy rather than pretending the underlying resume bug is fixed. Suspend/resume can be revisited later if there is a real reason to bring sleep back, using the captured evidence below instead of rediscovering it.

Useful previous-boot evidence commands:

```bash
journalctl -b -1 -k --no-pager | grep -Ei 'nouveau|drm|suspend|resume|PM:|ACPI|error|fail'
journalctl -b -1 --no-pager | grep -Ei 'suspend|sleep|lid|nouveau|drm|watchdog|freeze'
```

### Open hardware work

Still worth testing for the intended daily-use machine:

```text
audio
browser video / YouTube
long-running thermal stability
battery runtime
```

The 2 GB memory limit is the main product constraint. Normal Momarchy operation should try to avoid active swap churn; swap remains useful as a safety net.

## Secondary candidate: Lenovo IdeaPad Y500

Observed from firmware/Windows archaeology:

- Product: Lenovo IdeaPad Y500
- CPU: Intel Core i7-3630QM @ 2.40 GHz
- Memory: 8192 MB
- Storage: ST1000LM024 / HN-M101MBB, 1 TB mechanical HDD
- Firmware: InsydeH2O Rev. 3.7
- BIOS/EC: 6BCN34WW (V1.05)
- Boot mode: UEFI
- Original Windows license: Windows 8 Standard
- Believed to be the old dual-GPU gaming configuration

The HDD is mechanically suspicious: abnormal noises and extremely slow random-access behavior were observed, though it eventually booted the old Windows installation and old data was backed up successfully.

The Y500 has much more compute/memory than the MacBook, but the old dual-GPU gaming hardware, thermals and HDD make it a less attractive appliance target. Keep it as a possible test machine.

## Useful validation commands

```bash
lscpu
free -h
lsblk -o NAME,SIZE,MODEL,ROTA,FSTYPE,MOUNTPOINTS
lspci -nnk | grep -A3 -Ei 'vga|3d|display|network|wireless'
sensors
```

For questionable mechanical disks, inspect SMART data before destructive testing:

```bash
sudo smartctl -a /dev/sda
```

Recover wanted data before filesystem repair, bad-block scans or overwrite tests.
