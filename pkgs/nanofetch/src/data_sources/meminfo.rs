use anyhow::Context;
use serde::Serialize;

use super::read_file;

/// Memory and swap usage in bytes, parsed together from `/proc/meminfo`.
#[derive(Debug, Default, Serialize)]
pub struct Meminfo {
    /// Memory usage in bytes.
    pub memory: Memory,
    /// Swap usage in bytes.
    pub swap: Memory,
}

/// Memory used / total in bytes.
#[derive(Debug, Default, Serialize)]
pub struct Memory {
    /// Bytes in use.
    pub used: u64,
    /// Total bytes.
    pub total: u64,
}

/// Collect memory + swap usage in bytes from `/proc/meminfo`.
pub fn collect() -> anyhow::Result<Meminfo> {
    let content = read_file("/proc/meminfo")?;
    parse(&content).context("parse /proc/meminfo")
}

/// Parse memory + swap usage in bytes from `/proc/meminfo` content.
///
/// `used = total - available` for memory and `used = total - free` for swap.
/// `None` if any key is missing.
fn parse(content: &str) -> Option<Meminfo> {
    let mut mem_total = None;
    let mut mem_available = None;
    let mut swap_total = None;
    let mut swap_free = None;
    for line in content.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(kb) = value
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        match key.trim() {
            "MemTotal" => mem_total = Some(kb),
            "MemAvailable" => mem_available = Some(kb),
            "SwapTotal" => swap_total = Some(kb),
            "SwapFree" => swap_free = Some(kb),
            _ => {}
        }
    }
    let mem_total = mem_total?;
    let mem_available = mem_available?;
    let swap_total = swap_total?;
    let swap_free = swap_free?;
    Some(Meminfo {
        memory: Memory {
            used: (mem_total - mem_available) * 1024,
            total: mem_total * 1024,
        },
        swap: Memory {
            used: (swap_total - swap_free) * 1024,
            total: swap_total * 1024,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meminfo_memory_and_swap() {
        const MEMINFO: &str = "\
MemTotal:       33554432 kB
MemFree:        20971520 kB
MemAvailable:   20971520 kB
Buffers:           123456 kB
Cached:            456789 kB
SwapCached:            0 kB
SwapTotal:       8388608 kB
SwapFree:        8388608 kB
";

        let Some(info) = parse(MEMINFO) else {
            panic!("parse should succeed on valid meminfo");
        };
        assert_eq!(info.memory.used, 12_884_901_888);
        assert_eq!(info.memory.total, 34_359_738_368);
        assert_eq!(info.swap.used, 0);
        assert_eq!(info.swap.total, 8_589_934_592);
    }

    #[test]
    fn parse_meminfo_missing_keys() {
        let cases = [
            ("no MemAvailable", "MemTotal: 1024 kB\n"),
            ("no MemTotal", "MemAvailable: 512 kB\n"),
            (
                "no swap",
                "\
MemTotal: 1024 kB
MemAvailable: 512 kB
",
            ),
            ("not meminfo", "hello world\n"),
        ];
        for (label, content) in cases {
            assert!(parse(content).is_none(), "case: {label}");
        }
    }
}
