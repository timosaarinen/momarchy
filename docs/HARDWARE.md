# Hardware notes

## First target: Lenovo IdeaPad Y500

Observed from firmware setup:

- Product: Lenovo IdeaPad Y500
- CPU: Intel Core i7-3630QM @ 2.40 GHz
- Memory: 8192 MB
- Storage: ST1000LM024 / HN-M101MBB, 1 TB mechanical HDD
- Firmware: InsydeH2O Rev. 3.7
- BIOS/EC: 6BCN34WW (V1.05)
- Boot mode: UEFI
- Secure Boot: enabled at time of inspection
- Original Windows license: Windows 8 Standard

The HDD is suspected to be mechanically damaged: abnormal noises were observed and the machine did not boot its old Windows installation. Treat existing data as potentially fragile until copied elsewhere.

The machine is believed to be a dual-GPU gaming configuration. Exact GPU enumeration should be captured from a live Linux environment before deciding on Omarchy compatibility.

## Validation checklist

From a live Linux environment, capture at least:

```bash
lscpu
free -h
lsblk -o NAME,SIZE,MODEL,ROTA,FSTYPE,MOUNTPOINTS
lspci -nnk | grep -A3 -Ei 'vga|3d|display|network|wireless'
```

If available, inspect SMART data without initiating destructive tests:

```bash
sudo smartctl -a /dev/sda
```

Before any filesystem repair or overwrite, recover wanted files from the old Windows installation first.
