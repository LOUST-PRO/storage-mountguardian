use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

// Rate-limit dedup window: same device won't be re-amputated within this window.
const DEDUP_WINDOW: Duration = Duration::from_secs(5);
// Maximum line size we'll buffer from /dev/kmsg. Defensive bound against runaway
// allocation if a buggy kmsg source emits unbounded records.
const MAX_LINE_BYTES: usize = 64 * 1024;

fn main() {
    println!("Starting Storage Guardian Daemon...");
    println!("Monitoring /dev/kmsg for SCSI/USB IO errors...");

    // Regex to match SCSI errors related to block devices, e.g. "sd 1:0:0:0: [sdb] Synchronize Cache(10) failed: Result: hostbyte=DID_NO_CONNECT"
    let re_scsi_error = Regex::new(r"\[([a-z0-9]+)\].*(DID_NO_CONNECT|DID_BAD_TARGET|DID_ERROR)").unwrap();
    let re_usb_disconnect = Regex::new(r"usb .* USB disconnect").unwrap();

    let file = match File::open("/dev/kmsg") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open /dev/kmsg (Are you running with sudo?): {}", e);
            std::process::exit(1);
        }
    };

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    // device -> last amputate time, for rate-limit dedup
    let mut last_amputated: HashMap<String, Instant> = HashMap::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Ok(_) => {
                // Defensive bound: if a record exceeds MAX_LINE_BYTES, drop it
                // and continue. Prevents unbounded allocation if kmsg misbehaves.
                if line.len() > MAX_LINE_BYTES {
                    eprintln!(
                        "warning: dropping oversized kmsg record ({} bytes)",
                        line.len()
                    );
                    continue;
                }
                if let Some(caps) = re_scsi_error.captures(&line) {
                    let device = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    if !device.is_empty() {
                        let now = Instant::now();
                        // Dedup: skip if we amputated this device within window
                        if let Some(last) = last_amputated.get(device) {
                            if now.duration_since(*last) < DEDUP_WINDOW {
                                continue;
                            }
                        }
                        last_amputated.insert(device.to_string(), now);
                        // Opportunistic cleanup of stale entries (>5 min old)
                        last_amputated.retain(|_, t| now.duration_since(*t) < Duration::from_secs(300));
                        println!("🚨 CRITICAL: Hardware disconnect/error detected on device /dev/{}!", device);
                        amputate_device(device);
                    }
                } else if re_usb_disconnect.is_match(&line) {
                    println!("⚠️ WARNING: Unclean USB disconnect detected in kernel ring buffer.");
                }
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// Enumerate the actual partitions of a block device by reading /sys/block/<dev>/.
/// Returns partition device paths (e.g. ["/dev/sdb1", "/dev/sdb2"]) for sd-style
/// devices, and ["/dev/nvme0n1p1", "/dev/nvme0n1p2"] for nvme-style. Returns
/// an empty Vec if the device directory cannot be read.
fn discover_partitions(device: &str) -> Vec<String> {
    let dir = Path::new("/sys/block").join(device);
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut parts: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // Partition names start with the device prefix and are not the device itself.
            // For sd*: sdb1, sdb2 ... For nvme*: nvme0n1p1, nvme0n1p2 ...
            if name.starts_with(device) && name != device {
                Some(format!("/dev/{}", name))
            } else {
                None
            }
        })
        .collect();
    parts.sort();
    parts
}

fn amputate_device(device: &str) {
    println!("🔨 Amputating hanging mounts for /dev/{} to prevent D-state IO locks...", device);

    // Lazy unmount the main device (if directly mounted)
    let target = format!("/dev/{}", device);
    let _ = Command::new("umount").arg("-l").arg(&target).status();

    // Discover actual partitions via sysfs (handles sd*, nvme*, mmcblk* correctly).
    let parts = discover_partitions(device);
    if parts.is_empty() {
        // Fallback: probe sdx1..9 for sd-style devices where sysfs isn't ready
        for i in 1..=9 {
            let part = format!("/dev/{}{}", device, i);
            let _ = Command::new("umount").arg("-l").arg(&part).status();
        }
    } else {
        for part in &parts {
            let _ = Command::new("umount").arg("-l").arg(part).status();
        }
    }

    println!("✅ Device {} amputated. System processes like bwrap/Discord should be released from hanging.", device);
}
