use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::time::Duration;

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

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Ok(_) => {
                if let Some(caps) = re_scsi_error.captures(&line) {
                    let device = &caps[1]; // e.g. "sdb"
                    println!("🚨 CRITICAL: Hardware disconnect/error detected on device /dev/{}!", device);
                    amputate_device(device);
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

fn amputate_device(device: &str) {
    println!("🔨 Amputating hanging mounts for /dev/{} to prevent D-state IO locks...", device);
    
    // Lazy unmount the main device (if directly mounted)
    let target = format!("/dev/{}", device);
    let _ = Command::new("umount").arg("-l").arg(&target).status();

    // Lazy unmount partitions (e.g. sdb1, sdb2)
    for i in 1..=9 {
        let part = format!("/dev/{}{}", device, i);
        let _ = Command::new("umount").arg("-l").arg(&part).status();
    }

    println!("✅ Device {} amputated. System processes like bwrap/Discord should be released from hanging.", device);
}
