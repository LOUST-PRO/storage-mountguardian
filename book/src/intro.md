# Storage Mount Guardian

[![Crates.io](https://img.shields.io/crates/v/storage-mountguardian.svg)](https://crates.io/crates/storage-mountguardian)
[![Downloads](https://img.shields.io/crates/d/storage-mountguardian.svg)](https://crates.io/crates/storage-mountguardian)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/LICENSE)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org/)

> **TL;DR** — `storage-mountguardian` is a tiny Linux daemon (~4 MB RSS, zero
> polling) that watches `/dev/kmsg` for SCSI/USB disconnect events and
> surgically lazy-unmounts the affected block device the instant errors
> appear, so your file manager, sandboxed apps, and shell don't freeze in
> **Uninterruptible Sleep (D-state)** waiting for a dead disk to come back.

---

## What it is

A single static binary (`storage-mountguardian`) plus a hardened systemd
unit. Written in safe Rust, no `unsafe` code, no async runtime, no
background threads beyond the read loop on `/dev/kmsg`. Built once,
deployed everywhere that Linux touches a USB stick, an external HDD, or
an SD card reader.

The daemon reads kernel log records in real time. The millisecond it
matches a critical disconnect pattern (`DID_NO_CONNECT`, `DID_BAD_TARGET`,
or a generic `USB disconnect` event for a block device), it enumerates
the partitions of the affected device via `/sys/block/<dev>/` and
issues a **lazy** `umount -l` against each mount point. The kernel
breaks the IO wait loop instantly, the stuck processes wake up, and
your session stays interactive.

## Why it exists

When a block device drops its connection — bad USB cable, dying SD
card, faulty SATA link — the Linux SCSI/USB subsystem *waits* for it.
Default timeouts are measured in minutes. Anything that opened a file
on the device (file manager, editor, sandbox, browser download) gets
stuck in **D-state** until the kernel timeout expires.

That feels like a frozen desktop. You can't even `kill -9` the stuck
process — `kill` ignores D-state. The only mitigation today is
rebooting, which is destructive and unscheduled.

`storage-mountguardian` is the eject button. It doesn't try to *fix*
the failing hardware (nothing can); it surgically removes the mount
points so the kernel stops waiting, which lets everything else
continue normally. The hardware is still failing, but your system is
no longer hostage to it.

## At a glance

| Property | Value |
|---|---|
| Binary footprint | ~4 MB RSS, single static binary |
| CPU overhead | Negligible (one blocking read on `/dev/kmsg`) |
| Polling | None — kernel-pushed events only |
| Language | Rust (edition 2024), `#![deny(unsafe_code)]` |
| Runtime deps | None (only `regex` crate, statically linked) |
| Privileges | Must run as `root` (only process allowed to lazy-unmount) |
| Distribution | crates.io + system package builds (deb/rpm/PKGBUILD forthcoming) |
| License | Apache-2.0 |
| Source | [github.com/LOUST-PRO/storage-mountguardian](https://github.com/LOUST-PRO/storage-mountguardian) |

## Where to next

- New user? Jump to **[Getting started](./installation.md)**.
- Curious about internals? Read **[How it works](./how-it-works.md)**.
- Operating it in production? See **[Hardening](./hardening.md)**.
- Picking between this and alternatives? Read **[Comparison](./comparison.md)**.
- Hunting for a specific symbol or flag? Check **[Reference](./reference.md)**.
