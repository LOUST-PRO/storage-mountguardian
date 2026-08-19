# Getting started

This page covers the two supported install paths and how to confirm the
daemon is alive after installation. Both paths ship the same binary;
the difference is lifecycle management.

## Install via cargo

The simplest path — single command, no extra config.

```bash
cargo install storage-mountguardian
```

This places the binary at `~/.cargo/bin/storage-mountguardian`. The
binary is statically linked against the single runtime dependency
(`regex`) so no dynamic-library surprises at install time.

If you want the binary on `PATH` for non-interactive sessions, copy
or symlink it:

```bash
install -m 0755 ~/.cargo/bin/storage-mountguardian /usr/local/bin/
```

## Install as a systemd service

The recommended path for any host where the daemon should survive
reboots and restart on failure. A hardened unit file ships in the
repository at
[`contrib/storage-mountguardian.service`](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/contrib/storage-mountguardian.service).

### Step 1 — copy the unit

```bash
sudo install -m 0644 \
  contrib/storage-mountguardian.service \
  /etc/systemd/system/
```

If you installed via `cargo install` (so the binary lives at
`~/.cargo/bin/storage-mountguardian`), edit the unit's `ExecStart` to
match:

```ini
[Service]
ExecStart=/home/<your-user>/.cargo/bin/storage-mountguardian
```

Replace `<your-user>` with the user you ran `cargo install` under.

### Step 2 — enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now storage-mountguardian
```

The unit uses `Restart=always` with `RestartSec=5`, so transient
failures are absorbed without operator intervention. A
`StartLimitIntervalSec=300` + `StartLimitBurst=5` ceiling stops a
runaway restart loop from filling the journal.

### Step 3 — confirm

The unit uses `StandardOutput=journal` + `StandardError=journal` with
`SyslogIdentifier=storage-mountguardian`, so all output shows up in
the systemd journal under that identifier.

```bash
journalctl -u storage-mountguardian -f
```

You should see the daemon's startup banner:

```text
Starting Storage Guardian Daemon...
Monitoring /dev/kmsg for SCSI/USB IO errors...
```

## Verifying the daemon is alive

Three sanity checks, from cheapest to most thorough.

### 1. Process is running

```bash
systemctl is-active storage-mountguardian
# expected: active
```

### 2. Process is reading from `/dev/kmsg`

```bash
sudo ls -l /proc/$(pidof storage-mountguardian)/fd/0
# expected: ... /dev/kmsg
```

If the file descriptor points elsewhere, the daemon failed to open
`/dev/kmsg` (almost always a permissions issue — the unit runs as
`root` so this should not happen, but it's worth checking after a
non-standard install).

### 3. Process is matching kernel events

You don't need a real failing USB drive to test the regex matchers.
Trigger one synthetically by reading the current SCSI error buffer
while the daemon watches:

```bash
# In one terminal:
journalctl -u storage-mountguardian -f

# In another:
echo 1 > /sys/block/sdb/device/delete
# (only works if sdb exists; substitute your actual device)
```

The journal should immediately show an amputation line for `sdb`. If
you don't have a spare block device, skip this test — the daemon's
correctness is covered by the upstream integration tests, not by
operator manual probing.

## Uninstalling

```bash
sudo systemctl disable --now storage-mountguardian
sudo rm /etc/systemd/system/storage-mountguardian.service
sudo systemctl daemon-reload

# If installed via cargo:
cargo uninstall storage-mountguardian
```

The daemon is purely stateless — there are no config files, no
databases, no caches to wipe. Removal is fully reversible.
