use std::process::Command;

use anyhow::Context;
use serde::Deserialize;

/// A physical block device as reported by `lsblk -J`.
#[derive(Debug, Deserialize)]
pub struct LsblkDisk {
    /// Device name, e.g. `nvme0n1`.
    pub name: String,
    /// Total size in bytes.
    pub size: u64,
    /// Transport, e.g. `nvme`/`sata`/`usb`; `None` when unknown
    /// (loop/zram/dm).
    #[serde(default)]
    pub tran: Option<String>,
    /// Whether the device is rotational.
    pub rota: bool,
    /// Device model; `None` when the device reports none.
    #[serde(default)]
    pub model: Option<String>,
}

/// Collect physical block devices via `lsblk`, mirroring the reference
/// `add-disks`: `lsblk -b -J -d -o NAME,SIZE,TRAN,ROTA,MODEL`.
pub fn collect() -> anyhow::Result<Vec<LsblkDisk>> {
    let out = Command::new("lsblk")
        .args(["-b", "-J", "-d", "-o", "NAME,SIZE,TRAN,ROTA,MODEL"])
        .output()
        .context("run `lsblk -b -J -d`")?;
    if !out.status.success() {
        let status = out.status;
        anyhow::bail!("`lsblk -b -J -d` exited with {status}");
    }
    let Some(disks) = parse(&String::from_utf8_lossy(&out.stdout)) else {
        log::debug!("failed to parse `lsblk -b -J -d` output");
        anyhow::bail!("failed to parse `lsblk -b -J -d` output");
    };
    Ok(disks)
}

/// Parse `lsblk -b -J -d` JSON into physical block devices, skipping entries
/// with no transport (loop/zram/dm). Devices are sorted by name.
fn parse(json: &str) -> Option<Vec<LsblkDisk>> {
    #[derive(Deserialize)]
    struct LsblkRoot {
        blockdevices: Vec<LsblkDisk>,
    }
    let root: LsblkRoot = serde_json::from_str(json).ok()?;
    let mut out: Vec<LsblkDisk> = root
        .blockdevices
        .into_iter()
        .filter(|disk| disk.tran.as_deref().is_some_and(|tran| !tran.is_empty()))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lsblk_filters_and_sorts() {
        let json = r#"{
            "blockdevices": [
                {"name": "loop0", "size": 1234, "tran": null, "rota": false, "model": null},
                {"name": "sda", "size": 250059350016, "tran": "usb", "rota": false, "model": "250GB Card"},
                {"name": "nvme0n1", "size": 1000204886016, "tran": "nvme", "rota": false, "model": "WD_BLACK SN850X 1000GB"}
            ]
        }"#;
        let disks = parse(json).expect("valid json");
        // `loop0` is dropped (no transport); the rest are sorted by name.
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].name, "nvme0n1");
        assert_eq!(disks[0].size, 1_000_204_886_016);
        assert_eq!(disks[0].tran.as_deref(), Some("nvme"));
        assert!(!disks[0].rota);
        assert_eq!(disks[1].name, "sda");
        assert_eq!(disks[1].model.as_deref(), Some("250GB Card"));
    }

    #[test]
    fn parse_lsblk_empty_transport_string() {
        // `tran` may be emitted as an empty string rather than null.
        let json = r#"{
            "blockdevices": [
                {"name": "zram0", "size": 1, "tran": "", "rota": false, "model": null},
                {"name": "sda", "size": 1000, "tran": "sata", "rota": true, "model": "D"}
            ]
        }"#;
        let disks = parse(json).expect("valid json");
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].name, "sda");
        assert!(disks[0].rota);
    }

    #[test]
    fn parse_lsblk_garbage() {
        assert!(parse("not json").is_none());
    }
}
