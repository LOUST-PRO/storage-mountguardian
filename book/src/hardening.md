# Hardening

The daemon must run as `root` to call `umount -l` against arbitrary
mount points. That doesn't mean the daemon has to be wide-open to
the rest of the system. The shipped systemd unit applies the standard
"watchdog on a tight leash" hardening set. This page walks through
each directive so operators understand the surface they're exposing.

## systemd unit anatomy

The full unit ships at
[`contrib/storage-mountguardian.service`](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/contrib/storage-mountguardian.service).
The hardening-relevant directives:

```ini
[Service]
Type=simple
ExecStart=/usr/local/bin/storage-mountguardian
Restart=always
RestartSec=5
User=root
Group=root

NoNewPrivileges=false
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

StandardOutput=journal
StandardError=journal
SyslogIdentifier=storage-mountguardian
```

### `[Unit]` block

| Directive | Value | Purpose |
|---|---|---|
| `After=` | `systemd-udevd.service` | Start only after udev is alive — required so `/sys/block/` is populated |
| `StartLimitIntervalSec=` | `300` | Window over which restart bursts are counted |
| `StartLimitBurst=` | `5` | Max restarts inside the window before the unit is failed |

### Restart behaviour

```ini
Restart=always
RestartSec=5
```

The daemon is designed to never exit voluntarily, so `always` is
correct: if it does exit (panic, kernel bug, OOM kill), systemd
brings it back in 5 seconds. The `StartLimitBurst=5` ceiling stops a
runaway loop from flooding the journal.

## Why `root` and `ProtectSystem=strict`

The daemon needs `CAP_SYS_ADMIN` to issue `umount`. There's no way to
scope that capability down further without breaking the basic
operation, so the unit runs as `root` and mitigates the blast radius
with the systemd sandbox directives.

`ProtectSystem=strict` makes the entire filesystem read-only except
for `/dev/`, `/proc/`, `/sys/`, and `/run/`. The daemon writes
**nothing** to disk (no config, no cache, no state) so this doesn't
constrain functionality — it just removes an attack surface.

`ProtectHome=read-only` keeps `/home` visible but read-only. The
daemon doesn't need to read user files, but the visibility is
useful for diagnostics (e.g. `ls` of a user mount point to log what
was just amputated).

`PrivateTmp=true` gives the daemon a private `/tmp`. Doesn't change
behaviour because the daemon doesn't use `/tmp`, but it's free
defense-in-depth.

`ProtectKernelTunables=true` + `ProtectKernelModules=true` +
`ProtectControlGroups=true` close the sysctl/module/cgroup write
paths. The daemon doesn't need any of them, so denying them is a
free win.

## Why `NoNewPrivileges=false`

This one is deliberate — it deviates from the "secure-by-default"
recommendation.

`NoNewPrivileges=true` would prevent the daemon from acquiring new
privileges via `setuid` binaries. The daemon doesn't call any
`setuid` binary itself, so the flag would be inert. We leave it at
`false` so that operators who wrap the daemon in their own tooling
(say, a shell-script wrapper that drops privileges before exec'ing
the binary) can still rely on `setuid` semantics if they need them.

If you want to tighten further, override the unit to add
`NoNewPrivileges=true` after the `[Service]` block — the daemon will
still work.

## Logging

All output goes to the systemd journal:

```ini
StandardOutput=journal
StandardError=journal
SyslogIdentifier=storage-mountguardian
```

There are no log files on disk, no rotation, no log retention
policies to manage. The journal handles all of that. Operators tail
with:

```bash
journalctl -u storage-mountguardian -f
```

Or filter by priority:

```bash
journalctl -u storage-mountguardian -p warning
```

## What the daemon does NOT need

These are common hardening directives that the unit intentionally
omits because the daemon does not require them — adding them is
fine but not necessary:

- `MemoryMax=` — the daemon's RSS is ~4 MB by design. Adding a cap
  would only matter if a future bug caused unbounded growth; the
  v0.2.0 allocation bound prevents that.
- `CPUQuota=` — the daemon spends 99.9% of its time blocked on
  `read_line` from `/dev/kmsg`. CPU quota is irrelevant.
- `RestrictAddressFamilies=` — could be locked to `AF_UNIX` +
  `AF_NETLINK`, but the daemon doesn't open sockets at all, so
  there's no risk to constrain.
- `SystemCallFilter=` — could be locked to `@system-service`, but
  the binary is small enough that `seccomp` filtering adds more
  audit burden than it removes.
