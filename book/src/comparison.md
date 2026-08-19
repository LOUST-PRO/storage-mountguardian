# Comparison

`storage-mountguardian` is one of several approaches to handling
failing block devices on Linux. This page positions it honestly
against the alternatives so you can pick the right tool for the
job.

## vs udev rules

**udev** is the right tool for *detecting* device events at plug
time, but it is the wrong tool for *reacting* to runtime IO failure
events. udev rules run on `add`, `remove`, `change` uevents — not on
the SCSI/USB error log lines that signal a runtime failure.

You can construct a setup that approximates `storage-mountguardian`
with udev by:

1. Listening on `monitor` socket
2. Filtering for `ID_IO_ERROR` or similar (if your kernel emits them)
3. Running `umount -l` from a RUN+= directive

The problem is that the `ID_IO_ERROR` uevent path is not consistently
emitted across kernels and storage stacks. In practice, what shows up
in `/dev/kmsg` (the `DID_NO_CONNECT` hostbyte, the `USB disconnect`
line) does **not** reliably surface as a uevent. So the udev-only
approach has gaps.

`storage-mountguardian` reads `/dev/kmsg` directly, which is the
canonical record of every kernel-side event. The matchers there are
stable across kernels and storage stacks.

**When to pick udev instead**: if you want to react to plug-time
events (auto-mounting, permissions on insertion), keep udev. They
are complementary, not competing.

## vs `mdev`-style daemons

Several projects in this space take the shape of a daemon that
listens on a `udev` monitor socket and runs `umount` on detected
errors. Compared to `storage-mountguardian`:

| Property | `mdev`-style daemon | `storage-mountguardian` |
|---|---|---|
| Source of truth | udev uevent stream | `/dev/kmsg` (kernel ring buffer) |
| Coverage of `DID_NO_CONNECT` | Depends on kernel/storage stack emitting a uevent | Always present |
| Per-record allocation bound | Variable | Hard 64 KiB cap |
| Rate-limit dedup | Variable | Built-in (5 s window per device) |
| Partition enumeration | Heuristic (`sdb` → `sdb[0-9]+`) | sysfs walk (`/sys/block/<dev>/`) |
| Memory footprint | Often >20 MB (udev crate + glib loop) | ~4 MB |
| Distribution | Often a single shell script + systemd | Single static binary, crates.io |

If you are running a `mdev`-style daemon today and it works for
your hardware, there is no urgent reason to switch — but if you
have flaky devices that don't trigger the daemon reliably, the
`/dev/kmsg` approach in `storage-mountguardian` is worth evaluating.

## vs commercial hardware-RAID health tools

Vendor tools (`MegaRAID Storage Manager`, `HPE SSA`, `Dell
OpenManage`) monitor RAID controller batteries, drive SMART
attributes, and predictive failure events. They are excellent at
*predicting* drive death hours or days in advance.

They are not a substitute for `storage-mountguardian` because:

- They cover RAID-managed drives only, not USB-attached storage or
  consumer SD card readers.
- They trigger actions (email alerts, LED blink) on **prediction**,
  not on **runtime failure**. If a drive dies *without* prior SMART
  warnings, they don't help.
- They are heavyweight: vendor agents can pull hundreds of MB of
  RAM and ship proprietary protocols.

The honest positioning: run commercial RAID health tools *and*
`storage-mountguardian`. They cover different failure modes.

## What `storage-mountguardian` does NOT replace

- **Filesystem journaling** (`ext4`, `xfs`, `zfs`) — these protect
  against metadata corruption from unclean shutdowns. They don't
  help with the D-state freeze problem.
- **SMART monitoring** (`smartd`) — these predict drive death from
  attribute drift. They don't help with sudden cable disconnects.
- **UPS monitoring** (`apcupsd`, `nut`) — these handle power loss,
  which is a different failure mode entirely.

`storage-mountguardian` is a single-purpose tool: detect kernel-level
SCSI/USB disconnect events and break the IO wait loop. Use it
alongside the tools above, not instead of them.
