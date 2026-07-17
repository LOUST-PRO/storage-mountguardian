# storage-mountguardian

[![Crates.io](https://img.shields.io/crates/v/storage-mountguardian.svg)](https://crates.io/crates/storage-mountguardian)
[![Downloads](https://img.shields.io/crates/d/storage-mountguardian.svg)](https://crates.io/crates/storage-mountguardian)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`storage-mountguardian` is a highly efficient, lightweight Linux system daemon written in Rust. It acts as an emergency "eject button" for failing hardware, proactively preventing your system and applications from freezing when a USB drive, external HDD, or block device starts dropping connections or throwing hardware errors.

## The Problem
When a block device (like a faulty USB drive or external HDD with a bad cable) starts dropping its connection, the Linux SCSI/USB subsystem tries to wait for it. Applications (like file managers, Discord, or `bwrap` sandboxes) that try to access the mount point will get stuck in **Uninterruptible Sleep (D-state)** until the kernel timeout is reached. This freezes the applications and heavily impacts system usability.

## The Solution
`storage-mountguardian` monitors the kernel ring buffer (`/dev/kmsg`) in real-time with virtually zero CPU overhead. The millisecond it detects critical SCSI/USB disconnects or `DID_NO_CONNECT` hostbyte errors for a block device, it surgically amputates the hanging mounts using a lazy unmount (`umount -l`). 
This immediately breaks the IO wait loop, releasing any stuck processes and restoring system stability instantly.

## Features
- **Zero-Polling Overhead:** Uses native Rust I/O to read `/dev/kmsg` cleanly.
- **Regex-Powered Detection:** Instantly identifies `DID_NO_CONNECT`, `DID_BAD_TARGET`, and USB disconnects.
- **Surgical Amputation:** Safely and aggressively lazy-unmounts the faulty device and its partitions to unblock the kernel block layer.
- **Memory Safe:** Built entirely in safe Rust. Memory footprint is typically ~4 MB RSS.
- **Rate-limit dedup** *(v0.2.0+)*: Same device is not re-amputated within a 5 s window. Protects the system from self-DoS when a flaky USB emits hundreds of errors per second.
- **Sysfs partition discovery** *(v0.2.0+)*: Reads `/sys/block/<dev>/` to enumerate actual partitions. Correctly handles `sd*` (sdb1, sdb2), `nvme*` (nvme0n1p1, nvme0n1p2), `mmcblk*`, and other block device naming conventions.
- **Allocation bound** *(v0.2.0+)*: Drops kmsg records larger than 64 KiB to defend against unbounded buffer growth.

## Installation

You can install it directly via cargo:

```bash
cargo install storage-mountguardian
```

## Running as a systemd service (Recommended)

A hardened systemd unit ships in [`contrib/storage-mountguardian.service`](contrib/storage-mountguardian.service). It uses `ProtectSystem=strict`, `ProtectHome=read-only`, `PrivateTmp`, and other reasonable hardening for a watchdog.

1. Copy the unit into place:
   ```bash
   sudo install -m 0644 contrib/storage-mountguardian.service /etc/systemd/system/
   ```
   *(If you installed via `cargo install`, edit the unit's `ExecStart` to point at `~/.cargo/bin/storage-mountguardian` instead of `/usr/local/bin/`.)*

2. Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now storage-mountguardian
```

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
