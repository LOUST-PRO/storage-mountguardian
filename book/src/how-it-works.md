# How it works

`storage-mountguardian` is intentionally small — about 200 lines of
readable Rust across a single `main.rs`. This page walks the hot path
end to end so you understand exactly what runs when a USB drive dies.

## Kernel ring buffer pipeline

The daemon opens `/dev/kmsg` once and reads it in a blocking loop.
There is no polling, no `inotify`, no async runtime — the kernel
pushes log records into the ring buffer and the read returns whenever
new data is available.

```text
+-------------------------+      +----------------------+      +---------------------+
| Kernel SCSI/USB subsys  | ---> | /dev/kmsg ring buf   | ---> | storage-mountguardian|
| logs DID_NO_CONNECT,    |      | (device node,        |      | BufReader::read_line |
| USB disconnect events   |      |  blocking reads)     |      | + regex matchers    |
+-------------------------+      +----------------------+      +----------+----------+
                                                                         |
                                                                         | match?
                                                                         v
                                                              +----------+----------+
                                                              | enumerate /sys/block |
                                                              | partitions           |
                                                              +----------+----------+
                                                                         |
                                                                         v
                                                              +----------+----------+
                                                              | umount -l <mount>   |
                                                              | (one per partition) |
                                                              +---------------------+
```

Two regex matchers run on every record:

| Pattern | Regex |
|---|---|
| SCSI disconnect on a block device | `\[([a-z0-9]+)\].*(DID_NO_CONNECT\|DID_BAD_TARGET\|DID_ERROR)` |
| Generic USB disconnect | `usb .* USB disconnect` |

The first captures the device name (`sdb`, `nvme0n1`, `mmcblk0`, …) into
group 1; the second is a coarse fallback for USB stacks that don't
emit a SCSI-level error.

## Lazy unmount semantics

A **lazy** unmount (`umount -l`) detaches the filesystem from the
mount-tree hierarchy immediately but allows existing references (open
file descriptors, working directories, mmap'd regions) to continue
until they close naturally. This is the only safe call here:

- A regular `umount` would block waiting for the references to drop —
  except the references won't drop because the device is dead.
- A force `umount -f` would corrupt any active writes mid-flight.

`umount -l` says "you're no longer mounted, but the inode references
keep working until the kernel cleans them up". Combined with the
kernel breaking the IO wait loop the instant the mount point vanishes
from the namespace, stuck processes wake from D-state within
milliseconds rather than minutes.

## Rate-limit dedup (v0.2.0+)

A flaky USB drive can emit hundreds of `DID_NO_CONNECT` events per
second. Without dedup, the daemon would issue a `umount -l` against
the same mount point on every event — wasted syscalls and a flood of
journal entries.

v0.2.0 introduces a 5-second dedup window per device:

```rust
const DEDUP_WINDOW: Duration = Duration::from_secs(5);

let mut last_amputated: HashMap<String, Instant> = HashMap::new();
// ... in the read loop:
if let Some(prev) = last_amputated.get(&device) {
    if prev.elapsed() < DEDUP_WINDOW {
        continue; // skip re-amputation
    }
}
last_amputated.insert(device.clone(), Instant::now());
// proceed with umount -l ...
```

A second flap within 5 seconds is silently absorbed. The window is
deliberately small — a real recovery (cable reseated) takes longer
than 5 seconds, and a permanent failure should not be re-tried inside
the window because the mount point is already gone.

## Sysfs partition discovery (v0.2.0+)

A block device like `sdb` is rarely mounted directly; in practice the
partitions (`sdb1`, `sdb2`, …) carry the mount points. v0.2.0 reads
`/sys/block/<dev>/` to enumerate them rather than guessing from the
device name.

```rust
// Pseudocode
for entry in fs::read_dir(format!("/sys/block/{}/", device))? {
    let name = entry.file_name();
    if name.starts_with(&device) {
        // partition candidate — try to umount -l it
    }
}
```

This correctly handles the naming conventions of `sd*` (`sdb1`,
`sdb2`), `nvme*` (`nvme0n1p1`, `nvme0n1p2`), `mmcblk*` (`mmcblk0p1`),
and any other block device family that exposes partitions under
`/sys/block/<parent>/`.

## Allocation bound (v0.2.0+)

`/dev/kmsg` is normally well-behaved, but a buggy kernel module or a
corrupted ring buffer could emit unbounded record sizes. To defend
against runaway allocation, the read loop enforces a hard ceiling:

```rust
const MAX_LINE_BYTES: usize = 64 * 1024;

if line.len() > MAX_LINE_BYTES {
    eprintln!(
        "warning: dropping oversized kmsg record ({} bytes)",
        line.len()
    );
    continue;
}
```

Any record larger than 64 KiB is dropped with a single warning line
and the daemon continues. This protects the daemon (not the kernel)
from being weaponised into a memory-exhaustion DoS.
