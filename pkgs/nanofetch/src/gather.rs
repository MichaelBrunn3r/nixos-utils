use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::data_sources::CpuInfo;
use crate::data_sources::Fs;
use crate::data_sources::Load;
use crate::data_sources::LsblkDisk;
use crate::data_sources::Memory;
use crate::data_sources::Uptime;
use crate::data_sources::ip_addr::NetIfaces;
use crate::data_sources::net_dev::NetIfacesStats;
use crate::data_sources::read_line;

/// All system facts gathered by [`gather`].
///
/// Fields are filled in incrementally: steps 2-5 of `docs/nanofetch-plan.md`
/// each add the gather logic for one fact group. Optional fields are `None`
/// (and sections empty) until the corresponding gather step lands.
#[derive(Debug, Default, Serialize)]
pub struct Data {
    /// Pretty OS name, e.g. `NixOS 25.05` (`/etc/os-release` → `PRETTY_NAME`).
    pub os: Option<String>,
    /// Machine hostname (`/proc/sys/kernel/hostname`).
    pub hostname: Option<String>,
    /// Kernel version (`/proc/sys/kernel/osrelease`).
    pub kernel: Option<String>,
    /// Uptime duration plus the boot timestamp it was derived from.
    pub uptime: Option<Uptime>,
    /// Installed package counts per nix profile.
    pub packages: Option<Packages>,
    /// CPU brand, logical-core count, and clock frequency.
    pub cpu: Option<CpuInfo>,
    /// 1/5/15-minute load averages (`/proc/loadavg`).
    pub load: Option<Load>,
    /// Memory usage in bytes.
    pub memory: Option<Memory>,
    /// Swap usage in bytes.
    pub swap: Option<Memory>,
    /// Block devices, each with its mounted filesystems.
    pub disks: Vec<Disk>,
    /// Network interfaces with addresses, keyed by interface name.
    pub net_ifaces: NetIfaces,
    /// Cumulative per-interface byte counters (`/proc/net/dev`), keyed by
    /// interface name.
    pub net_ifaces_stats: NetIfacesStats,
}

/// Installed package counts per nix profile.
#[derive(Debug, Default, Serialize)]
pub struct Packages {
    /// Count in the system profile (`nix-system`).
    pub system: u64,
    /// Count in the user profile (`nix-user`).
    pub user: u64,
}

/// A block device with its mounted filesystems, ready to render as a tree.
#[derive(Debug, Default, Serialize)]
pub struct Disk {
    /// Device path, e.g. `/dev/nvme0n1`.
    pub path: String,
    /// Total size in bytes (the physical size, or the first filesystem's size
    /// when the device is not a physical block device).
    pub size: u64,
    /// Human-readable type, e.g. `NVMe SSD (WD_BLACK SN850X 1000GB)`; empty
    /// when the device is not a physical block device.
    pub kind: String,
    /// Mounted filesystems, as branched sub-rows.
    pub filesystems: Vec<Fs>,
}

fn join_default<T: Default>(handle: std::thread::JoinHandle<anyhow::Result<T>>) -> T {
    handle.join().ok().and_then(Result::ok).unwrap_or_default()
}

/// Gather system facts once and package them into a [`Data`].
///
/// Each fact is read and parsed independently; a failed read or parse leaves
/// that field `None` (the row just drops) rather than aborting the run.
pub fn gather() -> Data {
    let lsblk = std::thread::spawn(crate::data_sources::lsblk::collect);
    let df = std::thread::spawn(crate::data_sources::df::collect);
    let ip = std::thread::spawn(crate::data_sources::ip_addr::collect);

    let (memory, swap) = match crate::data_sources::meminfo::collect().ok() {
        Some(info) => (Some(info.memory), Some(info.swap)),
        None => (None, None),
    };
    let disks = merge_disks(&join_default(lsblk), &join_default(df));

    Data {
        os: crate::data_sources::os_release::collect().ok(),
        hostname: read_line("/proc/sys/kernel/hostname").ok(),
        kernel: read_line("/proc/sys/kernel/osrelease").ok(),
        uptime: crate::data_sources::uptime::collect().ok(),
        cpu: crate::data_sources::cpuinfo::collect().ok(),
        load: crate::data_sources::loadavg::collect().ok(),
        memory,
        swap,
        disks,
        net_ifaces: join_default(ip),
        net_ifaces_stats: crate::data_sources::net_dev::collect().unwrap_or_default(),
        ..Data::default()
    }
}

// ---- parsers (pure; unit-tested) ----

/// Merge physical disks with their mounted filesystems into a per-disk tree,
/// mirroring the reference `disk-rows`: a parent row per disk (including
/// "extra" disks that appear only in `df`), with filesystems as children
/// sorted by partition number.
fn merge_disks(disks: &[LsblkDisk], filesystems: &[Fs]) -> Vec<Disk> {
    // Group filesystems by disk name, children sorted by partition number.
    let mut groups: BTreeMap<String, Vec<&Fs>> = BTreeMap::new();
    for fs in filesystems {
        groups.entry(disk_of(&fs.source)).or_default().push(fs);
    }
    for fss in groups.values_mut() {
        fss.sort_by_key(|fs| part_num(&fs.source));
    }
    // Disk names: physical disks first, then filesystem-only disks in df order.
    let mut names: Vec<String> = disks.iter().map(|disk| disk.name.clone()).collect();
    let mut seen: BTreeSet<String> = names.iter().cloned().collect();
    for fs in filesystems {
        let name = disk_of(&fs.source);
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    names
        .into_iter()
        .map(|name| {
            let phys = disks.iter().find(|disk| disk.name == name);
            let fss = groups.get(&name).cloned().unwrap_or_default();
            let (size, kind) = phys.map_or_else(
                || (fss.first().map_or(0, |fs| fs.size), String::new()),
                |disk| (disk.size, disk_kind(disk)),
            );
            Disk {
                path: format!("/dev/{name}"),
                size,
                kind,
                filesystems: fss.into_iter().cloned().collect(),
            }
        })
        .collect()
}

/// Strip a partition suffix from a device path, e.g. `/dev/nvme0n1p2` →
/// `nvme0n1`, `/dev/sda1` → `sda`. Devices without a partition suffix (e.g.
/// `nvme0n1`, `sda`) are returned unchanged.
fn disk_of(source: &str) -> String {
    let base = source.rsplit('/').next().unwrap_or(source);
    partition_suffix(base).map_or_else(
        || base.to_owned(),
        |strip| base[..base.len() - strip].to_owned(),
    )
}

/// Length of the partition suffix at the end of a device basename, if any.
///
/// A partition suffix is `p?<digits>` following a recognized device name
/// (`nvme<n>n<n>`, `mmcblk<n>`, `vd<letters>`, `sd<letters>`), e.g. `p2` in
/// `nvme0n1p2` or `1` in `sda1`.
fn partition_suffix(base: &str) -> Option<usize> {
    let digits = base.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let end = base.len() - digits;
    let core = base[..end].strip_suffix('p').unwrap_or(&base[..end]);
    is_device_name(core).then_some(base.len() - core.len())
}

/// Split a leading run of ASCII digits from `s` into `(digits, rest)`.
fn split_digits(s: &str) -> Option<(&str, &str)> {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then_some(s.split_at(digits))
}

/// Whether `name` is a recognized device-name core (`nvme<n>n<n>`,
/// `mmcblk<n>`, `vd<letters>`, or `sd<letters>`).
fn is_device_name(name: &str) -> bool {
    // nvme<digits>n<digits>
    if let Some(rest) = name.strip_prefix("nvme") {
        let Some((_, rest)) = split_digits(rest) else {
            return false;
        };
        let Some(rest) = rest.strip_prefix('n') else {
            return false;
        };
        let Some((_, rest)) = split_digits(rest) else {
            return false;
        };
        return rest.is_empty();
    }
    // mmcblk<digits>
    if let Some(rest) = name.strip_prefix("mmcblk") {
        let Some((_, rest)) = split_digits(rest) else {
            return false;
        };
        return rest.is_empty();
    }
    // vd<letters> | sd<letters>
    name.strip_prefix("vd")
        .or_else(|| name.strip_prefix("sd"))
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_lowercase()))
}

/// Extract the trailing digit run from a device path as the partition number,
/// defaulting to `0` when there are none (mirrors the reference `part-num`).
fn part_num(source: &str) -> u64 {
    let base = source.rsplit('/').next().unwrap_or(source);
    let digits = base.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return 0;
    }
    base[base.len() - digits..].parse().unwrap_or(0)
}

/// Human-readable disk type, e.g. `NVMe SSD (WD_BLACK SN850X 1000GB)`:
/// `NVMe` → `NVMe SSD`, non-rotational → `<TRAN> SSD`, rotational → `<TRAN> HDD`;
/// the model is appended in parentheses when present.
fn disk_kind(disk: &LsblkDisk) -> String {
    let tran = disk
        .tran
        .as_deref()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let label = if tran == "NVME" {
        "NVMe SSD".to_owned()
    } else if !disk.rota {
        format!("{tran} SSD")
    } else {
        format!("{tran} HDD")
    };
    if let Some(model) = disk
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        format!("{label} ({model})")
    } else {
        label
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_of_strips_partitions() {
        let cases = [
            ("nvme partition", "/dev/nvme0n1p2", "nvme0n1"),
            ("nvme whole disk", "/dev/nvme0n1", "nvme0n1"),
            ("mmcblk partition", "/dev/mmcblk0p1", "mmcblk0"),
            ("mmcblk whole disk", "/dev/mmcblk0", "mmcblk0"),
            ("sata partition", "/dev/sda1", "sda"),
            ("sata whole disk", "/dev/sda", "sda"),
            ("virtio partition", "/dev/vda1", "vda"),
            ("virtio whole disk", "/dev/vda", "vda"),
            ("unknown device", "/dev/dm-1", "dm-1"),
            ("loop", "/dev/loop0", "loop0"),
        ];
        for (label, source, expected) in cases {
            assert_eq!(disk_of(source), expected, "case: {label}");
        }
    }

    #[test]
    fn part_num_extracts_digits() {
        let cases = [
            ("partition", "/dev/nvme0n1p2", 2),
            ("whole disk", "/dev/nvme0n1", 1),
            ("none", "/dev/vda", 0),
            ("sata partition", "/dev/sda1", 1),
        ];
        for (label, source, expected) in cases {
            assert_eq!(part_num(source), expected, "case: {label}");
        }
    }

    #[test]
    fn disk_kind_labels() {
        let disk = |rota: bool, tran: Option<&str>, model: Option<&str>| LsblkDisk {
            name: String::new(),
            size: 0,
            tran: tran.map(str::to_owned),
            rota,
            model: model.map(str::to_owned),
        };
        assert_eq!(
            disk_kind(&disk(false, Some("nvme"), Some("WD_BLACK SN850X"))),
            "NVMe SSD (WD_BLACK SN850X)"
        );
        assert_eq!(disk_kind(&disk(true, Some("sata"), None)), "SATA HDD");
        assert_eq!(disk_kind(&disk(false, Some("usb"), Some("  "))), "USB SSD");
    }

    #[test]
    fn merge_disks_tree() {
        let disks = vec![LsblkDisk {
            name: "nvme0n1".to_owned(),
            size: 1_000_204_886_016,
            tran: Some("nvme".to_owned()),
            rota: false,
            model: Some("WD_BLACK".to_owned()),
        }];
        let fss = vec![
            Fs {
                source: "/dev/nvme0n1p2".to_owned(),
                fstype: "ext4".to_owned(),
                size: 964_970_082_304,
                used: 591_121_698_816,
                mount: "/".to_owned(),
            },
            Fs {
                source: "/dev/nvme0n1p1".to_owned(),
                fstype: "vfat".to_owned(),
                size: 535_805_952,
                used: 126_922_752,
                mount: "/boot".to_owned(),
            },
        ];
        let tree = merge_disks(&disks, &fss);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].path, "/dev/nvme0n1");
        assert_eq!(tree[0].size, 1_000_204_886_016);
        assert_eq!(tree[0].kind, "NVMe SSD (WD_BLACK)");
        // Filesystems are sorted by partition number (p1 before p2).
        assert_eq!(tree[0].filesystems[0].source, "/dev/nvme0n1p1");
        assert_eq!(tree[0].filesystems[1].source, "/dev/nvme0n1p2");
        assert_eq!(tree[0].filesystems[1].mount, "/");
    }

    #[test]
    fn merge_disks_extra_filesystem_only() {
        // A filesystem whose disk is not in the lsblk list (e.g. /dev/dm-1)
        // becomes a parent disk of its own with the first fs's size and no
        // kind.
        let disks: Vec<LsblkDisk> = Vec::new();
        let fss = vec![Fs {
            source: "/dev/dm-1".to_owned(),
            fstype: "ext4".to_owned(),
            size: 964_970_082_304,
            used: 591_121_698_816,
            mount: "/".to_owned(),
        }];
        let tree = merge_disks(&disks, &fss);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].path, "/dev/dm-1");
        assert_eq!(tree[0].size, 964_970_082_304);
        assert_eq!(tree[0].kind, "");
        assert_eq!(tree[0].filesystems.len(), 1);
    }
}
