use chrono::{Local, TimeDelta};
use tabled::builder::Builder;
use tabled::settings::object::{Columns, Rows};
use tabled::settings::{Color, Modify, Padding, Style};

use crate::data_sources::Memory;
use crate::gather::Data;
use crate::percentage::PercentageLevel;

/// Render the gathered data: the top facts as a `key: value` table, with the
/// `Disks` and `Net` sections appended below when present.
///
/// When `colored`, the label (first) column is green; usage percentages
/// (Memory/Swap) and CPU load averages are colored by the shared threshold
/// schema (green/yellow/red bold).
#[must_use]
pub fn render(data: &Data, colored: bool) -> String {
    let mut out = render_table(data, colored);
    if !data.disks.is_empty() {
        out.push('\n');
        out.push_str(&render_disks(data, colored));
    }
    if !data.net_ifaces.is_empty() {
        out.push('\n');
        out.push_str(&render_net(data, colored));
    }
    out
}

/// Render the top facts (OS → Swap) as a two-column `key: value` table.
///
/// When `colored`, the label (first) column is green.
fn render_table(data: &Data, colored: bool) -> String {
    let mut builder = Builder::default();

    if let Some(os) = &data.os {
        builder.push_record(["OS", os.as_str()]);
    }
    if let Some(hostname) = &data.hostname {
        builder.push_record(["Host", hostname.as_str()]);
    }
    if let Some(kernel) = &data.kernel {
        builder.push_record(["Kernel", kernel.as_str()]);
    }
    if let Some(uptime) = &data.uptime {
        let human = fmt_duration(uptime.duration);
        let boot = uptime
            .booted_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M");
        builder.push_record(["Uptime", format!("{human} ({boot})").as_str()]);
    }
    if let Some(cpu) = &data.cpu {
        let value = format!("{} @ {:.2} GHz, {} Cores", cpu.brand, cpu.ghz, cpu.count);
        builder.push_record(["CPU", value.as_str()]);
    }
    if let Some(load) = &data.load {
        let count = data.cpu.as_ref().map_or(1, |cpu| cpu.count);
        let load = format!(
            "1m: {}, 5m: {}, 15m: {}",
            fmt_load(load.one, count, colored),
            fmt_load(load.five, count, colored),
            fmt_load(load.fifteen, count, colored),
        );
        builder.push_record(["Load", load.as_str()]);
    }
    if let Some(memory) = &data.memory {
        builder.push_record(["Memory", fmt_usage(memory, colored).as_str()]);
    }
    if let Some(swap) = &data.swap {
        builder.push_record(["Swap", fmt_usage(swap, colored).as_str()]);
    }

    let mut table = builder.build();
    table
        .with(Style::blank())
        .with(Padding::new(0, 0, 0, 0))
        .with(Modify::new(Columns::first()).with(Padding::new(0, 0, 0, 0)));
    if colored {
        table.with(Modify::new(Columns::first()).with(Color::FG_GREEN));
    }
    table.to_string()
}

/// Format a duration as a compact human-readable string, e.g. `6d 5h 49s`.
///
/// Non-zero components are emitted most-significant-first, space separated,
/// omitting zero components. A zero duration renders as `0s`.
#[must_use]
pub fn fmt_duration(duration: TimeDelta) -> String {
    let total = u64::try_from(duration.num_seconds()).unwrap_or_default();
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;
    let seconds = total % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }
    if parts.is_empty() {
        "0s".to_owned()
    } else {
        parts.join(" ")
    }
}

/// The binary unit of a byte count and its byte multiplier: `B`, `KiB`, `MiB`,
/// `GiB`, or `TiB`.
const fn size_unit(bytes: u64) -> (&'static str, u64) {
    const KIB: u64 = 1 << 10;
    const MIB: u64 = 1 << 20;
    const GIB: u64 = 1 << 30;
    const TIB: u64 = 1 << 40;

    if bytes < KIB {
        ("B", 1)
    } else if bytes < MIB {
        ("KiB", KIB)
    } else if bytes < GIB {
        ("MiB", MIB)
    } else if bytes < TIB {
        ("GiB", GIB)
    } else {
        ("TiB", TIB)
    }
}

/// Format a byte count as a compact binary size, e.g. `12.0 GiB`.
///
/// Bytes below 1 KiB render as an integer (e.g. `512 B`), KiB and above render with
/// `decimals` decimal places. A zero count renders as `0 B`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // byte count → f64 for display scaling
pub fn fmt_filesize(bytes: u64, decimals: usize) -> String {
    let (unit, multiplier) = size_unit(bytes);
    let value = if multiplier == 1 {
        bytes.to_string() // integer `B`
    } else {
        format!("{:.decimals$}", bytes as f64 / multiplier as f64)
    };
    format!("{value} {unit}")
}

/// Format a used/total byte pair as `used / total (pct%)`, e.g.
/// `12.0 GiB / 32.0 GiB (38%)`. The percentage is rounded to a whole number
/// and, when `colored`, wrapped in the shared usage color.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)] // byte counts → f64 → whole-number percentage for display
pub fn fmt_usage(mem: &Memory, colored: bool) -> String {
    let pct = if mem.total > 0 {
        (mem.used as f64 / mem.total as f64 * 100.0).round() as u64
    } else {
        0
    };
    format!(
        "{} / {} ({})",
        fmt_filesize(mem.used, 1),
        fmt_filesize(mem.total, 1),
        color_usage(pct, &format!("{pct}%"), colored),
    )
}

/// Format a used/total byte pair in a single shared unit — the unit of the
/// larger (total) value — e.g. `121.0 / 511.0 MiB (24%)`. The percentage is
/// rounded to a whole number and colored by the shared usage schema.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)] // byte counts → f64 → whole-number percentage for display
fn fmt_fs_usage(used: u64, total: u64, colored: bool) -> String {
    let pct = if total > 0 {
        (used as f64 / total as f64 * 100.0).round() as u64
    } else {
        0
    };
    let (unit, multiplier) = size_unit(total);
    let used = format!("{:.1}", used as f64 / multiplier as f64);
    let total = format!("{:.1}", total as f64 / multiplier as f64);
    format!(
        "{used} / {total} {unit} ({})",
        color_usage(pct, &format!("{pct}%"), colored),
    )
}

/// Wrap `text` in the shared usage color for a percentage (0..=100), matching
/// the reference `sysinfo.nu`: `red_bold` at >=90, `yellow_bold` at >=75,
/// `green_bold` otherwise. Returns `text` unchanged when `colored` is false.
#[must_use]
fn color_usage(pct: u64, text: &str, colored: bool) -> String {
    if !colored {
        return text.to_owned();
    }
    let percentage = f64::from(u32::try_from(pct).unwrap_or(u32::MAX));
    let color = match PercentageLevel::from_percentage(percentage) {
        PercentageLevel::Normal => "\x1b[1;32m",
        PercentageLevel::Warning => "\x1b[1;33m",
        PercentageLevel::Critical => "\x1b[1;31m",
    };
    format!("{color}{text}\x1b[0m")
}

/// Format a load-average value as a fixed two-decimal number, colored by the
/// shared usage schema with the load normalized against `count` CPUs
/// (`pct = value / count * 100`).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // normalized load → whole-number percentage for display
pub fn fmt_load(value: f64, count: u32, colored: bool) -> String {
    let pct = (value / f64::from(count) * 100.0).round() as u64;
    color_usage(pct, &format!("{value:.2}"), colored)
}

/// GiB in bytes.
const GIB: u64 = 1 << 30;

/// Format a byte count as GiB with two decimals, e.g. `4.40`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // byte count → f64 for display
pub fn fmt_gib(bytes: u64) -> String {
    format!("{:.2}", bytes as f64 / GIB as f64)
}

/// A section header line, e.g. `Disks`, colored green when `colored`.
fn section_header(title: &str, colored: bool) -> String {
    if colored {
        format!("\x1b[32m{title}\x1b[0m\n")
    } else {
        format!("{title}\n")
    }
}

/// Render the `Disks` section: a `Disks` header line followed by the
/// `Path  Usage  Type  Mount` sub-table, one row per disk with each disk's
/// filesystems as branched sub-rows (`├─`/`└─`). The parent usage cell shows
/// the disk size; filesystem usage cells show `used / total (pct%)` in a
/// single shared unit (the total's), colored by the shared usage schema.
fn render_disks(data: &Data, colored: bool) -> String {
    let mut out = section_header("Disks", colored);
    let mut builder = Builder::default();
    builder.push_record(["Path", "Usage", "Type", "Mount"]);
    for disk in &data.disks {
        let size = fmt_filesize(disk.size, 1);
        builder.push_record([disk.path.as_str(), size.as_str(), disk.kind.as_str(), ""]);
        let last = disk.filesystems.len().saturating_sub(1);
        for (index, fs) in disk.filesystems.iter().enumerate() {
            let branch = if index == last { "└─" } else { "├─" };
            let source = format!("{branch}{}", fs.source);
            let usage = fmt_fs_usage(fs.used, fs.size, colored);
            builder.push_record([
                source.as_str(),
                usage.as_str(),
                fs.fstype.as_str(),
                fs.mount.as_str(),
            ]);
        }
    }
    let mut table = builder.build();
    table.with(Style::blank()).with(Padding::new(0, 0, 0, 0));
    if colored {
        table.with(Modify::new(Rows::first()).with(Color::FG_CYAN));
    }
    out.push_str(&table.to_string());
    out
}

/// Render the `Net` section: a `Net` header line followed by the
/// Render the `Net` section: a `Net` header line followed by the
/// `Interface  MAC  ↓↑ GiB` sub-table, one row per interface with each
/// interface's addresses as branched sub-rows (`├─`/`└─`). The traffic cell
/// shows the cumulative `rx / tx` counters in GiB from `/proc/net/dev`.
fn render_net(data: &Data, colored: bool) -> String {
    let mut out = section_header("Net", colored);
    let mut builder = Builder::default();
    builder.push_record(["Interface", "MAC", "↓↑ GiB"]);
    for (name, iface) in &data.net_ifaces {
        let mac = iface.mac.as_deref().unwrap_or("");
        let traffic = data
            .net_ifaces_stats
            .get(name)
            .map(|stats| format!("{} / {}", fmt_gib(stats.rx), fmt_gib(stats.tx)))
            .unwrap_or_default();
        builder.push_record([name.as_str(), mac, traffic.as_str()]);
        let last = iface.ips.len().saturating_sub(1);
        for (index, ip) in iface.ips.iter().enumerate() {
            let branch = if index == last { "└─" } else { "├─" };
            let cell = format!("{branch}{ip}");
            builder.push_record([cell.as_str(), "", ""]);
        }
    }
    let mut table = builder.build();
    table.with(Style::blank()).with(Padding::new(0, 0, 0, 0));
    if colored {
        table.with(Modify::new(Rows::first()).with(Color::FG_CYAN));
    }
    out.push_str(&table.to_string());
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::data_sources::ip_addr::NetIface;
    use crate::data_sources::net_dev::NetIfaceStats;
    use crate::data_sources::{CpuInfo, Fs, Load, Memory, Uptime};
    use crate::gather::Disk;

    fn data1() -> Data {
        Data {
            os: Some("NixOS 25.05".to_owned()),
            hostname: Some("coolpc".to_owned()),
            kernel: Some("6.12.8".to_owned()),
            uptime: Some(Uptime {
                duration: TimeDelta::try_days(3).unwrap_or_default()
                    + TimeDelta::try_hours(4).unwrap_or_default()
                    + TimeDelta::try_minutes(5).unwrap_or_default(),
                booted_at: Local
                    .with_ymd_and_hms(2026, 8, 28, 10, 0, 0)
                    .single()
                    .unwrap_or_default()
                    .with_timezone(&Utc),
            }),
            packages: None, // step 3
            cpu: Some(CpuInfo {
                brand: "AMD Ryzen 7 5800X".to_owned(),
                count: 16,
                ghz: 4.7,
            }),
            load: Some(Load {
                one: 3.45,
                five: 3.60,
                fifteen: 3.80,
            }),
            memory: Some(Memory {
                used: 12_884_901_888,
                total: 34_359_738_368,
            }),
            swap: Some(Memory {
                used: 0,
                total: 8_589_934_592,
            }),
            disks: Vec::new(),
            net_ifaces: BTreeMap::new(),
            net_ifaces_stats: BTreeMap::new(),
        }
    }

    #[test]
    fn render_data1() {
        insta::assert_snapshot!("default", &render_table(&data1(), false));
    }

    #[test]
    fn format_duration() {
        let m = |v: i64| TimeDelta::minutes(v);
        let h = |v: i64| TimeDelta::hours(v);
        let d = |v: i64| TimeDelta::days(v);
        let s = |v: i64| TimeDelta::seconds(v);

        let cases = [
            ("zero", TimeDelta::zero(), "0s"),
            ("m s", m(5) + s(45), "5m 45s"),
            ("h m", h(2) + m(3), "2h 3m"),
            ("d h m s", d(2) + h(3) + m(4) + s(5), "2d 3h 4m 5s"),
            // Exact
            ("exactly 45 seconds", s(45), "45s"),
            ("exactly 5 minutes", m(5), "5m"),
            ("exactly 3 hours", h(3), "3h"),
            ("exactly 2 days", d(2), "2d"),
            // Gaps: `!` marks omitted components
            ("d !h !m s", d(2) + s(5), "2d 5s"),
            ("d !h m s", d(2) + m(4) + s(5), "2d 4m 5s"),
            ("d h !m s", d(2) + h(3) + s(5), "2d 3h 5s"),
            ("d h m !s", d(2) + h(3) + m(4), "2d 3h 4m"),
        ];
        for (label, duration, expected) in cases {
            assert_eq!(fmt_duration(duration), expected, "case: {label}");
        }
    }

    #[test]
    fn format_filesize() {
        let cases = [
            ("zero", 0, 1, "0 B"),
            ("bytes", 512, 1, "512 B"),
            ("1 KiB", 1 << 10, 1, "1.0 KiB"),
            ("1.5 KiB", 1536, 1, "1.5 KiB"),
            ("1 MiB", 1 << 20, 1, "1.0 MiB"),
            ("12 GiB", 12_884_901_888, 1, "12.0 GiB"),
            ("32 GiB", 34_359_738_368, 1, "32.0 GiB"),
            ("8 GiB", 8_589_934_592, 1, "8.0 GiB"),
            ("1 TiB", 1_u64 << 40, 1, "1.0 TiB"),
            // Decimals are configurable; < 1 KiB stays an integer
            ("0 decimals", 1 << 10, 0, "1 KiB"),
            ("2 decimals", 1536, 2, "1.50 KiB"),
            ("0 decimals bytes", 512, 0, "512 B"),
        ];
        for (label, bytes, decimals, expected) in cases {
            assert_eq!(fmt_filesize(bytes, decimals), expected, "case: {label}");
        }
    }

    #[test]
    fn format_usage() {
        let cases = [
            (
                "zero used",
                Memory {
                    used: 0,
                    total: 8_589_934_592,
                },
                "0 B / 8.0 GiB (0%)",
            ),
            (
                "half",
                Memory {
                    used: 16_777_216,
                    total: 33_554_432,
                },
                "16.0 MiB / 32.0 MiB (50%)",
            ),
            (
                "rounds up",
                Memory {
                    used: 12_884_901_888,
                    total: 34_359_738_368,
                },
                "12.0 GiB / 32.0 GiB (38%)",
            ),
            ("zero total", Memory { used: 0, total: 0 }, "0 B / 0 B (0%)"),
        ];
        for (label, mem, expected) in cases {
            assert_eq!(fmt_usage(&mem, false), expected, "case: {label}");
        }
    }

    #[test]
    fn format_fs_usage() {
        let cases = [
            (
                "same unit",
                126_922_752, // 121.0 MiB
                535_805_952, // 511.0 MiB
                "121.0 / 511.0 MiB (24%)",
            ),
            (
                "same unit gib",
                591_121_698_816, // 550.5 GiB
                964_970_082_304, // 898.7 GiB
                "550.5 / 898.7 GiB (61%)",
            ),
            (
                "total unit wins",
                10_485_760,    // 10 MiB
                1_610_612_736, // 1.5 GiB
                "0.0 / 1.5 GiB (1%)",
            ),
            ("zero total", 0, 0, "0.0 / 0.0 B (0%)"),
        ];
        for (label, used, total, expected) in cases {
            assert_eq!(fmt_fs_usage(used, total, false), expected, "case: {label}");
        }
    }

    #[test]
    fn format_load() {
        let cases = [
            ("zero", 0.0, 16, "0.00"),
            ("whole", 8.0, 16, "8.00"),
            ("half", 3.5, 16, "3.50"),
            ("large", 123.0, 16, "123.00"),
        ];
        for (label, value, count, expected) in cases {
            assert_eq!(fmt_load(value, count, false), expected, "case: {label}");
        }
    }

    #[test]
    fn format_gib() {
        let gib = 1_u64 << 30;
        let cases = [
            ("zero", 0, "0.00"),
            ("one gib", gib, "1.00"),
            ("one and a half", gib + gib / 2, "1.50"),
            ("two decimals", 1_234_567_890, "1.15"),
        ];
        for (label, bytes, expected) in cases {
            assert_eq!(fmt_gib(bytes), expected, "case: {label}");
        }
    }

    fn data_with_net() -> Data {
        let mut data = data1();
        data.net_ifaces = BTreeMap::from([
            (
                "enp3s0".to_owned(),
                NetIface {
                    mac: Some("aa:bb:cc:dd:ee:ff".to_owned()),
                    ips: vec!["192.168.1.5".to_owned(), "fe80::1".to_owned()],
                },
            ),
            (
                "wlp0".to_owned(),
                NetIface {
                    mac: Some("88:d8:2e:99:cf:55".to_owned()),
                    ips: vec!["192.168.178.40".to_owned()],
                },
            ),
        ]);
        data.net_ifaces_stats = BTreeMap::from([
            (
                "enp3s0".to_owned(),
                NetIfaceStats {
                    rx: 6_979_321_856, // 6.5 GiB
                    tx: 2_147_483_648, // 2.0 GiB
                },
            ),
            (
                "wlp0".to_owned(),
                NetIfaceStats {
                    rx: 1_073_741_824, // 1.0 GiB
                    tx: 0,
                },
            ),
        ]);
        data
    }

    #[test]
    fn render_net_section() {
        insta::assert_snapshot!("net", &render_net(&data_with_net(), false));
    }

    fn data_with_disks() -> Data {
        let mut data = data1();
        data.disks = vec![
            Disk {
                path: "/dev/nvme0n1".to_owned(),
                size: 1_000_204_886_016, // 931.5 GiB
                kind: "NVMe SSD (WD_BLACK SN850X 1000GB)".to_owned(),
                filesystems: vec![
                    Fs {
                        source: "/dev/nvme0n1p1".to_owned(),
                        fstype: "vfat".to_owned(),
                        size: 535_805_952, // 511.0 MiB
                        used: 126_922_752,
                        mount: "/boot".to_owned(),
                    },
                    Fs {
                        source: "/dev/nvme0n1p2".to_owned(),
                        fstype: "ext4".to_owned(),
                        size: 964_970_082_304,
                        used: 591_121_698_816,
                        mount: "/".to_owned(),
                    },
                ],
            },
            Disk {
                path: "/dev/sda".to_owned(),
                size: 250_059_350_016,
                kind: "USB SSD (250GB Card)".to_owned(),
                filesystems: Vec::new(),
            },
        ];
        data
    }

    #[test]
    fn render_disks_section() {
        insta::assert_snapshot!("disks", &render_disks(&data_with_disks(), false));
    }
}
