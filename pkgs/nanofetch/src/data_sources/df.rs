use std::collections::BTreeSet;
use std::process::Command;

use anyhow::Context;
use serde::Serialize;

/// A mounted filesystem on a disk.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Fs {
    /// Device path, e.g. `/dev/nvme0n1p2`.
    pub source: String,
    /// Filesystem type, e.g. `ext4`.
    pub fstype: String,
    /// Total size in bytes.
    pub size: u64,
    /// Used bytes.
    pub used: u64,
    /// Mount point.
    pub mount: String,
}

/// Filesystem types that are not real storage. Passed to `df` as `-x` so it
/// never `statfs`es them, and dropped defensively again by [`parse`]
/// (mirrors the reference `add-disks` filter).
const SKIP_FS_TYPES: &[&str] = &[
    "tmpfs", "devtmpfs", "efivarfs", "overlay", "squashfs", "ramfs",
];

/// Collect mounted filesystems via `df`, mirroring the reference `add-disks`:
/// `df -T -P -B1`, excluding the non-storage types in [`SKIP_FS_TYPES`] with
/// `-x` so `df` doesn't `statfs` filesystems whose rows are discarded anyway.
pub fn collect() -> anyhow::Result<Vec<Fs>> {
    let mut cmd = Command::new("df");
    cmd.args(["-T", "-P", "-B1"]);
    for fs_type in SKIP_FS_TYPES {
        cmd.args(["-x", fs_type]);
    }
    let out = cmd.output().context("run `df -T -P -B1`")?;
    if !out.status.success() {
        let status = out.status;
        anyhow::bail!("`df -T -P -B1` exited with {status}");
    }
    Ok(parse(&String::from_utf8_lossy(&out.stdout)))
}

/// Parse `df -T -P -B1` output into mounted filesystems, applying the shared
/// filters: drop non-storage filesystem types, keep only non-loop `/dev/*`
/// sources, and dedup by source (first occurrence wins).
fn parse(content: &str) -> Vec<Fs> {
    let mut out: Vec<Fs> = content
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 6 {
                return None;
            }
            Some(Fs {
                source: fields[0].to_owned(),
                fstype: fields[1].to_owned(),
                size: fields[2].parse().ok()?,
                used: fields[3].parse().ok()?,
                mount: fields[6..].join(" "),
            })
        })
        .filter(|fs| !SKIP_FS_TYPES.contains(&fs.fstype.as_str()))
        .filter(|fs| fs.source.starts_with("/dev/") && !fs.source.contains("loop"))
        .collect();
    let mut seen = BTreeSet::new();
    out.retain(|fs| seen.insert(fs.source.clone()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_df_filters_and_dedups() {
        const CONTENT: &str = "\
Filesystem     Type         1-blocks         Used    Available Capacity Mounted on
/dev/dm-1      ext4     964970082304 591121698816 324755161088      65% /
tmpfs          tmpfs      8336830464      8851456   8327979008       1% /run
devtmpfs       devtmpfs   1667366912            0   1667366912       0% /dev
efivarfs       efivarfs       274344       141608       127616      53% /sys/firmware/efi/efivars
none           tmpfs         1048576            0      1048576       0% /run/credentials
/dev/loop0     squashfs   123456789    100000000    23456789      81% /snap/foo
/dev/nvme0n1p1 vfat        535805952    126922752    408883200      24% /boot
/dev/nvme0n1p1 vfat        535805952    126922752    408883200      24% /boot2
/dev/sda3      ntfs3    249651400704  66869485568 182781915136      27% /run/media/WIN11
";
        let fs = parse(CONTENT);
        // tmpfs/devtmpfs/efivarfs dropped, loop dropped, duplicate source
        // deduped to the first occurrence.
        let sources: Vec<&str> = fs.iter().map(|f| f.source.as_str()).collect();
        assert_eq!(sources, ["/dev/dm-1", "/dev/nvme0n1p1", "/dev/sda3"]);
        assert_eq!(fs[1].fstype, "vfat");
        assert_eq!(fs[1].size, 535_805_952);
        assert_eq!(fs[1].used, 126_922_752);
        assert_eq!(fs[1].mount, "/boot");
    }

    #[test]
    fn parse_df_mount_with_spaces() {
        // `df -P` escapes spaces in mount points as `\040`, kept verbatim
        // (matches the reference, which joins fields without unescaping).
        let content = "\
Filesystem Type 1-blocks Used Available Capacity Mounted on
/dev/sda3 ntfs3 100 10 90 10% /run/media/foo\\040bar
";
        let fs = parse(content);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].mount, "/run/media/foo\\040bar");
    }
}
