# Hardware notes

## Primary target: 13-inch MacBook Pro (2009)

This became the first real Momarchy deployment target mostly by accident, and current Omarchy 4.0.2 works much better on it than expected.

Observed under Omarchy:

- CPU: Intel Core 2 Duo P7350 @ 2.00 GHz
- Architecture: x86-64
- Memory: 2 GB
- GPU/chipset: NVIDIA C79 / GeForce 9400M G
- Linux graphics driver: `nouveau`
- Wi-Fi: Broadcom BCM4322 802.11a/b/g/n
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

### Installation notes

- Kept OS X El Capitan as an archaeology/fallback install.
- Created ~50 GB from macOS Disk Utility, then deleted the temporary HFS+ partition in the Omarchy installer's partition tool so the space was truly unallocated.
- Ventoy 1.1.17 froze after selecting EFI Boot in old Apple firmware.
- Flashing the Omarchy 4.0.2 ISO directly to USB worked.
- Omarchy installed in 12m 56s.
- Normal reboot selected Omarchy correctly; no Option-key boot selection was required after installation.
- Linux's Apple keyboard driver can emulate the old embedded numpad layout when Num Lock is active: `j` becomes `1`, `k` becomes `2`, etc. This has occasionally come up enabled after boot on this machine; plain `F6` toggles Num Lock off.

### Open hardware work

Broadcom BCM4322 Wi-Fi is not working yet. Ethernet works and was used for the first Omarchy update. Investigate the correct current Arch/Omarchy Broadcom driver path before adding packages blindly.

Still worth testing for the intended daily-use machine:

```text
audio
Wi-Fi + reconnect after reboot
suspend/resume
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
