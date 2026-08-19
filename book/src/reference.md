# Reference

Compact lookup for the daemon's source layout, CLI surface, and exit
codes.

## Source layout

The repository is intentionally flat — no `src/lib.rs`, no
sub-crates, no workspaces. One binary, one file.

```text
storage-mountguardian/
├── Cargo.toml          # 7-field metadata profile
├── Cargo.lock
├── LICENSE             # Apache-2.0
├── LICENSE-FORK.md     # Hardening Addendum (this fork)
├── README.md           # Quick-start + features
├── CONTRIBUTING.md     # Workflow rules
├── SECURITY.md         # Disclosure process
├── src/
│   └── main.rs         # ~200 lines, the entire daemon
└── contrib/
    └── storage-mountguardian.service
```

`src/main.rs` is organised into:

| Region | Lines | Purpose |
|---|---|---|
| Constants | `DEDUP_WINDOW`, `MAX_LINE_BYTES` | Tunables (compile-time) |
| Matchers | `re_scsi_error`, `re_usb_disconnect` | Regex patterns |
| Setup | `File::open("/dev/kmsg")` | One-time open |
| State | `last_amputated: HashMap<String, Instant>` | Dedup map |
| Loop | `reader.read_line(...)` | Hot path |

There are no modules, no `mod` declarations, no `pub use` — the
binary is the crate.

## CLI flags

The daemon currently has **no CLI flags**. Configuration is done
via:

- Compile-time constants (`DEDUP_WINDOW`, `MAX_LINE_BYTES`) — change
  in `src/main.rs`, rebuild.
- systemd unit overrides — for environment-specific paths (binary
  location, journal identifier).

This is intentional. The daemon is a single-purpose tool, not a
configurable framework. If you need customisable thresholds, fork
the repo and edit the constants — the upstream prefers readable
source over a flags matrix.

Future flags are tracked in the upstream issue tracker; nothing is
planned for v0.3.x.

## Exit codes

The daemon is designed to run forever, so exit codes are mostly
relevant to the systemd supervisor:

| Code | Trigger | systemd reaction |
|---|---|---|
| `0` | `std::process::exit(0)` (deliberate clean exit) | Stopped, not restarted |
| `1` | `File::open("/dev/kmsg")` failed at startup (almost always a permissions issue — the unit runs as `root` so this should not happen) | Restarted (under `Restart=always`) |
| `101` (Rust panic) | Unrecoverable internal error | Restarted after `RestartSec=5` |
| Killed by SIGTERM | `systemctl stop` | Clean stop |

The daemon does not trap signals explicitly — it relies on systemd's
default behaviour of sending SIGTERM on `systemctl stop` and SIGKILL
after the default `TimeoutStopSec=90`.

## Constants reference

| Constant | Value | Effect of changing |
|---|---|---|
| `DEDUP_WINDOW` | `Duration::from_secs(5)` | Increases → fewer redundant `umount` calls on flapping devices, but slower response to real recovery. Decreases → noisier journal on flaky hardware. |
| `MAX_LINE_BYTES` | `64 * 1024` | Increases → more permissive on odd kernels, more memory risk. Decreases → drops legitimate large records. |

Both are conservative defaults that have not needed adjustment in
the canonical deployment; treat them as a starting point if you fork.
